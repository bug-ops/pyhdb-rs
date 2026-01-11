//! Streaming traits using Generic Associated Types (GATs).
//!
//! GATs enable streaming patterns where returned items can borrow from
//! the iterator itself, avoiding unnecessary allocations.
//!
//! # Why GATs?
//!
//! Traditional iterators cannot yield references to internal state because
//! the `Item` type is fixed at trait definition time. GATs allow the item
//! type to have a lifetime parameter tied to `&self`, enabling zero-copy
//! streaming.
//!
//! # Example
//!
//! ```rust,ignore
//! impl LendingBatchIterator for MyReader {
//!     type Item<'a> = &'a RecordBatch where Self: 'a;
//!
//!     fn next_batch(&mut self) -> Option<Result<Self::Item<'_>>> {
//!         // Return reference to internal buffer
//!         self.buffer.as_ref().map(Ok)
//!     }
//! }
//! ```

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;

/// A lending iterator that yields borrowed record batches.
///
/// This trait uses GATs to allow the yielded items to borrow from `self`,
/// enabling zero-copy streaming without intermediate allocations.
///
/// Unlike `Iterator`, which owns its items, `LendingBatchIterator` can
/// yield references to internal buffers that are reused between iterations.
pub trait LendingBatchIterator {
    /// The type of items yielded by this iterator.
    ///
    /// The lifetime parameter `'a` allows items to borrow from `self`.
    type Item<'a>
    where
        Self: 'a;

    /// Advance the iterator and return the next batch.
    ///
    /// Returns `None` when iteration is complete.
    fn next_batch(&mut self) -> Option<crate::Result<Self::Item<'_>>>;

    /// Returns the schema of batches produced by this iterator.
    fn schema(&self) -> SchemaRef;

    /// Returns a hint of the remaining number of batches, if known.
    ///
    /// Returns `(lower_bound, upper_bound)` where `upper_bound` is `None`
    /// if the count is unknown.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// A batch processor that transforms input rows into Arrow `RecordBatches`.
///
/// Uses GATs to allow flexible lifetime relationships between the processor
/// and the batches it produces.
pub trait BatchProcessor {
    /// Configuration type for this processor.
    type Config;

    /// Error type produced by this processor.
    type Error: std::error::Error;

    /// The batch type produced, which may borrow from the processor.
    type Batch<'a>
    where
        Self: 'a;

    /// Create a new processor with the given configuration.
    fn new(config: Self::Config, schema: SchemaRef) -> Self;

    /// Process a chunk of rows into a batch.
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails.
    fn process<'a>(&'a mut self, rows: &[hdbconnect::Row]) -> Result<Self::Batch<'a>, Self::Error>;

    /// Flush any buffered data and return the final batch.
    ///
    /// # Errors
    ///
    /// Returns an error if flushing fails.
    fn flush(&mut self) -> Result<Option<RecordBatch>, Self::Error>;
}

/// Configuration for batch processing.
///
/// Controls memory allocation and processing behavior for batch conversion.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of rows per batch.
    ///
    /// Larger batches reduce overhead but use more memory.
    /// Default: 65536 (64K rows).
    pub batch_size: usize,

    /// Initial capacity for string builders (bytes).
    ///
    /// Pre-allocating string capacity reduces reallocations.
    /// Default: 1MB.
    pub string_capacity: usize,

    /// Initial capacity for binary builders (bytes).
    ///
    /// Pre-allocating binary capacity reduces reallocations.
    /// Default: 1MB.
    pub binary_capacity: usize,

    /// Whether to coerce types when possible.
    ///
    /// When true, numeric types may be widened (e.g., INT to BIGINT)
    /// to avoid precision loss. Default: false.
    pub coerce_types: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 65536,
            string_capacity: 1024 * 1024, // 1MB
            binary_capacity: 1024 * 1024, // 1MB
            coerce_types: false,
        }
    }
}

impl BatchConfig {
    /// Create a new configuration with the specified batch size.
    #[must_use]
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            batch_size,
            ..Default::default()
        }
    }

    /// Set the string builder capacity.
    #[must_use]
    pub const fn string_capacity(mut self, capacity: usize) -> Self {
        self.string_capacity = capacity;
        self
    }

    /// Set the binary builder capacity.
    #[must_use]
    pub const fn binary_capacity(mut self, capacity: usize) -> Self {
        self.binary_capacity = capacity;
        self
    }

    /// Enable or disable type coercion.
    #[must_use]
    pub const fn coerce_types(mut self, coerce: bool) -> Self {
        self.coerce_types = coerce;
        self
    }

    /// Create a configuration optimized for small result sets.
    ///
    /// Uses smaller batch size and buffer capacities.
    #[must_use]
    pub const fn small() -> Self {
        Self {
            batch_size: 1024,
            string_capacity: 64 * 1024, // 64KB
            binary_capacity: 64 * 1024, // 64KB
            coerce_types: false,
        }
    }

    /// Create a configuration optimized for large result sets.
    ///
    /// Uses larger batch size and buffer capacities.
    #[must_use]
    pub const fn large() -> Self {
        Self {
            batch_size: 131_072,              // 128K rows
            string_capacity: 8 * 1024 * 1024, // 8MB
            binary_capacity: 8 * 1024 * 1024, // 8MB
            coerce_types: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 65536);
        assert_eq!(config.string_capacity, 1024 * 1024);
        assert!(!config.coerce_types);
    }

    #[test]
    fn test_batch_config_builder() {
        let config = BatchConfig::with_batch_size(1000)
            .string_capacity(500)
            .coerce_types(true);

        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.string_capacity, 500);
        assert!(config.coerce_types);
    }

    #[test]
    fn test_batch_config_presets() {
        let small = BatchConfig::small();
        assert_eq!(small.batch_size, 1024);

        let large = BatchConfig::large();
        assert_eq!(large.batch_size, 131072);
    }
}
