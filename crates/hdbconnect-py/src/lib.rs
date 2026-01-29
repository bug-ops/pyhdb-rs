//! Python bindings for SAP HANA via hdbconnect.
//!
//! This crate provides Python bindings using `PyO3` with:
//! - DB-API 2.0 compliant Connection and Cursor
//! - Zero-copy Arrow data transfer via `PyCapsule` Interface
//! - Native Polars/pandas integration
//!
//! # Example
//!
//! ```python
//! from pyhdb_rs import ConnectionBuilder
//! import polars as pl
//!
//! # Connect to HANA
//! conn = ConnectionBuilder.from_url("hdbsql://user:pass@host:30015").build()
//!
//! # Execute query
//! cursor = conn.cursor()
//! cursor.execute("SELECT * FROM USERS")
//!
//! # Fetch rows
//! for row in cursor:
//!     print(row)
//!
//! # Or get as Polars DataFrame via Arrow
//! reader = conn.execute_arrow("SELECT * FROM USERS")
//! df = pl.from_arrow(reader)
//!
//! conn.close()
//! ```

use pyo3::prelude::*;

pub mod config;
pub mod connection;
pub mod cursor;
pub mod cursor_holdability;
pub mod error;
mod private;
pub mod reader;
pub mod tls;
pub mod types;
pub mod utils;

#[cfg(feature = "async")]
pub mod async_support;

#[cfg(feature = "async")]
pub use async_support::{
    AsyncPyConnection, AsyncPyCursor, PooledConnection, PyConnectionPool, PyConnectionPoolBuilder,
};
pub use config::{PyArrowConfig, PyConnectionConfig};
#[cfg(feature = "async")]
pub use connection::PyAsyncConnectionBuilder;
pub use connection::{PyCacheStats, PyConnection, PyConnectionBuilder};
pub use cursor::PyCursor;
pub use cursor_holdability::PyCursorHoldability;
pub use error::PyHdbError;
pub use reader::PyRecordBatchReader;
pub use tls::PyTlsConfig;

/// DB-API 2.0 API level.
const APILEVEL: &str = "2.0";

/// DB-API 2.0 thread safety level.
/// 2 = Threads may share the module and connections.
const THREADSAFETY: i32 = 2;

/// DB-API 2.0 parameter style.
/// "qmark" = Question mark style (SELECT * FROM t WHERE id = ?)
const PARAMSTYLE: &str = "qmark";

/// Connect to a HANA database with optional configuration.
///
/// Args:
///     url: Connection URL (hdbsql://user:pass@host:port[/database])
///     config: Optional connection configuration for tuning performance
///
/// Returns:
///     Connection object
///
/// Raises:
///     `InterfaceError`: If URL is invalid
///     `OperationalError`: If connection fails
///
/// Example:
///     ```python
///     # Basic connection
///     conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")
///
///     # Connection with custom configuration
///     config = ConnectionConfig(fetch_size=50000, read_timeout=60.0)
///     conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015", config=config)
///     ```
#[pyfunction]
#[pyo3(signature = (url, *, config=None))]
fn connect(url: &str, config: Option<&PyConnectionConfig>) -> PyResult<PyConnection> {
    config.map_or_else(
        || PyConnection::new(url),
        |cfg| PyConnection::with_config(url, cfg),
    )
}

/// HANA Python driver module.
///
/// The module is named `_core` to support maturin's nested module structure:
/// `pyhdb_rs._core` (Rust) + `pyhdb_rs/__init__.py` (Python re-exports).
#[pymodule]
#[pyo3(name = "_core")]
fn pyhdb_rs_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // DB-API 2.0 module globals
    m.add("apilevel", APILEVEL)?;
    m.add("threadsafety", THREADSAFETY)?;
    m.add("paramstyle", PARAMSTYLE)?;

    // Version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Connection function
    m.add_function(wrap_pyfunction!(connect, m)?)?;

    // Classes
    m.add_class::<PyConnection>()?;
    m.add_class::<PyCursor>()?;
    m.add_class::<PyRecordBatchReader>()?;
    m.add_class::<PyConnectionConfig>()?;
    m.add_class::<PyArrowConfig>()?;
    m.add_class::<PyCacheStats>()?;

    // Builder API classes
    m.add_class::<PyTlsConfig>()?;
    m.add_class::<PyConnectionBuilder>()?;
    m.add_class::<PyCursorHoldability>()?;

    // Async classes (when feature enabled)
    #[cfg(feature = "async")]
    {
        m.add_class::<AsyncPyConnection>()?;
        m.add_class::<AsyncPyCursor>()?;
        m.add_class::<PyConnectionPool>()?;
        m.add_class::<PooledConnection>()?;
        m.add_class::<async_support::pool::PoolStatus>()?;
        m.add_class::<PyAsyncConnectionBuilder>()?;
        m.add_class::<PyConnectionPoolBuilder>()?;
        m.add("ASYNC_AVAILABLE", true)?;
    }

    #[cfg(not(feature = "async"))]
    {
        m.add("ASYNC_AVAILABLE", false)?;
    }

    // Exceptions
    error::register_exceptions(m.py(), m)?;

    Ok(())
}
