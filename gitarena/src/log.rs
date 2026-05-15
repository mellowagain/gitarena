use crate::sse::Broadcaster;
use crate::utils::admin_panel_layer::AdminPanelLayer;
use actix_web::web::Data;
use anyhow::{Context, Result};
use futures_locks::RwLock;
use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use std::env::VarError;
use std::error::Error;
use std::path::Path;
use std::{env, fs, io};
use tracing::Subscriber;
use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::filter::FromEnvError;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, layer};
use tracing_unwrap::ResultExt;

pub fn init_logger(broadcaster: &Data<RwLock<Broadcaster>>, logger_provider: Option<&SdkLoggerProvider>) -> Result<Vec<WorkerGuard>> {
    let mut guards = Vec::new();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|err| default_env(&err));

    let stdout_layer = stdout().map(|(layer, guard)| {
        guards.push(guard);
        layer
    });

    let file_layer = log_file()?.map(|(layer, guard)| {
        guards.push(guard);
        layer
    });

    let (env_filter, tokio_console_layer) = tokio_console(env_filter);

    let otel_tracing_layer = OpenTelemetryLayer::new(global::tracer("gitarena".to_string()));
    let otel_log_bridge = logger_provider.map(OpenTelemetryTracingBridge::new);

    // https://stackoverflow.com/a/66138267
    Registry::default()
        .with(AdminPanelLayer::new(broadcaster.clone()))
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(tokio_console_layer)
        .with(otel_tracing_layer)
        .with(otel_log_bridge)
        .try_init()
        .context("Failed to initialize logger")?;

    tracing::debug!("Successfully initialized logger");

    Ok(guards)
}

#[must_use]
pub fn stdout<S: Subscriber + for<'a> LookupSpan<'a>>() -> Option<(impl layer::Layer<S>, WorkerGuard)> {
    if env::var_os("NO_STDOUT_LOG").is_some() {
        return None;
    }

    let (writer, guard) = tracing_appender::non_blocking(io::stdout());

    let layer = Layer::new().with_thread_ids(true).with_writer(writer);

    Some((layer, guard))
}

pub fn log_file<S: Subscriber + for<'a> LookupSpan<'a>>() -> Result<Option<(impl layer::Layer<S>, WorkerGuard)>> {
    if cfg!(debug_assertions) || env::var_os("DEBUG_FILE_LOG").is_none() {
        return Ok(None);
    }

    let logs_dir = Path::new("logs");

    if !logs_dir.exists() {
        fs::create_dir_all(logs_dir)?;
    }

    let appender = rolling::daily(logs_dir, "gitarena");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let layer = Layer::new().with_thread_ids(true).with_writer(writer).json();

    Ok(Some((layer, guard)))
}

pub fn tokio_console<S: Subscriber + for<'a> LookupSpan<'a>>(filter: EnvFilter) -> (EnvFilter, Option<impl layer::Layer<S>>) {
    if !cfg!(tokio_unstable) {
        return (filter, None);
    }

    let filter = filter
        .add_directive("tokio=trace".parse().unwrap_or_log())
        .add_directive("runtime=trace".parse().unwrap_or_log());

    let layer = console_subscriber::spawn();

    (filter, Some(layer))
}

#[must_use]
pub fn default_env(err: &FromEnvError) -> EnvFilter {
    let not_found = err
        .source()
        .is_some_and(|o| o.downcast_ref::<VarError>().map_or_else(|| false, |err| matches!(err, VarError::NotPresent)));

    if !not_found {
        eprintln!(
            "Warning: Unable to parse `{}` environment variable, using default values: {}",
            EnvFilter::DEFAULT_ENV,
            err
        );
    }

    let level = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::INFO };

    EnvFilter::default()
        .add_directive(level.into())
        .add_directive("actix_http=info".parse().unwrap())
        .add_directive("actix_server=info".parse().unwrap())
        .add_directive("askalono=warn".parse().unwrap())
        .add_directive("globset=info".parse().unwrap())
        .add_directive("h2=info".parse().unwrap())
        .add_directive("hyper=info".parse().unwrap())
        .add_directive("reqwest=info".parse().unwrap())
        .add_directive("rustls=info".parse().unwrap())
        .add_directive("sqlx=warn".parse().unwrap())
}
