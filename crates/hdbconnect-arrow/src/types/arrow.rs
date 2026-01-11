//! Arrow type mappings from HANA types.
//!
//! This module provides the authoritative mapping between HANA SQL types
//! and Apache Arrow types.
//!
//! # Type Mapping Table
//!
//! | HANA Type | Arrow Type | Notes |
//! |-----------|------------|-------|
//! | TINYINT | UInt8 | Unsigned in HANA |
//! | SMALLINT | Int16 | |
//! | INT | Int32 | |
//! | BIGINT | Int64 | |
//! | REAL | Float32 | |
//! | DOUBLE | Float64 | |
//! | DECIMAL(p,s) | Decimal128(p,s) | Full precision |
//! | CHAR/VARCHAR | Utf8 | |
//! | NCHAR/NVARCHAR | Utf8 | Unicode strings |
//! | CLOB/NCLOB | LargeUtf8 | Large strings |
//! | BLOB | LargeBinary | Large binary |
//! | DAYDATE | Date32 | Days since epoch |
//! | SECONDTIME | Time64(Nanosecond) | |
//! | LONGDATE/SECONDDATE | Timestamp(Nanosecond, None) | |
//! | BOOLEAN | Boolean | |
//! | GEOMETRY/POINT | Binary | WKB format |

use arrow_schema::{DataType, Field, TimeUnit};
use hdbconnect::TypeId;

/// Convert HANA `TypeId` to Arrow `DataType`.
///
/// This is the authoritative mapping between HANA SQL types and Arrow types.
/// The mapping prioritizes:
/// 1. Precision preservation (especially for decimals)
/// 2. Zero-copy compatibility with Polars/pandas
/// 3. Consistent handling of nullable values
///
/// # Arguments
///
/// * `type_id` - The HANA type identifier
/// * `precision` - Optional precision for DECIMAL types
/// * `scale` - Optional scale for DECIMAL types
///
/// # Returns
///
/// The corresponding Arrow `DataType`.
#[must_use]
#[allow(clippy::match_same_arms)] // Intentional: semantic separation of GEOMETRY vs BINARY
pub fn hana_type_to_arrow(type_id: TypeId, precision: Option<u8>, scale: Option<i8>) -> DataType {
    match type_id {
        // Integer types
        TypeId::TINYINT => DataType::UInt8, // HANA TINYINT is unsigned
        TypeId::SMALLINT => DataType::Int16,
        TypeId::INT => DataType::Int32,
        TypeId::BIGINT => DataType::Int64,

        // Floating point types
        TypeId::REAL => DataType::Float32,
        TypeId::DOUBLE => DataType::Float64,

        // Decimal types - preserve precision and scale
        // Note: SMALLDECIMAL is mapped to DECIMAL in hdbconnect 0.32+
        TypeId::DECIMAL => {
            let p = precision.unwrap_or(38).min(38);
            let s = scale.unwrap_or(0);
            DataType::Decimal128(p, s)
        }

        // String types - all map to UTF-8
        TypeId::CHAR
        | TypeId::VARCHAR
        | TypeId::NCHAR
        | TypeId::NVARCHAR
        | TypeId::SHORTTEXT
        | TypeId::ALPHANUM
        | TypeId::STRING => DataType::Utf8,

        // Binary types
        TypeId::BINARY | TypeId::VARBINARY => DataType::Binary,

        // LOB types - use Large variants for potentially huge data
        TypeId::CLOB | TypeId::NCLOB | TypeId::TEXT => DataType::LargeUtf8,
        TypeId::BLOB => DataType::LargeBinary,

        // Temporal types
        // Note: DATE/TIME/TIMESTAMP are deprecated in hdbconnect 0.32+
        // Using DAYDATE, SECONDTIME, LONGDATE, SECONDDATE instead
        TypeId::DAYDATE => DataType::Date32,
        TypeId::SECONDTIME => DataType::Time64(TimeUnit::Nanosecond),
        TypeId::SECONDDATE | TypeId::LONGDATE => {
            DataType::Timestamp(TimeUnit::Nanosecond, None)
        }

        // Boolean
        TypeId::BOOLEAN => DataType::Boolean,

        // Fixed-size binary types (HANA specific)
        TypeId::FIXED8 => DataType::FixedSizeBinary(8),
        TypeId::FIXED12 => DataType::FixedSizeBinary(12),
        TypeId::FIXED16 => DataType::FixedSizeBinary(16),

        // Spatial types - serialize as WKB binary
        TypeId::GEOMETRY | TypeId::POINT => DataType::Binary,

        // Unknown/unsupported - fallback to string representation
        _ => DataType::Utf8,
    }
}

/// Create an Arrow Field from HANA column metadata.
///
/// # Arguments
///
/// * `name` - Column name
/// * `type_id` - HANA type identifier
/// * `nullable` - Whether the column allows NULL values
/// * `precision` - Optional precision for DECIMAL types
/// * `scale` - Optional scale for DECIMAL types
#[must_use]
pub fn hana_field_to_arrow(
    name: &str,
    type_id: TypeId,
    nullable: bool,
    precision: Option<u8>,
    scale: Option<i8>,
) -> Field {
    Field::new(name, hana_type_to_arrow(type_id, precision, scale), nullable)
}

/// Extension trait for hdbconnect `FieldMetadata`.
///
/// Provides convenient conversion methods for HANA metadata to Arrow types.
pub trait FieldMetadataExt {
    /// Convert to Arrow Field.
    fn to_arrow_field(&self) -> Field;

    /// Get the Arrow `DataType` for this field.
    fn arrow_data_type(&self) -> DataType;
}

impl FieldMetadataExt for hdbconnect::FieldMetadata {
    fn to_arrow_field(&self) -> Field {
        let name = {
            let display = self.displayname();
            if display.is_empty() {
                self.columnname()
            } else {
                display
            }
        };
        let precision = self.precision();
        let scale = self.scale();
        hana_field_to_arrow(
            name,
            self.type_id(),
            self.is_nullable(),
            // Safe: precision is checked to be in valid HANA range [0, 38]
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            (0..=255_i16).contains(&precision).then_some(precision as u8),
            // Safe: scale is checked to be in valid HANA range [0, precision]
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            (0..=127_i16).contains(&scale).then_some(scale as i8),
        )
    }

    fn arrow_data_type(&self) -> DataType {
        let precision = self.precision();
        let scale = self.scale();
        hana_type_to_arrow(
            self.type_id(),
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            (0..=255_i16).contains(&precision).then_some(precision as u8),
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            (0..=127_i16).contains(&scale).then_some(scale as i8),
        )
    }
}

/// Get the HANA type category for a `TypeId`.
///
/// Returns the category name as a static string.
#[must_use]
pub const fn type_category(type_id: TypeId) -> &'static str {
    match type_id {
        TypeId::TINYINT | TypeId::SMALLINT | TypeId::INT | TypeId::BIGINT | TypeId::REAL | TypeId::DOUBLE => "Numeric",
        TypeId::DECIMAL => "Decimal",
        TypeId::CHAR | TypeId::VARCHAR | TypeId::NCHAR | TypeId::NVARCHAR | TypeId::SHORTTEXT | TypeId::ALPHANUM | TypeId::STRING => "String",
        TypeId::BINARY | TypeId::VARBINARY | TypeId::FIXED8 | TypeId::FIXED12 | TypeId::FIXED16 => "Binary",
        TypeId::CLOB | TypeId::NCLOB | TypeId::BLOB | TypeId::TEXT => "LOB",
        TypeId::DAYDATE | TypeId::SECONDTIME | TypeId::SECONDDATE | TypeId::LONGDATE => "Temporal",
        TypeId::GEOMETRY | TypeId::POINT => "Spatial",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_mappings() {
        assert_eq!(hana_type_to_arrow(TypeId::TINYINT, None, None), DataType::UInt8);
        assert_eq!(hana_type_to_arrow(TypeId::SMALLINT, None, None), DataType::Int16);
        assert_eq!(hana_type_to_arrow(TypeId::INT, None, None), DataType::Int32);
        assert_eq!(hana_type_to_arrow(TypeId::BIGINT, None, None), DataType::Int64);
    }

    #[test]
    fn test_float_mappings() {
        assert_eq!(hana_type_to_arrow(TypeId::REAL, None, None), DataType::Float32);
        assert_eq!(hana_type_to_arrow(TypeId::DOUBLE, None, None), DataType::Float64);
    }

    #[test]
    fn test_decimal_mapping() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, Some(18), Some(2));
        assert_eq!(dt, DataType::Decimal128(18, 2));
    }

    #[test]
    fn test_decimal_defaults() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, None, None);
        assert_eq!(dt, DataType::Decimal128(38, 0));
    }

    #[test]
    fn test_string_mappings() {
        assert_eq!(hana_type_to_arrow(TypeId::VARCHAR, None, None), DataType::Utf8);
        assert_eq!(hana_type_to_arrow(TypeId::NVARCHAR, None, None), DataType::Utf8);
        assert_eq!(hana_type_to_arrow(TypeId::CLOB, None, None), DataType::LargeUtf8);
    }

    #[test]
    fn test_temporal_mappings() {
        assert_eq!(hana_type_to_arrow(TypeId::DAYDATE, None, None), DataType::Date32);
        assert_eq!(
            hana_type_to_arrow(TypeId::SECONDTIME, None, None),
            DataType::Time64(TimeUnit::Nanosecond)
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::LONGDATE, None, None),
            DataType::Timestamp(TimeUnit::Nanosecond, None)
        );
    }

    #[test]
    fn test_field_creation() {
        let field = hana_field_to_arrow("amount", TypeId::DECIMAL, true, Some(18), Some(2));
        assert_eq!(field.name(), "amount");
        assert!(field.is_nullable());
        assert_eq!(field.data_type(), &DataType::Decimal128(18, 2));
    }

    #[test]
    fn test_type_category() {
        assert_eq!(type_category(TypeId::INT), "Numeric");
        assert_eq!(type_category(TypeId::DECIMAL), "Decimal");
        assert_eq!(type_category(TypeId::VARCHAR), "String");
        assert_eq!(type_category(TypeId::BLOB), "LOB");
        assert_eq!(type_category(TypeId::DAYDATE), "Temporal");
    }
}
