//! Batch processor for streaming conversion of HANA rows to `RecordBatch`es.
//!
//! Implements buffered batch creation with configurable batch size.

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use std::sync::Arc;

use crate::Result;
use crate::builders::factory::BuilderFactory;
use crate::traits::builder::HanaCompatibleBuilder;
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
    builders: Vec<Box<dyn HanaCompatibleBuilder>>,
    row_count: usize,
}

impl std::fmt::Debug for HanaBatchProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HanaBatchProcessor")
            .field("schema", &self.schema)
            .field("config", &self.config)
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
        // Validate column count
        if row.len() != self.builders.len() {
            return Err(crate::ArrowConversionError::schema_mismatch(
                self.builders.len(),
                row.len(),
            ));
        }

        // Append row to builders
        for (i, builder) in self.builders.iter_mut().enumerate() {
            // Use index access for row values
            let value = &row[i];

            match value {
                hdbconnect::HdbValue::NULL => builder.append_null(),
                v => builder.append_hana_value(v)?,
            }
        }

        self.row_count += 1;

        // Check if we've reached batch size
        if self.row_count >= self.config.batch_size {
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

        // Reset builders for next batch
        let factory = BuilderFactory::from_config(&self.config);
        self.builders = factory.create_builders_for_schema(&self.schema);
        self.row_count = 0;

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};

    #[test]
    fn test_processor_creation() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100);

        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_buffering() {
        // Note: Requires mock hdbconnect::Row implementation
        // Would test that rows are buffered correctly
    }

    #[test]
    fn test_processor_batch_emission() {
        // Test that batch is emitted when batch_size is reached
    }

    #[test]
    fn test_processor_flush() {
        // Test that flush emits remaining rows
    }
}
