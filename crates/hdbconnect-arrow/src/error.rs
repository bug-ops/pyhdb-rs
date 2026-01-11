//! Error hierarchy for hdbconnect-arrow.
//!
//! Follows the "canonical error struct" pattern from Microsoft Rust Guidelines.
//! Exposes `is_xxx()` methods rather than internal `ErrorKind` for future-proofing.

use thiserror::Error;

/// Root error type for hdbconnect-arrow crate.
///
/// This error type captures all possible failure modes during HANA to Arrow
/// conversion. Exposes predicate methods (`is_xxx()`) for error classification
/// without exposing internals.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::ArrowConversionError;
///
/// fn handle_error(err: ArrowConversionError) {
///     if err.is_unsupported_type() {
///         eprintln!("Unsupported HANA type encountered");
///     } else if err.is_schema_mismatch() {
///         eprintln!("Schema mismatch detected");
///     }
/// }
/// ```
#[derive(Error, Debug)]
#[error("{kind}")]
pub struct ArrowConversionError {
    kind: ErrorKind,
}

/// Internal error classification.
///
/// This enum is `pub(crate)` to allow adding variants without breaking changes.
/// External code should use the `is_xxx()` predicate methods instead.
#[derive(Error, Debug)]
#[non_exhaustive]
pub(crate) enum ErrorKind {
    /// A HANA type that cannot be mapped to Arrow.
    #[error("unsupported HANA type: {type_id:?}")]
    UnsupportedType { type_id: i16 },

    /// Column count mismatch between expected and actual.
    #[error("schema mismatch: expected {expected} columns, got {actual}")]
    SchemaMismatch { expected: usize, actual: usize },

    /// Value conversion failure for a specific column.
    #[error("value conversion failed for column '{column}': {message}")]
    ValueConversion { column: String, message: String },

    /// Decimal value exceeds Arrow Decimal128 capacity.
    #[error("decimal overflow: precision {precision}, scale {scale}")]
    DecimalOverflow { precision: u8, scale: i8 },

    /// Error from Arrow library operations.
    #[error("arrow error: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),

    /// Error from hdbconnect library.
    #[error("hdbconnect error: {0}")]
    Hdbconnect(String),

    /// Error during LOB streaming operations.
    #[error("LOB streaming error: {message}")]
    LobStreaming { message: String },

    /// Invalid precision value for DECIMAL type.
    #[error("invalid precision: {0}")]
    InvalidPrecision(String),

    /// Invalid scale value for DECIMAL type.
    #[error("invalid scale: {0}")]
    InvalidScale(String),
}

impl ArrowConversionError {
    // ═══════════════════════════════════════════════════════════════════════
    // Constructors
    // ═══════════════════════════════════════════════════════════════════════

    /// Create error for unsupported HANA type.
    #[must_use]
    pub const fn unsupported_type(type_id: i16) -> Self {
        Self {
            kind: ErrorKind::UnsupportedType { type_id },
        }
    }

    /// Create error for schema mismatch.
    #[must_use]
    pub const fn schema_mismatch(expected: usize, actual: usize) -> Self {
        Self {
            kind: ErrorKind::SchemaMismatch { expected, actual },
        }
    }

    /// Create error for value conversion failure.
    #[must_use]
    pub fn value_conversion(column: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ValueConversion {
                column: column.into(),
                message: message.into(),
            },
        }
    }

    /// Create error for decimal overflow.
    #[must_use]
    pub const fn decimal_overflow(precision: u8, scale: i8) -> Self {
        Self {
            kind: ErrorKind::DecimalOverflow { precision, scale },
        }
    }

    /// Create error for LOB streaming failure.
    #[must_use]
    pub fn lob_streaming(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::LobStreaming {
                message: message.into(),
            },
        }
    }

    /// Create error for invalid precision.
    #[must_use]
    pub fn invalid_precision(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidPrecision(message.into()),
        }
    }

    /// Create error for invalid scale.
    #[must_use]
    pub fn invalid_scale(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidScale(message.into()),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Predicate Methods (is_xxx)
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns true if this is an unsupported type error.
    #[must_use]
    pub const fn is_unsupported_type(&self) -> bool {
        matches!(self.kind, ErrorKind::UnsupportedType { .. })
    }

    /// Returns true if this is a schema mismatch error.
    #[must_use]
    pub const fn is_schema_mismatch(&self) -> bool {
        matches!(self.kind, ErrorKind::SchemaMismatch { .. })
    }

    /// Returns true if this is a value conversion error.
    #[must_use]
    pub const fn is_value_conversion(&self) -> bool {
        matches!(self.kind, ErrorKind::ValueConversion { .. })
    }

    /// Returns true if this is a decimal overflow error.
    #[must_use]
    pub const fn is_decimal_overflow(&self) -> bool {
        matches!(self.kind, ErrorKind::DecimalOverflow { .. })
    }

    /// Returns true if this is an Arrow library error.
    #[must_use]
    pub const fn is_arrow_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Arrow(_))
    }

    /// Returns true if this is an hdbconnect error.
    #[must_use]
    pub const fn is_hdbconnect_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Hdbconnect(_))
    }

    /// Returns true if this is a LOB streaming error.
    #[must_use]
    pub const fn is_lob_streaming(&self) -> bool {
        matches!(self.kind, ErrorKind::LobStreaming { .. })
    }

    /// Returns true if this is an invalid precision error.
    #[must_use]
    pub const fn is_invalid_precision(&self) -> bool {
        matches!(self.kind, ErrorKind::InvalidPrecision(_))
    }

    /// Returns true if this is an invalid scale error.
    #[must_use]
    pub const fn is_invalid_scale(&self) -> bool {
        matches!(self.kind, ErrorKind::InvalidScale(_))
    }
}

impl From<hdbconnect::HdbError> for ArrowConversionError {
    fn from(err: hdbconnect::HdbError) -> Self {
        Self {
            kind: ErrorKind::Hdbconnect(err.to_string()),
        }
    }
}

impl From<arrow_schema::ArrowError> for ArrowConversionError {
    fn from(err: arrow_schema::ArrowError) -> Self {
        Self {
            kind: ErrorKind::Arrow(err),
        }
    }
}

/// Result type alias for Arrow conversion operations.
pub type Result<T> = std::result::Result<T, ArrowConversionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ArrowConversionError::unsupported_type(42);
        assert!(err.is_unsupported_type());
        assert!(!err.is_schema_mismatch());
    }

    #[test]
    fn test_schema_mismatch() {
        let err = ArrowConversionError::schema_mismatch(5, 3);
        assert!(err.is_schema_mismatch());
        assert!(err.to_string().contains("expected 5 columns, got 3"));
    }

    #[test]
    fn test_value_conversion() {
        let err = ArrowConversionError::value_conversion("col1", "invalid integer");
        assert!(err.is_value_conversion());
        assert!(err.to_string().contains("col1"));
    }

    #[test]
    fn test_decimal_overflow() {
        let err = ArrowConversionError::decimal_overflow(38, 10);
        assert!(err.is_decimal_overflow());
    }

    #[test]
    fn test_error_debug() {
        let err = ArrowConversionError::unsupported_type(99);
        // Debug should be implemented
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("ArrowConversionError"));
    }

    #[test]
    fn test_error_display() {
        let err = ArrowConversionError::lob_streaming("connection lost");
        let display = err.to_string();
        assert!(display.contains("LOB streaming error"));
        assert!(display.contains("connection lost"));
    }
}
