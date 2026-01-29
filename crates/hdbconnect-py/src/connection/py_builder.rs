//! Python-facing connection builders with runtime validation.
//!
//! These builders provide a fluent API for Python users to configure connections
//! with full TLS support. Unlike the internal typestate builders, these use runtime
//! validation since Python doesn't benefit from compile-time state tracking.
//!
//! # Example
//!
//! ```python
//! from pyhdb_rs import ConnectionBuilder, TlsConfig
//!
//! # Simple connection
//! conn = (ConnectionBuilder()
//!     .host("hana.example.com")
//!     .port(30015)
//!     .credentials("SYSTEM", "password")
//!     .build())
//!
//! # TLS with custom certificates
//! conn = (ConnectionBuilder()
//!     .host("hana.example.com")
//!     .credentials("SYSTEM", "password")
//!     .tls(TlsConfig.from_directory("/path/to/certs"))
//!     .build())
//!
//! # From URL with TLS override
//! conn = (ConnectionBuilder.from_url("hdbsql://user:pass@host:30015")
//!     .tls(TlsConfig.from_certificate(cert_pem))
//!     .build())
//! ```

use std::sync::Arc;

use parking_lot::Mutex;
use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::config::PyConnectionConfig;
use crate::connection::wrapper::{ConnectionInner, PyConnection};
use crate::cursor_holdability::PyCursorHoldability;
use crate::error::PyHdbError;
use crate::tls::{PyTlsConfig, TlsConfigInner};
use crate::types::prepared_cache::{DEFAULT_CACHE_CAPACITY, PreparedStatementCache};
use crate::utils::ParsedConnectionUrl;

/// Python-facing connection builder with runtime validation.
///
/// Use this builder when you need:
/// - TLS configuration with custom certificates
/// - Programmatic connection parameters
/// - Fine-grained control over connection settings
///
/// For simple URL-based connections, use `connect()` function instead.
///
/// # Example
///
/// ```python
/// # Simple connection
/// conn = (ConnectionBuilder()
///     .host("hana.example.com")
///     .port(30015)
///     .credentials("SYSTEM", "password")
///     .build())
///
/// # TLS with custom certificates
/// conn = (ConnectionBuilder()
///     .host("hana.example.com")
///     .credentials("SYSTEM", "password")
///     .tls(TlsConfig.from_directory("/path/to/certs"))
///     .build())
///
/// # From URL with TLS override
/// conn = (ConnectionBuilder.from_url("hdbsql://user:pass@host:30015")
///     .tls(TlsConfig.from_certificate(cert_pem))
///     .build())
/// ```
#[pyclass(name = "ConnectionBuilder", module = "pyhdb_rs._core")]
#[derive(Debug, Clone)]
pub struct PyConnectionBuilder {
    host: Option<String>,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    tls_config: Option<PyTlsConfig>,
    connection_config: Option<PyConnectionConfig>,
    cursor_holdability: Option<PyCursorHoldability>,
    network_group: Option<String>,
}

impl Default for PyConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyConnectionBuilder {
    /// Create a new connection builder with default settings.
    ///
    /// Default port is 30015 (SAP HANA standard port).
    #[new]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new() -> Self {
        Self {
            host: None,
            port: 30015,
            user: None,
            password: None,
            database: None,
            tls_config: None,
            connection_config: None,
            cursor_holdability: None,
            network_group: None,
        }
    }

    /// Create builder from a connection URL.
    ///
    /// The URL provides initial values that can be overridden with builder methods.
    /// If the URL scheme is `hdbsqls://`, TLS with system roots is automatically enabled.
    ///
    /// Args:
    ///     url: Connection URL in format `hdbsql://user:pass@host:port[/database]`
    ///
    /// Returns:
    ///     `ConnectionBuilder` initialized with URL values.
    ///
    /// Raises:
    ///     `InterfaceError`: If URL is invalid or missing required components.
    ///
    /// Example:
    ///     ```python
    ///     # Parse URL, then override TLS
    ///     builder = (ConnectionBuilder.from_url("hdbsql://user:pass@host:30015")
    ///         .tls(TlsConfig.from_certificate(cert_pem)))
    ///     ```
    #[classmethod]
    #[pyo3(text_signature = "(cls, url)")]
    fn from_url(_cls: &Bound<'_, PyType>, url: &str) -> PyResult<Self> {
        let parsed = ParsedConnectionUrl::parse(url)?;

        let tls_config = if parsed.use_tls {
            Some(PyTlsConfig {
                inner: TlsConfigInner::RootCertificates,
            })
        } else {
            None
        };

        Ok(Self {
            host: Some(parsed.host),
            port: parsed.port,
            user: Some(parsed.user),
            password: Some(parsed.password),
            database: parsed.database,
            tls_config,
            connection_config: None,
            cursor_holdability: None,
            network_group: None,
        })
    }

    /// Set the database host.
    ///
    /// Args:
    ///     hostname: Database server hostname or IP address.
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, hostname)")]
    fn host<'py>(mut slf: PyRefMut<'py, Self>, hostname: &str) -> PyRefMut<'py, Self> {
        slf.host = Some(hostname.to_string());
        slf
    }

    /// Set the database port.
    ///
    /// Args:
    ///     port: Database port (default: 30015).
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, port)")]
    fn port(mut slf: PyRefMut<'_, Self>, port: u16) -> PyRefMut<'_, Self> {
        slf.port = port;
        slf
    }

    /// Set authentication credentials.
    ///
    /// Args:
    ///     user: Database username.
    ///     password: Database password.
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, user, password)")]
    fn credentials<'py>(
        mut slf: PyRefMut<'py, Self>,
        user: &str,
        password: &str,
    ) -> PyRefMut<'py, Self> {
        slf.user = Some(user.to_string());
        slf.password = Some(password.to_string());
        slf
    }

    /// Set the database/tenant name.
    ///
    /// Args:
    ///     name: Database or tenant name.
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, name)")]
    fn database<'py>(mut slf: PyRefMut<'py, Self>, name: &str) -> PyRefMut<'py, Self> {
        slf.database = Some(name.to_string());
        slf
    }

    /// Configure TLS for secure connection.
    ///
    /// Args:
    ///     config: TLS configuration (use `TlsConfig` factory methods).
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

    /// Apply connection configuration (`fetch_size`, timeouts, etc.).
    ///
    /// Args:
    ///     config: Connection configuration.
    ///
    /// Returns:
    ///     Self for method chaining.
    ///
    /// Example:
    ///     ```python
    ///     config = ConnectionConfig(fetch_size=50000, read_timeout=60.0)
    ///     builder.config(config)
    ///     ```
    #[pyo3(text_signature = "(self, config)")]
    fn config(mut slf: PyRefMut<'_, Self>, config: PyConnectionConfig) -> PyRefMut<'_, Self> {
        slf.connection_config = Some(config);
        slf
    }

    /// Set cursor holdability for transaction behavior.
    ///
    /// Controls whether result set cursors remain open after COMMIT or ROLLBACK.
    ///
    /// Args:
    ///     holdability: Cursor holdability mode (see `CursorHoldability` enum).
    ///
    /// Returns:
    ///     Self for method chaining.
    ///
    /// Example:
    ///     ```python
    ///     from pyhdb_rs import CursorHoldability
    ///     builder.cursor_holdability(CursorHoldability.CommitAndRollback)
    ///     ```
    #[pyo3(text_signature = "(self, holdability)")]
    fn cursor_holdability(
        mut slf: PyRefMut<'_, Self>,
        holdability: PyCursorHoldability,
    ) -> PyRefMut<'_, Self> {
        slf.cursor_holdability = Some(holdability);
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
    ///     # Route to specific network group for HA
    ///     builder.network_group("analytics_group")
    ///     ```
    #[pyo3(text_signature = "(self, group)")]
    fn network_group<'py>(mut slf: PyRefMut<'py, Self>, group: &str) -> PyRefMut<'py, Self> {
        slf.network_group = Some(group.to_string());
        slf
    }

    /// Build and connect synchronously.
    ///
    /// Returns:
    ///     Connection object.
    ///
    /// Raises:
    ///     `InterfaceError`: If required parameters (host, credentials) not set.
    ///     `OperationalError`: If connection fails.
    ///
    /// Example:
    ///     ```python
    ///     conn = (ConnectionBuilder()
    ///         .host("localhost")
    ///         .credentials("user", "pass")
    ///         .build())
    ///     ```
    #[pyo3(text_signature = "(self)")]
    fn build(&self) -> PyResult<PyConnection> {
        let host = self
            .host
            .as_ref()
            .ok_or_else(|| PyHdbError::interface("host not set - call .host() before .build()"))?;
        let user = self.user.as_ref().ok_or_else(|| {
            PyHdbError::interface("credentials not set - call .credentials() before .build()")
        })?;
        let password = self.password.as_ref().ok_or_else(|| {
            PyHdbError::interface("credentials not set - call .credentials() before .build()")
        })?;

        let mut builder = hdbconnect::ConnectParams::builder();
        builder.hostname(host);
        builder.port(self.port);
        builder.dbuser(user);
        builder.password(password);

        if let Some(db) = &self.database {
            builder.dbname(db);
        }

        if let Some(ng) = &self.network_group {
            builder.network_group(ng);
        }

        if let Some(tls) = &self.tls_config {
            tls.apply_to_builder_mut(&mut builder);
        }

        let params = builder
            .build()
            .map_err(|e| PyHdbError::interface(e.to_string()))?;

        let (conn, cache_size) = if let Some(cfg) = &self.connection_config {
            let mut hdb_config = cfg.to_hdbconnect_config();

            if let Some(holdability) = self.cursor_holdability {
                hdb_config.set_cursor_holdability(holdability.into());
            }

            let connection = hdbconnect::Connection::with_configuration(params, &hdb_config)
                .map_err(|e| PyHdbError::operational(e.to_string()))?;
            (connection, cfg.statement_cache_size())
        } else {
            let connection = if let Some(holdability) = self.cursor_holdability {
                let mut hdb_config = hdbconnect::ConnectionConfiguration::default();
                hdb_config.set_cursor_holdability(holdability.into());
                hdbconnect::Connection::with_configuration(params, &hdb_config)
                    .map_err(|e| PyHdbError::operational(e.to_string()))?
            } else {
                hdbconnect::Connection::new(params)
                    .map_err(|e| PyHdbError::operational(e.to_string()))?
            };
            (connection, DEFAULT_CACHE_CAPACITY)
        };

        Ok(PyConnection::from_parts(
            Arc::new(Mutex::new(ConnectionInner::Connected(conn))),
            true,
            Mutex::new(PreparedStatementCache::new(cache_size)),
        ))
    }

    fn __repr__(&self) -> String {
        let host = self.host.as_deref().unwrap_or("<not set>");
        let has_creds = self.user.is_some() && self.password.is_some();
        let tls = self.tls_config.is_some();
        format!(
            "ConnectionBuilder(host={host:?}, port={}, credentials={}, tls={})",
            self.port, has_creds, tls
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// AsyncConnectionBuilder (async feature only)
// ═══════════════════════════════════════════════════════════════════════════════

/// Python-facing async connection builder with runtime validation.
///
/// Same API as `ConnectionBuilder` but produces async connections.
///
/// # Example
///
/// ```python
/// conn = await (AsyncConnectionBuilder()
///     .host("hana.example.com")
///     .credentials("SYSTEM", "password")
///     .tls(TlsConfig.with_system_roots())
///     .build())
/// ```
#[cfg(feature = "async")]
#[pyclass(name = "AsyncConnectionBuilder", module = "pyhdb_rs._core")]
#[derive(Debug, Clone)]
pub struct PyAsyncConnectionBuilder {
    host: Option<String>,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    tls_config: Option<PyTlsConfig>,
    connection_config: Option<PyConnectionConfig>,
    autocommit: bool,
    cursor_holdability: Option<PyCursorHoldability>,
    network_group: Option<String>,
}

#[cfg(feature = "async")]
impl Default for PyAsyncConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
#[pymethods]
impl PyAsyncConnectionBuilder {
    /// Create a new async connection builder with default settings.
    #[new]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new() -> Self {
        Self {
            host: None,
            port: 30015,
            user: None,
            password: None,
            database: None,
            tls_config: None,
            connection_config: None,
            autocommit: true,
            cursor_holdability: None,
            network_group: None,
        }
    }

    /// Create builder from a connection URL.
    #[classmethod]
    #[pyo3(text_signature = "(cls, url)")]
    fn from_url(_cls: &Bound<'_, PyType>, url: &str) -> PyResult<Self> {
        let parsed = ParsedConnectionUrl::parse(url)?;

        let tls_config = if parsed.use_tls {
            Some(PyTlsConfig {
                inner: TlsConfigInner::RootCertificates,
            })
        } else {
            None
        };

        Ok(Self {
            host: Some(parsed.host),
            port: parsed.port,
            user: Some(parsed.user),
            password: Some(parsed.password),
            database: parsed.database,
            tls_config,
            connection_config: None,
            autocommit: true,
            cursor_holdability: None,
            network_group: None,
        })
    }

    /// Set the database host.
    #[pyo3(text_signature = "(self, hostname)")]
    fn host<'py>(mut slf: PyRefMut<'py, Self>, hostname: &str) -> PyRefMut<'py, Self> {
        slf.host = Some(hostname.to_string());
        slf
    }

    /// Set the database port.
    #[pyo3(text_signature = "(self, port)")]
    fn port(mut slf: PyRefMut<'_, Self>, port: u16) -> PyRefMut<'_, Self> {
        slf.port = port;
        slf
    }

    /// Set authentication credentials.
    #[pyo3(text_signature = "(self, user, password)")]
    fn credentials<'py>(
        mut slf: PyRefMut<'py, Self>,
        user: &str,
        password: &str,
    ) -> PyRefMut<'py, Self> {
        slf.user = Some(user.to_string());
        slf.password = Some(password.to_string());
        slf
    }

    /// Set the database/tenant name.
    #[pyo3(text_signature = "(self, name)")]
    fn database<'py>(mut slf: PyRefMut<'py, Self>, name: &str) -> PyRefMut<'py, Self> {
        slf.database = Some(name.to_string());
        slf
    }

    /// Configure TLS for secure connection.
    #[pyo3(text_signature = "(self, config)")]
    fn tls(mut slf: PyRefMut<'_, Self>, config: PyTlsConfig) -> PyRefMut<'_, Self> {
        slf.tls_config = Some(config);
        slf
    }

    /// Apply connection configuration.
    #[pyo3(text_signature = "(self, config)")]
    fn config(mut slf: PyRefMut<'_, Self>, config: PyConnectionConfig) -> PyRefMut<'_, Self> {
        slf.connection_config = Some(config);
        slf
    }

    /// Set auto-commit mode (default: True).
    #[pyo3(text_signature = "(self, enabled)")]
    fn autocommit(mut slf: PyRefMut<'_, Self>, enabled: bool) -> PyRefMut<'_, Self> {
        slf.autocommit = enabled;
        slf
    }

    /// Set cursor holdability for transaction behavior.
    ///
    /// Controls whether result set cursors remain open after COMMIT or ROLLBACK.
    ///
    /// Args:
    ///     holdability: Cursor holdability mode (see `CursorHoldability` enum).
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, holdability)")]
    fn cursor_holdability(
        mut slf: PyRefMut<'_, Self>,
        holdability: PyCursorHoldability,
    ) -> PyRefMut<'_, Self> {
        slf.cursor_holdability = Some(holdability);
        slf
    }

    /// Set the network group for HANA Scale-Out and HA deployments.
    ///
    /// Args:
    ///     group: Network group name configured in HANA.
    ///
    /// Returns:
    ///     Self for method chaining.
    #[pyo3(text_signature = "(self, group)")]
    fn network_group<'py>(mut slf: PyRefMut<'py, Self>, group: &str) -> PyRefMut<'py, Self> {
        slf.network_group = Some(group.to_string());
        slf
    }

    /// Build and connect asynchronously.
    ///
    /// Returns:
    ///     `Awaitable[AsyncConnection]`
    ///
    /// Raises:
    ///     `InterfaceError`: If required parameters not set.
    ///     `OperationalError`: If connection fails.
    #[pyo3(text_signature = "(self)")]
    fn build<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use std::sync::Arc;

        use tokio::sync::Mutex as TokioMutex;

        use crate::async_support::{AsyncConnectionInner, AsyncPyConnection};
        use crate::types::prepared_cache::PreparedStatementCache;
        use crate::utils::apply_tls_to_async_builder;

        let host = self
            .host
            .clone()
            .ok_or_else(|| PyHdbError::interface("host not set - call .host() before .build()"))?;
        let user = self.user.clone().ok_or_else(|| {
            PyHdbError::interface("credentials not set - call .credentials() before .build()")
        })?;
        let password = self.password.clone().ok_or_else(|| {
            PyHdbError::interface("credentials not set - call .credentials() before .build()")
        })?;
        let port = self.port;
        let database = self.database.clone();
        let tls_config = self.tls_config.clone();
        let connection_config = self.connection_config.clone();
        let autocommit = self.autocommit;
        let cursor_holdability = self.cursor_holdability;
        let network_group = self.network_group.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut builder = hdbconnect_async::ConnectParams::builder();
            builder.hostname(&host);
            builder.port(port);
            builder.dbuser(&user);
            builder.password(&password);

            if let Some(db) = &database {
                builder.dbname(db);
            }

            if let Some(ng) = &network_group {
                builder.network_group(ng);
            }

            if let Some(tls) = &tls_config {
                apply_tls_to_async_builder(&tls.inner, &mut builder);
            }

            let params = builder
                .build()
                .map_err(|e| PyHdbError::interface(e.to_string()))?;

            let (connection, cache_size) = if let Some(cfg) = &connection_config {
                let mut hdb_config = cfg.to_hdbconnect_config();

                if let Some(holdability) = cursor_holdability {
                    hdb_config.set_cursor_holdability(holdability.into());
                }

                let conn = hdbconnect_async::Connection::with_configuration(params, &hdb_config)
                    .await
                    .map_err(|e| PyHdbError::operational(e.to_string()))?;
                (conn, cfg.statement_cache_size())
            } else {
                let conn = if let Some(holdability) = cursor_holdability {
                    let mut hdb_config = hdbconnect_async::ConnectionConfiguration::default();
                    hdb_config.set_cursor_holdability(holdability.into());
                    hdbconnect_async::Connection::with_configuration(params, &hdb_config)
                        .await
                        .map_err(|e| PyHdbError::operational(e.to_string()))?
                } else {
                    hdbconnect_async::Connection::new(params)
                        .await
                        .map_err(|e| PyHdbError::operational(e.to_string()))?
                };
                (conn, DEFAULT_CACHE_CAPACITY)
            };

            let inner = Arc::new(TokioMutex::new(AsyncConnectionInner::Connected {
                connection,
            }));

            let statement_cache =
                Arc::new(TokioMutex::new(PreparedStatementCache::new(cache_size)));

            Ok(AsyncPyConnection::from_parts(
                inner,
                autocommit,
                statement_cache,
            ))
        })
    }

    fn __repr__(&self) -> String {
        let host = self.host.as_deref().unwrap_or("<not set>");
        let has_creds = self.user.is_some() && self.password.is_some();
        let tls = self.tls_config.is_some();
        format!(
            "AsyncConnectionBuilder(host={host:?}, port={}, credentials={}, tls={}, autocommit={})",
            self.port, has_creds, tls, self.autocommit
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // PyConnectionBuilder Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_py_builder_new() {
        let builder = PyConnectionBuilder::new();
        assert!(builder.host.is_none());
        assert_eq!(builder.port, 30015);
        assert!(builder.user.is_none());
        assert!(builder.password.is_none());
        assert!(builder.database.is_none());
        assert!(builder.tls_config.is_none());
        assert!(builder.connection_config.is_none());
        assert!(builder.cursor_holdability.is_none());
        assert!(builder.network_group.is_none());
    }

    #[test]
    fn test_py_builder_default() {
        let builder = PyConnectionBuilder::default();
        assert!(builder.host.is_none());
        assert_eq!(builder.port, 30015);
    }

    #[test]
    fn test_py_builder_build_missing_host() {
        let builder = PyConnectionBuilder {
            host: None,
            port: 30015,
            user: Some("user".to_string()),
            password: Some("pass".to_string()),
            database: None,
            tls_config: None,
            connection_config: None,
            cursor_holdability: None,
            network_group: None,
        };

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_py_builder_build_missing_credentials() {
        let builder = PyConnectionBuilder {
            host: Some("localhost".to_string()),
            port: 30015,
            user: None,
            password: None,
            database: None,
            tls_config: None,
            connection_config: None,
            cursor_holdability: None,
            network_group: None,
        };

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_py_builder_clone() {
        let builder = PyConnectionBuilder {
            host: Some("localhost".to_string()),
            port: 30015,
            user: Some("user".to_string()),
            password: Some("pass".to_string()),
            database: Some("mydb".to_string()),
            tls_config: None,
            connection_config: None,
            cursor_holdability: Some(PyCursorHoldability::Commit),
            network_group: Some("test_group".to_string()),
        };

        let cloned = builder.clone();
        assert_eq!(cloned.host, builder.host);
        assert_eq!(cloned.port, builder.port);
        assert_eq!(cloned.user, builder.user);
        assert_eq!(cloned.cursor_holdability, builder.cursor_holdability);
        assert_eq!(cloned.network_group, builder.network_group);
    }

    #[test]
    fn test_py_builder_debug() {
        let builder = PyConnectionBuilder::new();
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("PyConnectionBuilder"));
    }

    #[test]
    fn test_py_builder_repr() {
        let builder = PyConnectionBuilder {
            host: Some("localhost".to_string()),
            port: 30015,
            user: Some("user".to_string()),
            password: Some("pass".to_string()),
            database: None,
            tls_config: None,
            connection_config: None,
            cursor_holdability: None,
            network_group: None,
        };

        let repr = builder.__repr__();
        assert!(repr.contains("ConnectionBuilder"));
        assert!(repr.contains("localhost"));
        assert!(repr.contains("30015"));
        assert!(repr.contains("credentials=true"));
    }

    #[test]
    fn test_py_builder_repr_no_creds() {
        let builder = PyConnectionBuilder::new();
        let repr = builder.__repr__();
        assert!(repr.contains("credentials=false"));
        assert!(repr.contains("<not set>"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PyAsyncConnectionBuilder Tests (async feature only)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(feature = "async")]
    #[test]
    fn test_py_async_builder_new() {
        let builder = PyAsyncConnectionBuilder::new();
        assert!(builder.host.is_none());
        assert_eq!(builder.port, 30015);
        assert!(builder.user.is_none());
        assert!(builder.password.is_none());
        assert!(builder.database.is_none());
        assert!(builder.tls_config.is_none());
        assert!(builder.connection_config.is_none());
        assert!(builder.autocommit);
        assert!(builder.cursor_holdability.is_none());
        assert!(builder.network_group.is_none());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_py_async_builder_default() {
        let builder = PyAsyncConnectionBuilder::default();
        assert!(builder.host.is_none());
        assert_eq!(builder.port, 30015);
        assert!(builder.autocommit);
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_py_async_builder_repr() {
        let builder = PyAsyncConnectionBuilder {
            host: Some("localhost".to_string()),
            port: 30015,
            user: Some("user".to_string()),
            password: Some("pass".to_string()),
            database: None,
            tls_config: None,
            connection_config: None,
            autocommit: false,
            cursor_holdability: None,
            network_group: None,
        };

        let repr = builder.__repr__();
        assert!(repr.contains("AsyncConnectionBuilder"));
        assert!(repr.contains("localhost"));
        assert!(repr.contains("autocommit=false"));
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_py_async_builder_clone() {
        let builder = PyAsyncConnectionBuilder {
            host: Some("localhost".to_string()),
            port: 30015,
            user: Some("user".to_string()),
            password: Some("pass".to_string()),
            database: Some("mydb".to_string()),
            tls_config: None,
            connection_config: None,
            autocommit: false,
            cursor_holdability: Some(PyCursorHoldability::CommitAndRollback),
            network_group: Some("ha_group".to_string()),
        };

        let cloned = builder.clone();
        assert_eq!(cloned.host, builder.host);
        assert_eq!(cloned.autocommit, builder.autocommit);
        assert_eq!(cloned.cursor_holdability, builder.cursor_holdability);
        assert_eq!(cloned.network_group, builder.network_group);
    }
}
