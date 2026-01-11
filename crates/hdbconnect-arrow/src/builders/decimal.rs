//! Decimal128 builder with precision and scale validation.
//!
//! Handles HANA DECIMAL and SMALLDECIMAL types with proper precision/scale
//! preservation using Arrow Decimal128 arrays.

use std::sync::Arc;

use arrow_array::ArrayRef;
use arrow_array::builder::Decimal128Builder;

use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::sealed::private::Sealed;
use crate::types::hana::{DecimalPrecision, DecimalScale};

/// Validated decimal configuration.
///
/// Ensures precision and scale are valid at construction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalConfig {
    precision: DecimalPrecision,
    scale: DecimalScale,
}

impl DecimalConfig {
    /// Create a new decimal configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if precision or scale are invalid.
    pub fn new(precision: u8, scale: i8) -> Result<Self> {
        let prec = DecimalPrecision::new(precision)?;
        let scl = DecimalScale::new(scale, prec)?;
        Ok(Self {
            precision: prec,
            scale: scl,
        })
    }

    /// Returns the precision value.
    #[must_use]
    pub const fn precision(&self) -> u8 {
        self.precision.value()
    }

    /// Returns the scale value.
    #[must_use]
    pub const fn scale(&self) -> i8 {
        self.scale.value()
    }
}

/// Builder for Arrow Decimal128 arrays.
///
/// Maintains precision and scale configuration for proper HANA DECIMAL handling.
#[derive(Debug)]
pub struct Decimal128BuilderWrapper {
    builder: Decimal128Builder,
    config: DecimalConfig,
    len: usize,
}

impl Decimal128BuilderWrapper {
    /// Create a new decimal builder with validated configuration.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of decimal values to pre-allocate
    /// * `precision` - Decimal precision (1-38)
    /// * `scale` - Decimal scale (0 ≤ scale ≤ precision)
    ///
    /// # Panics
    ///
    /// Panics if precision or scale are invalid (should be validated before calling).
    #[must_use]
    pub fn new(capacity: usize, precision: u8, scale: i8) -> Self {
        let config = DecimalConfig::new(precision, scale)
            .expect("decimal config should be validated before builder creation");

        let builder = Decimal128Builder::with_capacity(capacity)
            .with_data_type(arrow_schema::DataType::Decimal128(precision, scale));

        Self {
            builder,
            config,
            len: 0,
        }
    }

    /// Create from validated config.
    #[must_use]
    pub fn from_config(capacity: usize, config: DecimalConfig) -> Self {
        let builder = Decimal128Builder::with_capacity(capacity).with_data_type(
            arrow_schema::DataType::Decimal128(config.precision(), config.scale()),
        );

        Self {
            builder,
            config,
            len: 0,
        }
    }

    /// Convert a HANA decimal value to i128 with proper scaling.
    ///
    /// # Implementation Note
    ///
    /// HANA DECIMAL values are represented as `BigDecimal` in hdbconnect.
    /// We need to:
    /// 1. Extract mantissa and exponent
    /// 2. Scale to match Arrow Decimal128 scale
    /// 3. Convert to i128
    ///
    /// # Errors
    ///
    /// Returns error if value cannot be represented in Decimal128.
    fn convert_decimal(&self, value: &hdbconnect::HdbValue) -> Result<i128> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::DECIMAL(decimal) => {
                // Convert to string, then parse as i128 with proper scaling
                // This is a simplified approach - production code may need
                // more sophisticated decimal arithmetic

                let string_repr = decimal.to_string();

                // Parse the decimal string
                // Example: "123.45" with scale=2 -> 12345_i128
                let parts: Vec<&str> = string_repr.split('.').collect();

                let (int_part, frac_part) = match parts.len() {
                    1 => (parts[0], ""),
                    2 => (parts[0], parts[1]),
                    _ => {
                        return Err(crate::ArrowConversionError::value_conversion(
                            "decimal",
                            format!("invalid decimal format: {string_repr}"),
                        ));
                    }
                };

                // Build the scaled integer value
                #[allow(clippy::cast_sign_loss)]
                let target_scale = self.config.scale() as usize;
                let frac_digits = frac_part.len();

                let scaled_str = match frac_digits.cmp(&target_scale) {
                    std::cmp::Ordering::Less => {
                        // Pad with zeros
                        format!(
                            "{int_part}{frac_part}{}",
                            "0".repeat(target_scale - frac_digits)
                        )
                    }
                    std::cmp::Ordering::Greater => {
                        // Truncate (or round - implementation choice)
                        format!("{int_part}{}", &frac_part[..target_scale])
                    }
                    std::cmp::Ordering::Equal => {
                        format!("{int_part}{frac_part}")
                    }
                };

                scaled_str.parse::<i128>().map_err(|e| {
                    crate::ArrowConversionError::value_conversion(
                        "decimal",
                        format!(
                            "cannot convert {} to Decimal128({}, {}): {}",
                            string_repr,
                            self.config.precision(),
                            self.config.scale(),
                            e
                        ),
                    )
                })
            }
            other => Err(crate::ArrowConversionError::value_conversion(
                "decimal",
                format!("expected DECIMAL, got {other:?}"),
            )),
        }
    }
}

impl Sealed for Decimal128BuilderWrapper {}

impl HanaCompatibleBuilder for Decimal128BuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        let i128_val = self.convert_decimal(value)?;
        self.builder.append_value(i128_val);
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

#[cfg(test)]
mod tests {
    use arrow_array::Array;

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // DecimalConfig Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decimal_config_valid() {
        let config = DecimalConfig::new(18, 2).unwrap();
        assert_eq!(config.precision(), 18);
        assert_eq!(config.scale(), 2);
    }

    #[test]
    fn test_decimal_config_invalid_precision() {
        assert!(DecimalConfig::new(0, 0).is_err());
        assert!(DecimalConfig::new(39, 0).is_err());
    }

    #[test]
    fn test_decimal_config_invalid_scale() {
        assert!(DecimalConfig::new(18, -1).is_err());
        assert!(DecimalConfig::new(18, 20).is_err());
    }

    #[test]
    fn test_decimal_config_min_precision() {
        let config = DecimalConfig::new(1, 0).unwrap();
        assert_eq!(config.precision(), 1);
        assert_eq!(config.scale(), 0);
    }

    #[test]
    fn test_decimal_config_max_precision() {
        let config = DecimalConfig::new(38, 10).unwrap();
        assert_eq!(config.precision(), 38);
        assert_eq!(config.scale(), 10);
    }

    #[test]
    fn test_decimal_config_scale_equals_precision() {
        let config = DecimalConfig::new(5, 5).unwrap();
        assert_eq!(config.precision(), 5);
        assert_eq!(config.scale(), 5);
    }

    #[test]
    fn test_decimal_config_zero_scale() {
        let config = DecimalConfig::new(10, 0).unwrap();
        assert_eq!(config.precision(), 10);
        assert_eq!(config.scale(), 0);
    }

    #[test]
    fn test_decimal_config_equality() {
        let config1 = DecimalConfig::new(18, 2).unwrap();
        let config2 = DecimalConfig::new(18, 2).unwrap();
        let config3 = DecimalConfig::new(18, 3).unwrap();
        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn test_decimal_config_copy() {
        let config1 = DecimalConfig::new(18, 2).unwrap();
        let config2 = config1;
        assert_eq!(config1, config2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Decimal128BuilderWrapper Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decimal_builder_creation() {
        let builder = Decimal128BuilderWrapper::new(100, 18, 2);
        assert_eq!(builder.len(), 0);
        assert_eq!(builder.config.precision(), 18);
        assert_eq!(builder.config.scale(), 2);
    }

    #[test]
    fn test_decimal_builder_from_config() {
        let config = DecimalConfig::new(10, 4).unwrap();
        let builder = Decimal128BuilderWrapper::from_config(50, config);
        assert_eq!(builder.len(), 0);
        assert_eq!(builder.config.precision(), 10);
        assert_eq!(builder.config.scale(), 4);
    }

    #[test]
    fn test_decimal_builder_capacity() {
        let builder = Decimal128BuilderWrapper::new(100, 18, 2);
        assert!(builder.capacity().is_some());
    }

    #[test]
    fn test_decimal_builder_is_empty() {
        let builder = Decimal128BuilderWrapper::new(10, 18, 2);
        assert!(builder.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Null Handling Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decimal_builder_append_null() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        assert!(array.is_null(0));
    }

    #[test]
    fn test_decimal_builder_multiple_nulls() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        builder.append_null();
        builder.append_null();
        builder.append_null();
        assert_eq!(builder.len(), 3);

        let array = builder.finish();
        assert_eq!(array.len(), 3);
        assert!(array.is_null(0));
        assert!(array.is_null(1));
        assert!(array.is_null(2));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Finish and Reset Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decimal_builder_finish_resets_len() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        builder.append_null();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        let _ = builder.finish();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_decimal_builder_finish_empty() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        let array = builder.finish();
        assert_eq!(array.len(), 0);
    }

    #[test]
    fn test_decimal_builder_reuse_after_finish() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        builder.append_null();
        let array1 = builder.finish();
        assert_eq!(array1.len(), 1);

        builder.append_null();
        builder.append_null();
        let array2 = builder.finish();
        assert_eq!(array2.len(), 2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Different Precision/Scale Combinations
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decimal_builder_high_precision() {
        let builder = Decimal128BuilderWrapper::new(10, 38, 10);
        assert_eq!(builder.config.precision(), 38);
        assert_eq!(builder.config.scale(), 10);
    }

    #[test]
    fn test_decimal_builder_low_precision() {
        let builder = Decimal128BuilderWrapper::new(10, 1, 0);
        assert_eq!(builder.config.precision(), 1);
        assert_eq!(builder.config.scale(), 0);
    }

    #[test]
    fn test_decimal_builder_zero_scale() {
        let builder = Decimal128BuilderWrapper::new(10, 10, 0);
        assert_eq!(builder.config.precision(), 10);
        assert_eq!(builder.config.scale(), 0);
    }

    #[test]
    fn test_decimal_builder_scale_equals_precision() {
        let builder = Decimal128BuilderWrapper::new(10, 5, 5);
        assert_eq!(builder.config.precision(), 5);
        assert_eq!(builder.config.scale(), 5);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HanaCompatibleBuilder trait Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decimal_builder_len_increments() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        assert_eq!(builder.len(), 0);
        builder.append_null();
        assert_eq!(builder.len(), 1);
        builder.append_null();
        assert_eq!(builder.len(), 2);
    }

    #[test]
    fn test_decimal_builder_reset() {
        let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
        builder.append_null();
        builder.append_null();
        assert_eq!(builder.len(), 2);

        builder.reset();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
    }
}
