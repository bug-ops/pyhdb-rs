//! Observability module for tracing, metrics, and logging

#[cfg(feature = "telemetry")]
mod telemetry;

#[cfg(feature = "telemetry")]
pub use telemetry::init_telemetry;

use crate::Result;
use crate::config::TelemetryConfig;

/// Initialize observability stack
pub fn init_observability(config: &TelemetryConfig) -> Result<()> {
    #[cfg(feature = "telemetry")]
    {
        init_telemetry(config)?;
    }

    #[cfg(not(feature = "telemetry"))]
    {
        init_basic_logging(config);
    }

    Ok(())
}

/// Initialize basic logging without OpenTelemetry
#[cfg(not(feature = "telemetry"))]
fn init_basic_logging(config: &TelemetryConfig) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Layer};

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let fmt_layer = if config.json_logs {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer().with_target(true).boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

/// Shutdown observability stack
#[allow(clippy::missing_const_for_fn)]
pub fn shutdown_observability() {
    #[cfg(feature = "telemetry")]
    {
        telemetry::shutdown_telemetry();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(config.otlp_endpoint.is_none());
        assert!(config.service_name.is_empty());
        assert!(config.log_level.is_empty());
        assert!(!config.json_logs);
    }

    #[test]
    fn test_telemetry_config_with_values() {
        let config = TelemetryConfig {
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            service_name: "test-service".to_string(),
            log_level: "debug".to_string(),
            json_logs: true,
        };

        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.log_level, "debug");
        assert!(config.json_logs);
    }

    #[test]
    fn test_shutdown_observability_no_panic() {
        shutdown_observability();
    }
}
