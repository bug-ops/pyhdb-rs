//! Test utilities for unit testing without HANA connection.
//!
//! This module provides builders and helpers for creating test data.

use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
use hdbconnect::HdbValue;

use crate::traits::row::{MockRow, MockRowBuilder};

/// Builder for creating Arrow schemas for testing.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::test_utils::SchemaBuilder;
///
/// let schema = SchemaBuilder::new()
///     .int32("id")
///     .utf8("name")
///     .nullable_decimal128("price", 18, 2)
///     .build();
///
/// assert_eq!(schema.fields().len(), 3);
/// ```
#[derive(Debug, Default)]
pub struct SchemaBuilder {
    fields: Vec<Field>,
}

impl SchemaBuilder {
    /// Create a new empty schema builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a non-nullable Int32 field.
    #[must_use]
    pub fn int32(self, name: &str) -> Self {
        self.field(name, DataType::Int32, false)
    }

    /// Add a nullable Int32 field.
    #[must_use]
    pub fn nullable_int32(self, name: &str) -> Self {
        self.field(name, DataType::Int32, true)
    }

    /// Add a non-nullable Int64 field.
    #[must_use]
    pub fn int64(self, name: &str) -> Self {
        self.field(name, DataType::Int64, false)
    }

    /// Add a nullable Int64 field.
    #[must_use]
    pub fn nullable_int64(self, name: &str) -> Self {
        self.field(name, DataType::Int64, true)
    }

    /// Add a non-nullable Float64 field.
    #[must_use]
    pub fn float64(self, name: &str) -> Self {
        self.field(name, DataType::Float64, false)
    }

    /// Add a nullable Float64 field.
    #[must_use]
    pub fn nullable_float64(self, name: &str) -> Self {
        self.field(name, DataType::Float64, true)
    }

    /// Add a non-nullable Utf8 field.
    #[must_use]
    pub fn utf8(self, name: &str) -> Self {
        self.field(name, DataType::Utf8, false)
    }

    /// Add a nullable Utf8 field.
    #[must_use]
    pub fn nullable_utf8(self, name: &str) -> Self {
        self.field(name, DataType::Utf8, true)
    }

    /// Add a non-nullable Boolean field.
    #[must_use]
    pub fn boolean(self, name: &str) -> Self {
        self.field(name, DataType::Boolean, false)
    }

    /// Add a nullable Boolean field.
    #[must_use]
    pub fn nullable_boolean(self, name: &str) -> Self {
        self.field(name, DataType::Boolean, true)
    }

    /// Add a non-nullable Decimal128 field.
    #[must_use]
    pub fn decimal128(self, name: &str, precision: u8, scale: i8) -> Self {
        self.field(name, DataType::Decimal128(precision, scale), false)
    }

    /// Add a nullable Decimal128 field.
    #[must_use]
    pub fn nullable_decimal128(self, name: &str, precision: u8, scale: i8) -> Self {
        self.field(name, DataType::Decimal128(precision, scale), true)
    }

    /// Add a non-nullable Date32 field.
    #[must_use]
    pub fn date32(self, name: &str) -> Self {
        self.field(name, DataType::Date32, false)
    }

    /// Add a nullable Date32 field.
    #[must_use]
    pub fn nullable_date32(self, name: &str) -> Self {
        self.field(name, DataType::Date32, true)
    }

    /// Add a non-nullable Timestamp(Nanosecond) field.
    #[must_use]
    pub fn timestamp_ns(self, name: &str) -> Self {
        self.field(name, DataType::Timestamp(TimeUnit::Nanosecond, None), false)
    }

    /// Add a nullable Timestamp(Nanosecond) field.
    #[must_use]
    pub fn nullable_timestamp_ns(self, name: &str) -> Self {
        self.field(name, DataType::Timestamp(TimeUnit::Nanosecond, None), true)
    }

    /// Add a non-nullable Binary field.
    #[must_use]
    pub fn binary(self, name: &str) -> Self {
        self.field(name, DataType::Binary, false)
    }

    /// Add a nullable Binary field.
    #[must_use]
    pub fn nullable_binary(self, name: &str) -> Self {
        self.field(name, DataType::Binary, true)
    }

    /// Add a field with explicit type and nullability.
    #[must_use]
    pub fn field(mut self, name: &str, data_type: DataType, nullable: bool) -> Self {
        self.fields.push(Field::new(name, data_type, nullable));
        self
    }

    /// Build the schema.
    #[must_use]
    pub fn build(self) -> Schema {
        Schema::new(self.fields)
    }

    /// Build the schema wrapped in Arc.
    #[must_use]
    pub fn build_ref(self) -> SchemaRef {
        Arc::new(self.build())
    }
}

/// Create a simple test schema with common column types.
///
/// Schema: id (Int32), name (Utf8), value (Float64), active (Boolean)
#[must_use]
pub fn simple_test_schema() -> SchemaRef {
    SchemaBuilder::new()
        .int32("id")
        .nullable_utf8("name")
        .nullable_float64("value")
        .nullable_boolean("active")
        .build_ref()
}

/// Create test rows matching `simple_test_schema()`.
#[must_use]
pub fn simple_test_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            MockRowBuilder::new()
                .int(i32::try_from(i).unwrap_or(0))
                .string(format!("name_{i}"))
                .double(f64::from(i as i32) * 1.5)
                .boolean(i % 2 == 0)
                .build()
        })
        .collect()
}

/// Create a mock row with mixed types for testing.
#[must_use]
pub fn mixed_type_row() -> MockRow {
    MockRowBuilder::new()
        .tinyint(1)
        .smallint(2)
        .int(3)
        .bigint(4)
        .real(1.5)
        .double(2.5)
        .string("test")
        .boolean(true)
        .null()
        .build()
}

/// Create a mock row with all NULL values.
#[must_use]
pub fn null_row(column_count: usize) -> MockRow {
    MockRow::nulls(column_count)
}

/// Create test data for specific schema.
#[derive(Debug)]
pub struct TestDataBuilder {
    schema: SchemaRef,
    rows: Vec<MockRow>,
}

impl TestDataBuilder {
    /// Create a new builder with the given schema.
    #[must_use]
    pub fn new(schema: SchemaRef) -> Self {
        Self {
            schema,
            rows: Vec::new(),
        }
    }

    /// Add a row using a builder function.
    #[must_use]
    pub fn row<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MockRowBuilder) -> MockRow,
    {
        self.rows.push(f(MockRowBuilder::new()));
        self
    }

    /// Add a pre-built row.
    #[must_use]
    pub fn add_row(mut self, row: MockRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Add multiple rows.
    #[must_use]
    pub fn add_rows(mut self, rows: impl IntoIterator<Item = MockRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Get the schema.
    #[must_use]
    pub fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }

    /// Get the rows.
    #[must_use]
    pub fn rows(&self) -> &[MockRow] {
        &self.rows
    }

    /// Consume and return (schema, rows).
    #[must_use]
    pub fn build(self) -> (SchemaRef, Vec<MockRow>) {
        (self.schema, self.rows)
    }
}

/// Assertion helper for HdbValue comparisons.
pub fn assert_hdb_value_eq(actual: &HdbValue<'_>, expected: &HdbValue<'_>) {
    match (actual, expected) {
        (HdbValue::INT(a), HdbValue::INT(e)) => assert_eq!(a, e),
        (HdbValue::BIGINT(a), HdbValue::BIGINT(e)) => assert_eq!(a, e),
        (HdbValue::DOUBLE(a), HdbValue::DOUBLE(e)) => {
            assert!((a - e).abs() < f64::EPSILON, "float mismatch: {a} != {e}");
        }
        (HdbValue::STRING(a), HdbValue::STRING(e)) => assert_eq!(a, e),
        (HdbValue::BOOLEAN(a), HdbValue::BOOLEAN(e)) => assert_eq!(a, e),
        (HdbValue::NULL, HdbValue::NULL) => {}
        _ => panic!("HdbValue type mismatch: {actual:?} != {expected:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::row::RowLike;

    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new()
            .int32("id")
            .nullable_utf8("name")
            .float64("value")
            .build();

        assert_eq!(schema.fields().len(), 3);
        assert_eq!(schema.field(0).name(), "id");
        assert!(!schema.field(0).is_nullable());
        assert_eq!(schema.field(1).name(), "name");
        assert!(schema.field(1).is_nullable());
    }

    #[test]
    fn test_simple_test_schema() {
        let schema = simple_test_schema();
        assert_eq!(schema.fields().len(), 4);
    }

    #[test]
    fn test_simple_test_rows() {
        let rows = simple_test_rows(5);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].len(), 4);
    }

    #[test]
    fn test_test_data_builder() {
        let schema = SchemaBuilder::new()
            .int32("id")
            .nullable_utf8("name")
            .build_ref();

        let (schema, rows) = TestDataBuilder::new(schema)
            .row(|b| b.int(1).string("one").build())
            .row(|b| b.int(2).string("two").build())
            .build();

        assert_eq!(schema.fields().len(), 2);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_null_row() {
        let row = null_row(5);
        assert_eq!(row.len(), 5);
        for i in 0..5 {
            assert!(matches!(row.get(i), HdbValue::NULL));
        }
    }

    #[test]
    fn test_mixed_type_row() {
        let row = mixed_type_row();
        assert_eq!(row.len(), 9);
    }

    #[test]
    fn test_assert_hdb_value_eq() {
        assert_hdb_value_eq(&HdbValue::INT(42), &HdbValue::INT(42));
        assert_hdb_value_eq(&HdbValue::NULL, &HdbValue::NULL);
    }
}
