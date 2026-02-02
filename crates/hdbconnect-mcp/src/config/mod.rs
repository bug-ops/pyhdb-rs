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
    use super::*;

    #[test]
    fn test_load_config_no_file() {
        // Should not fail even if no config file exists
        let result = load_config();
        // Will fail on build() because URL is required, but load_config itself should succeed
        assert!(result.is_ok());
    }
}
