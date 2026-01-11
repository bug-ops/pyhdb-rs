//! Tests for async support module.
//!
//! Unit tests that don't require a live HANA connection.
//! Integration tests that require HANA should be in `tests/` directory.

use super::pool::PoolConfig;
use super::statement_cache::PreparedStatementCache;

// =============================================================================
// Statement Cache Tests
// =============================================================================

#[test]
fn test_statement_cache_basic() {
    let mut cache = PreparedStatementCache::new(10);

    assert!(cache.is_empty());
    assert_eq!(cache.capacity(), 10);

    cache.insert("SELECT * FROM users WHERE id = ?");
    assert_eq!(cache.len(), 1);
    assert!(cache.contains("SELECT * FROM users WHERE id = ?"));
}

#[test]
fn test_statement_cache_hit_miss() {
    let mut cache = PreparedStatementCache::new(10);

    cache.insert("SELECT 1 FROM DUMMY");

    // Hit
    let result = cache.get("SELECT 1 FROM DUMMY");
    assert!(result.is_some());
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 0);

    // Miss
    let result = cache.get("SELECT 2 FROM DUMMY");
    assert!(result.is_none());
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 1);
}

#[test]
fn test_statement_cache_lru_eviction() {
    let mut cache = PreparedStatementCache::new(2);

    cache.insert("SQL_1");
    cache.insert("SQL_2");

    // Access SQL_1 to make it recently used
    cache.get("SQL_1");

    // Insert SQL_3 - should evict SQL_2
    cache.insert("SQL_3");

    assert!(cache.contains("SQL_1"));
    assert!(!cache.contains("SQL_2"));
    assert!(cache.contains("SQL_3"));
}

#[test]
fn test_statement_cache_hit_rate() {
    let mut cache = PreparedStatementCache::new(10);

    cache.insert("SQL");

    // 3 hits
    cache.get("SQL");
    cache.get("SQL");
    cache.get("SQL");

    // 1 miss
    cache.get("OTHER");

    assert_eq!(cache.hits(), 3);
    assert_eq!(cache.misses(), 1);
    assert!((cache.hit_rate() - 0.75).abs() < 0.01);
}

#[test]
fn test_statement_cache_record_use() {
    let mut cache = PreparedStatementCache::new(10);

    cache.insert("SQL");
    cache.record_use("SQL");
    cache.record_use("SQL");

    let cached = cache.get("SQL").unwrap();
    // Initial use_count is 1, plus 2 record_use calls
    assert_eq!(cached.use_count, 3);
}

// =============================================================================
// PoolConfig Tests
// =============================================================================

#[test]
fn test_pool_config_default_values() {
    let config = PoolConfig::default();

    assert_eq!(config.max_size, 10);
    assert!(config.min_idle.is_none());
    assert_eq!(config.connection_timeout_secs, 30);
    assert_eq!(config.statement_cache_size, 0);
}

#[test]
fn test_pool_config_custom_values() {
    let config = PoolConfig {
        max_size: 20,
        min_idle: Some(5),
        connection_timeout_secs: 60,
        statement_cache_size: 100,
    };

    assert_eq!(config.max_size, 20);
    assert_eq!(config.min_idle, Some(5));
    assert_eq!(config.connection_timeout_secs, 60);
    assert_eq!(config.statement_cache_size, 100);
}

#[test]
fn test_pool_config_clone() {
    let config = PoolConfig {
        max_size: 15,
        min_idle: Some(3),
        connection_timeout_secs: 45,
        statement_cache_size: 50,
    };

    let cloned = config.clone();

    assert_eq!(config.max_size, cloned.max_size);
    assert_eq!(config.min_idle, cloned.min_idle);
    assert_eq!(
        config.connection_timeout_secs,
        cloned.connection_timeout_secs
    );
    assert_eq!(config.statement_cache_size, cloned.statement_cache_size);
}

#[test]
fn test_pool_config_debug() {
    let config = PoolConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("max_size"));
    assert!(debug_str.contains("min_idle"));
    assert!(debug_str.contains("connection_timeout_secs"));
    assert!(debug_str.contains("statement_cache_size"));
}

// =============================================================================
// CacheStats Tests
// =============================================================================

#[test]
fn test_cache_stats_initial() {
    let cache = PreparedStatementCache::new(10);
    let stats = cache.stats();

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert!((stats.hit_rate - 0.0).abs() < f64::EPSILON);
    assert_eq!(stats.size, 0);
    assert_eq!(stats.capacity, 10);
}

#[test]
fn test_cache_stats_after_operations() {
    let mut cache = PreparedStatementCache::new(5);

    cache.insert("SQL1");
    cache.insert("SQL2");
    cache.get("SQL1"); // hit
    cache.get("SQL3"); // miss

    let stats = cache.stats();

    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate - 0.5).abs() < 0.01);
    assert_eq!(stats.size, 2);
    assert_eq!(stats.capacity, 5);
}
