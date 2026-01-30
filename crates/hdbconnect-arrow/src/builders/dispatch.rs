//! Enum-based builder dispatch for eliminating dynamic dispatch overhead.
//!
//! This module provides `BuilderEnum` which wraps all concrete builder types
//! in a single enum, enabling monomorphized dispatch instead of vtable lookups.
//!
//! Large variants (string, binary, decimal) are wrapped in `Box` to reduce
//! enum size and improve cache locality for temporal/primitive builders.

use arrow_array::ArrayRef;

use super::boolean::BooleanBuilderWrapper;
use super::decimal::Decimal128BuilderWrapper;
use super::primitive::{
    Float32BuilderWrapper, Float64BuilderWrapper, Int16BuilderWrapper, Int32BuilderWrapper,
    Int64BuilderWrapper, UInt8BuilderWrapper,
};
use super::string::{
    BinaryBuilderWrapper, FixedSizeBinaryBuilderWrapper, LargeBinaryBuilderWrapper,
    LargeStringBuilderWrapper, StringBuilderWrapper,
};
use super::temporal::{
    Date32BuilderWrapper, Time64NanosecondBuilderWrapper, TimestampNanosecondBuilderWrapper,
};
use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;

/// Discriminant identifying the builder type without data.
///
/// Used for schema profile analysis to detect homogeneous schemas
/// where all columns share the same builder type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderKind {
    /// `UInt8` builder (HANA TINYINT)
    UInt8,
    /// `Int16` builder (HANA SMALLINT)
    Int16,
    /// `Int32` builder (HANA INT)
    Int32,
    /// `Int64` builder (HANA BIGINT)
    Int64,
    /// `Float32` builder (HANA REAL)
    Float32,
    /// `Float64` builder (HANA DOUBLE)
    Float64,
    /// `Decimal128` builder (HANA DECIMAL)
    Decimal128,
    /// `Boolean` builder (HANA BOOLEAN)
    Boolean,
    /// `Utf8` builder (HANA VARCHAR, NVARCHAR)
    Utf8,
    /// `LargeUtf8` builder (HANA CLOB, NCLOB)
    LargeUtf8,
    /// `Binary` builder (HANA BINARY)
    Binary,
    /// `LargeBinary` builder (HANA BLOB)
    LargeBinary,
    /// `FixedSizeBinary` builder (HANA FIXED8, FIXED12, FIXED16)
    FixedSizeBinary,
    /// `Date32` builder (HANA DAYDATE)
    Date32,
    /// `Time64Nanosecond` builder (HANA SECONDTIME)
    Time64Nanosecond,
    /// `TimestampNanosecond` builder (HANA LONGDATE, SECONDDATE)
    TimestampNanosecond,
}

/// Enum-wrapped builder for static dispatch.
///
/// Wraps all concrete builder types in a single enum, eliminating vtable
/// lookups and enabling the compiler to inline and optimize dispatch.
///
/// Large variants (string, binary, decimal) are wrapped in `Box` to reduce
/// enum size and improve cache locality for temporal/primitive builders.
///
/// For homogeneous schemas (all columns same type), the match can be hoisted
/// outside the row loop for monomorphized inner loops.
#[derive(Debug)]
pub enum BuilderEnum {
    // ═══════════════════════════════════════════════════════════════════════════
    // Inline small variants (primitive and temporal types)
    // ═══════════════════════════════════════════════════════════════════════════
    /// `UInt8` builder variant
    UInt8(UInt8BuilderWrapper),
    /// `Int16` builder variant
    Int16(Int16BuilderWrapper),
    /// `Int32` builder variant
    Int32(Int32BuilderWrapper),
    /// `Int64` builder variant
    Int64(Int64BuilderWrapper),
    /// `Float32` builder variant
    Float32(Float32BuilderWrapper),
    /// `Float64` builder variant
    Float64(Float64BuilderWrapper),
    /// `Boolean` builder variant
    Boolean(BooleanBuilderWrapper),
    /// `Date32` builder variant
    Date32(Date32BuilderWrapper),
    /// `Time64Nanosecond` builder variant
    Time64Nanosecond(Time64NanosecondBuilderWrapper),
    /// `TimestampNanosecond` builder variant
    TimestampNanosecond(TimestampNanosecondBuilderWrapper),

    // ═══════════════════════════════════════════════════════════════════════════
    // Boxed large variants (string, binary, decimal - larger inner size)
    // ═══════════════════════════════════════════════════════════════════════════
    /// `Decimal128` builder variant (boxed)
    Decimal128(Box<Decimal128BuilderWrapper>),
    /// `Utf8` builder variant (boxed)
    Utf8(Box<StringBuilderWrapper>),
    /// `LargeUtf8` builder variant (boxed)
    LargeUtf8(Box<LargeStringBuilderWrapper>),
    /// `Binary` builder variant (boxed)
    Binary(Box<BinaryBuilderWrapper>),
    /// `LargeBinary` builder variant (boxed)
    LargeBinary(Box<LargeBinaryBuilderWrapper>),
    /// `FixedSizeBinary` builder variant (boxed)
    FixedSizeBinary(Box<FixedSizeBinaryBuilderWrapper>),
}

impl BuilderEnum {
    /// Returns the kind of this builder (discriminant only).
    #[must_use]
    pub const fn kind(&self) -> BuilderKind {
        match self {
            Self::UInt8(_) => BuilderKind::UInt8,
            Self::Int16(_) => BuilderKind::Int16,
            Self::Int32(_) => BuilderKind::Int32,
            Self::Int64(_) => BuilderKind::Int64,
            Self::Float32(_) => BuilderKind::Float32,
            Self::Float64(_) => BuilderKind::Float64,
            Self::Decimal128(_) => BuilderKind::Decimal128,
            Self::Boolean(_) => BuilderKind::Boolean,
            Self::Utf8(_) => BuilderKind::Utf8,
            Self::LargeUtf8(_) => BuilderKind::LargeUtf8,
            Self::Binary(_) => BuilderKind::Binary,
            Self::LargeBinary(_) => BuilderKind::LargeBinary,
            Self::FixedSizeBinary(_) => BuilderKind::FixedSizeBinary,
            Self::Date32(_) => BuilderKind::Date32,
            Self::Time64Nanosecond(_) => BuilderKind::Time64Nanosecond,
            Self::TimestampNanosecond(_) => BuilderKind::TimestampNanosecond,
        }
    }

    /// Append a HANA value to this builder.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to the target type.
    #[inline]
    pub fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        match self {
            Self::UInt8(b) => b.append_hana_value(value),
            Self::Int16(b) => b.append_hana_value(value),
            Self::Int32(b) => b.append_hana_value(value),
            Self::Int64(b) => b.append_hana_value(value),
            Self::Float32(b) => b.append_hana_value(value),
            Self::Float64(b) => b.append_hana_value(value),
            Self::Decimal128(b) => b.append_hana_value(value),
            Self::Boolean(b) => b.append_hana_value(value),
            Self::Utf8(b) => b.append_hana_value(value),
            Self::LargeUtf8(b) => b.append_hana_value(value),
            Self::Binary(b) => b.append_hana_value(value),
            Self::LargeBinary(b) => b.append_hana_value(value),
            Self::FixedSizeBinary(b) => b.append_hana_value(value),
            Self::Date32(b) => b.append_hana_value(value),
            Self::Time64Nanosecond(b) => b.append_hana_value(value),
            Self::TimestampNanosecond(b) => b.append_hana_value(value),
        }
    }

    /// Append a null value to this builder.
    #[inline]
    pub fn append_null(&mut self) {
        match self {
            Self::UInt8(b) => b.append_null(),
            Self::Int16(b) => b.append_null(),
            Self::Int32(b) => b.append_null(),
            Self::Int64(b) => b.append_null(),
            Self::Float32(b) => b.append_null(),
            Self::Float64(b) => b.append_null(),
            Self::Decimal128(b) => b.append_null(),
            Self::Boolean(b) => b.append_null(),
            Self::Utf8(b) => b.append_null(),
            Self::LargeUtf8(b) => b.append_null(),
            Self::Binary(b) => b.append_null(),
            Self::LargeBinary(b) => b.append_null(),
            Self::FixedSizeBinary(b) => b.append_null(),
            Self::Date32(b) => b.append_null(),
            Self::Time64Nanosecond(b) => b.append_null(),
            Self::TimestampNanosecond(b) => b.append_null(),
        }
    }

    /// Finish building and return the Arrow array.
    ///
    /// After calling this method, the builder is reset and can be reused.
    pub fn finish(&mut self) -> ArrayRef {
        match self {
            Self::UInt8(b) => b.finish(),
            Self::Int16(b) => b.finish(),
            Self::Int32(b) => b.finish(),
            Self::Int64(b) => b.finish(),
            Self::Float32(b) => b.finish(),
            Self::Float64(b) => b.finish(),
            Self::Decimal128(b) => b.finish(),
            Self::Boolean(b) => b.finish(),
            Self::Utf8(b) => b.finish(),
            Self::LargeUtf8(b) => b.finish(),
            Self::Binary(b) => b.finish(),
            Self::LargeBinary(b) => b.finish(),
            Self::FixedSizeBinary(b) => b.finish(),
            Self::Date32(b) => b.finish(),
            Self::Time64Nanosecond(b) => b.finish(),
            Self::TimestampNanosecond(b) => b.finish(),
        }
    }

    /// Returns the number of values (including nulls) appended so far.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::UInt8(b) => b.len(),
            Self::Int16(b) => b.len(),
            Self::Int32(b) => b.len(),
            Self::Int64(b) => b.len(),
            Self::Float32(b) => b.len(),
            Self::Float64(b) => b.len(),
            Self::Decimal128(b) => b.len(),
            Self::Boolean(b) => b.len(),
            Self::Utf8(b) => b.len(),
            Self::LargeUtf8(b) => b.len(),
            Self::Binary(b) => b.len(),
            Self::LargeBinary(b) => b.len(),
            Self::FixedSizeBinary(b) => b.len(),
            Self::Date32(b) => b.len(),
            Self::Time64Nanosecond(b) => b.len(),
            Self::TimestampNanosecond(b) => b.len(),
        }
    }

    /// Returns true if no values have been appended.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use arrow_array::{
        Array, BinaryArray, BooleanArray, Date32Array, Float32Array, Float64Array, Int16Array,
        Int32Array, Int64Array, LargeBinaryArray, LargeStringArray, StringArray,
        Time64NanosecondArray, TimestampNanosecondArray, UInt8Array,
    };
    use hdbconnect::HdbValue;

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // BuilderEnum Size Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_boxed_variants_smaller_than_inline() {
        // Verify that the boxed variants would be larger if inline
        let string_wrapper_size = size_of::<StringBuilderWrapper>();
        let decimal_wrapper_size = size_of::<Decimal128BuilderWrapper>();
        let box_size = size_of::<Box<StringBuilderWrapper>>();

        assert!(
            string_wrapper_size > box_size,
            "StringBuilderWrapper ({string_wrapper_size}) should be larger than Box ({box_size})"
        );
        assert!(
            decimal_wrapper_size > box_size,
            "Decimal128BuilderWrapper ({decimal_wrapper_size}) should be larger than Box ({box_size})"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BuilderKind Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_kind_equality() {
        assert_eq!(BuilderKind::Int32, BuilderKind::Int32);
        assert_ne!(BuilderKind::Int32, BuilderKind::Int64);
    }

    #[test]
    fn test_builder_kind_copy() {
        let kind = BuilderKind::Utf8;
        let kind2 = kind;
        assert_eq!(kind, kind2);
    }

    #[test]
    fn test_builder_kind_debug() {
        let kind = BuilderKind::Decimal128;
        let debug = format!("{kind:?}");
        assert!(debug.contains("Decimal128"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BuilderEnum Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_enum_uint8() {
        let builder = BuilderEnum::UInt8(UInt8BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::UInt8);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_builder_enum_int16() {
        let builder = BuilderEnum::Int16(Int16BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Int16);
    }

    #[test]
    fn test_builder_enum_int32() {
        let builder = BuilderEnum::Int32(Int32BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Int32);
    }

    #[test]
    fn test_builder_enum_int64() {
        let builder = BuilderEnum::Int64(Int64BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Int64);
    }

    #[test]
    fn test_builder_enum_float32() {
        let builder = BuilderEnum::Float32(Float32BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Float32);
    }

    #[test]
    fn test_builder_enum_float64() {
        let builder = BuilderEnum::Float64(Float64BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Float64);
    }

    #[test]
    fn test_builder_enum_decimal128() {
        let builder = BuilderEnum::Decimal128(Box::new(Decimal128BuilderWrapper::new(10, 18, 2)));
        assert_eq!(builder.kind(), BuilderKind::Decimal128);
    }

    #[test]
    fn test_builder_enum_boolean() {
        let builder = BuilderEnum::Boolean(BooleanBuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Boolean);
    }

    #[test]
    fn test_builder_enum_utf8() {
        let builder = BuilderEnum::Utf8(Box::new(StringBuilderWrapper::new(10, 100)));
        assert_eq!(builder.kind(), BuilderKind::Utf8);
    }

    #[test]
    fn test_builder_enum_large_utf8() {
        let builder = BuilderEnum::LargeUtf8(Box::new(LargeStringBuilderWrapper::new(10, 1000)));
        assert_eq!(builder.kind(), BuilderKind::LargeUtf8);
    }

    #[test]
    fn test_builder_enum_binary() {
        let builder = BuilderEnum::Binary(Box::new(BinaryBuilderWrapper::new(10, 100)));
        assert_eq!(builder.kind(), BuilderKind::Binary);
    }

    #[test]
    fn test_builder_enum_large_binary() {
        let builder = BuilderEnum::LargeBinary(Box::new(LargeBinaryBuilderWrapper::new(10, 1000)));
        assert_eq!(builder.kind(), BuilderKind::LargeBinary);
    }

    #[test]
    fn test_builder_enum_fixed_size_binary() {
        let builder =
            BuilderEnum::FixedSizeBinary(Box::new(FixedSizeBinaryBuilderWrapper::new(10, 8)));
        assert_eq!(builder.kind(), BuilderKind::FixedSizeBinary);
    }

    #[test]
    fn test_builder_enum_date32() {
        let builder = BuilderEnum::Date32(Date32BuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Date32);
    }

    #[test]
    fn test_builder_enum_time64() {
        let builder = BuilderEnum::Time64Nanosecond(Time64NanosecondBuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::Time64Nanosecond);
    }

    #[test]
    fn test_builder_enum_timestamp() {
        let builder = BuilderEnum::TimestampNanosecond(TimestampNanosecondBuilderWrapper::new(10));
        assert_eq!(builder.kind(), BuilderKind::TimestampNanosecond);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BuilderEnum Append and Finish Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_enum_uint8_append() {
        let mut builder = BuilderEnum::UInt8(UInt8BuilderWrapper::new(10));
        builder.append_hana_value(&HdbValue::TINYINT(42)).unwrap();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        let array = builder.finish();
        let uint_array = array.as_any().downcast_ref::<UInt8Array>().unwrap();
        assert_eq!(uint_array.value(0), 42);
        assert!(uint_array.is_null(1));
    }

    #[test]
    fn test_builder_enum_int16_append() {
        let mut builder = BuilderEnum::Int16(Int16BuilderWrapper::new(10));
        builder
            .append_hana_value(&HdbValue::SMALLINT(1000))
            .unwrap();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int16Array>().unwrap();
        assert_eq!(int_array.value(0), 1000);
    }

    #[test]
    fn test_builder_enum_int32_append() {
        let mut builder = BuilderEnum::Int32(Int32BuilderWrapper::new(10));
        builder.append_hana_value(&HdbValue::INT(12345)).unwrap();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(int_array.value(0), 12345);
        assert!(int_array.is_null(1));
    }

    #[test]
    fn test_builder_enum_int64_append() {
        let mut builder = BuilderEnum::Int64(Int64BuilderWrapper::new(10));
        builder
            .append_hana_value(&HdbValue::BIGINT(i64::MAX))
            .unwrap();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let int_array = array.as_any().downcast_ref::<Int64Array>().unwrap();
        assert_eq!(int_array.value(0), i64::MAX);
    }

    #[test]
    fn test_builder_enum_float32_append() {
        let mut builder = BuilderEnum::Float32(Float32BuilderWrapper::new(10));
        builder.append_hana_value(&HdbValue::REAL(3.14)).unwrap();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let float_array = array.as_any().downcast_ref::<Float32Array>().unwrap();
        assert!((float_array.value(0) - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_builder_enum_float64_append() {
        let mut builder = BuilderEnum::Float64(Float64BuilderWrapper::new(10));
        builder.append_hana_value(&HdbValue::DOUBLE(2.718)).unwrap();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        let array = builder.finish();
        let float_array = array.as_any().downcast_ref::<Float64Array>().unwrap();
        assert!((float_array.value(0) - 2.718).abs() < 0.0001);
        assert!(float_array.is_null(1));
    }

    #[test]
    fn test_builder_enum_boolean_append() {
        let mut builder = BuilderEnum::Boolean(BooleanBuilderWrapper::new(10));
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();
        builder.append_null();
        assert_eq!(builder.len(), 3);

        let array = builder.finish();
        let bool_array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(bool_array.value(0));
        assert!(!bool_array.value(1));
        assert!(bool_array.is_null(2));
    }

    #[test]
    fn test_builder_enum_utf8_append() {
        let mut builder = BuilderEnum::Utf8(Box::new(StringBuilderWrapper::new(10, 100)));
        builder
            .append_hana_value(&HdbValue::STRING("hello".to_string()))
            .unwrap();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        let array = builder.finish();
        let str_array = array.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(str_array.value(0), "hello");
        assert!(str_array.is_null(1));
    }

    #[test]
    fn test_builder_enum_large_utf8_append() {
        let mut builder =
            BuilderEnum::LargeUtf8(Box::new(LargeStringBuilderWrapper::new(10, 1000)));
        builder
            .append_hana_value(&HdbValue::STRING("large text".to_string()))
            .unwrap();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let str_array = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
        assert_eq!(str_array.value(0), "large text");
    }

    #[test]
    fn test_builder_enum_binary_append() {
        let mut builder = BuilderEnum::Binary(Box::new(BinaryBuilderWrapper::new(10, 100)));
        builder
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3]))
            .unwrap();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        let array = builder.finish();
        let bin_array = array.as_any().downcast_ref::<BinaryArray>().unwrap();
        assert_eq!(bin_array.value(0), &[1, 2, 3]);
        assert!(bin_array.is_null(1));
    }

    #[test]
    fn test_builder_enum_large_binary_append() {
        let mut builder =
            BuilderEnum::LargeBinary(Box::new(LargeBinaryBuilderWrapper::new(10, 1000)));
        builder
            .append_hana_value(&HdbValue::BINARY(vec![4, 5, 6, 7]))
            .unwrap();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let bin_array = array.as_any().downcast_ref::<LargeBinaryArray>().unwrap();
        assert_eq!(bin_array.value(0), &[4, 5, 6, 7]);
    }

    #[test]
    fn test_builder_enum_date32_append_null() {
        let mut builder = BuilderEnum::Date32(Date32BuilderWrapper::new(10));
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let date_array = array.as_any().downcast_ref::<Date32Array>().unwrap();
        assert!(date_array.is_null(0));
    }

    #[test]
    fn test_builder_enum_time64_append_null() {
        let mut builder = BuilderEnum::Time64Nanosecond(Time64NanosecondBuilderWrapper::new(10));
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let time_array = array
            .as_any()
            .downcast_ref::<Time64NanosecondArray>()
            .unwrap();
        assert!(time_array.is_null(0));
    }

    #[test]
    fn test_builder_enum_timestamp_append_null() {
        let mut builder =
            BuilderEnum::TimestampNanosecond(TimestampNanosecondBuilderWrapper::new(10));
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let ts_array = array
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap();
        assert!(ts_array.is_null(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BuilderEnum Error Handling Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_enum_type_mismatch() {
        let mut builder = BuilderEnum::Int32(Int32BuilderWrapper::new(10));
        let result = builder.append_hana_value(&HdbValue::STRING("hello".to_string()));
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BuilderEnum Reuse Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_enum_reuse_after_finish() {
        let mut builder = BuilderEnum::Int32(Int32BuilderWrapper::new(10));
        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        let array1 = builder.finish();
        assert_eq!(array1.len(), 1);
        assert!(builder.is_empty());

        builder.append_hana_value(&HdbValue::INT(2)).unwrap();
        builder.append_hana_value(&HdbValue::INT(3)).unwrap();
        let array2 = builder.finish();
        assert_eq!(array2.len(), 2);

        let int_array = array2.as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(int_array.value(0), 2);
        assert_eq!(int_array.value(1), 3);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // All 16 Variants Test
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_all_16_builder_kinds() {
        let kinds = [
            BuilderKind::UInt8,
            BuilderKind::Int16,
            BuilderKind::Int32,
            BuilderKind::Int64,
            BuilderKind::Float32,
            BuilderKind::Float64,
            BuilderKind::Decimal128,
            BuilderKind::Boolean,
            BuilderKind::Utf8,
            BuilderKind::LargeUtf8,
            BuilderKind::Binary,
            BuilderKind::LargeBinary,
            BuilderKind::FixedSizeBinary,
            BuilderKind::Date32,
            BuilderKind::Time64Nanosecond,
            BuilderKind::TimestampNanosecond,
        ];
        assert_eq!(kinds.len(), 16);

        for (i, kind) in kinds.iter().enumerate() {
            for (j, other) in kinds.iter().enumerate() {
                if i == j {
                    assert_eq!(kind, other);
                } else {
                    assert_ne!(kind, other);
                }
            }
        }
    }

    #[test]
    fn test_all_16_builder_enum_variants() {
        let builders: Vec<BuilderEnum> = vec![
            BuilderEnum::UInt8(UInt8BuilderWrapper::new(1)),
            BuilderEnum::Int16(Int16BuilderWrapper::new(1)),
            BuilderEnum::Int32(Int32BuilderWrapper::new(1)),
            BuilderEnum::Int64(Int64BuilderWrapper::new(1)),
            BuilderEnum::Float32(Float32BuilderWrapper::new(1)),
            BuilderEnum::Float64(Float64BuilderWrapper::new(1)),
            BuilderEnum::Decimal128(Box::new(Decimal128BuilderWrapper::new(1, 18, 2))),
            BuilderEnum::Boolean(BooleanBuilderWrapper::new(1)),
            BuilderEnum::Utf8(Box::new(StringBuilderWrapper::new(1, 10))),
            BuilderEnum::LargeUtf8(Box::new(LargeStringBuilderWrapper::new(1, 100))),
            BuilderEnum::Binary(Box::new(BinaryBuilderWrapper::new(1, 10))),
            BuilderEnum::LargeBinary(Box::new(LargeBinaryBuilderWrapper::new(1, 100))),
            BuilderEnum::FixedSizeBinary(Box::new(FixedSizeBinaryBuilderWrapper::new(1, 8))),
            BuilderEnum::Date32(Date32BuilderWrapper::new(1)),
            BuilderEnum::Time64Nanosecond(Time64NanosecondBuilderWrapper::new(1)),
            BuilderEnum::TimestampNanosecond(TimestampNanosecondBuilderWrapper::new(1)),
        ];
        assert_eq!(builders.len(), 16);

        let expected_kinds = [
            BuilderKind::UInt8,
            BuilderKind::Int16,
            BuilderKind::Int32,
            BuilderKind::Int64,
            BuilderKind::Float32,
            BuilderKind::Float64,
            BuilderKind::Decimal128,
            BuilderKind::Boolean,
            BuilderKind::Utf8,
            BuilderKind::LargeUtf8,
            BuilderKind::Binary,
            BuilderKind::LargeBinary,
            BuilderKind::FixedSizeBinary,
            BuilderKind::Date32,
            BuilderKind::Time64Nanosecond,
            BuilderKind::TimestampNanosecond,
        ];

        for (builder, expected) in builders.iter().zip(expected_kinds.iter()) {
            assert_eq!(builder.kind(), *expected);
            assert!(builder.is_empty());
        }
    }
}
