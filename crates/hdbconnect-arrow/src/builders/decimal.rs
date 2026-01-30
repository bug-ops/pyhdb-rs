//! Decimal128 builder with precision and scale validation.
//!
//! Handles HANA DECIMAL and SMALLDECIMAL types with proper precision/scale
//! preservation using Arrow Decimal128 arrays.

use std::sync::{Arc, LazyLock};

use arrow_array::ArrayRef;
use arrow_array::builder::Decimal128Builder;
use num_bigint::BigInt;

use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::sealed::private::Sealed;
use crate::types::hana::{DecimalPrecision, DecimalScale};

// Pre-computed powers of 10 as i128 for O(1) lookup in fast path.
// Covers 10^0 through 10^38 (39 elements), sufficient for max DECIMAL precision.
const POWERS_OF_10_I128: [i128; 39] = [
    1,                                                   // 10^0
    10,                                                  // 10^1
    100,                                                 // 10^2
    1_000,                                               // 10^3
    10_000,                                              // 10^4
    100_000,                                             // 10^5
    1_000_000,                                           // 10^6
    10_000_000,                                          // 10^7
    100_000_000,                                         // 10^8
    1_000_000_000,                                       // 10^9
    10_000_000_000,                                      // 10^10
    100_000_000_000,                                     // 10^11
    1_000_000_000_000,                                   // 10^12
    10_000_000_000_000,                                  // 10^13
    100_000_000_000_000,                                 // 10^14
    1_000_000_000_000_000,                               // 10^15
    10_000_000_000_000_000,                              // 10^16
    100_000_000_000_000_000,                             // 10^17
    1_000_000_000_000_000_000,                           // 10^18
    10_000_000_000_000_000_000,                          // 10^19
    100_000_000_000_000_000_000,                         // 10^20
    1_000_000_000_000_000_000_000,                       // 10^21
    10_000_000_000_000_000_000_000,                      // 10^22
    100_000_000_000_000_000_000_000,                     // 10^23
    1_000_000_000_000_000_000_000_000,                   // 10^24
    10_000_000_000_000_000_000_000_000,                  // 10^25
    100_000_000_000_000_000_000_000_000,                 // 10^26
    1_000_000_000_000_000_000_000_000_000,               // 10^27
    10_000_000_000_000_000_000_000_000_000,              // 10^28
    100_000_000_000_000_000_000_000_000_000,             // 10^29
    1_000_000_000_000_000_000_000_000_000_000,           // 10^30
    10_000_000_000_000_000_000_000_000_000_000,          // 10^31
    100_000_000_000_000_000_000_000_000_000_000,         // 10^32
    1_000_000_000_000_000_000_000_000_000_000_000,       // 10^33
    10_000_000_000_000_000_000_000_000_000_000_000,      // 10^34
    100_000_000_000_000_000_000_000_000_000_000_000,     // 10^35
    1_000_000_000_000_000_000_000_000_000_000_000_000,   // 10^36
    10_000_000_000_000_000_000_000_000_000_000_000_000,  // 10^37
    100_000_000_000_000_000_000_000_000_000_000_000_000, // 10^38
];

// Pre-computed powers of 10 as BigInt for slow path fallback.
// Initialized lazily on first access, thread-safe via LazyLock.
static POWERS_OF_10_BIGINT: LazyLock<[BigInt; 39]> =
    LazyLock::new(|| std::array::from_fn(|i| BigInt::from(POWERS_OF_10_I128[i])));

/// Fast path: multiply mantissa by power of 10 using i128 arithmetic.
/// Returns `None` if `scale_diff` is out of range or multiplication overflows.
#[inline]
fn scale_up_i128(mantissa: i128, scale_diff: i64) -> Option<i128> {
    let idx = usize::try_from(scale_diff).ok()?;
    let multiplier = POWERS_OF_10_I128.get(idx)?;
    mantissa.checked_mul(*multiplier)
}

/// Fast path: divide mantissa by power of 10 using i128 arithmetic.
/// Returns `None` if `scale_diff` is out of range.
#[inline]
fn scale_down_i128(mantissa: i128, scale_diff: i64) -> Option<i128> {
    let idx = usize::try_from(scale_diff).ok()?;
    let divisor = POWERS_OF_10_I128.get(idx)?;
    Some(mantissa / divisor)
}

/// Slow path: scale up using `BigInt` arithmetic with pre-computed powers.
fn scale_up_bigint(mantissa: &BigInt, scale_diff: i64) -> BigInt {
    let exp = usize::try_from(scale_diff).unwrap_or(0).min(38);
    mantissa * &POWERS_OF_10_BIGINT[exp]
}

/// Slow path: scale down using `BigInt` arithmetic with pre-computed powers.
fn scale_down_bigint(mantissa: &BigInt, scale_diff: i64) -> BigInt {
    let exp = usize::try_from(scale_diff).unwrap_or(0).min(38);
    mantissa / &POWERS_OF_10_BIGINT[exp]
}

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

    /// Slow path: `BigInt` arithmetic with pre-computed powers lookup.
    /// Used when mantissa exceeds i128 range or fast path overflows.
    fn convert_decimal_slow(&self, mantissa: &BigInt, scale_diff: i64) -> Result<i128> {
        use num_traits::ToPrimitive;

        let scaled_value = if scale_diff >= 0 {
            scale_up_bigint(mantissa, scale_diff)
        } else {
            scale_down_bigint(mantissa, -scale_diff)
        };

        scaled_value.to_i128().ok_or_else(|| {
            crate::ArrowConversionError::decimal_overflow(
                self.config.precision(),
                self.config.scale(),
            )
        })
    }

    /// Convert a HANA decimal value to i128 with proper scaling.
    ///
    /// Uses fast path (i128 arithmetic) when possible, falling back to
    /// `BigInt` arithmetic for edge cases (mantissa exceeds i128 or overflow).
    ///
    /// # Errors
    ///
    /// Returns error if value cannot be represented in Decimal128.
    fn convert_decimal(&self, value: &hdbconnect::HdbValue) -> Result<i128> {
        use hdbconnect::HdbValue;
        use num_traits::ToPrimitive;

        match value {
            HdbValue::DECIMAL(decimal) => {
                // Zero-copy mantissa extraction via Cow::Borrowed.
                // as_bigint_and_scale() returns (Cow<'_, BigInt>, i64) which borrows
                // the mantissa instead of cloning it. Cow<BigInt> automatically derefs
                // to &BigInt when needed, providing transparent zero-cost abstraction.
                let (mantissa, exponent) = decimal.as_bigint_and_scale();
                let target_scale = i64::from(self.config.scale());
                let scale_diff = target_scale - exponent;

                // Fast path: try i128 arithmetic first (covers >99% of cases)
                if let Some(m) = mantissa.to_i128() {
                    if scale_diff >= 0 {
                        if let Some(result) = scale_up_i128(m, scale_diff) {
                            return Ok(result);
                        }
                    } else if let Some(result) = scale_down_i128(m, -scale_diff) {
                        return Ok(result);
                    }
                }

                // Slow path: BigInt arithmetic with cached powers
                // Cow<BigInt> derefs to &BigInt automatically
                self.convert_decimal_slow(&mantissa, scale_diff)
            }
            other => Err(crate::ArrowConversionError::value_conversion(
                "decimal",
                format!("expected DECIMAL, got {:?}", std::mem::discriminant(other)),
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Powers of 10 Lookup Table Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_powers_of_10_i128_correctness() {
        for (i, &power) in POWERS_OF_10_I128.iter().enumerate() {
            let expected = 10_i128.pow(i as u32);
            assert_eq!(power, expected, "POWERS_OF_10_I128[{i}] mismatch");
        }
    }

    #[test]
    fn test_powers_of_10_bigint_correctness() {
        for (i, power) in POWERS_OF_10_BIGINT.iter().enumerate() {
            let expected = BigInt::from(10_i128).pow(i as u32);
            assert_eq!(power, &expected, "POWERS_OF_10_BIGINT[{i}] mismatch");
        }
    }

    #[test]
    fn test_powers_of_10_i128_boundary_values() {
        assert_eq!(POWERS_OF_10_I128[0], 1);
        assert_eq!(
            POWERS_OF_10_I128[38],
            100_000_000_000_000_000_000_000_000_000_000_000_000
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Fast Path Method Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_scale_up_i128_simple() {
        assert_eq!(scale_up_i128(100, 2), Some(10000));
    }

    #[test]
    fn test_scale_up_i128_zero_scale_diff() {
        assert_eq!(scale_up_i128(12345, 0), Some(12345));
    }

    #[test]
    fn test_scale_up_i128_max_scale_diff() {
        assert_eq!(scale_up_i128(1, 38), Some(POWERS_OF_10_I128[38]));
    }

    #[test]
    fn test_scale_up_i128_overflow() {
        // i128::MAX * 10 would overflow
        assert_eq!(scale_up_i128(i128::MAX, 1), None);
    }

    #[test]
    fn test_scale_up_i128_negative_scale_diff() {
        assert_eq!(scale_up_i128(100, -1), None);
    }

    #[test]
    fn test_scale_up_i128_out_of_range_scale_diff() {
        assert_eq!(scale_up_i128(1, 39), None);
    }

    #[test]
    fn test_scale_down_i128_simple() {
        assert_eq!(scale_down_i128(12345, 2), Some(123));
    }

    #[test]
    fn test_scale_down_i128_zero_scale_diff() {
        assert_eq!(scale_down_i128(12345, 0), Some(12345));
    }

    #[test]
    fn test_scale_down_i128_truncation() {
        assert_eq!(scale_down_i128(12399, 2), Some(123));
    }

    #[test]
    fn test_scale_down_i128_negative_scale_diff() {
        assert_eq!(scale_down_i128(100, -1), None);
    }

    #[test]
    fn test_scale_down_i128_out_of_range() {
        assert_eq!(scale_down_i128(1, 39), None);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Slow Path Method Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_convert_decimal_slow_scale_up() {
        let builder = Decimal128BuilderWrapper::new(10, 18, 2);
        let mantissa = BigInt::from(100_i128);
        assert_eq!(builder.convert_decimal_slow(&mantissa, 2).unwrap(), 10000);
    }

    #[test]
    fn test_convert_decimal_slow_scale_down() {
        let builder = Decimal128BuilderWrapper::new(10, 18, 0);
        let mantissa = BigInt::from(12345_i128);
        assert_eq!(builder.convert_decimal_slow(&mantissa, -2).unwrap(), 123);
    }

    #[test]
    fn test_convert_decimal_slow_large_mantissa() {
        let builder = Decimal128BuilderWrapper::new(10, 38, 0);
        // Value that fits in i128 but tests slow path
        let mantissa = BigInt::from(i128::MAX);
        assert_eq!(
            builder.convert_decimal_slow(&mantissa, 0).unwrap(),
            i128::MAX
        );
    }

    #[test]
    fn test_convert_decimal_slow_overflow() {
        let builder = Decimal128BuilderWrapper::new(10, 38, 0);
        // Value exceeding i128::MAX
        let mantissa = BigInt::from(i128::MAX) * 10;
        assert!(builder.convert_decimal_slow(&mantissa, 0).is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // convert_decimal Tests (CRIT-001)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(feature = "test-utils")]
    mod convert_decimal_tests {
        use arrow_array::Decimal128Array;

        use super::*;
        use crate::traits::row::MockRowBuilder;

        /// Helper to extract i128 value from builder after appending decimal
        fn convert_and_get_value(precision: u8, scale: i8, decimal_str: &str) -> i128 {
            let mut builder = Decimal128BuilderWrapper::new(10, precision, scale);
            let row = MockRowBuilder::new().decimal_str(decimal_str).build();
            builder.append_hana_value(&row[0]).unwrap();
            let array = builder.finish();
            let decimal_array = array.as_any().downcast_ref::<Decimal128Array>().unwrap();
            decimal_array.value(0)
        }

        // ───────────────────────────────────────────────────────────────────────
        // Basic decimal conversion tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_simple_value() {
            // 123.45 with scale=2 should store as 12345
            let value = convert_and_get_value(18, 2, "123.45");
            assert_eq!(value, 12345);
        }

        #[test]
        fn test_convert_decimal_integer_value() {
            // 100 with scale=0 should store as 100
            let value = convert_and_get_value(10, 0, "100");
            assert_eq!(value, 100);
        }

        #[test]
        fn test_convert_decimal_large_value() {
            // 9999999.99 with scale=2 should store as 999999999
            let value = convert_and_get_value(18, 2, "9999999.99");
            assert_eq!(value, 999999999);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Scale-up tests (target_scale > source_scale)
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_scale_up_integer_to_decimal() {
            // 100 (scale=0) to scale=2 should become 10000 (100.00)
            let value = convert_and_get_value(10, 2, "100");
            assert_eq!(value, 10000);
        }

        #[test]
        fn test_convert_decimal_scale_up_by_one() {
            // 123.4 (scale=1) to scale=2 should become 12340 (123.40)
            let value = convert_and_get_value(10, 2, "123.4");
            assert_eq!(value, 12340);
        }

        #[test]
        fn test_convert_decimal_scale_up_by_multiple() {
            // 5 (scale=0) to scale=4 should become 50000 (5.0000)
            let value = convert_and_get_value(10, 4, "5");
            assert_eq!(value, 50000);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Scale-down tests (target_scale < source_scale, may lose precision)
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_scale_down_truncate() {
            // 123.456 (scale=3) to scale=2 should become 12345 (123.45, truncated)
            let value = convert_and_get_value(10, 2, "123.456");
            assert_eq!(value, 12345);
        }

        #[test]
        fn test_convert_decimal_scale_down_to_integer() {
            // 123.99 (scale=2) to scale=0 should become 123 (truncated)
            let value = convert_and_get_value(10, 0, "123.99");
            assert_eq!(value, 123);
        }

        #[test]
        fn test_convert_decimal_scale_down_multiple_places() {
            // 1.23456789 (scale=8) to scale=2 should become 123 (1.23)
            let value = convert_and_get_value(18, 2, "1.23456789");
            assert_eq!(value, 123);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Scale-match tests (target_scale == source_scale)
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_scale_match_exact() {
            // 123.45 (scale=2) to scale=2 should be 12345 (no change)
            let value = convert_and_get_value(18, 2, "123.45");
            assert_eq!(value, 12345);
        }

        #[test]
        fn test_convert_decimal_scale_match_high_scale() {
            // 1.234567890123456789 (scale=18) to scale=18
            let value = convert_and_get_value(38, 18, "1.234567890123456789");
            assert_eq!(value, 1_234_567_890_123_456_789_i128);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Negative value tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_negative_simple() {
            // -123.45 with scale=2 should store as -12345
            let value = convert_and_get_value(18, 2, "-123.45");
            assert_eq!(value, -12345);
        }

        #[test]
        fn test_convert_decimal_negative_scale_up() {
            // -100 (scale=0) to scale=2 should become -10000
            let value = convert_and_get_value(10, 2, "-100");
            assert_eq!(value, -10000);
        }

        #[test]
        fn test_convert_decimal_negative_scale_down() {
            // -123.456 (scale=3) to scale=2 should become -12345 (truncated toward zero)
            let value = convert_and_get_value(10, 2, "-123.456");
            assert_eq!(value, -12345);
        }

        #[test]
        fn test_convert_decimal_negative_large() {
            // -9999999.99 with scale=2
            let value = convert_and_get_value(18, 2, "-9999999.99");
            assert_eq!(value, -999999999);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Zero value tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_zero() {
            let value = convert_and_get_value(10, 2, "0");
            assert_eq!(value, 0);
        }

        #[test]
        fn test_convert_decimal_zero_with_scale() {
            // 0.00 with scale=2 should store as 0
            let value = convert_and_get_value(10, 2, "0.00");
            assert_eq!(value, 0);
        }

        #[test]
        fn test_convert_decimal_negative_zero() {
            // -0 should still be 0
            let value = convert_and_get_value(10, 2, "-0");
            assert_eq!(value, 0);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Maximum precision (38-digit) tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_max_precision_integer() {
            // Maximum 38-digit integer that fits in i128 (precision=38, scale=0)
            // i128::MAX is 170141183460469231731687303715884105727 (39 digits)
            // We use a 38-digit number
            let value = convert_and_get_value(38, 0, "99999999999999999999999999999999999999");
            assert_eq!(
                value,
                99_999_999_999_999_999_999_999_999_999_999_999_999_i128
            );
        }

        #[test]
        fn test_convert_decimal_max_precision_with_scale() {
            // 38-digit decimal with scale: 9999999999999999999999999999999999.9999
            // stored as 99999999999999999999999999999999999999 with scale=4
            let value = convert_and_get_value(38, 4, "9999999999999999999999999999999999.9999");
            assert_eq!(
                value,
                99_999_999_999_999_999_999_999_999_999_999_999_999_i128
            );
        }

        #[test]
        fn test_convert_decimal_i128_max_boundary() {
            // Test value close to i128::MAX
            // i128::MAX = 170141183460469231731687303715884105727
            let value = convert_and_get_value(38, 0, "170141183460469231731687303715884105727");
            assert_eq!(value, i128::MAX);
        }

        #[test]
        fn test_convert_decimal_i128_min_boundary() {
            // Test value close to i128::MIN
            // i128::MIN = -170141183460469231731687303715884105728
            let value = convert_and_get_value(38, 0, "-170141183460469231731687303715884105728");
            assert_eq!(value, i128::MIN);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Overflow detection tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_overflow_scale_up() {
            // Scale up operation that causes overflow:
            // i128::MAX with additional scaling would overflow
            let mut builder = Decimal128BuilderWrapper::new(10, 38, 10);
            // Create a value that when scaled up by 10^10, exceeds i128::MAX
            let row = MockRowBuilder::new()
                .decimal_str("17014118346046923173168730371588410573")
                .build();
            let result = builder.append_hana_value(&row[0]);
            assert!(result.is_err());
        }

        #[test]
        fn test_convert_decimal_overflow_large_value() {
            // Value already exceeding i128 range after scaling
            let mut builder = Decimal128BuilderWrapper::new(10, 38, 0);
            // This is larger than i128::MAX
            let row = MockRowBuilder::new()
                .decimal_str("999999999999999999999999999999999999999")
                .build();
            let result = builder.append_hana_value(&row[0]);
            assert!(result.is_err());
        }

        // ───────────────────────────────────────────────────────────────────────
        // Wrong type error handling tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_wrong_type_int() {
            let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
            let row = MockRowBuilder::new().int(42).build();
            let result = builder.append_hana_value(&row[0]);
            assert!(result.is_err());
        }

        #[test]
        fn test_convert_decimal_wrong_type_string() {
            let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
            let row = MockRowBuilder::new().string("123.45").build();
            let result = builder.append_hana_value(&row[0]);
            assert!(result.is_err());
        }

        #[test]
        fn test_convert_decimal_wrong_type_null() {
            let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
            let row = MockRowBuilder::new().null().build();
            let result = builder.append_hana_value(&row[0]);
            // NULL should be handled via append_null, not convert_decimal
            // append_hana_value with NULL should fail
            assert!(result.is_err());
        }

        #[test]
        fn test_convert_decimal_wrong_type_double() {
            let mut builder = Decimal128BuilderWrapper::new(10, 18, 2);
            let row = MockRowBuilder::new().double(123.45).build();
            let result = builder.append_hana_value(&row[0]);
            assert!(result.is_err());
        }

        // ───────────────────────────────────────────────────────────────────────
        // Additional edge cases
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_convert_decimal_very_small_value() {
            // 0.00000001 with scale=8 should be 1
            let value = convert_and_get_value(18, 8, "0.00000001");
            assert_eq!(value, 1);
        }

        #[test]
        fn test_convert_decimal_leading_zeros() {
            // 00123.45 should be same as 123.45
            let value = convert_and_get_value(18, 2, "00123.45");
            assert_eq!(value, 12345);
        }

        #[test]
        fn test_convert_decimal_trailing_zeros() {
            // 123.450 should be same as 123.45 when target scale is 2
            let value = convert_and_get_value(18, 2, "123.450");
            assert_eq!(value, 12345);
        }

        #[test]
        fn test_convert_decimal_scientific_notation() {
            // 1.23e2 = 123 with scale=0
            let value = convert_and_get_value(10, 0, "1.23e2");
            assert_eq!(value, 123);
        }

        #[test]
        fn test_convert_decimal_scientific_notation_negative_exp() {
            // 12300e-2 = 123 with scale=0
            let value = convert_and_get_value(10, 0, "12300e-2");
            assert_eq!(value, 123);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Fast path boundary tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_fast_path_boundary_i128_max() {
            // i128::MAX with scale=0 should use fast path (no scaling needed)
            let value = convert_and_get_value(38, 0, "170141183460469231731687303715884105727");
            assert_eq!(value, i128::MAX);
        }

        #[test]
        fn test_fast_path_boundary_i128_min() {
            // i128::MIN with scale=0 should use fast path
            let value = convert_and_get_value(38, 0, "-170141183460469231731687303715884105728");
            assert_eq!(value, i128::MIN);
        }

        #[test]
        fn test_fast_path_boundary_scale_38() {
            // Maximum scale difference (38)
            let value = convert_and_get_value(38, 38, "1");
            assert_eq!(value, POWERS_OF_10_I128[38]);
        }

        // ───────────────────────────────────────────────────────────────────────
        // Slow path fallback tests
        // ───────────────────────────────────────────────────────────────────────

        #[test]
        fn test_slow_path_fallback_overflow_triggers_slow_path() {
            // Large value that fits in i128 but scaling causes fast path overflow
            // 10^37 * 10 = 10^38 which fits in i128
            let value = convert_and_get_value(38, 1, "10000000000000000000000000000000000000");
            assert_eq!(value, POWERS_OF_10_I128[38]);
        }

        #[test]
        fn test_slow_path_mantissa_exceeds_i128() {
            // Mantissa that exceeds i128 but result fits after scale down
            // This tests that slow path is triggered when mantissa.to_i128() returns None
            let mut builder = Decimal128BuilderWrapper::new(10, 38, 0);
            // 10^39 / 10 = 10^38 which fits
            let row = MockRowBuilder::new()
                .decimal_str("1000000000000000000000000000000000000000")
                .build();
            // Scale down from source scale to 0 should result in truncation
            // BigDecimal "1000000000000000000000000000000000000000" has 40 digits
            let result = builder.append_hana_value(&row[0]);
            // This exceeds i128::MAX, so should error
            assert!(result.is_err());
        }
    }
}
