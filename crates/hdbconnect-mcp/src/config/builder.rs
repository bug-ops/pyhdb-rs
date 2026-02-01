//! Configuration builder

use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::str::FromStr;
use std::time::Duration;

use url::Url;

use crate::Error;
use crate::security::SchemaFilter;

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub connection_url: Url,
    pub pool_size: NonZeroUsize,
    pub read_only: bool,
    pub row_limit: Option<NonZeroU32>,
    pub query_timeout: Duration,
    pub schema_filter: SchemaFilter,
    pub transport: TransportConfig,
    pub telemetry: TelemetryConfig,
}

impl Config {
    #[must_use]
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    #[must_use]
    pub const fn row_limit(&self) -> Option<NonZeroU32> {
        self.row_limit
    }

    #[must_use]
    pub const fn query_timeout(&self) -> Duration {
        self.query_timeout
    }

    #[must_use]
    pub const fn schema_filter(&self) -> &SchemaFilter {
        &self.schema_filter
    }
}

/// Transport mode configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub mode: TransportMode,
    pub http_host: IpAddr,
    pub http_port: u16,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::Stdio,
            http_host: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            http_port: 8080,
        }
    }
}

/// Transport mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportMode {
    #[default]
    Stdio,
    Http,
}

impl FromStr for TransportMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "http" | "sse" => Self::Http,
            _ => Self::Stdio,
        })
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Default)]
pub struct TelemetryConfig {
    pub otlp_endpoint: Option<String>,
    pub service_name: String,
    pub log_level: String,
    pub json_logs: bool,
}

/// Configuration builder with fluent API
#[derive(Debug)]
pub struct ConfigBuilder {
    connection_url: Option<Url>,
    pool_size: NonZeroUsize,
    read_only: bool,
    row_limit: Option<NonZeroU32>,
    query_timeout: Duration,
    schema_filter: SchemaFilter,
    transport: TransportConfig,
    telemetry: TelemetryConfig,
}

impl ConfigBuilder {
    // Cannot use NonZeroUsize::new() in const context without unwrap, so we use MIN
    const DEFAULT_POOL_SIZE: NonZeroUsize = NonZeroUsize::MIN.saturating_add(3); // 4

    #[must_use]
    pub const fn new() -> Self {
        Self {
            connection_url: None,
            pool_size: Self::DEFAULT_POOL_SIZE,
            read_only: true,
            row_limit: None,
            query_timeout: Duration::from_secs(30),
            schema_filter: SchemaFilter::AllowAll,
            transport: TransportConfig {
                mode: TransportMode::Stdio,
                http_host: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                http_port: 8080,
            },
            telemetry: TelemetryConfig {
                otlp_endpoint: None,
                service_name: String::new(),
                log_level: String::new(),
                json_logs: false,
            },
        }
    }

    #[must_use]
    pub fn connection_url(mut self, url: Url) -> Self {
        self.connection_url = Some(url);
        self
    }

    #[must_use]
    pub const fn pool_size(mut self, size: NonZeroUsize) -> Self {
        self.pool_size = size;
        self
    }

    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub const fn row_limit(mut self, limit: Option<NonZeroU32>) -> Self {
        self.row_limit = limit;
        self
    }

    #[must_use]
    pub const fn query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    #[must_use]
    pub fn schema_filter(mut self, filter: SchemaFilter) -> Self {
        self.schema_filter = filter;
        self
    }

    #[must_use]
    pub const fn transport_mode(mut self, mode: TransportMode) -> Self {
        self.transport.mode = mode;
        self
    }

    #[must_use]
    pub const fn http_host(mut self, host: IpAddr) -> Self {
        self.transport.http_host = host;
        self
    }

    #[must_use]
    pub const fn http_port(mut self, port: u16) -> Self {
        self.transport.http_port = port;
        self
    }

    #[must_use]
    pub fn otlp_endpoint(mut self, endpoint: Option<String>) -> Self {
        self.telemetry.otlp_endpoint = endpoint;
        self
    }

    #[must_use]
    pub fn service_name(mut self, name: String) -> Self {
        self.telemetry.service_name = name;
        self
    }

    #[must_use]
    pub fn log_level(mut self, level: String) -> Self {
        self.telemetry.log_level = level;
        self
    }

    #[must_use]
    pub const fn json_logs(mut self, enabled: bool) -> Self {
        self.telemetry.json_logs = enabled;
        self
    }

    /// Build the configuration
    pub fn build(self) -> crate::Result<Config> {
        let connection_url = self
            .connection_url
            .ok_or_else(|| Error::Config("connection_url is required".into()))?;

        // Apply defaults for row_limit
        let row_limit = self.row_limit.or(NonZeroU32::new(10000));

        // Apply defaults for telemetry
        let service_name = if self.telemetry.service_name.is_empty() {
            "hdbconnect-mcp".to_string()
        } else {
            self.telemetry.service_name
        };

        let log_level = if self.telemetry.log_level.is_empty() {
            "info".to_string()
        } else {
            self.telemetry.log_level
        };

        Ok(Config {
            connection_url,
            pool_size: self.pool_size,
            read_only: self.read_only,
            row_limit,
            query_timeout: self.query_timeout,
            schema_filter: self.schema_filter,
            transport: self.transport,
            telemetry: TelemetryConfig {
                otlp_endpoint: self.telemetry.otlp_endpoint,
                service_name,
                log_level,
                json_logs: self.telemetry.json_logs,
            },
        })
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = ConfigBuilder::new();
        assert!(builder.read_only);
        assert_eq!(builder.query_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_builder_requires_url() {
        let result = ConfigBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_url() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new().connection_url(url).build().unwrap();

        assert!(config.read_only);
        assert_eq!(config.query_timeout, Duration::from_secs(30));
        assert_eq!(config.row_limit, NonZeroU32::new(10000));
    }

    #[test]
    fn test_transport_mode_parsing() {
        assert_eq!(
            "stdio".parse::<TransportMode>().unwrap(),
            TransportMode::Stdio
        );
        assert_eq!(
            "http".parse::<TransportMode>().unwrap(),
            TransportMode::Http
        );
        assert_eq!("sse".parse::<TransportMode>().unwrap(), TransportMode::Http);
        assert_eq!(
            "HTTP".parse::<TransportMode>().unwrap(),
            TransportMode::Http
        );
        assert_eq!(
            "unknown".parse::<TransportMode>().unwrap(),
            TransportMode::Stdio
        );
    }
}
