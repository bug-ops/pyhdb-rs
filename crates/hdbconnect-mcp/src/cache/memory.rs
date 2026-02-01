//! In-memory cache implementation with TTL support

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use parking_lot::RwLock;

use super::config::DEFAULT_MAX_VALUE_SIZE;
use super::error::{CacheError, CacheResult};
use super::key::CacheKey;
use super::provider::{CacheEntryMeta, CacheProvider, CacheStats};

/// Cache entry with value and expiration
struct CacheEntry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
    #[allow(dead_code)]
    created_at: Instant,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Instant::now() > exp)
    }

    fn ttl_remaining(&self) -> Option<Duration> {
        self.expires_at.and_then(|exp| {
            let now = Instant::now();
            if now < exp { Some(exp - now) } else { None }
        })
    }
}

/// In-memory cache stats (internal)
#[derive(Default)]
struct InMemoryStats {
    hits: u64,
    misses: u64,
    sets: u64,
    deletes: u64,
}

/// Thread-safe in-memory cache with TTL support
///
/// # Stats Behavior
///
/// Statistics (hits, misses, sets, deletes) are updated after the primary operation
/// completes and the lock is released. This means stats may be slightly stale during
/// concurrent access but is acceptable for cache metrics where eventual consistency
/// is sufficient.
///
/// # Eviction Behavior
///
/// When `max_entries` is reached, the cache evicts an arbitrary entry (the first
/// one returned by `HashMap` iteration). This is NOT true FIFO/LRU eviction because
/// `HashMap` does not preserve insertion order. For predictable eviction order,
/// consider using `IndexMap` or tracking insertion times separately.
#[derive(Clone)]
pub struct InMemoryCache {
    store: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<InMemoryStats>>,
    max_entries: Option<usize>,
    max_value_size: usize,
    default_ttl: Option<Duration>,
}

impl std::fmt::Debug for InMemoryCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryCache")
            .field("max_entries", &self.max_entries)
            .field("max_value_size", &self.max_value_size)
            .field("default_ttl", &self.default_ttl)
            .field("entry_count", &self.store.read().len())
            .finish_non_exhaustive()
    }
}

impl InMemoryCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(InMemoryStats::default())),
            max_entries: None,
            max_value_size: DEFAULT_MAX_VALUE_SIZE,
            default_ttl: None,
        }
    }

    #[must_use]
    pub const fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = Some(max);
        self
    }

    #[must_use]
    pub const fn with_max_value_size(mut self, max: usize) -> Self {
        self.max_value_size = max;
        self
    }

    #[must_use]
    pub const fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    /// Remove expired entries (call periodically or on access)
    fn cleanup_expired(&self) {
        let mut store = self.store.write();
        store.retain(|_, entry| !entry.is_expired());
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheProvider for InMemoryCache {
    async fn get(&self, key: &CacheKey) -> CacheResult<Option<Vec<u8>>> {
        let key_str = key.to_key_string();
        let store = self.store.read();

        if let Some(entry) = store.get(&key_str) {
            if entry.is_expired() {
                self.stats.write().misses += 1;
                drop(store);
                self.cleanup_expired();
                return Ok(None);
            }
            let value = entry.value.clone();
            drop(store);
            self.stats.write().hits += 1;
            Ok(Some(value))
        } else {
            drop(store);
            self.stats.write().misses += 1;
            Ok(None)
        }
    }

    async fn set(&self, key: &CacheKey, value: &[u8], ttl: Option<Duration>) -> CacheResult<()> {
        if value.len() > self.max_value_size {
            return Err(CacheError::ValueTooLarge {
                size: value.len(),
                max: self.max_value_size,
            });
        }

        let key_str = key.to_key_string();
        let effective_ttl = ttl.or(self.default_ttl);

        let entry = CacheEntry {
            value: value.to_vec(),
            expires_at: effective_ttl.map(|d| Instant::now() + d),
            created_at: Instant::now(),
        };

        let mut store = self.store.write();

        // Evict if over limit (arbitrary eviction due to HashMap iteration order)
        if let Some(max) = self.max_entries
            && store.len() >= max
            && !store.contains_key(&key_str)
            && let Some(oldest_key) = store.keys().next().cloned()
        {
            store.remove(&oldest_key);
        }

        store.insert(key_str, entry);
        drop(store);

        self.stats.write().sets += 1;

        Ok(())
    }

    async fn delete(&self, key: &CacheKey) -> CacheResult<bool> {
        let key_str = key.to_key_string();
        let removed = self.store.write().remove(&key_str).is_some();

        if removed {
            self.stats.write().deletes += 1;
        }

        Ok(removed)
    }

    async fn exists(&self, key: &CacheKey) -> CacheResult<bool> {
        let key_str = key.to_key_string();
        let store = self.store.read();

        Ok(store.get(&key_str).is_some_and(|e| !e.is_expired()))
    }

    async fn delete_by_prefix(&self, prefix: &str) -> CacheResult<u64> {
        let mut store = self.store.write();
        let before = store.len();
        store.retain(|k, _| !k.starts_with(prefix));
        let deleted = (before - store.len()) as u64;
        drop(store);

        if deleted > 0 {
            self.stats.write().deletes += deleted;
        }

        Ok(deleted)
    }

    async fn metadata(&self, key: &CacheKey) -> CacheResult<Option<CacheEntryMeta>> {
        let key_str = key.to_key_string();
        let store = self.store.read();

        Ok(store.get(&key_str).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(CacheEntryMeta {
                    size_bytes: Some(entry.value.len()),
                    ttl_remaining: entry.ttl_remaining(),
                    compressed: false,
                })
            }
        }))
    }

    async fn clear(&self) -> CacheResult<()> {
        self.store.write().clear();
        Ok(())
    }

    async fn health_check(&self) -> CacheResult<()> {
        Ok(())
    }

    async fn stats(&self) -> CacheStats {
        let stats = self.stats.read();
        let store = self.store.read();

        let size_bytes: u64 = store
            .values()
            .filter(|e| !e.is_expired())
            .map(|e| e.value.len() as u64)
            .sum();

        let entry_count = store.values().filter(|e| !e.is_expired()).count() as u64;
        drop(store);

        CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            sets: stats.sets,
            deletes: stats.deletes,
            errors: 0,
            size_bytes: Some(size_bytes),
            entry_count: Some(entry_count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_set_get() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache.set(&key, b"test data", None).await.unwrap();
        let result = cache.get(&key).await.unwrap();

        assert_eq!(result, Some(b"test data".to_vec()));
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let result = cache.get(&key).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_existing_key() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache.set(&key, b"test data", None).await.unwrap();
        let deleted = cache.delete(&key).await.unwrap();

        assert!(deleted);
        assert!(cache.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_key() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");
        let deleted = cache.delete(&key).await.unwrap();

        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_exists() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        assert!(!cache.exists(&key).await.unwrap());

        cache.set(&key, b"test data", None).await.unwrap();

        assert!(cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn test_ttl_expiry() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache
            .set(&key, b"test data", Some(Duration::from_millis(10)))
            .await
            .unwrap();

        assert!(cache.get(&key).await.unwrap().is_some());

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(cache.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_default_ttl() {
        let cache = InMemoryCache::new().with_default_ttl(Duration::from_millis(10));
        let key = CacheKey::table_schema(Some("test"), "users");

        cache.set(&key, b"test data", None).await.unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(cache.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_explicit_ttl_overrides_default() {
        let cache = InMemoryCache::new().with_default_ttl(Duration::from_millis(10));
        let key = CacheKey::table_schema(Some("test"), "users");

        cache
            .set(&key, b"test data", Some(Duration::from_secs(60)))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(cache.get(&key).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_max_entries_eviction() {
        let cache = InMemoryCache::new().with_max_entries(2);

        let key1 = CacheKey::table_schema(Some("test"), "t1");
        let key2 = CacheKey::table_schema(Some("test"), "t2");
        let key3 = CacheKey::table_schema(Some("test"), "t3");

        cache.set(&key1, b"data1", None).await.unwrap();
        cache.set(&key2, b"data2", None).await.unwrap();
        cache.set(&key3, b"data3", None).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, Some(2));
    }

    #[tokio::test]
    async fn test_delete_by_prefix() {
        let cache = InMemoryCache::new();

        let key1 = CacheKey::table_schema(Some("test"), "t1");
        let key2 = CacheKey::table_schema(Some("test"), "t2");
        let key3 = CacheKey::procedure_schema(Some("test"), "p1");

        cache.set(&key1, b"data1", None).await.unwrap();
        cache.set(&key2, b"data2", None).await.unwrap();
        cache.set(&key3, b"data3", None).await.unwrap();

        let deleted = cache.delete_by_prefix("tbl_schema").await.unwrap();

        assert_eq!(deleted, 2);
        assert!(cache.get(&key1).await.unwrap().is_none());
        assert!(cache.get(&key2).await.unwrap().is_none());
        assert!(cache.get(&key3).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_metadata() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache
            .set(&key, b"test data", Some(Duration::from_secs(60)))
            .await
            .unwrap();

        let meta = cache.metadata(&key).await.unwrap().unwrap();

        assert_eq!(meta.size_bytes, Some(9));
        assert!(meta.ttl_remaining.is_some());
        assert!(!meta.compressed);
    }

    #[tokio::test]
    async fn test_metadata_nonexistent() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        let meta = cache.metadata(&key).await.unwrap();
        assert!(meta.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = InMemoryCache::new();

        let key1 = CacheKey::table_schema(Some("test"), "t1");
        let key2 = CacheKey::table_schema(Some("test"), "t2");

        cache.set(&key1, b"data1", None).await.unwrap();
        cache.set(&key2, b"data2", None).await.unwrap();

        cache.clear().await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, Some(0));
    }

    #[tokio::test]
    async fn test_health_check() {
        let cache = InMemoryCache::new();
        let result = cache.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stats_accuracy() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache.get(&key).await.unwrap();
        cache.set(&key, b"test", None).await.unwrap();
        cache.get(&key).await.unwrap();
        cache.delete(&key).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.sets, 1);
        assert_eq!(stats.deletes, 1);
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        let cache_clone = cache.clone();
        cache.set(&key, b"test data", None).await.unwrap();

        let result = cache_clone.get(&key).await.unwrap();
        assert_eq!(result, Some(b"test data".to_vec()));
    }

    #[tokio::test]
    async fn test_expired_entry_not_in_exists() {
        let cache = InMemoryCache::new();
        let key = CacheKey::table_schema(Some("test"), "users");

        cache
            .set(&key, b"test", Some(Duration::from_millis(10)))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn test_value_too_large_rejected() {
        let cache = InMemoryCache::new().with_max_value_size(100);
        let key = CacheKey::table_schema(Some("test"), "users");
        let large_value = vec![0u8; 200];

        let result = cache.set(&key, &large_value, None).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            CacheError::ValueTooLarge { size, max } => {
                assert_eq!(size, 200);
                assert_eq!(max, 100);
            }
            e => panic!("Expected ValueTooLarge error, got: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_value_at_limit_accepted() {
        let cache = InMemoryCache::new().with_max_value_size(100);
        let key = CacheKey::table_schema(Some("test"), "users");
        let value = vec![0u8; 100];

        let result = cache.set(&key, &value, None).await;
        assert!(result.is_ok());

        let retrieved = cache.get(&key).await.unwrap();
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_default_max_value_size() {
        let cache = InMemoryCache::new();
        assert_eq!(cache.max_value_size, DEFAULT_MAX_VALUE_SIZE);
    }

    #[test]
    fn test_debug_impl() {
        let cache = InMemoryCache::new()
            .with_max_entries(100)
            .with_max_value_size(1024)
            .with_default_ttl(Duration::from_secs(60));
        let debug_str = format!("{cache:?}");
        assert!(debug_str.contains("InMemoryCache"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("1024"));
    }
}
