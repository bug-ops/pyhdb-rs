//! No-op cache implementation

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use super::error::CacheResult;
use super::key::CacheKey;
use super::provider::{CacheEntryMeta, CacheProvider, CacheStats};

/// No-op cache implementation that never stores
///
/// Used for testing and when caching is disabled.
#[derive(Debug, Clone, Default)]
pub struct NoopCache {
    misses: Arc<AtomicU64>,
}

impl NoopCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CacheProvider for NoopCache {
    async fn get(&self, _key: &CacheKey) -> CacheResult<Option<Vec<u8>>> {
        self.misses.fetch_add(1, Ordering::Relaxed);
        Ok(None)
    }

    async fn set(&self, _key: &CacheKey, _value: &[u8], _ttl: Option<Duration>) -> CacheResult<()> {
        Ok(())
    }

    async fn delete(&self, _key: &CacheKey) -> CacheResult<bool> {
        Ok(false)
    }

    async fn exists(&self, _key: &CacheKey) -> CacheResult<bool> {
        Ok(false)
    }

    async fn delete_by_prefix(&self, _prefix: &str) -> CacheResult<u64> {
        Ok(0)
    }

    async fn metadata(&self, _key: &CacheKey) -> CacheResult<Option<CacheEntryMeta>> {
        Ok(None)
    }

    async fn clear(&self) -> CacheResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> CacheResult<()> {
        Ok(())
    }

    async fn stats(&self) -> CacheStats {
        CacheStats {
            misses: self.misses.load(Ordering::Relaxed),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_get_always_none() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let result = cache.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_set_succeeds() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let result = cache
            .set(&key, b"test data", Some(Duration::from_secs(60)))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_noop_delete_returns_false() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let result = cache.delete(&key).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_noop_exists_returns_false() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let result = cache.exists(&key).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_noop_delete_by_prefix_returns_zero() {
        let cache = NoopCache::new();
        let result = cache.delete_by_prefix("tbl_schema").await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_noop_metadata_returns_none() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let result = cache.metadata(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_clear_succeeds() {
        let cache = NoopCache::new();
        let result = cache.clear().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_noop_health_check_succeeds() {
        let cache = NoopCache::new();
        let result = cache.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_noop_stats_tracks_misses() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache.get(&key).await.unwrap();
        cache.get(&key).await.unwrap();
        cache.get(&key).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.misses, 3);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.sets, 0);
    }

    #[tokio::test]
    async fn test_noop_clone_shares_stats() {
        let cache = NoopCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        let cache_clone = cache.clone();
        cache.get(&key).await.unwrap();
        cache_clone.get(&key).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.misses, 2);
    }
}
