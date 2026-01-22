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

// Note: hdbconnect_async::HdbError is the same type as hdbconnect::HdbError,
// so the From impl above handles both sync and async cases.

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
    let msg = build_detailed_error_message(err);
    categorize_hana_error(msg)
}

/// Build a detailed error message from `hdbconnect::HdbError`.
fn build_detailed_error_message(err: &hdbconnect::HdbError) -> String {
    if let Some(server_err) = err.server_error() {
        return format_server_error(server_err);
    }
    err.to_string()
}

/// Format server error with all available details (code, severity, SQLSTATE, position).
fn format_server_error(server_err: &hdbconnect::ServerError) -> String {
    use std::fmt::Write;

    let mut details = format!(
        "[{}] {} (severity: {:?})",
        server_err.code(),
        server_err.text(),
        server_err.severity(),
    );

    let sqlstate = server_err.sqlstate();
    if !sqlstate.is_empty() {
        write!(details, ", SQLSTATE: {}", String::from_utf8_lossy(sqlstate)).ok();
    }

    if server_err.position() > 0 {
        write!(details, ", at position {}", server_err.position()).ok();
    }

    details
}

/// Categorize a HANA error message into the appropriate DB-API 2.0 exception type.
fn categorize_hana_error(msg: String) -> PyHdbError {
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
    // Pattern: "[123]"
    if let Some(start) = msg.find('[')
        && let Some(end) = msg[start..].find(']')
        && let Ok(code) = msg[start + 1..start + end].parse::<i32>()
    {
        return Some(code);
    }

    // Pattern: "Error 123:"
    if let Some(pos) = msg.find("Error ")
        && let Some(colon) = msg[pos + 6..].find(':')
        && let Ok(code) = msg[pos + 6..pos + 6 + colon].trim().parse::<i32>()
    {
        return Some(code);
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

    // ═══════════════════════════════════════════════════════════════════════════
    // extract_hana_error_code Tests
    // ═══════════════════════════════════════════════════════════════════════════

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
    fn test_extract_hana_error_code_bracket_format() {
        assert_eq!(extract_hana_error_code("[123] some error"), Some(123));
        assert_eq!(extract_hana_error_code("[0] zero code"), Some(0));
        assert_eq!(extract_hana_error_code("[999999] large code"), Some(999999));
    }

    #[test]
    fn test_extract_hana_error_code_error_format() {
        assert_eq!(extract_hana_error_code("Error 100: connection"), Some(100));
        assert_eq!(extract_hana_error_code("Error 1: minimal"), Some(1));
        assert_eq!(extract_hana_error_code("Error 50000: big"), Some(50000));
    }

    #[test]
    fn test_extract_hana_error_code_invalid() {
        assert_eq!(extract_hana_error_code("no code"), None);
        assert_eq!(extract_hana_error_code("[abc] not a number"), None);
        assert_eq!(extract_hana_error_code("Error abc: not a number"), None);
        assert_eq!(extract_hana_error_code(""), None);
        assert_eq!(extract_hana_error_code("[]"), None);
        assert_eq!(extract_hana_error_code("[ ]"), None);
    }

    #[test]
    fn test_extract_hana_error_code_edge_cases() {
        assert_eq!(extract_hana_error_code("[1]"), Some(1));
        assert_eq!(extract_hana_error_code("Error 1:"), Some(1));
        assert_eq!(extract_hana_error_code("prefix [456] suffix"), Some(456));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PyHdbError Constructor Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_error_constructors() {
        let err = PyHdbError::interface("test");
        assert!(matches!(err, PyHdbError::Interface(_)));

        let err = PyHdbError::programming("test");
        assert!(matches!(err, PyHdbError::Programming(_)));
    }

    #[test]
    fn test_error_interface_constructor() {
        let err = PyHdbError::interface("connection failed");
        assert!(matches!(err, PyHdbError::Interface(_)));
        assert!(err.to_string().contains("InterfaceError"));
        assert!(err.to_string().contains("connection failed"));
    }

    #[test]
    fn test_error_operational_constructor() {
        let err = PyHdbError::operational("connection lost");
        assert!(matches!(err, PyHdbError::Operational(_)));
        assert!(err.to_string().contains("OperationalError"));
    }

    #[test]
    fn test_error_programming_constructor() {
        let err = PyHdbError::programming("syntax error");
        assert!(matches!(err, PyHdbError::Programming(_)));
        assert!(err.to_string().contains("ProgrammingError"));
    }

    #[test]
    fn test_error_integrity_constructor() {
        let err = PyHdbError::integrity("constraint violation");
        assert!(matches!(err, PyHdbError::Integrity(_)));
        assert!(err.to_string().contains("IntegrityError"));
    }

    #[test]
    fn test_error_data_constructor() {
        let err = PyHdbError::data("type conversion failed");
        assert!(matches!(err, PyHdbError::Data(_)));
        assert!(err.to_string().contains("DataError"));
    }

    #[test]
    fn test_error_not_supported_constructor() {
        let err = PyHdbError::not_supported("feature not available");
        assert!(matches!(err, PyHdbError::NotSupported(_)));
        assert!(err.to_string().contains("NotSupportedError"));
    }

    #[test]
    fn test_error_internal_constructor() {
        let err = PyHdbError::internal("unexpected state");
        assert!(matches!(err, PyHdbError::Internal(_)));
        assert!(err.to_string().contains("InternalError"));
    }

    #[test]
    fn test_error_arrow_constructor() {
        let err = PyHdbError::arrow("conversion failed");
        assert!(matches!(err, PyHdbError::Arrow(_)));
        assert!(err.to_string().contains("ArrowError"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // String Coercion Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_error_constructors_accept_string() {
        let _ = PyHdbError::interface(String::from("owned string"));
        let _ = PyHdbError::operational(String::from("owned"));
        let _ = PyHdbError::programming(String::from("owned"));
        let _ = PyHdbError::integrity(String::from("owned"));
        let _ = PyHdbError::data(String::from("owned"));
        let _ = PyHdbError::not_supported(String::from("owned"));
        let _ = PyHdbError::internal(String::from("owned"));
        let _ = PyHdbError::arrow(String::from("owned"));
    }

    #[test]
    fn test_error_constructors_accept_str() {
        let _ = PyHdbError::interface("borrowed string");
        let _ = PyHdbError::operational("borrowed");
        let _ = PyHdbError::programming("borrowed");
        let _ = PyHdbError::integrity("borrowed");
        let _ = PyHdbError::data("borrowed");
        let _ = PyHdbError::not_supported("borrowed");
        let _ = PyHdbError::internal("borrowed");
        let _ = PyHdbError::arrow("borrowed");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // categorize_hana_error Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_categorize_hana_error_integrity_codes() {
        let err = categorize_hana_error("[301] duplicate key".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));

        let err = categorize_hana_error("[302] referential integrity".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));

        let err = categorize_hana_error("[303] check constraint".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));

        let err = categorize_hana_error("[461] unique constraint".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));
    }

    #[test]
    fn test_categorize_hana_error_programming_codes() {
        let err = categorize_hana_error("[257] syntax error".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));

        let err = categorize_hana_error("[260] table not found".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));

        let err = categorize_hana_error("[261] column not found".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));

        let err = categorize_hana_error("[262] invalid identifier".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));

        let err = categorize_hana_error("[263] parse error".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));
    }

    #[test]
    fn test_categorize_hana_error_data_codes() {
        let err = categorize_hana_error("[304] numeric overflow".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));

        let err = categorize_hana_error("[305] division by zero".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));

        let err = categorize_hana_error("[306] string too long".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));

        let err = categorize_hana_error("[411] type mismatch".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));

        let err = categorize_hana_error("[412] conversion error".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));
    }

    #[test]
    fn test_categorize_hana_error_unknown_code_defaults_to_operational() {
        let err = categorize_hana_error("[999] unknown error".to_string());
        assert!(matches!(err, PyHdbError::Operational(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_connection() {
        let err = categorize_hana_error("connection refused".to_string());
        assert!(matches!(err, PyHdbError::Operational(_)));

        let err = categorize_hana_error("Connection lost".to_string());
        assert!(matches!(err, PyHdbError::Operational(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_timeout() {
        let err = categorize_hana_error("timeout occurred".to_string());
        assert!(matches!(err, PyHdbError::Operational(_)));

        let err = categorize_hana_error("TIMEOUT: operation took too long".to_string());
        assert!(matches!(err, PyHdbError::Operational(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_syntax() {
        let err = categorize_hana_error("syntax error near SELECT".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));

        let err = categorize_hana_error("SYNTAX ERROR in query".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_parse() {
        let err = categorize_hana_error("parse error".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));

        let err = categorize_hana_error("PARSE failed".to_string());
        assert!(matches!(err, PyHdbError::Programming(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_constraint() {
        let err = categorize_hana_error("constraint violation".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));

        let err = categorize_hana_error("CONSTRAINT check failed".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_duplicate() {
        let err = categorize_hana_error("duplicate key".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));

        let err = categorize_hana_error("DUPLICATE value".to_string());
        assert!(matches!(err, PyHdbError::Integrity(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_type() {
        let err = categorize_hana_error("type mismatch".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));

        let err = categorize_hana_error("TYPE error".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));
    }

    #[test]
    fn test_categorize_hana_error_by_message_content_conversion() {
        let err = categorize_hana_error("conversion failed".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));

        let err = categorize_hana_error("CONVERSION error".to_string());
        assert!(matches!(err, PyHdbError::Data(_)));
    }

    #[test]
    fn test_categorize_hana_error_unknown_message_defaults_to_operational() {
        let err = categorize_hana_error("something went wrong".to_string());
        assert!(matches!(err, PyHdbError::Operational(_)));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // From<url::ParseError> Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_from_url_parse_error() {
        let url_err = url::Url::parse("not a valid url").unwrap_err();
        let err: PyHdbError = url_err.into();
        assert!(matches!(err, PyHdbError::Interface(_)));
        assert!(err.to_string().contains("invalid URL"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // From<hdbconnect_arrow::ArrowConversionError> Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_from_arrow_conversion_error() {
        let arrow_err = hdbconnect_arrow::ArrowConversionError::schema_mismatch(5, 3);
        let err: PyHdbError = arrow_err.into();
        assert!(matches!(err, PyHdbError::Arrow(_)));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Display Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_error_display_format() {
        let err = PyHdbError::interface("test message");
        assert_eq!(err.to_string(), "InterfaceError: test message");

        let err = PyHdbError::operational("test message");
        assert_eq!(err.to_string(), "OperationalError: test message");

        let err = PyHdbError::programming("test message");
        assert_eq!(err.to_string(), "ProgrammingError: test message");

        let err = PyHdbError::integrity("test message");
        assert_eq!(err.to_string(), "IntegrityError: test message");

        let err = PyHdbError::data("test message");
        assert_eq!(err.to_string(), "DataError: test message");

        let err = PyHdbError::not_supported("test message");
        assert_eq!(err.to_string(), "NotSupportedError: test message");

        let err = PyHdbError::internal("test message");
        assert_eq!(err.to_string(), "InternalError: test message");

        let err = PyHdbError::arrow("test message");
        assert_eq!(err.to_string(), "ArrowError: test message");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Debug Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_error_debug() {
        let err = PyHdbError::interface("test");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Interface"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge Cases
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_error_with_empty_message() {
        let err = PyHdbError::interface("");
        assert!(matches!(err, PyHdbError::Interface(_)));
    }

    #[test]
    fn test_error_with_unicode_message() {
        let err = PyHdbError::interface("エラー: 接続失敗");
        assert!(err.to_string().contains("エラー"));
    }

    #[test]
    fn test_error_with_special_characters() {
        let err = PyHdbError::interface("error: \"quotes\" and 'apostrophes'");
        assert!(err.to_string().contains("quotes"));
    }
}
