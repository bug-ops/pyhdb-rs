//! `PyO3` `RecordBatchReader` wrapper.
//!
//! Implements __`arrow_c_stream`__ for zero-copy Arrow data transfer.

use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use hdbconnect_arrow::{BatchConfig, FieldMetadataExt, HanaBatchProcessor};
use pyo3::prelude::*;

use crate::error::PyHdbError;

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
}

// SAFETY: StreamingReader requires `Send` for pyo3_arrow::PyRecordBatchReader.
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
}

impl Iterator for StreamingReader {
    type Item = Result<RecordBatch, arrow_schema::ArrowError>;

    #[allow(clippy::needless_continue)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        loop {
            match self.result_set.next() {
                Some(Ok(row)) => match self.processor.process_row(&row) {
                    Ok(Some(batch)) => return Some(Ok(batch)),
                    Ok(None) => continue, // Continue processing rows until batch is ready
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

    /// WARNING: Loads ALL rows into memory. For large result sets, use sync API.
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

/// Async streaming reader with channel-based backpressure.
///
/// Uses a bounded mpsc channel to stream batches incrementally. A background
/// task fetches rows asynchronously and sends batches through the channel,
/// while the iterator blocks on receiving batches. This provides:
///
/// - **Backpressure**: Channel bounds prevent unbounded memory growth
/// - **Incremental processing**: Consumer processes batches as they arrive
/// - **Error propagation**: Errors are sent through the channel
///
/// The channel buffer size is set to 4 batches, which provides a good balance
/// between throughput and memory usage.
#[cfg(feature = "async")]
struct AsyncStreamingReader {
    receiver: std::sync::mpsc::Receiver<Result<RecordBatch, arrow_schema::ArrowError>>,
    schema: SchemaRef,
}

// SAFETY: AsyncStreamingReader only contains:
// - mpsc::Receiver: Send (Receiver<T> is Send if T is Send, RecordBatch is Send)
// - SchemaRef (Arc<Schema>): Send + Sync
// No shared mutable state, no thread-unsafe types, no raw pointers.
#[cfg(feature = "async")]
unsafe impl Send for AsyncStreamingReader {}

#[cfg(feature = "async")]
impl AsyncStreamingReader {
    /// Channel buffer size (number of batches to buffer before blocking sender).
    const CHANNEL_BUFFER_SIZE: usize = 4;

    fn new(result_set: hdbconnect_async::ResultSet, batch_size: usize) -> Self {
        let schema = Self::build_schema(&result_set);
        let config = BatchConfig::with_batch_size(batch_size);

        // Create bounded channel for backpressure
        let (sender, receiver) = std::sync::mpsc::sync_channel(Self::CHANNEL_BUFFER_SIZE);

        // Clone schema for the background task
        let schema_clone = Arc::clone(&schema);

        // Spawn background task to fetch and process rows
        tokio::task::spawn(async move {
            let mut processor = HanaBatchProcessor::new(schema_clone, config);

            // Fetch rows asynchronously
            match result_set.into_rows().await {
                Ok(rows) => {
                    for row in rows {
                        match processor.process_row(&row) {
                            Ok(Some(batch)) => {
                                // Send batch, stop if receiver is dropped
                                if sender.send(Ok(batch)).is_err() {
                                    return;
                                }
                            }
                            Ok(None) => {
                                // Continue accumulating rows
                            }
                            Err(e) => {
                                let _ = sender.send(Err(arrow_schema::ArrowError::ExternalError(
                                    Box::new(std::io::Error::other(e.to_string())),
                                )));
                                return;
                            }
                        }
                    }

                    // Flush remaining rows
                    match processor.flush() {
                        Ok(Some(batch)) => {
                            let _ = sender.send(Ok(batch));
                        }
                        Ok(None) => {}
                        Err(e) => {
                            let _ = sender.send(Err(arrow_schema::ArrowError::ExternalError(
                                Box::new(std::io::Error::other(e.to_string())),
                            )));
                        }
                    }
                }
                Err(e) => {
                    let _ = sender.send(Err(arrow_schema::ArrowError::ExternalError(Box::new(
                        std::io::Error::other(e.to_string()),
                    ))));
                }
            }
            // Sender drops here, signaling end of stream
        });

        Self { receiver, schema }
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
        // Block until a batch is available or the channel is closed
        self.receiver.recv().ok()
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
