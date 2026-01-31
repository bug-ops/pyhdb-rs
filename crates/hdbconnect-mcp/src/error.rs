use rmcp::ErrorData;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Connection error: {0}")]
    Connection(#[from] hdbconnect::HdbError),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Read-only mode: {0}")]
    ReadOnlyViolation(String),
}

impl Error {
    pub const fn read_only_violation(msg: String) -> Self {
        Self::ReadOnlyViolation(msg)
    }

    #[must_use]
    pub const fn is_read_only_violation(&self) -> bool {
        matches!(self, Self::ReadOnlyViolation(_))
    }
}

/// Convert our Error type to rmcp `ErrorData`
impl From<Error> for ErrorData {
    fn from(err: Error) -> Self {
        match err {
            Error::Connection(e) => {
                Self::internal_error(format!("Database connection error: {e}"), None)
            }
            Error::Query(msg) => Self::internal_error(format!("Query error: {msg}"), None),
            Error::Config(msg) => Self::invalid_params(format!("Configuration error: {msg}"), None),
            Error::PoolExhausted => Self::internal_error("Connection pool exhausted", None),
            Error::ReadOnlyViolation(msg) => {
                Self::invalid_params(format!("Read-only mode violation: {msg}"), None)
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
