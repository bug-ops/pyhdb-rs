//! Cursor module for query execution and result fetching.
//!
//! Provides typestate-based cursor management.

pub mod state;
pub mod wrapper;

pub use state::{CursorState, Executed, Exhausted, Fetching, Idle};
pub use wrapper::PyCursor;
