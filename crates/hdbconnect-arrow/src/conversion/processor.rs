//! Batch processor for streaming conversion of HANA rows to `RecordBatch`es.
//!
//! Implements buffered batch creation with configurable batch size.

use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;

use crate::Result;
use crate::builders::factory::BuilderFactory;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::row::RowLike;
use crate::traits::streaming::BatchConfig;

/// Processor that converts HANA rows into Arrow `RecordBatch`es.
///
/// Buffers rows until `batch_size` is reached, then emits a `RecordBatch`.
/// Implements the `BatchProcessor` trait with GAT support.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::conversion::HanaBatchProcessor;
/// use hdbconnect_arrow::traits::streaming::BatchConfig;
///
/// let schema = /* Arrow schema */;
/// let config = BatchConfig::with_batch_size(10000);
/// let mut processor = HanaBatchProcessor::new(Arc::new(schema), config);
///
/// for row in result_set {
///     if let Some(batch) = processor.process_row(row)? {
///         // Process batch
///     }
/// }
///
/// // Don't forget to flush remaining rows
/// if let Some(batch) = processor.flush()? {
///     // Process final batch
/// }
/// ```
pub struct HanaBatchProcessor {
    schema: SchemaRef,
    config: BatchConfig,
    factory: BuilderFactory,
    builders: Vec<Box<dyn HanaCompatibleBuilder>>,
    row_count: usize,
}

impl std::fmt::Debug for HanaBatchProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HanaBatchProcessor")
            .field("schema", &self.schema)
            .field("config", &self.config)
            .field("factory", &"BuilderFactory { ... }")
            .field("builders", &format!("[{} builders]", self.builders.len()))
            .field("row_count", &self.row_count)
            .finish()
    }
}

impl HanaBatchProcessor {
    /// Create a new batch processor.
    ///
    /// # Arguments
    ///
    /// * `schema` - Arrow schema for the batches
    /// * `config` - Batch processing configuration
    #[must_use]
    pub fn new(schema: SchemaRef, config: BatchConfig) -> Self {
        let factory = BuilderFactory::from_config(&config);
        let builders = factory.create_builders_for_schema(&schema);

        Self {
            schema,
            config,
            factory,
            builders,
            row_count: 0,
        }
    }

    /// Create with default configuration.
    #[must_use]
    pub fn with_defaults(schema: SchemaRef) -> Self {
        Self::new(schema, BatchConfig::default())
    }

    /// Process a single row.
    ///
    /// Returns `Ok(Some(batch))` when a batch is ready, `Ok(None)` when more
    /// rows are needed to fill a batch.
    ///
    /// # Errors
    ///
    /// Returns error if value conversion fails or schema mismatches.
    pub fn process_row(&mut self, row: &hdbconnect::Row) -> Result<Option<RecordBatch>> {
        self.process_row_generic(row)
    }

    /// Process a single row using the generic `RowLike` trait.
    ///
    /// This method enables unit testing with `MockRow` instead of requiring
    /// a HANA connection.
    ///
    /// Returns `Ok(Some(batch))` when a batch is ready, `Ok(None)` when more
    /// rows are needed to fill a batch.
    ///
    /// # Errors
    ///
    /// Returns error if value conversion fails or schema mismatches.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use hdbconnect_arrow::traits::row::{MockRow, MockRowBuilder};
    ///
    /// let row = MockRowBuilder::new().int(42).string("test").build();
    /// let result = processor.process_row_generic(&row)?;
    /// ```
    pub fn process_row_generic<R: RowLike>(&mut self, row: &R) -> Result<Option<RecordBatch>> {
        // Validate column count
        if row.len() != self.builders.len() {
            return Err(crate::ArrowConversionError::schema_mismatch(
                self.builders.len(),
                row.len(),
            ));
        }

        // Append row to builders
        for (i, builder) in self.builders.iter_mut().enumerate() {
            let value = row.get(i);

            match value {
                hdbconnect::HdbValue::NULL => builder.append_null(),
                v => builder.append_hana_value(v)?,
            }
        }

        self.row_count += 1;

        // Check if we've reached batch size
        if self.row_count >= self.config.batch_size.get() {
            return Ok(Some(self.finish_current_batch()?));
        }

        Ok(None)
    }

    /// Flush any remaining rows as a final batch.
    ///
    /// # Errors
    ///
    /// Returns error if `RecordBatch` creation fails.
    pub fn flush(&mut self) -> Result<Option<RecordBatch>> {
        if self.row_count == 0 {
            return Ok(None);
        }

        Ok(Some(self.finish_current_batch()?))
    }

    /// Returns the schema of batches produced by this processor.
    #[must_use]
    pub fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }

    /// Returns the current row count in the buffer.
    #[must_use]
    pub const fn buffered_rows(&self) -> usize {
        self.row_count
    }

    /// Finish the current batch and reset builders.
    ///
    /// # Errors
    ///
    /// Returns error if `RecordBatch` creation fails.
    fn finish_current_batch(&mut self) -> Result<RecordBatch> {
        // Finish all builders to get arrays
        let arrays: Vec<_> = self.builders.iter_mut().map(|b| b.finish()).collect();

        // Create RecordBatch
        let batch = RecordBatch::try_new(Arc::clone(&self.schema), arrays)
            .map_err(|e| crate::ArrowConversionError::value_conversion("batch", e.to_string()))?;

        // Use cached factory instead of creating new one
        self.builders = self.factory.create_builders_for_schema(&self.schema);
        self.row_count = 0;

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // Processor Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_creation() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100);

        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_defaults() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_schema() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(Arc::clone(&schema));

        let returned_schema = processor.schema();
        assert_eq!(returned_schema.fields().len(), 2);
        assert_eq!(returned_schema.field(0).name(), "id");
        assert_eq!(returned_schema.field(1).name(), "name");
    }

    #[test]
    fn test_processor_initial_buffered_rows() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Processor with Different Configs
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_with_small_batch_size() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(10);
        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_large_batch_size() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100000);
        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_custom_config() {
        let schema = Arc::new(Schema::new(vec![Field::new("data", DataType::Utf8, true)]));
        let config = BatchConfig::with_batch_size(500)
            .string_capacity(10000)
            .binary_capacity(5000);
        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Processor with Different Schema Types
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_with_empty_schema() {
        let fields: Vec<Field> = vec![];
        let schema = Arc::new(Schema::new(fields));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_single_column_schema() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_multi_column_schema() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("price", DataType::Decimal128(18, 2), false),
            Field::new("is_active", DataType::Boolean, false),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_all_numeric_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("tiny", DataType::UInt8, false),
            Field::new("small", DataType::Int16, false),
            Field::new("int", DataType::Int32, false),
            Field::new("big", DataType::Int64, false),
            Field::new("real", DataType::Float32, false),
            Field::new("double", DataType::Float64, false),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_string_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("small_str", DataType::Utf8, true),
            Field::new("large_str", DataType::LargeUtf8, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_binary_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("bin", DataType::Binary, true),
            Field::new("large_bin", DataType::LargeBinary, true),
            Field::new("fixed_bin", DataType::FixedSizeBinary(16), true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Flush Tests (without rows - tests empty flush)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_flush_empty() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let mut processor = HanaBatchProcessor::with_defaults(schema);

        let result = processor.flush();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_processor_flush_multiple_times_when_empty() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let mut processor = HanaBatchProcessor::with_defaults(schema);

        assert!(processor.flush().unwrap().is_none());
        assert!(processor.flush().unwrap().is_none());
        assert!(processor.flush().unwrap().is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Debug Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_debug() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);

        let debug_str = format!("{:?}", processor);
        assert!(debug_str.contains("HanaBatchProcessor"));
        assert!(debug_str.contains("row_count"));
        assert!(debug_str.contains("builders"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Schema Ref Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_schema_returns_same_schema() {
        let original_schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("value", DataType::Float64, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(Arc::clone(&original_schema));

        let schema1 = processor.schema();
        let schema2 = processor.schema();

        assert!(Arc::ptr_eq(&schema1, &schema2));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MockRow Tests (unit testing without HANA connection)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_process_row_generic_with_mock_row() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(10);
        let mut processor = HanaBatchProcessor::new(schema, config);

        let row = MockRowBuilder::new().int(42).string("test").build();

        let result = processor.process_row_generic(&row);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Not enough rows for batch
        assert_eq!(processor.buffered_rows(), 1);
    }

    #[test]
    fn test_process_row_generic_batch_ready() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(3);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Add rows until batch is ready
        for i in 0..3 {
            let row = MockRowBuilder::new().int(i).build();
            let result = processor.process_row_generic(&row).unwrap();
            if i < 2 {
                assert!(result.is_none());
            } else {
                // Third row should trigger batch
                let batch = result.expect("batch should be ready");
                assert_eq!(batch.num_rows(), 3);
            }
        }
    }

    #[test]
    fn test_process_row_generic_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, true),
            Field::new("name", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Row with null values
        let row = MockRowBuilder::new().null().null().build();

        let result = processor.process_row_generic(&row);
        assert!(result.is_ok());
        assert_eq!(processor.buffered_rows(), 1);
    }

    #[test]
    fn test_process_row_generic_schema_mismatch() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let mut processor = HanaBatchProcessor::with_defaults(schema);

        // Row with wrong number of columns
        let row = MockRowBuilder::new().int(1).string("extra").build();

        let result = processor.process_row_generic(&row);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_schema_mismatch());
    }

    #[test]
    fn test_process_row_generic_flush() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Add some rows (less than batch size)
        for i in 0..5 {
            let row = MockRowBuilder::new().int(i).build();
            processor.process_row_generic(&row).unwrap();
        }

        assert_eq!(processor.buffered_rows(), 5);

        // Flush remaining rows
        let batch = processor
            .flush()
            .unwrap()
            .expect("should have remaining rows");
        assert_eq!(batch.num_rows(), 5);
        assert_eq!(processor.buffered_rows(), 0);
    }
}
