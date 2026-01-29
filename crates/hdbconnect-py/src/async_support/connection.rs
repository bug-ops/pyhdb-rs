//! Async connection for Python.
//!
//! # Statement Cache Deprecation
//!
//! The `statement_cache_size` parameter is deprecated and ignored.
//! Statement caching is not available due to hdbconnect API limitations.
//! The parameter will be removed in version 0.3.0.

use std::sync::Arc;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyType;
use tokio::sync::Mutex as TokioMutex;

use super::common::{
    ConnectionState, VALIDATION_QUERY, commit_impl, execute_arrow_impl, rollback_impl,
};
use super::cursor::AsyncPyCursor;
use crate::config::PyConnectionConfig;
use crate::connection::ConnectionBuilder;
use crate::error::PyHdbError;

pub type SharedAsyncConnection = Arc<TokioMutex<AsyncConnectionInner>>;

#[derive(Debug)]
pub enum AsyncConnectionInner {
    Connected {
        connection: hdbconnect_async::Connection,
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
/// import polars as pl
///
/// async with await AsyncConnection.connect("hdbsql://...") as conn:
///     reader = await conn.execute_arrow(
///         "SELECT PRODUCT_NAME, SUM(QUANTITY) AS TOTAL_SOLD FROM SALES_ITEMS WHERE FISCAL_YEAR = 2025 GROUP BY PRODUCT_NAME"
///     )
///     df = pl.from_arrow(reader)
/// ```
#[pyclass(name = "AsyncConnection", module = "hdbconnect.aio")]
#[derive(Debug)]
pub struct AsyncPyConnection {
    inner: SharedAsyncConnection,
    autocommit: bool,
}

impl AsyncPyConnection {
    pub fn shared(&self) -> SharedAsyncConnection {
        Arc::clone(&self.inner)
    }
}

#[pymethods]
impl AsyncPyConnection {
    /// Connect to HANA database asynchronously.
    ///
    /// # Arguments
    ///
    /// * `url` - Connection URL (hdbsql://user:pass@host:port)
    /// * `autocommit` - Enable auto-commit mode (default: True)
    /// * `config` - Optional connection configuration for tuning performance
    /// * `statement_cache_size` - **DEPRECATED**: This parameter is ignored. Statement caching is
    ///   not available due to hdbconnect API limitations. Will be removed in 0.3.0.
    #[classmethod]
    #[pyo3(signature = (url, *, autocommit=true, config=None, statement_cache_size=0))]
    fn connect<'py>(
        _cls: &Bound<'py, PyType>,
        py: Python<'py>,
        url: String,
        autocommit: bool,
        config: Option<PyConnectionConfig>,
        #[allow(unused_variables)] statement_cache_size: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let params = ConnectionBuilder::from_url(&url)?.build()?;

            let connection = match config {
                Some(cfg) => {
                    let hdb_config = cfg.to_hdbconnect_config();
                    hdbconnect_async::Connection::with_configuration(params, &hdb_config)
                        .await
                        .map_err(|e| PyHdbError::operational(e.to_string()))?
                }
                None => hdbconnect_async::Connection::new(params)
                    .await
                    .map_err(|e| PyHdbError::operational(e.to_string()))?,
            };

            let inner = Arc::new(TokioMutex::new(AsyncConnectionInner::Connected {
                connection,
            }));

            Ok(Self { inner, autocommit })
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
                AsyncConnectionInner::Connected { connection, .. } => commit_impl(connection).await,
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    fn rollback<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection, .. } => {
                    rollback_impl(connection).await
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
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

    /// Check if connection is valid.
    ///
    /// Returns an awaitable that resolves to a boolean.
    ///
    /// # Arguments
    ///
    /// * `check_connection` - If True (default), executes `SELECT 1 FROM DUMMY` to verify the
    ///   connection is alive. If False, only checks internal state without network round-trip.
    ///
    /// # Returns
    ///
    /// Awaitable[bool]: True if connection is valid, False otherwise.
    ///
    /// # Example
    ///
    /// ```python
    /// if not await conn.is_valid():
    ///     conn = await connect(uri)  # Reconnect
    /// ```
    #[pyo3(signature = (check_connection=true))]
    fn is_valid<'py>(
        &self,
        py: Python<'py>,
        check_connection: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection } => {
                    if check_connection {
                        Ok(connection.query(VALIDATION_QUERY).await.is_ok())
                    } else {
                        Ok(true)
                    }
                }
                AsyncConnectionInner::Disconnected => Ok(false),
            }
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

    /// Get current fetch size (rows per network round-trip).
    #[getter]
    fn fetch_size<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            match &*guard {
                AsyncConnectionInner::Connected { connection } => {
                    let val = connection.fetch_size().await;
                    Ok(val)
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Set fetch size at runtime (async operation).
    ///
    /// Args:
    ///     value: Number of rows to fetch per network round-trip
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is 0
    ///     `OperationalError`: If connection is closed
    fn set_fetch_size<'py>(&self, py: Python<'py>, value: u32) -> PyResult<Bound<'py, PyAny>> {
        if value == 0 {
            return Err(PyHdbError::programming("fetch_size must be > 0").into());
        }
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection } => {
                    connection.set_fetch_size(value).await;
                    Ok(())
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Get current read timeout in seconds (None = no timeout).
    #[getter]
    fn read_timeout<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            match &*guard {
                AsyncConnectionInner::Connected { connection } => {
                    let timeout: Option<Duration> =
                        connection.read_timeout().await.map_err(PyHdbError::from)?;
                    Ok(timeout.map(|d| d.as_secs_f64()))
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Set read timeout at runtime (async operation).
    ///
    /// Args:
    ///     value: Timeout in seconds, or None to disable
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is negative
    ///     `OperationalError`: If connection is closed
    fn set_read_timeout<'py>(
        &self,
        py: Python<'py>,
        value: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if let Some(v) = value
            && v < 0.0
        {
            return Err(PyHdbError::programming("read_timeout cannot be negative").into());
        }
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection } => {
                    let duration = value.filter(|&v| v > 0.0).map(Duration::from_secs_f64);
                    connection
                        .set_read_timeout(duration)
                        .await
                        .map_err(PyHdbError::from)?;
                    Ok(())
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Get current LOB read length.
    #[getter]
    fn lob_read_length<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            match &*guard {
                AsyncConnectionInner::Connected { connection } => {
                    let val = connection.lob_read_length().await;
                    Ok(val)
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Set LOB read length at runtime (async operation).
    ///
    /// Args:
    ///     value: Bytes per LOB read operation
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is 0
    ///     `OperationalError`: If connection is closed
    fn set_lob_read_length<'py>(&self, py: Python<'py>, value: u32) -> PyResult<Bound<'py, PyAny>> {
        if value == 0 {
            return Err(PyHdbError::programming("lob_read_length must be > 0").into());
        }
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection } => {
                    connection.set_lob_read_length(value).await;
                    Ok(())
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Get current LOB write length.
    #[getter]
    fn lob_write_length<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            match &*guard {
                AsyncConnectionInner::Connected { connection } => {
                    let val = connection.lob_write_length().await;
                    Ok(val)
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Set LOB write length at runtime (async operation).
    ///
    /// Args:
    ///     value: Bytes per LOB write operation
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is 0
    ///     `OperationalError`: If connection is closed
    fn set_lob_write_length<'py>(
        &self,
        py: Python<'py>,
        value: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        if value == 0 {
            return Err(PyHdbError::programming("lob_write_length must be > 0").into());
        }
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            match &mut *guard {
                AsyncConnectionInner::Connected { connection } => {
                    connection.set_lob_write_length(value).await;
                    Ok(())
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Executes a SQL query and returns an Arrow `RecordBatchReader`.
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
                AsyncConnectionInner::Connected { connection } => {
                    let reader = execute_arrow_impl(connection, &sql, batch_size).await?;
                    drop(guard);
                    Ok(reader)
                }
                AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
            }
        })
    }

    /// Returns statement cache statistics.
    ///
    /// # Deprecation
    ///
    /// Always returns None. Statement caching is deprecated and will be
    /// removed in version 0.3.0.
    #[allow(deprecated, clippy::unused_self)]
    fn cache_stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Python::attach(|py| Ok(py.None().into_any()))
        })
    }

    // PyO3 requires &self for Python __aenter__ protocol binding.
    #[allow(clippy::unused_self)]
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
        format!("AsyncConnection(autocommit={})", self.autocommit)
    }
}
