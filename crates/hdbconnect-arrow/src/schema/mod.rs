//! Schema conversion and mapping utilities.
//!
//! This module provides:
//!
//! - [`mapping`] - HANA to Arrow schema mapping

pub mod mapping;

pub use mapping::SchemaMapper;
