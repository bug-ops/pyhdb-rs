//! Async connection for Python.
//!
//! # Statement Cache
//!
//! The async connection includes a prepared statement cache for improved
//! performance on repeated parameterized queries. Configure via
//! `ConnectionConfig(max_cached_statements=N)`.

use std::sync::Arc;
use std::time::Duration;

use pyo3::prelude::*;
use tokio::sync::Mutex as TokioMutex;

use super::common::{
    ConnectionState, VALIDATION_QUERY, commit_impl, execute_arrow_impl, get_batch_size,
    rollback_impl, validate_non_negative_f64, validate_positive_u32,
};
use super::cursor::AsyncPyCursor;
use crate::config::PyArrowConfig;
use crate::connection::PyCacheStats;
use crate::error::PyHdbError;
use crate::types::prepared_cache::{CacheStatistics, PreparedStatementCache};

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
/// from pyhdb_rs.aio import AsyncConnectionBuilder
///
/// async with await AsyncConnectionBuilder.from_url("hdbsql://...").build() as conn:
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
    statement_cache: Arc<TokioMutex<PreparedStatementCache<hdbconnect_async::PreparedStatement>>>,
}

impl AsyncPyConnection {
    /// Create an `AsyncPyConnection` from pre-built components.
    ///
    /// Used by `PyAsyncConnectionBuilder` to construct connections without going
    /// through URL parsing.
    pub fn from_parts(
        inner: SharedAsyncConnection,
        autocommit: bool,
        statement_cache: Arc<
            TokioMutex<PreparedStatementCache<hdbconnect_async::PreparedStatement>>,
        >,
    ) -> Self {
        Self {
            inner,
            autocommit,
            statement_cache,
        }
    }

    pub fn shared(&self) -> SharedAsyncConnection {
        Arc::clone(&self.inner)
    }

    pub fn statement_cache(
        &self,
    ) -> Arc<TokioMutex<PreparedStatementCache<hdbconnect_async::PreparedStatement>>> {
        Arc::clone(&self.statement_cache)
    }
}

#[pymethods]
impl AsyncPyConnection {
    fn cursor(&self) -> AsyncPyCursor {
        AsyncPyCursor::new(Arc::clone(&self.inner))
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        let cache = Arc::clone(&self.statement_cache);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut cache_guard = cache.lock().await;
            let evicted = cache_guard.clear();
            drop(evicted);
            drop(cache_guard);

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
        validate_positive_u32(value, "fetch_size")?;
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
        validate_non_negative_f64(value, "read_timeout")?;
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
        validate_positive_u32(value, "lob_read_length")?;
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
        validate_positive_u32(value, "lob_write_length")?;
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
    ///
    /// Args:
    ///     sql: SQL query string
    ///     config: Optional Arrow configuration (`batch_size`, etc.)
    ///
    /// Returns:
    ///     `RecordBatchReader` for streaming results
    ///
    /// Example:
    ///     ```python
    ///     from pyhdb_rs import ArrowConfig
    ///     import polars as pl
    ///
    ///     # With default config
    ///     reader = await conn.execute_arrow("SELECT * FROM T")
    ///     df = pl.from_arrow(reader)
    ///
    ///     # With custom batch size
    ///     config = ArrowConfig(batch_size=10000)
    ///     reader = await conn.execute_arrow("SELECT * FROM T", config=config)
    ///     ```
    #[pyo3(signature = (sql, config=None))]
    fn execute_arrow<'py>(
        &self,
        py: Python<'py>,
        sql: String,
        config: Option<&PyArrowConfig>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let batch_size = get_batch_size(config);
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

    /// Get prepared statement cache statistics.
    ///
    /// Returns an awaitable that resolves to `CacheStats`.
    ///
    /// Example:
    ///     ```python
    ///     stats = await conn.cache_stats()
    ///     print(f"Cache hit rate: {stats.hit_rate:.2%}")
    ///     ```
    fn cache_stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let cache = Arc::clone(&self.statement_cache);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let cache_guard = cache.lock().await;
            let stats: CacheStatistics = cache_guard.stats();
            Ok(PyCacheStats::from(stats))
        })
    }

    /// Clear the prepared statement cache.
    ///
    /// Returns an awaitable that completes when the cache is cleared.
    fn clear_cache<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let cache = Arc::clone(&self.statement_cache);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut cache_guard = cache.lock().await;
            let evicted = cache_guard.clear();
            drop(evicted);
            Ok(())
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
        let cache = Arc::clone(&self.statement_cache);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut cache_guard = cache.lock().await;
            let evicted = cache_guard.clear();
            drop(evicted);
            drop(cache_guard);

            let mut guard = inner.lock().await;
            *guard = AsyncConnectionInner::Disconnected;
            Ok(false)
        })
    }

    fn __repr__(&self) -> String {
        format!("AsyncConnection(autocommit={})", self.autocommit)
    }
}
