//! URL parsing utilities for connection strings.

use crate::error::PyHdbError;

/// Parsed connection URL with validated components.
///
/// Use `ParsedConnectionUrl::parse()` to create from a URL string.
/// All fields are guaranteed valid after successful parsing.
#[derive(Clone)]
pub struct ParsedConnectionUrl {
    /// Database hostname or IP address.
    pub host: String,
    /// Database port (default: 30015).
    pub port: u16,
    /// Database username.
    pub user: String,
    /// Database password.
    pub password: String,
    /// Optional database/tenant name.
    pub database: Option<String>,
    /// Whether TLS should be enabled (hdbsqls:// scheme).
    pub use_tls: bool,
}

impl std::fmt::Debug for ParsedConnectionUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedConnectionUrl")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &"[REDACTED]")
            .field("database", &self.database)
            .field("use_tls", &self.use_tls)
            .finish()
    }
}

impl ParsedConnectionUrl {
    /// Parse a connection URL.
    ///
    /// # Format
    ///
    /// `hdbsql://user:password@host:port[/database]`
    /// `hdbsqls://user:password@host:port[/database]` (TLS enabled)
    ///
    /// # Arguments
    ///
    /// * `url` - Connection URL string
    ///
    /// # Errors
    ///
    /// Returns `InterfaceError` if:
    /// - URL is malformed
    /// - Host is missing
    /// - Username is missing
    /// - Password is missing
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let parsed = ParsedConnectionUrl::parse("hdbsql://user:pass@host:30015/mydb")?;
    /// assert_eq!(parsed.host, "host");
    /// assert_eq!(parsed.port, 30015);
    /// assert_eq!(parsed.database, Some("mydb".to_string()));
    /// ```
    pub fn parse(url: &str) -> Result<Self, PyHdbError> {
        let parsed =
            url::Url::parse(url).map_err(|e| PyHdbError::interface(format!("invalid URL: {e}")))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| PyHdbError::interface("missing host in URL"))?
            .to_string();

        let port = parsed.port().unwrap_or(30015);

        if parsed.username().is_empty() {
            return Err(PyHdbError::interface("missing username in URL"));
        }
        let user = parsed.username().to_string();

        let password = parsed
            .password()
            .ok_or_else(|| PyHdbError::interface("missing password in URL"))?
            .to_string();

        let database = parsed
            .path()
            .strip_prefix('/')
            .filter(|s| !s.is_empty())
            .map(String::from);

        let use_tls = parsed.scheme() == "hdbsqls";

        Ok(Self {
            host,
            port,
            user,
            password,
            database,
            use_tls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_url() {
        let parsed = ParsedConnectionUrl::parse("hdbsql://user:pass@host:30015/mydb").unwrap();
        assert_eq!(parsed.host, "host");
        assert_eq!(parsed.port, 30015);
        assert_eq!(parsed.user, "user");
        assert_eq!(parsed.password, "pass");
        assert_eq!(parsed.database, Some("mydb".to_string()));
        assert!(!parsed.use_tls);
    }

    #[test]
    fn test_parse_tls_url() {
        let parsed = ParsedConnectionUrl::parse("hdbsqls://user:pass@host:30015").unwrap();
        assert!(parsed.use_tls);
    }

    #[test]
    fn test_parse_default_port() {
        let parsed = ParsedConnectionUrl::parse("hdbsql://user:pass@host").unwrap();
        assert_eq!(parsed.port, 30015);
    }

    #[test]
    fn test_parse_missing_host() {
        let result = ParsedConnectionUrl::parse("hdbsql://user:pass@/mydb");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_username() {
        let result = ParsedConnectionUrl::parse("hdbsql://:pass@host:30015");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_password() {
        let result = ParsedConnectionUrl::parse("hdbsql://user@host:30015");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_url() {
        let result = ParsedConnectionUrl::parse("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_no_database() {
        let parsed = ParsedConnectionUrl::parse("hdbsql://user:pass@host:30015").unwrap();
        assert!(parsed.database.is_none());
    }

    #[test]
    fn test_debug_redacts_password() {
        let parsed =
            ParsedConnectionUrl::parse("hdbsql://user:secret_password@host:30015/mydb").unwrap();
        let debug_output = format!("{:?}", parsed);

        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should contain [REDACTED]"
        );
        assert!(
            !debug_output.contains("secret_password"),
            "Debug output must not contain actual password"
        );
        assert!(
            debug_output.contains("user"),
            "Debug output should still contain username"
        );
        assert!(
            debug_output.contains("host"),
            "Debug output should still contain host"
        );
    }
}
