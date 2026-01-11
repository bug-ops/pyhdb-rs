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
    if let Some(first_row) = rows.first() {
        let row_len = first_row.len();
        if row_len != num_columns {
            return Err(crate::ArrowConversionError::schema_mismatch(
                num_columns,
                row_len,
            ));
        }
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

    #[test]
    fn test_empty_rows() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));

        let batch = rows_to_record_batch(&[], Arc::clone(&schema)).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 1);
    }

    #[test]
    fn test_rows_to_batch() {
        // Note: This test requires mock hdbconnect::Row implementation
        // Actual implementation would use real HANA rows

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));

        // Mock rows would go here
        // let rows = vec![...];
        // let batch = rows_to_record_batch(&rows, schema).unwrap();
        // assert_eq!(batch.num_rows(), rows.len());
    }

    #[test]
    fn test_schema_mismatch() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));

        // Mock row with wrong column count
        // This test would verify error handling
    }
}
