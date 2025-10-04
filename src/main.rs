#![forbid(unsafe_code)]

use crate::error::error_renderer_middleware;
use crate::ipc::Ipc;
use crate::sse::Broadcaster;
use crate::utils::admin_panel_layer::AdminPanelLayer;
use crate::utils::system::SYSTEM_INFO;

use std::env;
use std::env::VarError;
use std::sync::Arc;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::body::{BoxBody, EitherBody};
use actix_web::cookie::SameSite;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::header::{HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, LOCATION};
use actix_web::http::Method;
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::web::{route, to, Data};
use actix_web::{App, HttpResponse, HttpServer};
use anyhow::{anyhow, Context, Result};
use futures_locks::RwLock;
use gitarena_common::database::create_postgres_pool;
use gitarena_common::log::{default_env, init_logger, log_file, stdout, tokio_console};
use gitarena_macros::from_optional_config;
use log::info;
use magic::{Cookie, CookieFlags};
use time::Duration as TimeDuration;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};
use tracing_unwrap::ResultExt;

mod captcha;
mod config;
mod crypto;
mod error;
mod git;
mod ipc;
mod issue;
mod licenses;
mod mail;
mod prelude;
mod privileges;
mod repository;
mod routes;
mod session;
mod sse;
mod ssh;
mod sso;
mod templates;
mod user;
mod utils;
mod verification;

#[tokio::main]
async fn main() -> Result<()> {
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
    )?;

    let db_pool = create_postgres_pool("gitarena", None).await?;
    sqlx::migrate!().run(&db_pool).await?;

    licenses::init().await;

    // read the `Lazy` to initialize it but immediately drop the returned guard to prevent a deadlock
    let _ = SYSTEM_INFO.read().await;
    let _watcher = templates::init().await?;

    let bind_address = env::var("BIND_ADDRESS")
        .context("Unable to read mandatory BIND_ADDRESS environment variable")?;

    let (secret, domain): (Option<String>, Option<String>) =
        from_optional_config!("secret" => String, "domain" => String);
    let secret = secret.ok_or_else(|| anyhow!("Unable to read secret from database"))?;
    let secure = domain.map_or_else(|| false, |d| d.starts_with("https"));

    let ipc = RwLock::new(Ipc::new().await?);

    if !ipc.read().await.is_connected() {
        ipc::spawn_connection_task(ipc.clone());
    }

    let server = HttpServer::new(move || {
        let identity_service = IdentityService::new(
            CookieIdentityPolicy::new(secret.as_bytes())
                .name("gitarena-auth")
                .max_age(TimeDuration::days(10))
                .http_only(true)
                .same_site(SameSite::Lax)
                .secure(secure),
        );

        let cookie = Arc::new(read_magic_database().expect_or_log("Failed to libmagic database"));

        let mut app = App::new()
            .app_data(Data::new(db_pool.clone())) // Pool<Postgres> is just a wrapper around Arc<P> so .clone() is cheap
            .app_data(Data::new(cookie))
            .app_data(Data::new(ipc.clone()))
            .app_data(broadcaster.clone())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(identity_service)
            .wrap_fn(|req, srv| {
                let fut = srv.call(req);
                async {
                    let mut res: ServiceResponse<EitherBody<BoxBody>> = fut.await?;

                    if res.request().path().contains(".git") {
                        // https://git-scm.com/docs/http-protocol/en#_smart_server_response
                        // "Cache-Control headers SHOULD be used to disable caching of the returned entity."
                        res.headers_mut().insert(
                            CACHE_CONTROL,
                            HeaderValue::from_static("no-cache, max-age=0, must-revalidate"),
                        );
                    }

                    if res.request().path().starts_with("/api") {
                        res.headers_mut()
                            .insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
                    }

                    Ok(res)
                }
            })
            .wrap_fn(error_renderer_middleware)
            .default_service(
                route()
                    .method(Method::GET)
                    .to(routes::not_found::default_handler),
            )
            .service(routes::admin::all())
            .configure(routes::init)
            .configure(routes::proxy::init)
            .configure(routes::user::init)
            .configure(routes::repository::init) // Repository routes need to be always last
            .route(
                "/favicon.ico",
                to(|| async {
                    HttpResponse::MovedPermanently()
                        .append_header((LOCATION, "/static/img/favicon.ico"))
                        .finish()
                }),
            );

        let debug_mode = cfg!(debug_assertions);
        let serve_static = matches!(
            env::var("SERVE_STATIC_FILES"),
            Ok(_) | Err(VarError::NotUnicode(_))
        ) || debug_mode;

        if serve_static {
            app = app.service(
                Files::new("/static", "./static")
                    .use_etag(!debug_mode)
                    .use_last_modified(!debug_mode)
                    .use_hidden_files(),
            );
        }

        app
    })
    .bind(bind_address.as_str())
    .context("Unable to bind HTTP server.")?;

    server.run().await.context("Unable to start HTTP server.")?;

    info!("Thank you and goodbye.");

    Ok(())
}

fn read_magic_database() -> Result<Cookie> {
    let cookie = Cookie::open(CookieFlags::default())?;

    // https://man7.org/linux/man-pages/man3/libmagic.3.html
    let database_path = if let Some(magic_env) = env::var_os("MAGIC") {
        magic_env
            .into_string()
            .expect_or_log("`MAGIC` environment variable contains invalid UTF-8 string")
    } else {
        "magic".to_owned()
    };

    cookie.load(&[database_path.as_str()])?;

    Ok(cookie)
}
