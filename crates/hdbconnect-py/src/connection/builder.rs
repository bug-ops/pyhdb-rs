//! Type-safe connection builder with phantom types.
//!
//! Enforces that host and credentials are set before building.

use std::marker::PhantomData;

use crate::error::PyHdbError;
use crate::private::sealed::Sealed;

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
        let parsed = url::Url::parse(url)?;

        let host = parsed
            .host_str()
            .ok_or_else(|| PyHdbError::interface("missing host in URL"))?
            .to_string();

        let port = parsed.port().unwrap_or(30015);

        let user = if parsed.username().is_empty() {
            return Err(PyHdbError::interface("missing username in URL"));
        } else {
            parsed.username().to_string()
        };

        let password = parsed
            .password()
            .ok_or_else(|| PyHdbError::interface("missing password in URL"))?
            .to_string();

        let database = parsed
            .path()
            .strip_prefix('/')
            .filter(|s| !s.is_empty())
            .map(String::from);

        let tls = parsed.scheme() == "hdbsqls";

        Ok(ConnectionBuilder {
            host: Some(host),
            port,
            user: Some(user),
            password: Some(password),
            database,
            tls,
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
    /// # Errors
    ///
    /// Returns error if parameters are invalid.
    pub fn build(self) -> Result<hdbconnect::ConnectParams, PyHdbError> {
        let host = self.host.expect("host is set");
        let user = self.user.expect("user is set");
        let password = self.password.expect("password is set");

        let params = hdbconnect::ConnectParams::builder()
            .hostname(&host)
            .port(self.port)
            .dbuser(&user)
            .password(&password)
            .build()
            .map_err(|e| PyHdbError::interface(e.to_string()))?;

        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .credentials("user", "pass")
            .database("mydb")
            .tls(true);
        // Type system ensures this is ConnectionBuilder<HasHost, HasCredentials>
    }
}
