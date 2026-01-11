//! Type conversion implementations.
//!
//! This module contains conversion utilities between HANA values and
//! Rust/Arrow types.

use arrow_schema::DataType;
use hdbconnect::TypeId;

/// Classification of HANA types into logical categories.
///
/// Provides a single source of truth for type classification,
/// replacing scattered `is_xxx()` functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeCategory {
    /// Integer and floating-point types (TINYINT, SMALLINT, INT, BIGINT, REAL, DOUBLE)
    Numeric,
    /// Fixed-precision decimal types (DECIMAL)
    Decimal,
    /// Boolean type
    Boolean,
    /// Character string types (CHAR, VARCHAR, NCHAR, NVARCHAR, etc.)
    String,
    /// Binary types (BINARY, VARBINARY, FIXED8, FIXED12, FIXED16)
    Binary,
    /// Large object types (CLOB, NCLOB, BLOB, TEXT)
    Lob,
    /// Date and time types (DAYDATE, SECONDTIME, SECONDDATE, LONGDATE)
    Temporal,
    /// Spatial types (GEOMETRY, POINT)
    Spatial,
    /// Unknown or unsupported type
    Unknown,
}

impl TypeCategory {
    /// Classify a HANA `TypeId` into its category.
    #[must_use]
    pub const fn from_type_id(type_id: TypeId) -> Self {
        match type_id {
            TypeId::TINYINT
            | TypeId::SMALLINT
            | TypeId::INT
            | TypeId::BIGINT
            | TypeId::REAL
            | TypeId::DOUBLE => Self::Numeric,

            TypeId::DECIMAL => Self::Decimal,

            TypeId::BOOLEAN => Self::Boolean,

            TypeId::CHAR
            | TypeId::VARCHAR
            | TypeId::NCHAR
            | TypeId::NVARCHAR
            | TypeId::SHORTTEXT
            | TypeId::ALPHANUM
            | TypeId::STRING => Self::String,

            TypeId::BINARY
            | TypeId::VARBINARY
            | TypeId::FIXED8
            | TypeId::FIXED12
            | TypeId::FIXED16 => Self::Binary,

            TypeId::CLOB | TypeId::NCLOB | TypeId::BLOB | TypeId::TEXT => Self::Lob,

            TypeId::DAYDATE | TypeId::SECONDTIME | TypeId::SECONDDATE | TypeId::LONGDATE => {
                Self::Temporal
            }

            TypeId::GEOMETRY | TypeId::POINT => Self::Spatial,

            _ => Self::Unknown,
        }
    }

    /// Returns the category name as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Numeric => "Numeric",
            Self::Decimal => "Decimal",
            Self::Boolean => "Boolean",
            Self::String => "String",
            Self::Binary => "Binary",
            Self::Lob => "LOB",
            Self::Temporal => "Temporal",
            Self::Spatial => "Spatial",
            Self::Unknown => "Unknown",
        }
    }

    /// Check if this is a numeric type (integer or float).
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Numeric)
    }

    /// Check if this is a decimal type.
    #[must_use]
    pub const fn is_decimal(&self) -> bool {
        matches!(self, Self::Decimal)
    }

    /// Check if this is a string type.
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String)
    }

    /// Check if this is a LOB type.
    #[must_use]
    pub const fn is_lob(&self) -> bool {
        matches!(self, Self::Lob)
    }

    /// Check if this is a temporal type.
    #[must_use]
    pub const fn is_temporal(&self) -> bool {
        matches!(self, Self::Temporal)
    }

    /// Check if this type requires streaming (LOB types).
    #[must_use]
    pub const fn requires_streaming(&self) -> bool {
        matches!(self, Self::Lob)
    }
}

/// Get the Arrow `DataType` for a given HANA `TypeId` with optional precision/scale.
///
/// This is a convenience function that delegates to [`super::arrow::hana_type_to_arrow`].
#[inline]
#[must_use]
pub fn arrow_type_for(type_id: TypeId, precision: Option<u8>, scale: Option<i8>) -> DataType {
    super::arrow::hana_type_to_arrow(type_id, precision, scale)
}

// Keep existing helper functions for backward compatibility but delegate to TypeCategory

/// Check if a HANA type is numeric (integer or float).
#[must_use]
pub const fn is_numeric(type_id: TypeId) -> bool {
    TypeCategory::from_type_id(type_id).is_numeric()
}

/// Check if a HANA type is a decimal type.
#[must_use]
pub const fn is_decimal(type_id: TypeId) -> bool {
    TypeCategory::from_type_id(type_id).is_decimal()
}

/// Check if a HANA type is a string type.
#[must_use]
pub const fn is_string(type_id: TypeId) -> bool {
    TypeCategory::from_type_id(type_id).is_string()
}

/// Check if a HANA type is a LOB (Large Object) type.
#[must_use]
pub const fn is_lob(type_id: TypeId) -> bool {
    TypeCategory::from_type_id(type_id).is_lob()
}

/// Check if a HANA type is a temporal type.
#[must_use]
pub const fn is_temporal(type_id: TypeId) -> bool {
    TypeCategory::from_type_id(type_id).is_temporal()
}

/// Check if a HANA type requires LOB streaming (potentially large).
#[must_use]
pub const fn requires_streaming(type_id: TypeId) -> bool {
    TypeCategory::from_type_id(type_id).requires_streaming()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // TypeCategory Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_type_category_numeric() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::TINYINT),
            TypeCategory::Numeric
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::SMALLINT),
            TypeCategory::Numeric
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::INT),
            TypeCategory::Numeric
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::BIGINT),
            TypeCategory::Numeric
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::REAL),
            TypeCategory::Numeric
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::DOUBLE),
            TypeCategory::Numeric
        );
    }

    #[test]
    fn test_type_category_decimal() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::DECIMAL),
            TypeCategory::Decimal
        );
    }

    #[test]
    fn test_type_category_boolean() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::BOOLEAN),
            TypeCategory::Boolean
        );
    }

    #[test]
    fn test_type_category_string() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::CHAR),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::VARCHAR),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::NCHAR),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::NVARCHAR),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::SHORTTEXT),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::ALPHANUM),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::STRING),
            TypeCategory::String
        );
    }

    #[test]
    fn test_type_category_binary() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::BINARY),
            TypeCategory::Binary
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::VARBINARY),
            TypeCategory::Binary
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::FIXED8),
            TypeCategory::Binary
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::FIXED12),
            TypeCategory::Binary
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::FIXED16),
            TypeCategory::Binary
        );
    }

    #[test]
    fn test_type_category_lob() {
        assert_eq!(TypeCategory::from_type_id(TypeId::CLOB), TypeCategory::Lob);
        assert_eq!(TypeCategory::from_type_id(TypeId::NCLOB), TypeCategory::Lob);
        assert_eq!(TypeCategory::from_type_id(TypeId::BLOB), TypeCategory::Lob);
        assert_eq!(TypeCategory::from_type_id(TypeId::TEXT), TypeCategory::Lob);
    }

    #[test]
    fn test_type_category_temporal() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::DAYDATE),
            TypeCategory::Temporal
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::SECONDTIME),
            TypeCategory::Temporal
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::SECONDDATE),
            TypeCategory::Temporal
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::LONGDATE),
            TypeCategory::Temporal
        );
    }

    #[test]
    fn test_type_category_spatial() {
        assert_eq!(
            TypeCategory::from_type_id(TypeId::GEOMETRY),
            TypeCategory::Spatial
        );
        assert_eq!(
            TypeCategory::from_type_id(TypeId::POINT),
            TypeCategory::Spatial
        );
    }

    #[test]
    fn test_type_category_as_str() {
        assert_eq!(TypeCategory::Numeric.as_str(), "Numeric");
        assert_eq!(TypeCategory::Decimal.as_str(), "Decimal");
        assert_eq!(TypeCategory::Boolean.as_str(), "Boolean");
        assert_eq!(TypeCategory::String.as_str(), "String");
        assert_eq!(TypeCategory::Binary.as_str(), "Binary");
        assert_eq!(TypeCategory::Lob.as_str(), "LOB");
        assert_eq!(TypeCategory::Temporal.as_str(), "Temporal");
        assert_eq!(TypeCategory::Spatial.as_str(), "Spatial");
        assert_eq!(TypeCategory::Unknown.as_str(), "Unknown");
    }

    #[test]
    fn test_type_category_predicates() {
        assert!(TypeCategory::Numeric.is_numeric());
        assert!(!TypeCategory::String.is_numeric());

        assert!(TypeCategory::Decimal.is_decimal());
        assert!(!TypeCategory::Numeric.is_decimal());

        assert!(TypeCategory::String.is_string());
        assert!(!TypeCategory::Lob.is_string());

        assert!(TypeCategory::Lob.is_lob());
        assert!(!TypeCategory::String.is_lob());

        assert!(TypeCategory::Temporal.is_temporal());
        assert!(!TypeCategory::Numeric.is_temporal());
    }

    #[test]
    fn test_type_category_requires_streaming() {
        assert!(TypeCategory::Lob.requires_streaming());
        assert!(!TypeCategory::String.requires_streaming());
        assert!(!TypeCategory::Binary.requires_streaming());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Backward-Compatible Function Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_is_numeric() {
        assert!(is_numeric(TypeId::INT));
        assert!(is_numeric(TypeId::BIGINT));
        assert!(is_numeric(TypeId::DOUBLE));
        assert!(!is_numeric(TypeId::VARCHAR));
        assert!(!is_numeric(TypeId::DECIMAL));
    }

    #[test]
    fn test_is_decimal() {
        assert!(is_decimal(TypeId::DECIMAL));
        assert!(!is_decimal(TypeId::INT));
    }

    #[test]
    fn test_is_string() {
        assert!(is_string(TypeId::VARCHAR));
        assert!(is_string(TypeId::NVARCHAR));
        assert!(!is_string(TypeId::CLOB)); // CLOB is LOB, not string
    }

    #[test]
    fn test_is_lob() {
        assert!(is_lob(TypeId::CLOB));
        assert!(is_lob(TypeId::BLOB));
        assert!(!is_lob(TypeId::VARCHAR));
    }

    #[test]
    fn test_is_temporal() {
        assert!(is_temporal(TypeId::DAYDATE));
        assert!(is_temporal(TypeId::LONGDATE));
        assert!(!is_temporal(TypeId::VARCHAR));
    }

    #[test]
    fn test_requires_streaming() {
        assert!(requires_streaming(TypeId::BLOB));
        assert!(requires_streaming(TypeId::CLOB));
        assert!(!requires_streaming(TypeId::VARCHAR));
    }
}
