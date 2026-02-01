//! TOML configuration file loading

use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use url::Url;

use super::builder::{ConfigBuilder, TransportMode};
use crate::Result;
use crate::security::SchemaFilter;

/// Configuration file locations checked in order
const CONFIG_PATHS: &[&str] = &[
    "./hdbconnect-mcp.toml",
    "~/.config/hdbconnect-mcp/config.toml",
    "/etc/hdbconnect-mcp/config.toml",
];

/// Find the first existing configuration file
pub fn find_config_file() -> Option<PathBuf> {
    for path_str in CONFIG_PATHS {
        let path = if path_str.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(path_str.replacen('~', &home, 1))
            } else {
                continue;
            }
        } else {
            PathBuf::from(path_str)
        };

        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Load configuration from a TOML file
pub fn load_from_file(path: &Path, mut builder: ConfigBuilder) -> Result<ConfigBuilder> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        crate::Error::Config(format!(
            "Failed to read config file {}: {}",
            path.display(),
            e
        ))
    })?;

    let file_config: FileConfig = toml::from_str(&content).map_err(|e| {
        crate::Error::Config(format!(
            "Failed to parse config file {}: {}",
            path.display(),
            e
        ))
    })?;

    builder = apply_file_config(builder, file_config)?;
    Ok(builder)
}

fn apply_file_config(mut builder: ConfigBuilder, config: FileConfig) -> Result<ConfigBuilder> {
    // Connection settings
    if let Some(conn) = config.connection {
        if let Some(url_str) = conn.url {
            let url = Url::parse(&url_str)
                .map_err(|e| crate::Error::Config(format!("Invalid connection URL: {e}")))?;
            builder = builder.connection_url(url);
        }

        if let Some(size) = conn.pool_size
            && let Some(nz) = NonZeroUsize::new(size)
        {
            builder = builder.pool_size(nz);
        }
    }

    // Security settings
    if let Some(sec) = config.security {
        if let Some(read_only) = sec.read_only {
            builder = builder.read_only(read_only);
        }

        if let Some(limit) = sec.row_limit {
            builder = builder.row_limit(NonZeroU32::new(limit));
        }

        if let Some(timeout) = sec.query_timeout_secs {
            builder = builder.query_timeout(Duration::from_secs(timeout));
        }

        if let Some(filter_config) = sec.schema_filter {
            let schemas: Vec<String> = filter_config
                .schemas
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.to_uppercase())
                .collect();

            let mode = filter_config.mode.as_deref().unwrap_or("none");
            let filter = SchemaFilter::from_config(mode, &schemas)?;
            builder = builder.schema_filter(filter);
        }
    }

    // Transport settings
    if let Some(transport) = config.transport {
        if let Some(mode_str) = transport.mode {
            let mode: TransportMode = mode_str.parse().unwrap_or_default();
            builder = builder.transport_mode(mode);
        }

        if let Some(host_str) = transport.http_host
            && let Ok(host) = host_str.parse::<IpAddr>()
        {
            builder = builder.http_host(host);
        }

        if let Some(port) = transport.http_port {
            builder = builder.http_port(port);
        }
    }

    // Observability settings
    if let Some(obs) = config.observability {
        if let Some(endpoint) = obs.otlp_endpoint {
            builder = builder.otlp_endpoint(Some(endpoint));
        }

        if let Some(name) = obs.service_name {
            builder = builder.service_name(name);
        }

        if let Some(level) = obs.log_level {
            builder = builder.log_level(level);
        }

        if let Some(json) = obs.json_logs {
            builder = builder.json_logs(json);
        }
    }

    Ok(builder)
}

/// Root configuration file structure
#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    connection: Option<ConnectionConfig>,
    security: Option<SecurityConfig>,
    transport: Option<TransportFileConfig>,
    observability: Option<ObservabilityConfig>,
}

#[derive(Debug, Deserialize)]
struct ConnectionConfig {
    url: Option<String>,
    pool_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SecurityConfig {
    read_only: Option<bool>,
    row_limit: Option<u32>,
    query_timeout_secs: Option<u64>,
    schema_filter: Option<SchemaFilterConfig>,
}

#[derive(Debug, Deserialize)]
struct SchemaFilterConfig {
    mode: Option<String>,
    schemas: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TransportFileConfig {
    mode: Option<String>,
    http_host: Option<String>,
    http_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct ObservabilityConfig {
    otlp_endpoint: Option<String>,
    service_name: Option<String>,
    log_level: Option<String>,
    json_logs: Option<bool>,
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn create_temp_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_parse_full_config() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"
pool_size = 8

[security]
read_only = true
row_limit = 5000
query_timeout_secs = 60

[security.schema_filter]
mode = "whitelist"
schemas = ["SCHEMA1", "SCHEMA2"]

[transport]
mode = "http"
http_host = "0.0.0.0"
http_port = 9090

[observability]
otlp_endpoint = "http://localhost:4317"
service_name = "test-mcp"
log_level = "debug"
json_logs = true
"#;

        let config: FileConfig = toml::from_str(toml_content).unwrap();

        assert!(config.connection.is_some());
        assert!(config.security.is_some());
        assert!(config.transport.is_some());
        assert!(config.observability.is_some());

        let conn = config.connection.unwrap();
        assert_eq!(
            conn.url,
            Some("hdbsql://user:pass@localhost:30015".to_string())
        );
        assert_eq!(conn.pool_size, Some(8));

        let sec = config.security.unwrap();
        assert_eq!(sec.read_only, Some(true));
        assert_eq!(sec.row_limit, Some(5000));
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"
"#;

        let config: FileConfig = toml::from_str(toml_content).unwrap();
        assert!(config.connection.is_some());
        assert!(config.security.is_none());
        assert!(config.transport.is_none());
    }

    #[test]
    fn test_load_from_file_success() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"
pool_size = 16

[security]
read_only = false
row_limit = 20000
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert_eq!(
            config.connection_url.as_str(),
            "hdbsql://user:pass@localhost:30015"
        );
        assert_eq!(config.pool_size.get(), 16);
        assert!(!config.read_only);
        assert_eq!(config.row_limit, NonZeroU32::new(20000));
    }

    #[test]
    fn test_load_from_file_not_found() {
        let result = load_from_file(
            Path::new("/nonexistent/path/config.toml"),
            ConfigBuilder::new(),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to read config file"));
    }

    #[test]
    fn test_load_from_file_invalid_toml() {
        let temp_file = create_temp_config("this is not valid toml {{{{");

        let result = load_from_file(temp_file.path(), ConfigBuilder::new());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse config file"));
    }

    #[test]
    fn test_load_from_file_invalid_url() {
        let toml_content = r#"
[connection]
url = "not a valid url"
"#;
        let temp_file = create_temp_config(toml_content);

        let result = load_from_file(temp_file.path(), ConfigBuilder::new());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid connection URL"));
    }

    #[test]
    fn test_load_transport_config() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"

[transport]
mode = "http"
http_host = "192.168.1.1"
http_port = 8888
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert_eq!(config.transport.mode, TransportMode::Http);
        assert_eq!(
            config.transport.http_host,
            "192.168.1.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(config.transport.http_port, 8888);
    }

    #[test]
    fn test_load_invalid_http_host_ignored() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"

[transport]
http_host = "not_an_ip"
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert_eq!(
            config.transport.http_host,
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_load_observability_config() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"

[observability]
otlp_endpoint = "http://jaeger:4317"
service_name = "my-service"
log_level = "trace"
json_logs = true
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert_eq!(
            config.telemetry.otlp_endpoint,
            Some("http://jaeger:4317".to_string())
        );
        assert_eq!(config.telemetry.service_name, "my-service");
        assert_eq!(config.telemetry.log_level, "trace");
        assert!(config.telemetry.json_logs);
    }

    #[test]
    fn test_load_schema_filter_blacklist() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"

[security.schema_filter]
mode = "blacklist"
schemas = ["SYS", "SYSTEM"]
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        match config.schema_filter {
            SchemaFilter::Blacklist(schemas) => {
                assert!(schemas.contains("SYS"));
                assert!(schemas.contains("SYSTEM"));
            }
            _ => panic!("Expected Blacklist filter"),
        }
    }

    #[test]
    fn test_load_schema_filter_none() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"

[security.schema_filter]
mode = "none"
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert!(matches!(config.schema_filter, SchemaFilter::AllowAll));
    }

    #[test]
    fn test_load_zero_pool_size_ignored() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"
pool_size = 0
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert_eq!(config.pool_size.get(), 4);
    }

    #[test]
    fn test_load_query_timeout() {
        let toml_content = r#"
[connection]
url = "hdbsql://user:pass@localhost:30015"

[security]
query_timeout_secs = 300
"#;
        let temp_file = create_temp_config(toml_content);

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let config = builder.build().unwrap();

        assert_eq!(config.query_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_empty_config_file() {
        let temp_file = create_temp_config("");

        let builder = load_from_file(temp_file.path(), ConfigBuilder::new()).unwrap();
        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_find_config_file_not_found() {
        let result = find_config_file();
        assert!(result.is_none() || result.unwrap().exists());
    }
}
