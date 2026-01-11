//! HANA SQL type representations with compile-time safety.
//!
//! Uses phantom types and newtypes to ensure type safety at compile time.
//! Phantom types encode type categories without runtime cost, while newtypes
//! like [`DecimalPrecision`] provide validated values.
//!
//! # Phantom Types
//!
//! The type category markers (e.g., [`Numeric`], [`Decimal`]) are used as
//! phantom type parameters to [`TypedColumn`], enabling type-safe operations
//! at compile time.
//!
//! ```rust,ignore
//! let numeric_col: TypedColumn<Numeric> = TypedColumn::new("amount", true);
//! let decimal_col: TypedColumn<Decimal> = TypedColumn::new("price", true)
//!     .with_precision(18)
//!     .with_scale(2);
//!
//! // These are different types - cannot be mixed up!
//! ```

use std::marker::PhantomData;

use crate::traits::sealed::private::Sealed;

/// Marker trait for HANA type categories.
///
/// Sealed to prevent external implementations.
/// Each category groups related HANA SQL types.
pub trait HanaTypeCategory: Sealed {
    /// The name of this category for debugging/logging.
    const CATEGORY_NAME: &'static str;
}

// ═══════════════════════════════════════════════════════════════════════════
// Type Category Markers
// ═══════════════════════════════════════════════════════════════════════════

/// Marker for numeric types (TINYINT, SMALLINT, INT, BIGINT, REAL, DOUBLE).
///
/// These types map directly to Arrow primitive arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numeric {}

impl Sealed for Numeric {}
impl HanaTypeCategory for Numeric {
    const CATEGORY_NAME: &'static str = "Numeric";
}

/// Marker for decimal types (DECIMAL, SMALLDECIMAL).
///
/// These types require precision and scale tracking and map to Arrow Decimal128.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decimal {}

impl Sealed for Decimal {}
impl HanaTypeCategory for Decimal {
    const CATEGORY_NAME: &'static str = "Decimal";
}

/// Marker for string types (CHAR, VARCHAR, NCHAR, NVARCHAR, SHORTTEXT, etc.).
///
/// All string types map to Arrow Utf8 or `LargeUtf8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringType {}

impl Sealed for StringType {}
impl HanaTypeCategory for StringType {
    const CATEGORY_NAME: &'static str = "String";
}

/// Marker for binary types (BINARY, VARBINARY).
///
/// Maps to Arrow Binary or `FixedSizeBinary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {}

impl Sealed for Binary {}
impl HanaTypeCategory for Binary {
    const CATEGORY_NAME: &'static str = "Binary";
}

/// Marker for LOB types (CLOB, NCLOB, BLOB, TEXT).
///
/// LOB types require special streaming handling for large values.
/// Maps to Arrow `LargeUtf8` or `LargeBinary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lob {}

impl Sealed for Lob {}
impl HanaTypeCategory for Lob {
    const CATEGORY_NAME: &'static str = "LOB";
}

/// Marker for temporal types (DATE, TIME, TIMESTAMP, SECONDDATE, etc.).
///
/// Maps to Arrow Date32, Time64, or Timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temporal {}

impl Sealed for Temporal {}
impl HanaTypeCategory for Temporal {
    const CATEGORY_NAME: &'static str = "Temporal";
}

/// Marker for spatial types (`ST_GEOMETRY`, `ST_POINT`).
///
/// Spatial types are serialized as WKB binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spatial {}

impl Sealed for Spatial {}
impl HanaTypeCategory for Spatial {
    const CATEGORY_NAME: &'static str = "Spatial";
}

// ═══════════════════════════════════════════════════════════════════════════
// TypedColumn with Phantom Type
// ═══════════════════════════════════════════════════════════════════════════

/// A HANA column descriptor with phantom type for category.
///
/// The phantom type parameter encodes the column's type category,
/// enabling type-safe operations at compile time.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::types::hana::{TypedColumn, Numeric, Decimal};
///
/// let numeric: TypedColumn<Numeric> = TypedColumn::new("count", false);
/// let decimal: TypedColumn<Decimal> = TypedColumn::new("price", true)
///     .with_precision(18)
///     .with_scale(2);
///
/// // Type system prevents mixing column categories
/// ```
#[derive(Debug, Clone)]
pub struct TypedColumn<C: HanaTypeCategory> {
    name: String,
    nullable: bool,
    precision: Option<u8>,
    scale: Option<i8>,
    _category: PhantomData<C>,
}

impl<C: HanaTypeCategory> TypedColumn<C> {
    /// Create a new typed column descriptor.
    ///
    /// # Arguments
    ///
    /// * `name` - Column name
    /// * `nullable` - Whether the column allows NULL values
    #[must_use]
    pub fn new(name: impl Into<String>, nullable: bool) -> Self {
        Self {
            name: name.into(),
            nullable,
            precision: None,
            scale: None,
            _category: PhantomData,
        }
    }

    /// Set precision for decimal/numeric columns.
    #[must_use]
    pub const fn with_precision(mut self, precision: u8) -> Self {
        self.precision = Some(precision);
        self
    }

    /// Set scale for decimal columns.
    #[must_use]
    pub const fn with_scale(mut self, scale: i8) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Returns the column name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether the column is nullable.
    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.nullable
    }

    /// Returns the precision, if set.
    #[must_use]
    pub const fn precision(&self) -> Option<u8> {
        self.precision
    }

    /// Returns the scale, if set.
    #[must_use]
    pub const fn scale(&self) -> Option<i8> {
        self.scale
    }

    /// Returns the type category name.
    #[must_use]
    pub const fn category_name(&self) -> &'static str {
        C::CATEGORY_NAME
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Validated Newtypes
// ═══════════════════════════════════════════════════════════════════════════

/// Validated precision for DECIMAL types.
///
/// HANA DECIMAL supports precision 1-38. This newtype ensures values
/// are validated at construction time.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::types::hana::DecimalPrecision;
///
/// let precision = DecimalPrecision::new(18)?; // OK
/// let invalid = DecimalPrecision::new(0);     // Error
/// let invalid = DecimalPrecision::new(39);    // Error
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalPrecision(u8);

impl DecimalPrecision {
    /// Maximum precision for HANA DECIMAL (38 digits).
    pub const MAX: u8 = 38;

    /// Minimum precision for HANA DECIMAL (1 digit).
    pub const MIN: u8 = 1;

    /// Create a new validated precision value.
    ///
    /// # Errors
    ///
    /// Returns an error if precision is 0 or greater than 38.
    pub fn new(precision: u8) -> crate::Result<Self> {
        if !(Self::MIN..=Self::MAX).contains(&precision) {
            return Err(crate::ArrowConversionError::invalid_precision(format!(
                "precision must be {}-{}, got {}",
                Self::MIN,
                Self::MAX,
                precision
            )));
        }
        Ok(Self(precision))
    }

    /// Create precision without validation (for internal use).
    ///
    /// # Safety
    ///
    /// Caller must ensure precision is in valid range.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) const fn new_unchecked(precision: u8) -> Self {
        Self(precision)
    }

    /// Returns the precision value.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for DecimalPrecision {
    type Error = crate::ArrowConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Validated scale for DECIMAL types.
///
/// Scale must be non-negative and not exceed precision.
/// This newtype ensures values are validated at construction time.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::types::hana::{DecimalPrecision, DecimalScale};
///
/// let precision = DecimalPrecision::new(18)?;
/// let scale = DecimalScale::new(2, precision)?;   // OK: 2 <= 18
/// let invalid = DecimalScale::new(20, precision); // Error: 20 > 18
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalScale(i8);

impl DecimalScale {
    /// Create a new validated scale value.
    ///
    /// # Arguments
    ///
    /// * `scale` - The scale value (must be non-negative)
    /// * `precision` - The precision this scale is associated with
    ///
    /// # Errors
    ///
    /// Returns an error if scale is negative or exceeds precision.
    pub fn new(scale: i8, precision: DecimalPrecision) -> crate::Result<Self> {
        if scale < 0 {
            return Err(crate::ArrowConversionError::invalid_scale(format!(
                "scale must be non-negative, got {scale}"
            )));
        }
        // Safe: scale is already checked to be non-negative
        #[allow(clippy::cast_sign_loss)]
        if scale as u8 > precision.value() {
            return Err(crate::ArrowConversionError::invalid_scale(format!(
                "scale ({}) cannot exceed precision ({})",
                scale,
                precision.value()
            )));
        }
        Ok(Self(scale))
    }

    /// Create scale without validation (for internal use).
    ///
    /// # Safety
    ///
    /// Caller must ensure scale is valid for the associated precision.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) const fn new_unchecked(scale: i8) -> Self {
        Self(scale)
    }

    /// Returns the scale value.
    #[must_use]
    pub const fn value(self) -> i8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_precision_valid() {
        assert!(DecimalPrecision::new(1).is_ok());
        assert!(DecimalPrecision::new(18).is_ok());
        assert!(DecimalPrecision::new(38).is_ok());
    }

    #[test]
    fn test_decimal_precision_invalid() {
        assert!(DecimalPrecision::new(0).is_err());
        assert!(DecimalPrecision::new(39).is_err());
    }

    #[test]
    fn test_decimal_scale_valid() {
        let prec = DecimalPrecision::new(18).unwrap();
        assert!(DecimalScale::new(0, prec).is_ok());
        assert!(DecimalScale::new(2, prec).is_ok());
        assert!(DecimalScale::new(18, prec).is_ok());
    }

    #[test]
    fn test_decimal_scale_invalid() {
        let prec = DecimalPrecision::new(18).unwrap();
        assert!(DecimalScale::new(-1, prec).is_err());
        assert!(DecimalScale::new(19, prec).is_err());
    }

    #[test]
    fn test_typed_column() {
        let col: TypedColumn<Numeric> = TypedColumn::new("amount", false);
        assert_eq!(col.name(), "amount");
        assert!(!col.nullable());
        assert_eq!(col.category_name(), "Numeric");
    }

    #[test]
    fn test_typed_column_decimal() {
        let col: TypedColumn<Decimal> = TypedColumn::new("price", true)
            .with_precision(18)
            .with_scale(2);

        assert_eq!(col.name(), "price");
        assert!(col.nullable());
        assert_eq!(col.precision(), Some(18));
        assert_eq!(col.scale(), Some(2));
        assert_eq!(col.category_name(), "Decimal");
    }

    #[test]
    fn test_category_names() {
        assert_eq!(Numeric::CATEGORY_NAME, "Numeric");
        assert_eq!(Decimal::CATEGORY_NAME, "Decimal");
        assert_eq!(StringType::CATEGORY_NAME, "String");
        assert_eq!(Binary::CATEGORY_NAME, "Binary");
        assert_eq!(Lob::CATEGORY_NAME, "LOB");
        assert_eq!(Temporal::CATEGORY_NAME, "Temporal");
        assert_eq!(Spatial::CATEGORY_NAME, "Spatial");
    }
}
