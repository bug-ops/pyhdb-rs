//! Connection pool using [`deadpool`].
//!
//! Provides an async connection pool for SAP HANA with configurable size limits.
//!
//! # Statement Cache
//!
//! Each pooled connection maintains its own prepared statement cache.
//! Configure cache size via `ConnectionConfig(max_cached_statements=N)`.
//!
//! # Note on `min_idle`
//!
//! The [`deadpool`] crate's managed pool does not natively support `min_idle`.
//! The `min_idle` configuration is exposed for API consistency and future
//! implementation. Currently, connections are created on-demand.

// Intentionally omits connection details from Debug output for security/brevity.
#![allow(clippy::missing_fields_in_debug)]

use std::sync::Arc;

use deadpool::managed::{Manager, Metrics, Object, RecycleError, RecycleResult};
use hdbconnect::ConnectionConfiguration;
use pyo3::prelude::*;
use tokio::sync::Mutex as TokioMutex;

use super::common::{
    ConnectionState, VALIDATION_QUERY, commit_impl, execute_arrow_impl, rollback_impl,
};
use crate::config::PyConnectionConfig;
use crate::connection::{ConnectionBuilder, PyCacheStats};
use crate::error::PyHdbError;
use crate::types::prepared_cache::{
    CacheStatistics, DEFAULT_CACHE_CAPACITY, PreparedStatementCache,
};

/// Pool configuration parameters.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool.
    pub max_size: usize,
    /// Minimum number of idle connections to maintain.
    /// Note: Currently not enforced by deadpool; connections are created on-demand.
    pub min_idle: Option<usize>,
    /// Connection acquisition timeout in seconds.
    pub connection_timeout_secs: u64,
    /// Size of the prepared statement cache per connection.
    pub max_cached_statements: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: None,
            connection_timeout_secs: 30,
            max_cached_statements: DEFAULT_CACHE_CAPACITY,
        }
    }
}

/// Wrapper around async HANA connection for pool management.
///
/// This wrapper exists to provide a clean separation between pool management
/// and connection logic, allowing future extensions like connection-level
/// statement caching or connection metadata without modifying the underlying
/// [`hdbconnect_async::Connection`].
pub struct PooledConnectionInner {
    pub connection: hdbconnect_async::Connection,
    pub statement_cache: PreparedStatementCache<hdbconnect_async::PreparedStatement>,
}

impl std::fmt::Debug for PooledConnectionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnectionInner")
            .field("cache_size", &self.statement_cache.len())
            .finish()
    }
}

pub type PooledObject = Object<HanaConnectionManager>;

#[derive(Debug)]
pub struct HanaConnectionManager {
    url: String,
    config: Option<ConnectionConfiguration>,
    cache_size: usize,
}

impl HanaConnectionManager {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            config: None,
            cache_size: DEFAULT_CACHE_CAPACITY,
        }
    }

    pub fn with_config(
        url: impl Into<String>,
        config: ConnectionConfiguration,
        cache_size: usize,
    ) -> Self {
        Self {
            url: url.into(),
            config: Some(config),
            cache_size,
        }
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

        let connection = match &self.config {
            Some(cfg) => hdbconnect_async::Connection::with_configuration(params, cfg).await?,
            None => hdbconnect_async::Connection::new(params).await?,
        };

        Ok(PooledConnectionInner {
            connection,
            statement_cache: PreparedStatementCache::new(self.cache_size),
        })
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _metrics: &Metrics,
    ) -> RecycleResult<Self::Error> {
        conn.connection
            .query(VALIDATION_QUERY)
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
/// import polars as pl
///
/// pool = create_pool("hdbsql://user:pass@host:30015", max_size=10)
/// async with pool.acquire() as conn:
///     reader = await conn.execute_arrow(
///         "SELECT CUSTOMER_ID, COUNT(*) AS ORDER_COUNT FROM SALES_ORDERS WHERE ORDER_DATE >= '2025-01-01' GROUP BY CUSTOMER_ID"
///     )
///     df = pl.from_arrow(reader)
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
    /// Creates a new connection pool.
    ///
    /// # Arguments
    ///
    /// * `url` - HANA connection URL (hdbsql://user:pass@host:port)
    /// * `max_size` - Maximum number of connections (default: 10)
    /// * `min_idle` - Minimum idle connections to maintain (accepted for API compatibility, not
    ///   enforced by underlying pool)
    /// * `connection_timeout` - Connection acquisition timeout in seconds (default: 30)
    /// * `config` - Optional connection configuration applied to all pooled connections
    #[new]
    #[pyo3(signature = (url, *, max_size=10, min_idle=None, connection_timeout=30, config=None))]
    fn new(
        url: String,
        max_size: usize,
        min_idle: Option<usize>,
        connection_timeout: u64,
        config: Option<&PyConnectionConfig>,
    ) -> PyResult<Self> {
        // Validate min_idle doesn't exceed max_size
        if let Some(min) = min_idle
            && min > max_size
        {
            return Err(PyHdbError::programming(format!(
                "min_idle ({min}) cannot exceed max_size ({max_size})"
            ))
            .into());
        }

        let manager = config.map_or_else(
            || HanaConnectionManager::new(&url),
            |cfg| {
                HanaConnectionManager::with_config(
                    &url,
                    cfg.to_hdbconnect_config(),
                    cfg.statement_cache_size(),
                )
            },
        );

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
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let reader = execute_arrow_impl(&mut obj.connection, &sql, batch_size).await?;
            drop(guard);
            Ok(reader)
        })
    }

    fn cursor<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            if guard.is_none() {
                return Err(ConnectionState::ReturnedToPool.into_error().into());
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
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            commit_impl(&mut obj.connection).await
        })
    }

    fn rollback<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            rollback_impl(&mut obj.connection).await
        })
    }

    /// Get current fetch size (rows per network round-trip).
    #[getter]
    fn fetch_size<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            let obj = guard
                .as_ref()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let val = obj.connection.fetch_size().await;
            Ok(val)
        })
    }

    /// Set fetch size at runtime (async operation).
    fn set_fetch_size<'py>(&self, py: Python<'py>, value: u32) -> PyResult<Bound<'py, PyAny>> {
        if value == 0 {
            return Err(PyHdbError::programming("fetch_size must be > 0").into());
        }
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            obj.connection.set_fetch_size(value).await;
            Ok(())
        })
    }

    /// Get current read timeout in seconds (None = no timeout).
    #[getter]
    fn read_timeout<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            let obj = guard
                .as_ref()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let timeout = obj
                .connection
                .read_timeout()
                .await
                .map_err(PyHdbError::from)?;
            Ok(timeout.map(|d: std::time::Duration| d.as_secs_f64()))
        })
    }

    /// Set read timeout at runtime (async operation).
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
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let duration = value
                .filter(|&v| v > 0.0)
                .map(std::time::Duration::from_secs_f64);
            obj.connection
                .set_read_timeout(duration)
                .await
                .map_err(PyHdbError::from)?;
            Ok(())
        })
    }

    /// Get current LOB read length.
    #[getter]
    fn lob_read_length<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            let obj = guard
                .as_ref()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let val = obj.connection.lob_read_length().await;
            Ok(val)
        })
    }

    /// Set LOB read length at runtime (async operation).
    fn set_lob_read_length<'py>(&self, py: Python<'py>, value: u32) -> PyResult<Bound<'py, PyAny>> {
        if value == 0 {
            return Err(PyHdbError::programming("lob_read_length must be > 0").into());
        }
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            obj.connection.set_lob_read_length(value).await;
            Ok(())
        })
    }

    /// Get current LOB write length.
    #[getter]
    fn lob_write_length<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            let obj = guard
                .as_ref()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let val = obj.connection.lob_write_length().await;
            Ok(val)
        })
    }

    /// Set LOB write length at runtime (async operation).
    fn set_lob_write_length<'py>(
        &self,
        py: Python<'py>,
        value: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        if value == 0 {
            return Err(PyHdbError::programming("lob_write_length must be > 0").into());
        }
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            obj.connection.set_lob_write_length(value).await;
            Ok(())
        })
    }

    /// Check if pooled connection is valid.
    ///
    /// Returns an awaitable that resolves to a boolean.
    ///
    /// # Arguments
    ///
    /// * `check_connection` - If True (default), executes `SELECT 1 FROM DUMMY` to verify the
    ///   connection is alive. If False, only checks if connection is still held (not returned to
    ///   pool).
    ///
    /// # Returns
    ///
    /// Awaitable[bool]: True if connection is valid, False otherwise.
    ///
    /// # Example
    ///
    /// ```python
    /// async with pool.acquire() as conn:
    ///     if not await conn.is_valid():
    ///         # Connection invalid, handle error
    ///         pass
    /// ```
    #[pyo3(signature = (check_connection=true))]
    fn is_valid<'py>(
        &self,
        py: Python<'py>,
        check_connection: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            match guard.as_mut() {
                Some(obj) => {
                    if check_connection {
                        Ok(obj.connection.query(VALIDATION_QUERY).await.is_ok())
                    } else {
                        Ok(true)
                    }
                }
                None => Ok(false), // Returned to pool
            }
        })
    }

    /// Get prepared statement cache statistics.
    ///
    /// Returns an awaitable that resolves to `CacheStats`.
    fn cache_stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let guard = object.lock().await;
            let obj = guard
                .as_ref()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let stats: CacheStatistics = obj.statement_cache.stats();
            Ok(PyCacheStats::from(stats))
        })
    }

    /// Clear the prepared statement cache.
    ///
    /// Returns an awaitable that completes when the cache is cleared.
    fn clear_cache<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let object = Arc::clone(&self.object);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = object.lock().await;
            let obj = guard
                .as_mut()
                .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;

            let _ = obj.statement_cache.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_size, 10);
        assert_eq!(config.min_idle, None);
        assert_eq!(config.connection_timeout_secs, 30);
        assert_eq!(config.max_cached_statements, DEFAULT_CACHE_CAPACITY);
    }

    #[test]
    fn test_pool_config_clone() {
        let config = PoolConfig {
            max_size: 20,
            min_idle: Some(5),
            connection_timeout_secs: 60,
            max_cached_statements: 32,
        };

        let cloned = config.clone();
        assert_eq!(cloned.max_size, 20);
        assert_eq!(cloned.min_idle, Some(5));
        assert_eq!(cloned.connection_timeout_secs, 60);
        assert_eq!(cloned.max_cached_statements, 32);
    }

    #[test]
    fn test_pool_config_debug() {
        let config = PoolConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("PoolConfig"));
        assert!(debug_str.contains("max_size"));
    }

    #[test]
    fn test_hana_connection_manager_new() {
        let manager = HanaConnectionManager::new("hdbsql://user:pass@host:30015");
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("HanaConnectionManager"));
    }

    #[test]
    fn test_hana_connection_manager_with_config() {
        let config = ConnectionConfiguration::default();
        let manager =
            HanaConnectionManager::with_config("hdbsql://user:pass@host:30015", config, 32);
        assert!(manager.config.is_some());
        assert_eq!(manager.cache_size, 32);
    }

    #[test]
    fn test_pool_status_repr() {
        let status = PoolStatus {
            size: 5,
            available: 3,
            max_size: 10,
        };

        let repr = status.__repr__();
        assert!(repr.contains("size=5"));
        assert!(repr.contains("available=3"));
        assert!(repr.contains("max_size=10"));
    }

    #[test]
    fn test_pool_status_clone() {
        let status = PoolStatus {
            size: 5,
            available: 3,
            max_size: 10,
        };

        let cloned = status.clone();
        assert_eq!(cloned.size, 5);
        assert_eq!(cloned.available, 3);
        assert_eq!(cloned.max_size, 10);
    }

    #[test]
    fn test_pool_status_debug() {
        let status = PoolStatus {
            size: 1,
            available: 1,
            max_size: 5,
        };

        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("PoolStatus"));
    }
}
