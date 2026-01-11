//! `PyO3` Connection wrapper for Python.
//!
//! Provides thread-safe connection sharing via Arc<Mutex>.

use std::sync::Arc;

use parking_lot::Mutex;
use pyo3::prelude::*;

use crate::cursor::PyCursor;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;

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
#[pyclass(name = "Connection", module = "hdbconnect")]
#[derive(Debug)]
pub struct PyConnection {
    /// Shared connection for thread safety.
    inner: SharedConnection,
    /// Auto-commit mode.
    autocommit: bool,
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

    /// Execute a query and return Polars `DataFrame`.
    ///
    /// Requires polars to be installed.
    ///
    /// Args:
    ///     sql: SQL query string
    ///
    /// Returns:
    ///     Polars `DataFrame`
    #[pyo3(signature = (sql))]
    fn execute_polars<'py>(&self, py: Python<'py>, sql: &str) -> PyResult<Bound<'py, PyAny>> {
        let reader = self.execute_arrow(sql, 65536)?;

        // Import polars and use from_arrow
        let polars = py.import("polars")?;
        polars.call_method1("from_arrow", (reader,))
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
