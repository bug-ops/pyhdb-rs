//! Reader module for Arrow `RecordBatchReader`.
//!
//! Provides streaming Arrow results via `PyCapsule` interface.

pub mod wrapper;

pub use wrapper::PyRecordBatchReader;
