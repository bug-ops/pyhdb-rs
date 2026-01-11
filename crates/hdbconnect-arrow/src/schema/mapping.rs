//! HANA to Arrow schema mapping.
//!
//! Converts HANA `ResultSet` metadata to Arrow Schema.

use arrow_schema::{Field, Schema, SchemaRef};
use std::sync::Arc;

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
    use super::*;

    #[test]
    fn test_schema_mapper_new() {
        let mapper = SchemaMapper::new();
        // Just verify it can be created
        assert!(std::mem::size_of_val(&mapper) == 0); // Zero-sized type
    }

    #[test]
    fn test_schema_mapper_default() {
        let mapper = SchemaMapper::default();
        assert!(std::mem::size_of_val(&mapper) == 0);
    }
}
