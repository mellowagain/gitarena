use std::env::VarError;
use std::error::Error;
use std::path::Path;
use std::{env, fs, io};

use anyhow::{Context, Result};
use log::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};
use tracing_unwrap::ResultExt;

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guards = init_logger()?;

    Ok(())
}

// TODO: Move this to gitarena-common as this will be commonly shared between all gitarena crates
fn init_logger() -> Result<Vec<WorkerGuard>> {
    let mut env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|err| {
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
    });

    let mut guards = Vec::<WorkerGuard>::with_capacity(1);

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
            fs::create_dir_all(logs_dir)?;
        }

        let appender = rolling::daily("logs", "gitarena-workhorse.log");
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

    let tokio_console = if cfg!(tokio_unstable) {
        env_filter = env_filter.add_directive("tokio=trace".parse().unwrap_or_log())
            .add_directive("runtime=trace".parse().unwrap_or_log());

        Some(console_subscriber::spawn())
    } else {
        None
    };

    // https://stackoverflow.com/a/66138267
    Registry::default()
        .with(env_filter)
        .with(stdout_log)
        .with(file_log)
        .with(tokio_console)
        .try_init()
        .context("Failed to initialize logger")?;

    Ok(guards)
}
