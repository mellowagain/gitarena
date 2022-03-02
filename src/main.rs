#![forbid(unsafe_code)]

use crate::error::error_renderer_middleware;

use std::env::VarError;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use std::{env, io};
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
use anyhow::{anyhow, bail, Context, Result};
use fs_extra::dir;
use gitarena_macros::from_optional_config;
use log::info;
use magic::{Cookie, CookieFlags};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use time::Duration as TimeDuration;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{FmtSubscriber, EnvFilter, Registry};
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
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

#[tokio::main]
async fn main() -> Result<()> {
    let mut _log_guards = init_logger()?;

    let max_pool_connections = match env::var("MAX_POOL_CONNECTIONS") {
        Ok(env_str) => env_str.parse::<u32>().context("Unable to parse MAX_POOL_CONNECTIONS environment variable into a u32")?,
        Err(VarError::NotPresent) => num_cpus::get() as u32,
        Err(VarError::NotUnicode(_)) => bail!("MAX_POOL_CONNECTIONS environment variable is not a valid unicode string")
    };

    let db_pool = PgPoolOptions::new()
        .max_connections(max_pool_connections)
        .connect_timeout(Duration::from_secs(10))
        .connect_with(read_database_config()?)
        .await?;

    _log_guards = config::init(&db_pool, _log_guards).await.context("Unable to initialize config in database")?;

    licenses::init().await;

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

        let cookie = Arc::new(read_magic_database().expect_or_log("Failed to libmagic database"));

        let mut app = App::new()
            .app_data(Data::new(db_pool.clone())) // Pool<Postgres> is just a wrapper around Arc<P> so .clone() is cheap
            .app_data(Data::new(cookie.clone()))
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
                    .show_files_listing()
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

fn init_logger() -> Result<Vec<WorkerGuard>> {
    #[cfg(debug_assertions)]
    const GUARD_VEC_PRE_ALLOCATION: usize = 1;
    #[cfg(not(debug_assertions))]
    const GUARD_VEC_PRE_ALLOCATION: usize = 2;

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

    let mut guards = Vec::<WorkerGuard>::with_capacity(GUARD_VEC_PRE_ALLOCATION);

    // In debug mode we only write to stdout, in production to a both stdout (pretty) and file (json)
    let stdout_log = {
        let (writer, guard) = tracing_appender::non_blocking(io::stdout());

        let layer = Layer::new()
            .with_thread_ids(true)
            .with_writer(writer);

        guards.push(guard);
        layer
    };

    let file_log = if cfg!(debug_assertions) || env::var_os("DEBUG_FILE_LOG").is_some() {
        let logs_dir = Path::new("logs");

        if !logs_dir.exists() {
            dir::create_all(logs_dir, false)?;
        }

        let appender = rolling::daily("logs", "gitarena.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);

        let layer = Layer::new()
            .with_thread_ids(true)
            .with_writer(writer)
            .json();

        guards.push(guard);
        Some(layer)
    } else {
        None
    };

    // https://stackoverflow.com/a/66138267
    Registry::default()
        .with(env_filter)
        .with(stdout_log)
        .with(file_log)
        .try_init()
        .map_err(|err| anyhow!(err))?; // https://github.com/dtolnay/anyhow/issues/83

    Ok(guards)
}

fn read_database_config() -> Result<PgConnectOptions> {
    let mut options = match (env::var_os("DATABASE_URL"), env::var_os("DATABASE_URL_FILE")) {
        (Some(url), None) => {
            let str = url.into_string().map_err(|_| anyhow!("`DATABASE_URL` environment variable is not valid unicode"))?;
            PgConnectOptions::from_str(str.as_str())?
        },
        (None, Some(file)) => {
            let url = fs::read_to_string(file)?;
            PgConnectOptions::from_str(url.as_str())?
        },
        _ => bail!("Either environment variable `DATABASE_URL` or `DATABASE_URL_FILE` needs to be specified to before starting GitArena")
    };

    match env::var("DATABASE_PASSWORD_FILE") {
        Ok(file) => {
            let password = fs::read_to_string(file)?;
            options = options.password(password.as_str());
        }
        Err(VarError::NotUnicode(_)) => bail!("`DATABASE_PASSWORD_FILE` environment variable is not valid unicode"),
        Err(VarError::NotPresent) => { /* No password auth required, or it was already set in the connection string; safe to ignore */ }
    }

    Ok(options)
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
