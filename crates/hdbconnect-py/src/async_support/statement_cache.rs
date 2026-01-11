//! Prepared statement LRU cache.
//!
//! Provides caching for SQL statement metadata to track query patterns and usage.
//! The cache helps identify frequently executed queries and maintains hit/miss statistics.
//!
//! # Performance Note
//!
//! The current implementation uses `String` keys which allocate on each lookup.
//! For high-frequency lookups, consider using a cache implementation that supports
//! heterogeneous lookups (e.g., `hashbrown` with `raw_entry` API). The LRU crate
//! doesn't support `Borrow` trait-based lookups, so we accept this trade-off for
//! simplicity and correctness.

use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

use lru::LruCache;

/// Key for statement cache lookups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementKey {
    sql: String,
}

impl Hash for StatementKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sql.hash(state);
    }
}

impl StatementKey {
    /// Creates a new key from a SQL string.
    #[inline]
    pub fn new(sql: impl Into<String>) -> Self {
        Self { sql: sql.into() }
    }

    /// Returns the SQL string reference.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Consumes the key and returns the owned SQL string.
    pub fn into_string(self) -> String {
        self.sql
    }
}

/// Cached statement metadata.
#[derive(Debug, Clone)]
pub struct CachedStatement {
    /// The SQL statement text.
    pub sql: String,
    /// Number of times this statement has been executed.
    pub use_count: u64,
}

/// LRU cache for prepared statements.
///
/// Tracks SQL statement execution patterns and provides hit/miss statistics.
#[derive(Debug)]
pub struct PreparedStatementCache {
    cache: LruCache<StatementKey, CachedStatement>,
    hits: u64,
    misses: u64,
}

impl PreparedStatementCache {
    /// Creates a new cache with the specified capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is 0.
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            cache: LruCache::new(capacity),
            hits: 0,
            misses: 0,
        }
    }

    /// Returns the maximum capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.cache.cap().get()
    }

    /// Returns the current number of cached statements.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Returns the total number of cache hits.
    pub const fn hits(&self) -> u64 {
        self.hits
    }

    /// Returns the total number of cache misses.
    pub const fn misses(&self) -> u64 {
        self.misses
    }

    /// Returns the cache hit rate as a value between 0.0 and 1.0.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Checks if a statement is in the cache without updating LRU order.
    pub fn contains(&self, sql: &str) -> bool {
        let key = StatementKey::new(sql);
        self.cache.peek(&key).is_some()
    }

    /// Gets a cached statement, updating LRU order and hit/miss stats.
    pub fn get(&mut self, sql: &str) -> Option<&CachedStatement> {
        let key = StatementKey::new(sql);
        if let Some(cached) = self.cache.get(&key) {
            self.hits += 1;
            Some(cached)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Inserts a statement into the cache.
    ///
    /// Returns the evicted statement if the cache was at capacity.
    pub fn insert(&mut self, sql: impl Into<String>) -> Option<CachedStatement> {
        let sql = sql.into();
        let key = StatementKey::new(&sql);
        let cached = CachedStatement { sql, use_count: 1 };
        self.cache.push(key, cached).map(|(_, v)| v)
    }

    /// Records a use of an existing cached statement, incrementing its use count.
    pub fn record_use(&mut self, sql: &str) {
        let key = StatementKey::new(sql);
        if let Some(cached) = self.cache.get_mut(&key) {
            cached.use_count += 1;
        }
    }

    /// Clears all cached statements and resets statistics.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Gets or inserts a statement, calling the closure on cache miss.
    ///
    /// The closure is only called when the statement is not in the cache.
    /// This is useful for tracking when new statements are executed.
    pub fn get_or_insert<F>(&mut self, sql: &str, on_miss: F) -> &CachedStatement
    where
        F: FnOnce(),
    {
        let key = StatementKey::new(sql);
        if self.cache.get(&key).is_some() {
            self.hits += 1;
            if let Some(cached) = self.cache.get_mut(&key) {
                cached.use_count += 1;
            }
            return self.cache.peek(&key).expect("just accessed");
        }

        self.misses += 1;
        on_miss();
        let cached = CachedStatement {
            sql: sql.to_string(),
            use_count: 1,
        };
        let key = StatementKey::new(sql);
        self.cache.push(key.clone(), cached);
        self.cache.peek(&key).expect("just inserted")
    }

    /// Returns cache statistics snapshot.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
            hit_rate: self.hit_rate(),
            size: self.len(),
            capacity: self.capacity(),
        }
    }
}

/// Cache statistics snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CacheStats {
    /// Total number of cache hits.
    pub hits: u64,
    /// Total number of cache misses.
    pub misses: u64,
    /// Hit rate as a value between 0.0 and 1.0.
    pub hit_rate: f64,
    /// Current number of cached statements.
    pub size: usize,
    /// Maximum cache capacity.
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let mut cache = PreparedStatementCache::new(2);

        assert!(cache.is_empty());
        assert_eq!(cache.capacity(), 2);

        cache.insert("SELECT 1");
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("SELECT 1"));

        let cached = cache.get("SELECT 1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().sql, "SELECT 1");
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = PreparedStatementCache::new(2);

        cache.insert("SELECT 1");
        cache.insert("SELECT 2");
        assert_eq!(cache.len(), 2);

        // Access SELECT 1 to make it recently used
        cache.get("SELECT 1");

        // Insert SELECT 3, should evict SELECT 2 (least recently used)
        cache.insert("SELECT 3");
        assert_eq!(cache.len(), 2);

        assert!(cache.contains("SELECT 1"));
        assert!(!cache.contains("SELECT 2"));
        assert!(cache.contains("SELECT 3"));
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1");

        // 2 hits
        cache.get("SELECT 1");
        cache.get("SELECT 1");

        // 1 miss
        cache.get("SELECT 2");

        assert_eq!(cache.hits(), 2);
        assert_eq!(cache.misses(), 1);
        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1");
        cache.insert("SELECT 2");
        cache.get("SELECT 1");

        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.misses(), 0);
    }

    #[test]
    fn test_statement_key_hash_equality() {
        use std::collections::hash_map::DefaultHasher;

        let key1 = StatementKey::new("SELECT * FROM users");
        let key2 = StatementKey::new("SELECT * FROM users");
        let key3 = StatementKey::new("SELECT * FROM orders");

        // Same SQL should produce equal keys and hashes
        assert_eq!(key1, key2);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        key1.hash(&mut hasher1);
        key2.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());

        // Different SQL should produce unequal keys
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_get_or_insert_closure_called_on_miss() {
        let mut cache = PreparedStatementCache::new(10);
        let mut called = false;

        cache.get_or_insert("SELECT 1", || {
            called = true;
        });

        assert!(called);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 0);
    }

    #[test]
    fn test_get_or_insert_closure_not_called_on_hit() {
        let mut cache = PreparedStatementCache::new(10);
        cache.insert("SELECT 1");

        let mut called = false;
        cache.get_or_insert("SELECT 1", || {
            called = true;
        });

        assert!(!called);
        assert_eq!(cache.hits(), 1);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_zero_capacity_panics() {
        let _ = PreparedStatementCache::new(0);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = PreparedStatementCache::new(10);
        cache.insert("SQL1");
        cache.insert("SQL2");
        cache.get("SQL1"); // hit
        cache.get("SQL3"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.5).abs() < 0.01);
        assert_eq!(stats.size, 2);
        assert_eq!(stats.capacity, 10);
    }
}
