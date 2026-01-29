//! Connection pool using [`deadpool`].
//!
//! Provides an async connection pool for SAP HANA with configurable size limits
//! and TLS support.
//!
//! # Statement Cache
//!
//! Each pooled connection maintains its own prepared statement cache.
//! Configure cache size via `ConnectionConfig(max_cached_statements=N)`.
//!
//! # TLS Configuration
//!
//! Connection pools support TLS via the `tls_config` parameter or the builder API:
//!
//! ```python
//! # Direct construction
//! pool = ConnectionPool(
//!     "hdbsql://user:pass@host:30015",
//!     max_size=10,
//!     tls_config=TlsConfig.from_directory("/path/to/certs")
//! )
//!
//! # Builder API
//! pool = (ConnectionPoolBuilder()
//!     .url("hdbsql://user:pass@host:30015")
//!     .max_size(10)
//!     .tls(TlsConfig.with_system_roots())
//!     .build())
//! ```
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
    ConnectionState, VALIDATION_QUERY, commit_impl, execute_arrow_impl, get_batch_size,
    rollback_impl, validate_non_negative_f64, validate_positive_u32,
};
use crate::config::{PyArrowConfig, PyConnectionConfig};
use crate::connection::PyCacheStats;
use crate::error::PyHdbError;
use crate::tls::{PyTlsConfig, TlsConfigInner};
use crate::types::prepared_cache::{
    CacheStatistics, DEFAULT_CACHE_CAPACITY, PreparedStatementCache,
};
use crate::utils::{ParsedConnectionUrl, apply_tls_to_async_builder};

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
    tls_config: Option<TlsConfigInner>,
    network_group: Option<String>,
}

impl HanaConnectionManager {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            config: None,
            cache_size: DEFAULT_CACHE_CAPACITY,
            tls_config: None,
            network_group: None,
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
            tls_config: None,
            network_group: None,
        }
    }

    pub(crate) fn with_tls(
        url: impl Into<String>,
        config: Option<ConnectionConfiguration>,
        cache_size: usize,
        tls_config: TlsConfigInner,
    ) -> Self {
        Self {
            url: url.into(),
            config,
            cache_size,
            tls_config: Some(tls_config),
            network_group: None,
        }
    }

    pub(crate) fn with_network_group(mut self, network_group: String) -> Self {
        self.network_group = Some(network_group);
        self
    }
}

impl Manager for HanaConnectionManager {
    type Type = PooledConnectionInner;
    type Error = hdbconnect_async::HdbError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let parsed = ParsedConnectionUrl::parse(&self.url)
            .map_err(|e| hdbconnect_async::HdbError::from(std::io::Error::other(e.to_string())))?;

        let mut builder = hdbconnect_async::ConnectParams::builder();
        builder.hostname(&parsed.host);
        builder.port(parsed.port);
        builder.dbuser(&parsed.user);
        builder.password(&parsed.password);

        if let Some(db) = &parsed.database {
            builder.dbname(db);
        }

        if let Some(ng) = &self.network_group {
            builder.network_group(ng);
        }

        // Apply TLS from explicit config, or from URL scheme
        if let Some(tls) = &self.tls_config {
            apply_tls_to_async_builder(tls, &mut builder);
        } else if parsed.use_tls {
            builder.tls_with(hdbconnect_async::ServerCerts::RootCertificates);
        }

        let params = builder
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
/// Use `ConnectionPoolBuilder` to create instances.
///
/// # Example
///
/// ```python
/// import polars as pl
/// from pyhdb_rs.aio import ConnectionPoolBuilder
///
/// pool = (ConnectionPoolBuilder()
///     .url("hdbsql://user:pass@host:30015")
///     .max_size(10)
///     .build())
///
/// async with pool.acquire() as conn:
///     reader = await conn.execute_arrow(
///         "SELECT CUSTOMER_ID, COUNT(*) FROM SALES_ORDERS"
///     )
///     df = pl.from_arrow(reader)
/// ```
///
/// # TLS Example
///
/// ```python
/// from pyhdb_rs import TlsConfig
/// from pyhdb_rs.aio import ConnectionPoolBuilder
///
/// pool = (ConnectionPoolBuilder()
///     .url("hdbsql://user:pass@host:30015")
///     .max_size(10)
///     .tls(TlsConfig.from_directory("/etc/hana/certs"))
///     .build())
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

// ═══════════════════════════════════════════════════════════════════════════════
// ConnectionPoolBuilder
// ═══════════════════════════════════════════════════════════════════════════════

/// Builder for creating connection pools with fluent API.
///
/// # Example
///
/// ```python
/// from pyhdb_rs import TlsConfig
/// from pyhdb_rs.aio import ConnectionPoolBuilder
///
/// # Minimal configuration
/// pool = (ConnectionPoolBuilder()
///     .url("hdbsql://user:pass@host:30015")
///     .build())
///
/// # Full configuration
/// pool = (ConnectionPoolBuilder()
///     .url("hdbsql://user:pass@host:30015")
///     .max_size(20)
///     .min_idle(5)
///     .connection_timeout(60)
///     .tls(TlsConfig.with_system_roots())
///     .config(ConnectionConfig(fetch_size=50000))
///     .network_group("analytics_group")
///     .build())
/// ```
#[pyclass(name = "ConnectionPoolBuilder", module = "hdbconnect.aio")]
#[derive(Debug, Clone, Default)]
pub struct PyConnectionPoolBuilder {
    url: Option<String>,
    max_size: usize,
    min_idle: Option<usize>,
    connection_timeout: u64,
    config: Option<PyConnectionConfig>,
    tls_config: Option<PyTlsConfig>,
    network_group: Option<String>,
}

#[pymethods]
impl PyConnectionPoolBuilder {
    /// Create a new pool builder with default settings.
    ///
    /// Defaults:
    /// - `max_size`: 10
    /// - `min_idle`: None
    /// - `connection_timeout`: 30 seconds
    #[new]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new() -> Self {
        Self {
            url: None,
            max_size: 10,
            min_idle: None,
            connection_timeout: 30,
            config: None,
            tls_config: None,
            network_group: None,
        }
    }

    /// Set the connection URL.
    ///
    /// Args:
    ///     url: Connection URL (`hdbsql://user:pass@host:port`)
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, url)")]
    fn url<'py>(mut slf: PyRefMut<'py, Self>, url: &str) -> PyRefMut<'py, Self> {
        slf.url = Some(url.to_string());
        slf
    }

    /// Set maximum pool size.
    ///
    /// Args:
    ///     size: Maximum number of connections (default: 10)
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, size)")]
    fn max_size(mut slf: PyRefMut<'_, Self>, size: usize) -> PyRefMut<'_, Self> {
        slf.max_size = size;
        slf
    }

    /// Set minimum idle connections.
    ///
    /// Note: Currently not enforced by the underlying pool implementation.
    /// Exposed for API consistency and future support.
    ///
    /// Args:
    ///     size: Minimum idle connections to maintain
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, size)")]
    fn min_idle(mut slf: PyRefMut<'_, Self>, size: usize) -> PyRefMut<'_, Self> {
        slf.min_idle = Some(size);
        slf
    }

    /// Set connection acquisition timeout.
    ///
    /// Args:
    ///     seconds: Timeout in seconds (default: 30)
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, seconds)")]
    fn connection_timeout(mut slf: PyRefMut<'_, Self>, seconds: u64) -> PyRefMut<'_, Self> {
        slf.connection_timeout = seconds;
        slf
    }

    /// Configure TLS for secure connections.
    ///
    /// Args:
    ///     config: TLS configuration (use `TlsConfig` factory methods)
    ///
    /// Returns:
    ///     Self for method chaining.
    ///
    /// Example:
    ///     ```python
    ///     builder.tls(TlsConfig.from_directory("/path/to/certs"))
    ///     builder.tls(TlsConfig.with_system_roots())
    ///     builder.tls(TlsConfig.insecure())  # Development only!
    ///     ```
    #[pyo3(text_signature = "(self, config)")]
    fn tls(mut slf: PyRefMut<'_, Self>, config: PyTlsConfig) -> PyRefMut<'_, Self> {
        slf.tls_config = Some(config);
        slf
    }

    /// Apply connection configuration to all pooled connections.
    ///
    /// Args:
    ///     config: Connection configuration (`fetch_size`, timeouts, etc.)
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, config)")]
    fn config(mut slf: PyRefMut<'_, Self>, config: PyConnectionConfig) -> PyRefMut<'_, Self> {
        slf.config = Some(config);
        slf
    }

    /// Set the network group for HANA Scale-Out and HA deployments.
    ///
    /// Network groups allow routing connections to specific HANA nodes in
    /// scale-out or high-availability configurations.
    ///
    /// Args:
    ///     group: Network group name configured in HANA.
    ///
    /// Returns:
    ///     Self for method chaining.
    ///
    /// Example:
    ///     ```python
    ///     # Route pool connections to specific network group
    ///     builder.network_group("analytics_group")
    ///     ```
    #[pyo3(text_signature = "(self, group)")]
    fn network_group<'py>(mut slf: PyRefMut<'py, Self>, group: &str) -> PyRefMut<'py, Self> {
        slf.network_group = Some(group.to_string());
        slf
    }

    /// Build the connection pool.
    ///
    /// Returns:
    ///     `ConnectionPool`
    ///
    /// Raises:
    ///     `InterfaceError`: If URL not set or invalid
    ///     `ProgrammingError`: If `min_idle` > `max_size`
    #[pyo3(text_signature = "(self)")]
    fn build(&self) -> PyResult<PyConnectionPool> {
        let url = self
            .url
            .clone()
            .ok_or_else(|| PyHdbError::interface("url not set - call .url() before .build()"))?;

        // Validate min_idle doesn't exceed max_size
        if let Some(min) = self.min_idle
            && min > self.max_size
        {
            return Err(PyHdbError::programming(format!(
                "min_idle ({min}) cannot exceed max_size ({})",
                self.max_size
            ))
            .into());
        }

        let mut manager = match (&self.config, &self.tls_config) {
            (None, None) => HanaConnectionManager::new(&url),
            (Some(cfg), None) => HanaConnectionManager::with_config(
                &url,
                cfg.to_hdbconnect_config(),
                cfg.statement_cache_size(),
            ),
            (None, Some(tls)) => HanaConnectionManager::with_tls(
                &url,
                None,
                DEFAULT_CACHE_CAPACITY,
                tls.inner.clone(),
            ),
            (Some(cfg), Some(tls)) => HanaConnectionManager::with_tls(
                &url,
                Some(cfg.to_hdbconnect_config()),
                cfg.statement_cache_size(),
                tls.inner.clone(),
            ),
        };

        if let Some(ng) = &self.network_group {
            manager = manager.with_network_group(ng.clone());
        }

        let pool = Pool::builder(manager)
            .max_size(self.max_size)
            .wait_timeout(Some(std::time::Duration::from_secs(
                self.connection_timeout,
            )))
            .build()
            .map_err(|e| PyHdbError::operational(e.to_string()))?;

        Ok(PyConnectionPool { pool, url })
    }

    fn __repr__(&self) -> String {
        let url = self.url.as_deref().unwrap_or("<not set>");
        format!(
            "ConnectionPoolBuilder(url={url:?}, max_size={}, tls={})",
            self.max_size,
            self.tls_config.is_some()
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
    // Wrapped in `Arc<TokioMutex>` for thread-safe async access. `None` = returned to pool.
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
    ///     async with pool.acquire() as conn:
    ///         reader = await conn.execute_arrow("SELECT * FROM T")
    ///         df = pl.from_arrow(reader)
    ///
    ///     # With custom batch size
    ///     config = ArrowConfig(batch_size=10000)
    ///     async with pool.acquire() as conn:
    ///         reader = await conn.execute_arrow("SELECT * FROM T", config=config)
    ///     ```
    #[pyo3(signature = (sql, config=None))]
    fn execute_arrow<'py>(
        &self,
        py: Python<'py>,
        sql: String,
        config: Option<&PyArrowConfig>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let batch_size = get_batch_size(config);
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
        validate_positive_u32(value, "fetch_size")?;
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

    /// Get current read timeout in seconds (`None` = no timeout).
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
        validate_non_negative_f64(value, "read_timeout")?;
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
        validate_positive_u32(value, "lob_read_length")?;
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
        validate_positive_u32(value, "lob_write_length")?;
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
    /// `Awaitable[bool]`: True if connection is valid, False otherwise.
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
                Some(obj) if check_connection => {
                    Ok(obj.connection.query(VALIDATION_QUERY).await.is_ok())
                }
                Some(_) => Ok(true),
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

            let evicted = obj.statement_cache.clear();
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
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("PoolConfig"));
        assert!(debug_str.contains("max_size"));
    }

    #[test]
    fn test_hana_connection_manager_new() {
        let manager = HanaConnectionManager::new("hdbsql://user:pass@host:30015");
        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("HanaConnectionManager"));
        assert!(manager.tls_config.is_none());
        assert!(manager.network_group.is_none());
    }

    #[test]
    fn test_hana_connection_manager_with_config() {
        let config = ConnectionConfiguration::default();
        let manager =
            HanaConnectionManager::with_config("hdbsql://user:pass@host:30015", config, 32);
        assert!(manager.config.is_some());
        assert_eq!(manager.cache_size, 32);
        assert!(manager.tls_config.is_none());
    }

    #[test]
    fn test_hana_connection_manager_with_tls() {
        let manager = HanaConnectionManager::with_tls(
            "hdbsql://user:pass@host:30015",
            None,
            DEFAULT_CACHE_CAPACITY,
            TlsConfigInner::RootCertificates,
        );
        assert!(manager.tls_config.is_some());
        assert!(matches!(
            manager.tls_config,
            Some(TlsConfigInner::RootCertificates)
        ));
    }

    #[test]
    fn test_hana_connection_manager_with_tls_and_config() {
        let config = ConnectionConfiguration::default();
        let manager = HanaConnectionManager::with_tls(
            "hdbsql://user:pass@host:30015",
            Some(config),
            64,
            TlsConfigInner::Directory("/path/to/certs".to_string()),
        );
        assert!(manager.config.is_some());
        assert!(manager.tls_config.is_some());
        assert_eq!(manager.cache_size, 64);
    }

    #[test]
    fn test_hana_connection_manager_with_network_group() {
        let manager = HanaConnectionManager::new("hdbsql://user:pass@host:30015")
            .with_network_group("analytics_group".to_string());
        assert_eq!(manager.network_group, Some("analytics_group".to_string()));
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

        let debug_str = format!("{status:?}");
        assert!(debug_str.contains("PoolStatus"));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // ConnectionPoolBuilder Tests
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_pool_builder_new() {
        let builder = PyConnectionPoolBuilder::new();
        assert!(builder.url.is_none());
        assert_eq!(builder.max_size, 10);
        assert!(builder.min_idle.is_none());
        assert_eq!(builder.connection_timeout, 30);
        assert!(builder.config.is_none());
        assert!(builder.tls_config.is_none());
        assert!(builder.network_group.is_none());
    }

    #[test]
    fn test_pool_builder_default() {
        let builder = PyConnectionPoolBuilder::default();
        assert!(builder.url.is_none());
        assert_eq!(builder.max_size, 10);
    }

    #[test]
    fn test_pool_builder_build_missing_url() {
        let builder = PyConnectionPoolBuilder::new();
        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_builder_build_min_idle_exceeds_max_size() {
        let builder = PyConnectionPoolBuilder {
            url: Some("hdbsql://user:pass@host:30015".to_string()),
            max_size: 5,
            min_idle: Some(10),
            connection_timeout: 30,
            config: None,
            tls_config: None,
            network_group: None,
        };

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_builder_with_tls_config() {
        let builder = PyConnectionPoolBuilder {
            url: Some("hdbsql://user:pass@host:30015".to_string()),
            max_size: 10,
            min_idle: None,
            connection_timeout: 30,
            config: None,
            tls_config: Some(PyTlsConfig {
                inner: TlsConfigInner::RootCertificates,
            }),
            network_group: None,
        };

        assert!(builder.tls_config.is_some());
    }

    #[test]
    fn test_pool_builder_with_network_group() {
        let builder = PyConnectionPoolBuilder {
            url: Some("hdbsql://user:pass@host:30015".to_string()),
            max_size: 10,
            min_idle: None,
            connection_timeout: 30,
            config: None,
            tls_config: None,
            network_group: Some("ha_group".to_string()),
        };

        assert_eq!(builder.network_group, Some("ha_group".to_string()));
    }

    #[test]
    fn test_pool_builder_clone() {
        let builder = PyConnectionPoolBuilder {
            url: Some("hdbsql://user:pass@host:30015".to_string()),
            max_size: 20,
            min_idle: Some(5),
            connection_timeout: 60,
            config: None,
            tls_config: None,
            network_group: Some("test_group".to_string()),
        };

        let cloned = builder.clone();
        assert_eq!(cloned.url, builder.url);
        assert_eq!(cloned.max_size, 20);
        assert_eq!(cloned.min_idle, Some(5));
        assert_eq!(cloned.connection_timeout, 60);
        assert_eq!(cloned.network_group, Some("test_group".to_string()));
    }

    #[test]
    fn test_pool_builder_debug() {
        let builder = PyConnectionPoolBuilder::new();
        let debug_str = format!("{builder:?}");
        assert!(debug_str.contains("PyConnectionPoolBuilder"));
    }

    #[test]
    fn test_pool_builder_repr() {
        let builder = PyConnectionPoolBuilder {
            url: Some("hdbsql://user:pass@host:30015".to_string()),
            max_size: 10,
            min_idle: None,
            connection_timeout: 30,
            config: None,
            tls_config: None,
            network_group: None,
        };

        let repr = builder.__repr__();
        assert!(repr.contains("ConnectionPoolBuilder"));
        assert!(repr.contains("max_size=10"));
        assert!(repr.contains("tls=false"));
    }

    #[test]
    fn test_pool_builder_repr_no_url() {
        let builder = PyConnectionPoolBuilder::new();
        let repr = builder.__repr__();
        assert!(repr.contains("<not set>"));
    }

    #[test]
    fn test_pool_builder_repr_with_tls() {
        let builder = PyConnectionPoolBuilder {
            url: Some("hdbsql://user:pass@host:30015".to_string()),
            max_size: 10,
            min_idle: None,
            connection_timeout: 30,
            config: None,
            tls_config: Some(PyTlsConfig {
                inner: TlsConfigInner::RootCertificates,
            }),
            network_group: None,
        };

        let repr = builder.__repr__();
        assert!(repr.contains("tls=true"));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // TLS Configuration Tests
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_apply_tls_directory() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(
            &TlsConfigInner::Directory("/path/to/certs".to_string()),
            &mut builder,
        );
        // Builder is modified in place, no error means success
    }

    #[test]
    fn test_apply_tls_environment() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(
            &TlsConfigInner::Environment("HANA_CA_CERT".to_string()),
            &mut builder,
        );
    }

    #[test]
    fn test_apply_tls_direct() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(
            &TlsConfigInner::Direct(
                "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----".to_string(),
            ),
            &mut builder,
        );
    }

    #[test]
    fn test_apply_tls_root_certificates() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(&TlsConfigInner::RootCertificates, &mut builder);
    }

    #[test]
    fn test_apply_tls_insecure() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(&TlsConfigInner::Insecure, &mut builder);
    }
}
