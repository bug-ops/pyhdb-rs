//! Cache provider trait definition

use std::time::Duration;

use async_trait::async_trait;

use super::error::CacheResult;
use super::key::CacheKey;

/// Cache entry metadata for observability
#[derive(Debug, Clone)]
pub struct CacheEntryMeta {
    /// Size in bytes (if known)
    pub size_bytes: Option<usize>,
    /// Time-to-live remaining
    pub ttl_remaining: Option<Duration>,
    /// Whether entry was compressed
    pub compressed: bool,
}

/// Cache statistics for metrics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
    pub errors: u64,
    pub size_bytes: Option<u64>,
    pub entry_count: Option<u64>,
}

/// Async cache provider trait
///
/// Implementors provide cache storage and retrieval capabilities.
/// All operations are async to support network-based backends.
#[async_trait]
pub trait CacheProvider: Send + Sync {
    /// Get a value from cache by key
    async fn get(&self, key: &CacheKey) -> CacheResult<Option<Vec<u8>>>;

    /// Set a value in cache with optional TTL
    async fn set(&self, key: &CacheKey, value: &[u8], ttl: Option<Duration>) -> CacheResult<()>;

    /// Delete a key from cache
    async fn delete(&self, key: &CacheKey) -> CacheResult<bool>;

    /// Check if a key exists
    async fn exists(&self, key: &CacheKey) -> CacheResult<bool>;

    /// Delete all keys matching a pattern (namespace prefix)
    async fn delete_by_prefix(&self, prefix: &str) -> CacheResult<u64>;

    /// Get entry metadata without retrieving value
    async fn metadata(&self, key: &CacheKey) -> CacheResult<Option<CacheEntryMeta>>;

    /// Clear entire cache (use with caution)
    async fn clear(&self) -> CacheResult<()>;

    /// Health check for the cache backend
    async fn health_check(&self) -> CacheResult<()>;

    /// Get cache statistics for observability
    async fn stats(&self) -> CacheStats;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.sets, 0);
        assert_eq!(stats.deletes, 0);
        assert_eq!(stats.errors, 0);
        assert!(stats.size_bytes.is_none());
        assert!(stats.entry_count.is_none());
    }

    #[test]
    fn test_cache_entry_meta_debug() {
        let meta = CacheEntryMeta {
            size_bytes: Some(100),
            ttl_remaining: Some(Duration::from_secs(60)),
            compressed: false,
        };
        let debug_str = format!("{meta:?}");
        assert!(debug_str.contains("100"));
    }
}
