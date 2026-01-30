//! String and binary type builders.
//!
//! Implements builders for:
//! - `Utf8` (VARCHAR, NVARCHAR, etc.)
//! - `LargeUtf8` (CLOB, NCLOB)
//! - `Binary` (BINARY)
//! - `LargeBinary` (BLOB)
//! - `FixedSizeBinary` (FIXED8, FIXED12, FIXED16)

use std::sync::Arc;

use arrow_array::ArrayRef;
use arrow_array::builder::{
    BinaryBuilder, FixedSizeBinaryBuilder, LargeBinaryBuilder, LargeStringBuilder, StringBuilder,
};

use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::sealed::private::Sealed;

// ═══════════════════════════════════════════════════════════════════════════
// String Builders
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for Arrow Utf8 arrays (VARCHAR, NVARCHAR).
#[derive(Debug)]
pub struct StringBuilderWrapper {
    builder: StringBuilder,
    len: usize,
}

impl StringBuilderWrapper {
    /// Create a new string builder.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of strings to pre-allocate
    /// * `data_capacity` - Bytes to pre-allocate for string data
    #[must_use]
    pub fn new(capacity: usize, data_capacity: usize) -> Self {
        Self {
            builder: StringBuilder::with_capacity(capacity, data_capacity),
            len: 0,
        }
    }

    /// Create builder with custom capacity for values and data.
    #[must_use]
    pub fn with_capacity(item_capacity: usize, data_capacity: usize) -> Self {
        Self::new(item_capacity, data_capacity)
    }

    /// Create with default capacities (1024 items, 32KB data).
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024, 32 * 1024)
    }
}

impl Sealed for StringBuilderWrapper {}

impl HanaCompatibleBuilder for StringBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::STRING(s) => {
                self.builder.append_value(s);
            }
            // Fallback: convert other types to string representation
            other => {
                self.builder.append_value(format!("{other:?}"));
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
        // StringBuilder doesn't expose capacity()
        None
    }
}

/// Builder for Arrow `LargeUtf8` arrays (CLOB, NCLOB).
///
/// Supports eager materialization of CLOB and NCLOB LOB values with optional
/// size limits to prevent OOM conditions.
#[derive(Debug)]
pub struct LargeStringBuilderWrapper {
    builder: LargeStringBuilder,
    len: usize,
    max_lob_bytes: Option<usize>,
}

impl LargeStringBuilderWrapper {
    /// Create a new large string builder.
    #[must_use]
    pub fn new(capacity: usize, data_capacity: usize) -> Self {
        Self {
            builder: LargeStringBuilder::with_capacity(capacity, data_capacity),
            len: 0,
            max_lob_bytes: None,
        }
    }

    /// Create builder with custom capacity for values and data.
    #[must_use]
    pub fn with_capacity(item_capacity: usize, data_capacity: usize) -> Self {
        Self::new(item_capacity, data_capacity)
    }

    /// Create with default capacities.
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024, 1024 * 1024) // 1MB default for LOBs
    }

    /// Set the maximum LOB size in bytes.
    ///
    /// LOB values exceeding this size will cause an error during conversion.
    #[must_use]
    pub const fn with_max_lob_bytes(mut self, max: usize) -> Self {
        self.max_lob_bytes = Some(max);
        self
    }

    /// Materialize a CLOB value with size checking.
    fn materialize_clob(&self, clob: hdbconnect::types::CLob) -> Result<String> {
        if let Some(max) = self.max_lob_bytes {
            // Intentional truncation: LOBs > usize::MAX are rejected anyway by size check
            #[allow(clippy::cast_possible_truncation)]
            let lob_size = clob.total_byte_length() as usize;
            if lob_size > max {
                return Err(crate::ArrowConversionError::lob_streaming(format!(
                    "CLOB size {lob_size} bytes exceeds max_lob_bytes limit {max} bytes",
                )));
            }
        }

        clob.into_string().map_err(|e| {
            crate::ArrowConversionError::lob_streaming(format!("CLOB read failed: {e}"))
        })
    }

    /// Materialize an NCLOB value with size checking.
    fn materialize_nclob(&self, nclob: hdbconnect::types::NCLob) -> Result<String> {
        if let Some(max) = self.max_lob_bytes {
            // Intentional truncation: LOBs > usize::MAX are rejected anyway by size check
            #[allow(clippy::cast_possible_truncation)]
            let lob_size = nclob.total_byte_length() as usize;
            if lob_size > max {
                return Err(crate::ArrowConversionError::lob_streaming(format!(
                    "NCLOB size {lob_size} bytes exceeds max_lob_bytes limit {max} bytes",
                )));
            }
        }

        nclob.into_string().map_err(|e| {
            crate::ArrowConversionError::lob_streaming(format!("NCLOB read failed: {e}"))
        })
    }
}

impl Sealed for LargeStringBuilderWrapper {}

impl HanaCompatibleBuilder for LargeStringBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::STRING(s) => {
                self.builder.append_value(s);
            }
            HdbValue::SYNC_CLOB(clob) => {
                let content = self.materialize_clob(clob.clone())?;
                self.builder.append_value(&content);
            }
            HdbValue::SYNC_NCLOB(nclob) => {
                let content = self.materialize_nclob(nclob.clone())?;
                self.builder.append_value(&content);
            }
            other => {
                return Err(crate::ArrowConversionError::value_conversion(
                    "large_string",
                    format!(
                        "cannot convert {:?} to LargeUtf8",
                        std::mem::discriminant(other)
                    ),
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
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Binary Builders
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for Arrow Binary arrays (BINARY).
#[derive(Debug)]
pub struct BinaryBuilderWrapper {
    builder: BinaryBuilder,
    len: usize,
}

impl BinaryBuilderWrapper {
    /// Create a new binary builder.
    #[must_use]
    pub fn new(capacity: usize, data_capacity: usize) -> Self {
        Self {
            builder: BinaryBuilder::with_capacity(capacity, data_capacity),
            len: 0,
        }
    }

    /// Create with default capacities.
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024, 64 * 1024) // 64KB default
    }
}

impl Sealed for BinaryBuilderWrapper {}

impl HanaCompatibleBuilder for BinaryBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        use hdbconnect::HdbValue;

        match value {
            // Binary and spatial types as WKB
            HdbValue::BINARY(bytes) | HdbValue::GEOMETRY(bytes) | HdbValue::POINT(bytes) => {
                self.builder.append_value(bytes);
            }
            other => {
                return Err(crate::ArrowConversionError::value_conversion(
                    "binary",
                    format!("cannot convert {other:?} to binary"),
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
        None
    }
}

/// Builder for Arrow `LargeBinary` arrays (BLOB).
///
/// Supports eager materialization of BLOB LOB values with optional
/// size limits to prevent OOM conditions.
#[derive(Debug)]
pub struct LargeBinaryBuilderWrapper {
    builder: LargeBinaryBuilder,
    len: usize,
    max_lob_bytes: Option<usize>,
}

impl LargeBinaryBuilderWrapper {
    /// Create a new large binary builder.
    #[must_use]
    pub fn new(capacity: usize, data_capacity: usize) -> Self {
        Self {
            builder: LargeBinaryBuilder::with_capacity(capacity, data_capacity),
            len: 0,
            max_lob_bytes: None,
        }
    }

    /// Create with default capacities.
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024, 1024 * 1024) // 1MB default for BLOBs
    }

    /// Set the maximum LOB size in bytes.
    ///
    /// LOB values exceeding this size will cause an error during conversion.
    #[must_use]
    pub const fn with_max_lob_bytes(mut self, max: usize) -> Self {
        self.max_lob_bytes = Some(max);
        self
    }

    /// Materialize a BLOB value with size checking.
    fn materialize_blob(&self, blob: hdbconnect::types::BLob) -> Result<Vec<u8>> {
        if let Some(max) = self.max_lob_bytes {
            // Intentional truncation: LOBs > usize::MAX are rejected anyway by size check
            #[allow(clippy::cast_possible_truncation)]
            let lob_size = blob.total_byte_length() as usize;
            if lob_size > max {
                return Err(crate::ArrowConversionError::lob_streaming(format!(
                    "BLOB size {lob_size} bytes exceeds max_lob_bytes limit {max} bytes",
                )));
            }
        }

        blob.into_bytes().map_err(|e| {
            crate::ArrowConversionError::lob_streaming(format!("BLOB read failed: {e}"))
        })
    }
}

impl Sealed for LargeBinaryBuilderWrapper {}

impl HanaCompatibleBuilder for LargeBinaryBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::BINARY(bytes) => {
                self.builder.append_value(bytes);
            }
            HdbValue::SYNC_BLOB(blob) => {
                let content = self.materialize_blob(blob.clone())?;
                self.builder.append_value(&content);
            }
            other => {
                return Err(crate::ArrowConversionError::value_conversion(
                    "large_binary",
                    format!(
                        "cannot convert {:?} to LargeBinary",
                        std::mem::discriminant(other)
                    ),
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
        None
    }
}

/// Builder for Arrow `FixedSizeBinary` arrays (FIXED8, FIXED12, FIXED16).
#[derive(Debug)]
pub struct FixedSizeBinaryBuilderWrapper {
    builder: FixedSizeBinaryBuilder,
    byte_width: i32,
    len: usize,
}

impl FixedSizeBinaryBuilderWrapper {
    /// Create a new fixed-size binary builder.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of fixed-size binary values to pre-allocate
    /// * `byte_width` - Size of each binary value in bytes
    #[must_use]
    pub fn new(capacity: usize, byte_width: i32) -> Self {
        Self {
            builder: FixedSizeBinaryBuilder::with_capacity(capacity, byte_width),
            byte_width,
            len: 0,
        }
    }
}

impl Sealed for FixedSizeBinaryBuilderWrapper {}

impl HanaCompatibleBuilder for FixedSizeBinaryBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::BINARY(bytes) => {
                #[allow(clippy::cast_sign_loss)]
                if bytes.len() != self.byte_width as usize {
                    return Err(crate::ArrowConversionError::value_conversion(
                        "fixed_size_binary",
                        format!("expected {} bytes, got {}", self.byte_width, bytes.len()),
                    ));
                }
                self.builder.append_value(bytes).map_err(|e| {
                    crate::ArrowConversionError::value_conversion(
                        "fixed_size_binary",
                        e.to_string(),
                    )
                })?;
            }
            other => {
                return Err(crate::ArrowConversionError::value_conversion(
                    "fixed_size_binary",
                    format!("cannot convert {other:?} to fixed-size binary"),
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
        None
    }
}

#[cfg(test)]
mod tests {
    use arrow_array::{
        Array, BinaryArray, FixedSizeBinaryArray, LargeBinaryArray, LargeStringArray, StringArray,
    };
    use hdbconnect::HdbValue;

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // StringBuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_string_builder_new() {
        let builder = StringBuilderWrapper::new(10, 100);
        assert_eq!(builder.len(), 0);
        assert!(builder.capacity().is_none());
    }

    #[test]
    fn test_string_builder_with_capacity() {
        let builder = StringBuilderWrapper::with_capacity(10, 100);
        assert_eq!(builder.len(), 0);
        assert!(builder.capacity().is_none());
    }

    #[test]
    fn test_string_builder_default_capacity() {
        let builder = StringBuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_string_builder_append_string() {
        let mut builder = StringBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::STRING("hello".to_string()))
            .unwrap();
        assert_eq!(builder.len(), 1);
    }

    #[test]
    fn test_string_builder_append_null() {
        let mut builder = StringBuilderWrapper::new(10, 100);
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let string_array = array.as_any().downcast_ref::<StringArray>().unwrap();
        assert!(string_array.is_null(0));
    }

    #[test]
    fn test_string_builder_append_non_string_type() {
        let mut builder = StringBuilderWrapper::new(10, 100);
        builder.append_hana_value(&HdbValue::INT(42)).unwrap();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let string_array = array.as_any().downcast_ref::<StringArray>().unwrap();
        assert!(string_array.value(0).contains("INT"));
    }

    #[test]
    fn test_string_builder_finish_and_reuse() {
        let mut builder = StringBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::STRING("first".to_string()))
            .unwrap();
        let _array1 = builder.finish();
        assert_eq!(builder.len(), 0);

        builder
            .append_hana_value(&HdbValue::STRING("second".to_string()))
            .unwrap();
        let array2 = builder.finish();
        assert_eq!(array2.len(), 1);
    }

    #[test]
    fn test_string_builder_empty_string() {
        let mut builder = StringBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::STRING(String::new()))
            .unwrap();

        let array = builder.finish();
        let string_array = array.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(string_array.value(0), "");
    }

    #[test]
    fn test_string_builder_unicode() {
        let mut builder = StringBuilderWrapper::new(10, 1000);
        builder
            .append_hana_value(&HdbValue::STRING("日本語テスト".to_string()))
            .unwrap();
        builder
            .append_hana_value(&HdbValue::STRING("émojis: 🚀🎉".to_string()))
            .unwrap();

        let array = builder.finish();
        let string_array = array.as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(string_array.value(0), "日本語テスト");
        assert_eq!(string_array.value(1), "émojis: 🚀🎉");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LargeStringBuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_large_string_builder_new() {
        let builder = LargeStringBuilderWrapper::new(10, 1000);
        assert_eq!(builder.len(), 0);
        assert!(builder.max_lob_bytes.is_none());
    }

    #[test]
    fn test_large_string_builder_with_capacity() {
        let builder = LargeStringBuilderWrapper::with_capacity(10, 1000);
        assert_eq!(builder.len(), 0);
        assert!(builder.max_lob_bytes.is_none());
    }

    #[test]
    fn test_large_string_builder_default_capacity() {
        let builder = LargeStringBuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_large_string_builder_with_max_lob_bytes() {
        let builder = LargeStringBuilderWrapper::new(10, 1000).with_max_lob_bytes(1_000_000);
        assert_eq!(builder.max_lob_bytes, Some(1_000_000));
    }

    #[test]
    fn test_large_string_builder_append_string() {
        let mut builder = LargeStringBuilderWrapper::new(10, 1000);
        builder
            .append_hana_value(&HdbValue::STRING("large text".to_string()))
            .unwrap();

        let array = builder.finish();
        let large_string_array = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
        assert_eq!(large_string_array.value(0), "large text");
    }

    #[test]
    fn test_large_string_builder_append_null() {
        let mut builder = LargeStringBuilderWrapper::new(10, 1000);
        builder.append_null();

        let array = builder.finish();
        let large_string_array = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
        assert!(large_string_array.is_null(0));
    }

    #[test]
    fn test_large_string_builder_reject_non_string_type() {
        let mut builder = LargeStringBuilderWrapper::new(10, 1000);
        let result = builder.append_hana_value(&HdbValue::BIGINT(123456789));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_value_conversion());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BinaryBuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_binary_builder_new() {
        let builder = BinaryBuilderWrapper::new(10, 100);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_binary_builder_default_capacity() {
        let builder = BinaryBuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_binary_builder_append_binary() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3]))
            .unwrap();

        let array = builder.finish();
        let binary_array = array.as_any().downcast_ref::<BinaryArray>().unwrap();
        assert_eq!(binary_array.value(0), &[1, 2, 3]);
    }

    #[test]
    fn test_binary_builder_append_null() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);
        builder.append_null();

        let array = builder.finish();
        let binary_array = array.as_any().downcast_ref::<BinaryArray>().unwrap();
        assert!(binary_array.is_null(0));
    }

    #[test]
    fn test_binary_builder_append_geometry() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::GEOMETRY(vec![0x00, 0x01, 0x02]))
            .unwrap();

        let array = builder.finish();
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn test_binary_builder_append_point() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::POINT(vec![0xAB, 0xCD]))
            .unwrap();

        let array = builder.finish();
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn test_binary_builder_reject_string() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);
        let result = builder.append_hana_value(&HdbValue::STRING("text".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    #[test]
    fn test_binary_builder_empty_binary() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);
        builder
            .append_hana_value(&HdbValue::BINARY(vec![]))
            .unwrap();

        let array = builder.finish();
        let binary_array = array.as_any().downcast_ref::<BinaryArray>().unwrap();
        assert!(binary_array.value(0).is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LargeBinaryBuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_large_binary_builder_new() {
        let builder = LargeBinaryBuilderWrapper::new(10, 1000);
        assert_eq!(builder.len(), 0);
        assert!(builder.max_lob_bytes.is_none());
    }

    #[test]
    fn test_large_binary_builder_default_capacity() {
        let builder = LargeBinaryBuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_large_binary_builder_with_max_lob_bytes() {
        let builder = LargeBinaryBuilderWrapper::new(10, 1000).with_max_lob_bytes(1_000_000);
        assert_eq!(builder.max_lob_bytes, Some(1_000_000));
    }

    #[test]
    fn test_large_binary_builder_append_binary() {
        let mut builder = LargeBinaryBuilderWrapper::new(10, 1000);
        builder
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3, 4, 5]))
            .unwrap();

        let array = builder.finish();
        let large_binary_array = array.as_any().downcast_ref::<LargeBinaryArray>().unwrap();
        assert_eq!(large_binary_array.value(0), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_large_binary_builder_append_null() {
        let mut builder = LargeBinaryBuilderWrapper::new(10, 1000);
        builder.append_null();

        let array = builder.finish();
        let large_binary_array = array.as_any().downcast_ref::<LargeBinaryArray>().unwrap();
        assert!(large_binary_array.is_null(0));
    }

    #[test]
    fn test_large_binary_builder_reject_string() {
        let mut builder = LargeBinaryBuilderWrapper::new(10, 1000);
        let result = builder.append_hana_value(&HdbValue::STRING("text".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // FixedSizeBinaryBuilderWrapper Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_fixed_size_binary_builder_new() {
        let builder = FixedSizeBinaryBuilderWrapper::new(10, 4);
        assert_eq!(builder.len(), 0);
        assert_eq!(builder.byte_width, 4);
    }

    #[test]
    fn test_fixed_size_binary_builder_correct_size() {
        let mut builder = FixedSizeBinaryBuilderWrapper::new(10, 4);
        builder
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3, 4]))
            .unwrap();

        let array = builder.finish();
        let fixed_binary = array
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        assert_eq!(fixed_binary.value(0), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_fixed_size_binary_builder_wrong_size_smaller() {
        let mut builder = FixedSizeBinaryBuilderWrapper::new(10, 4);
        let result = builder.append_hana_value(&HdbValue::BINARY(vec![1, 2]));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_value_conversion());
        assert!(err.to_string().contains("expected 4 bytes"));
    }

    #[test]
    fn test_fixed_size_binary_builder_wrong_size_larger() {
        let mut builder = FixedSizeBinaryBuilderWrapper::new(10, 4);
        let result = builder.append_hana_value(&HdbValue::BINARY(vec![1, 2, 3, 4, 5, 6]));
        assert!(result.is_err());
    }

    #[test]
    fn test_fixed_size_binary_builder_append_null() {
        let mut builder = FixedSizeBinaryBuilderWrapper::new(10, 4);
        builder.append_null();

        let array = builder.finish();
        let fixed_binary = array
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        assert!(fixed_binary.is_null(0));
    }

    #[test]
    fn test_fixed_size_binary_builder_reject_non_binary() {
        let mut builder = FixedSizeBinaryBuilderWrapper::new(10, 4);
        let result = builder.append_hana_value(&HdbValue::INT(42));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    #[test]
    fn test_fixed_size_binary_builder_different_widths() {
        // Test with byte_width = 8
        let mut builder8 = FixedSizeBinaryBuilderWrapper::new(10, 8);
        builder8
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3, 4, 5, 6, 7, 8]))
            .unwrap();
        assert_eq!(builder8.len(), 1);

        // Test with byte_width = 16
        let mut builder16 = FixedSizeBinaryBuilderWrapper::new(10, 16);
        builder16
            .append_hana_value(&HdbValue::BINARY(vec![0; 16]))
            .unwrap();
        assert_eq!(builder16.len(), 1);
    }
}
