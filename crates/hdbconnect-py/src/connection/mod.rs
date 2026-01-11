//! Connection module for SAP HANA database connections.
//!
//! Provides:
//! - `PyConnection`: `PyO3` class for DB-API 2.0 compliant connections
//! - `ConnectionBuilder`: Type-safe builder with compile-time validation
//! - State types for typestate pattern

pub mod builder;
pub mod state;
pub mod wrapper;

pub use builder::ConnectionBuilder;
pub use state::{Connected, ConnectionState, Disconnected, InTransaction, TypedConnection};
pub use wrapper::{ConnectionInner, PyConnection, SharedConnection};
