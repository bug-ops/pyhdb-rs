//! Connection module for SAP HANA database connections.
//!
//! Provides:
//! - `PyConnection`: `PyO3` class for DB-API 2.0 compliant connections
//! - `PyCacheStats`: Python-exposed cache statistics
//! - `ConnectionBuilder`: Type-safe builder with compile-time validation
//! - `AsyncConnectionBuilder`: Async-aware builder with configuration support (async feature)
//! - State types for typestate pattern

pub mod builder;
pub mod state;
pub mod wrapper;

#[cfg(feature = "async")]
pub use builder::AsyncConnectionBuilder;
pub use builder::ConnectionBuilder;
pub use state::{Connected, ConnectionState, Disconnected, InTransaction, TypedConnection};
pub use wrapper::{ConnectionInner, PyCacheStats, PyConnection, SharedConnection};
