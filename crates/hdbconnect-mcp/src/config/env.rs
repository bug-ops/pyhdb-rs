//! Environment variable loading for configuration

use std::env;
use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::str::FromStr;
use std::time::Duration;

use url::Url;

use super::builder::{ConfigBuilder, TransportMode};
use super::dml::AllowedOperations;
use crate::Result;
#[cfg(feature = "cache")]
use crate::cache::CacheBackend;
use crate::security::SchemaFilter;

/// Environment variable names
mod vars {
    pub const HANA_URL: &str = "HANA_URL";
    pub const HANA_USER: &str = "HANA_USER";
    pub const HANA_PASSWORD: &str = "HANA_PASSWORD";
    pub const HANA_POOL_SIZE: &str = "HANA_POOL_SIZE";
    pub const MCP_READ_ONLY: &str = "MCP_READ_ONLY";
    pub const MCP_ROW_LIMIT: &str = "MCP_ROW_LIMIT";
    pub const MCP_QUERY_TIMEOUT_SECS: &str = "MCP_QUERY_TIMEOUT_SECS";
    pub const MCP_SCHEMA_FILTER_MODE: &str = "MCP_SCHEMA_FILTER_MODE";
    pub const MCP_SCHEMA_FILTER_SCHEMAS: &str = "MCP_SCHEMA_FILTER_SCHEMAS";
    pub const MCP_TRANSPORT: &str = "MCP_TRANSPORT";
    pub const MCP_HTTP_HOST: &str = "MCP_HTTP_HOST";
    pub const MCP_HTTP_PORT: &str = "MCP_HTTP_PORT";
    pub const OTEL_EXPORTER_OTLP_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";
    pub const OTEL_SERVICE_NAME: &str = "OTEL_SERVICE_NAME";
    pub const RUST_LOG: &str = "RUST_LOG";
    pub const MCP_JSON_LOGS: &str = "MCP_JSON_LOGS";
    // DML configuration
    pub const HANA_ALLOW_DML: &str = "HANA_ALLOW_DML";
    pub const HANA_DML_CONFIRM: &str = "HANA_DML_CONFIRM";
    pub const HANA_DML_MAX_ROWS: &str = "HANA_DML_MAX_ROWS";
    pub const HANA_DML_REQUIRE_WHERE: &str = "HANA_DML_REQUIRE_WHERE";
    pub const HANA_DML_OPERATIONS: &str = "HANA_DML_OPERATIONS";
    // Procedure configuration
    pub const HANA_ALLOW_PROCEDURES: &str = "HANA_ALLOW_PROCEDURES";
    pub const HANA_PROCEDURE_CONFIRM: &str = "HANA_PROCEDURE_CONFIRM";
    pub const HANA_PROCEDURE_MAX_RESULT_SETS: &str = "HANA_PROCEDURE_MAX_RESULT_SETS";
    pub const HANA_PROCEDURE_MAX_ROWS: &str = "HANA_PROCEDURE_MAX_ROWS";
    // Cache configuration
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_ENABLED: &str = "MCP_CACHE_ENABLED";
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_BACKEND: &str = "MCP_CACHE_BACKEND";
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_DEFAULT_TTL_SECS: &str = "MCP_CACHE_DEFAULT_TTL_SECS";
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_MAX_ENTRIES: &str = "MCP_CACHE_MAX_ENTRIES";
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_MAX_VALUE_SIZE: &str = "MCP_CACHE_MAX_VALUE_SIZE";
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_SCHEMA_TTL_SECS: &str = "MCP_CACHE_SCHEMA_TTL_SECS";
    #[cfg(feature = "cache")]
    pub const MCP_CACHE_QUERY_TTL_SECS: &str = "MCP_CACHE_QUERY_TTL_SECS";
}

/// Load configuration from environment variables
pub fn load_from_env(builder: ConfigBuilder) -> Result<ConfigBuilder> {
    let builder = load_connection_config(builder)?;
    let builder = load_transport_config(builder);
    let builder = load_telemetry_config(builder);
    let builder = load_dml_config(builder);
    let builder = load_procedure_config(builder);
    #[cfg(feature = "cache")]
    let builder = load_cache_config(builder);
    Ok(builder)
}

fn load_connection_config(mut builder: ConfigBuilder) -> Result<ConfigBuilder> {
    if let Ok(url_str) = env::var(vars::HANA_URL) {
        let mut url = Url::parse(&url_str)
            .map_err(|e| crate::Error::Config(format!("Invalid {}: {}", vars::HANA_URL, e)))?;

        if let Ok(user) = env::var(vars::HANA_USER) {
            url.set_username(&user)
                .map_err(|()| crate::Error::Config("Failed to set username in URL".into()))?;
        }
        if let Ok(password) = env::var(vars::HANA_PASSWORD) {
            url.set_password(Some(&password))
                .map_err(|()| crate::Error::Config("Failed to set password in URL".into()))?;
        }

        builder = builder.connection_url(url);
    }

    if let Ok(size_str) = env::var(vars::HANA_POOL_SIZE)
        && let Ok(size) = size_str.parse::<usize>()
        && let Some(nz) = NonZeroUsize::new(size)
    {
        builder = builder.pool_size(nz);
    }

    if let Ok(val) = env::var(vars::MCP_READ_ONLY) {
        builder = builder.read_only(parse_bool(&val));
    }

    if let Ok(limit_str) = env::var(vars::MCP_ROW_LIMIT)
        && let Ok(limit) = limit_str.parse::<u32>()
    {
        builder = builder.row_limit(NonZeroU32::new(limit));
    }

    if let Ok(timeout_str) = env::var(vars::MCP_QUERY_TIMEOUT_SECS)
        && let Ok(secs) = timeout_str.parse::<u64>()
    {
        builder = builder.query_timeout(Duration::from_secs(secs));
    }

    if let Ok(mode) = env::var(vars::MCP_SCHEMA_FILTER_MODE) {
        let schemas: Vec<String> = env::var(vars::MCP_SCHEMA_FILTER_SCHEMAS)
            .map(|s| s.split(',').map(|s| s.trim().to_uppercase()).collect())
            .unwrap_or_default();

        let filter = SchemaFilter::from_config(&mode, &schemas)?;
        builder = builder.schema_filter(filter);
    }

    Ok(builder)
}

fn load_transport_config(mut builder: ConfigBuilder) -> ConfigBuilder {
    if let Ok(transport) = env::var(vars::MCP_TRANSPORT) {
        let mode: TransportMode = transport.parse().unwrap_or_default();
        builder = builder.transport_mode(mode);
    }

    if let Ok(host_str) = env::var(vars::MCP_HTTP_HOST)
        && let Ok(host) = host_str.parse::<IpAddr>()
    {
        builder = builder.http_host(host);
    }

    if let Ok(port_str) = env::var(vars::MCP_HTTP_PORT)
        && let Ok(port) = port_str.parse::<u16>()
    {
        builder = builder.http_port(port);
    }

    builder
}

fn load_telemetry_config(mut builder: ConfigBuilder) -> ConfigBuilder {
    if let Ok(endpoint) = env::var(vars::OTEL_EXPORTER_OTLP_ENDPOINT) {
        builder = builder.otlp_endpoint(Some(endpoint));
    }

    if let Ok(name) = env::var(vars::OTEL_SERVICE_NAME) {
        builder = builder.service_name(name);
    }

    if let Ok(level) = env::var(vars::RUST_LOG) {
        builder = builder.log_level(level);
    }

    if let Ok(val) = env::var(vars::MCP_JSON_LOGS) {
        builder = builder.json_logs(parse_bool(&val));
    }

    builder
}

fn load_dml_config(mut builder: ConfigBuilder) -> ConfigBuilder {
    if let Ok(val) = env::var(vars::HANA_ALLOW_DML) {
        builder = builder.allow_dml(parse_bool(&val));
    }

    if let Ok(val) = env::var(vars::HANA_DML_CONFIRM) {
        builder = builder.require_dml_confirmation(parse_bool(&val));
    }

    if let Ok(limit_str) = env::var(vars::HANA_DML_MAX_ROWS)
        && let Ok(limit) = limit_str.parse::<u32>()
    {
        builder = builder.max_affected_rows(NonZeroU32::new(limit));
    }

    if let Ok(val) = env::var(vars::HANA_DML_REQUIRE_WHERE) {
        builder = builder.require_where_clause(parse_bool(&val));
    }

    if let Ok(val) = env::var(vars::HANA_DML_OPERATIONS) {
        let ops = AllowedOperations::from_str(&val).unwrap_or_default();
        builder = builder.allowed_operations(ops);
    }

    builder
}

fn load_procedure_config(mut builder: ConfigBuilder) -> ConfigBuilder {
    if let Ok(val) = env::var(vars::HANA_ALLOW_PROCEDURES) {
        builder = builder.allow_procedures(parse_bool(&val));
    }

    if let Ok(val) = env::var(vars::HANA_PROCEDURE_CONFIRM) {
        builder = builder.require_procedure_confirmation(parse_bool(&val));
    }

    if let Ok(limit_str) = env::var(vars::HANA_PROCEDURE_MAX_RESULT_SETS)
        && let Ok(limit) = limit_str.parse::<u32>()
    {
        builder = builder.max_result_sets(NonZeroU32::new(limit));
    }

    if let Ok(limit_str) = env::var(vars::HANA_PROCEDURE_MAX_ROWS)
        && let Ok(limit) = limit_str.parse::<u32>()
    {
        builder = builder.max_rows_per_result_set(NonZeroU32::new(limit));
    }

    builder
}

#[cfg(feature = "cache")]
fn load_cache_config(mut builder: ConfigBuilder) -> ConfigBuilder {
    if let Ok(val) = env::var(vars::MCP_CACHE_ENABLED) {
        builder = builder.cache_enabled(parse_bool(&val));
    }

    if let Ok(val) = env::var(vars::MCP_CACHE_BACKEND) {
        let backend = CacheBackend::from_str(&val).unwrap_or_default();
        builder = builder.cache_backend(backend);
    }

    if let Ok(secs_str) = env::var(vars::MCP_CACHE_DEFAULT_TTL_SECS)
        && let Ok(secs) = secs_str.parse::<u64>()
    {
        if secs == 0 {
            tracing::warn!(
                "{}=0 disables TTL expiration, cache entries will never expire",
                vars::MCP_CACHE_DEFAULT_TTL_SECS
            );
        }
        builder = builder.cache_default_ttl(Duration::from_secs(secs));
    }

    if let Ok(max_str) = env::var(vars::MCP_CACHE_MAX_ENTRIES)
        && let Ok(max) = max_str.parse::<usize>()
    {
        if max == 0 {
            tracing::warn!(
                "{}=0 prevents any entries from being cached",
                vars::MCP_CACHE_MAX_ENTRIES
            );
        }
        builder = builder.cache_max_entries(Some(max));
    }

    if let Ok(max_str) = env::var(vars::MCP_CACHE_MAX_VALUE_SIZE)
        && let Ok(max) = max_str.parse::<usize>()
    {
        if max == 0 {
            tracing::warn!(
                "{}=0 prevents any values from being cached",
                vars::MCP_CACHE_MAX_VALUE_SIZE
            );
        }
        builder = builder.cache_max_value_size(max);
    }

    if let Ok(secs_str) = env::var(vars::MCP_CACHE_SCHEMA_TTL_SECS)
        && let Ok(secs) = secs_str.parse::<u64>()
    {
        if secs == 0 {
            tracing::warn!(
                "{}=0 disables TTL for schema cache entries",
                vars::MCP_CACHE_SCHEMA_TTL_SECS
            );
        }
        builder = builder.cache_schema_ttl(Duration::from_secs(secs));
    }

    if let Ok(secs_str) = env::var(vars::MCP_CACHE_QUERY_TTL_SECS)
        && let Ok(secs) = secs_str.parse::<u64>()
    {
        if secs == 0 {
            tracing::warn!(
                "{}=0 disables TTL for query result cache entries",
                vars::MCP_CACHE_QUERY_TTL_SECS
            );
        }
        builder = builder.cache_query_ttl(Duration::from_secs(secs));
    }

    builder
}

fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_env_vars<F, R>(vars: &[(&str, &str)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_MUTEX.lock().unwrap();

        let old_values: Vec<_> = vars.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();

        for (key, value) in vars {
            // SAFETY: We hold a mutex lock to ensure no concurrent modifications
            unsafe { env::set_var(key, value) };
        }

        let result = f();

        for (key, old_value) in old_values {
            match old_value {
                // SAFETY: We hold a mutex lock to ensure no concurrent modifications
                Some(v) => unsafe { env::set_var(key, v) },
                None => unsafe { env::remove_var(key) },
            }
        }

        result
    }

    fn clear_env_vars<F, R>(vars: &[&str], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_MUTEX.lock().unwrap();

        let old_values: Vec<_> = vars.iter().map(|k| (*k, env::var(k).ok())).collect();

        for key in vars {
            // SAFETY: We hold a mutex lock to ensure no concurrent modifications
            unsafe { env::remove_var(key) };
        }

        let result = f();

        for (key, old_value) in old_values {
            if let Some(v) = old_value {
                // SAFETY: We hold a mutex lock to ensure no concurrent modifications
                unsafe { env::set_var(key, v) };
            }
        }

        result
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("1"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("on"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool(""));
    }

    #[test]
    fn test_load_connection_url() {
        with_env_vars(
            &[("HANA_URL", "hdbsql://user:pass@localhost:30015")],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder
                    .build()
                    .expect("Should build with valid connection URL");
                assert_eq!(
                    config.connection_url.as_str(),
                    "hdbsql://user:pass@localhost:30015"
                );
            },
        );
    }

    #[test]
    fn test_load_invalid_url() {
        with_env_vars(&[("HANA_URL", "not a valid url")], || {
            let result = load_from_env(ConfigBuilder::new());
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_load_url_with_user_override() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://olduser:pass@localhost:30015"),
                ("HANA_USER", "newuser"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(config.connection_url.as_str().contains("newuser"));
            },
        );
    }

    #[test]
    fn test_load_pool_size() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_POOL_SIZE", "8"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.pool_size.get(), 8);
            },
        );
    }

    #[test]
    fn test_load_invalid_pool_size_ignored() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_POOL_SIZE", "not_a_number"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.pool_size.get(), 4);
            },
        );
    }

    #[test]
    fn test_load_zero_pool_size_ignored() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_POOL_SIZE", "0"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.pool_size.get(), 4);
            },
        );
    }

    #[test]
    fn test_load_read_only() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_READ_ONLY", "false"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(!config.read_only);
            },
        );
    }

    #[test]
    fn test_load_row_limit() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_ROW_LIMIT", "5000"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.row_limit, NonZeroU32::new(5000));
            },
        );
    }

    #[test]
    fn test_load_query_timeout() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_QUERY_TIMEOUT_SECS", "120"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.query_timeout, Duration::from_secs(120));
            },
        );
    }

    #[test]
    fn test_load_transport_mode_http() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_TRANSPORT", "http"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.transport.mode, TransportMode::Http);
            },
        );
    }

    #[test]
    fn test_load_http_host_and_port() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_HTTP_HOST", "0.0.0.0"),
                ("MCP_HTTP_PORT", "9090"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(
                    config.transport.http_host,
                    "0.0.0.0".parse::<IpAddr>().unwrap()
                );
                assert_eq!(config.transport.http_port, 9090);
            },
        );
    }

    #[test]
    fn test_load_invalid_http_host_ignored() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_HTTP_HOST", "not_an_ip"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(
                    config.transport.http_host,
                    "127.0.0.1".parse::<IpAddr>().unwrap()
                );
            },
        );
    }

    #[test]
    fn test_load_telemetry_config() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317"),
                ("OTEL_SERVICE_NAME", "test-service"),
                ("RUST_LOG", "debug"),
                ("MCP_JSON_LOGS", "true"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(
                    config.telemetry.otlp_endpoint,
                    Some("http://localhost:4317".to_string())
                );
                assert_eq!(config.telemetry.service_name, "test-service");
                assert_eq!(config.telemetry.log_level, "debug");
                assert!(config.telemetry.json_logs);
            },
        );
    }

    #[test]
    fn test_load_schema_filter_whitelist() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_SCHEMA_FILTER_MODE", "whitelist"),
                ("MCP_SCHEMA_FILTER_SCHEMAS", "SCHEMA1, schema2"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                match config.schema_filter {
                    SchemaFilter::Whitelist(schemas) => {
                        assert!(schemas.contains("SCHEMA1"));
                        assert!(schemas.contains("SCHEMA2"));
                    }
                    _ => panic!("Expected Whitelist filter"),
                }
            },
        );
    }

    #[test]
    fn test_load_no_env_vars() {
        clear_env_vars(
            &[
                "HANA_URL",
                "HANA_USER",
                "HANA_PASSWORD",
                "HANA_POOL_SIZE",
                "MCP_READ_ONLY",
                "MCP_ROW_LIMIT",
                "MCP_QUERY_TIMEOUT_SECS",
                "MCP_SCHEMA_FILTER_MODE",
                "MCP_TRANSPORT",
                "MCP_HTTP_HOST",
                "MCP_HTTP_PORT",
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let result = builder.build();
                assert!(result.is_err());
            },
        );
    }

    // DML configuration tests
    #[test]
    fn test_load_dml_allow() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_ALLOW_DML", "true"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(config.dml.allow_dml);
            },
        );
    }

    #[test]
    fn test_load_dml_confirm() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_DML_CONFIRM", "false"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(!config.dml.require_confirmation);
            },
        );
    }

    #[test]
    fn test_load_dml_max_rows() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_DML_MAX_ROWS", "500"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.dml.max_affected_rows, NonZeroU32::new(500));
            },
        );
    }

    #[test]
    fn test_load_dml_require_where() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_DML_REQUIRE_WHERE", "false"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(!config.dml.require_where_clause);
            },
        );
    }

    #[test]
    fn test_load_dml_operations() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_DML_OPERATIONS", "insert,update"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(config.dml.allowed_operations.insert);
                assert!(config.dml.allowed_operations.update);
                assert!(!config.dml.allowed_operations.delete);
            },
        );
    }

    // Procedure configuration tests
    #[test]
    fn test_load_procedure_allow() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_ALLOW_PROCEDURES", "true"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(config.procedure.allow_procedures);
            },
        );
    }

    #[test]
    fn test_load_procedure_confirm() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_PROCEDURE_CONFIRM", "false"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(!config.procedure.require_confirmation);
            },
        );
    }

    #[test]
    fn test_load_procedure_max_result_sets() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_PROCEDURE_MAX_RESULT_SETS", "5"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.procedure.max_result_sets, NonZeroU32::new(5));
            },
        );
    }

    #[test]
    fn test_load_procedure_max_rows() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("HANA_PROCEDURE_MAX_ROWS", "500"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(
                    config.procedure.max_rows_per_result_set,
                    NonZeroU32::new(500)
                );
            },
        );
    }

    // Cache configuration tests (only with cache feature)
    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_enabled() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_ENABLED", "true"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert!(config.cache.enabled);
            },
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_backend() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_BACKEND", "memory"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.cache.backend, CacheBackend::Memory);
            },
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_default_ttl() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_DEFAULT_TTL_SECS", "600"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.cache.ttl.default, Duration::from_secs(600));
            },
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_max_entries() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_MAX_ENTRIES", "5000"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.cache.max_entries, Some(5000));
            },
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_max_value_size() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_MAX_VALUE_SIZE", "2000000"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.cache.max_value_size, 2_000_000);
            },
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_schema_ttl() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_SCHEMA_TTL_SECS", "7200"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.cache.ttl.schema, Duration::from_secs(7200));
            },
        );
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_load_cache_query_ttl() {
        with_env_vars(
            &[
                ("HANA_URL", "hdbsql://user:pass@localhost:30015"),
                ("MCP_CACHE_QUERY_TTL_SECS", "120"),
            ],
            || {
                let builder = load_from_env(ConfigBuilder::new()).unwrap();
                let config = builder.build().unwrap();
                assert_eq!(config.cache.ttl.query, Duration::from_secs(120));
            },
        );
    }
}
