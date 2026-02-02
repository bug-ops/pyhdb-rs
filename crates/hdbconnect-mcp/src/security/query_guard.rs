//! Query execution wrapper with security checks

use std::future::Future;
use std::num::NonZeroU32;
use std::time::Duration;

use super::SchemaFilter;
use crate::Error;

/// Security wrapper for query execution
#[derive(Debug, Clone)]
pub struct QueryGuard {
    timeout: Duration,
    schema_filter: SchemaFilter,
    row_limit: Option<NonZeroU32>,
}

impl QueryGuard {
    /// Create a new query guard with security settings
    #[must_use]
    pub const fn new(
        timeout: Duration,
        schema_filter: SchemaFilter,
        row_limit: Option<NonZeroU32>,
    ) -> Self {
        Self {
            timeout,
            schema_filter,
            row_limit,
        }
    }

    /// Get the configured row limit
    #[must_use]
    pub const fn row_limit(&self) -> Option<NonZeroU32> {
        self.row_limit
    }

    /// Get the configured timeout
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Validate schema access
    pub fn validate_schema(&self, schema: &str) -> Result<(), Error> {
        self.schema_filter.validate(schema)
    }

    /// Execute a query function with timeout
    pub async fn execute<F, T, E>(&self, query_fn: F) -> Result<T, Error>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<Error>,
    {
        tokio::time::timeout(self.timeout, query_fn)
            .await
            .map_err(|_| Error::QueryTimeout(self.timeout))?
            .map_err(Into::into)
    }

    /// Execute a query function with timeout, returning the raw error type
    pub async fn execute_with_error<F, T, E>(&self, query_fn: F) -> Result<T, ExecuteError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        tokio::time::timeout(self.timeout, query_fn)
            .await
            .map_err(|_| ExecuteError::Timeout(self.timeout))?
            .map_err(ExecuteError::Query)
    }
}

/// Error type for [`Self::execute_with_error`]
#[derive(Debug)]
pub enum ExecuteError<E> {
    Timeout(Duration),
    Query(E),
}

impl<E> ExecuteError<E> {
    /// Check if this is a timeout error
    #[must_use]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }

    /// Check if this is a query error
    #[must_use]
    pub const fn is_query(&self) -> bool {
        matches!(self, Self::Query(_))
    }
}

impl<E: std::fmt::Display> std::fmt::Display for ExecuteError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(d) => write!(f, "query timeout after {d:?}"),
            Self::Query(e) => write!(f, "{e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ExecuteError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timeout(_) => None,
            Self::Query(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[tokio::test]
    async fn test_query_guard_success() {
        let guard = QueryGuard::new(
            Duration::from_secs(5),
            SchemaFilter::AllowAll,
            NonZeroU32::new(1000),
        );

        let result: Result<i32, Error> = guard.execute(async { Ok::<_, Error>(42) }).await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_query_guard_timeout() {
        let guard = QueryGuard::new(Duration::from_millis(10), SchemaFilter::AllowAll, None);

        let result: Result<i32, Error> = guard
            .execute(async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok::<_, Error>(42)
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::QueryTimeout(_)));
    }

    #[test]
    fn test_query_guard_schema_validation() {
        let denied: HashSet<String> = ["SYS"].iter().map(|s| (*s).to_string()).collect();
        let guard = QueryGuard::new(
            Duration::from_secs(30),
            SchemaFilter::Blacklist(denied),
            None,
        );

        assert!(guard.validate_schema("APP").is_ok());
        assert!(guard.validate_schema("SYS").is_err());
    }

    #[test]
    fn test_query_guard_row_limit() {
        let guard = QueryGuard::new(
            Duration::from_secs(30),
            SchemaFilter::AllowAll,
            NonZeroU32::new(5000),
        );

        assert_eq!(guard.row_limit(), NonZeroU32::new(5000));
    }

    #[test]
    fn test_query_guard_row_limit_none() {
        let guard = QueryGuard::new(Duration::from_secs(30), SchemaFilter::AllowAll, None);

        assert!(guard.row_limit().is_none());
    }

    #[test]
    fn test_query_guard_timeout_accessor() {
        let guard = QueryGuard::new(Duration::from_secs(42), SchemaFilter::AllowAll, None);

        assert_eq!(guard.timeout(), Duration::from_secs(42));
    }

    #[test]
    fn test_query_guard_whitelist_filter() {
        let allowed: HashSet<String> = ["APP", "PUBLIC"].iter().map(|s| (*s).to_string()).collect();
        let guard = QueryGuard::new(
            Duration::from_secs(30),
            SchemaFilter::Whitelist(allowed),
            None,
        );

        assert!(guard.validate_schema("APP").is_ok());
        assert!(guard.validate_schema("PUBLIC").is_ok());
        assert!(guard.validate_schema("SYS").is_err());
    }

    #[test]
    fn test_query_guard_debug() {
        let guard = QueryGuard::new(Duration::from_secs(30), SchemaFilter::AllowAll, None);
        let debug_str = format!("{guard:?}");
        assert!(debug_str.contains("QueryGuard"));
    }

    #[test]
    fn test_query_guard_clone() {
        let guard = QueryGuard::new(
            Duration::from_secs(30),
            SchemaFilter::AllowAll,
            NonZeroU32::new(1000),
        );
        let cloned = guard.clone();
        assert_eq!(cloned.timeout(), guard.timeout());
        assert_eq!(cloned.row_limit(), guard.row_limit());
    }

    #[tokio::test]
    async fn test_execute_with_error_success() {
        let guard = QueryGuard::new(Duration::from_secs(5), SchemaFilter::AllowAll, None);

        let result: Result<i32, ExecuteError<std::io::Error>> =
            guard.execute_with_error(async { Ok(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_execute_with_error_timeout() {
        let guard = QueryGuard::new(Duration::from_millis(10), SchemaFilter::AllowAll, None);

        let result: Result<i32, ExecuteError<std::io::Error>> = guard
            .execute_with_error(async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(42)
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_timeout());
        assert!(!err.is_query());
    }

    #[tokio::test]
    async fn test_execute_with_error_query_error() {
        let guard = QueryGuard::new(Duration::from_secs(5), SchemaFilter::AllowAll, None);

        let result: Result<i32, ExecuteError<std::io::Error>> = guard
            .execute_with_error(async {
                Err::<i32, _>(std::io::Error::new(std::io::ErrorKind::Other, "test error"))
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_query());
        assert!(!err.is_timeout());
    }

    #[test]
    fn test_execute_error_display_timeout() {
        let err: ExecuteError<std::io::Error> = ExecuteError::Timeout(Duration::from_secs(30));
        let display = format!("{err}");
        assert!(display.contains("timeout"));
        assert!(display.contains("30"));
    }

    #[test]
    fn test_execute_error_display_query() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let err: ExecuteError<std::io::Error> = ExecuteError::Query(io_err);
        let display = format!("{err}");
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_execute_error_source_timeout() {
        let err: ExecuteError<std::io::Error> = ExecuteError::Timeout(Duration::from_secs(30));
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn test_execute_error_source_query() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let err: ExecuteError<std::io::Error> = ExecuteError::Query(io_err);
        assert!(std::error::Error::source(&err).is_some());
    }
}
