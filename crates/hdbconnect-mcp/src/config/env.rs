//! Environment variable loading for configuration

use std::env;
use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::time::Duration;

use url::Url;

use super::builder::{ConfigBuilder, TransportMode};
use crate::Result;
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
}

/// Load configuration from environment variables
pub fn load_from_env(mut builder: ConfigBuilder) -> Result<ConfigBuilder> {
    // Connection URL
    if let Ok(url_str) = env::var(vars::HANA_URL) {
        let mut url = Url::parse(&url_str)
            .map_err(|e| crate::Error::Config(format!("Invalid {}: {}", vars::HANA_URL, e)))?;

        // Optionally override user/password from separate env vars
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

    // Pool size
    if let Ok(size_str) = env::var(vars::HANA_POOL_SIZE)
        && let Ok(size) = size_str.parse::<usize>()
        && let Some(nz) = NonZeroUsize::new(size)
    {
        builder = builder.pool_size(nz);
    }

    // Read-only mode
    if let Ok(val) = env::var(vars::MCP_READ_ONLY) {
        builder = builder.read_only(parse_bool(&val));
    }

    // Row limit
    if let Ok(limit_str) = env::var(vars::MCP_ROW_LIMIT)
        && let Ok(limit) = limit_str.parse::<u32>()
    {
        builder = builder.row_limit(NonZeroU32::new(limit));
    }

    // Query timeout
    if let Ok(timeout_str) = env::var(vars::MCP_QUERY_TIMEOUT_SECS)
        && let Ok(secs) = timeout_str.parse::<u64>()
    {
        builder = builder.query_timeout(Duration::from_secs(secs));
    }

    // Schema filter
    if let Ok(mode) = env::var(vars::MCP_SCHEMA_FILTER_MODE) {
        let schemas: Vec<String> = env::var(vars::MCP_SCHEMA_FILTER_SCHEMAS)
            .map(|s| s.split(',').map(|s| s.trim().to_uppercase()).collect())
            .unwrap_or_default();

        let filter = SchemaFilter::from_config(&mode, &schemas)?;
        builder = builder.schema_filter(filter);
    }

    // Transport
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

    // Telemetry
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

    Ok(builder)
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
}
