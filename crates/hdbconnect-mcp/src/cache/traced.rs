//! Traced cache wrapper for observability

use std::time::Duration;

use async_trait::async_trait;
use tracing::Instrument;

use super::error::CacheResult;
use super::key::CacheKey;
use super::provider::{CacheEntryMeta, CacheProvider, CacheStats};

/// Wrapper that adds tracing to any `CacheProvider`
///
/// Uses debug-level spans to avoid exposing cache keys in production logs.
/// Cache keys may reveal database schema information (table names, columns).
pub struct TracedCache<C> {
    inner: C,
    #[allow(dead_code)]
    service_name: String,
}

impl<C: std::fmt::Debug> std::fmt::Debug for TracedCache<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracedCache")
            .field("inner", &self.inner)
            .field("service_name", &self.service_name)
            .finish()
    }
}

impl<C: Clone> Clone for TracedCache<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            service_name: self.service_name.clone(),
        }
    }
}

impl<C> TracedCache<C> {
    pub fn new(cache: C, service_name: impl Into<String>) -> Self {
        Self {
            inner: cache,
            service_name: service_name.into(),
        }
    }
}

#[async_trait]
impl<C: CacheProvider> CacheProvider for TracedCache<C> {
    async fn get(&self, key: &CacheKey) -> CacheResult<Option<Vec<u8>>> {
        let span = tracing::debug_span!(
            "cache.get",
            cache.key = %key,
            cache.namespace = key.namespace().as_str(),
            otel.name = "cache.get",
        );

        let result = self.inner.get(key).instrument(span).await;

        match &result {
            Ok(Some(data)) => {
                tracing::debug!(
                    cache.result = "hit",
                    cache.key = %key,
                    cache.size_bytes = data.len(),
                );
            }
            Ok(None) => {
                tracing::debug!(cache.result = "miss", cache.key = %key);
            }
            Err(e) => {
                tracing::warn!(cache.result = "error", cache.key = %key, error = %e);
            }
        }

        result
    }

    async fn set(&self, key: &CacheKey, value: &[u8], ttl: Option<Duration>) -> CacheResult<()> {
        let span = tracing::debug_span!(
            "cache.set",
            cache.key = %key,
            cache.namespace = key.namespace().as_str(),
            cache.value_size = value.len(),
            cache.ttl_secs = ttl.map(|d| d.as_secs()),
            otel.name = "cache.set",
        );

        let result = self.inner.set(key, value, ttl).instrument(span).await;

        if let Err(ref e) = result {
            tracing::warn!(
                cache.operation = "set",
                cache.key = %key,
                error = %e,
            );
        }

        result
    }

    async fn delete(&self, key: &CacheKey) -> CacheResult<bool> {
        let span = tracing::debug_span!(
            "cache.delete",
            cache.key = %key,
            cache.namespace = key.namespace().as_str(),
            otel.name = "cache.delete",
        );

        let result = self.inner.delete(key).instrument(span).await;

        match &result {
            Ok(deleted) => {
                tracing::debug!(
                    cache.operation = "delete",
                    cache.key = %key,
                    cache.deleted = deleted,
                );
            }
            Err(e) => {
                tracing::warn!(
                    cache.operation = "delete",
                    cache.key = %key,
                    error = %e,
                );
            }
        }

        result
    }

    async fn exists(&self, key: &CacheKey) -> CacheResult<bool> {
        let span = tracing::debug_span!(
            "cache.exists",
            cache.key = %key,
            cache.namespace = key.namespace().as_str(),
            otel.name = "cache.exists",
        );

        self.inner.exists(key).instrument(span).await
    }

    async fn delete_by_prefix(&self, prefix: &str) -> CacheResult<u64> {
        let span = tracing::debug_span!(
            "cache.delete_by_prefix",
            cache.prefix = prefix,
            otel.name = "cache.delete_by_prefix",
        );

        let result = self.inner.delete_by_prefix(prefix).instrument(span).await;

        match &result {
            Ok(count) => {
                tracing::debug!(
                    cache.operation = "delete_by_prefix",
                    cache.prefix = prefix,
                    cache.deleted_count = count,
                );
            }
            Err(e) => {
                tracing::warn!(
                    cache.operation = "delete_by_prefix",
                    cache.prefix = prefix,
                    error = %e,
                );
            }
        }

        result
    }

    async fn metadata(&self, key: &CacheKey) -> CacheResult<Option<CacheEntryMeta>> {
        let span = tracing::debug_span!(
            "cache.metadata",
            cache.key = %key,
            cache.namespace = key.namespace().as_str(),
            otel.name = "cache.metadata",
        );

        self.inner.metadata(key).instrument(span).await
    }

    async fn clear(&self) -> CacheResult<()> {
        let span = tracing::debug_span!("cache.clear", otel.name = "cache.clear",);

        let result = self.inner.clear().instrument(span).await;

        if result.is_ok() {
            tracing::debug!(cache.operation = "clear");
        } else if let Err(ref e) = result {
            tracing::warn!(cache.operation = "clear", error = %e);
        }

        result
    }

    async fn health_check(&self) -> CacheResult<()> {
        let span = tracing::debug_span!("cache.health_check", otel.name = "cache.health_check",);

        self.inner.health_check().instrument(span).await
    }

    async fn stats(&self) -> CacheStats {
        self.inner.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::super::noop::NoopCache;
    use super::*;

    #[tokio::test]
    async fn test_traced_cache_get_miss() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let key = CacheKey::table_schema(Some("test"), "users");

        let result = traced.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_traced_cache_set() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let key = CacheKey::table_schema(Some("test"), "users");

        let result = traced
            .set(&key, b"test data", Some(Duration::from_secs(60)))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_traced_cache_delete() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let key = CacheKey::table_schema(Some("test"), "users");

        let result = traced.delete(&key).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_traced_cache_exists() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let key = CacheKey::table_schema(Some("test"), "users");

        let result = traced.exists(&key).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_traced_cache_delete_by_prefix() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");

        let result = traced.delete_by_prefix("tbl_schema").await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_traced_cache_metadata() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let key = CacheKey::table_schema(Some("test"), "users");

        let result = traced.metadata(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_traced_cache_clear() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");

        let result = traced.clear().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_traced_cache_health_check() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");

        let result = traced.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_traced_cache_stats() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");

        let stats = traced.stats().await;
        assert_eq!(stats.hits, 0);
    }

    #[tokio::test]
    async fn test_traced_cache_clone() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let cloned = traced.clone();

        let key = CacheKey::table_schema(Some("test"), "users");
        assert!(cloned.get(&key).await.unwrap().is_none());
    }

    #[test]
    fn test_traced_cache_debug() {
        let inner = NoopCache::new();
        let traced = TracedCache::new(inner, "test-service");
        let debug_str = format!("{traced:?}");
        assert!(debug_str.contains("TracedCache"));
        assert!(debug_str.contains("test-service"));
    }
}
