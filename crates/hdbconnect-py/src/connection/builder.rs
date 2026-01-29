//! Type-safe connection builders with phantom types.
//!
//! Provides both sync and async connection builders that enforce at compile-time
//! that host and credentials are set before building.
//!
//! # Typestate Pattern
//!
//! The builders use phantom types to track whether required parameters have been set:
//! - `MissingHost` / `HasHost` - tracks if host is configured
//! - `MissingCredentials` / `HasCredentials` - tracks if credentials are configured
//!
//! The `build()` and `connect()` methods are only available when both host and credentials
//! are set, preventing invalid connection attempts at compile time.

use std::marker::PhantomData;

#[cfg(feature = "async")]
use hdbconnect::ConnectionConfiguration;

use crate::error::PyHdbError;
use crate::private::sealed::Sealed;
use crate::utils::ParsedConnectionUrl;

/// Marker trait for builder states.
pub trait BuilderState: Sealed {}

/// Missing host state.
#[derive(Debug, Default)]
pub struct MissingHost;
impl Sealed for MissingHost {}
impl BuilderState for MissingHost {}

/// Has host state.
#[derive(Debug, Default)]
pub struct HasHost;
impl Sealed for HasHost {}
impl BuilderState for HasHost {}

/// Missing credentials state.
#[derive(Debug, Default)]
pub struct MissingCredentials;
impl Sealed for MissingCredentials {}
impl BuilderState for MissingCredentials {}

/// Has credentials state.
#[derive(Debug, Default)]
pub struct HasCredentials;
impl Sealed for HasCredentials {}
impl BuilderState for HasCredentials {}

/// Type-safe connection builder.
///
/// Uses phantom types to enforce that host and credentials are set.
#[derive(Debug)]
pub struct ConnectionBuilder<H: BuilderState, C: BuilderState> {
    host: Option<String>,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    tls: bool,
    _host_state: PhantomData<H>,
    _cred_state: PhantomData<C>,
}

impl Default for ConnectionBuilder<MissingHost, MissingCredentials> {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionBuilder<MissingHost, MissingCredentials> {
    /// Create a new connection builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            host: None,
            port: 30015,
            user: None,
            password: None,
            database: None,
            tls: false,
            _host_state: PhantomData,
            _cred_state: PhantomData,
        }
    }

    /// Parse a connection URL.
    ///
    /// Format: `hdbsql://user:password@host:port[/database]`
    ///
    /// # Errors
    ///
    /// Returns error if URL is invalid.
    pub fn from_url(url: &str) -> Result<ConnectionBuilder<HasHost, HasCredentials>, PyHdbError> {
        let parsed = ParsedConnectionUrl::parse(url)?;

        Ok(ConnectionBuilder {
            host: Some(parsed.host),
            port: parsed.port,
            user: Some(parsed.user),
            password: Some(parsed.password),
            database: parsed.database,
            tls: parsed.use_tls,
            _host_state: PhantomData,
            _cred_state: PhantomData,
        })
    }
}

impl<C: BuilderState> ConnectionBuilder<MissingHost, C> {
    /// Set the host.
    #[must_use]
    pub fn host(self, host: impl Into<String>) -> ConnectionBuilder<HasHost, C> {
        ConnectionBuilder {
            host: Some(host.into()),
            port: self.port,
            user: self.user,
            password: self.password,
            database: self.database,
            tls: self.tls,
            _host_state: PhantomData,
            _cred_state: PhantomData,
        }
    }
}

impl<H: BuilderState> ConnectionBuilder<H, MissingCredentials> {
    /// Set the credentials.
    #[must_use]
    pub fn credentials(
        self,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> ConnectionBuilder<H, HasCredentials> {
        ConnectionBuilder {
            host: self.host,
            port: self.port,
            user: Some(user.into()),
            password: Some(password.into()),
            database: self.database,
            tls: self.tls,
            _host_state: PhantomData,
            _cred_state: PhantomData,
        }
    }
}

impl<H: BuilderState, C: BuilderState> ConnectionBuilder<H, C> {
    /// Set the port.
    #[must_use]
    pub const fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the database name.
    #[must_use]
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Enable TLS.
    #[must_use]
    pub const fn tls(mut self, enabled: bool) -> Self {
        self.tls = enabled;
        self
    }
}

impl ConnectionBuilder<HasHost, HasCredentials> {
    /// Build connection parameters.
    ///
    /// Only available when both host and credentials are set.
    ///
    /// # Type Safety
    ///
    /// The typestate pattern guarantees that `host`, `user`, and `password` are always
    /// `Some` when this method is callable. The `ok_or_else` checks below are defensive
    /// measures for defense-in-depth. If these errors ever occur, they indicate an
    /// internal driver bug rather than a user error.
    ///
    /// # Example (Compile-Time Safety)
    ///
    /// ```compile_fail
    /// use hdbconnect_py::connection::builder::ConnectionBuilder;
    ///
    /// // This won't compile - build() requires HasHost and HasCredentials
    /// let params = ConnectionBuilder::new().build();
    /// // Error: method `build` not found for this type
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid or if an internal invariant is violated.
    pub fn build(self) -> Result<hdbconnect::ConnectParams, PyHdbError> {
        let host = self
            .host
            .ok_or_else(|| PyHdbError::internal("internal: host missing"))?;
        let user = self
            .user
            .ok_or_else(|| PyHdbError::internal("internal: user missing"))?;
        let password = self
            .password
            .ok_or_else(|| PyHdbError::internal("internal: password missing"))?;

        let mut params_builder = hdbconnect::ConnectParams::builder();
        params_builder.hostname(&host);
        params_builder.port(self.port);
        params_builder.dbuser(&user);
        params_builder.password(&password);

        if let Some(db) = &self.database {
            params_builder.dbname(db);
        }

        let params = params_builder
            .build()
            .map_err(|e| PyHdbError::interface(e.to_string()))?;

        Ok(params)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// AsyncConnectionBuilder (async feature only)
// ═══════════════════════════════════════════════════════════════════════════════

/// Async-aware connection builder with configuration support.
///
/// Wraps `ConnectionBuilder` and adds async connection methods with optional
/// configuration support. Uses the same typestate pattern for compile-time safety.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_py::connection::AsyncConnectionBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // From URL (most common)
/// let conn = AsyncConnectionBuilder::from_url("hdbsql://user:pass@host:30015")?
///     .connect()
///     .await?;
///
/// // With configuration
/// let config = hdbconnect::ConnectionConfiguration::default();
/// let conn = AsyncConnectionBuilder::from_url("hdbsql://user:pass@host:30015")?
///     .with_config(&config)
///     .connect()
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// # Programmatic Construction
///
/// ```rust,ignore
/// use hdbconnect_py::connection::AsyncConnectionBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = AsyncConnectionBuilder::new()
///     .host("localhost")
///     .port(30015)
///     .credentials("user", "password")
///     .tls(true)
///     .connect()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncConnectionBuilder<H: BuilderState, C: BuilderState> {
    inner: ConnectionBuilder<H, C>,
    config: Option<ConnectionConfiguration>,
}

#[cfg(feature = "async")]
impl Default for AsyncConnectionBuilder<MissingHost, MissingCredentials> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
impl AsyncConnectionBuilder<MissingHost, MissingCredentials> {
    /// Create a new async connection builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: ConnectionBuilder::new(),
            config: None,
        }
    }

    /// Parse a connection URL.
    ///
    /// Format: `hdbsql://user:password@host:port[/database]`
    ///
    /// # Errors
    ///
    /// Returns error if URL is invalid.
    pub fn from_url(
        url: &str,
    ) -> Result<AsyncConnectionBuilder<HasHost, HasCredentials>, PyHdbError> {
        let inner = ConnectionBuilder::from_url(url)?;
        Ok(AsyncConnectionBuilder {
            inner,
            config: None,
        })
    }
}

#[cfg(feature = "async")]
impl<C: BuilderState> AsyncConnectionBuilder<MissingHost, C> {
    /// Set the host.
    #[must_use]
    pub fn host(self, host: impl Into<String>) -> AsyncConnectionBuilder<HasHost, C> {
        AsyncConnectionBuilder {
            inner: self.inner.host(host),
            config: self.config,
        }
    }
}

#[cfg(feature = "async")]
impl<H: BuilderState> AsyncConnectionBuilder<H, MissingCredentials> {
    /// Set the credentials.
    #[must_use]
    pub fn credentials(
        self,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> AsyncConnectionBuilder<H, HasCredentials> {
        AsyncConnectionBuilder {
            inner: self.inner.credentials(user, password),
            config: self.config,
        }
    }
}

#[cfg(feature = "async")]
impl<H: BuilderState, C: BuilderState> AsyncConnectionBuilder<H, C> {
    /// Set the port.
    #[must_use]
    pub fn port(self, port: u16) -> Self {
        Self {
            inner: self.inner.port(port),
            config: self.config,
        }
    }

    /// Set the database name.
    #[must_use]
    pub fn database(self, database: impl Into<String>) -> Self {
        Self {
            inner: self.inner.database(database),
            config: self.config,
        }
    }

    /// Enable TLS.
    #[must_use]
    pub fn tls(self, enabled: bool) -> Self {
        Self {
            inner: self.inner.tls(enabled),
            config: self.config,
        }
    }

    /// Set connection configuration.
    ///
    /// Configuration includes fetch size, LOB settings, timeouts, etc.
    #[must_use]
    pub fn with_config(mut self, config: &ConnectionConfiguration) -> Self {
        self.config = Some(config.clone());
        self
    }
}

#[cfg(feature = "async")]
impl AsyncConnectionBuilder<HasHost, HasCredentials> {
    /// Build connection parameters.
    ///
    /// Only available when both host and credentials are set.
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid.
    pub fn build(self) -> Result<hdbconnect::ConnectParams, PyHdbError> {
        self.inner.build()
    }

    /// Connect to the database asynchronously.
    ///
    /// Only available when both host and credentials are set.
    ///
    /// # Errors
    ///
    /// Returns error if connection fails.
    pub async fn connect(self) -> Result<hdbconnect_async::Connection, PyHdbError> {
        let params = self.inner.build()?;

        let connection = match self.config {
            Some(cfg) => hdbconnect_async::Connection::with_configuration(params, &cfg)
                .await
                .map_err(|e| PyHdbError::operational(e.to_string()))?,
            None => hdbconnect_async::Connection::new(params)
                .await
                .map_err(|e| PyHdbError::operational(e.to_string()))?,
        };

        Ok(connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // ConnectionBuilder Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_from_url() {
        let builder = ConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015/mydb");
        assert!(builder.is_ok());
    }

    #[test]
    fn test_builder_missing_host() {
        let result = ConnectionBuilder::from_url("hdbsql://user:pass@/mydb");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_fluent() {
        let _builder = ConnectionBuilder::new()
            .host("localhost")
            .port(30015)
            .credentials("test_user", "test_password")
            .database("mydb")
            .tls(true);
        // Type system ensures this is ConnectionBuilder<HasHost, HasCredentials>
    }

    #[test]
    fn test_builder_missing_username() {
        let result = ConnectionBuilder::from_url("hdbsql://:pass@localhost:30015");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_missing_password() {
        let result = ConnectionBuilder::from_url("hdbsql://user@localhost:30015");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_tls_scheme() {
        let builder = ConnectionBuilder::from_url("hdbsqls://user:pass@localhost:30015");
        assert!(builder.is_ok());
        // TLS should be enabled for hdbsqls scheme
    }

    #[test]
    fn test_builder_without_database() {
        let builder = ConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015");
        assert!(builder.is_ok());
    }

    #[test]
    fn test_builder_default_port() {
        let builder = ConnectionBuilder::from_url("hdbsql://user:pass@localhost");
        assert!(builder.is_ok());
    }

    #[test]
    fn test_builder_default() {
        let builder = ConnectionBuilder::<MissingHost, MissingCredentials>::default();
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("ConnectionBuilder"));
    }

    #[test]
    fn test_builder_invalid_url() {
        let result = ConnectionBuilder::from_url("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_state_debug_implementations() {
        assert_eq!(format!("{:?}", MissingHost), "MissingHost");
        assert_eq!(format!("{:?}", HasHost), "HasHost");
        assert_eq!(format!("{:?}", MissingCredentials), "MissingCredentials");
        assert_eq!(format!("{:?}", HasCredentials), "HasCredentials");
    }

    #[test]
    fn test_state_default_implementations() {
        let _missing_host: MissingHost = Default::default();
        let _has_host: HasHost = Default::default();
        let _missing_creds: MissingCredentials = Default::default();
        let _has_creds: HasCredentials = Default::default();
    }

    #[test]
    fn test_builder_with_database_creates_params() {
        let builder = ConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015/mydb");
        assert!(builder.is_ok());
        let params = builder.unwrap().build();
        assert!(params.is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AsyncConnectionBuilder Tests (async feature only)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_from_url() {
        let builder = AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015/mydb");
        assert!(builder.is_ok());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_missing_host() {
        let result = AsyncConnectionBuilder::from_url("hdbsql://user:pass@/mydb");
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_fluent() {
        let _builder = AsyncConnectionBuilder::new()
            .host("localhost")
            .port(30015)
            .credentials("test_user", "test_password")
            .database("mydb")
            .tls(true);
        // Type system ensures this is AsyncConnectionBuilder<HasHost, HasCredentials>
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_missing_username() {
        let result = AsyncConnectionBuilder::from_url("hdbsql://:pass@localhost:30015");
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_missing_password() {
        let result = AsyncConnectionBuilder::from_url("hdbsql://user@localhost:30015");
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_with_tls_scheme() {
        let builder = AsyncConnectionBuilder::from_url("hdbsqls://user:pass@localhost:30015");
        assert!(builder.is_ok());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_without_database() {
        let builder = AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015");
        assert!(builder.is_ok());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_default_port() {
        let builder = AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost");
        assert!(builder.is_ok());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_default() {
        let builder = AsyncConnectionBuilder::<MissingHost, MissingCredentials>::default();
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("AsyncConnectionBuilder"));
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_invalid_url() {
        let result = AsyncConnectionBuilder::from_url("not-a-valid-url");
        assert!(result.is_err());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_with_config() {
        let config = ConnectionConfiguration::default();
        let builder = AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015")
            .unwrap()
            .with_config(&config);
        assert!(builder.config.is_some());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_build_params() {
        let result = AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015")
            .unwrap()
            .build();
        assert!(result.is_ok());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_programmatic_build() {
        let result = AsyncConnectionBuilder::new()
            .host("localhost")
            .port(30015)
            .credentials("test_user", "test_password")
            .database("mydb")
            .build();
        assert!(result.is_ok());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_config_chaining() {
        let config = ConnectionConfiguration::default();
        let builder = AsyncConnectionBuilder::new()
            .host("localhost")
            .credentials("test_user", "test_password")
            .port(30015)
            .database("mydb")
            .tls(false)
            .with_config(&config);

        assert!(builder.config.is_some());
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("AsyncConnectionBuilder"));
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_without_config() {
        let builder =
            AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015").unwrap();
        assert!(builder.config.is_none());
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_builder_with_database_creates_params() {
        let builder = AsyncConnectionBuilder::from_url("hdbsql://user:pass@localhost:30015/mydb");
        assert!(builder.is_ok());
        let params = builder.unwrap().build();
        assert!(params.is_ok());
    }
}
