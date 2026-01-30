//! Apache Arrow integration for hdbconnect SAP HANA driver.
//!
//! This crate provides zero-copy conversion from HANA `ResultSets` to Apache Arrow
//! `RecordBatches`, enabling efficient data transfer to Python via `PyO3`.
//!
//! # Features
//!
//! - Type-safe HANA to Arrow type mapping
//! - Streaming `RecordBatch` iteration for large result sets
//! - Sealed traits for API stability
//! - Generic Associated Types (GATs) for lending iterators
//!
//! # Example
//!
//! ```rust,ignore
//! use hdbconnect_arrow::{Result, BatchConfig, HanaBatchProcessor};
//!
//! // Configure batch processing
//! let config = BatchConfig::default();
//! let schema = /* Arrow schema */;
//! let mut processor = HanaBatchProcessor::new(schema, config);
//!
//! // Process rows
//! for row in result_set {
//!     if let Some(batch) = processor.process_row(&row)? {
//!         // Handle batch
//!     }
//! }
//!
//! // Flush remaining rows
//! if let Some(batch) = processor.flush()? {
//!     // Handle final batch
//! }
//! ```
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

// Profiling support: dhat heap allocator (enabled via `--features profiling`)
#[cfg(feature = "profiling")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

pub mod builders;
pub mod conversion;
pub mod error;
#[cfg(feature = "profiling")]
pub mod profiling;
pub mod schema;
#[cfg(test)]
pub mod test_utils;
pub mod traits;
pub mod types;

// Re-export main types for convenience
pub use builders::factory::BuilderFactory;
pub use conversion::{HanaBatchProcessor, rows_to_record_batch};
pub use error::{ArrowConversionError, Result};
pub use schema::mapping::SchemaMapper;
pub use traits::builder::HanaCompatibleBuilder;
pub use traits::row::RowLike;
#[cfg(any(test, feature = "test-utils"))]
pub use traits::row::{MockRow, MockRowBuilder};
pub use traits::sealed::FromHanaValue;
pub use traits::streaming::{BatchConfig, BatchProcessor, LendingBatchIterator};
#[cfg(feature = "async")]
pub use types::arrow::FieldMetadataExtAsync;
pub use types::arrow::{FieldMetadataExt, hana_field_to_arrow, hana_type_to_arrow};
pub use types::conversion::TypeCategory;
pub use types::hana::{
    Binary, Decimal, DecimalPrecision, DecimalScale, HanaTypeCategory, Lob, Numeric, Spatial,
    StringType, Temporal, TypedColumn,
};
