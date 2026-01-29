//! Type-safe builder factory using phantom types.
//!
//! The factory pattern ensures that builders are created with correct
//! configurations for each Arrow data type.

use arrow_schema::{DataType, TimeUnit};

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
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::streaming::BatchConfig;

/// Factory for creating type-safe Arrow builders.
///
/// The factory ensures builders are created with appropriate capacity
/// and configuration for each Arrow data type.
#[derive(Debug, Clone)]
pub struct BuilderFactory {
    /// Number of rows to pre-allocate in each builder.
    capacity: usize,
    /// Bytes to pre-allocate for string data.
    string_capacity: usize,
    /// Bytes to pre-allocate for binary data.
    binary_capacity: usize,
    /// Maximum LOB size in bytes before rejecting.
    max_lob_bytes: Option<usize>,
}

impl BuilderFactory {
    /// Create a new factory with the specified row capacity.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            string_capacity: capacity * 32, // Estimate 32 bytes per string
            binary_capacity: capacity * 64, // Estimate 64 bytes per binary
            max_lob_bytes: None,
        }
    }

    /// Create from `BatchConfig`.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_config(config: &BatchConfig) -> Self {
        Self {
            capacity: config.batch_size.get(),
            string_capacity: config.string_capacity,
            binary_capacity: config.binary_capacity,
            max_lob_bytes: config.max_lob_bytes,
        }
    }

    /// Set the string data capacity.
    #[must_use]
    pub const fn with_string_capacity(mut self, capacity: usize) -> Self {
        self.string_capacity = capacity;
        self
    }

    /// Set the binary data capacity.
    #[must_use]
    pub const fn with_binary_capacity(mut self, capacity: usize) -> Self {
        self.binary_capacity = capacity;
        self
    }

    /// Set the maximum LOB size in bytes.
    #[must_use]
    pub const fn with_max_lob_bytes(mut self, max: Option<usize>) -> Self {
        self.max_lob_bytes = max;
        self
    }

    /// Create a builder for the specified Arrow data type.
    ///
    /// Returns a boxed trait object that implements `HanaCompatibleBuilder`.
    ///
    /// # Panics
    ///
    /// Panics if the data type is not supported (should not happen if using
    /// `hana_type_to_arrow` for type mapping).
    #[must_use]
    #[allow(clippy::match_same_arms)] // Intentional: explicit Utf8 case for clarity
    pub fn create_builder(&self, data_type: &DataType) -> Box<dyn HanaCompatibleBuilder> {
        match data_type {
            // Primitive numeric types
            DataType::UInt8 => Box::new(UInt8BuilderWrapper::new(self.capacity)),
            DataType::Int16 => Box::new(Int16BuilderWrapper::new(self.capacity)),
            DataType::Int32 => Box::new(Int32BuilderWrapper::new(self.capacity)),
            DataType::Int64 => Box::new(Int64BuilderWrapper::new(self.capacity)),
            DataType::Float32 => Box::new(Float32BuilderWrapper::new(self.capacity)),
            DataType::Float64 => Box::new(Float64BuilderWrapper::new(self.capacity)),

            // Decimal
            DataType::Decimal128(precision, scale) => Box::new(Decimal128BuilderWrapper::new(
                self.capacity,
                *precision,
                *scale,
            )),

            // Strings
            DataType::Utf8 => Box::new(StringBuilderWrapper::new(
                self.capacity,
                self.string_capacity,
            )),
            DataType::LargeUtf8 => {
                let mut builder =
                    LargeStringBuilderWrapper::new(self.capacity, self.string_capacity);
                if let Some(max) = self.max_lob_bytes {
                    builder = builder.with_max_lob_bytes(max);
                }
                Box::new(builder)
            }

            // Binary
            DataType::Binary => Box::new(BinaryBuilderWrapper::new(
                self.capacity,
                self.binary_capacity,
            )),
            DataType::LargeBinary => {
                let mut builder =
                    LargeBinaryBuilderWrapper::new(self.capacity, self.binary_capacity);
                if let Some(max) = self.max_lob_bytes {
                    builder = builder.with_max_lob_bytes(max);
                }
                Box::new(builder)
            }
            DataType::FixedSizeBinary(size) => {
                Box::new(FixedSizeBinaryBuilderWrapper::new(self.capacity, *size))
            }

            // Temporal
            DataType::Date32 => Box::new(Date32BuilderWrapper::new(self.capacity)),
            DataType::Time64(TimeUnit::Nanosecond) => {
                Box::new(Time64NanosecondBuilderWrapper::new(self.capacity))
            }
            DataType::Timestamp(TimeUnit::Nanosecond, None) => {
                Box::new(TimestampNanosecondBuilderWrapper::new(self.capacity))
            }

            // Boolean
            DataType::Boolean => Box::new(BooleanBuilderWrapper::new(self.capacity)),

            // Unsupported - fallback to string
            _ => Box::new(StringBuilderWrapper::new(
                self.capacity,
                self.string_capacity,
            )),
        }
    }

    /// Create builders for all fields in a schema.
    ///
    /// Returns a vector of boxed builders in the same order as schema fields.
    #[must_use]
    pub fn create_builders_for_schema(
        &self,
        schema: &arrow_schema::Schema,
    ) -> Vec<Box<dyn HanaCompatibleBuilder>> {
        let fields = schema.fields();
        let mut builders = Vec::with_capacity(fields.len());
        for field in fields {
            builders.push(self.create_builder(field.data_type()));
        }
        builders
    }
}

impl Default for BuilderFactory {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use arrow_schema::{DataType, Field, Schema, TimeUnit};

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // Factory Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_factory_creation() {
        let factory = BuilderFactory::new(100);
        assert_eq!(factory.capacity, 100);
        assert_eq!(factory.string_capacity, 3200);
        assert_eq!(factory.binary_capacity, 6400);
        assert!(factory.max_lob_bytes.is_none());
    }

    #[test]
    fn test_factory_default() {
        let factory = BuilderFactory::default();
        assert_eq!(factory.capacity, 1024);
        assert_eq!(factory.string_capacity, 1024 * 32);
        assert_eq!(factory.binary_capacity, 1024 * 64);
        assert!(factory.max_lob_bytes.is_none());
    }

    #[test]
    fn test_factory_from_config() {
        let config = BatchConfig::with_batch_size(500)
            .string_capacity(10000)
            .binary_capacity(20000)
            .max_lob_bytes(Some(50_000_000));

        let factory = BuilderFactory::from_config(&config);
        assert_eq!(factory.capacity, 500);
        assert_eq!(factory.string_capacity, 10000);
        assert_eq!(factory.binary_capacity, 20000);
        assert_eq!(factory.max_lob_bytes, Some(50_000_000));
    }

    #[test]
    fn test_factory_from_config_without_lob_limit() {
        let config = BatchConfig::with_batch_size(500)
            .string_capacity(10000)
            .binary_capacity(20000);

        let factory = BuilderFactory::from_config(&config);
        assert!(factory.max_lob_bytes.is_none());
    }

    #[test]
    fn test_factory_with_string_capacity() {
        let factory = BuilderFactory::new(100).with_string_capacity(5000);
        assert_eq!(factory.capacity, 100);
        assert_eq!(factory.string_capacity, 5000);
        assert_eq!(factory.binary_capacity, 6400);
    }

    #[test]
    fn test_factory_with_binary_capacity() {
        let factory = BuilderFactory::new(100).with_binary_capacity(8000);
        assert_eq!(factory.capacity, 100);
        assert_eq!(factory.string_capacity, 3200);
        assert_eq!(factory.binary_capacity, 8000);
    }

    #[test]
    fn test_factory_with_max_lob_bytes() {
        let factory = BuilderFactory::new(100).with_max_lob_bytes(Some(10_000_000));
        assert_eq!(factory.max_lob_bytes, Some(10_000_000));
    }

    #[test]
    fn test_factory_builder_chaining() {
        let factory = BuilderFactory::new(200)
            .with_string_capacity(1000)
            .with_binary_capacity(2000)
            .with_max_lob_bytes(Some(5_000_000));

        assert_eq!(factory.capacity, 200);
        assert_eq!(factory.string_capacity, 1000);
        assert_eq!(factory.binary_capacity, 2000);
        assert_eq!(factory.max_lob_bytes, Some(5_000_000));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Primitive Type Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_primitive_builders() {
        let factory = BuilderFactory::new(100);

        let _ = factory.create_builder(&DataType::Int32);
        let _ = factory.create_builder(&DataType::Float64);
        let _ = factory.create_builder(&DataType::Utf8);
    }

    #[test]
    fn test_create_uint8_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::UInt8);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_int16_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Int16);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_int32_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Int32);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_int64_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Int64);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_float32_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Float32);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_float64_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Float64);
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Decimal Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_decimal_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Decimal128(18, 2));
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_decimal_builder_high_precision() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Decimal128(38, 10));
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_decimal_builder_low_precision() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Decimal128(1, 0));
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // String/Binary Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_utf8_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Utf8);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_large_utf8_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::LargeUtf8);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_large_utf8_builder_with_lob_limit() {
        let factory = BuilderFactory::new(100).with_max_lob_bytes(Some(1_000_000));
        let builder = factory.create_builder(&DataType::LargeUtf8);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_binary_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Binary);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_large_binary_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::LargeBinary);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_large_binary_builder_with_lob_limit() {
        let factory = BuilderFactory::new(100).with_max_lob_bytes(Some(1_000_000));
        let builder = factory.create_builder(&DataType::LargeBinary);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_fixed_size_binary_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::FixedSizeBinary(8));
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_fixed_size_binary_builder_various_sizes() {
        let factory = BuilderFactory::new(100);

        let builder8 = factory.create_builder(&DataType::FixedSizeBinary(8));
        assert_eq!(builder8.len(), 0);

        let builder12 = factory.create_builder(&DataType::FixedSizeBinary(12));
        assert_eq!(builder12.len(), 0);

        let builder16 = factory.create_builder(&DataType::FixedSizeBinary(16));
        assert_eq!(builder16.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Temporal Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_date32_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Date32);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_time64_nanosecond_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Time64(TimeUnit::Nanosecond));
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_create_timestamp_nanosecond_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Timestamp(TimeUnit::Nanosecond, None));
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Boolean Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_boolean_builder() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Boolean);
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Unsupported Type Fallback Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_builder_unsupported_falls_back_to_string() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder(&DataType::Duration(TimeUnit::Second));
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Schema Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_builders_for_schema() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("price", DataType::Decimal128(18, 2), false),
        ]);

        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_for_schema(&schema);
        assert_eq!(builders.len(), 3);
    }

    #[test]
    fn test_create_builders_for_empty_schema() {
        let fields: Vec<Field> = vec![];
        let schema = Schema::new(fields);
        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_for_schema(&schema);
        assert_eq!(builders.len(), 0);
    }

    #[test]
    fn test_create_builders_for_single_field_schema() {
        let schema = Schema::new(vec![Field::new("id", DataType::Int32, false)]);
        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_for_schema(&schema);
        assert_eq!(builders.len(), 1);
    }

    #[test]
    fn test_create_builders_for_complex_schema() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("price", DataType::Decimal128(18, 2), false),
            Field::new("is_active", DataType::Boolean, false),
            Field::new("created_at", DataType::Date32, true),
            Field::new(
                "updated_at",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                true,
            ),
            Field::new("data", DataType::Binary, true),
            Field::new("notes", DataType::LargeUtf8, true),
        ]);

        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_for_schema(&schema);
        assert_eq!(builders.len(), 8);
    }

    #[test]
    fn test_create_builders_all_numeric_types() {
        let schema = Schema::new(vec![
            Field::new("tiny", DataType::UInt8, false),
            Field::new("small", DataType::Int16, false),
            Field::new("int", DataType::Int32, false),
            Field::new("big", DataType::Int64, false),
            Field::new("real", DataType::Float32, false),
            Field::new("double", DataType::Float64, false),
        ]);

        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_for_schema(&schema);
        assert_eq!(builders.len(), 6);

        for builder in &builders {
            assert_eq!(builder.len(), 0);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Clone and Debug Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_factory_clone() {
        let factory1 = BuilderFactory::new(100)
            .with_string_capacity(5000)
            .with_binary_capacity(10000)
            .with_max_lob_bytes(Some(1_000_000));
        let factory2 = factory1.clone();

        assert_eq!(factory1.capacity, factory2.capacity);
        assert_eq!(factory1.string_capacity, factory2.string_capacity);
        assert_eq!(factory1.binary_capacity, factory2.binary_capacity);
        assert_eq!(factory1.max_lob_bytes, factory2.max_lob_bytes);
    }

    #[test]
    fn test_factory_debug() {
        let factory = BuilderFactory::new(100);
        let debug_str = format!("{:?}", factory);
        assert!(debug_str.contains("BuilderFactory"));
        assert!(debug_str.contains("capacity"));
    }
}
