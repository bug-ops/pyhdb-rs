//! Async connection for Python.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyType;
use tokio::sync::Mutex as TokioMutex;

use crate::connection::ConnectionBuilder;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;

use super::cursor::AsyncPyCursor;
use super::statement_cache::PreparedStatementCache;

pub type SharedAsyncConnection = Arc<TokioMutex<AsyncConnectionInner>>;

#[derive(Debug)]
pub enum AsyncConnectionInner {
    Connected {
        connection: hdbconnect_async::Connection,
        statement_cache: Option<PreparedStatementCache>,
    },
    Disconnected,
}

impl AsyncConnectionInner {
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
}

/// Async Python Connection class.
///
/// # Example
///
/// ```python
/// async with await AsyncConnection.connect("hdbsql://...") as conn:
///     df = await conn.execute_polars("SELECT * FROM sales")
/// ```
#[pyclass(name = "AsyncConnection", module = "hdbconnect.aio")]
#[derive(Debug)]
pub struct AsyncPyConnection {
    inner: SharedAsyncConnection,
    autocommit: bool,
    cache_capacity: usize,
}

impl AsyncPyConnection {
    pub fn shared(&self) -> SharedAsyncConnection {
        Arc::clone(&self.inner)
    }
}

#[pymethods]
impl AsyncPyConnection {
    #[classmethod]
    #[pyo3(signature = (url, *, autocommit=true, statement_cache_size=0))]
    fn connect<'py>(
        _cls: &Bound<'py, PyType>,
        py: Python<'py>,
        url: String,
        autocommit: bool,
        statement_cache_size: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let params = ConnectionBuilder::from_url(&url)?.build()?;

            let connection = hdbconnect_async::Connection::new(params)
                .await
                .map_err(|e| PyHdbError::operational(e.to_string()))?;

            let statement_cache = if statement_cache_size > 0 {
                Some(PreparedStatementCache::new(statement_cache_size))
            } else {
                None
            };

            let inner = Arc::new(TokioMutex::new(AsyncConnectionInner::Connected {
                connection,
                statement_cache,
            }));

            Ok(Self {
                inner,
                autocommit,
                cache_capacity: statement_cache_size,
            })
        })
    }

    fn cursor(&self) -> AsyncPyCursor {
        AsyncPyCursor::new(Arc::clone(&self.inner))
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            *guard = AsyncConnectionInner::Disconnected;
            Ok(())
        })
    }

    fn commit<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection, .. } => {
                    connection.commit().await.map_err(PyHdbError::from)?;
                    Ok(())
                }
                AsyncConnectionInner::Disconnected => {
                    Err(PyHdbError::operational("connection is closed").into())
                }
            }
        })
    }

    fn rollback<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection, .. } => {
                    connection.rollback().await.map_err(PyHdbError::from)?;
                    Ok(())
                }
                AsyncConnectionInner::Disconnected => {
                    Err(PyHdbError::operational("connection is closed").into())
                }
            }
        })
    }

    #[getter]
    fn is_connected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            Ok(guard.is_connected())
        })
    }

    #[getter]
    const fn autocommit(&self) -> bool {
        self.autocommit
    }

    #[setter]
    fn set_autocommit(&mut self, value: bool) -> PyResult<()> {
        self.autocommit = value;
        Ok(())
    }

    // TODO(PERF-004): Integrate PreparedStatementCache with query execution
    #[pyo3(signature = (sql, batch_size=65536))]
    fn execute_arrow<'py>(
        &self,
        py: Python<'py>,
        sql: String,
        batch_size: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection, .. } => {
                    let rs = connection.query(&sql).await.map_err(PyHdbError::from)?;
                    drop(guard);
                    PyRecordBatchReader::from_resultset_async(rs, batch_size)
                }
                AsyncConnectionInner::Disconnected => {
                    Err(PyHdbError::operational("connection is closed").into())
                }
            }
        })
    }

    #[pyo3(signature = (sql))]
    fn execute_polars<'py>(&self, py: Python<'py>, sql: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection, .. } => {
                    let rs = connection.query(&sql).await.map_err(PyHdbError::from)?;
                    drop(guard);
                    let reader = PyRecordBatchReader::from_resultset_async(rs, 65536)?;

                    Python::with_gil(|py| {
                        let polars = py.import("polars")?;
                        let df = polars.call_method1("from_arrow", (reader,))?;
                        Ok(df.unbind())
                    })
                }
                AsyncConnectionInner::Disconnected => {
                    Err(PyHdbError::operational("connection is closed").into())
                }
            }
        })
    }

    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Option<&Bound<'py, PyAny>>,
        _exc_val: Option<&Bound<'py, PyAny>>,
        _exc_tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            *guard = AsyncConnectionInner::Disconnected;
            Ok(false)
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncConnection(autocommit={}, cache_capacity={})",
            self.autocommit, self.cache_capacity
        )
    }
}
