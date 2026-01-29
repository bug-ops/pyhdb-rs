//! Common utilities for async connection implementations.
//!
//! Extracts shared logic from `AsyncPyConnection` and `PooledConnection` to
//! eliminate code duplication while preserving type safety.

use pyo3::prelude::*;

use crate::config::{DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig};
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;
// Re-export from centralized utils module
pub use crate::utils::{VALIDATION_QUERY, validate_non_negative_f64, validate_positive_u32};

/// Connection state error for consistent error messages.
#[derive(Debug, Clone, Copy)]
pub enum ConnectionState {
    /// Direct connection is closed.
    Closed,
    /// Pooled connection has been returned to the pool.
    ReturnedToPool,
}

impl ConnectionState {
    /// Returns the error message for this connection state.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Closed => "connection is closed",
            Self::ReturnedToPool => "connection returned to pool",
        }
    }

    /// Converts this state into a `PyHdbError`.
    #[must_use]
    pub fn into_error(self) -> PyHdbError {
        PyHdbError::operational(self.message())
    }
}

impl From<ConnectionState> for PyErr {
    fn from(state: ConnectionState) -> Self {
        state.into_error().into()
    }
}

/// Executes commit on an async HANA connection.
pub async fn commit_impl(connection: &mut hdbconnect_async::Connection) -> PyResult<()> {
    connection.commit().await.map_err(PyHdbError::from)?;
    Ok(())
}

/// Executes rollback on an async HANA connection.
pub async fn rollback_impl(connection: &mut hdbconnect_async::Connection) -> PyResult<()> {
    connection.rollback().await.map_err(PyHdbError::from)?;
    Ok(())
}

/// Extracts batch size from optional config, using default if not provided.
#[must_use]
pub fn get_batch_size(config: Option<&PyArrowConfig>) -> usize {
    config.map_or(DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig::batch_size)
}

/// Executes a query and returns an Arrow `RecordBatchReader`.
pub async fn execute_arrow_impl(
    connection: &mut hdbconnect_async::Connection,
    sql: &str,
    batch_size: usize,
) -> PyResult<PyRecordBatchReader> {
    let rs = connection.query(sql).await.map_err(PyHdbError::from)?;
    PyRecordBatchReader::from_resultset_async(rs, batch_size)
}

/// Executes a query without returning results (for cursor execute).
pub async fn execute_query_impl(
    connection: &mut hdbconnect_async::Connection,
    sql: &str,
) -> PyResult<()> {
    connection.query(sql).await.map_err(PyHdbError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_query_constant() {
        assert_eq!(VALIDATION_QUERY, "SELECT 1 FROM DUMMY");
    }

    #[test]
    fn test_connection_state_closed_message() {
        let state = ConnectionState::Closed;
        assert_eq!(state.message(), "connection is closed");
    }

    #[test]
    fn test_connection_state_returned_to_pool_message() {
        let state = ConnectionState::ReturnedToPool;
        assert_eq!(state.message(), "connection returned to pool");
    }

    #[test]
    fn test_connection_state_into_error() {
        let state = ConnectionState::Closed;
        let error = state.into_error();
        assert!(error.to_string().contains("connection is closed"));
    }

    #[test]
    fn test_connection_state_clone() {
        let state = ConnectionState::Closed;
        let cloned = state;
        assert_eq!(cloned.message(), state.message());
    }

    #[test]
    fn test_connection_state_debug() {
        let state = ConnectionState::Closed;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Closed"));

        let state = ConnectionState::ReturnedToPool;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("ReturnedToPool"));
    }

    #[test]
    fn test_validate_positive_u32_valid() {
        assert!(validate_positive_u32(1, "test_param").is_ok());
        assert!(validate_positive_u32(100, "test_param").is_ok());
        assert!(validate_positive_u32(u32::MAX, "test_param").is_ok());
    }

    #[test]
    fn test_validate_positive_u32_zero() {
        let result = validate_positive_u32(0, "fetch_size");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_non_negative_f64_valid() {
        assert!(validate_non_negative_f64(None, "test_param").is_ok());
        assert!(validate_non_negative_f64(Some(0.0), "test_param").is_ok());
        assert!(validate_non_negative_f64(Some(1.5), "test_param").is_ok());
    }

    #[test]
    fn test_validate_non_negative_f64_negative() {
        let result = validate_non_negative_f64(Some(-1.0), "read_timeout");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_batch_size_default() {
        assert_eq!(get_batch_size(None), DEFAULT_ARROW_BATCH_SIZE);
    }

    #[test]
    fn test_get_batch_size_custom() {
        let config = PyArrowConfig::default();
        assert_eq!(get_batch_size(Some(&config)), DEFAULT_ARROW_BATCH_SIZE);
    }
}
