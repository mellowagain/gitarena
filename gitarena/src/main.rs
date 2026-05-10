#![deny(unsafe_code)]

use crate::error::error_renderer_middleware;
use crate::ipc::Ipc;
use crate::metrics::db_pool::spawn_db_pool_metrics_task;
use crate::routes::ApiDoc;
use crate::sse::Broadcaster;
use crate::utils::admin_panel_layer::AdminPanelLayer;
use crate::utils::system::SYSTEM_INFO;

use std::env;

use crate::verification::ExpiredVerifyLinkRemovalTask;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::body::{BoxBody, EitherBody};
use actix_web::cookie::SameSite;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::Method;
use actix_web::http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, HeaderValue, LOCATION};
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::web::{Data, route, to};
use actix_web::{App, HttpResponse, HttpServer, web};
use anyhow::{Context, Result, anyhow};
use fang::{AsyncQueueable, AsyncRunnable};
use futures_locks::RwLock;
use gitarena_common::database::{Pool, create_postgres_pool};
use gitarena_common::log::init_logger;
use gitarena_common::telemetry;
use gitarena_macros::from_optional_config;
use opentelemetry_instrumentation_actix_web::{RequestMetrics, RequestTracing};
use time::Duration as TimeDuration;
use tokio::sync::OnceCell;
use tracing::{info, warn};
use tracing_subscriber::Layer;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;

mod captcha;
mod config;
mod crypto;
mod error;
mod geoip;
mod git;
mod ipc;
mod issue;
mod licenses;
mod mail;
mod metrics;
mod organization;
mod passkey;
mod prelude;
mod privileges;
mod queue;
mod repository;
mod routes;
mod session;
mod sse;
mod ssh;
mod sso;
mod user;
mod utils;
mod verification;

pub(crate) static TASK_DB_POOL: OnceCell<Pool> = OnceCell::const_new();

#[tokio::main]
async fn main() -> Result<()> {
    let (telemetry_guards, logger_provider) = telemetry::init("gitarena")?;

    let broadcaster = Broadcaster::new();
    let _log_guards = init_logger(
        "gitarena",
        &[
            "actix_http=info",
            "actix_server=info",
            "askalono=warn",
            "globset=info",
            "h2=info",
            "hyper=info",
            "reqwest=info",
            "rustls=info",
            "sqlx=warn",
        ],
        Some(AdminPanelLayer::new(broadcaster.clone()).boxed()),
        Some(&logger_provider),
    )?;

    if !telemetry_guards.is_guarding() {
        warn!("OpenTelemetry exporting is disabled because the env variables were not set");
    }

    let db_pool = create_postgres_pool("gitarena", None).await?;
    TASK_DB_POOL
        .set(db_pool.clone())
        .map_err(|_| anyhow!("task db pool should not be set more than once"))?;

    let _task_handle = spawn_db_pool_metrics_task(db_pool.clone());

    licenses::init();

    // read the `Lazy` to initialize it but immediately drop the returned guard to prevent a deadlock
    let _ = SYSTEM_INFO.read().await;

    let bind_address = env::var("BIND_ADDRESS").context("Unable to read mandatory BIND_ADDRESS environment variable")?;

    let (secret, domain): (Option<String>, Option<String>) = from_optional_config!("secret" => String, "domain" => String);
    let secret = secret.ok_or_else(|| anyhow!("Unable to read secret from database"))?;
    let secure = domain.as_deref().is_some_and(|d| d.starts_with("https"));

    let webauthn_origin: Option<String> = from_optional_config!("webauthn.origin" => String);
    let webauthn_domain = domain.unwrap_or_else(|| "http://localhost:8320".to_owned());
    let webauthn = passkey::build_webauthn(&webauthn_domain, webauthn_origin.as_deref())?;

    mail::create_transport(&db_pool).await?;

    let queue = queue::init().await?;

    let cron = ExpiredVerifyLinkRemovalTask {};
    queue
        .schedule_task(&cron as &dyn AsyncRunnable)
        .await
        .context("failed to schedule remove expired verify link cron job")?;

    let ipc = RwLock::new(Ipc::new().await?);

    if !ipc.read().await.is_connected() {
        ipc::spawn_connection_task(ipc.clone());
    }

    ssh::init(db_pool.clone(), &bind_address).await?;

    let server = HttpServer::new(move || {
        let identity_service = IdentityService::new(
            CookieIdentityPolicy::new(secret.as_bytes())
                .name("gitarena-auth")
                .max_age(TimeDuration::days(10))
                .http_only(true)
                .same_site(SameSite::Lax)
                .secure(secure),
        );

        App::new()
            .app_data(Data::new(db_pool.clone()))
            .app_data(Data::new(ipc.clone()))
            .app_data(broadcaster.clone())
            .app_data(Data::new(webauthn.clone()))
            .app_data(Data::new(queue.clone()))
            .wrap(RequestTracing::new()) // must we outermost wrap to capture full duration
            .wrap(RequestMetrics::default())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(identity_service)
            .wrap_fn(|req, srv| {
                let fut = srv.call(req);
                async {
                    let mut res: ServiceResponse<EitherBody<BoxBody>> = fut.await?;

                    if res.request().path().contains(".git") {
                        // https://git-scm.com/docs/http-protocol/en#_smart_server_response
                        // "Cache-Control headers SHOULD be used to disable caching of the returned entity."
                        res.headers_mut()
                            .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache, max-age=0, must-revalidate"));
                    } else {
                        res.headers_mut().insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
                    }

                    Ok(res)
                }
            })
            .wrap_fn(error_renderer_middleware)
            .default_service(route().method(Method::GET).to(routes::not_found::default_handler))
            .configure(routes::init)
            .configure(routes::proxy::init)
            .configure(routes::user::init)
            .configure(routes::organization::init)
            .service(RapiDoc::with_openapi("/api-docs/openapi.json", ApiDoc::openapi()).path("/rapidoc"))
            .configure(routes::repository::init) // Repository routes need to be always last
            .route("/healthz", web::get().to(|| async { "healthy" }))
            .route(
                "/favicon.ico",
                to(|| async { HttpResponse::MovedPermanently().append_header((LOCATION, "/static/img/favicon.ico")).finish() }),
            )
    })
    .bind(bind_address.as_str())
    .context("Unable to bind HTTP server.")?;

    server.run().await.context("Unable to start HTTP server.")?;

    info!("Thank you and goodbye.");

    Ok(())
}
