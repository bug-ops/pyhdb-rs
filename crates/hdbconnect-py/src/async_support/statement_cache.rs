//! Prepared statement LRU cache.
//!
//! Provides caching for prepared statements to reduce PREPARE overhead.
//!
//! # Technical Debt
//!
//! TODO(PERF-005): Use `Cow<'a, str>` or `hashbrown`'s `raw_entry` API to avoid
//! String allocation on cache lookup. Currently `StatementKey::new()` allocates
//! a new String for every lookup, even cache hits. This causes unnecessary
//! allocations for frequently executed queries. Consider:
//! ```ignore
//! pub struct StatementKey<'a> {
//!     sql: Cow<'a, str>,
//! }
//! ```

use std::hash::{Hash, Hasher};

use lru::LruCache;
use std::num::NonZeroUsize;

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
    pub fn new(sql: impl Into<String>) -> Self {
        Self { sql: sql.into() }
    }

    pub fn sql(&self) -> &str {
        &self.sql
    }
}

#[derive(Debug, Clone)]
pub struct CachedStatement {
    pub sql: String,
    pub use_count: u64,
}

#[derive(Debug)]
pub struct PreparedStatementCache {
    cache: LruCache<StatementKey, CachedStatement>,
    hits: u64,
    misses: u64,
}

impl PreparedStatementCache {
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            cache: LruCache::new(capacity),
            hits: 0,
            misses: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.cache.cap().get()
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub const fn hits(&self) -> u64 {
        self.hits
    }

    pub const fn misses(&self) -> u64 {
        self.misses
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn contains(&self, sql: &str) -> bool {
        let key = StatementKey::new(sql);
        self.cache.peek(&key).is_some()
    }

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

    pub fn insert(&mut self, sql: impl Into<String>) -> Option<CachedStatement> {
        let sql = sql.into();
        let key = StatementKey::new(&sql);
        let cached = CachedStatement { sql, use_count: 1 };
        self.cache.push(key, cached).map(|(_, v)| v)
    }

    pub fn record_use(&mut self, sql: &str) {
        let key = StatementKey::new(sql);
        if let Some(cached) = self.cache.get_mut(&key) {
            cached.use_count += 1;
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    pub fn get_or_insert<F>(&mut self, sql: &str, f: F) -> &CachedStatement
    where
        F: FnOnce(),
    {
        let key = StatementKey::new(sql);
        if let Some(cached) = self.cache.get_mut(&key) {
            self.hits += 1;
            cached.use_count += 1;
            return self.cache.peek(&key).expect("just accessed");
        }

        self.misses += 1;
        f();
        let cached = CachedStatement {
            sql: sql.to_string(),
            use_count: 1,
        };
        self.cache.push(key.clone(), cached);
        self.cache.peek(&key).expect("just inserted")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO(TEST): Add test_statement_key_hash_equality - verify hash consistency
    // TODO(TEST): Add test_get_or_insert_closure_called - verify closure is called on miss
    // TODO(TEST): Add test_get_or_insert_closure_not_called - verify closure not called on hit
    // TODO(TEST): Add test_zero_capacity_panics - verify panic on 0 capacity

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
}
