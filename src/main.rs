#![forbid(unsafe_code)]

use actix_web::{App, HttpServer};
use anyhow::{Result, Context, anyhow};
use chrono::Local;
use config::Config;
use fern::{Dispatch, log_file};
use log::{LevelFilter, info, warn, error, debug};
use sqlx::PgPool;
use std::borrow::{Cow, Borrow};
use std::env;
use std::fs;
use std::io::stdout;
use std::path::Path;

mod config;

#[actix_rt::main]
async fn main() -> Result<()> {
    init_logger()?;

    let cfg = load_config().context("Unable to load config file.")?;

    info!("Successfully loaded config file.");

    let db_pool = PgPool::new(&cfg.database).await?;
    sqlx::query("SELECT 1;").execute(&db_pool).await.context("Unable to connect to database.")?;

    info!("Successfully connected to database.");

    let bind_address: &str = cfg.bind.borrow();

    let server = HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
    }).bind(bind_address).context("Unable to bind HTTP server.")?;

    server.run().await.context("Unable to start HTTP server.")?;

    info!("Thank you and goodbye.");

    Ok(())
}

fn load_config() -> Result<Cow<'static, Config>> {
    let cfg_str = env::var("GITARENA_CONFIG").unwrap_or("config.toml".to_owned());
    let cfg_path = Path::new(cfg_str.as_str());

    if !cfg_path.is_file() {
        return Err(anyhow!("Config file does not exist: {}", cfg_path.display()));
    }

    match Config::load_from(cfg_path) {
        Ok(config) => Ok(config),
        Err(err) => Err(anyhow!("Unable to load config file: {}", err)),
    }
}

fn init_logger() -> Result<()> {
    let logs_dir = Path::new("logs");

    if !logs_dir.is_dir() {
        // Check if `logs` is a file and not a directory
        if logs_dir.exists() {
            fs::remove_file(logs_dir).context("Unable to delete `logs` file.")?;
        }

        fs::create_dir(logs_dir).context("Unable to create `logs` directory.")?;
    }

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {} {} - {}",
                record.target(),
                record.level(),
                record.module_path().unwrap_or("null"),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .level_for("sqlx", LevelFilter::Info)
        .chain(stdout())
        .chain(log_file(format!("logs/{}.log", Local::now().timestamp_millis()))?)
        .apply()
        .context("Failed to initialize logger.")
}
