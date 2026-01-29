//! Async support module for SAP HANA Python driver.
//!
//! Provides async/await support using tokio runtime and pyo3-async-runtimes.
//! Feature-gated behind `async` feature.
//!
//! # Example
//!
//! ```python
//! import asyncio
//! from pyhdb_rs.aio import connect, create_pool
//!
//! async def main():
//!     async with await connect("hdbsql://user:pass@host:30015") as conn:
//!         reader = await conn.execute_arrow("SELECT * FROM sales")
//!         df = pl.from_arrow(reader)
//!
//!     pool = create_pool("hdbsql://user:pass@host:30015", max_size=10)
//!     async with pool.acquire() as conn:
//!         await cursor.execute("SELECT * FROM products")
//!
//! asyncio.run(main())
//! ```
//!
//! # Statement Cache
//!
//! Both `AsyncConnection` and `PooledConnection` include prepared statement
//! caching. Configure via `ConnectionConfig(max_cached_statements=N)`.

// PyO3 async FFI captures connection state in futures; boxing would add unnecessary overhead.
// These futures necessarily hold Arc<TokioMutex<T>> and connection state for Python interop.
#![allow(clippy::large_futures)]
// Async code requires explicit lock release before yield points via drop(guard).
// Clippy's significant_drop_tightening doesn't account for async boundaries where
// we intentionally release locks before calling other async operations.
#![allow(clippy::significant_drop_tightening)]
// Types like Arc<TokioMutex<Option<PooledObject>>> are inherently complex but correctly
// model the domain: shared ownership (Arc) + async safety (TokioMutex) + pool return
// semantics (Option) + pooled connection (PooledObject).
#![allow(clippy::type_complexity)]
// PyO3 pymethods returning PyResult<T> where T could theoretically be returned directly.
// Required by PyO3 trait bounds for Python FFI error propagation even when success is
// guaranteed (e.g., set_autocommit setter).
#![allow(clippy::unnecessary_wraps)]
// PyO3 #[pymethods] cannot be const fn due to Python FFI requirements. Clippy suggests
// const for simple functions that would be const in pure Rust but are bound to Python.
#![allow(clippy::missing_const_for_fn)]

pub mod common;
pub mod connection;
pub mod cursor;
pub mod pool;

pub use common::ConnectionState;
pub use connection::{AsyncConnectionInner, AsyncPyConnection, SharedAsyncConnection};
pub use cursor::AsyncPyCursor;
pub use pool::{
    HanaConnectionManager, PoolConfig, PooledConnection, PyConnectionPool, PyConnectionPoolBuilder,
};
