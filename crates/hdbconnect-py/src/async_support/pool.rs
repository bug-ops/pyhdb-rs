//! Connection pool using `deadpool`.
//!
//! TODO: Add min_idle configuration support
#![allow(
    clippy::doc_markdown,
    clippy::missing_fields_in_debug,
    clippy::significant_drop_tightening
)]

use std::sync::Arc;

use deadpool::managed::{Manager, Metrics, Object, RecycleError, RecycleResult};
use pyo3::prelude::*;
use tokio::sync::Mutex as TokioMutex;

use crate::connection::ConnectionBuilder;
use crate::error::PyHdbError;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: usize,
    pub min_idle: Option<usize>,
    pub connection_timeout_secs: u64,
    pub statement_cache_size: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: None,
            connection_timeout_secs: 30,
            statement_cache_size: 0,
        }
    }
}

// TODO(REFACTOR-001): Consider using Connection directly instead of this wrapper
pub struct PooledConnectionInner {
    pub connection: hdbconnect_async::Connection,
}

impl std::fmt::Debug for PooledConnectionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnectionInner").finish()
    }
}

pub type PooledObject = Object<HanaConnectionManager>;

#[derive(Debug)]
pub struct HanaConnectionManager {
    url: String,
}

impl HanaConnectionManager {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl Manager for HanaConnectionManager {
    type Type = PooledConnectionInner;
    type Error = hdbconnect_async::HdbError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let params = ConnectionBuilder::from_url(&self.url)
            .map_err(|e| hdbconnect_async::HdbError::from(std::io::Error::other(e.to_string())))?
            .build()
            .map_err(|e| hdbconnect_async::HdbError::from(std::io::Error::other(e.to_string())))?;

        let connection = hdbconnect_async::Connection::new(params).await?;
        Ok(PooledConnectionInner { connection })
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _metrics: &Metrics,
    ) -> RecycleResult<Self::Error> {
        conn.connection
            .query("SELECT 1 FROM DUMMY")
            .await
            .map_err(RecycleError::Backend)?;
        Ok(())
    }
}

pub type Pool = deadpool::managed::Pool<HanaConnectionManager>;

/// Python connection pool.
///
/// # Example
///
/// ```python
/// pool = create_pool("hdbsql://user:pass@host:30015", max_size=10)
/// async with pool.acquire() as conn:
///     df = await conn.execute_polars("SELECT * FROM sales")
/// ```
#[pyclass(name = "ConnectionPool", module = "hdbconnect.aio")]
pub struct PyConnectionPool {
    pool: Pool,
    url: String,
}

impl std::fmt::Debug for PyConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyConnectionPool")
            .field("url", &self.url)
            .field("max_size", &self.pool.status().max_size)
            .finish()
    }
}

#[pymethods]
impl PyConnectionPool {
    #[new]
    #[pyo3(signature = (url, *, max_size=10, connection_timeout=30))]
    fn new(url: String, max_size: usize, connection_timeout: u64) -> PyResult<Self> {
        let manager = HanaConnectionManager::new(&url);

        let pool = Pool::builder(manager)
            .max_size(max_size)
            .wait_timeout(Some(std::time::Duration::from_secs(connection_timeout)))
            .build()
            .map_err(|e| PyHdbError::operational(e.to_string()))?;

        Ok(Self { pool, url })
    }

    /// Acquire a connection from the pool.
    fn acquire<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pool = self.pool.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let obj = pool
                .get()
                .await
                .map_err(|e| PyHdbError::operational(e.to_string()))?;

            Ok(PooledConnection::new(obj))
        })
    }

    #[getter]
    fn status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            size: status.size,
            available: status.available,
            max_size: status.max_size,
        }
    }

    #[getter]
    fn max_size(&self) -> usize {
        self.pool.status().max_size
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pool = self.pool.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            pool.close();
            Ok(())
        })
    }

    fn __repr__(&self) -> String {
        let status = self.pool.status();
        format!(
            "ConnectionPool(size={}, available={}, max_size={})",
            status.size, status.available, status.max_size
        )
    }
}

#[pyclass(name = "PoolStatus", module = "hdbconnect.aio")]
#[derive(Debug, Clone)]
pub struct PoolStatus {
    #[pyo3(get)]
    pub size: usize,
    #[pyo3(get)]
    pub available: usize,
    #[pyo3(get)]
    pub max_size: usize,
}

#[pymethods]
impl PoolStatus {
    fn __repr__(&self) -> String {
        format!(
            "PoolStatus(size={}, available={}, max_size={})",
            self.size, self.available, self.max_size
        )
    }
}

/// A connection borrowed from the pool.
///
/// Automatically returns to the pool when dropped via deadpool's RAII mechanism.
#[pyclass(name = "PooledConnection", module = "hdbconnect.aio")]
pub struct PooledConnection {
    // Wrapped in Arc<TokioMutex> for thread-safe async access. None = returned to pool.
    object: Arc<TokioMutex<Option<PooledObject>>>,
}

impl PooledConnection {
    pub fn new(obj: PooledObject) -> Self {
        Self {
            object: Arc::new(TokioMutex::new(Some(obj))),
        }
    }
}

impl std::fmt::Debug for PooledConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection").finish_non_exhaustive()
    }
}

#[pymethods]
impl PooledConnection {
    #[pyo3(signature = (sql, batch_size=65536))]
    fn execute_arrow<'py>(
        &self,
        py: Python<'py>,
        sql: String,
        batch_size: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| PyHdbError::operational("connection returned to pool"))?;

            let rs = obj.connection.query(&sql).await.map_err(PyHdbError::from)?;
            drop(guard);
            crate::reader::PyRecordBatchReader::from_resultset_async(rs, batch_size)
        })
    }

    #[pyo3(signature = (sql))]
    fn execute_polars<'py>(&self, py: Python<'py>, sql: String) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| PyHdbError::operational("connection returned to pool"))?;

            let rs = obj.connection.query(&sql).await.map_err(PyHdbError::from)?;
            drop(guard);
            let reader = crate::reader::PyRecordBatchReader::from_resultset_async(rs, 65536)?;

            Python::with_gil(|py| {
                let polars = py.import("polars")?;
                let df = polars.call_method1("from_arrow", (reader,))?;
                Ok(df.unbind())
            })
        })
    }

    fn cursor<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            if guard.is_none() {
                return Err(PyHdbError::operational("connection returned to pool").into());
            }
            Ok(super::cursor::AsyncPyCursor::from_pooled(Arc::clone(
                &object,
            )))
        })
    }

    fn commit<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| PyHdbError::operational("connection returned to pool"))?;

            obj.connection.commit().await.map_err(PyHdbError::from)?;
            Ok(())
        })
    }

    fn rollback<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| PyHdbError::operational("connection returned to pool"))?;

            obj.connection.rollback().await.map_err(PyHdbError::from)?;
            Ok(())
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
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let _ = guard.take();
            Ok(false)
        })
    }

    fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            if guard.is_some() {
                Ok("PooledConnection(active)".to_string())
            } else {
                Ok("PooledConnection(returned)".to_string())
            }
        })
    }
}
