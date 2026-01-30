//! Type-safe builder factory using phantom types.
//!
//! The factory pattern ensures that builders are created with correct
//! configurations for each Arrow data type.

use arrow_schema::{DataType, Field, Schema, TimeUnit};

use super::boolean::BooleanBuilderWrapper;
use super::decimal::Decimal128BuilderWrapper;
use super::dispatch::BuilderEnum;
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
        schema: &Schema,
    ) -> Vec<Box<dyn HanaCompatibleBuilder>> {
        let fields = schema.fields();
        let mut builders = Vec::with_capacity(fields.len());
        for field in fields {
            builders.push(self.create_builder(field.data_type()));
        }
        builders
    }

    /// Create an enum-wrapped builder for the specified Arrow data type.
    ///
    /// # Performance
    ///
    /// Prefer this over `create_builder()` when using `HanaBatchProcessor`.
    /// Enum dispatch eliminates vtable overhead from `Box<dyn HanaCompatibleBuilder>`:
    ///
    /// - **No pointer indirection**: Enum is inline in `Vec<BuilderEnum>`
    /// - **No vtable lookup**: Match dispatch is direct jump
    /// - **Better cache locality**: Contiguous memory layout
    /// - **Compiler optimization**: Enables inlining and monomorphization
    ///
    /// # Box Wrapping Strategy
    ///
    /// Large builder variants (String, Binary, Decimal) are wrapped in `Box` to reduce
    /// overall enum size. This improves cache performance for frequently-used small
    /// builders (primitives, temporal types) by allowing more enum instances to fit
    /// in CPU cache lines. The Box indirection overhead (<1%) is negligible compared
    /// to the cost of string/decimal conversions (~100-1000ns).
    ///
    /// Expected performance gain: 10-20% on typical workloads.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn create_builder_enum(&self, data_type: &DataType) -> BuilderEnum {
        match data_type {
            // ═══════════════════════════════════════════════════════════════════
            // Inline small variants (primitive and temporal types)
            // ═══════════════════════════════════════════════════════════════════
            DataType::UInt8 => BuilderEnum::UInt8(UInt8BuilderWrapper::new(self.capacity)),
            DataType::Int16 => BuilderEnum::Int16(Int16BuilderWrapper::new(self.capacity)),
            DataType::Int32 => BuilderEnum::Int32(Int32BuilderWrapper::new(self.capacity)),
            DataType::Int64 => BuilderEnum::Int64(Int64BuilderWrapper::new(self.capacity)),
            DataType::Float32 => BuilderEnum::Float32(Float32BuilderWrapper::new(self.capacity)),
            DataType::Float64 => BuilderEnum::Float64(Float64BuilderWrapper::new(self.capacity)),
            DataType::Boolean => BuilderEnum::Boolean(BooleanBuilderWrapper::new(self.capacity)),
            DataType::Date32 => BuilderEnum::Date32(Date32BuilderWrapper::new(self.capacity)),
            DataType::Time64(TimeUnit::Nanosecond) => {
                BuilderEnum::Time64Nanosecond(Time64NanosecondBuilderWrapper::new(self.capacity))
            }
            DataType::Timestamp(TimeUnit::Nanosecond, None) => BuilderEnum::TimestampNanosecond(
                TimestampNanosecondBuilderWrapper::new(self.capacity),
            ),

            // ═══════════════════════════════════════════════════════════════════
            // Boxed large variants (>32 bytes inner size)
            // ═══════════════════════════════════════════════════════════════════
            DataType::Decimal128(precision, scale) => BuilderEnum::Decimal128(Box::new(
                Decimal128BuilderWrapper::new(self.capacity, *precision, *scale),
            )),
            DataType::Utf8 => BuilderEnum::Utf8(Box::new(StringBuilderWrapper::new(
                self.capacity,
                self.string_capacity,
            ))),
            DataType::LargeUtf8 => {
                let mut builder =
                    LargeStringBuilderWrapper::new(self.capacity, self.string_capacity);
                if let Some(max) = self.max_lob_bytes {
                    builder = builder.with_max_lob_bytes(max);
                }
                BuilderEnum::LargeUtf8(Box::new(builder))
            }
            DataType::Binary => BuilderEnum::Binary(Box::new(BinaryBuilderWrapper::new(
                self.capacity,
                self.binary_capacity,
            ))),
            DataType::LargeBinary => {
                let mut builder =
                    LargeBinaryBuilderWrapper::new(self.capacity, self.binary_capacity);
                if let Some(max) = self.max_lob_bytes {
                    builder = builder.with_max_lob_bytes(max);
                }
                BuilderEnum::LargeBinary(Box::new(builder))
            }
            DataType::FixedSizeBinary(size) => BuilderEnum::FixedSizeBinary(Box::new(
                FixedSizeBinaryBuilderWrapper::new(self.capacity, *size),
            )),

            // Unsupported - fallback to string (boxed)
            _ => BuilderEnum::Utf8(Box::new(StringBuilderWrapper::new(
                self.capacity,
                self.string_capacity,
            ))),
        }
    }

    /// Create enum builders for all fields in a schema.
    ///
    /// Returns a vector of enum-wrapped builders in the same order as schema fields.
    /// Prefer this over `create_builders_for_schema()` for better performance.
    #[must_use]
    pub fn create_builders_enum_for_schema(&self, schema: &Schema) -> Vec<BuilderEnum> {
        schema
            .fields()
            .iter()
            .map(|field| self.create_builder_enum(field.data_type()))
            .collect()
    }

    /// Calculate safe string data capacity from field metadata.
    ///
    /// Reads `max_length` from field metadata and applies Unicode multiplier:
    /// - VARCHAR (Utf8): 1x multiplier
    /// - NVARCHAR (LargeUtf8): 4x multiplier (worst-case UTF-8 encoding)
    ///
    /// Clamps result to `[4 KB, 256 MB]` to prevent overflow and excessive memory.
    fn calculate_string_data_capacity(&self, field: &Field) -> usize {
        const MIN_CAPACITY: usize = 4 * 1024; // 4 KB
        const MAX_CAPACITY: usize = 256 * 1024 * 1024; // 256 MB

        // Read max_length from field metadata
        let max_length = field
            .metadata()
            .get("max_length")
            .and_then(|s| s.parse::<usize>().ok());

        let max_length = match max_length {
            Some(len) if len > 0 => len,
            _ => return self.string_capacity, // Fallback to default
        };

        // Unicode multiplier: 4x for LargeUtf8 (NVARCHAR), 1x for Utf8 (VARCHAR)
        let multiplier = match field.data_type() {
            DataType::Utf8 => 1,
            DataType::LargeUtf8 => 4,
            _ => return 0,
        };

        // Calculate with overflow protection
        max_length
            .saturating_mul(multiplier)
            .saturating_mul(self.capacity)
            .clamp(MIN_CAPACITY, MAX_CAPACITY)
    }

    /// Create builder enum with capacity hints from field metadata.
    ///
    /// For string types (Utf8, `LargeUtf8`), uses `max_length` metadata to calculate
    /// optimal data capacity. Falls back to `create_builder_enum()` for other types.
    #[must_use]
    pub fn create_builder_enum_for_field(&self, field: &Field) -> BuilderEnum {
        match field.data_type() {
            DataType::Utf8 => {
                let capacity = self.calculate_string_data_capacity(field);
                BuilderEnum::Utf8(Box::new(StringBuilderWrapper::with_capacity(
                    self.capacity,
                    capacity,
                )))
            }
            DataType::LargeUtf8 => {
                let capacity = self.calculate_string_data_capacity(field);
                let mut builder = LargeStringBuilderWrapper::with_capacity(self.capacity, capacity);
                if let Some(max) = self.max_lob_bytes {
                    builder = builder.with_max_lob_bytes(max);
                }
                BuilderEnum::LargeUtf8(Box::new(builder))
            }
            _ => {
                // Delegate to existing method for non-string types
                self.create_builder_enum(field.data_type())
            }
        }
    }

    /// Create builders for schema with metadata support.
    ///
    /// Uses field metadata (e.g., `max_length`) to optimize string buffer pre-sizing.
    /// Prefer this over `create_builders_enum_for_schema()` when schema has metadata.
    #[must_use]
    pub fn create_builders_enum_for_schema_with_metadata(
        &self,
        schema: &Schema,
    ) -> Vec<BuilderEnum> {
        schema
            .fields()
            .iter()
            .map(|field| self.create_builder_enum_for_field(field))
            .collect()
    }
}

impl Default for BuilderFactory {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use arrow_schema::{DataType, Field, Schema, TimeUnit};

    use super::*;
    use crate::builders::BuilderKind;

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

    // ═══════════════════════════════════════════════════════════════════════════
    // Enum Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_builder_enum_uint8() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::UInt8);
        assert_eq!(builder.kind(), BuilderKind::UInt8);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_create_builder_enum_int16() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Int16);
        assert_eq!(builder.kind(), BuilderKind::Int16);
    }

    #[test]
    fn test_create_builder_enum_int32() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Int32);
        assert_eq!(builder.kind(), BuilderKind::Int32);
    }

    #[test]
    fn test_create_builder_enum_int64() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Int64);
        assert_eq!(builder.kind(), BuilderKind::Int64);
    }

    #[test]
    fn test_create_builder_enum_float32() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Float32);
        assert_eq!(builder.kind(), BuilderKind::Float32);
    }

    #[test]
    fn test_create_builder_enum_float64() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Float64);
        assert_eq!(builder.kind(), BuilderKind::Float64);
    }

    #[test]
    fn test_create_builder_enum_decimal128() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Decimal128(18, 2));
        assert_eq!(builder.kind(), BuilderKind::Decimal128);
    }

    #[test]
    fn test_create_builder_enum_boolean() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Boolean);
        assert_eq!(builder.kind(), BuilderKind::Boolean);
    }

    #[test]
    fn test_create_builder_enum_utf8() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Utf8);
        assert_eq!(builder.kind(), BuilderKind::Utf8);
    }

    #[test]
    fn test_create_builder_enum_large_utf8() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::LargeUtf8);
        assert_eq!(builder.kind(), BuilderKind::LargeUtf8);
    }

    #[test]
    fn test_create_builder_enum_large_utf8_with_lob_limit() {
        let factory = BuilderFactory::new(100).with_max_lob_bytes(Some(1_000_000));
        let builder = factory.create_builder_enum(&DataType::LargeUtf8);
        assert_eq!(builder.kind(), BuilderKind::LargeUtf8);
    }

    #[test]
    fn test_create_builder_enum_binary() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Binary);
        assert_eq!(builder.kind(), BuilderKind::Binary);
    }

    #[test]
    fn test_create_builder_enum_large_binary() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::LargeBinary);
        assert_eq!(builder.kind(), BuilderKind::LargeBinary);
    }

    #[test]
    fn test_create_builder_enum_fixed_size_binary() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::FixedSizeBinary(16));
        assert_eq!(builder.kind(), BuilderKind::FixedSizeBinary);
    }

    #[test]
    fn test_create_builder_enum_date32() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Date32);
        assert_eq!(builder.kind(), BuilderKind::Date32);
    }

    #[test]
    fn test_create_builder_enum_time64_nanosecond() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Time64(TimeUnit::Nanosecond));
        assert_eq!(builder.kind(), BuilderKind::Time64Nanosecond);
    }

    #[test]
    fn test_create_builder_enum_timestamp_nanosecond() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Timestamp(TimeUnit::Nanosecond, None));
        assert_eq!(builder.kind(), BuilderKind::TimestampNanosecond);
    }

    #[test]
    fn test_create_builder_enum_unsupported_falls_back_to_utf8() {
        let factory = BuilderFactory::new(100);
        let builder = factory.create_builder_enum(&DataType::Duration(TimeUnit::Second));
        assert_eq!(builder.kind(), BuilderKind::Utf8);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Enum Schema Builder Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_create_builders_enum_for_schema() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("price", DataType::Decimal128(18, 2), false),
        ]);

        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_enum_for_schema(&schema);

        assert_eq!(builders.len(), 3);
        assert_eq!(builders[0].kind(), BuilderKind::Int32);
        assert_eq!(builders[1].kind(), BuilderKind::Utf8);
        assert_eq!(builders[2].kind(), BuilderKind::Decimal128);
    }

    #[test]
    fn test_create_builders_enum_for_empty_schema() {
        let fields: Vec<Field> = vec![];
        let schema = Schema::new(fields);
        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_enum_for_schema(&schema);
        assert_eq!(builders.len(), 0);
    }

    #[test]
    fn test_create_builders_enum_for_homogeneous_schema() {
        let schema = Schema::new(vec![
            Field::new("col1", DataType::Int64, false),
            Field::new("col2", DataType::Int64, false),
            Field::new("col3", DataType::Int64, false),
            Field::new("col4", DataType::Int64, false),
        ]);

        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_enum_for_schema(&schema);

        assert_eq!(builders.len(), 4);
        for builder in &builders {
            assert_eq!(builder.kind(), BuilderKind::Int64);
        }
    }

    #[test]
    fn test_create_builders_enum_all_types() {
        let schema = Schema::new(vec![
            Field::new("uint8", DataType::UInt8, false),
            Field::new("int16", DataType::Int16, false),
            Field::new("int32", DataType::Int32, false),
            Field::new("int64", DataType::Int64, false),
            Field::new("float32", DataType::Float32, false),
            Field::new("float64", DataType::Float64, false),
            Field::new("decimal", DataType::Decimal128(18, 2), false),
            Field::new("boolean", DataType::Boolean, false),
            Field::new("utf8", DataType::Utf8, true),
            Field::new("large_utf8", DataType::LargeUtf8, true),
            Field::new("binary", DataType::Binary, true),
            Field::new("large_binary", DataType::LargeBinary, true),
            Field::new("fixed_binary", DataType::FixedSizeBinary(8), true),
            Field::new("date32", DataType::Date32, true),
            Field::new("time64", DataType::Time64(TimeUnit::Nanosecond), true),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                true,
            ),
        ]);

        let factory = BuilderFactory::new(100);
        let builders = factory.create_builders_enum_for_schema(&schema);

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
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // String Capacity Pre-sizing Tests (Phase 6)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_calculate_string_data_capacity_varchar() {
        let factory = BuilderFactory::new(1000);
        let mut metadata = HashMap::new();
        metadata.insert("max_length".to_string(), "100".to_string());
        let field = Field::new("test", DataType::Utf8, false).with_metadata(metadata);

        let capacity = factory.calculate_string_data_capacity(&field);
        assert_eq!(capacity, 100 * 1 * 1000); // max_length * multiplier * batch_size
    }

    #[test]
    fn test_calculate_string_data_capacity_nvarchar() {
        let factory = BuilderFactory::new(1000);
        let mut metadata = HashMap::new();
        metadata.insert("max_length".to_string(), "100".to_string());
        let field = Field::new("test", DataType::LargeUtf8, false).with_metadata(metadata);

        let capacity = factory.calculate_string_data_capacity(&field);
        assert_eq!(capacity, 100 * 4 * 1000); // 4x multiplier for Unicode
    }

    #[test]
    fn test_calculate_string_data_capacity_overflow() {
        let factory = BuilderFactory::new(65536);
        let mut metadata = HashMap::new();
        metadata.insert("max_length".to_string(), "100000".to_string());
        let field = Field::new("test", DataType::Utf8, false).with_metadata(metadata);

        let capacity = factory.calculate_string_data_capacity(&field);
        assert_eq!(capacity, 256 * 1024 * 1024); // Clamped to MAX
    }

    #[test]
    fn test_calculate_string_data_capacity_missing_metadata() {
        let factory = BuilderFactory::new(1000);
        let field = Field::new("test", DataType::Utf8, false);

        let capacity = factory.calculate_string_data_capacity(&field);
        assert_eq!(capacity, factory.string_capacity); // Falls back
    }

    #[test]
    fn test_calculate_string_data_capacity_min_clamp() {
        let factory = BuilderFactory::new(10);
        let mut metadata = HashMap::new();
        metadata.insert("max_length".to_string(), "1".to_string());
        let field = Field::new("test", DataType::Utf8, false).with_metadata(metadata);

        let capacity = factory.calculate_string_data_capacity(&field);
        assert_eq!(capacity, 4 * 1024); // Clamped to MIN_CAPACITY
    }

    #[test]
    fn test_create_builder_enum_for_field_varchar_with_metadata() {
        let factory = BuilderFactory::new(1000);
        let mut metadata = HashMap::new();
        metadata.insert("max_length".to_string(), "50".to_string());
        let field = Field::new("name", DataType::Utf8, false).with_metadata(metadata);

        let builder = factory.create_builder_enum_for_field(&field);
        assert_eq!(builder.kind(), BuilderKind::Utf8);
    }

    #[test]
    fn test_create_builder_enum_for_field_nvarchar_with_metadata() {
        let factory = BuilderFactory::new(1000);
        let mut metadata = HashMap::new();
        metadata.insert("max_length".to_string(), "50".to_string());
        let field = Field::new("name", DataType::LargeUtf8, false).with_metadata(metadata);

        let builder = factory.create_builder_enum_for_field(&field);
        assert_eq!(builder.kind(), BuilderKind::LargeUtf8);
    }

    #[test]
    fn test_create_builders_enum_for_schema_with_metadata() {
        let mut varchar_meta = HashMap::new();
        varchar_meta.insert("max_length".to_string(), "100".to_string());
        let mut nvarchar_meta = HashMap::new();
        nvarchar_meta.insert("max_length".to_string(), "200".to_string());

        let schema = Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false).with_metadata(varchar_meta),
            Field::new("description", DataType::LargeUtf8, true).with_metadata(nvarchar_meta),
        ]);

        let factory = BuilderFactory::new(1000);
        let builders = factory.create_builders_enum_for_schema_with_metadata(&schema);

        assert_eq!(builders.len(), 3);
        assert_eq!(builders[0].kind(), BuilderKind::Int32);
        assert_eq!(builders[1].kind(), BuilderKind::Utf8);
        assert_eq!(builders[2].kind(), BuilderKind::LargeUtf8);
    }
}
