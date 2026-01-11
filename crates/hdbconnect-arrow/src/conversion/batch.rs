//! Single-shot conversion from HANA rows to Arrow `RecordBatch`.
//!
//! Provides convenience functions for converting a vector of rows
//! into a `RecordBatch` without streaming.

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;

use crate::Result;
use crate::builders::factory::BuilderFactory;
use crate::traits::builder::HanaCompatibleBuilder;

/// Convert a vector of HANA rows to an Arrow `RecordBatch`.
///
/// This is a convenience function for small result sets that fit in memory.
/// For large result sets, use streaming conversion instead.
///
/// # Arguments
///
/// * `rows` - Vector of HANA rows
/// * `schema` - Arrow schema matching the row structure
///
/// # Errors
///
/// Returns error if:
/// - Schema doesn't match row structure
/// - Value conversion fails
/// - `RecordBatch` creation fails
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::conversion::rows_to_record_batch;
///
/// let rows = vec![/* HANA rows */];
/// let schema = Arc::new(/* Arrow schema */);
/// let batch = rows_to_record_batch(&rows, schema)?;
/// ```
pub fn rows_to_record_batch(rows: &[hdbconnect::Row], schema: SchemaRef) -> Result<RecordBatch> {
    if rows.is_empty() {
        // Return empty batch with correct schema
        return Ok(RecordBatch::new_empty(schema));
    }

    let num_columns = schema.fields().len();

    // Validate first row has correct number of columns
    if let Some(first_row) = rows.first()
        && first_row.len() != num_columns
    {
        return Err(crate::ArrowConversionError::schema_mismatch(
            num_columns,
            first_row.len(),
        ));
    }

    // Create builders
    let factory = BuilderFactory::new(rows.len());
    let mut builders = factory.create_builders_for_schema(&schema);

    // Process all rows
    for row in rows {
        append_row_to_builders(&mut builders, row)?;
    }

    // Finish builders and create arrays
    let arrays: Vec<_> = builders.iter_mut().map(|b| b.finish()).collect();

    // Create RecordBatch
    RecordBatch::try_new(schema, arrays)
        .map_err(|e| crate::ArrowConversionError::value_conversion("batch", e.to_string()))
}

/// Append a single row to a vector of builders.
///
/// # Errors
///
/// Returns error if value conversion fails or column count mismatches.
fn append_row_to_builders(
    builders: &mut [Box<dyn HanaCompatibleBuilder>],
    row: &hdbconnect::Row,
) -> Result<()> {
    if builders.len() != row.len() {
        return Err(crate::ArrowConversionError::schema_mismatch(
            builders.len(),
            row.len(),
        ));
    }

    for (i, builder) in builders.iter_mut().enumerate() {
        // Use index access for row values
        let value = &row[i];

        match value {
            hdbconnect::HdbValue::NULL => builder.append_null(),
            v => builder.append_hana_value(v)?,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // Empty Rows Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_empty_rows() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 1);
    }

    #[test]
    fn test_empty_rows_with_multiple_columns() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("price", DataType::Float64, false),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_empty_rows_with_empty_schema() {
        let fields: Vec<Field> = vec![];
        let schema = Arc::new(Schema::new(fields));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 0);
    }

    #[test]
    fn test_empty_rows_preserves_schema_field_names() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("column_a", DataType::Int32, false),
            Field::new("column_b", DataType::Utf8, true),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.schema().field(0).name(), "column_a");
        assert_eq!(batch.schema().field(1).name(), "column_b");
    }

    #[test]
    fn test_empty_rows_preserves_schema_data_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("int_col", DataType::Int64, false),
            Field::new("str_col", DataType::Utf8, true),
            Field::new("dec_col", DataType::Decimal128(18, 2), false),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.schema().field(0).data_type(), &DataType::Int64);
        assert_eq!(batch.schema().field(1).data_type(), &DataType::Utf8);
        assert_eq!(
            batch.schema().field(2).data_type(),
            &DataType::Decimal128(18, 2)
        );
    }

    #[test]
    fn test_empty_rows_preserves_schema_nullability() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("not_nullable", DataType::Int32, false),
            Field::new("nullable", DataType::Utf8, true),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert!(!batch.schema().field(0).is_nullable());
        assert!(batch.schema().field(1).is_nullable());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Schema Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_with_all_numeric_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("tiny", DataType::UInt8, false),
            Field::new("small", DataType::Int16, false),
            Field::new("int", DataType::Int32, false),
            Field::new("big", DataType::Int64, false),
            Field::new("real", DataType::Float32, false),
            Field::new("double", DataType::Float64, false),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 6);
    }

    #[test]
    fn test_schema_with_string_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("utf8", DataType::Utf8, true),
            Field::new("large_utf8", DataType::LargeUtf8, true),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 2);
    }

    #[test]
    fn test_schema_with_binary_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("bin", DataType::Binary, true),
            Field::new("large_bin", DataType::LargeBinary, true),
            Field::new("fixed_bin", DataType::FixedSizeBinary(8), true),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_schema_with_temporal_types() {
        use arrow_schema::TimeUnit;

        let schema = Arc::new(Schema::new(vec![
            Field::new("date", DataType::Date32, true),
            Field::new("time", DataType::Time64(TimeUnit::Nanosecond), true),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                true,
            ),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_schema_with_decimal_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("dec_small", DataType::Decimal128(5, 2), false),
            Field::new("dec_medium", DataType::Decimal128(18, 4), false),
            Field::new("dec_large", DataType::Decimal128(38, 10), false),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_schema_with_boolean_type() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "flag",
            DataType::Boolean,
            false,
        )]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Complex Schema Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_complex_mixed_schema() {
        use arrow_schema::TimeUnit;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("email", DataType::Utf8, true),
            Field::new("price", DataType::Decimal128(18, 2), false),
            Field::new("quantity", DataType::Int32, false),
            Field::new("is_active", DataType::Boolean, false),
            Field::new("created_at", DataType::Date32, true),
            Field::new(
                "updated_at",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                true,
            ),
            Field::new("data", DataType::Binary, true),
            Field::new("notes", DataType::LargeUtf8, true),
        ]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 10);
        assert_eq!(batch.num_rows(), 0);
    }

    #[test]
    fn test_single_column_schema() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "only",
            DataType::Int32,
            false,
        )]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 1);
        assert_eq!(batch.num_rows(), 0);
    }

    #[test]
    fn test_many_columns_schema() {
        let fields: Vec<Field> = (0..50)
            .map(|i| Field::new(format!("col_{i}"), DataType::Int32, false))
            .collect();

        let schema = Arc::new(Schema::new(fields));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_columns(), 50);
        assert_eq!(batch.num_rows(), 0);
    }
}
