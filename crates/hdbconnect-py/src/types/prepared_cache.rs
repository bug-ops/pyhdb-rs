//! True prepared statement LRU cache implementation.
//!
//! Provides `PreparedStatementCache<S>` for storing and reusing actual
//! `PreparedStatement` handles from hdbconnect. Unlike the deprecated
//! tracking-only cache, this implementation provides real performance
//! benefits by avoiding repeated statement preparation on the server.
//!
//! # Design
//!
//! - SQL string as cache key (not hash) for correctness
//! - Per-connection cache (`PreparedStatement` handles are tied to connections)
//! - LRU eviction policy with configurable capacity
//! - RAII cleanup: dropped statements are properly deallocated
//!
//! # Thread Safety
//!
//! The cache itself is not thread-safe. Callers must wrap it in appropriate
//! synchronization primitives:
//! - Sync: `parking_lot::Mutex<PreparedStatementCache<...>>`
//! - Async: `tokio::sync::Mutex<PreparedStatementCache<...>>`

use std::num::NonZeroUsize;
use std::time::Instant;

use lru::LruCache;

/// Default number of prepared statements to cache per connection.
pub const DEFAULT_CACHE_CAPACITY: usize = 16;

/// Cached prepared statement with execution metadata.
///
/// Wraps the actual prepared statement handle with usage statistics
/// for monitoring and debugging.
#[derive(Debug)]
pub struct CachedPreparedStatement<S> {
    /// The actual prepared statement handle.
    pub statement: S,
    /// Number of times this statement has been executed.
    pub use_count: u64,
    /// Timestamp of last use.
    pub last_used: Instant,
}

impl<S> CachedPreparedStatement<S> {
    /// Creates a new cached statement wrapper.
    pub fn new(statement: S) -> Self {
        Self {
            statement,
            use_count: 1,
            last_used: Instant::now(),
        }
    }

    /// Records a use of this statement, updating metadata.
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.last_used = Instant::now();
    }
}

/// Cache statistics snapshot.
///
/// Provides a point-in-time view of cache performance metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CacheStatistics {
    /// Current number of cached statements.
    pub size: usize,
    /// Maximum cache capacity.
    pub capacity: usize,
    /// Total number of cache hits.
    pub hits: u64,
    /// Total number of cache misses.
    pub misses: u64,
    /// Total number of evictions.
    pub evictions: u64,
    /// Cache hit rate (0.0 - 1.0).
    pub hit_rate: f64,
}

impl Default for CacheStatistics {
    fn default() -> Self {
        Self {
            size: 0,
            capacity: DEFAULT_CACHE_CAPACITY,
            hits: 0,
            misses: 0,
            evictions: 0,
            hit_rate: 0.0,
        }
    }
}

/// LRU cache for prepared statements.
///
/// Stores actual `PreparedStatement` handles and manages their lifecycle.
/// When the cache is full, the least recently used statement is evicted
/// and properly dropped (triggering server-side deallocation via RAII).
///
/// # Type Parameters
///
/// * `S` - The prepared statement type (sync or async variant)
///
/// # Example
///
/// ```ignore
/// use hdbconnect::PreparedStatement;
///
/// let mut cache = PreparedStatementCache::<PreparedStatement>::new(16);
///
/// // Insert a prepared statement
/// let stmt = connection.prepare("SELECT * FROM users WHERE id = ?")?;
/// cache.insert("SELECT * FROM users WHERE id = ?".to_string(), stmt);
///
/// // Later, retrieve and execute
/// if let Some(cached) = cache.get_mut("SELECT * FROM users WHERE id = ?") {
///     cached.statement.execute(&(42,))?;
/// }
/// ```
#[derive(Debug)]
pub struct PreparedStatementCache<S> {
    cache: LruCache<String, CachedPreparedStatement<S>>,
    capacity: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl<S> PreparedStatementCache<S> {
    /// Creates a new cache with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of statements to cache. If 0, uses default.
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0 after defaulting (should not happen with default).
    pub fn new(capacity: usize) -> Self {
        let capacity = if capacity == 0 {
            DEFAULT_CACHE_CAPACITY
        } else {
            capacity
        };
        let cap = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            cache: LruCache::new(cap),
            capacity,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Creates a cache with the default capacity.
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CACHE_CAPACITY)
    }

    /// Returns the maximum capacity of the cache.
    pub const fn capacity(&self) -> usize {
        self.capacity
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

    /// Returns the total number of evictions.
    pub const fn evictions(&self) -> u64 {
        self.evictions
    }

    /// Returns the cache hit rate as a value between 0.0 and 1.0.
    #[allow(clippy::cast_precision_loss)]
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
        self.cache.peek(sql).is_some()
    }

    /// Gets a mutable reference to a cached statement, updating LRU order.
    ///
    /// Updates hit/miss statistics and the statement's use count.
    /// Returns `None` and increments miss count if not found.
    pub fn get_mut(&mut self, sql: &str) -> Option<&mut CachedPreparedStatement<S>> {
        if let Some(cached) = self.cache.get_mut(sql) {
            self.hits += 1;
            cached.record_use();
            Some(cached)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Peeks at a cached statement without updating LRU order or statistics.
    pub fn peek(&self, sql: &str) -> Option<&CachedPreparedStatement<S>> {
        self.cache.peek(sql)
    }

    /// Inserts a new prepared statement into the cache.
    ///
    /// If the cache is at capacity, the least recently used statement is evicted.
    /// Returns the evicted statement if any.
    ///
    /// # Arguments
    ///
    /// * `sql` - The SQL string used as cache key
    /// * `statement` - The prepared statement handle to cache
    pub fn insert(&mut self, sql: String, statement: S) -> Option<S> {
        let evicted = if self.cache.len() >= self.capacity {
            self.cache.pop_lru().map(|(_, cached)| {
                self.evictions += 1;
                cached.statement
            })
        } else {
            None
        };

        let cached = CachedPreparedStatement::new(statement);
        self.cache.put(sql, cached);
        evicted
    }

    /// Removes a statement from the cache.
    ///
    /// Useful for invalidating stale statements after schema changes.
    /// Returns the removed statement if it was present.
    pub fn remove(&mut self, sql: &str) -> Option<S> {
        self.cache.pop(sql).map(|cached| cached.statement)
    }

    /// Clears all cached statements.
    ///
    /// Returns all cached statements for explicit cleanup.
    /// Resets all statistics.
    pub fn clear(&mut self) -> Vec<S> {
        let mut statements = Vec::with_capacity(self.cache.len());

        // Drain all entries from the cache
        while let Some((_, cached)) = self.cache.pop_lru() {
            statements.push(cached.statement);
        }

        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
        statements
    }

    /// Returns a snapshot of cache statistics.
    pub fn stats(&self) -> CacheStatistics {
        CacheStatistics {
            size: self.cache.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: self.hit_rate(),
        }
    }

    /// Resets statistics without clearing the cache.
    pub const fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockStatement {
        sql: String,
    }

    impl MockStatement {
        fn new(sql: &str) -> Self {
            Self {
                sql: sql.to_string(),
            }
        }
    }

    #[test]
    fn test_cache_new() {
        let cache = PreparedStatementCache::<MockStatement>::new(16);
        assert_eq!(cache.capacity(), 16);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_default_capacity() {
        let cache = PreparedStatementCache::<MockStatement>::with_default_capacity();
        assert_eq!(cache.capacity(), DEFAULT_CACHE_CAPACITY);
    }

    #[test]
    fn test_cache_zero_capacity_uses_default() {
        let cache = PreparedStatementCache::<MockStatement>::new(0);
        assert_eq!(cache.capacity(), DEFAULT_CACHE_CAPACITY);
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let cached = cache.get_mut("SELECT 1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().statement.sql, "SELECT 1");
    }

    #[test]
    fn test_cache_hit_miss_stats() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));

        // Hit
        let _ = cache.get_mut("SELECT 1");
        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.misses(), 0);

        // Miss
        let _ = cache.get_mut("SELECT 2");
        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.misses(), 1);

        assert!((cache.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = PreparedStatementCache::new(2);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        cache.insert("SELECT 2".to_string(), MockStatement::new("SELECT 2"));
        assert_eq!(cache.len(), 2);

        // Access SELECT 1 to make it recently used
        let _ = cache.get_mut("SELECT 1");

        // Insert SELECT 3, should evict SELECT 2 (least recently used)
        let evicted = cache.insert("SELECT 3".to_string(), MockStatement::new("SELECT 3"));
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().sql, "SELECT 2");

        assert_eq!(cache.len(), 2);
        assert!(cache.contains("SELECT 1"));
        assert!(!cache.contains("SELECT 2"));
        assert!(cache.contains("SELECT 3"));
        assert_eq!(cache.evictions(), 1);
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        assert_eq!(cache.len(), 1);

        let removed = cache.remove("SELECT 1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().sql, "SELECT 1");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        cache.insert("SELECT 2".to_string(), MockStatement::new("SELECT 2"));
        let _ = cache.get_mut("SELECT 1");

        let cleared = cache.clear();
        assert_eq!(cleared.len(), 2);
        assert!(cache.is_empty());
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.misses(), 0);
        assert_eq!(cache.evictions(), 0);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        cache.insert("SELECT 2".to_string(), MockStatement::new("SELECT 2"));
        let _ = cache.get_mut("SELECT 1"); // hit
        let _ = cache.get_mut("SELECT 3"); // miss

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.evictions, 0);
        assert!((stats.hit_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_reset_stats() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        let _ = cache.get_mut("SELECT 1");
        let _ = cache.get_mut("SELECT 2");

        cache.reset_stats();
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.misses(), 0);
        assert_eq!(cache.evictions(), 0);
        // Cache contents should remain
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_contains_without_lru_update() {
        let mut cache = PreparedStatementCache::new(2);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        cache.insert("SELECT 2".to_string(), MockStatement::new("SELECT 2"));

        // contains() should not update LRU order
        assert!(cache.contains("SELECT 1"));

        // Insert SELECT 3, should evict SELECT 1 (still LRU since contains doesn't update)
        let evicted = cache.insert("SELECT 3".to_string(), MockStatement::new("SELECT 3"));
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().sql, "SELECT 1");
    }

    #[test]
    fn test_cache_peek_without_lru_update() {
        let mut cache = PreparedStatementCache::new(2);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));
        cache.insert("SELECT 2".to_string(), MockStatement::new("SELECT 2"));

        // peek() should not update LRU order or stats
        let peeked = cache.peek("SELECT 1");
        assert!(peeked.is_some());
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.misses(), 0);

        // Insert SELECT 3, should evict SELECT 1 (still LRU since peek doesn't update)
        let evicted = cache.insert("SELECT 3".to_string(), MockStatement::new("SELECT 3"));
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().sql, "SELECT 1");
    }

    #[test]
    fn test_cached_statement_use_count() {
        let mut cache = PreparedStatementCache::new(10);

        cache.insert("SELECT 1".to_string(), MockStatement::new("SELECT 1"));

        // Initial use_count is 1
        {
            let cached = cache.peek("SELECT 1").unwrap();
            assert_eq!(cached.use_count, 1);
        }

        // After get_mut, use_count should be 2
        let _ = cache.get_mut("SELECT 1");
        {
            let cached = cache.peek("SELECT 1").unwrap();
            assert_eq!(cached.use_count, 2);
        }

        // After another get_mut, use_count should be 3
        let _ = cache.get_mut("SELECT 1");
        {
            let cached = cache.peek("SELECT 1").unwrap();
            assert_eq!(cached.use_count, 3);
        }
    }

    #[test]
    fn test_hit_rate_zero_when_empty() {
        let cache = PreparedStatementCache::<MockStatement>::new(10);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_statistics_default() {
        let stats = CacheStatistics::default();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.capacity, DEFAULT_CACHE_CAPACITY);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[test]
    fn test_cache_statistics_copy() {
        let stats = CacheStatistics {
            size: 5,
            capacity: 10,
            hits: 100,
            misses: 50,
            evictions: 10,
            hit_rate: 0.667,
        };
        let copied = stats;
        assert_eq!(copied.hits, 100);
        assert_eq!(copied.misses, 50);
    }

    #[test]
    fn test_cache_statistics_partial_eq() {
        let stats1 = CacheStatistics {
            size: 5,
            capacity: 10,
            hits: 100,
            misses: 50,
            evictions: 10,
            hit_rate: 0.667,
        };
        let stats2 = CacheStatistics {
            size: 5,
            capacity: 10,
            hits: 100,
            misses: 50,
            evictions: 10,
            hit_rate: 0.667,
        };
        assert_eq!(stats1, stats2);
    }

    #[test]
    fn test_cached_statement_new() {
        let stmt = MockStatement::new("SELECT 1");
        let cached = CachedPreparedStatement::new(stmt);

        assert_eq!(cached.use_count, 1);
        assert_eq!(cached.statement.sql, "SELECT 1");
    }

    #[test]
    fn test_cached_statement_record_use() {
        let stmt = MockStatement::new("SELECT 1");
        let mut cached = CachedPreparedStatement::new(stmt);
        let first_use = cached.last_used;

        std::thread::sleep(std::time::Duration::from_millis(10));
        cached.record_use();

        assert_eq!(cached.use_count, 2);
        assert!(cached.last_used >= first_use);
    }
}
