//! Arrow array builders for HANA value conversion.
//!
//! This module provides implementations of the [`HanaCompatibleBuilder`]
//! trait for all supported Arrow data types.

pub mod boolean;
pub mod decimal;
pub mod dispatch;
pub mod factory;
pub mod primitive;
pub mod string;
pub mod temporal;

// Re-export main types
pub use boolean::BooleanBuilderWrapper;
pub use decimal::{Decimal128BuilderWrapper, DecimalConfig};
pub use dispatch::{BuilderEnum, BuilderKind};
pub use factory::BuilderFactory;
pub use primitive::{
    Float32BuilderWrapper, Float64BuilderWrapper, Int16BuilderWrapper, Int32BuilderWrapper,
    Int64BuilderWrapper, UInt8BuilderWrapper,
};
pub use string::{
    BinaryBuilderWrapper, FixedSizeBinaryBuilderWrapper, LargeBinaryBuilderWrapper,
    LargeStringBuilderWrapper, StringBuilderWrapper,
};
pub use temporal::{
    Date32BuilderWrapper, Time64NanosecondBuilderWrapper, TimestampNanosecondBuilderWrapper,
};
