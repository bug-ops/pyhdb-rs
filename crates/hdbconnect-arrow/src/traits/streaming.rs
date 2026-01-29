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

use std::num::NonZeroUsize;

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
    /// Uses `NonZeroUsize` to prevent division-by-zero and infinite loops.
    /// Default: 65536 (64K rows).
    pub batch_size: NonZeroUsize,

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

    /// Maximum LOB size in bytes before rejecting.
    ///
    /// When set, LOB values exceeding this size will trigger an error
    /// instead of being materialized. This prevents OOM conditions
    /// when processing result sets with large LOB values.
    ///
    /// Default: None (no limit).
    pub max_lob_bytes: Option<usize>,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            // SAFETY: 65536 is non-zero
            batch_size: NonZeroUsize::new(65536).unwrap(),
            string_capacity: 1024 * 1024, // 1MB
            binary_capacity: 1024 * 1024, // 1MB
            coerce_types: false,
            max_lob_bytes: None,
        }
    }
}

impl BatchConfig {
    /// Create a new configuration with the specified batch size.
    ///
    /// # Panics
    ///
    /// Panics if `batch_size` is zero. Use `try_with_batch_size` for fallible construction.
    #[must_use]
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            batch_size: NonZeroUsize::new(batch_size).expect("batch_size must be non-zero"),
            ..Default::default()
        }
    }

    /// Create a new configuration with the specified batch size.
    ///
    /// Returns `None` if `batch_size` is zero.
    #[must_use]
    pub fn try_with_batch_size(batch_size: usize) -> Option<Self> {
        Some(Self {
            batch_size: NonZeroUsize::new(batch_size)?,
            ..Default::default()
        })
    }

    /// Get batch size as usize for iteration.
    #[must_use]
    pub const fn batch_size_usize(&self) -> usize {
        self.batch_size.get()
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

    /// Set the maximum LOB size in bytes.
    ///
    /// LOB values exceeding this size will cause an error during conversion.
    /// Set to `None` to disable the limit (default).
    #[must_use]
    pub const fn max_lob_bytes(mut self, max: Option<usize>) -> Self {
        self.max_lob_bytes = max;
        self
    }

    /// Create a configuration optimized for small result sets.
    ///
    /// Uses smaller batch size and buffer capacities.
    ///
    /// # Panics
    ///
    /// Never panics - the batch size is a compile-time constant.
    #[must_use]
    pub const fn small() -> Self {
        Self {
            // SAFETY: 1024 is non-zero - unwrap() in const context panics at compile time if None
            batch_size: match NonZeroUsize::new(1024) {
                Some(v) => v,
                None => panic!("batch_size must be non-zero"),
            },
            string_capacity: 64 * 1024, // 64KB
            binary_capacity: 64 * 1024, // 64KB
            coerce_types: false,
            max_lob_bytes: None,
        }
    }

    /// Create a configuration optimized for large result sets.
    ///
    /// Uses larger batch size and buffer capacities.
    ///
    /// # Panics
    ///
    /// Never panics - the batch size is a compile-time constant.
    #[must_use]
    pub const fn large() -> Self {
        Self {
            // SAFETY: 131_072 is non-zero - unwrap() in const context panics at compile time if
            // None
            batch_size: match NonZeroUsize::new(131_072) {
                Some(v) => v,
                None => panic!("batch_size must be non-zero"),
            },
            string_capacity: 8 * 1024 * 1024, // 8MB
            binary_capacity: 8 * 1024 * 1024, // 8MB
            coerce_types: false,
            max_lob_bytes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig Default Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size.get(), 65536);
        assert_eq!(config.string_capacity, 1024 * 1024);
        assert!(!config.coerce_types);
        assert!(config.max_lob_bytes.is_none());
    }

    #[test]
    fn test_batch_config_default_binary_capacity() {
        let config = BatchConfig::default();
        assert_eq!(config.binary_capacity, 1024 * 1024);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig Builder Pattern Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_batch_config_builder() {
        let config = BatchConfig::with_batch_size(1000)
            .string_capacity(500)
            .coerce_types(true);

        assert_eq!(config.batch_size.get(), 1000);
        assert_eq!(config.string_capacity, 500);
        assert!(config.coerce_types);
    }

    #[test]
    fn test_batch_config_with_batch_size() {
        let config = BatchConfig::with_batch_size(100);
        assert_eq!(config.batch_size.get(), 100);
        assert_eq!(config.string_capacity, 1024 * 1024);
        assert_eq!(config.binary_capacity, 1024 * 1024);
        assert!(!config.coerce_types);
    }

    #[test]
    fn test_batch_config_string_capacity() {
        let config = BatchConfig::default().string_capacity(2048);
        assert_eq!(config.string_capacity, 2048);
    }

    #[test]
    fn test_batch_config_binary_capacity() {
        let config = BatchConfig::default().binary_capacity(4096);
        assert_eq!(config.binary_capacity, 4096);
    }

    #[test]
    fn test_batch_config_coerce_types_true() {
        let config = BatchConfig::default().coerce_types(true);
        assert!(config.coerce_types);
    }

    #[test]
    fn test_batch_config_coerce_types_false() {
        let config = BatchConfig::default().coerce_types(false);
        assert!(!config.coerce_types);
    }

    #[test]
    fn test_batch_config_builder_chaining() {
        let config = BatchConfig::with_batch_size(5000)
            .string_capacity(10000)
            .binary_capacity(20000)
            .coerce_types(true);

        assert_eq!(config.batch_size.get(), 5000);
        assert_eq!(config.string_capacity, 10000);
        assert_eq!(config.binary_capacity, 20000);
        assert!(config.coerce_types);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig max_lob_bytes Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_batch_config_max_lob_bytes_none() {
        let config = BatchConfig::default();
        assert!(config.max_lob_bytes.is_none());
    }

    #[test]
    fn test_batch_config_max_lob_bytes_some() {
        let config = BatchConfig::default().max_lob_bytes(Some(50_000_000));
        assert_eq!(config.max_lob_bytes, Some(50_000_000));
    }

    #[test]
    fn test_batch_config_max_lob_bytes_reset_to_none() {
        let config = BatchConfig::default()
            .max_lob_bytes(Some(1000))
            .max_lob_bytes(None);
        assert!(config.max_lob_bytes.is_none());
    }

    #[test]
    fn test_batch_config_max_lob_bytes_chaining() {
        let config = BatchConfig::with_batch_size(1000)
            .string_capacity(500)
            .max_lob_bytes(Some(10_000_000));

        assert_eq!(config.batch_size.get(), 1000);
        assert_eq!(config.string_capacity, 500);
        assert_eq!(config.max_lob_bytes, Some(10_000_000));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig Preset Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_batch_config_presets() {
        let small = BatchConfig::small();
        assert_eq!(small.batch_size.get(), 1024);
        assert!(small.max_lob_bytes.is_none());

        let large = BatchConfig::large();
        assert_eq!(large.batch_size.get(), 131072);
        assert!(large.max_lob_bytes.is_none());
    }

    #[test]
    fn test_batch_config_small() {
        let config = BatchConfig::small();
        assert_eq!(config.batch_size.get(), 1024);
        assert_eq!(config.string_capacity, 64 * 1024);
        assert_eq!(config.binary_capacity, 64 * 1024);
        assert!(!config.coerce_types);
    }

    #[test]
    fn test_batch_config_large() {
        let config = BatchConfig::large();
        assert_eq!(config.batch_size.get(), 131_072);
        assert_eq!(config.string_capacity, 8 * 1024 * 1024);
        assert_eq!(config.binary_capacity, 8 * 1024 * 1024);
        assert!(!config.coerce_types);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig Edge Cases
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    #[should_panic(expected = "batch_size must be non-zero")]
    fn test_batch_config_zero_batch_size_panics() {
        let _ = BatchConfig::with_batch_size(0);
    }

    #[test]
    fn test_batch_config_try_with_zero_returns_none() {
        assert!(BatchConfig::try_with_batch_size(0).is_none());
    }

    #[test]
    fn test_batch_config_try_with_nonzero_returns_some() {
        let config = BatchConfig::try_with_batch_size(100);
        assert!(config.is_some());
        assert_eq!(config.unwrap().batch_size.get(), 100);
    }

    #[test]
    fn test_batch_config_zero_string_capacity() {
        let config = BatchConfig::default().string_capacity(0);
        assert_eq!(config.string_capacity, 0);
    }

    #[test]
    fn test_batch_config_zero_binary_capacity() {
        let config = BatchConfig::default().binary_capacity(0);
        assert_eq!(config.binary_capacity, 0);
    }

    #[test]
    fn test_batch_config_large_values() {
        let config = BatchConfig::with_batch_size(1_000_000)
            .string_capacity(100_000_000)
            .binary_capacity(100_000_000);

        assert_eq!(config.batch_size.get(), 1_000_000);
        assert_eq!(config.string_capacity, 100_000_000);
        assert_eq!(config.binary_capacity, 100_000_000);
    }

    #[test]
    fn test_batch_config_batch_size_usize() {
        let config = BatchConfig::with_batch_size(42);
        assert_eq!(config.batch_size_usize(), 42);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig Clone and Debug Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_batch_config_clone() {
        let config1 = BatchConfig::with_batch_size(100)
            .string_capacity(200)
            .max_lob_bytes(Some(1000));
        let config2 = config1.clone();

        assert_eq!(config1.batch_size, config2.batch_size);
        assert_eq!(config1.string_capacity, config2.string_capacity);
        assert_eq!(config1.binary_capacity, config2.binary_capacity);
        assert_eq!(config1.coerce_types, config2.coerce_types);
        assert_eq!(config1.max_lob_bytes, config2.max_lob_bytes);
    }

    #[test]
    fn test_batch_config_debug() {
        let config = BatchConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("BatchConfig"));
        assert!(debug_str.contains("batch_size"));
        assert!(debug_str.contains("max_lob_bytes"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BatchConfig Override Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_batch_config_override_after_preset() {
        let config = BatchConfig::small()
            .string_capacity(1_000_000)
            .coerce_types(true);

        assert_eq!(config.batch_size.get(), 1024);
        assert_eq!(config.string_capacity, 1_000_000);
        assert!(config.coerce_types);
    }

    #[test]
    fn test_batch_config_multiple_overrides() {
        let config = BatchConfig::default()
            .string_capacity(100)
            .string_capacity(200)
            .string_capacity(300);

        assert_eq!(config.string_capacity, 300);
    }
}
