//! Shared utilities for connection management.
//!
//! This module centralizes common functionality used across sync and async
//! connection implementations to eliminate code duplication.

mod tls;
pub mod url_parser;
pub mod validation;

#[cfg(feature = "async")]
pub(crate) use tls::apply_tls_to_async_builder;
pub(crate) use tls::apply_tls_to_sync_builder;
pub use url_parser::ParsedConnectionUrl;
pub use validation::{VALIDATION_QUERY, validate_non_negative_f64, validate_positive_u32};
