//! Error types and Python exception mapping.
//!
//! Maps HANA errors to DB-API 2.0 Python exceptions:
//! - `InterfaceError`: connection parameters, driver issues
//! - `OperationalError`: connection lost, timeout
//! - `ProgrammingError`: SQL syntax, wrong table name
//! - `IntegrityError`: constraint violation
//! - `DataError`: value conversion issues
//! - `NotSupportedError`: unsupported feature
//! - `InternalError`: unexpected internal error

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::{PyErr, create_exception};
use thiserror::Error;

// DB-API 2.0 exception hierarchy
create_exception!(hdbconnect, Error, PyException, "Base HANA error.");
create_exception!(hdbconnect, Warning, PyException, "Database warning.");
create_exception!(hdbconnect, InterfaceError, Error, "Interface error.");
create_exception!(hdbconnect, DatabaseError, Error, "Database error.");
create_exception!(hdbconnect, DataError, DatabaseError, "Data error.");
create_exception!(
    hdbconnect,
    OperationalError,
    DatabaseError,
    "Operational error."
);
create_exception!(
    hdbconnect,
    IntegrityError,
    DatabaseError,
    "Integrity error."
);
create_exception!(hdbconnect, InternalError, DatabaseError, "Internal error.");
create_exception!(
    hdbconnect,
    ProgrammingError,
    DatabaseError,
    "Programming error."
);
create_exception!(
    hdbconnect,
    NotSupportedError,
    DatabaseError,
    "Not supported error."
);

/// HANA Python driver error.
#[derive(Debug, Error)]
pub enum PyHdbError {
    /// Interface error (connection parameters, driver issues).
    #[error("InterfaceError: {0}")]
    Interface(String),

    /// Operational error (connection lost, timeout).
    #[error("OperationalError: {0}")]
    Operational(String),

    /// Programming error (SQL syntax, wrong table name).
    #[error("ProgrammingError: {0}")]
    Programming(String),

    /// Integrity error (constraint violation).
    #[error("IntegrityError: {0}")]
    Integrity(String),

    /// Data error (value conversion issues).
    #[error("DataError: {0}")]
    Data(String),

    /// Not supported error (unsupported feature).
    #[error("NotSupportedError: {0}")]
    NotSupported(String),

    /// Internal error (unexpected internal error).
    #[error("InternalError: {0}")]
    Internal(String),

    /// Arrow conversion error.
    #[error("ArrowError: {0}")]
    Arrow(String),
}

impl PyHdbError {
    /// Create an interface error.
    #[must_use]
    pub fn interface(msg: impl Into<String>) -> Self {
        Self::Interface(msg.into())
    }

    /// Create an operational error.
    #[must_use]
    pub fn operational(msg: impl Into<String>) -> Self {
        Self::Operational(msg.into())
    }

    /// Create a programming error.
    #[must_use]
    pub fn programming(msg: impl Into<String>) -> Self {
        Self::Programming(msg.into())
    }

    /// Create an integrity error.
    #[must_use]
    pub fn integrity(msg: impl Into<String>) -> Self {
        Self::Integrity(msg.into())
    }

    /// Create a data error.
    #[must_use]
    pub fn data(msg: impl Into<String>) -> Self {
        Self::Data(msg.into())
    }

    /// Create a not supported error.
    #[must_use]
    pub fn not_supported(msg: impl Into<String>) -> Self {
        Self::NotSupported(msg.into())
    }

    /// Create an internal error.
    #[must_use]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create an arrow error.
    #[must_use]
    pub fn arrow(msg: impl Into<String>) -> Self {
        Self::Arrow(msg.into())
    }
}

impl From<hdbconnect::HdbError> for PyHdbError {
    fn from(err: hdbconnect::HdbError) -> Self {
        map_hdbconnect_error(&err)
    }
}

impl From<hdbconnect_arrow::ArrowConversionError> for PyHdbError {
    fn from(err: hdbconnect_arrow::ArrowConversionError) -> Self {
        Self::Arrow(err.to_string())
    }
}

impl From<url::ParseError> for PyHdbError {
    fn from(err: url::ParseError) -> Self {
        Self::Interface(format!("invalid URL: {err}"))
    }
}

impl From<PyHdbError> for PyErr {
    fn from(err: PyHdbError) -> Self {
        match err {
            PyHdbError::Interface(msg) => InterfaceError::new_err(msg),
            PyHdbError::Operational(msg) => OperationalError::new_err(msg),
            PyHdbError::Programming(msg) => ProgrammingError::new_err(msg),
            PyHdbError::Integrity(msg) => IntegrityError::new_err(msg),
            PyHdbError::Data(msg) | PyHdbError::Arrow(msg) => DataError::new_err(msg),
            PyHdbError::NotSupported(msg) => NotSupportedError::new_err(msg),
            PyHdbError::Internal(msg) => InternalError::new_err(msg),
        }
    }
}

/// Map HANA error codes to DB-API 2.0 exception types.
fn map_hdbconnect_error(err: &hdbconnect::HdbError) -> PyHdbError {
    let msg = err.to_string();

    // Extract error code from message if available
    // HANA error format: "Error [code]: message"
    if let Some(code) = extract_hana_error_code(&msg) {
        return match code {
            // Integrity errors
            301..=303 | 461 => PyHdbError::Integrity(msg),

            // Programming errors (syntax, missing table, etc.)
            257 | 260..=263 => PyHdbError::Programming(msg),

            // Data errors (type conversion, overflow)
            304..=306 | 411 | 412 => PyHdbError::Data(msg),

            // Default to operational error (includes connection codes 131, 133)
            _ => PyHdbError::Operational(msg),
        };
    }

    // If no code, try to categorize by message content
    let lower = msg.to_lowercase();
    if lower.contains("connection") || lower.contains("timeout") {
        PyHdbError::Operational(msg)
    } else if lower.contains("syntax") || lower.contains("parse") {
        PyHdbError::Programming(msg)
    } else if lower.contains("constraint") || lower.contains("duplicate") {
        PyHdbError::Integrity(msg)
    } else if lower.contains("type") || lower.contains("conversion") {
        PyHdbError::Data(msg)
    } else {
        PyHdbError::Operational(msg)
    }
}

/// Extract HANA error code from error message.
fn extract_hana_error_code(msg: &str) -> Option<i32> {
    // Try to find pattern like "[123]" or "Error 123:"
    if let Some(start) = msg.find('[') {
        if let Some(end) = msg[start..].find(']') {
            if let Ok(code) = msg[start + 1..start + end].parse::<i32>() {
                return Some(code);
            }
        }
    }

    // Try "Error 123:" pattern
    if let Some(pos) = msg.find("Error ") {
        let rest = &msg[pos + 6..];
        if let Some(colon) = rest.find(':') {
            if let Ok(code) = rest[..colon].trim().parse::<i32>() {
                return Some(code);
            }
        }
    }

    None
}

/// Register exception types with the Python module.
pub fn register_exceptions(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("Error", py.get_type::<Error>())?;
    m.add("Warning", py.get_type::<Warning>())?;
    m.add("InterfaceError", py.get_type::<InterfaceError>())?;
    m.add("DatabaseError", py.get_type::<DatabaseError>())?;
    m.add("DataError", py.get_type::<DataError>())?;
    m.add("OperationalError", py.get_type::<OperationalError>())?;
    m.add("IntegrityError", py.get_type::<IntegrityError>())?;
    m.add("InternalError", py.get_type::<InternalError>())?;
    m.add("ProgrammingError", py.get_type::<ProgrammingError>())?;
    m.add("NotSupportedError", py.get_type::<NotSupportedError>())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hana_error_code() {
        assert_eq!(extract_hana_error_code("[301] duplicate key"), Some(301));
        assert_eq!(
            extract_hana_error_code("Error 257: syntax error"),
            Some(257)
        );
        assert_eq!(extract_hana_error_code("no code here"), None);
    }

    #[test]
    fn test_error_constructors() {
        let err = PyHdbError::interface("test");
        assert!(matches!(err, PyHdbError::Interface(_)));

        let err = PyHdbError::programming("test");
        assert!(matches!(err, PyHdbError::Programming(_)));
    }
}
