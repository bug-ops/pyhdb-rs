//! Type definitions for HANA to Arrow conversion.
//!
//! This module contains:
//!
//! - [`hana`] - HANA type representations with phantom type markers
//! - [`arrow`] - Arrow type mappings from HANA types
//! - [`conversion`] - Type conversion implementations

pub mod arrow;
pub mod conversion;
pub mod hana;

pub use arrow::{FieldMetadataExt, hana_field_to_arrow, hana_type_to_arrow};
pub use conversion::TypeCategory;
pub use hana::{
    Binary, Decimal, DecimalPrecision, DecimalScale, HanaTypeCategory, Lob, Numeric, Spatial,
    StringType, Temporal, TypedColumn,
};
