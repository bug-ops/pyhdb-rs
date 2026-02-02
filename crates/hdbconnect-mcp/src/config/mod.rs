//! Configuration management
//!
//! Supports configuration loading with precedence: env > file > CLI > defaults

mod builder;
mod dml;
mod env;
mod file;
mod procedure;
mod runtime;

pub use builder::{Config, ConfigBuilder, TelemetryConfig, TransportConfig, TransportMode};
pub use dml::{AllowedOperations, DmlConfig, DmlOperation};
pub use procedure::ProcedureConfig;
pub use runtime::{ReloadResult, ReloadTrigger, RuntimeConfig, RuntimeConfigHolder};

use crate::Result;

/// Load configuration with precedence: env > file > defaults
pub fn load_config() -> Result<ConfigBuilder> {
    let mut builder = ConfigBuilder::new();

    // Load from config file if exists
    if let Some(path) = file::find_config_file() {
        tracing::info!("Loading configuration from {}", path.display());
        builder = file::load_from_file(&path, builder)?;
    }

    // Override with environment variables
    builder = env::load_from_env(builder)?;

    Ok(builder)
}

/// Load configuration from a specific file path
pub fn load_config_from_path(path: &std::path::Path) -> Result<ConfigBuilder> {
    let mut builder = ConfigBuilder::new();

    // Load from specified file
    builder = file::load_from_file(path, builder)?;

    // Override with environment variables
    builder = env::load_from_env(builder)?;

    Ok(builder)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn test_load_config_no_file() {
        let result = load_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_from_path_valid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, r#"url = "hdbsql://user:pass@localhost:30015""#).unwrap();
        writeln!(file, r#"pool_size = 5"#).unwrap();

        let result = load_config_from_path(&config_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_from_path_not_found() {
        let path = std::path::Path::new("/nonexistent/config.toml");
        let result = load_config_from_path(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_from_path_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "not valid toml [[[").unwrap();

        let result = load_config_from_path(&config_path);
        assert!(result.is_err());
    }
}
