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
    use arrow_array::{Array, Float32Array, Int16Array, Int32Array, Int64Array, UInt8Array};
    use hdbconnect::HdbValue;

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // Int32BuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

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
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_int32_builder_default_capacity() {
        let builder = Int32BuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_int32_builder_wrong_type() {
        let mut builder = Int32BuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::STRING("test".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    #[test]
    fn test_int32_builder_boundary_values() {
        let mut builder = Int32BuilderWrapper::new(3);
        builder.append_hana_value(&HdbValue::INT(i32::MAX)).unwrap();
        builder.append_hana_value(&HdbValue::INT(i32::MIN)).unwrap();
        builder.append_hana_value(&HdbValue::INT(0)).unwrap();

        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(int_array.value(0), i32::MAX);
        assert_eq!(int_array.value(1), i32::MIN);
        assert_eq!(int_array.value(2), 0);
    }

    #[test]
    fn test_int32_builder_reuse() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        let _ = builder.finish();

        builder.append_hana_value(&HdbValue::INT(2)).unwrap();
        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(int_array.value(0), 2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Float64BuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_float64_builder_wrapper() {
        let mut builder = Float64BuilderWrapper::new(5);

        builder.append_hana_value(&HdbValue::DOUBLE(3.14)).unwrap();
        builder.append_null();

        let array = builder.finish();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_float64_builder_default_capacity() {
        let builder = Float64BuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_float64_builder_special_values() {
        let mut builder = Float64BuilderWrapper::new(5);
        builder
            .append_hana_value(&HdbValue::DOUBLE(f64::MAX))
            .unwrap();
        builder
            .append_hana_value(&HdbValue::DOUBLE(f64::MIN))
            .unwrap();
        builder.append_hana_value(&HdbValue::DOUBLE(0.0)).unwrap();
        builder.append_hana_value(&HdbValue::DOUBLE(-0.0)).unwrap();
        builder
            .append_hana_value(&HdbValue::DOUBLE(f64::INFINITY))
            .unwrap();

        let array = builder.finish();
        assert_eq!(array.len(), 5);
    }

    #[test]
    fn test_float64_builder_wrong_type() {
        let mut builder = Float64BuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::INT(42));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // UInt8BuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_uint8_builder_wrapper() {
        let mut builder = UInt8BuilderWrapper::new(5);

        builder.append_hana_value(&HdbValue::TINYINT(255)).unwrap();
        let array = builder.finish();
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn test_uint8_builder_default_capacity() {
        let builder = UInt8BuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_uint8_builder_boundary_values() {
        let mut builder = UInt8BuilderWrapper::new(3);
        builder.append_hana_value(&HdbValue::TINYINT(0)).unwrap();
        builder.append_hana_value(&HdbValue::TINYINT(255)).unwrap();
        builder.append_hana_value(&HdbValue::TINYINT(128)).unwrap();

        let array = builder.finish();
        let uint_array = array.as_any().downcast_ref::<UInt8Array>().unwrap();
        assert_eq!(uint_array.value(0), 0);
        assert_eq!(uint_array.value(1), 255);
        assert_eq!(uint_array.value(2), 128);
    }

    #[test]
    fn test_uint8_builder_wrong_type() {
        let mut builder = UInt8BuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::STRING("test".to_string()));
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Int16BuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_int16_builder_wrapper() {
        let mut builder = Int16BuilderWrapper::new(5);
        builder
            .append_hana_value(&HdbValue::SMALLINT(32767))
            .unwrap();
        builder
            .append_hana_value(&HdbValue::SMALLINT(-32768))
            .unwrap();
        builder.append_null();

        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int16Array>().unwrap();
        assert_eq!(int_array.value(0), 32767);
        assert_eq!(int_array.value(1), -32768);
        assert!(int_array.is_null(2));
    }

    #[test]
    fn test_int16_builder_default_capacity() {
        let builder = Int16BuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_int16_builder_wrong_type() {
        let mut builder = Int16BuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::BIGINT(100));
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Int64BuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_int64_builder_wrapper() {
        let mut builder = Int64BuilderWrapper::new(5);
        builder
            .append_hana_value(&HdbValue::BIGINT(i64::MAX))
            .unwrap();
        builder
            .append_hana_value(&HdbValue::BIGINT(i64::MIN))
            .unwrap();
        builder.append_null();

        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int64Array>().unwrap();
        assert_eq!(int_array.value(0), i64::MAX);
        assert_eq!(int_array.value(1), i64::MIN);
        assert!(int_array.is_null(2));
    }

    #[test]
    fn test_int64_builder_default_capacity() {
        let builder = Int64BuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_int64_builder_wrong_type() {
        let mut builder = Int64BuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::DOUBLE(1.0));
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Float32BuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_float32_builder_wrapper() {
        let mut builder = Float32BuilderWrapper::new(5);
        builder.append_hana_value(&HdbValue::REAL(1.5)).unwrap();
        builder.append_hana_value(&HdbValue::REAL(-2.5)).unwrap();
        builder.append_null();

        let array = builder.finish();
        let float_array = array.as_any().downcast_ref::<Float32Array>().unwrap();
        assert!((float_array.value(0) - 1.5).abs() < f32::EPSILON);
        assert!((float_array.value(1) - (-2.5)).abs() < f32::EPSILON);
        assert!(float_array.is_null(2));
    }

    #[test]
    fn test_float32_builder_default_capacity() {
        let builder = Float32BuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_float32_builder_wrong_type() {
        let mut builder = Float32BuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::INT(42));
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Common Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_capacity_hint() {
        let builder = Int32BuilderWrapper::new(100);
        assert_eq!(builder.capacity(), Some(100));
    }

    #[test]
    fn test_all_builders_debug() {
        let _ = format!("{:?}", UInt8BuilderWrapper::new(1));
        let _ = format!("{:?}", Int16BuilderWrapper::new(1));
        let _ = format!("{:?}", Int32BuilderWrapper::new(1));
        let _ = format!("{:?}", Int64BuilderWrapper::new(1));
        let _ = format!("{:?}", Float32BuilderWrapper::new(1));
        let _ = format!("{:?}", Float64BuilderWrapper::new(1));
    }

    #[test]
    fn test_all_builders_null_handling() {
        let mut uint8 = UInt8BuilderWrapper::new(1);
        uint8.append_null();
        assert_eq!(uint8.len(), 1);

        let mut int16 = Int16BuilderWrapper::new(1);
        int16.append_null();
        assert_eq!(int16.len(), 1);

        let mut int32 = Int32BuilderWrapper::new(1);
        int32.append_null();
        assert_eq!(int32.len(), 1);

        let mut int64 = Int64BuilderWrapper::new(1);
        int64.append_null();
        assert_eq!(int64.len(), 1);

        let mut float32 = Float32BuilderWrapper::new(1);
        float32.append_null();
        assert_eq!(float32.len(), 1);

        let mut float64 = Float64BuilderWrapper::new(1);
        float64.append_null();
        assert_eq!(float64.len(), 1);
    }
}
