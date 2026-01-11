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
}

impl BuilderFactory {
    /// Create a new factory with the specified row capacity.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            string_capacity: capacity * 32, // Estimate 32 bytes per string
            binary_capacity: capacity * 64, // Estimate 64 bytes per binary
        }
    }

    /// Create from `BatchConfig`.
    #[must_use]
    pub const fn from_config(config: &BatchConfig) -> Self {
        Self {
            capacity: config.batch_size,
            string_capacity: config.string_capacity,
            binary_capacity: config.binary_capacity,
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
            DataType::LargeUtf8 => Box::new(LargeStringBuilderWrapper::new(
                self.capacity,
                self.string_capacity,
            )),

            // Binary
            DataType::Binary => Box::new(BinaryBuilderWrapper::new(
                self.capacity,
                self.binary_capacity,
            )),
            DataType::LargeBinary => Box::new(LargeBinaryBuilderWrapper::new(
                self.capacity,
                self.binary_capacity,
            )),
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
        schema
            .fields()
            .iter()
            .map(|field| self.create_builder(field.data_type()))
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
    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = BuilderFactory::new(100);
        assert_eq!(factory.capacity, 100);
        assert_eq!(factory.string_capacity, 3200);
        assert_eq!(factory.binary_capacity, 6400);
    }

    #[test]
    fn test_factory_from_config() {
        let config = BatchConfig::with_batch_size(500)
            .string_capacity(10000)
            .binary_capacity(20000);

        let factory = BuilderFactory::from_config(&config);
        assert_eq!(factory.capacity, 500);
        assert_eq!(factory.string_capacity, 10000);
        assert_eq!(factory.binary_capacity, 20000);
    }

    #[test]
    fn test_create_primitive_builders() {
        let factory = BuilderFactory::new(100);

        let _ = factory.create_builder(&DataType::Int32);
        let _ = factory.create_builder(&DataType::Float64);
        let _ = factory.create_builder(&DataType::Utf8);
    }

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
}
