//! Conversion utilities for HANA rows to Arrow `RecordBatch`es.

pub mod batch;
pub mod processor;

pub use batch::rows_to_record_batch;
pub use processor::{HanaBatchProcessor, SchemaProfile};
