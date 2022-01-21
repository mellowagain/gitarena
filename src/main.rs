#![forbid(unsafe_code)]

use crate::error::error_renderer_middleware;

use std::env::VarError;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use std::{env, io};

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
use anyhow::{anyhow, bail, Context, Result};
use fs_extra::dir;
use gitarena_macros::from_optional_config;
use log::info;
use sqlx::postgres::PgPoolOptions;
use time::Duration as TimeDuration;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use tracing_unwrap::ResultExt;

mod captcha;
mod config;
mod crypto;
mod error;
mod git;
mod licenses;
mod mail;
mod prelude;
mod privileges;
mod repository;
mod routes;
mod session;
mod sso;
mod templates;
mod user;
mod utils;
mod verification;

#[actix_rt::main]
async fn main() -> Result<()> {
    let mut _log_guard = init_logger()?;

    let db_url = env::var("DATABASE_URL").context("Unable to read mandatory DATABASE_URL environment variable")?;
    env::remove_var("DATABASE_URL"); // Remove the env variable now to prevent it from being passed to a untrusted child process later

    let max_pool_connections = match env::var("MAX_POOL_CONNECTIONS") {
        Ok(env_str) => env_str.parse::<u32>().context("Unable to parse MAX_POOL_CONNECTIONS environment variable into a u32")?,
        Err(VarError::NotPresent) => num_cpus::get() as u32,
        Err(VarError::NotUnicode(_)) => bail!("MAX_POOL_CONNECTIONS environment variable is not a valid unicode string")
    };

    let db_pool = PgPoolOptions::new()
        .max_connections(max_pool_connections)
        .connect_timeout(Duration::from_secs(10))
        .connect(db_url.as_str())
        .await?;

    _log_guard = config::init(&db_pool, _log_guard).await.context("Unable to initialize config in database")?;

    licenses::init().await?;

    let _watcher = templates::init().await?;

    let bind_address = env::var("BIND_ADDRESS").context("Unable to read mandatory BIND_ADDRESS environment variable")?;

    let (secret, domain): (Option<String>, Option<String>) = from_optional_config!("secret" => String, "domain" => String);
    let secret = secret.ok_or_else(|| anyhow!("Unable to read secret from database"))?;
    let secure = domain.map_or_else(|| false, |d| d.starts_with("https"));

    let server = HttpServer::new(move || {
        let identity_service = IdentityService::new(
            CookieIdentityPolicy::new(secret.as_bytes())
                .name("gitarena-auth")
                .max_age(TimeDuration::days(10))
                .http_only(true)
                .same_site(SameSite::Lax)
                .secure(secure)
        );

        let mut app = App::new()
            .app_data(Data::new(db_pool.clone())) // Pool<Postgres> is just a wrapper around Arc<P> so .clone() is cheap
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
            .route("/favicon.ico", to(|| HttpResponse::MovedPermanently().append_header((LOCATION, "/static/img/favicon.ico")).finish()));

        if cfg!(debug_assertions) {
            app = app.service(
                Files::new("/static", "./static")
                    .show_files_listing()
                    .use_etag(false)
                    .use_last_modified(false)
                    .use_hidden_files()
            );
        }

        app
    }).bind(bind_address.as_str()).context("Unable to bind HTTP server.")?;

    server.run().await.context("Unable to start HTTP server.")?;

    info!("Thank you and goodbye.");

    Ok(())
}

fn init_logger() -> Result<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|err| {
        let not_found = err.source()
            .map(|o| o.downcast_ref::<VarError>().map_or_else(|| false, |err| matches!(err, VarError::NotPresent)))
            .unwrap_or(false);

        if !not_found {
            eprintln!("Warning: Unable to parse `{}` environment variable, using default values: {}", EnvFilter::DEFAULT_ENV, err);
        }

        let level = if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        };

        EnvFilter::default()
            .add_directive(level.into())
            .add_directive("askalono=warn".parse().unwrap_or_log())
            .add_directive("globset=info".parse().unwrap_or_log())
            .add_directive("h2=info".parse().unwrap_or_log())
            .add_directive("hyper=info".parse().unwrap_or_log())
            .add_directive("reqwest=info".parse().unwrap_or_log())
            .add_directive("rustls=info".parse().unwrap_or_log())
            .add_directive("sqlx=warn".parse().unwrap_or_log())
    });

    // In debug mode we only write to stdout (pretty), in production to a file (json)
    Ok(if cfg!(debug_assertions) {
        let (writer, guard) = tracing_appender::non_blocking(io::stdout());

        FmtSubscriber::builder()
            .with_writer(writer)
            .with_env_filter(env_filter)
            .with_thread_ids(true)
            .try_init()
            .map_err(|err| anyhow!(err))?; // https://github.com/dtolnay/anyhow/issues/83

        guard
    } else {
        let logs_dir = Path::new("logs");

        if !logs_dir.exists() {
            dir::create_all(logs_dir, false)?;
        }

        let appender = rolling::daily("logs", "gitarena");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        FmtSubscriber::builder()
            .with_writer(writer)
            .with_env_filter(env_filter)
            .with_thread_ids(true)
            .json()
            .try_init()
            .map_err(|err| anyhow!(err))?; // https://github.com/dtolnay/anyhow/issues/83

        guard
    })
}
