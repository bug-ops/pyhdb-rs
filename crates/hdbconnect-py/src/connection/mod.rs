//! Connection module for SAP HANA database connections.
//!
//! Provides:
//! - `PyConnection`: `PyO3` class for DB-API 2.0 compliant connections
//! - `PyCacheStats`: Python-exposed cache statistics
//! - `ConnectionBuilder`: Type-safe builder with compile-time validation
//! - `AsyncConnectionBuilder`: Async-aware builder with configuration support (async feature)
//! - `PyConnectionBuilder`: Python-facing builder with runtime validation
//! - `PyAsyncConnectionBuilder`: Python-facing async builder (async feature)
//! - State types for typestate pattern

pub mod builder;
pub mod py_builder;
pub mod wrapper;

#[cfg(feature = "async")]
pub use builder::AsyncConnectionBuilder;
pub use builder::ConnectionBuilder;
#[cfg(feature = "async")]
pub use py_builder::PyAsyncConnectionBuilder;
pub use py_builder::PyConnectionBuilder;
pub use wrapper::{ConnectionInner, PyCacheStats, PyConnection, SharedConnection};
