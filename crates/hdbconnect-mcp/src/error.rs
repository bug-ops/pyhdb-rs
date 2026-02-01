use std::time::Duration;

use rmcp::ErrorData;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Connection error: {0}")]
    Connection(#[from] hdbconnect::HdbError),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Read-only mode: {0}")]
    ReadOnlyViolation(String),

    #[error("Query timeout after {0:?}")]
    QueryTimeout(Duration),

    #[error("Schema access denied: {0}")]
    SchemaAccessDenied(String),

    #[error("Transport error: {0}")]
    Transport(String),
}

impl Error {
    pub const fn read_only_violation(msg: String) -> Self {
        Self::ReadOnlyViolation(msg)
    }

    #[must_use]
    pub const fn is_read_only_violation(&self) -> bool {
        matches!(self, Self::ReadOnlyViolation(_))
    }

    #[must_use]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::QueryTimeout(_))
    }

    #[must_use]
    pub const fn is_schema_denied(&self) -> bool {
        matches!(self, Self::SchemaAccessDenied(_))
    }

    #[must_use]
    pub const fn is_config(&self) -> bool {
        matches!(self, Self::Config(_))
    }

    #[must_use]
    pub const fn is_pool_exhausted(&self) -> bool {
        matches!(self, Self::PoolExhausted)
    }

    #[must_use]
    pub const fn is_transport(&self) -> bool {
        matches!(self, Self::Transport(_))
    }

    #[must_use]
    pub const fn is_query(&self) -> bool {
        matches!(self, Self::Query(_))
    }
}

/// Convert our Error type to rmcp `ErrorData`
impl From<Error> for ErrorData {
    fn from(err: Error) -> Self {
        match err {
            Error::Connection(e) => {
                Self::internal_error(format!("Database connection error: {e}"), None)
            }
            Error::Query(msg) => Self::internal_error(format!("Query error: {msg}"), None),
            Error::Config(msg) => Self::invalid_params(format!("Configuration error: {msg}"), None),
            Error::PoolExhausted => Self::internal_error("Connection pool exhausted", None),
            Error::ReadOnlyViolation(msg) => {
                Self::invalid_params(format!("Read-only mode violation: {msg}"), None)
            }
            Error::QueryTimeout(duration) => {
                Self::internal_error(format!("Query timeout after {duration:?}"), None)
            }
            Error::SchemaAccessDenied(schema) => {
                Self::invalid_params(format!("Schema access denied: {schema}"), None)
            }
            Error::Transport(msg) => Self::internal_error(format!("Transport error: {msg}"), None),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_violation_predicate() {
        let err = Error::ReadOnlyViolation("test".to_string());
        assert!(err.is_read_only_violation());
        assert!(!err.is_timeout());
        assert!(!err.is_schema_denied());
        assert!(!err.is_config());
    }

    #[test]
    fn test_timeout_predicate() {
        let err = Error::QueryTimeout(Duration::from_secs(30));
        assert!(err.is_timeout());
        assert!(!err.is_read_only_violation());
    }

    #[test]
    fn test_schema_denied_predicate() {
        let err = Error::SchemaAccessDenied("SYS".to_string());
        assert!(err.is_schema_denied());
        assert!(!err.is_timeout());
    }

    #[test]
    fn test_config_predicate() {
        let err = Error::Config("invalid config".to_string());
        assert!(err.is_config());
        assert!(!err.is_schema_denied());
    }

    #[test]
    fn test_pool_exhausted_predicate() {
        let err = Error::PoolExhausted;
        assert!(err.is_pool_exhausted());
        assert!(!err.is_config());
    }

    #[test]
    fn test_transport_predicate() {
        let err = Error::Transport("connection refused".to_string());
        assert!(err.is_transport());
        assert!(!err.is_pool_exhausted());
    }

    #[test]
    fn test_query_predicate() {
        let err = Error::Query("syntax error".to_string());
        assert!(err.is_query());
        assert!(!err.is_transport());
    }

    #[test]
    fn test_error_display() {
        let err = Error::ReadOnlyViolation("INSERT not allowed".to_string());
        assert_eq!(err.to_string(), "Read-only mode: INSERT not allowed");

        let err = Error::QueryTimeout(Duration::from_secs(30));
        assert!(err.to_string().contains("30"));

        let err = Error::SchemaAccessDenied("SECRET".to_string());
        assert!(err.to_string().contains("SECRET"));
    }

    #[test]
    fn test_read_only_violation_constructor() {
        let err = Error::read_only_violation("DML blocked".to_string());
        assert!(err.is_read_only_violation());
        assert!(err.to_string().contains("DML blocked"));
    }

    #[test]
    fn test_error_to_error_data_read_only() {
        let err = Error::ReadOnlyViolation("test".to_string());
        let data: ErrorData = err.into();
        assert!(data.message.contains("Read-only mode violation"));
    }

    #[test]
    fn test_error_to_error_data_timeout() {
        let err = Error::QueryTimeout(Duration::from_secs(60));
        let data: ErrorData = err.into();
        assert!(data.message.contains("timeout"));
    }

    #[test]
    fn test_error_to_error_data_schema_denied() {
        let err = Error::SchemaAccessDenied("PRIVATE".to_string());
        let data: ErrorData = err.into();
        assert!(data.message.contains("Schema access denied"));
        assert!(data.message.contains("PRIVATE"));
    }

    #[test]
    fn test_error_to_error_data_config() {
        let err = Error::Config("missing URL".to_string());
        let data: ErrorData = err.into();
        assert!(data.message.contains("Configuration error"));
    }

    #[test]
    fn test_error_to_error_data_pool_exhausted() {
        let err = Error::PoolExhausted;
        let data: ErrorData = err.into();
        assert!(data.message.contains("pool exhausted"));
    }

    #[test]
    fn test_error_to_error_data_transport() {
        let err = Error::Transport("connection refused".to_string());
        let data: ErrorData = err.into();
        assert!(data.message.contains("Transport error"));
    }

    #[test]
    fn test_error_to_error_data_query() {
        let err = Error::Query("invalid SQL".to_string());
        let data: ErrorData = err.into();
        assert!(data.message.contains("Query error"));
    }
}
