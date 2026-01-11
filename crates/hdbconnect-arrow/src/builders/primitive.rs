//! Primitive type builders for numeric Arrow arrays.
//!
//! Implements builders for:
//! - `UInt8` (HANA TINYINT)
//! - `Int16` (HANA SMALLINT)
//! - `Int32` (HANA INT)
//! - `Int64` (HANA BIGINT)
//! - `Float32` (HANA REAL)
//! - `Float64` (HANA DOUBLE)

use std::sync::Arc;

use arrow_array::ArrayRef;
use arrow_array::builder::{
    Float32Builder, Float64Builder, Int16Builder, Int32Builder, Int64Builder, UInt8Builder,
};

use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::sealed::private::Sealed;

// ═══════════════════════════════════════════════════════════════════════════
// Macro for Primitive Builder Implementation
// ═══════════════════════════════════════════════════════════════════════════

/// Generate builder wrapper implementation for primitive types.
///
/// # Arguments
///
/// * `$name` - Wrapper struct name (e.g., `Int32BuilderWrapper`)
/// * `$builder` - Arrow builder type (e.g., `Int32Builder`)
/// * `$rust_ty` - Rust target type (e.g., `i32`)
/// * `$hana_variant` - `HdbValue` variant name (e.g., `INT`)
macro_rules! impl_primitive_builder {
    ($name:ident, $builder:ty, $rust_ty:ty, $hana_variant:ident) => {
        /// Builder wrapper for Arrow primitive arrays.
        ///
        /// Implements [`HanaCompatibleBuilder`] for HANA value conversion.
        #[derive(Debug)]
        pub struct $name {
            builder: $builder,
            len: usize,
        }

        impl $name {
            /// Create a new builder with the specified capacity.
            #[must_use]
            pub fn new(capacity: usize) -> Self {
                Self {
                    builder: <$builder>::with_capacity(capacity),
                    len: 0,
                }
            }

            /// Create a builder with default capacity.
            #[must_use]
            pub fn default_capacity() -> Self {
                Self::new(1024)
            }
        }

        impl Sealed for $name {}

        impl HanaCompatibleBuilder for $name {
            fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
                use hdbconnect::HdbValue;

                match value {
                    HdbValue::$hana_variant(v) => {
                        let converted: $rust_ty = (*v).try_into().map_err(|e| {
                            crate::ArrowConversionError::value_conversion(
                                stringify!($name),
                                format!("cannot convert {} to {}: {}", v, stringify!($rust_ty), e),
                            )
                        })?;
                        self.builder.append_value(converted);
                    }
                    other => {
                        return Err(crate::ArrowConversionError::value_conversion(
                            stringify!($name),
                            format!("expected {}, got {:?}", stringify!($hana_variant), other),
                        ));
                    }
                }
                self.len += 1;
                Ok(())
            }

            fn append_null(&mut self) {
                self.builder.append_null();
                self.len += 1;
            }

            fn finish(&mut self) -> ArrayRef {
                self.len = 0;
                Arc::new(self.builder.finish())
            }

            fn len(&self) -> usize {
                self.len
            }

            fn capacity(&self) -> Option<usize> {
                Some(self.builder.capacity())
            }
        }
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// Builder Implementations
// ═══════════════════════════════════════════════════════════════════════════

impl_primitive_builder!(UInt8BuilderWrapper, UInt8Builder, u8, TINYINT);
impl_primitive_builder!(Int16BuilderWrapper, Int16Builder, i16, SMALLINT);
impl_primitive_builder!(Int32BuilderWrapper, Int32Builder, i32, INT);
impl_primitive_builder!(Int64BuilderWrapper, Int64Builder, i64, BIGINT);
impl_primitive_builder!(Float32BuilderWrapper, Float32Builder, f32, REAL);
impl_primitive_builder!(Float64BuilderWrapper, Float64Builder, f64, DOUBLE);

#[cfg(test)]
mod tests {
    use hdbconnect::HdbValue;

    use super::*;

    #[test]
    fn test_int32_builder_wrapper() {
        let mut builder = Int32BuilderWrapper::new(10);

        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());

        builder.append_hana_value(&HdbValue::INT(42)).unwrap();
        builder.append_null();
        builder.append_hana_value(&HdbValue::INT(-100)).unwrap();

        assert_eq!(builder.len(), 3);

        let array = builder.finish();
        assert_eq!(array.len(), 3);
        assert_eq!(builder.len(), 0); // Reset after finish
    }

    #[test]
    fn test_float64_builder_wrapper() {
        let mut builder = Float64BuilderWrapper::new(5);

        builder.append_hana_value(&HdbValue::DOUBLE(3.14)).unwrap();
        builder.append_null();

        let array = builder.finish();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_uint8_builder_wrapper() {
        let mut builder = UInt8BuilderWrapper::new(5);

        builder.append_hana_value(&HdbValue::TINYINT(255)).unwrap();
        let array = builder.finish();
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn test_capacity_hint() {
        let builder = Int32BuilderWrapper::new(100);
        assert_eq!(builder.capacity(), Some(100));
    }
}
