//! Helper utilities for MCP server

#[cfg(feature = "cache")]
use std::future::Future;

use hdbconnect_async::HdbValue;
use rmcp::ErrorData;
#[cfg(feature = "cache")]
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::pool::{Pool, PooledConnection};

/// Get a connection from the pool, returning `ErrorData` on failure
pub async fn get_connection(pool: &Pool) -> Result<PooledConnection, ErrorData> {
    Box::pin(pool.get())
        .await
        .map_err(|_| Error::PoolExhausted.into())
}

/// Convert `HdbValue` to `serde_json::Value`
pub fn hdb_value_to_json(value: &HdbValue) -> serde_json::Value {
    match value {
        HdbValue::NULL => serde_json::Value::Null,
        HdbValue::TINYINT(v) => serde_json::json!(v),
        HdbValue::SMALLINT(v) => serde_json::json!(v),
        HdbValue::INT(v) => serde_json::json!(v),
        HdbValue::BIGINT(v) => serde_json::json!(v),
        HdbValue::DECIMAL(v) => serde_json::json!(v.to_string()),
        HdbValue::REAL(v) => serde_json::json!(v),
        HdbValue::DOUBLE(v) => serde_json::json!(v),
        HdbValue::STRING(v) => serde_json::json!(v),
        HdbValue::BOOLEAN(v) => serde_json::json!(v),
        _ => serde_json::json!(format!("{value:?}")),
    }
}

/// Try to get a value from cache, falling back to a fetch function.
/// Cache errors are logged but never propagate - always fallback to fetch.
#[cfg(feature = "cache")]
pub async fn cached_or_fetch<T, F, Fut>(
    cache: &dyn crate::cache::CacheProvider,
    key: &crate::cache::CacheKey,
    ttl: std::time::Duration,
    fetch: F,
) -> crate::Result<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FnOnce() -> Fut,
    Fut: Future<Output = crate::Result<T>>,
{
    // 1. Try cache first
    match cache.get(key).await {
        Ok(Some(data)) => match serde_json::from_slice::<T>(&data) {
            Ok(value) => {
                tracing::debug!(
                    cache.result = "hit",
                    cache.key = %key,
                    "Returning cached value"
                );
                return Ok(value);
            }
            Err(e) => {
                tracing::warn!(
                    cache.key = %key,
                    error = %e,
                    "Cache deserialization failed, fetching from source"
                );
            }
        },
        Ok(None) => {
            tracing::debug!(
                cache.result = "miss",
                cache.key = %key,
                "Cache miss, fetching from source"
            );
        }
        Err(e) => {
            tracing::warn!(
                cache.key = %key,
                error = %e,
                "Cache get failed, fetching from source"
            );
        }
    }

    // 2. Fetch from source
    let value = fetch().await?;

    // 3. Store in cache (fire-and-forget, errors logged but not propagated)
    match serde_json::to_vec(&value) {
        Ok(data) => {
            if let Err(e) = cache.set(key, &data, Some(ttl)).await {
                tracing::warn!(
                    cache.key = %key,
                    error = %e,
                    "Failed to cache value"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                cache.key = %key,
                error = %e,
                "Failed to serialize value for caching"
            );
        }
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdb_value_to_json_null() {
        let result = hdb_value_to_json(&HdbValue::NULL);
        assert!(result.is_null());
    }

    #[test]
    fn test_hdb_value_to_json_tinyint() {
        let result = hdb_value_to_json(&HdbValue::TINYINT(42));
        assert_eq!(result.as_u64(), Some(42));
    }

    #[test]
    fn test_hdb_value_to_json_smallint() {
        let result = hdb_value_to_json(&HdbValue::SMALLINT(1234));
        assert_eq!(result.as_i64(), Some(1234));
    }

    #[test]
    fn test_hdb_value_to_json_int() {
        let result = hdb_value_to_json(&HdbValue::INT(123456));
        assert_eq!(result.as_i64(), Some(123456));
    }

    #[test]
    fn test_hdb_value_to_json_bigint() {
        let result = hdb_value_to_json(&HdbValue::BIGINT(9_876_543_210));
        assert_eq!(result.as_i64(), Some(9_876_543_210));
    }

    #[test]
    fn test_hdb_value_to_json_real() {
        let result = hdb_value_to_json(&HdbValue::REAL(3.14));
        assert!(result.is_number());
    }

    #[test]
    fn test_hdb_value_to_json_double() {
        let result = hdb_value_to_json(&HdbValue::DOUBLE(2.71828));
        assert_eq!(result.as_f64(), Some(2.71828));
    }

    #[test]
    fn test_hdb_value_to_json_string() {
        let result = hdb_value_to_json(&HdbValue::STRING("hello world".to_string()));
        assert_eq!(result.as_str(), Some("hello world"));
    }

    #[test]
    fn test_hdb_value_to_json_boolean_true() {
        let result = hdb_value_to_json(&HdbValue::BOOLEAN(true));
        assert_eq!(result.as_bool(), Some(true));
    }

    #[test]
    fn test_hdb_value_to_json_boolean_false() {
        let result = hdb_value_to_json(&HdbValue::BOOLEAN(false));
        assert_eq!(result.as_bool(), Some(false));
    }

    #[test]
    fn test_hdb_value_to_json_binary_fallback() {
        let result = hdb_value_to_json(&HdbValue::BINARY(vec![1, 2, 3]));
        assert!(result.is_string());
        assert!(result.as_str().unwrap().contains("BINARY"));
    }
}

#[cfg(all(test, feature = "cache"))]
mod cache_tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use async_trait::async_trait;
    use parking_lot::RwLock;

    use super::*;
    use crate::cache::{
        CacheEntryMeta, CacheError, CacheKey, CacheProvider, CacheResult, CacheStats,
    };

    #[derive(Default)]
    struct MockCache {
        get_calls: AtomicU64,
        set_calls: AtomicU64,
        stored: RwLock<std::collections::HashMap<String, Vec<u8>>>,
        fail_get: AtomicU64,
        fail_set: AtomicU64,
    }

    #[async_trait]
    impl CacheProvider for MockCache {
        async fn get(&self, key: &CacheKey) -> CacheResult<Option<Vec<u8>>> {
            self.get_calls.fetch_add(1, Ordering::Relaxed);
            if self.fail_get.load(Ordering::Relaxed) > 0 {
                self.fail_get.fetch_sub(1, Ordering::Relaxed);
                return Err(CacheError::Connection("mock failure".into()));
            }
            Ok(self.stored.read().get(&key.to_key_string()).cloned())
        }

        async fn set(
            &self,
            key: &CacheKey,
            value: &[u8],
            _ttl: Option<Duration>,
        ) -> CacheResult<()> {
            self.set_calls.fetch_add(1, Ordering::Relaxed);
            if self.fail_set.load(Ordering::Relaxed) > 0 {
                self.fail_set.fetch_sub(1, Ordering::Relaxed);
                return Err(CacheError::Connection("mock failure".into()));
            }
            self.stored
                .write()
                .insert(key.to_key_string(), value.to_vec());
            Ok(())
        }

        async fn delete(&self, key: &CacheKey) -> CacheResult<bool> {
            Ok(self.stored.write().remove(&key.to_key_string()).is_some())
        }

        async fn exists(&self, key: &CacheKey) -> CacheResult<bool> {
            Ok(self.stored.read().contains_key(&key.to_key_string()))
        }

        async fn delete_by_prefix(&self, prefix: &str) -> CacheResult<u64> {
            let mut count = 0;
            self.stored.write().retain(|k, _| {
                if k.starts_with(prefix) {
                    count += 1;
                    false
                } else {
                    true
                }
            });
            Ok(count)
        }

        async fn metadata(&self, key: &CacheKey) -> CacheResult<Option<CacheEntryMeta>> {
            Ok(self
                .stored
                .read()
                .get(&key.to_key_string())
                .map(|data| CacheEntryMeta {
                    size_bytes: Some(data.len()),
                    ttl_remaining: None,
                    compressed: false,
                }))
        }

        async fn clear(&self) -> CacheResult<()> {
            self.stored.write().clear();
            Ok(())
        }

        async fn health_check(&self) -> CacheResult<()> {
            Ok(())
        }

        async fn stats(&self) -> CacheStats {
            CacheStats {
                hits: 0,
                misses: 0,
                sets: self.set_calls.load(Ordering::Relaxed),
                deletes: 0,
                errors: 0,
                size_bytes: None,
                entry_count: Some(self.stored.read().len() as u64),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
    }

    #[tokio::test]
    async fn test_cached_or_fetch_cache_miss() {
        let cache = Arc::new(MockCache::default());
        let key = CacheKey::table_schema(Some("test"), "users");
        let fetch_count = Arc::new(AtomicU64::new(0));
        let fetch_count_clone = Arc::clone(&fetch_count);

        let result: TestData =
            cached_or_fetch(cache.as_ref(), &key, Duration::from_secs(60), || {
                let fetch_count = Arc::clone(&fetch_count_clone);
                async move {
                    fetch_count.fetch_add(1, Ordering::Relaxed);
                    Ok(TestData {
                        value: "from_fetch".to_string(),
                    })
                }
            })
            .await
            .unwrap();

        assert_eq!(result.value, "from_fetch");
        assert_eq!(fetch_count.load(Ordering::Relaxed), 1);
        assert_eq!(cache.get_calls.load(Ordering::Relaxed), 1);
        assert_eq!(cache.set_calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_cached_or_fetch_cache_hit() {
        let cache = Arc::new(MockCache::default());
        let key = CacheKey::table_schema(Some("test"), "users");
        let fetch_count = Arc::new(AtomicU64::new(0));

        // Pre-populate cache
        let cached_data = TestData {
            value: "cached".to_string(),
        };
        cache
            .set(&key, &serde_json::to_vec(&cached_data).unwrap(), None)
            .await
            .unwrap();
        cache.set_calls.store(0, Ordering::Relaxed);

        let fetch_count_clone = Arc::clone(&fetch_count);
        let result: TestData =
            cached_or_fetch(cache.as_ref(), &key, Duration::from_secs(60), || {
                let fetch_count = Arc::clone(&fetch_count_clone);
                async move {
                    fetch_count.fetch_add(1, Ordering::Relaxed);
                    Ok(TestData {
                        value: "from_fetch".to_string(),
                    })
                }
            })
            .await
            .unwrap();

        assert_eq!(result.value, "cached");
        assert_eq!(fetch_count.load(Ordering::Relaxed), 0);
        assert_eq!(cache.get_calls.load(Ordering::Relaxed), 1);
        assert_eq!(cache.set_calls.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_cached_or_fetch_cache_error_fallback() {
        let cache = Arc::new(MockCache::default());
        cache.fail_get.store(1, Ordering::Relaxed);
        let key = CacheKey::table_schema(Some("test"), "users");
        let fetch_count = Arc::new(AtomicU64::new(0));
        let fetch_count_clone = Arc::clone(&fetch_count);

        let result: TestData =
            cached_or_fetch(cache.as_ref(), &key, Duration::from_secs(60), || {
                let fetch_count = Arc::clone(&fetch_count_clone);
                async move {
                    fetch_count.fetch_add(1, Ordering::Relaxed);
                    Ok(TestData {
                        value: "from_fetch".to_string(),
                    })
                }
            })
            .await
            .unwrap();

        assert_eq!(result.value, "from_fetch");
        assert_eq!(fetch_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_cached_or_fetch_deserialization_error() {
        let cache = Arc::new(MockCache::default());
        let key = CacheKey::table_schema(Some("test"), "users");

        // Pre-populate cache with invalid data
        cache.set(&key, b"invalid json", None).await.unwrap();
        cache.set_calls.store(0, Ordering::Relaxed);

        let fetch_count = Arc::new(AtomicU64::new(0));
        let fetch_count_clone = Arc::clone(&fetch_count);

        let result: TestData =
            cached_or_fetch(cache.as_ref(), &key, Duration::from_secs(60), || {
                let fetch_count = Arc::clone(&fetch_count_clone);
                async move {
                    fetch_count.fetch_add(1, Ordering::Relaxed);
                    Ok(TestData {
                        value: "from_fetch".to_string(),
                    })
                }
            })
            .await
            .unwrap();

        assert_eq!(result.value, "from_fetch");
        assert_eq!(fetch_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_cached_or_fetch_set_error_still_returns_value() {
        let cache = Arc::new(MockCache::default());
        cache.fail_set.store(1, Ordering::Relaxed);
        let key = CacheKey::table_schema(Some("test"), "users");

        let result: TestData = cached_or_fetch(
            cache.as_ref(),
            &key,
            Duration::from_secs(60),
            || async move {
                Ok(TestData {
                    value: "from_fetch".to_string(),
                })
            },
        )
        .await
        .unwrap();

        assert_eq!(result.value, "from_fetch");
        assert_eq!(cache.set_calls.load(Ordering::Relaxed), 1);
        // Cache should be empty because set failed
        assert!(cache.stored.read().is_empty());
    }
}
