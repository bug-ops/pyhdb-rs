//! Cache configuration types

use std::str::FromStr;
use std::time::Duration;

/// Default maximum value size: 1MB
pub const DEFAULT_MAX_VALUE_SIZE: usize = 1_048_576;

/// Cache backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheBackend {
    #[default]
    Noop,
    Memory,
}

impl FromStr for CacheBackend {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "memory" | "mem" => Self::Memory,
            _ => Self::Noop,
        })
    }
}

/// Cache TTL configuration
#[derive(Debug, Clone, Copy)]
pub struct CacheTtlConfig {
    /// Default TTL for unspecified cache entries
    pub default: Duration,
    /// TTL for schema metadata
    pub schema: Duration,
    /// TTL for query results
    pub query: Duration,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            default: Duration::from_secs(300),
            schema: Duration::from_secs(3600),
            query: Duration::from_secs(60),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Copy)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Cache backend type
    pub backend: CacheBackend,
    /// TTL configuration
    pub ttl: CacheTtlConfig,
    /// Maximum entries for in-memory cache
    pub max_entries: Option<usize>,
    /// Maximum size of a single cached value in bytes (default: 1MB)
    pub max_value_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: CacheBackend::Noop,
            ttl: CacheTtlConfig::default(),
            max_entries: Some(10000),
            max_value_size: DEFAULT_MAX_VALUE_SIZE,
        }
    }
}

impl CacheConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enabled: false,
            backend: CacheBackend::Noop,
            ttl: CacheTtlConfig {
                default: Duration::from_secs(300),
                schema: Duration::from_secs(3600),
                query: Duration::from_secs(60),
            },
            max_entries: Some(10000),
            max_value_size: DEFAULT_MAX_VALUE_SIZE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_backend_from_str_memory() {
        assert_eq!(
            "memory".parse::<CacheBackend>().unwrap(),
            CacheBackend::Memory
        );
        assert_eq!(
            "Memory".parse::<CacheBackend>().unwrap(),
            CacheBackend::Memory
        );
        assert_eq!(
            "MEMORY".parse::<CacheBackend>().unwrap(),
            CacheBackend::Memory
        );
        assert_eq!("mem".parse::<CacheBackend>().unwrap(), CacheBackend::Memory);
    }

    #[test]
    fn test_cache_backend_from_str_noop() {
        assert_eq!("noop".parse::<CacheBackend>().unwrap(), CacheBackend::Noop);
        assert_eq!("none".parse::<CacheBackend>().unwrap(), CacheBackend::Noop);
        assert_eq!(
            "disabled".parse::<CacheBackend>().unwrap(),
            CacheBackend::Noop
        );
    }

    #[test]
    fn test_cache_backend_from_str_unknown() {
        assert_eq!(
            "unknown".parse::<CacheBackend>().unwrap(),
            CacheBackend::Noop
        );
        assert_eq!("redis".parse::<CacheBackend>().unwrap(), CacheBackend::Noop);
    }

    #[test]
    fn test_cache_backend_default() {
        assert_eq!(CacheBackend::default(), CacheBackend::Noop);
    }

    #[test]
    fn test_cache_ttl_config_default() {
        let ttl = CacheTtlConfig::default();
        assert_eq!(ttl.default, Duration::from_secs(300));
        assert_eq!(ttl.schema, Duration::from_secs(3600));
        assert_eq!(ttl.query, Duration::from_secs(60));
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.backend, CacheBackend::Noop);
        assert_eq!(config.max_entries, Some(10000));
        assert_eq!(config.max_value_size, DEFAULT_MAX_VALUE_SIZE);
    }

    #[test]
    fn test_cache_config_new() {
        let config = CacheConfig::new();
        assert!(!config.enabled);
        assert_eq!(config.backend, CacheBackend::Noop);
        assert_eq!(config.ttl.default, Duration::from_secs(300));
        assert_eq!(config.max_value_size, DEFAULT_MAX_VALUE_SIZE);
    }

    #[test]
    fn test_default_max_value_size_is_1mb() {
        assert_eq!(DEFAULT_MAX_VALUE_SIZE, 1024 * 1024);
    }
}
