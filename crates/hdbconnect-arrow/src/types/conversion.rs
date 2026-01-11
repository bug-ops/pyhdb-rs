//! Type conversion implementations.
//!
//! This module contains conversion utilities between HANA values and
//! Rust/Arrow types.

use arrow_schema::DataType;

/// Get the Arrow `DataType` for a given HANA `TypeId` with optional precision/scale.
///
/// This is a convenience function that delegates to [`super::arrow::hana_type_to_arrow`].
#[inline]
#[must_use]
pub fn arrow_type_for(
    type_id: hdbconnect::TypeId,
    precision: Option<u8>,
    scale: Option<i8>,
) -> DataType {
    super::arrow::hana_type_to_arrow(type_id, precision, scale)
}

/// Check if a HANA type is numeric (integer or float).
#[must_use]
pub const fn is_numeric(type_id: hdbconnect::TypeId) -> bool {
    use hdbconnect::TypeId;
    matches!(
        type_id,
        TypeId::TINYINT
            | TypeId::SMALLINT
            | TypeId::INT
            | TypeId::BIGINT
            | TypeId::REAL
            | TypeId::DOUBLE
    )
}

/// Check if a HANA type is a decimal type.
#[must_use]
pub const fn is_decimal(type_id: hdbconnect::TypeId) -> bool {
    use hdbconnect::TypeId;
    // Note: SMALLDECIMAL is mapped to DECIMAL in hdbconnect 0.32+
    matches!(type_id, TypeId::DECIMAL)
}

/// Check if a HANA type is a string type.
#[must_use]
pub const fn is_string(type_id: hdbconnect::TypeId) -> bool {
    use hdbconnect::TypeId;
    matches!(
        type_id,
        TypeId::CHAR
            | TypeId::VARCHAR
            | TypeId::NCHAR
            | TypeId::NVARCHAR
            | TypeId::SHORTTEXT
            | TypeId::ALPHANUM
            | TypeId::STRING
    )
}

/// Check if a HANA type is a LOB (Large Object) type.
#[must_use]
pub const fn is_lob(type_id: hdbconnect::TypeId) -> bool {
    use hdbconnect::TypeId;
    matches!(
        type_id,
        TypeId::CLOB | TypeId::NCLOB | TypeId::BLOB | TypeId::TEXT
    )
}

/// Check if a HANA type is a temporal type.
#[must_use]
pub const fn is_temporal(type_id: hdbconnect::TypeId) -> bool {
    use hdbconnect::TypeId;
    // Note: DATE/TIME/TIMESTAMP are deprecated in hdbconnect 0.32+
    matches!(
        type_id,
        TypeId::DAYDATE | TypeId::SECONDTIME | TypeId::SECONDDATE | TypeId::LONGDATE
    )
}

/// Check if a HANA type requires LOB streaming (potentially large).
#[must_use]
pub const fn requires_streaming(type_id: hdbconnect::TypeId) -> bool {
    is_lob(type_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdbconnect::TypeId;

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
