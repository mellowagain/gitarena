use std::env;

use anyhow::Result;
use opentelemetry::global;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::BatchLogProcessor;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};

/// dropping this struct causes the observability stack to be flushed
pub struct TelemetryGuards {
    tracer: Option<SdkTracerProvider>,
    meter: Option<SdkMeterProvider>,
    logger: SdkLoggerProvider,
}

impl TelemetryGuards {
    /// Whether this guard even guards something lol
    pub fn is_guarding(&self) -> bool {
        self.tracer.is_some() && self.meter.is_some()
    }
}

impl Drop for TelemetryGuards {
    fn drop(&mut self) {
        if let Some(ref p) = self.tracer {
            let _ = p.shutdown();
        }
        if let Some(ref p) = self.meter {
            let _ = p.shutdown();
        }
        let _ = self.logger.shutdown();
    }
}

fn is_configured() -> bool {
    env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_some()
        || env::var_os("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_some()
        || env::var_os("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT").is_some()
        || env::var_os("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT").is_some()
}

/// needs to be called before `init_logger`
pub fn init(service_name: &'static str) -> Result<(TelemetryGuards, SdkLoggerProvider)> {
    if !is_configured() {
        let noop_logger = SdkLoggerProvider::builder().build();
        let clone = noop_logger.clone();
        return Ok((
            TelemetryGuards {
                tracer: None,
                meter: None,
                logger: noop_logger,
            },
            clone,
        ));
    }

    let resource = Resource::builder()
        .with_service_name(env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| service_name.to_owned()))
        .build();

    let span_exporter = SpanExporter::builder().with_tonic().build()?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_span_processor(BatchSpanProcessor::builder(span_exporter).build())
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    let metric_exporter = MetricExporter::builder().with_tonic().build()?;
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_reader(PeriodicReader::builder(metric_exporter).build())
        .build();
    global::set_meter_provider(meter_provider.clone());

    let log_exporter = LogExporter::builder().with_tonic().build()?;
    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_log_processor(BatchLogProcessor::builder(log_exporter).build())
        .build();

    let logger_for_bridge = logger_provider.clone();

    Ok((
        TelemetryGuards {
            tracer: Some(tracer_provider),
            meter: Some(meter_provider),
            logger: logger_provider,
        },
        logger_for_bridge,
    ))
}
