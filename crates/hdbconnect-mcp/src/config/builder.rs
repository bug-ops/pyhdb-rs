//! Configuration builder

use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::str::FromStr;
use std::time::Duration;

use url::Url;

use super::dml::{AllowedOperations, DmlConfig};
use super::procedure::ProcedureConfig;
use crate::Error;
#[cfg(feature = "cache")]
use crate::cache::{CacheBackend, CacheConfig, CacheTtlConfig};
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
    pub dml: DmlConfig,
    pub procedure: ProcedureConfig,
    #[cfg(feature = "cache")]
    pub cache: CacheConfig,
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

    #[must_use]
    pub const fn dml(&self) -> &DmlConfig {
        &self.dml
    }

    #[must_use]
    pub const fn procedure(&self) -> &ProcedureConfig {
        &self.procedure
    }

    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache(&self) -> &CacheConfig {
        &self.cache
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
    dml: DmlConfig,
    procedure: ProcedureConfig,
    #[cfg(feature = "cache")]
    cache: CacheConfig,
}

impl ConfigBuilder {
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
            dml: DmlConfig::new(),
            procedure: ProcedureConfig::new(),
            #[cfg(feature = "cache")]
            cache: CacheConfig::new(),
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

    // DML configuration methods

    /// Enable DML operations (disabled by default)
    #[must_use]
    pub const fn allow_dml(mut self, allow: bool) -> Self {
        self.dml.allow_dml = allow;
        self
    }

    /// Require confirmation before DML execution
    #[must_use]
    pub const fn require_dml_confirmation(mut self, require: bool) -> Self {
        self.dml.require_confirmation = require;
        self
    }

    /// Set maximum affected rows limit
    #[must_use]
    pub const fn max_affected_rows(mut self, limit: Option<NonZeroU32>) -> Self {
        self.dml.max_affected_rows = limit;
        self
    }

    /// Require WHERE clause for UPDATE/DELETE
    #[must_use]
    pub const fn require_where_clause(mut self, require: bool) -> Self {
        self.dml.require_where_clause = require;
        self
    }

    /// Set allowed DML operations
    #[must_use]
    pub const fn allowed_operations(mut self, ops: AllowedOperations) -> Self {
        self.dml.allowed_operations = ops;
        self
    }

    // Procedure configuration methods

    /// Enable stored procedure execution (disabled by default)
    #[must_use]
    pub const fn allow_procedures(mut self, allow: bool) -> Self {
        self.procedure.allow_procedures = allow;
        self
    }

    /// Require confirmation before procedure execution
    #[must_use]
    pub const fn require_procedure_confirmation(mut self, require: bool) -> Self {
        self.procedure.require_confirmation = require;
        self
    }

    /// Set maximum result sets from procedures
    #[must_use]
    pub const fn max_result_sets(mut self, limit: Option<NonZeroU32>) -> Self {
        self.procedure.max_result_sets = limit;
        self
    }

    /// Set maximum rows per result set
    #[must_use]
    pub const fn max_rows_per_result_set(mut self, limit: Option<NonZeroU32>) -> Self {
        self.procedure.max_rows_per_result_set = limit;
        self
    }

    // Cache configuration methods (only available with cache feature)

    /// Enable caching (disabled by default)
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache.enabled = enabled;
        self
    }

    /// Set cache backend type
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_backend(mut self, backend: CacheBackend) -> Self {
        self.cache.backend = backend;
        self
    }

    /// Set cache TTL configuration
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_ttl(mut self, ttl: CacheTtlConfig) -> Self {
        self.cache.ttl = ttl;
        self
    }

    /// Set default cache TTL
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_default_ttl(mut self, ttl: Duration) -> Self {
        self.cache.ttl.default = ttl;
        self
    }

    /// Set schema metadata cache TTL
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_schema_ttl(mut self, ttl: Duration) -> Self {
        self.cache.ttl.schema = ttl;
        self
    }

    /// Set query results cache TTL
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_query_ttl(mut self, ttl: Duration) -> Self {
        self.cache.ttl.query = ttl;
        self
    }

    /// Set maximum cache entries
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_max_entries(mut self, max: Option<usize>) -> Self {
        self.cache.max_entries = max;
        self
    }

    /// Set maximum value size for cache entries (default: 1MB)
    #[cfg(feature = "cache")]
    #[must_use]
    pub const fn cache_max_value_size(mut self, max: usize) -> Self {
        self.cache.max_value_size = max;
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

        // Apply default for DML max_affected_rows
        let dml = DmlConfig {
            max_affected_rows: self.dml.max_affected_rows.or(NonZeroU32::new(1000)),
            ..self.dml
        };

        // Apply defaults for procedure config
        let procedure = ProcedureConfig {
            max_result_sets: self.procedure.max_result_sets.or(NonZeroU32::new(10)),
            max_rows_per_result_set: self
                .procedure
                .max_rows_per_result_set
                .or(NonZeroU32::new(1000)),
            ..self.procedure
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
            dml,
            procedure,
            #[cfg(feature = "cache")]
            cache: self.cache,
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
    use std::collections::HashSet;

    use super::*;
    #[cfg(feature = "cache")]
    use crate::cache::DEFAULT_MAX_VALUE_SIZE;

    #[test]
    fn test_builder_defaults() {
        let builder = ConfigBuilder::new();
        assert!(builder.read_only);
        assert_eq!(builder.query_timeout, Duration::from_secs(30));
        assert!(!builder.dml.allow_dml);
        assert!(builder.dml.require_confirmation);
        assert!(builder.dml.require_where_clause);
        assert!(!builder.procedure.allow_procedures);
        assert!(builder.procedure.require_confirmation);
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_builder_cache_defaults() {
        let builder = ConfigBuilder::new();
        assert!(!builder.cache.enabled);
        assert_eq!(builder.cache.backend, CacheBackend::Noop);
        assert_eq!(builder.cache.max_value_size, DEFAULT_MAX_VALUE_SIZE);
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

    #[test]
    fn test_builder_dml_config() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .allow_dml(true)
            .require_dml_confirmation(false)
            .max_affected_rows(NonZeroU32::new(500))
            .require_where_clause(false)
            .build()
            .unwrap();

        assert!(config.dml.allow_dml);
        assert!(!config.dml.require_confirmation);
        assert_eq!(config.dml.max_affected_rows, NonZeroU32::new(500));
        assert!(!config.dml.require_where_clause);
    }

    #[test]
    fn test_builder_dml_default_max_rows() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new().connection_url(url).build().unwrap();

        assert_eq!(config.dml.max_affected_rows, NonZeroU32::new(1000));
    }

    #[test]
    fn test_builder_allowed_operations() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let ops = AllowedOperations {
            insert: true,
            update: false,
            delete: false,
        };
        let config = ConfigBuilder::new()
            .connection_url(url)
            .allowed_operations(ops)
            .build()
            .unwrap();

        assert!(config.dml.allowed_operations.insert);
        assert!(!config.dml.allowed_operations.update);
        assert!(!config.dml.allowed_operations.delete);
    }

    #[test]
    fn test_builder_procedure_config() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .allow_procedures(true)
            .require_procedure_confirmation(false)
            .max_result_sets(NonZeroU32::new(5))
            .max_rows_per_result_set(NonZeroU32::new(500))
            .build()
            .unwrap();

        assert!(config.procedure.allow_procedures);
        assert!(!config.procedure.require_confirmation);
        assert_eq!(config.procedure.max_result_sets, NonZeroU32::new(5));
        assert_eq!(
            config.procedure.max_rows_per_result_set,
            NonZeroU32::new(500)
        );
    }

    #[test]
    fn test_builder_procedure_default_limits() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new().connection_url(url).build().unwrap();

        assert_eq!(config.procedure.max_result_sets, NonZeroU32::new(10));
        assert_eq!(
            config.procedure.max_rows_per_result_set,
            NonZeroU32::new(1000)
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_builder_cache_config() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .cache_enabled(true)
            .cache_backend(CacheBackend::Memory)
            .cache_default_ttl(Duration::from_secs(600))
            .cache_schema_ttl(Duration::from_secs(7200))
            .cache_query_ttl(Duration::from_secs(120))
            .cache_max_entries(Some(5000))
            .cache_max_value_size(2_000_000)
            .build()
            .unwrap();

        assert!(config.cache.enabled);
        assert_eq!(config.cache.backend, CacheBackend::Memory);
        assert_eq!(config.cache.ttl.default, Duration::from_secs(600));
        assert_eq!(config.cache.ttl.schema, Duration::from_secs(7200));
        assert_eq!(config.cache.ttl.query, Duration::from_secs(120));
        assert_eq!(config.cache.max_entries, Some(5000));
        assert_eq!(config.cache.max_value_size, 2_000_000);
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_builder_cache_defaults_in_config() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new().connection_url(url).build().unwrap();

        assert!(!config.cache.enabled);
        assert_eq!(config.cache.backend, CacheBackend::Noop);
        assert_eq!(config.cache.ttl.default, Duration::from_secs(300));
        assert_eq!(config.cache.ttl.schema, Duration::from_secs(3600));
        assert_eq!(config.cache.ttl.query, Duration::from_secs(60));
        assert_eq!(config.cache.max_entries, Some(10000));
        assert_eq!(config.cache.max_value_size, DEFAULT_MAX_VALUE_SIZE);
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_config_cache_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .cache_enabled(true)
            .build()
            .unwrap();

        assert!(config.cache().enabled);
    }

    #[test]
    fn test_config_read_only_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .read_only(false)
            .build()
            .unwrap();

        assert!(!config.read_only());
    }

    #[test]
    fn test_config_row_limit_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .row_limit(NonZeroU32::new(500))
            .build()
            .unwrap();

        assert_eq!(config.row_limit(), NonZeroU32::new(500));
    }

    #[test]
    fn test_config_query_timeout_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .query_timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        assert_eq!(config.query_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_config_schema_filter_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let allowed: HashSet<String> = ["APP", "PUBLIC"].iter().map(|s| (*s).to_string()).collect();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .schema_filter(SchemaFilter::Whitelist(allowed))
            .build()
            .unwrap();

        assert!(matches!(config.schema_filter(), SchemaFilter::Whitelist(_)));
    }

    #[test]
    fn test_config_dml_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .allow_dml(true)
            .build()
            .unwrap();

        assert!(config.dml().allow_dml);
    }

    #[test]
    fn test_config_procedure_accessor() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .allow_procedures(true)
            .build()
            .unwrap();

        assert!(config.procedure().allow_procedures);
    }

    #[test]
    fn test_config_builder_static_method() {
        let builder = Config::builder();
        assert!(builder.read_only);
    }

    #[test]
    fn test_transport_config_default() {
        let transport = TransportConfig::default();
        assert_eq!(transport.mode, TransportMode::Stdio);
        assert_eq!(transport.http_port, 8080);
    }

    #[test]
    fn test_transport_mode_default() {
        let mode = TransportMode::default();
        assert_eq!(mode, TransportMode::Stdio);
    }

    #[test]
    fn test_builder_pool_size() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .pool_size(NonZeroUsize::new(8).unwrap())
            .build()
            .unwrap();

        assert_eq!(config.pool_size.get(), 8);
    }

    #[test]
    fn test_builder_transport_mode() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .transport_mode(TransportMode::Http)
            .build()
            .unwrap();

        assert_eq!(config.transport.mode, TransportMode::Http);
    }

    #[test]
    fn test_builder_http_host_and_port() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .http_host(IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)))
            .http_port(9000)
            .build()
            .unwrap();

        assert_eq!(
            config.transport.http_host,
            IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))
        );
        assert_eq!(config.transport.http_port, 9000);
    }

    #[test]
    fn test_builder_telemetry_config() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new()
            .connection_url(url)
            .otlp_endpoint(Some("http://localhost:4317".to_string()))
            .service_name("test-service".to_string())
            .log_level("debug".to_string())
            .json_logs(true)
            .build()
            .unwrap();

        assert_eq!(
            config.telemetry.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert_eq!(config.telemetry.service_name, "test-service");
        assert_eq!(config.telemetry.log_level, "debug");
        assert!(config.telemetry.json_logs);
    }

    #[test]
    fn test_builder_telemetry_defaults() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new().connection_url(url).build().unwrap();

        assert!(config.telemetry.otlp_endpoint.is_none());
        assert_eq!(config.telemetry.service_name, "hdbconnect-mcp");
        assert_eq!(config.telemetry.log_level, "info");
        assert!(!config.telemetry.json_logs);
    }

    #[test]
    fn test_telemetry_config_default() {
        let telemetry = TelemetryConfig::default();
        assert!(telemetry.otlp_endpoint.is_none());
        assert!(telemetry.service_name.is_empty());
        assert!(telemetry.log_level.is_empty());
        assert!(!telemetry.json_logs);
    }

    #[test]
    fn test_config_builder_default() {
        let builder1 = ConfigBuilder::new();
        let builder2 = ConfigBuilder::default();

        assert_eq!(builder1.read_only, builder2.read_only);
        assert_eq!(builder1.query_timeout, builder2.query_timeout);
    }

    #[test]
    fn test_config_debug() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let config = ConfigBuilder::new().connection_url(url).build().unwrap();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("Config"));
    }

    #[test]
    fn test_transport_config_debug() {
        let transport = TransportConfig::default();
        let debug_str = format!("{transport:?}");
        assert!(debug_str.contains("TransportConfig"));
    }

    #[test]
    fn test_transport_mode_debug() {
        let mode = TransportMode::Http;
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("Http"));
    }

    #[test]
    fn test_builder_debug() {
        let builder = ConfigBuilder::new();
        let debug_str = format!("{builder:?}");
        assert!(debug_str.contains("ConfigBuilder"));
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_builder_cache_ttl() {
        let url = Url::parse("hdbsql://user:pass@localhost:30015").unwrap();
        let ttl = CacheTtlConfig {
            default: Duration::from_secs(100),
            schema: Duration::from_secs(200),
            query: Duration::from_secs(300),
        };
        let config = ConfigBuilder::new()
            .connection_url(url)
            .cache_ttl(ttl)
            .build()
            .unwrap();

        assert_eq!(config.cache.ttl.default, Duration::from_secs(100));
        assert_eq!(config.cache.ttl.schema, Duration::from_secs(200));
        assert_eq!(config.cache.ttl.query, Duration::from_secs(300));
    }
}
