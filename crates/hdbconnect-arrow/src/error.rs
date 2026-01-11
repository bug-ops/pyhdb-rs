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
    ValueConversion {
        column: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Decimal value exceeds Arrow Decimal128 capacity.
    #[error("decimal overflow: precision {precision}, scale {scale}")]
    DecimalOverflow { precision: u8, scale: i8 },

    /// Error from Arrow library operations.
    #[error("arrow error")]
    Arrow(
        #[source]
        #[from]
        arrow_schema::ArrowError,
    ),

    /// Error from hdbconnect library.
    #[error("hdbconnect error: {message}")]
    Hdbconnect {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

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
                source: None,
            },
        }
    }

    /// Create error for value conversion failure with source error.
    #[must_use]
    pub fn value_conversion_with_source<E>(
        column: impl Into<String>,
        message: impl Into<String>,
        source: E,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind: ErrorKind::ValueConversion {
                column: column.into(),
                message: message.into(),
                source: Some(Box::new(source)),
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
        matches!(self.kind, ErrorKind::Hdbconnect { .. })
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

    // ═══════════════════════════════════════════════════════════════════════
    // Error Classification Methods
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns true if this error is potentially recoverable.
    ///
    /// Recoverable errors are typically transient issues that might
    /// succeed if retried (e.g., network timeouts, temporary failures).
    ///
    /// Non-recoverable errors indicate permanent failures like schema
    /// mismatches, unsupported types, or data corruption.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn process_with_retry<T>(f: impl Fn() -> Result<T>) -> Result<T> {
    ///     for _ in 0..3 {
    ///         match f() {
    ///             Ok(v) => return Ok(v),
    ///             Err(e) if e.is_recoverable() => continue,
    ///             Err(e) => return Err(e),
    ///         }
    ///     }
    ///     f() // Final attempt
    /// }
    /// ```
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn is_recoverable(&self) -> bool {
        match &self.kind {
            // Configuration/data errors - not recoverable
            ErrorKind::UnsupportedType { .. } => false,
            ErrorKind::SchemaMismatch { .. } => false,
            ErrorKind::ValueConversion { .. } => false,
            ErrorKind::DecimalOverflow { .. } => false,
            ErrorKind::InvalidPrecision(_) => false,
            ErrorKind::InvalidScale(_) => false,

            // Arrow errors - generally not recoverable
            ErrorKind::Arrow(_) => false,

            // LOB streaming might be recoverable (network issues)
            ErrorKind::LobStreaming { .. } => true,

            // HANA errors need inspection - some might be transient
            // Default to recoverable to allow retry logic
            ErrorKind::Hdbconnect { .. } => true,
        }
    }

    /// Returns true if this error is a configuration error.
    ///
    /// Configuration errors indicate incorrect setup that won't
    /// be fixed by retrying.
    #[must_use]
    pub const fn is_configuration_error(&self) -> bool {
        matches!(
            &self.kind,
            ErrorKind::UnsupportedType { .. }
                | ErrorKind::SchemaMismatch { .. }
                | ErrorKind::InvalidPrecision(_)
                | ErrorKind::InvalidScale(_)
        )
    }

    /// Returns true if this error is a data error.
    ///
    /// Data errors indicate issues with the data being processed.
    #[must_use]
    pub const fn is_data_error(&self) -> bool {
        matches!(
            &self.kind,
            ErrorKind::ValueConversion { .. } | ErrorKind::DecimalOverflow { .. }
        )
    }
}

impl From<hdbconnect::HdbError> for ArrowConversionError {
    fn from(err: hdbconnect::HdbError) -> Self {
        Self {
            kind: ErrorKind::Hdbconnect {
                message: err.to_string(),
                source: Some(Box::new(err)),
            },
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Constructor Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_unsupported_type_creation() {
        let err = ArrowConversionError::unsupported_type(42);
        assert!(err.is_unsupported_type());
        assert!(!err.is_schema_mismatch());
        assert!(!err.is_value_conversion());
        assert!(!err.is_decimal_overflow());
        assert!(!err.is_arrow_error());
        assert!(!err.is_hdbconnect_error());
        assert!(!err.is_lob_streaming());
        assert!(!err.is_invalid_precision());
        assert!(!err.is_invalid_scale());
    }

    #[test]
    fn test_schema_mismatch_creation() {
        let err = ArrowConversionError::schema_mismatch(5, 3);
        assert!(err.is_schema_mismatch());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("expected 5 columns, got 3"));
    }

    #[test]
    fn test_value_conversion_creation() {
        let err = ArrowConversionError::value_conversion("col1", "invalid integer");
        assert!(err.is_value_conversion());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("col1"));
        assert!(err.to_string().contains("invalid integer"));
    }

    #[test]
    fn test_decimal_overflow_creation() {
        let err = ArrowConversionError::decimal_overflow(38, 10);
        assert!(err.is_decimal_overflow());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("precision 38"));
        assert!(err.to_string().contains("scale 10"));
    }

    #[test]
    fn test_lob_streaming_creation() {
        let err = ArrowConversionError::lob_streaming("connection lost");
        assert!(err.is_lob_streaming());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("LOB streaming error"));
        assert!(err.to_string().contains("connection lost"));
    }

    #[test]
    fn test_invalid_precision_creation() {
        let err = ArrowConversionError::invalid_precision("precision must be positive");
        assert!(err.is_invalid_precision());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("invalid precision"));
    }

    #[test]
    fn test_invalid_scale_creation() {
        let err = ArrowConversionError::invalid_scale("scale exceeds precision");
        assert!(err.is_invalid_scale());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("invalid scale"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // From Conversion Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_from_arrow_error() {
        let arrow_err = arrow_schema::ArrowError::SchemaError("test error".to_string());
        let err: ArrowConversionError = arrow_err.into();
        assert!(err.is_arrow_error());
        assert!(!err.is_unsupported_type());
        assert!(err.to_string().contains("arrow error"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Display and Debug Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_error_debug() {
        let err = ArrowConversionError::unsupported_type(99);
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("ArrowConversionError"));
        assert!(debug_str.contains("UnsupportedType"));
    }

    #[test]
    fn test_unsupported_type_display() {
        let err = ArrowConversionError::unsupported_type(127);
        let display = err.to_string();
        assert!(display.contains("unsupported HANA type"));
        assert!(display.contains("127"));
    }

    #[test]
    fn test_schema_mismatch_display() {
        let err = ArrowConversionError::schema_mismatch(10, 5);
        let display = err.to_string();
        assert!(display.contains("schema mismatch"));
        assert!(display.contains("expected 10 columns"));
        assert!(display.contains("got 5"));
    }

    #[test]
    fn test_value_conversion_display() {
        let err = ArrowConversionError::value_conversion("my_column", "parse error");
        let display = err.to_string();
        assert!(display.contains("value conversion failed"));
        assert!(display.contains("my_column"));
        assert!(display.contains("parse error"));
    }

    #[test]
    fn test_decimal_overflow_display() {
        let err = ArrowConversionError::decimal_overflow(50, 20);
        let display = err.to_string();
        assert!(display.contains("decimal overflow"));
        assert!(display.contains("precision 50"));
        assert!(display.contains("scale 20"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Predicate Exhaustive Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_all_predicates_false_for_unsupported_type() {
        let err = ArrowConversionError::unsupported_type(1);
        assert!(err.is_unsupported_type());
        assert!(!err.is_schema_mismatch());
        assert!(!err.is_value_conversion());
        assert!(!err.is_decimal_overflow());
        assert!(!err.is_arrow_error());
        assert!(!err.is_hdbconnect_error());
        assert!(!err.is_lob_streaming());
        assert!(!err.is_invalid_precision());
        assert!(!err.is_invalid_scale());
    }

    #[test]
    fn test_all_predicates_false_for_schema_mismatch() {
        let err = ArrowConversionError::schema_mismatch(1, 2);
        assert!(!err.is_unsupported_type());
        assert!(err.is_schema_mismatch());
        assert!(!err.is_value_conversion());
        assert!(!err.is_decimal_overflow());
        assert!(!err.is_arrow_error());
        assert!(!err.is_hdbconnect_error());
        assert!(!err.is_lob_streaming());
        assert!(!err.is_invalid_precision());
        assert!(!err.is_invalid_scale());
    }

    #[test]
    fn test_all_predicates_false_for_lob_streaming() {
        let err = ArrowConversionError::lob_streaming("test");
        assert!(!err.is_unsupported_type());
        assert!(!err.is_schema_mismatch());
        assert!(!err.is_value_conversion());
        assert!(!err.is_decimal_overflow());
        assert!(!err.is_arrow_error());
        assert!(!err.is_hdbconnect_error());
        assert!(err.is_lob_streaming());
        assert!(!err.is_invalid_precision());
        assert!(!err.is_invalid_scale());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge Case Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_empty_column_name() {
        let err = ArrowConversionError::value_conversion("", "error");
        assert!(err.is_value_conversion());
    }

    #[test]
    fn test_empty_message() {
        let err = ArrowConversionError::lob_streaming("");
        assert!(err.is_lob_streaming());
    }

    #[test]
    fn test_unicode_in_messages() {
        let err = ArrowConversionError::value_conversion("列名", "无效数据");
        assert!(err.is_value_conversion());
        assert!(err.to_string().contains("列名"));
    }

    #[test]
    fn test_zero_schema_mismatch() {
        let err = ArrowConversionError::schema_mismatch(0, 0);
        assert!(err.is_schema_mismatch());
    }

    #[test]
    fn test_negative_type_id() {
        let err = ArrowConversionError::unsupported_type(-1);
        assert!(err.is_unsupported_type());
    }

    #[test]
    fn test_negative_scale() {
        let err = ArrowConversionError::decimal_overflow(10, -5);
        assert!(err.is_decimal_overflow());
        assert!(err.to_string().contains("scale -5"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Error Classification Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_is_recoverable() {
        // Non-recoverable errors
        assert!(!ArrowConversionError::unsupported_type(42).is_recoverable());
        assert!(!ArrowConversionError::schema_mismatch(1, 2).is_recoverable());
        assert!(!ArrowConversionError::value_conversion("col", "msg").is_recoverable());
        assert!(!ArrowConversionError::decimal_overflow(38, 10).is_recoverable());
        assert!(!ArrowConversionError::invalid_precision("msg").is_recoverable());
        assert!(!ArrowConversionError::invalid_scale("msg").is_recoverable());

        // Recoverable errors
        assert!(ArrowConversionError::lob_streaming("network timeout").is_recoverable());
    }

    #[test]
    fn test_is_configuration_error() {
        // Configuration errors
        assert!(ArrowConversionError::unsupported_type(42).is_configuration_error());
        assert!(ArrowConversionError::schema_mismatch(1, 2).is_configuration_error());
        assert!(ArrowConversionError::invalid_precision("msg").is_configuration_error());
        assert!(ArrowConversionError::invalid_scale("msg").is_configuration_error());

        // Non-configuration errors
        assert!(!ArrowConversionError::value_conversion("col", "msg").is_configuration_error());
        assert!(!ArrowConversionError::decimal_overflow(38, 10).is_configuration_error());
        assert!(!ArrowConversionError::lob_streaming("msg").is_configuration_error());
    }

    #[test]
    fn test_is_data_error() {
        // Data errors
        assert!(ArrowConversionError::value_conversion("col", "msg").is_data_error());
        assert!(ArrowConversionError::decimal_overflow(38, 10).is_data_error());

        // Non-data errors
        assert!(!ArrowConversionError::unsupported_type(42).is_data_error());
        assert!(!ArrowConversionError::schema_mismatch(1, 2).is_data_error());
        assert!(!ArrowConversionError::lob_streaming("msg").is_data_error());
        assert!(!ArrowConversionError::invalid_precision("msg").is_data_error());
    }

    #[test]
    fn test_error_classification_mutual_exclusivity() {
        // Each error should be in exactly one classification category
        let config_err = ArrowConversionError::unsupported_type(42);
        assert!(config_err.is_configuration_error());
        assert!(!config_err.is_data_error());
        assert!(!config_err.is_recoverable());

        let data_err = ArrowConversionError::value_conversion("col", "msg");
        assert!(!data_err.is_configuration_error());
        assert!(data_err.is_data_error());
        assert!(!data_err.is_recoverable());

        let recoverable = ArrowConversionError::lob_streaming("timeout");
        assert!(!recoverable.is_configuration_error());
        assert!(!recoverable.is_data_error());
        assert!(recoverable.is_recoverable());
    }

    #[test]
    fn test_value_conversion_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "parse failed");
        let err = ArrowConversionError::value_conversion_with_source("col1", "failed", source);
        assert!(err.is_value_conversion());
        assert!(err.is_data_error());
        assert!(err.to_string().contains("col1"));
    }
}
