//! String and binary type builders.
//!
//! Implements builders for:
//! - `Utf8` (VARCHAR, NVARCHAR, etc.)
//! - `LargeUtf8` (CLOB, NCLOB)
//! - `Binary` (BINARY)
//! - `LargeBinary` (BLOB)
//! - `FixedSizeBinary` (FIXED8, FIXED12, FIXED16)

use arrow_array::ArrayRef;
use arrow_array::builder::{
    BinaryBuilder, FixedSizeBinaryBuilder, LargeBinaryBuilder, LargeStringBuilder, StringBuilder,
};
use std::sync::Arc;

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
#[derive(Debug)]
pub struct LargeStringBuilderWrapper {
    builder: LargeStringBuilder,
    len: usize,
}

impl LargeStringBuilderWrapper {
    /// Create a new large string builder.
    #[must_use]
    pub fn new(capacity: usize, data_capacity: usize) -> Self {
        Self {
            builder: LargeStringBuilder::with_capacity(capacity, data_capacity),
            len: 0,
        }
    }

    /// Create with default capacities.
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024, 1024 * 1024) // 1MB default for LOBs
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
            // LOBs are handled differently in hdbconnect 0.32+
            // They require async reading which is beyond this sync builder
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
#[derive(Debug)]
pub struct LargeBinaryBuilderWrapper {
    builder: LargeBinaryBuilder,
    len: usize,
}

impl LargeBinaryBuilderWrapper {
    /// Create a new large binary builder.
    #[must_use]
    pub fn new(capacity: usize, data_capacity: usize) -> Self {
        Self {
            builder: LargeBinaryBuilder::with_capacity(capacity, data_capacity),
            len: 0,
        }
    }

    /// Create with default capacities.
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024, 1024 * 1024) // 1MB default for BLOBs
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
            // BLOBs are handled differently in hdbconnect 0.32+
            other => {
                return Err(crate::ArrowConversionError::value_conversion(
                    "large_binary",
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
    use super::*;
    use hdbconnect::HdbValue;

    #[test]
    fn test_string_builder() {
        let mut builder = StringBuilderWrapper::new(10, 100);

        builder
            .append_hana_value(&HdbValue::STRING("hello".to_string()))
            .unwrap();
        builder.append_null();
        builder
            .append_hana_value(&HdbValue::STRING("world".to_string()))
            .unwrap();

        assert_eq!(builder.len(), 3);
        let array = builder.finish();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_binary_builder() {
        let mut builder = BinaryBuilderWrapper::new(10, 100);

        builder
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3]))
            .unwrap();
        builder.append_null();

        let array = builder.finish();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_fixed_size_binary_builder() {
        let mut builder = FixedSizeBinaryBuilderWrapper::new(10, 4);

        builder
            .append_hana_value(&HdbValue::BINARY(vec![1, 2, 3, 4]))
            .unwrap();

        // Wrong size should error
        let result = builder.append_hana_value(&HdbValue::BINARY(vec![1, 2]));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }
}
