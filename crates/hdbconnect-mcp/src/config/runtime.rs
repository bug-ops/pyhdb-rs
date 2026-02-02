//! Runtime configuration that can be reloaded without restart

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use super::Config;

/// Runtime configuration parameters that can be changed without restart.
///
/// These parameters are safe to reload because they:
/// - Are applied per-request (not at startup)
/// - Don't affect resource allocation (pool size, memory)
/// - Don't change security boundaries (schema filter)
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum rows to return per query
    pub row_limit: Option<NonZeroU32>,
    /// Query execution timeout
    pub query_timeout: Duration,
    /// Log level filter
    pub log_level: String,
    #[cfg(feature = "cache")]
    /// Default cache TTL
    pub cache_default_ttl: Duration,
    #[cfg(feature = "cache")]
    /// Schema metadata cache TTL
    pub cache_schema_ttl: Duration,
    #[cfg(feature = "cache")]
    /// Query results cache TTL
    pub cache_query_ttl: Duration,
}

impl RuntimeConfig {
    /// Create runtime config from full config
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            row_limit: config.row_limit,
            query_timeout: config.query_timeout,
            log_level: config.telemetry.log_level.clone(),
            #[cfg(feature = "cache")]
            cache_default_ttl: config.cache.ttl.default,
            #[cfg(feature = "cache")]
            cache_schema_ttl: config.cache.ttl.schema,
            #[cfg(feature = "cache")]
            cache_query_ttl: config.cache.ttl.query,
        }
    }
}

/// Thread-safe runtime configuration holder with atomic updates.
///
/// Uses `ArcSwap` for lock-free reads during request handling.
/// In-flight requests continue with their captured config reference.
#[derive(Debug)]
pub struct RuntimeConfigHolder {
    inner: ArcSwap<RuntimeConfig>,
}

impl RuntimeConfigHolder {
    /// Create a new holder with initial config
    #[must_use]
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            inner: ArcSwap::from_pointee(config),
        }
    }

    /// Get current runtime config (lock-free read)
    #[must_use]
    pub fn load(&self) -> Arc<RuntimeConfig> {
        self.inner.load_full()
    }

    /// Update runtime config atomically
    pub fn store(&self, config: RuntimeConfig) {
        self.inner.store(Arc::new(config));
    }

    /// Get row limit from current config
    #[must_use]
    pub fn row_limit(&self) -> Option<NonZeroU32> {
        self.inner.load().row_limit
    }

    /// Get query timeout from current config
    #[must_use]
    pub fn query_timeout(&self) -> Duration {
        self.inner.load().query_timeout
    }
}

/// Reload trigger source for audit logging
#[derive(Debug, Clone)]
pub enum ReloadTrigger {
    /// SIGHUP signal received
    Signal,
    /// HTTP /admin/reload endpoint called
    HttpEndpoint {
        /// Remote IP address
        remote_addr: Option<String>,
    },
    /// Programmatic reload
    Manual,
}

impl std::fmt::Display for ReloadTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal => write!(f, "SIGHUP"),
            Self::HttpEndpoint { remote_addr } => {
                if let Some(addr) = remote_addr {
                    write!(f, "HTTP /admin/reload from {addr}")
                } else {
                    write!(f, "HTTP /admin/reload")
                }
            }
            Self::Manual => write!(f, "manual"),
        }
    }
}

/// Result of a configuration reload attempt
#[derive(Debug, Clone)]
pub struct ReloadResult {
    /// Whether reload succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Parameters that changed
    pub changed: Vec<String>,
}

impl ReloadResult {
    /// Create a successful reload result
    #[must_use]
    pub const fn success(changed: Vec<String>) -> Self {
        Self {
            success: true,
            error: None,
            changed,
        }
    }

    /// Create a failed reload result
    #[must_use]
    pub const fn failure(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            changed: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_runtime_config() -> RuntimeConfig {
        RuntimeConfig {
            row_limit: NonZeroU32::new(1000),
            query_timeout: Duration::from_secs(30),
            log_level: "info".to_string(),
            #[cfg(feature = "cache")]
            cache_default_ttl: Duration::from_secs(300),
            #[cfg(feature = "cache")]
            cache_schema_ttl: Duration::from_secs(3600),
            #[cfg(feature = "cache")]
            cache_query_ttl: Duration::from_secs(60),
        }
    }

    #[test]
    fn test_runtime_config_holder_load() {
        let config = create_test_runtime_config();
        let holder = RuntimeConfigHolder::new(config);

        assert_eq!(holder.row_limit(), NonZeroU32::new(1000));
        assert_eq!(holder.query_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_runtime_config_holder_store() {
        let config = create_test_runtime_config();
        let holder = RuntimeConfigHolder::new(config);

        let new_config = RuntimeConfig {
            row_limit: NonZeroU32::new(500),
            query_timeout: Duration::from_secs(60),
            log_level: "debug".to_string(),
            #[cfg(feature = "cache")]
            cache_default_ttl: Duration::from_secs(600),
            #[cfg(feature = "cache")]
            cache_schema_ttl: Duration::from_secs(7200),
            #[cfg(feature = "cache")]
            cache_query_ttl: Duration::from_secs(120),
        };

        holder.store(new_config);

        assert_eq!(holder.row_limit(), NonZeroU32::new(500));
        assert_eq!(holder.query_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_reload_trigger_display() {
        assert_eq!(ReloadTrigger::Signal.to_string(), "SIGHUP");
        assert_eq!(ReloadTrigger::Manual.to_string(), "manual");
        assert_eq!(
            ReloadTrigger::HttpEndpoint {
                remote_addr: Some("127.0.0.1".to_string())
            }
            .to_string(),
            "HTTP /admin/reload from 127.0.0.1"
        );
    }

    #[test]
    fn test_reload_result_success() {
        let result = ReloadResult::success(vec!["row_limit".to_string()]);
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.changed, vec!["row_limit"]);
    }

    #[test]
    fn test_reload_result_failure() {
        let result = ReloadResult::failure("Invalid config".to_string());
        assert!(!result.success);
        assert_eq!(result.error, Some("Invalid config".to_string()));
        assert!(result.changed.is_empty());
    }
}
