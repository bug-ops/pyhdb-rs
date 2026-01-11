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
//!         df = await conn.execute_polars("SELECT * FROM sales")
//!
//!     pool = create_pool("hdbsql://user:pass@host:30015", max_size=10)
//!     async with pool.acquire() as conn:
//!         await cursor.execute("SELECT * FROM products")
//!
//! asyncio.run(main())
//! ```

#![allow(
    clippy::large_futures,
    clippy::significant_drop_tightening,
    clippy::unnecessary_wraps,
    clippy::type_complexity,
    clippy::missing_const_for_fn,
    clippy::unused_self,
    clippy::cast_precision_loss
)]

pub mod connection;
pub mod cursor;
pub mod pool;
pub mod statement_cache;

pub use connection::{AsyncConnectionInner, AsyncPyConnection, SharedAsyncConnection};
pub use cursor::AsyncPyCursor;
pub use pool::{HanaConnectionManager, PoolConfig, PooledConnection, PyConnectionPool};
pub use statement_cache::{CacheStats, PreparedStatementCache};

#[cfg(test)]
mod tests;
