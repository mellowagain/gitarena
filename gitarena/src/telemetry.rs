use std::env;

use anyhow::{Result, bail};
use opentelemetry::global;
use opentelemetry_otlp::Protocol as OtelProtocol;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
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
    #[must_use]
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

#[derive(Clone, Copy)]
enum Protocol {
    Grpc,
    HttpProtobuf,
    HttpJson,
}

fn is_configured() -> bool {
    env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_some()
        || env::var_os("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_some()
        || env::var_os("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT").is_some()
        || env::var_os("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT").is_some()
}

fn protocol() -> Result<Protocol> {
    let value = env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or_else(|_| "http/protobuf".to_owned());

    match value.as_str() {
        "grpc" => Ok(Protocol::Grpc),
        "http/protobuf" => Ok(Protocol::HttpProtobuf),
        "http/json" => Ok(Protocol::HttpJson),
        other => bail!("unsupported OTEL_EXPORTER_OTLP_PROTOCOL value: {other:?} (expected \"grpc\", \"http/protobuf\", or \"http/json\")"),
    }
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

    let protocol = protocol()?;

    let resource = Resource::builder()
        .with_service_name(env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| service_name.to_owned()))
        .build();

    let span_exporter = match protocol {
        Protocol::Grpc => SpanExporter::builder().with_tonic().with_protocol(OtelProtocol::Grpc).build()?,
        Protocol::HttpProtobuf => SpanExporter::builder().with_http().with_protocol(OtelProtocol::HttpBinary).build()?,
        Protocol::HttpJson => SpanExporter::builder().with_http().with_protocol(OtelProtocol::HttpJson).build()?,
    };

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_span_processor(BatchSpanProcessor::builder(span_exporter).build())
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    let metric_exporter = match protocol {
        Protocol::Grpc => MetricExporter::builder().with_tonic().with_protocol(OtelProtocol::Grpc).build()?,
        Protocol::HttpProtobuf => MetricExporter::builder().with_http().with_protocol(OtelProtocol::HttpBinary).build()?,
        Protocol::HttpJson => MetricExporter::builder().with_http().with_protocol(OtelProtocol::HttpJson).build()?,
    };

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_reader(PeriodicReader::builder(metric_exporter).build())
        .build();

    global::set_meter_provider(meter_provider.clone());

    let log_exporter = match protocol {
        Protocol::Grpc => LogExporter::builder().with_tonic().with_protocol(OtelProtocol::Grpc).build()?,
        Protocol::HttpProtobuf => LogExporter::builder().with_http().with_protocol(OtelProtocol::HttpBinary).build()?,
        Protocol::HttpJson => LogExporter::builder().with_http().with_protocol(OtelProtocol::HttpJson).build()?,
    };

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
