//! Sealed trait pattern for API evolution without breaking changes.
//!
//! External code can USE these traits but CANNOT implement them.
//! This allows adding methods to traits without breaking downstream crates.
//!
//! # Pattern
//!
//! ```rust,ignore
//! mod private {
//!     pub trait Sealed {}
//! }
//!
//! pub trait MyPublicTrait: private::Sealed {
//!     fn method(&self);
//! }
//!
//! // External code can use MyPublicTrait but cannot implement it
//! // because they cannot implement private::Sealed
//! ```

use arrow_schema::DataType;

/// Private module that external crates cannot access.
///
/// This module is `pub(crate)` so that implementations can be provided
/// within this crate, but external crates cannot see or implement `Sealed`.
pub(crate) mod private {
    /// Marker trait that seals the public traits.
    ///
    /// Implementations are only provided within this crate.
    /// External crates cannot implement this trait because the module is private.
    pub trait Sealed {}
}

/// Marker trait for types that can be converted from HANA values.
///
/// This trait is sealed - external implementations are not allowed.
/// Use the provided implementations for supported Arrow types.
///
/// # Sealed
///
/// This trait requires implementing [`private::Sealed`], which is not
/// accessible outside this crate. This prevents external implementations
/// and allows us to add methods without breaking changes.
pub trait FromHanaValue: private::Sealed {
    /// The Arrow data type this converter produces.
    fn arrow_type() -> DataType;

    /// Convert a HANA value to the target type.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self>
    where
        Self: Sized;
}

/// Marker trait for streaming result processors.
///
/// Sealed to ensure consistent streaming semantics across implementations.
/// Processors consume HANA rows and produce outputs in a streaming fashion.
pub trait StreamingProcessor: private::Sealed {
    /// The output type produced by this processor.
    type Output;

    /// Process a batch of rows and produce output.
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails.
    fn process_batch(&mut self, rows: Vec<hdbconnect::Row>) -> crate::Result<Self::Output>;

    /// Signal that no more rows will be provided and flush any buffered data.
    ///
    /// # Errors
    ///
    /// Returns an error if flushing fails.
    fn finish(self) -> crate::Result<Self::Output>;
}

// ═══════════════════════════════════════════════════════════════════════════
// Sealed Implementations for Primitive Types
// ═══════════════════════════════════════════════════════════════════════════

impl private::Sealed for i8 {}
impl FromHanaValue for i8 {
    fn arrow_type() -> DataType {
        DataType::Int8
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            #[allow(clippy::cast_possible_wrap)]
            HdbValue::TINYINT(v) => Ok(*v as Self),
            HdbValue::SMALLINT(v) => Self::try_from(*v).map_err(|_| {
                crate::ArrowConversionError::value_conversion("i8", "value out of range")
            }),
            HdbValue::INT(v) => Self::try_from(*v).map_err(|_| {
                crate::ArrowConversionError::value_conversion("i8", "value out of range")
            }),
            other => Err(crate::ArrowConversionError::value_conversion(
                "i8",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for i16 {}
impl FromHanaValue for i16 {
    fn arrow_type() -> DataType {
        DataType::Int16
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::TINYINT(v) => Ok(Self::from(*v)),
            HdbValue::SMALLINT(v) => Ok(*v),
            HdbValue::INT(v) => Self::try_from(*v).map_err(|_| {
                crate::ArrowConversionError::value_conversion("i16", "value out of range")
            }),
            other => Err(crate::ArrowConversionError::value_conversion(
                "i16",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for i32 {}
impl FromHanaValue for i32 {
    fn arrow_type() -> DataType {
        DataType::Int32
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::TINYINT(v) => Ok(Self::from(*v)),
            HdbValue::SMALLINT(v) => Ok(Self::from(*v)),
            HdbValue::INT(v) => Ok(*v),
            HdbValue::BIGINT(v) => Self::try_from(*v).map_err(|_| {
                crate::ArrowConversionError::value_conversion("i32", "value out of range")
            }),
            other => Err(crate::ArrowConversionError::value_conversion(
                "i32",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for i64 {}
impl FromHanaValue for i64 {
    fn arrow_type() -> DataType {
        DataType::Int64
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::TINYINT(v) => Ok(Self::from(*v)),
            HdbValue::SMALLINT(v) => Ok(Self::from(*v)),
            HdbValue::INT(v) => Ok(Self::from(*v)),
            HdbValue::BIGINT(v) => Ok(*v),
            other => Err(crate::ArrowConversionError::value_conversion(
                "i64",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for f32 {}
impl FromHanaValue for f32 {
    fn arrow_type() -> DataType {
        DataType::Float32
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::REAL(v) => Ok(*v),
            #[allow(clippy::cast_possible_truncation)]
            HdbValue::DOUBLE(v) => Ok(*v as Self),
            other => Err(crate::ArrowConversionError::value_conversion(
                "f32",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for f64 {}
impl FromHanaValue for f64 {
    fn arrow_type() -> DataType {
        DataType::Float64
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::REAL(v) => Ok(Self::from(*v)),
            HdbValue::DOUBLE(v) => Ok(*v),
            other => Err(crate::ArrowConversionError::value_conversion(
                "f64",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for bool {}
impl FromHanaValue for bool {
    fn arrow_type() -> DataType {
        DataType::Boolean
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::BOOLEAN(v) => Ok(*v),
            other => Err(crate::ArrowConversionError::value_conversion(
                "bool",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

impl private::Sealed for String {}
impl FromHanaValue for String {
    fn arrow_type() -> DataType {
        DataType::Utf8
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::STRING(s) => Ok(s.clone()),
            // Note: LOB handles are now handled differently in hdbconnect 0.32+
            other => Ok(format!("{other:?}")),
        }
    }
}

impl private::Sealed for Vec<u8> {}
impl FromHanaValue for Vec<u8> {
    fn arrow_type() -> DataType {
        DataType::Binary
    }

    fn from_hana(value: &hdbconnect::HdbValue) -> crate::Result<Self> {
        use hdbconnect::HdbValue;
        match value {
            HdbValue::BINARY(b) => Ok(b.clone()),
            other => Err(crate::ArrowConversionError::value_conversion(
                "Vec<u8>",
                format!("unexpected type: {other:?}"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_arrow_type() {
        assert_eq!(i32::arrow_type(), DataType::Int32);
    }

    #[test]
    fn test_string_arrow_type() {
        assert_eq!(String::arrow_type(), DataType::Utf8);
    }

    #[test]
    fn test_f64_arrow_type() {
        assert_eq!(f64::arrow_type(), DataType::Float64);
    }
}
