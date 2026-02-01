use std::time::Duration;

use rmcp::ErrorData;
use thiserror::Error;

use crate::config::DmlOperation;

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

    // DML-specific errors
    #[error("DML operations are disabled. Set allow_dml=true in configuration")]
    DmlDisabled,

    #[error("DML operation not allowed: {0}")]
    DmlOperationNotAllowed(DmlOperation),

    #[error("WHERE clause required for {0} statements")]
    DmlWhereClauseRequired(DmlOperation),

    #[error("Affected rows ({actual}) exceeds limit ({limit})")]
    DmlRowLimitExceeded { actual: u64, limit: u32 },

    #[error("DML operation cancelled by user")]
    DmlCancelled,

    #[error("Not a valid DML statement. Use INSERT, UPDATE, or DELETE")]
    DmlNotAStatement,
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

    #[must_use]
    pub const fn is_dml_error(&self) -> bool {
        matches!(
            self,
            Self::DmlDisabled
                | Self::DmlOperationNotAllowed(_)
                | Self::DmlWhereClauseRequired(_)
                | Self::DmlRowLimitExceeded { .. }
                | Self::DmlCancelled
                | Self::DmlNotAStatement
        )
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
            // DML errors
            Error::DmlDisabled => Self::invalid_params(
                "DML operations are disabled. Set allow_dml=true in configuration",
                None,
            ),
            Error::DmlOperationNotAllowed(op) => {
                Self::invalid_params(format!("DML operation not allowed: {op}"), None)
            }
            Error::DmlWhereClauseRequired(op) => {
                Self::invalid_params(format!("WHERE clause required for {op} statements"), None)
            }
            Error::DmlRowLimitExceeded { actual, limit } => Self::invalid_params(
                format!("Affected rows ({actual}) exceeds limit ({limit})"),
                None,
            ),
            Error::DmlCancelled => Self::invalid_params("DML operation cancelled by user", None),
            Error::DmlNotAStatement => Self::invalid_params(
                "Not a valid DML statement. Use INSERT, UPDATE, or DELETE",
                None,
            ),
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

    // DML error tests
    #[test]
    fn test_dml_disabled_error() {
        let err = Error::DmlDisabled;
        assert!(err.is_dml_error());
        assert!(err.to_string().contains("disabled"));
    }

    #[test]
    fn test_dml_operation_not_allowed_error() {
        let err = Error::DmlOperationNotAllowed(DmlOperation::Delete);
        assert!(err.is_dml_error());
        assert!(err.to_string().contains("DELETE"));
    }

    #[test]
    fn test_dml_where_clause_required_error() {
        let err = Error::DmlWhereClauseRequired(DmlOperation::Update);
        assert!(err.is_dml_error());
        assert!(err.to_string().contains("WHERE"));
        assert!(err.to_string().contains("UPDATE"));
    }

    #[test]
    fn test_dml_row_limit_exceeded_error() {
        let err = Error::DmlRowLimitExceeded {
            actual: 5000,
            limit: 1000,
        };
        assert!(err.is_dml_error());
        assert!(err.to_string().contains("5000"));
        assert!(err.to_string().contains("1000"));
    }

    #[test]
    fn test_dml_cancelled_error() {
        let err = Error::DmlCancelled;
        assert!(err.is_dml_error());
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn test_dml_not_a_statement_error() {
        let err = Error::DmlNotAStatement;
        assert!(err.is_dml_error());
        assert!(err.to_string().contains("INSERT"));
    }

    #[test]
    fn test_error_to_error_data_dml_disabled() {
        let err = Error::DmlDisabled;
        let data: ErrorData = err.into();
        assert!(data.message.contains("disabled"));
        assert!(data.message.contains("allow_dml"));
    }

    #[test]
    fn test_error_to_error_data_dml_row_limit() {
        let err = Error::DmlRowLimitExceeded {
            actual: 2000,
            limit: 500,
        };
        let data: ErrorData = err.into();
        assert!(data.message.contains("2000"));
        assert!(data.message.contains("500"));
    }
}
