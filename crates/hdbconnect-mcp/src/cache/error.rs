//! Cache error types

use thiserror::Error;

/// Cache operation errors
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Cache operation timeout")]
    Timeout,

    #[error("Value too large: {size} bytes (max: {max} bytes)")]
    ValueTooLarge { size: usize, max: usize },

    #[error("Cache error: {0}")]
    Other(String),
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_display() {
        let err = CacheError::Connection("refused".to_string());
        assert!(err.to_string().contains("connection"));
        assert!(err.to_string().contains("refused"));
    }

    #[test]
    fn test_serialization_error_display() {
        let err = CacheError::Serialization("invalid utf8".to_string());
        assert!(err.to_string().contains("Serialization"));
    }

    #[test]
    fn test_deserialization_error_display() {
        let err = CacheError::Deserialization("unexpected token".to_string());
        assert!(err.to_string().contains("Deserialization"));
    }

    #[test]
    fn test_not_found_error_display() {
        let err = CacheError::NotFound("tbl_schema:TEST:USERS".to_string());
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("tbl_schema"));
    }

    #[test]
    fn test_timeout_error_display() {
        let err = CacheError::Timeout;
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_value_too_large_error_display() {
        let err = CacheError::ValueTooLarge {
            size: 2_000_000,
            max: 1_048_576,
        };
        let msg = err.to_string();
        assert!(msg.contains("too large"));
        assert!(msg.contains("2000000"));
        assert!(msg.contains("1048576"));
    }

    #[test]
    fn test_other_error_display() {
        let err = CacheError::Other("unknown issue".to_string());
        assert!(err.to_string().contains("unknown issue"));
    }
}
