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

use std::collections::HashMap;

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
        TypeId::SECONDDATE | TypeId::LONGDATE => DataType::Timestamp(TimeUnit::Nanosecond, None),

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
    Field::new(
        name,
        hana_type_to_arrow(type_id, precision, scale),
        nullable,
    )
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

/// Extension trait for `hdbconnect_async` `FieldMetadata`.
///
/// Provides convenient conversion methods for async HANA metadata to Arrow types.
#[cfg(feature = "async")]
pub trait FieldMetadataExtAsync {
    /// Convert to Arrow Field.
    fn to_arrow_field(&self) -> Field;

    /// Get the Arrow `DataType` for this field.
    fn arrow_data_type(&self) -> DataType;
}

/// Internal macro to implement `FieldMetadataExt` for different `FieldMetadata` types.
///
/// Both `hdbconnect::FieldMetadata` and `hdbconnect_async::FieldMetadata` have
/// identical interfaces, so we use a macro to avoid code duplication.
macro_rules! impl_field_metadata_ext {
    ($trait_name:ident for $type:ty) => {
        impl $trait_name for $type {
            fn to_arrow_field(&self) -> Field {
                let name = {
                    let display = self.displayname();
                    if display.is_empty() {
                        self.columnname()
                    } else {
                        display
                    }
                };
                let type_id = self.type_id();
                let precision = self.precision();
                let scale = self.scale();

                // Convert i16 precision to Option<u8> safely
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let precision_u8 = (0..=255_i16)
                    .contains(&precision)
                    .then_some(precision as u8);

                // Convert i16 scale to Option<i8> safely
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let scale_i8 = (0..=127_i16).contains(&scale).then_some(scale as i8);

                // Build field metadata for capacity hints
                let mut metadata = HashMap::new();

                // For VARCHAR/NVARCHAR, precision contains max_length
                if matches!(
                    type_id,
                    TypeId::VARCHAR | TypeId::NVARCHAR | TypeId::CHAR | TypeId::NCHAR
                ) {
                    metadata.insert("max_length".to_string(), precision.to_string());
                }

                let field =
                    hana_field_to_arrow(name, type_id, self.is_nullable(), precision_u8, scale_i8);

                if metadata.is_empty() {
                    field
                } else {
                    field.with_metadata(metadata)
                }
            }

            fn arrow_data_type(&self) -> DataType {
                let precision = self.precision();
                let scale = self.scale();

                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let precision_u8 = (0..=255_i16)
                    .contains(&precision)
                    .then_some(precision as u8);

                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let scale_i8 = (0..=127_i16).contains(&scale).then_some(scale as i8);

                hana_type_to_arrow(self.type_id(), precision_u8, scale_i8)
            }
        }
    };
}

// Apply macro for sync version
impl_field_metadata_ext!(FieldMetadataExt for hdbconnect::FieldMetadata);

// Apply macro for async version
#[cfg(feature = "async")]
impl_field_metadata_ext!(FieldMetadataExtAsync for hdbconnect_async::FieldMetadata);

/// Get the HANA type category for a `TypeId`.
///
/// Returns the category name as a static string.
#[must_use]
pub const fn type_category(type_id: TypeId) -> &'static str {
    super::conversion::TypeCategory::from_type_id(type_id).as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // Integer Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_integer_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::TINYINT, None, None),
            DataType::UInt8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::SMALLINT, None, None),
            DataType::Int16
        );
        assert_eq!(hana_type_to_arrow(TypeId::INT, None, None), DataType::Int32);
        assert_eq!(
            hana_type_to_arrow(TypeId::BIGINT, None, None),
            DataType::Int64
        );
    }

    #[test]
    fn test_integer_mappings_ignore_precision_scale() {
        assert_eq!(
            hana_type_to_arrow(TypeId::INT, Some(10), Some(2)),
            DataType::Int32
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::BIGINT, Some(20), Some(0)),
            DataType::Int64
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Float Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_float_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::REAL, None, None),
            DataType::Float32
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::DOUBLE, None, None),
            DataType::Float64
        );
    }

    #[test]
    fn test_float_mappings_ignore_precision_scale() {
        assert_eq!(
            hana_type_to_arrow(TypeId::REAL, Some(24), None),
            DataType::Float32
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::DOUBLE, Some(53), None),
            DataType::Float64
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Decimal Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

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
    fn test_decimal_with_only_precision() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, Some(10), None);
        assert_eq!(dt, DataType::Decimal128(10, 0));
    }

    #[test]
    fn test_decimal_with_only_scale() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, None, Some(5));
        assert_eq!(dt, DataType::Decimal128(38, 5));
    }

    #[test]
    fn test_decimal_max_precision() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, Some(38), Some(10));
        assert_eq!(dt, DataType::Decimal128(38, 10));
    }

    #[test]
    fn test_decimal_min_precision() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, Some(1), Some(0));
        assert_eq!(dt, DataType::Decimal128(1, 0));
    }

    #[test]
    fn test_decimal_precision_clamped_to_max() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, Some(50), Some(10));
        assert_eq!(dt, DataType::Decimal128(38, 10));
    }

    #[test]
    fn test_decimal_zero_scale() {
        let dt = hana_type_to_arrow(TypeId::DECIMAL, Some(18), Some(0));
        assert_eq!(dt, DataType::Decimal128(18, 0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // String Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_string_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::VARCHAR, None, None),
            DataType::Utf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::NVARCHAR, None, None),
            DataType::Utf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::CLOB, None, None),
            DataType::LargeUtf8
        );
    }

    #[test]
    fn test_all_string_type_variants() {
        assert_eq!(hana_type_to_arrow(TypeId::CHAR, None, None), DataType::Utf8);
        assert_eq!(
            hana_type_to_arrow(TypeId::NCHAR, None, None),
            DataType::Utf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::SHORTTEXT, None, None),
            DataType::Utf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::ALPHANUM, None, None),
            DataType::Utf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::STRING, None, None),
            DataType::Utf8
        );
    }

    #[test]
    fn test_lob_string_types() {
        assert_eq!(
            hana_type_to_arrow(TypeId::CLOB, None, None),
            DataType::LargeUtf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::NCLOB, None, None),
            DataType::LargeUtf8
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::TEXT, None, None),
            DataType::LargeUtf8
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Binary Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_binary_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::BINARY, None, None),
            DataType::Binary
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::VARBINARY, None, None),
            DataType::Binary
        );
    }

    #[test]
    fn test_blob_mapping() {
        assert_eq!(
            hana_type_to_arrow(TypeId::BLOB, None, None),
            DataType::LargeBinary
        );
    }

    #[test]
    fn test_fixed_size_binary_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::FIXED8, None, None),
            DataType::FixedSizeBinary(8)
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::FIXED12, None, None),
            DataType::FixedSizeBinary(12)
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::FIXED16, None, None),
            DataType::FixedSizeBinary(16)
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Temporal Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_temporal_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::DAYDATE, None, None),
            DataType::Date32
        );
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
    fn test_seconddate_mapping() {
        assert_eq!(
            hana_type_to_arrow(TypeId::SECONDDATE, None, None),
            DataType::Timestamp(TimeUnit::Nanosecond, None)
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Boolean Type Mapping
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_boolean_mapping() {
        assert_eq!(
            hana_type_to_arrow(TypeId::BOOLEAN, None, None),
            DataType::Boolean
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Spatial Type Mappings
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_spatial_mappings() {
        assert_eq!(
            hana_type_to_arrow(TypeId::GEOMETRY, None, None),
            DataType::Binary
        );
        assert_eq!(
            hana_type_to_arrow(TypeId::POINT, None, None),
            DataType::Binary
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Field Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_field_creation() {
        let field = hana_field_to_arrow("amount", TypeId::DECIMAL, true, Some(18), Some(2));
        assert_eq!(field.name(), "amount");
        assert!(field.is_nullable());
        assert_eq!(field.data_type(), &DataType::Decimal128(18, 2));
    }

    #[test]
    fn test_field_creation_non_nullable() {
        let field = hana_field_to_arrow("id", TypeId::INT, false, None, None);
        assert_eq!(field.name(), "id");
        assert!(!field.is_nullable());
        assert_eq!(field.data_type(), &DataType::Int32);
    }

    #[test]
    fn test_field_creation_string() {
        let field = hana_field_to_arrow("name", TypeId::VARCHAR, true, None, None);
        assert_eq!(field.name(), "name");
        assert!(field.is_nullable());
        assert_eq!(field.data_type(), &DataType::Utf8);
    }

    #[test]
    fn test_field_creation_temporal() {
        let field = hana_field_to_arrow("created_at", TypeId::LONGDATE, false, None, None);
        assert_eq!(field.name(), "created_at");
        assert!(!field.is_nullable());
        assert_eq!(
            field.data_type(),
            &DataType::Timestamp(TimeUnit::Nanosecond, None)
        );
    }

    #[test]
    fn test_field_creation_empty_name() {
        let field = hana_field_to_arrow("", TypeId::INT, false, None, None);
        assert_eq!(field.name(), "");
        assert_eq!(field.data_type(), &DataType::Int32);
    }

    #[test]
    fn test_field_creation_special_characters_in_name() {
        let field = hana_field_to_arrow("column-name_123", TypeId::INT, false, None, None);
        assert_eq!(field.name(), "column-name_123");
    }

    #[test]
    fn test_field_creation_unicode_name() {
        let field = hana_field_to_arrow("列名", TypeId::VARCHAR, true, None, None);
        assert_eq!(field.name(), "列名");
        assert_eq!(field.data_type(), &DataType::Utf8);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Type Category Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_type_category() {
        assert_eq!(type_category(TypeId::INT), "Numeric");
        assert_eq!(type_category(TypeId::DECIMAL), "Decimal");
        assert_eq!(type_category(TypeId::VARCHAR), "String");
        assert_eq!(type_category(TypeId::BLOB), "LOB");
        assert_eq!(type_category(TypeId::DAYDATE), "Temporal");
    }

    #[test]
    fn test_type_category_all_numeric() {
        assert_eq!(type_category(TypeId::TINYINT), "Numeric");
        assert_eq!(type_category(TypeId::SMALLINT), "Numeric");
        assert_eq!(type_category(TypeId::INT), "Numeric");
        assert_eq!(type_category(TypeId::BIGINT), "Numeric");
        assert_eq!(type_category(TypeId::REAL), "Numeric");
        assert_eq!(type_category(TypeId::DOUBLE), "Numeric");
    }

    #[test]
    fn test_type_category_all_string() {
        assert_eq!(type_category(TypeId::CHAR), "String");
        assert_eq!(type_category(TypeId::VARCHAR), "String");
        assert_eq!(type_category(TypeId::NCHAR), "String");
        assert_eq!(type_category(TypeId::NVARCHAR), "String");
        assert_eq!(type_category(TypeId::SHORTTEXT), "String");
        assert_eq!(type_category(TypeId::ALPHANUM), "String");
        assert_eq!(type_category(TypeId::STRING), "String");
    }

    #[test]
    fn test_type_category_all_binary() {
        assert_eq!(type_category(TypeId::BINARY), "Binary");
        assert_eq!(type_category(TypeId::VARBINARY), "Binary");
        assert_eq!(type_category(TypeId::FIXED8), "Binary");
        assert_eq!(type_category(TypeId::FIXED12), "Binary");
        assert_eq!(type_category(TypeId::FIXED16), "Binary");
    }

    #[test]
    fn test_type_category_all_lob() {
        assert_eq!(type_category(TypeId::CLOB), "LOB");
        assert_eq!(type_category(TypeId::NCLOB), "LOB");
        assert_eq!(type_category(TypeId::BLOB), "LOB");
        assert_eq!(type_category(TypeId::TEXT), "LOB");
    }

    #[test]
    fn test_type_category_all_temporal() {
        assert_eq!(type_category(TypeId::DAYDATE), "Temporal");
        assert_eq!(type_category(TypeId::SECONDTIME), "Temporal");
        assert_eq!(type_category(TypeId::SECONDDATE), "Temporal");
        assert_eq!(type_category(TypeId::LONGDATE), "Temporal");
    }

    #[test]
    fn test_type_category_spatial() {
        assert_eq!(type_category(TypeId::GEOMETRY), "Spatial");
        assert_eq!(type_category(TypeId::POINT), "Spatial");
    }

    #[test]
    fn test_type_category_boolean() {
        assert_eq!(type_category(TypeId::BOOLEAN), "Boolean");
    }
}
