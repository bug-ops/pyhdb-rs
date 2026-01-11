//! `PyO3` `RecordBatchReader` wrapper.
//!
//! Implements __`arrow_c_stream`__ for zero-copy Arrow data transfer.

use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use pyo3::prelude::*;

use crate::error::PyHdbError;
use hdbconnect_arrow::{BatchConfig, FieldMetadataExt, HanaBatchProcessor};

/// Streams Arrow `RecordBatches` from HANA result set.
/// Implements `__arrow_c_stream__` for zero-copy transfer.
#[pyclass(name = "RecordBatchReader", module = "hdbconnect")]
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

// SAFETY: StreamingReader is only used within a single Python thread context.
// The hdbconnect::ResultSet is not shared across threads; it is created and
// consumed within the same connection context. The Send impl is required by
// pyo3_arrow::PyRecordBatchReader but we ensure thread isolation through:
// 1. Python GIL protection on the PyRecordBatchReader wrapper
// 2. Mutex<ConnectionInner> guards all connection state in parent
// 3. ResultSet iteration is single-threaded by design (no concurrent access)
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

/// Async streaming reader - loads ALL data into memory.
///
/// WARNING: This is NOT true streaming. For large result sets, use sync API.
///
/// TODO(PERF-001): Implement true async streaming using `tokio::sync::mpsc`
/// channel to stream batches incrementally with backpressure support.
#[cfg(feature = "async")]
struct AsyncStreamingReader {
    batches: std::vec::IntoIter<RecordBatch>,
    schema: SchemaRef,
}

// SAFETY: AsyncStreamingReader only contains:
// - Vec<RecordBatch>::IntoIter: RecordBatch is Send + Sync (contains Arc<ArrayData>)
// - SchemaRef (Arc<Schema>): Send + Sync
// No shared mutable state, no thread-unsafe types, no raw pointers.
#[cfg(feature = "async")]
unsafe impl Send for AsyncStreamingReader {}

#[cfg(feature = "async")]
impl AsyncStreamingReader {
    fn new(result_set: hdbconnect_async::ResultSet, batch_size: usize) -> Self {
        let schema = Self::build_schema(&result_set);
        let config = BatchConfig::with_batch_size(batch_size);
        let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
        let mut batches = Vec::new();

        // Block to fetch all rows - hdbconnect_async::ResultSet.into_rows() is async
        let rows_result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(result_set.into_rows())
        });

        if let Ok(rows) = rows_result {
            for row in rows {
                if let Ok(Some(batch)) = processor.process_row(&row) {
                    batches.push(batch);
                }
            }
        }

        if let Ok(Some(batch)) = processor.flush() {
            batches.push(batch);
        }

        Self {
            batches: batches.into_iter(),
            schema,
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
        self.batches.next().map(Ok)
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
}
