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
//! use hdbconnect_arrow::{Result, BatchConfig};
//!
//! // Configure batch processing
//! let config = BatchConfig::default();
//! // ... use with HANA result set processing
//! ```
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod error;
pub mod schema;
pub mod traits;
pub mod types;

// Re-export main types for convenience
pub use error::{ArrowConversionError, Result};
pub use schema::mapping::SchemaMapper;
pub use traits::builder::HanaCompatibleBuilder;
pub use traits::sealed::FromHanaValue;
pub use traits::streaming::{BatchConfig, BatchProcessor, LendingBatchIterator};
pub use types::arrow::{hana_field_to_arrow, hana_type_to_arrow, FieldMetadataExt};
pub use types::hana::{
    Binary, Decimal, DecimalPrecision, DecimalScale, HanaTypeCategory, Lob, Numeric, Spatial,
    StringType, Temporal, TypedColumn,
};
