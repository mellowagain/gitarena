#![forbid(unsafe_code)]

use std::borrow::{Borrow, Cow};
use std::env;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::dev::Service;
use actix_web::http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL};
use actix_web::http::HeaderValue;
use actix_web::{App, HttpServer};
use anyhow::{anyhow, Context, Result};
use askalono::Store;
use config::Config;
use fs_extra::dir;
use lazy_static::lazy_static;
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
mod extensions;
mod git;
mod licenses;
mod mail;
mod repository;
mod routes;
mod templates;
mod user;
mod verification;
mod privileges;

lazy_static! {
    static ref CONFIG: Cow<'static, Config> = load_config();
    static ref LICENSE_STORE: Mutex<Store> = Mutex::new(Store::new());
}

// TODO: big executables are not pushable
// todo: big commits are just not pushable

#[actix_rt::main]
async fn main() -> Result<()> {
    let _log = init_logger()?;

    let db_pool = PgPoolOptions::new()
        .max_connections(num_cpus::get() as u32)
        .connect_timeout(Duration::from_secs(10))
        .connect(&CONFIG.database)
        .await?;

    sqlx::query("select 1;").execute(&db_pool).await.context("Unable to connect to database.")?;

    info!("Successfully connected to database.");

    licenses::init()?;

    info!("Successfully loaded SPDX license data.");

    let _watcher = templates::init().await?;

    let bind_address: &str = CONFIG.bind.borrow();

    let server = HttpServer::new(move || {
        let secret = (CONFIG.secret.borrow() as &str).as_bytes();
        let domain: &str = CONFIG.domain.borrow();
        let secure = domain.starts_with("https");

        let identity_service = IdentityService::new(
            CookieIdentityPolicy::new(secret)
                .name("gitarena-auth")
                .max_age(TimeDuration::days(10).whole_seconds())
                .http_only(true)
                .secure(secure)
        );

        let mut app = App::new()
            .data(db_pool.clone())
            .wrap(identity_service)
            .wrap_fn(|req, srv| {
                let fut = srv.call(req);
                async {
                    let mut res = fut.await?;

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
            .configure(routes::repository::init)
            .configure(routes::user::init);

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
    }).bind(bind_address).context("Unable to bind HTTP server.")?;

    server.run().await.context("Unable to start HTTP server.")?;

    info!("Thank you and goodbye.");

    Ok(())
}

fn load_config() -> Cow<'static, Config> {
    let cfg_str = env::var("GITARENA_CONFIG").unwrap_or("config.toml".to_owned());
    let cfg_path = Path::new(cfg_str.as_str());

    if !cfg_path.is_file() {
        panic!("Config file does not exist: {}", cfg_path.display());
    }

    let config = match Config::load_from(cfg_path) {
        Ok(config) => config,
        Err(err) => panic!("Unable to load config file: {}", err),
    };

    let secret: &str = config.secret.borrow();

    if secret.is_empty() {
        panic!("Found empty secret in config");
    }

    let secret_bytes = secret.as_bytes();

    if secret_bytes.len() < 32 {
        panic!("Secret in config needs to be at least 32 bytes long");
    }

    config
}

fn init_logger() -> Result<WorkerGuard> {
    let logs_dir = Path::new("logs");

    if !logs_dir.exists() {
        dir::create_all(logs_dir, false)?;
    }

    let level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let appender = rolling::daily("logs", "gitarena");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|err| {
        // err (type FromEnvError) does not expose its `kind` field so we have to display it and compare it to the output
        if format!("{}", err).as_str() != "environment variable not found" {
            eprintln!("Warning: Unable to parse `RUST_LOG` environment variable, using default values: {}", err);
        }

        EnvFilter::default()
            .add_directive(level.into())
            .add_directive("sqlx=warn".parse().unwrap_or_log())
            .add_directive("reqwest=info".parse().unwrap_or_log())
            .add_directive("globset=info".parse().unwrap_or_log())
            .add_directive("askalono=warn".parse().unwrap_or_log())
    });

    // In debug mode we only write to stdout, in production only to a file
    if cfg!(debug_assertions) {
        FmtSubscriber::builder()
            .with_env_filter(env_filter)
            .try_init()
            .map_err(|err| anyhow!("Unable to create logger: {}", err))?;
    } else {
        FmtSubscriber::builder()
            .with_writer(writer) // TODO: Write additionally also to stdout in production
            .with_env_filter(env_filter)
            .try_init()
            .map_err(|err| anyhow!("Unable to create logger: {}", err))?;
    }

    Ok(guard)
}
