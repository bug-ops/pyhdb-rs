//! `PyO3` Connection wrapper for Python.
//!
//! Provides thread-safe connection sharing via `Arc<Mutex>`.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use pyo3::prelude::*;

use crate::config::PyConnectionConfig;
use crate::cursor::PyCursor;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;

/// Lightweight validation query for connection health checks.
///
/// SAP HANA's `DUMMY` table is equivalent to Oracle's `DUAL` - a special
/// single-row, single-column table designed for this purpose.
const VALIDATION_QUERY: &str = "SELECT 1 FROM DUMMY";

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
}

impl PyConnection {
    /// Create a connection with custom configuration.
    pub fn with_config(url: &str, config: &PyConnectionConfig) -> PyResult<Self> {
        let params = crate::connection::ConnectionBuilder::from_url(url)?.build()?;
        let hdb_config = config.to_hdbconnect_config();

        let conn = hdbconnect::Connection::with_configuration(params, &hdb_config)
            .map_err(|e| PyHdbError::operational(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(ConnectionInner::Connected(conn))),
            autocommit: true,
        })
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
        if value == 0 {
            return Err(PyHdbError::programming("fetch_size must be > 0").into());
        }
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
        if let Some(v) = value
            && v < 0.0
        {
            return Err(PyHdbError::programming("read_timeout cannot be negative").into());
        }
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
        if value == 0 {
            return Err(PyHdbError::programming("lob_read_length must be > 0").into());
        }
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
        if value == 0 {
            return Err(PyHdbError::programming("lob_write_length must be > 0").into());
        }
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
    ///     `batch_size`: Rows per batch (default: 65536)
    ///
    /// Returns:
    ///     `RecordBatchReader` for streaming results
    #[pyo3(signature = (sql, batch_size=65536))]
    fn execute_arrow(&self, sql: &str, batch_size: usize) -> PyResult<PyRecordBatchReader> {
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
        let state = if self.is_connected() {
            "connected"
        } else {
            "closed"
        };
        format!("Connection(state={state}, autocommit={})", self.autocommit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_query_constant() {
        assert_eq!(VALIDATION_QUERY, "SELECT 1 FROM DUMMY");
    }

    #[test]
    fn test_connection_inner_disconnected() {
        let inner = ConnectionInner::Disconnected;
        assert!(matches!(inner, ConnectionInner::Disconnected));
    }
}
