//! `PyO3` `RecordBatchReader` wrapper.
//!
//! Implements __`arrow_c_stream`__ for zero-copy Arrow data transfer.
//!
//! # Memory Guarantees
//!
//! Both sync and async readers provide `O(batch_size)` memory usage through
//! true row-by-row streaming:
//!
//! - **Sync API**: Uses `ResultSet::next()` iterator for streaming
//! - **Async API**: Uses `ResultSet::next_row().await` for streaming
//!
//! ```python
//! # Both APIs stream efficiently
//! sync_conn = pyhdb_rs.connect(uri)
//! reader = sync_conn.execute_arrow("SELECT * FROM large_table", batch_size=10000)
//!
//! async_conn = await pyhdb_rs.connect_async(uri)
//! reader = await async_conn.execute_arrow("SELECT * FROM large_table", batch_size=10000)
//! ```
//!
//! # Safety Model
//!
//! This module contains `unsafe impl Send` for `StreamingReader` and
//! `AsyncStreamingReader`. This section documents the safety guarantees
//! and usage patterns.
//!
//! ## Why `unsafe impl Send` is Required
//!
//! `pyo3_arrow::PyRecordBatchReader::new()` requires its iterator to be `Send`.
//! This allows the reader to be moved across thread boundaries, which is
//! necessary for Python's threading model and async runtimes.
//!
//! `hdbconnect::ResultSet` is `!Send` because it may contain non-thread-safe
//! internals (TCP stream state, buffers). However, we can safely implement
//! `Send` because we maintain strict invariants.
//!
//! ## Safety Invariants
//!
//! 1. **Single-Owner Semantics**: Only one `StreamingReader` owns the `ResultSet` at a time.
//!    Ownership is transferred via `std::mem::replace` in `fetch_arrow()`.
//!
//! 2. **GIL Synchronization**: All Python object access requires holding the GIL. `PyO3`'s type
//!    system enforces this at compile time.
//!
//! 3. **Sequential Iteration**: The Arrow C Stream protocol is inherently sequential. `get_next()`
//!    is called one batch at a time.
//!
//! 4. **Lifetime Bound**: The reader's lifetime is tied to the Python object, preventing
//!    use-after-free.
//!
//! ## Correct Usage
//!
//! ```python
//! # Safe: Reader moved to consumer
//! reader = conn.execute_arrow("SELECT * FROM table")
//! df = polars.from_arrow(reader)  # Reader consumed
//! ```
//!
//! ## Anti-Patterns (DO NOT DO)
//!
//! ```python
//! # UNSAFE: Concurrent access
//! reader = conn.execute_arrow("SELECT * FROM table")
//! # DO NOT access reader from multiple threads simultaneously
//! ```
//!
//! ## Anti-Pattern: Attempting to Clone Reader (Rust)
//!
//! `StreamingReader` does NOT implement `Clone`. The following will not compile:
//!
//! ```compile_fail
//! // ERROR: Clone is intentionally not implemented for StreamingReader
//! // because it would violate single-owner semantics.
//! fn attempt_clone(reader: hdbconnect_py::reader::StreamingReader) {
//!     let _cloned = reader.clone();  // no method named `clone` found
//! }
//! ```
//!
//! ## Review Policy
//!
//! `unsafe impl Send` implementations are reviewed every 6 months.
//! See `SAFETY REVIEW` comments above each impl.

use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use hdbconnect_arrow::{BatchConfig, FieldMetadataExt, HanaBatchProcessor};
use pyo3::prelude::*;

use crate::error::PyHdbError;

#[cfg(debug_assertions)]
mod safety_validator {
    use std::sync::atomic::{AtomicBool, Ordering};

    pub struct IterationGuard {
        is_iterating: AtomicBool,
    }

    impl IterationGuard {
        pub const fn new() -> Self {
            Self {
                is_iterating: AtomicBool::new(false),
            }
        }

        pub fn begin_iteration(&self) {
            let was_iterating = self.is_iterating.swap(true, Ordering::SeqCst);
            assert!(
                !was_iterating,
                "SAFETY VIOLATION: Concurrent iteration detected on StreamingReader. \
                 The Arrow C Stream protocol requires sequential access."
            );
        }

        pub fn end_iteration(&self) {
            self.is_iterating.store(false, Ordering::SeqCst);
        }
    }
}

#[cfg(not(debug_assertions))]
mod safety_validator {
    pub struct IterationGuard;

    impl IterationGuard {
        pub const fn new() -> Self {
            Self
        }
        #[inline(always)]
        #[allow(clippy::unused_self, clippy::missing_const_for_fn)]
        pub fn begin_iteration(&self) {}
        #[inline(always)]
        #[allow(clippy::unused_self, clippy::missing_const_for_fn)]
        pub fn end_iteration(&self) {}
    }
}

/// Streams Arrow `RecordBatches` from HANA result set.
/// Implements `__arrow_c_stream__` for zero-copy transfer.
#[pyclass(name = "RecordBatchReader", module = "pyhdb_rs._core")]
pub struct PyRecordBatchReader {
    inner: Option<pyo3_arrow::PyRecordBatchReader>,
}

impl std::fmt::Debug for PyRecordBatchReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyRecordBatchReader")
            .field("has_reader", &self.inner.is_some())
            .finish()
    }
}

struct StreamingReader {
    result_set: hdbconnect::ResultSet,
    processor: HanaBatchProcessor,
    schema: SchemaRef,
    exhausted: bool,
    guard: safety_validator::IterationGuard,
}

// SAFETY REVIEW: 2026-01-28
// Reviewer: rust-architect
// Next review: 2026-07-28 (6 months)
// Invariants verified: [arrow-sequential, gil-sync, single-owner, lifetime-bound]
//
// Safety justification:
// - ResultSet is !Send but we maintain single-owner semantics via mem::replace
// - GIL synchronization enforced by PyO3 type system
// - Arrow C Stream protocol is sequential by design
// - Lifetime bound to Python object prevents use-after-free
//
// hdbconnect::ResultSet is !Send because it may contain non-thread-safe internals
// (e.g., TCP stream state, internal buffers). However, we guarantee thread safety
// through the following invariants:
//
// INVARIANTS:
// 1. Single-owner semantics: StreamingReader takes ownership of ResultSet via std::mem::replace in
//    fetch_arrow(), transferring it out of the Mutex-protected CursorInner. Only one
//    StreamingReader can own a ResultSet at a time.
//
// 2. GIL synchronization: pyo3_arrow::PyRecordBatchReader exposes the iterator through Python's
//    Arrow C Stream interface. All access from Python code requires holding the GIL, which
//    serializes access.
//
// 3. No concurrent iteration: The Arrow C Stream protocol is inherently sequential - get_next() is
//    called one batch at a time. The RecordBatchReader trait's Iterator impl is not accessed from
//    multiple threads simultaneously.
//
// 4. Lifetime bound to Python object: The PyRecordBatchReader Python object prevents the underlying
//    reader from being accessed after the object is dropped.
//
// VERIFICATION: If pyo3_arrow ever changes to access iterators without GIL held,
// this impl would become unsound. Review pyo3_arrow updates for changes to
// thread-safety guarantees.
unsafe impl Send for StreamingReader {}

impl StreamingReader {
    fn new(result_set: hdbconnect::ResultSet, batch_size: usize) -> Self {
        let schema = Self::build_schema(&result_set);
        let config = BatchConfig::with_batch_size(batch_size);
        let processor = HanaBatchProcessor::new(Arc::clone(&schema), config);

        Self {
            result_set,
            processor,
            schema,
            exhausted: false,
            guard: safety_validator::IterationGuard::new(),
        }
    }

    fn build_schema(result_set: &hdbconnect::ResultSet) -> SchemaRef {
        let fields: Vec<_> = result_set
            .metadata()
            .iter()
            .map(FieldMetadataExt::to_arrow_field)
            .collect();

        Arc::new(arrow_schema::Schema::new(fields))
    }

    #[allow(clippy::needless_continue)]
    fn next_inner(&mut self) -> Option<Result<RecordBatch, arrow_schema::ArrowError>> {
        if self.exhausted {
            return None;
        }

        loop {
            match self.result_set.next() {
                Some(Ok(row)) => match self.processor.process_row(&row) {
                    Ok(Some(batch)) => return Some(Ok(batch)),
                    Ok(None) => continue,
                    Err(e) => {
                        return Some(Err(arrow_schema::ArrowError::ExternalError(Box::new(
                            std::io::Error::other(e.to_string()),
                        ))));
                    }
                },
                Some(Err(e)) => {
                    self.exhausted = true;
                    return Some(Err(arrow_schema::ArrowError::ExternalError(Box::new(
                        std::io::Error::other(e.to_string()),
                    ))));
                }
                None => {
                    self.exhausted = true;
                    return match self.processor.flush() {
                        Ok(Some(batch)) => Some(Ok(batch)),
                        Ok(None) => None,
                        Err(e) => Some(Err(arrow_schema::ArrowError::ExternalError(Box::new(
                            std::io::Error::other(e.to_string()),
                        )))),
                    };
                }
            }
        }
    }
}

impl Iterator for StreamingReader {
    type Item = Result<RecordBatch, arrow_schema::ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.guard.begin_iteration();
        let result = self.next_inner();
        self.guard.end_iteration();
        result
    }
}

impl arrow_array::RecordBatchReader for StreamingReader {
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }
}

impl PyRecordBatchReader {
    pub fn from_resultset(result_set: hdbconnect::ResultSet, batch_size: usize) -> PyResult<Self> {
        let reader = StreamingReader::new(result_set, batch_size);
        let pyo3_reader = pyo3_arrow::PyRecordBatchReader::new(Box::new(reader));
        Ok(Self {
            inner: Some(pyo3_reader),
        })
    }

    /// Creates a reader from async result set with true streaming semantics.
    ///
    /// # Memory Guarantees
    ///
    /// Memory usage is `O(batch_size)` - constant regardless of total row count.
    /// Rows are fetched one-by-one via `next_row().await` and batched incrementally.
    #[cfg(feature = "async")]
    pub fn from_resultset_async(
        result_set: hdbconnect_async::ResultSet,
        batch_size: usize,
    ) -> PyResult<Self> {
        let reader = AsyncStreamingReader::new(result_set, batch_size);
        let pyo3_reader = pyo3_arrow::PyRecordBatchReader::new(Box::new(reader));
        Ok(Self {
            inner: Some(pyo3_reader),
        })
    }
}

/// Converts conversion error to Arrow error for channel transmission.
#[cfg(feature = "async")]
fn to_arrow_error(e: impl std::fmt::Display) -> arrow_schema::ArrowError {
    arrow_schema::ArrowError::ExternalError(Box::new(std::io::Error::other(e.to_string())))
}

/// Async streaming reader with true row-by-row streaming and async backpressure.
///
/// Uses `tokio::sync::mpsc` channel and `next_row().await` for efficient streaming:
///
/// - `O(batch_size)` memory: Rows fetched one-by-one, not materialized
/// - Async backpressure: `sender.send().await` pauses producer when channel full
/// - Early termination: Consumer can stop iteration without loading all data
///
/// The channel buffer size is set to 4 batches, providing a balance between
/// throughput (pipelining) and memory usage.
///
/// # Thread Safety
///
/// The `blocking_recv()` call in `Iterator::next` is safe because:
/// 1. Consumer runs in Python GIL thread (not a tokio worker thread)
/// 2. Producer runs on tokio runtime with async send semantics
/// 3. This pattern is explicitly supported by tokio for bridging async/sync code
#[cfg(feature = "async")]
struct AsyncStreamingReader {
    receiver: tokio::sync::mpsc::Receiver<Result<RecordBatch, arrow_schema::ArrowError>>,
    schema: SchemaRef,
    guard: safety_validator::IterationGuard,
}

// Safety justification:
// - AsyncStreamingReader contains only Send types:
//   - tokio::sync::mpsc::Receiver<T> is Send if T: Send (RecordBatch is Send)
//   - SchemaRef (Arc<Schema>) is Send + Sync
//   - IterationGuard contains AtomicBool which is Send + Sync
// - No shared mutable state, no thread-unsafe types, no raw pointers
//
// The tokio channel receiver is moved into this struct and never shared.
// All access is sequential via the Iterator trait.
#[cfg(feature = "async")]
unsafe impl Send for AsyncStreamingReader {}

#[cfg(feature = "async")]
impl AsyncStreamingReader {
    /// Channel buffer size (number of batches to buffer before backpressure).
    const CHANNEL_BUFFER_SIZE: usize = 4;

    /// Creates async reader with true streaming semantics.
    ///
    /// # Memory Guarantees
    ///
    /// Memory usage is `O(batch_size)` - constant regardless of total row count.
    /// Uses `next_row().await` to fetch rows incrementally, never materializing
    /// the entire result set.
    fn new(mut result_set: hdbconnect_async::ResultSet, batch_size: usize) -> Self {
        let schema = Self::build_schema(&result_set);
        let config = BatchConfig::with_batch_size(batch_size);

        // tokio::sync::mpsc for async producer with backpressure
        let (sender, receiver) = tokio::sync::mpsc::channel(Self::CHANNEL_BUFFER_SIZE);

        let schema_clone = Arc::clone(&schema);

        // Spawn streaming producer task
        tokio::task::spawn(async move {
            let mut processor = HanaBatchProcessor::new(schema_clone, config);

            // True streaming: fetch rows one-by-one via next_row().await
            loop {
                match result_set.next_row().await {
                    Ok(Some(row)) => {
                        match processor.process_row(&row) {
                            Ok(Some(batch)) => {
                                // Async send with backpressure - pauses if channel full
                                if sender.send(Ok(batch)).await.is_err() {
                                    return; // Consumer dropped, stop producing
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                let _ = sender.send(Err(to_arrow_error(e))).await;
                                return;
                            }
                        }
                    }
                    Ok(None) => {
                        // End of result set - flush remaining buffered rows
                        match processor.flush() {
                            Ok(Some(final_batch)) => {
                                let _ = sender.send(Ok(final_batch)).await;
                            }
                            Err(e) => {
                                let _ = sender.send(Err(to_arrow_error(e))).await;
                            }
                            Ok(None) => {}
                        }
                        return;
                    }
                    Err(e) => {
                        let _ = sender.send(Err(to_arrow_error(e))).await;
                        return;
                    }
                }
            }
            // Sender drops on function return (consumer dropped or error occurred)
        });

        Self {
            receiver,
            schema,
            guard: safety_validator::IterationGuard::new(),
        }
    }

    fn build_schema(result_set: &hdbconnect_async::ResultSet) -> SchemaRef {
        let fields: Vec<_> = result_set
            .metadata()
            .iter()
            .map(FieldMetadataExt::to_arrow_field)
            .collect();

        Arc::new(arrow_schema::Schema::new(fields))
    }
}

#[cfg(feature = "async")]
impl Iterator for AsyncStreamingReader {
    type Item = Result<RecordBatch, arrow_schema::ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.guard.begin_iteration();
        // blocking_recv() is safe here because:
        // 1. Consumer runs in Python GIL thread (not tokio worker)
        // 2. Producer is on tokio runtime with async send
        // 3. This pattern is the recommended way to bridge async/sync in tokio
        let result = self.receiver.blocking_recv();
        self.guard.end_iteration();
        result
    }
}

#[cfg(feature = "async")]
impl arrow_array::RecordBatchReader for AsyncStreamingReader {
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }
}

#[pymethods]
impl PyRecordBatchReader {
    #[allow(clippy::wrong_self_convention)]
    fn to_pyarrow<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyHdbError::programming("reader already consumed"))?;

        inner.into_pyarrow(py)
    }

    fn schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self
            .inner
            .as_ref()
            .ok_or_else(|| PyHdbError::programming("reader already consumed"))?;

        let schema = inner.schema_ref()?;
        let pyo3_schema = pyo3_arrow::PySchema::new(schema);
        pyo3_schema.into_pyarrow(py)
    }

    fn __repr__(&self) -> String {
        if self.inner.is_some() {
            "RecordBatchReader(active)".to_string()
        } else {
            "RecordBatchReader(consumed)".to_string()
        }
    }

    /// Export Arrow C Stream via `PyCapsule` protocol.
    ///
    /// This method implements the Arrow `PyCapsule` Interface, allowing zero-copy
    /// data transfer to Arrow-compatible libraries (e.g., Polars, `PyArrow`).
    ///
    /// The stream can only be exported once. After export, the reader is marked
    /// as consumed and cannot be reused.
    ///
    /// # Arguments
    ///
    /// * `requested_schema` - Optional memory address for C schema struct (as per Arrow spec)
    ///
    /// # Returns
    ///
    /// `PyCapsule` object containing the Arrow C Stream pointer
    ///
    /// # Errors
    ///
    /// Returns `PyHdbError::Programming` if reader has already been consumed.
    ///
    /// # Example (Python)
    ///
    /// ```python
    /// import polars as pl
    /// reader = conn.execute_arrow("SELECT * FROM table")
    /// df = pl.from_arrow(reader)  # Uses __arrow_c_stream__ internally
    /// ```
    #[pyo3(signature = (requested_schema=None))]
    fn __arrow_c_stream__<'py>(
        &'py mut self,
        py: Python<'py>,
        requested_schema: Option<usize>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyHdbError::programming("reader already consumed"))?;

        // Convert inner reader to Python object so we can call its __arrow_c_stream__ method
        let py_reader = Bound::new(py, inner)?;

        // Call __arrow_c_stream__ on the inner pyo3_arrow::PyRecordBatchReader
        // The pyo3_arrow reader handles the C interface internally
        let _ = requested_schema;
        py_reader.call_method0("__arrow_c_stream__")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}

    #[test]
    fn test_streaming_reader_is_send() {
        // Compile-time verification that Send is implemented
        assert_send::<StreamingReader>();
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_streaming_reader_is_send() {
        // Compile-time verification for async variant
        assert_send::<AsyncStreamingReader>();
    }
}
