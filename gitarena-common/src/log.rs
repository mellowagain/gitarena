use std::env::VarError;
use std::error::Error;
use std::path::Path;
use std::{env, fs, io};

use anyhow::{Context, Result};
use log::debug;
use tracing::metadata::LevelFilter;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::filter::FromEnvError;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{layer, EnvFilter, Registry};
use tracing_unwrap::ResultExt;

// Keep in sync with `gitarena::init_logger`
pub fn init_logger(module: &str, directives: &'static [&str]) -> Result<Vec<WorkerGuard>> {
    let mut guards = Vec::new();

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|err| default_env(err, directives));

    let stdout_layer = stdout().map(|(layer, guard)| {
        guards.push(guard);
        layer
    });

    let file_layer = log_file(module)?.map(|(layer, guard)| {
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
        .try_init()
        .context("Failed to initialize logger")?;

    debug!("Successfully initialized logger for {}", module);

    Ok(guards)
}

pub fn stdout<S: Subscriber + for<'a> LookupSpan<'a>>(
) -> Option<(impl layer::Layer<S>, WorkerGuard)> {
    if env::var_os("NO_STDOUT_LOG").is_some() {
        return None;
    }

    let (writer, guard) = tracing_appender::non_blocking(io::stdout());

    let layer = Layer::new().with_thread_ids(true).with_writer(writer);

    Some((layer, guard))
}

pub fn log_file<S: Subscriber + for<'a> LookupSpan<'a>>(
    module: &str,
) -> Result<Option<(impl layer::Layer<S>, WorkerGuard)>> {
    if cfg!(debug_assertions) || env::var_os("DEBUG_FILE_LOG").is_none() {
        return Ok(None);
    }

    let logs_dir = Path::new("logs");

    if !logs_dir.exists() {
        fs::create_dir_all(logs_dir)?;
    }

    let appender = rolling::daily(logs_dir, module);
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let layer = Layer::new()
        .with_thread_ids(true)
        .with_writer(writer)
        .json();

    Ok(Some((layer, guard)))
}

pub fn tokio_console<S: Subscriber + for<'a> LookupSpan<'a>>(
    filter: EnvFilter,
) -> (EnvFilter, Option<impl layer::Layer<S>>) {
    if !cfg!(tokio_unstable) {
        return (filter, None);
    }

    let filter = filter
        .add_directive("tokio=trace".parse().unwrap_or_log())
        .add_directive("runtime=trace".parse().unwrap_or_log());

    let layer = console_subscriber::spawn();

    (filter, Some(layer))
}

pub fn default_env(err: FromEnvError, directives: &[&str]) -> EnvFilter {
    let not_found = err
        .source()
        .map(|o| {
            o.downcast_ref::<VarError>()
                .map_or_else(|| false, |err| matches!(err, VarError::NotPresent))
        })
        .unwrap_or(false);

    if !not_found {
        eprintln!(
            "Warning: Unable to parse `{}` environment variable, using default values: {}",
            EnvFilter::DEFAULT_ENV,
            err
        );
    }

    let level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let mut filter = EnvFilter::default().add_directive(level.into());

    for directive in directives {
        filter = filter.add_directive(directive.parse().unwrap_or_log());
    }

    filter
}
