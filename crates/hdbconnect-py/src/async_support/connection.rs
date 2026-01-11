//! Async connection for Python.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyType;
use tokio::sync::Mutex as TokioMutex;

use super::cursor::AsyncPyCursor;
use super::statement_cache::PreparedStatementCache;
use crate::connection::ConnectionBuilder;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;

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

    /// Executes a SQL query and returns an Arrow `RecordBatchReader`.
    ///
    /// If statement caching is enabled, tracks query statistics.
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
                AsyncConnectionInner::Connected {
                    connection,
                    statement_cache,
                } => {
                    // Track query in cache if enabled
                    if let Some(cache) = statement_cache {
                        cache.get_or_insert(&sql, || {});
                    }

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

    /// Executes a SQL query and returns a Polars `DataFrame`.
    ///
    /// If statement caching is enabled, tracks query statistics.
    #[pyo3(signature = (sql))]
    fn execute_polars<'py>(&self, py: Python<'py>, sql: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected {
                    connection,
                    statement_cache,
                } => {
                    // Track query in cache if enabled
                    if let Some(cache) = statement_cache {
                        cache.get_or_insert(&sql, || {});
                    }

                    let rs = connection.query(&sql).await.map_err(PyHdbError::from)?;
                    drop(guard);
                    let reader = PyRecordBatchReader::from_resultset_async(rs, 65536)?;

                    Python::attach(|py| {
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

    /// Returns statement cache statistics if caching is enabled.
    fn cache_stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            match &*guard {
                AsyncConnectionInner::Connected {
                    statement_cache, ..
                } => statement_cache.as_ref().map_or_else(
                    || Python::attach(|py| Ok(py.None().into_any())),
                    |cache| {
                        let stats = cache.stats();
                        Python::attach(|py| {
                            let dict = pyo3::types::PyDict::new(py);
                            dict.set_item("hits", stats.hits)?;
                            dict.set_item("misses", stats.misses)?;
                            dict.set_item("hit_rate", stats.hit_rate)?;
                            dict.set_item("size", stats.size)?;
                            dict.set_item("capacity", stats.capacity)?;
                            Ok(dict.unbind().into_any())
                        })
                    },
                ),
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
