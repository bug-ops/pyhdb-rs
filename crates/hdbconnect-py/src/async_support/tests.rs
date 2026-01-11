//! Tests for async support module.
//!
//! TODO(TEST): Add unit tests for AsyncPyConnection without live HANA:
//!   - test_connection_state_transitions (Connected -> Disconnected)
//!   - test_connection_autocommit_property
//!   - test_connection_repr_format
//!
//! TODO(TEST): Add unit tests for AsyncPyCursor:
//!   - test_cursor_creation_from_connection
//!   - test_cursor_close_clears_state
//!   - test_cursor_repr_format
//!   - test_cursor_arraysize_property
//!
//! TODO(TEST): Add unit tests for PoolConfig:
//!   - test_pool_config_default_values
//!   - test_pool_config_custom_values
//!
//! TODO(TEST): Add unit tests for PoolStatus:
//!   - test_pool_status_repr_format
//!
//! TODO(TEST): Add integration tests (require HANA):
//!   - test_async_connection_commit_rollback
//!   - test_async_connection_context_manager
//!   - test_async_cursor_execute_dml
//!   - test_pool_health_check_recycle
//!   - test_pool_exhaustion_timeout

use super::statement_cache::PreparedStatementCache;

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
