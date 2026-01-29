//! `PyO3` Connection wrapper for Python.
//!
//! Provides thread-safe connection sharing via `Arc<Mutex>`.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use pyo3::prelude::*;

use crate::config::{DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig, PyConnectionConfig};
use crate::cursor::PyCursor;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;
use crate::types::prepared_cache::{
    CacheStatistics, DEFAULT_CACHE_CAPACITY, PreparedStatementCache,
};
use crate::utils::{VALIDATION_QUERY, validate_non_negative_f64, validate_positive_u32};

/// Shared connection type for thread-safe access.
pub type SharedConnection = Arc<Mutex<ConnectionInner>>;

/// Internal connection state.
#[derive(Debug)]
pub enum ConnectionInner {
    /// Active connection.
    Connected(hdbconnect::Connection),
    /// Disconnected state.
    Disconnected,
}

/// Python-exposed cache statistics.
#[pyclass(name = "CacheStats", module = "pyhdb_rs._core", frozen)]
#[derive(Debug, Clone)]
pub struct PyCacheStats {
    /// Current number of cached statements.
    #[pyo3(get)]
    pub size: usize,
    /// Maximum cache capacity.
    #[pyo3(get)]
    pub capacity: usize,
    /// Total number of cache hits.
    #[pyo3(get)]
    pub hits: u64,
    /// Total number of cache misses.
    #[pyo3(get)]
    pub misses: u64,
    /// Total number of evictions.
    #[pyo3(get)]
    pub evictions: u64,
    /// Cache hit rate (0.0 - 1.0).
    #[pyo3(get)]
    pub hit_rate: f64,
}

impl From<CacheStatistics> for PyCacheStats {
    fn from(stats: CacheStatistics) -> Self {
        Self {
            size: stats.size,
            capacity: stats.capacity,
            hits: stats.hits,
            misses: stats.misses,
            evictions: stats.evictions,
            hit_rate: stats.hit_rate,
        }
    }
}

#[pymethods]
impl PyCacheStats {
    fn __repr__(&self) -> String {
        format!(
            "CacheStats(size={}, capacity={}, hits={}, misses={}, evictions={}, hit_rate={:.3})",
            self.size, self.capacity, self.hits, self.misses, self.evictions, self.hit_rate
        )
    }
}

/// Python Connection class.
///
/// DB-API 2.0 compliant connection object.
///
/// # Example
///
/// ```python
/// import hdbconnect
///
/// conn = hdbconnect.connect("hdbsql://user:pass@host:30015")
/// cursor = conn.cursor()
/// cursor.execute("SELECT * FROM DUMMY")
/// result = cursor.fetchone()
/// conn.close()
/// ```
#[pyclass(name = "Connection", module = "pyhdb_rs._core")]
#[derive(Debug)]
pub struct PyConnection {
    /// Shared connection for thread safety.
    inner: SharedConnection,
    /// Auto-commit mode.
    autocommit: bool,
    /// Prepared statement cache.
    stmt_cache: Mutex<PreparedStatementCache<hdbconnect::PreparedStatement>>,
}

impl PyConnection {
    /// Create a `PyConnection` from pre-built components.
    ///
    /// Used by `PyConnectionBuilder` to construct connections without going
    /// through URL parsing.
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_parts(
        inner: SharedConnection,
        autocommit: bool,
        stmt_cache: Mutex<PreparedStatementCache<hdbconnect::PreparedStatement>>,
    ) -> Self {
        Self {
            inner,
            autocommit,
            stmt_cache,
        }
    }

    /// Create a connection with custom configuration.
    pub fn with_config(url: &str, config: &PyConnectionConfig) -> PyResult<Self> {
        let params = crate::connection::ConnectionBuilder::from_url(url)?.build()?;
        let hdb_config = config.to_hdbconnect_config();

        let conn = hdbconnect::Connection::with_configuration(params, &hdb_config)
            .map_err(|e| PyHdbError::operational(e.to_string()))?;

        let cache_size = config.statement_cache_size();

        Ok(Self {
            inner: Arc::new(Mutex::new(ConnectionInner::Connected(conn))),
            autocommit: true,
            stmt_cache: Mutex::new(PreparedStatementCache::new(cache_size)),
        })
    }

    /// Get the shared connection reference.
    pub fn shared(&self) -> SharedConnection {
        Arc::clone(&self.inner)
    }

    /// Get the statement cache reference (for cursor integration).
    pub const fn statement_cache(
        &self,
    ) -> &Mutex<PreparedStatementCache<hdbconnect::PreparedStatement>> {
        &self.stmt_cache
    }
}

#[pymethods]
impl PyConnection {
    /// Create a new connection from URL.
    ///
    /// Args:
    ///     url: Connection URL (hdbsql://user:pass@host:port[/database])
    ///
    /// Returns:
    ///     New connection object
    ///
    /// Raises:
    ///     `InterfaceError`: If URL is invalid
    ///     `OperationalError`: If connection fails
    #[new]
    #[pyo3(signature = (url))]
    pub fn new(url: &str) -> PyResult<Self> {
        let params = crate::connection::ConnectionBuilder::from_url(url)?.build()?;
        let conn = hdbconnect::Connection::new(params)
            .map_err(|e| PyHdbError::operational(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(ConnectionInner::Connected(conn))),
            autocommit: true,
            stmt_cache: Mutex::new(PreparedStatementCache::new(DEFAULT_CACHE_CAPACITY)),
        })
    }

    /// Create a new cursor.
    ///
    /// Returns:
    ///     New cursor object
    fn cursor(&self) -> PyCursor {
        PyCursor::new(Arc::clone(&self.inner))
    }

    /// Close the connection.
    fn close(&self) {
        let mut cache = self.stmt_cache.lock();
        let evicted = cache.clear();
        drop(evicted);
        drop(cache);

        *self.inner.lock() = ConnectionInner::Disconnected;
    }

    /// Commit the current transaction.
    fn commit(&self) -> PyResult<()> {
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                conn.commit().map_err(PyHdbError::from)?;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Rollback the current transaction.
    fn rollback(&self) -> PyResult<()> {
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                conn.rollback().map_err(PyHdbError::from)?;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Check if connection is open.
    #[getter]
    fn is_connected(&self) -> bool {
        matches!(*self.inner.lock(), ConnectionInner::Connected(_))
    }

    /// Check if connection is valid.
    ///
    /// # Arguments
    ///
    /// * `check_connection` - If True (default), executes `SELECT 1 FROM DUMMY` to verify the
    ///   connection is alive. If False, only checks internal state without network round-trip.
    ///
    /// # Returns
    ///
    /// True if connection is valid, False otherwise.
    ///
    /// # Example
    ///
    /// ```python
    /// if not conn.is_valid():
    ///     conn = pyhdb_rs.connect(uri)  # Reconnect
    /// ```
    #[pyo3(signature = (check_connection=true))]
    fn is_valid(&self, check_connection: bool) -> bool {
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                if check_connection {
                    conn.query(VALIDATION_QUERY).is_ok()
                } else {
                    true
                }
            }
            ConnectionInner::Disconnected => false,
        }
    }

    /// Get/set autocommit mode.
    #[getter]
    const fn autocommit(&self) -> bool {
        self.autocommit
    }

    #[setter]
    fn set_autocommit(&mut self, value: bool) -> PyResult<()> {
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                conn.set_auto_commit(value).map_err(PyHdbError::from)?;
                drop(guard);
                self.autocommit = value;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Get current fetch size (rows per network round-trip).
    #[getter]
    fn fetch_size(&self) -> PyResult<u32> {
        let guard = self.inner.lock();
        match &*guard {
            ConnectionInner::Connected(conn) => Ok(conn.fetch_size().map_err(PyHdbError::from)?),
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Set fetch size at runtime.
    ///
    /// Args:
    ///     value: Number of rows to fetch per network round-trip
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is 0
    ///     `OperationalError`: If connection is closed
    #[setter]
    fn set_fetch_size(&self, value: u32) -> PyResult<()> {
        validate_positive_u32(value, "fetch_size")?;
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                conn.set_fetch_size(value).map_err(PyHdbError::from)?;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Get current read timeout in seconds (None = no timeout).
    #[getter]
    fn read_timeout(&self) -> PyResult<Option<f64>> {
        let guard = self.inner.lock();
        match &*guard {
            ConnectionInner::Connected(conn) => {
                let timeout: Option<Duration> = conn.read_timeout().map_err(PyHdbError::from)?;
                Ok(timeout.map(|d| d.as_secs_f64()))
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Set read timeout at runtime.
    ///
    /// Args:
    ///     value: Timeout in seconds, or None to disable
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is negative
    ///     `OperationalError`: If connection is closed
    #[setter]
    fn set_read_timeout(&self, value: Option<f64>) -> PyResult<()> {
        validate_non_negative_f64(value, "read_timeout")?;
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                let duration = value.filter(|&v| v > 0.0).map(Duration::from_secs_f64);
                conn.set_read_timeout(duration).map_err(PyHdbError::from)?;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Get current LOB read length.
    #[getter]
    fn lob_read_length(&self) -> PyResult<u32> {
        let guard = self.inner.lock();
        match &*guard {
            ConnectionInner::Connected(conn) => {
                Ok(conn.lob_read_length().map_err(PyHdbError::from)?)
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Set LOB read length at runtime.
    ///
    /// Args:
    ///     value: Bytes per LOB read operation
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is 0
    ///     `OperationalError`: If connection is closed
    #[setter]
    fn set_lob_read_length(&self, value: u32) -> PyResult<()> {
        validate_positive_u32(value, "lob_read_length")?;
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                conn.set_lob_read_length(value).map_err(PyHdbError::from)?;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Get current LOB write length.
    #[getter]
    fn lob_write_length(&self) -> PyResult<u32> {
        let guard = self.inner.lock();
        match &*guard {
            ConnectionInner::Connected(conn) => {
                Ok(conn.lob_write_length().map_err(PyHdbError::from)?)
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Set LOB write length at runtime.
    ///
    /// Args:
    ///     value: Bytes per LOB write operation
    ///
    /// Raises:
    ///     `ProgrammingError`: If value is 0
    ///     `OperationalError`: If connection is closed
    #[setter]
    fn set_lob_write_length(&self, value: u32) -> PyResult<()> {
        validate_positive_u32(value, "lob_write_length")?;
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                conn.set_lob_write_length(value).map_err(PyHdbError::from)?;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Execute a query and return Arrow `RecordBatchReader`.
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
    ///     reader = conn.execute_arrow("SELECT * FROM T")
    ///     df = pl.from_arrow(reader)
    ///
    ///     # With custom batch size
    ///     config = ArrowConfig(batch_size=10000)
    ///     reader = conn.execute_arrow("SELECT * FROM T", config=config)
    ///     ```
    #[pyo3(signature = (sql, config=None))]
    fn execute_arrow(
        &self,
        sql: &str,
        config: Option<&PyArrowConfig>,
    ) -> PyResult<PyRecordBatchReader> {
        let batch_size = config.map_or(DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig::batch_size);
        let mut guard = self.inner.lock();
        match &mut *guard {
            ConnectionInner::Connected(conn) => {
                let rs = conn.query(sql).map_err(PyHdbError::from)?;
                drop(guard);
                PyRecordBatchReader::from_resultset(rs, batch_size)
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Get prepared statement cache statistics.
    ///
    /// Returns:
    ///     `CacheStats` with size, capacity, hits, misses, evictions, `hit_rate`
    ///
    /// Example:
    ///     ```python
    ///     stats = conn.cache_stats()
    ///     print(f"Cache hit rate: {stats.hit_rate:.2%}")
    ///     print(f"Size: {stats.size}/{stats.capacity}")
    ///     ```
    fn cache_stats(&self) -> PyCacheStats {
        let cache = self.stmt_cache.lock();
        cache.stats().into()
    }

    /// Clear the prepared statement cache.
    ///
    /// Drops all cached prepared statements. Useful after schema changes
    /// or to free server resources.
    fn clear_cache(&self) {
        let evicted = self.stmt_cache.lock().clear();
        drop(evicted);
    }

    // Context manager protocol
    const fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }

    fn __repr__(&self) -> String {
        let conn_state = if self.is_connected() {
            "connected"
        } else {
            "closed"
        };
        let cache_stats = self.stmt_cache.lock().stats();
        format!(
            "Connection(state={conn_state}, autocommit={}, cache_size={}/{})",
            self.autocommit, cache_stats.size, cache_stats.capacity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_inner_disconnected() {
        let inner = ConnectionInner::Disconnected;
        assert!(matches!(inner, ConnectionInner::Disconnected));
    }

    #[test]
    fn test_py_cache_stats_from_cache_statistics() {
        let stats = CacheStatistics {
            size: 5,
            capacity: 16,
            hits: 100,
            misses: 20,
            evictions: 3,
            hit_rate: 0.833,
        };

        let py_stats: PyCacheStats = stats.into();
        assert_eq!(py_stats.size, 5);
        assert_eq!(py_stats.capacity, 16);
        assert_eq!(py_stats.hits, 100);
        assert_eq!(py_stats.misses, 20);
        assert_eq!(py_stats.evictions, 3);
        assert!((py_stats.hit_rate - 0.833).abs() < 0.001);
    }

    #[test]
    fn test_py_cache_stats_repr() {
        let stats = PyCacheStats {
            size: 5,
            capacity: 16,
            hits: 100,
            misses: 20,
            evictions: 3,
            hit_rate: 0.833,
        };

        let repr = stats.__repr__();
        assert!(repr.contains("CacheStats"));
        assert!(repr.contains("size=5"));
        assert!(repr.contains("capacity=16"));
        assert!(repr.contains("hits=100"));
        assert!(repr.contains("misses=20"));
        assert!(repr.contains("evictions=3"));
        assert!(repr.contains("hit_rate=0.833"));
    }

    #[test]
    fn test_py_cache_stats_clone() {
        let stats = PyCacheStats {
            size: 5,
            capacity: 16,
            hits: 100,
            misses: 20,
            evictions: 3,
            hit_rate: 0.833,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.size, stats.size);
        assert_eq!(cloned.capacity, stats.capacity);
    }

    #[test]
    fn test_from_parts() {
        let inner = Arc::new(Mutex::new(ConnectionInner::Disconnected));
        let cache = Mutex::new(PreparedStatementCache::new(16));
        let conn = PyConnection::from_parts(inner, false, cache);

        assert!(!conn.autocommit);
        assert!(!conn.is_connected());
    }
}
