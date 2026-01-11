//! HANA to Arrow schema mapping.
//!
//! Converts HANA `ResultSet` metadata to Arrow Schema.

use std::sync::Arc;

use arrow_schema::{Field, Schema, SchemaRef};

/// Schema mapper for converting HANA metadata to Arrow schema.
///
/// Provides utilities for building Arrow schemas from HANA `ResultSet` metadata.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::schema::SchemaMapper;
///
/// let schema = SchemaMapper::from_result_set(&result_set);
/// let fields = schema.fields();
/// ```
#[derive(Debug, Clone, Default)]
pub struct SchemaMapper;

impl SchemaMapper {
    /// Create a new schema mapper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Build an Arrow schema from HANA `ResultSet` metadata.
    ///
    /// # Arguments
    ///
    /// * `result_set` - The HANA `ResultSet` to extract metadata from
    #[must_use]
    pub fn from_result_set(result_set: &hdbconnect::ResultSet) -> Schema {
        // ResultSetMetadata derefs to Vec<FieldMetadata>
        let metadata = result_set.metadata();
        let fields: Vec<Field> = metadata
            .iter()
            .map(super::super::types::arrow::FieldMetadataExt::to_arrow_field)
            .collect();

        Schema::new(fields)
    }

    /// Build an Arrow schema from a slice of HANA `FieldMetadata`.
    ///
    /// # Arguments
    ///
    /// * `metadata` - Slice of HANA field metadata
    #[must_use]
    pub fn from_field_metadata(metadata: &[hdbconnect::FieldMetadata]) -> Schema {
        let fields: Vec<Field> = metadata
            .iter()
            .map(super::super::types::arrow::FieldMetadataExt::to_arrow_field)
            .collect();

        Schema::new(fields)
    }

    /// Build an Arrow `SchemaRef` from HANA `ResultSet` metadata.
    ///
    /// Returns an `Arc<Schema>` for efficient sharing.
    #[must_use]
    pub fn schema_ref_from_result_set(result_set: &hdbconnect::ResultSet) -> SchemaRef {
        Arc::new(Self::from_result_set(result_set))
    }

    /// Build an Arrow `SchemaRef` from HANA field metadata.
    ///
    /// Returns an `Arc<Schema>` for efficient sharing.
    #[must_use]
    pub fn schema_ref_from_field_metadata(metadata: &[hdbconnect::FieldMetadata]) -> SchemaRef {
        Arc::new(Self::from_field_metadata(metadata))
    }
}

/// Extension trait for building Arrow Schema from HANA metadata.
pub trait SchemaFromHana {
    /// Build an Arrow schema from HANA field metadata.
    fn from_hana_metadata(metadata: &[hdbconnect::FieldMetadata]) -> Schema;
}

impl SchemaFromHana for Schema {
    fn from_hana_metadata(metadata: &[hdbconnect::FieldMetadata]) -> Schema {
        SchemaMapper::from_field_metadata(metadata)
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of_val;

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // SchemaMapper Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_mapper_new() {
        let mapper = SchemaMapper::new();
        assert!(size_of_val(&mapper) == 0);
    }

    #[test]
    fn test_schema_mapper_default() {
        let mapper = SchemaMapper::default();
        assert!(size_of_val(&mapper) == 0);
    }

    #[test]
    fn test_schema_mapper_is_zero_sized() {
        assert_eq!(std::mem::size_of::<SchemaMapper>(), 0);
    }

    #[test]
    fn test_schema_mapper_clone() {
        let mapper1 = SchemaMapper::new();
        #[allow(clippy::clone_on_copy)]
        let mapper2 = mapper1.clone();
        assert!(size_of_val(&mapper2) == 0);
    }

    #[test]
    fn test_schema_mapper_debug() {
        let mapper = SchemaMapper::new();
        let debug_str = format!("{:?}", mapper);
        assert!(debug_str.contains("SchemaMapper"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SchemaMapper Const Construction Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_mapper_const_new() {
        const _MAPPER: SchemaMapper = SchemaMapper::new();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Arrow Field Helper Tests (using hana_field_to_arrow directly)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_arrow_field_int32() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("id", TypeId::INT, false, None, None);
        assert_eq!(field.name(), "id");
        assert!(!field.is_nullable());
        assert_eq!(field.data_type(), &DataType::Int32);
    }

    #[test]
    fn test_arrow_field_nullable() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("value", TypeId::INT, true, None, None);
        assert!(field.is_nullable());
        assert_eq!(field.data_type(), &DataType::Int32);
    }

    #[test]
    fn test_arrow_field_decimal() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("amount", TypeId::DECIMAL, false, Some(18), Some(2));
        assert_eq!(field.data_type(), &DataType::Decimal128(18, 2));
    }

    #[test]
    fn test_arrow_field_varchar() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("name", TypeId::VARCHAR, true, None, None);
        assert_eq!(field.data_type(), &DataType::Utf8);
    }

    #[test]
    fn test_arrow_field_clob() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("content", TypeId::CLOB, true, None, None);
        assert_eq!(field.data_type(), &DataType::LargeUtf8);
    }

    #[test]
    fn test_arrow_field_blob() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("data", TypeId::BLOB, true, None, None);
        assert_eq!(field.data_type(), &DataType::LargeBinary);
    }

    #[test]
    fn test_arrow_field_date() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("created", TypeId::DAYDATE, true, None, None);
        assert_eq!(field.data_type(), &DataType::Date32);
    }

    #[test]
    fn test_arrow_field_timestamp() {
        use arrow_schema::{DataType, TimeUnit};
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("updated", TypeId::LONGDATE, true, None, None);
        assert_eq!(
            field.data_type(),
            &DataType::Timestamp(TimeUnit::Nanosecond, None)
        );
    }

    #[test]
    fn test_arrow_field_boolean() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("active", TypeId::BOOLEAN, false, None, None);
        assert_eq!(field.data_type(), &DataType::Boolean);
    }

    #[test]
    fn test_arrow_field_tinyint() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("tiny", TypeId::TINYINT, false, None, None);
        assert_eq!(field.data_type(), &DataType::UInt8);
    }

    #[test]
    fn test_arrow_field_smallint() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("small", TypeId::SMALLINT, false, None, None);
        assert_eq!(field.data_type(), &DataType::Int16);
    }

    #[test]
    fn test_arrow_field_bigint() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("big", TypeId::BIGINT, false, None, None);
        assert_eq!(field.data_type(), &DataType::Int64);
    }

    #[test]
    fn test_arrow_field_real() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("real_val", TypeId::REAL, false, None, None);
        assert_eq!(field.data_type(), &DataType::Float32);
    }

    #[test]
    fn test_arrow_field_double() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("double_val", TypeId::DOUBLE, false, None, None);
        assert_eq!(field.data_type(), &DataType::Float64);
    }

    #[test]
    fn test_arrow_field_binary() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("bin", TypeId::BINARY, true, None, None);
        assert_eq!(field.data_type(), &DataType::Binary);
    }

    #[test]
    fn test_arrow_field_time() {
        use arrow_schema::{DataType, TimeUnit};
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("time", TypeId::SECONDTIME, true, None, None);
        assert_eq!(field.data_type(), &DataType::Time64(TimeUnit::Nanosecond));
    }

    #[test]
    fn test_arrow_field_geometry() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("geom", TypeId::GEOMETRY, true, None, None);
        assert_eq!(field.data_type(), &DataType::Binary);
    }

    #[test]
    fn test_arrow_field_point() {
        use arrow_schema::DataType;
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("pt", TypeId::POINT, true, None, None);
        assert_eq!(field.data_type(), &DataType::Binary);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Field Name Edge Cases
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_arrow_field_empty_name() {
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("", TypeId::INT, false, None, None);
        assert_eq!(field.name(), "");
    }

    #[test]
    fn test_arrow_field_special_chars_in_name() {
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("col-name_123", TypeId::INT, false, None, None);
        assert_eq!(field.name(), "col-name_123");
    }

    #[test]
    fn test_arrow_field_unicode_name() {
        use hdbconnect::TypeId;

        use crate::types::arrow::hana_field_to_arrow;

        let field = hana_field_to_arrow("列名", TypeId::VARCHAR, true, None, None);
        assert_eq!(field.name(), "列名");
    }
}
