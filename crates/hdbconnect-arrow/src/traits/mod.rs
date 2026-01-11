//! Trait definitions for HANA to Arrow conversion.
//!
//! This module contains the core trait hierarchy:
//!
//! - [`sealed`] - Sealed trait pattern for API stability
//! - [`builder`] - Builder traits for Arrow array construction
//! - [`streaming`] - GAT-based streaming traits for large result sets
//! - [`row`] - Row abstraction for testing without HANA connection

pub mod builder;
pub mod row;
pub mod sealed;
pub mod streaming;

pub use builder::HanaCompatibleBuilder;
pub use row::RowLike;
#[cfg(any(test, feature = "test-utils"))]
pub use row::{MockRow, MockRowBuilder};
pub use sealed::FromHanaValue;
pub use streaming::{BatchConfig, BatchProcessor, LendingBatchIterator};
