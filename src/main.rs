#![forbid(unsafe_code)]

use crate::error::error_renderer_middleware;
use crate::ipc::Ipc;
use crate::sse::Broadcaster;
use crate::utils::admin_panel_layer::AdminPanelLayer;

use std::env::VarError;
use std::env;
use std::sync::Arc;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::body::{BoxBody, EitherBody};
use actix_web::cookie::SameSite;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, HeaderValue, LOCATION};
use actix_web::http::Method;
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::web::{Data, route, to};
use actix_web::{App, HttpResponse, HttpServer};
use anyhow::{anyhow, Context, Result};
use futures_locks::RwLock;
use gitarena_common::database::create_postgres_pool;
use gitarena_common::log::{default_env, log_file, stdout, tokio_console};
use gitarena_macros::from_optional_config;
use log::info;
use magic::{Cookie, CookieFlags};
use time::Duration as TimeDuration;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};
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
    let mut _log_guards = init_logger(broadcaster.clone())?;

    let db_pool = create_postgres_pool("gitarena", None).await?;
    sqlx::migrate!().run(&db_pool).await?;

    licenses::init().await;

    let _watcher = templates::init().await?;

    let bind_address = env::var("BIND_ADDRESS").context("Unable to read mandatory BIND_ADDRESS environment variable")?;

    let (secret, domain): (Option<String>, Option<String>) = from_optional_config!("secret" => String, "domain" => String);
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
                .secure(secure)
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
                            CACHE_CONTROL, HeaderValue::from_static("no-cache, max-age=0, must-revalidate"),
                        );
                    }

                    if res.request().path().starts_with("/api") {
                        res.headers_mut().insert(
                            ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"),
                        );
                    }

                    Ok(res)
                }
            })
            .wrap_fn(error_renderer_middleware)
            .default_service(route().method(Method::GET).to(routes::not_found::default_handler))
            .service(routes::admin::all())
            .configure(routes::init)
            .configure(routes::proxy::init)
            .configure(routes::user::init)
            .configure(routes::repository::init) // Repository routes need to be always last
            .route("/favicon.ico", to(|| async {
                HttpResponse::MovedPermanently().append_header((LOCATION, "/static/img/favicon.ico")).finish()
            }));

        let debug_mode = cfg!(debug_assertions);
        let serve_static = matches!(env::var("SERVE_STATIC_FILES"), Ok(_) | Err(VarError::NotUnicode(_))) || debug_mode;

        if serve_static {
            app = app.service(
                Files::new("/static", "./static")
                    .use_etag(!debug_mode)
                    .use_last_modified(!debug_mode)
                    .use_hidden_files()
            );
        }

        app
    }).bind(bind_address.as_str()).context("Unable to bind HTTP server.")?;

    server.run().await.context("Unable to start HTTP server.")?;

    info!("Thank you and goodbye.");

    Ok(())
}

// This method is basically the same as `gitarena_common::log::init_logger` except it additionally adds the AdminPanelLayer at the end
// Please keep this in sync with it
fn init_logger(broadcaster: Data<RwLock<Broadcaster>>) -> Result<Vec<WorkerGuard>> {
    let mut guards = Vec::new();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|err| default_env(err, &[
        "actix_http=info",
        "actix_server=info",
        "askalono=warn",
        "globset=info",
        "h2=info",
        "hyper=info",
        "reqwest=info",
        "rustls=info",
        "sqlx=warn"
    ]));

    let stdout_layer = stdout().map(|(layer, guard)| {
        guards.push(guard);
        layer
    });

    let file_layer = log_file("gitarena")?.map(|(layer, guard)| {
        guards.push(guard);
        layer
    });

    let (env_filter, tokio_console_layer) = tokio_console(env_filter);

    // https://stackoverflow.com/a/66138267
    Registry::default()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(tokio_console_layer)
        .with(AdminPanelLayer::new(broadcaster))
        .try_init()
        .context("Failed to initialize logger")?;

    Ok(guards)
}

fn read_magic_database() -> Result<Cookie> {
    let cookie = Cookie::open(CookieFlags::default())?;

    // https://man7.org/linux/man-pages/man3/libmagic.3.html
    let database_path = if let Some(magic_env) = env::var_os("MAGIC") {
        magic_env.into_string().expect_or_log("`MAGIC` environment variable contains invalid UTF-8 string")
    } else {
        "magic".to_owned()
    };

    cookie.load(&[database_path.as_str()])?;

    Ok(cookie)
}
