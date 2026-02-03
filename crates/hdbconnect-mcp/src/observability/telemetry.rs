//! OpenTelemetry initialization

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::Result;
use crate::config::TelemetryConfig;

static TRACER_PROVIDER: std::sync::OnceLock<SdkTracerProvider> = std::sync::OnceLock::new();

/// Initialize OpenTelemetry tracing
pub fn init_telemetry(config: &TelemetryConfig) -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Write logs to stderr to avoid mixing with JSON-RPC on stdout
    let fmt_layer = if config.json_logs {
        tracing_subscriber::fmt::layer()
            .json()
            .with_writer(std::io::stderr)
            .with_target(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_target(true)
            .boxed()
    };

    // Build subscriber with optional OTLP layer
    if let Some(ref endpoint) = config.otlp_endpoint {
        let resource = Resource::builder()
            .with_service_name(config.service_name.clone())
            .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
            .build();

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
            .map_err(|e| crate::Error::Config(format!("Failed to create OTLP exporter: {e}")))?;

        let provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build();

        let tracer = provider.tracer(config.service_name.clone());

        // Store provider for shutdown
        let _ = TRACER_PROVIDER.set(provider);

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();

        tracing::info!("OpenTelemetry initialized with endpoint: {}", endpoint);
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();

        tracing::debug!("OpenTelemetry disabled (no endpoint configured)");
    }

    Ok(())
}

/// Shutdown OpenTelemetry and flush pending traces
pub fn shutdown_telemetry() {
    if let Some(provider) = TRACER_PROVIDER.get()
        && let Err(e) = provider.shutdown()
    {
        tracing::warn!("Failed to shutdown tracer provider: {e:?}");
    }
}
