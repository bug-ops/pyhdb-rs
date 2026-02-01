//! Cache abstraction layer for MCP server tools
//!
//! Provides pluggable cache backends with a unified async interface.
//! Supports in-memory caching with TTL, eviction, and observability hooks.
//!
//! # Available Backends
//!
//! - [`NoopCache`] - No-op implementation (caching disabled)
//! - [`InMemoryCache`] - Thread-safe in-memory cache with TTL support
//!
//! # Observability
//!
//! Wrap any cache with [`TracedCache`] to add tracing spans and logging.
//!
//! # Deployment Considerations
//!
//! The cache is designed for **single-user MCP deployments** where all queries
//! run under the same database user or service account.
//!
//! ## Multi-User Limitation
//!
//! Cache keys do not include user context. In multi-tenant deployments with
//! per-user database permissions (row-level security), cached results from
//! one user may be served to another, potentially bypassing authorization.
//!
//! For multi-user environments, consider:
//! - Disabling the cache feature entirely
//! - Using cache only for non-sensitive metadata (table lists, column definitions)
//! - Implementing user-scoped cache keys at the application layer
//!
//! For typical single-user MCP scenarios (personal AI assistant, service account),
//! the cache is safe and recommended for performance.
//!
//! ## Schema Staleness
//!
//! Schema metadata is cached for 1 hour by default. DDL changes (ALTER TABLE,
//! DROP COLUMN) may not be reflected until TTL expires. Reduce
//! [`CacheTtlConfig::schema`] for environments with frequent schema changes.

mod config;
mod error;
mod key;
mod memory;
mod noop;
mod provider;
mod traced;

use std::sync::Arc;

pub use config::{CacheBackend, CacheConfig, CacheTtlConfig, DEFAULT_MAX_VALUE_SIZE};
pub use error::{CacheError, CacheResult};
pub use key::{CacheKey, CacheNamespace};
pub use memory::InMemoryCache;
pub use noop::NoopCache;
pub use provider::{CacheEntryMeta, CacheProvider, CacheStats};
pub use traced::TracedCache;

/// Create a cache provider based on configuration
#[must_use]
pub fn create_cache(config: &CacheConfig) -> Arc<dyn CacheProvider> {
    if !config.enabled {
        return Arc::new(NoopCache::new());
    }

    match config.backend {
        CacheBackend::Noop => Arc::new(NoopCache::new()),
        CacheBackend::Memory => {
            let mut cache = InMemoryCache::new()
                .with_default_ttl(config.ttl.default)
                .with_max_value_size(config.max_value_size);

            if let Some(max) = config.max_entries {
                cache = cache.with_max_entries(max);
            }

            Arc::new(TracedCache::new(cache, "hdbconnect-mcp"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cache_disabled() {
        let config = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        let _ = create_cache(&config);
    }

    #[test]
    fn test_create_cache_noop() {
        let config = CacheConfig {
            enabled: true,
            backend: CacheBackend::Noop,
            ..Default::default()
        };
        let _ = create_cache(&config);
    }

    #[test]
    fn test_create_cache_memory() {
        let config = CacheConfig {
            enabled: true,
            backend: CacheBackend::Memory,
            ..Default::default()
        };
        let _ = create_cache(&config);
    }

    #[tokio::test]
    async fn test_create_cache_memory_functional() {
        let config = CacheConfig {
            enabled: true,
            backend: CacheBackend::Memory,
            max_entries: Some(100),
            ..Default::default()
        };
        let cache = create_cache(&config);
        let key = CacheKey::table_schema(Some("test"), "users");

        cache.set(&key, b"test data", None).await.unwrap();
        let result = cache.get(&key).await.unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_create_cache_memory_with_custom_max_value_size() {
        let config = CacheConfig {
            enabled: true,
            backend: CacheBackend::Memory,
            max_value_size: 100,
            ..Default::default()
        };
        let cache = create_cache(&config);
        let key = CacheKey::table_schema(Some("test"), "users");

        let small_value = vec![0u8; 50];
        cache.set(&key, &small_value, None).await.unwrap();
        assert!(cache.get(&key).await.unwrap().is_some());

        let large_value = vec![0u8; 200];
        let result = cache.set(&key, &large_value, None).await;
        assert!(result.is_err());
    }
}
