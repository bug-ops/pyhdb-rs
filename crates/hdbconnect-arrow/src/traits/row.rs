//! Row abstraction trait for testing without HANA connection.
//!
//! This module provides the [`RowLike`] trait for abstracting over row sources,
//! enabling unit tests to use mock rows instead of requiring a HANA connection.

use hdbconnect::HdbValue;

/// Trait for types that can be used as row sources in batch processing.
///
/// Implemented by:
/// - `hdbconnect::Row` - real HANA rows
/// - `MockRow` - test rows for unit testing
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::traits::row::{RowLike, MockRow};
///
/// // Create mock rows for testing
/// let row = MockRow::new(vec![
///     HdbValue::INT(42),
///     HdbValue::STRING("hello".to_string()),
/// ]);
///
/// // Use in processor (requires process_row to accept RowLike)
/// processor.process_row_generic(&row)?;
/// ```
pub trait RowLike {
    /// Number of columns in this row.
    fn len(&self) -> usize;

    /// Returns true if this row has no columns.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get value at the specified column index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.len()`.
    fn get(&self, index: usize) -> &HdbValue<'static>;
}

// Implement for hdbconnect::Row
impl RowLike for hdbconnect::Row {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> &HdbValue<'static> {
        &self[index]
    }
}

/// Mock row for unit testing without HANA connection.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect::HdbValue;
/// use hdbconnect_arrow::traits::row::MockRow;
///
/// let row = MockRow::new(vec![
///     HdbValue::INT(1),
///     HdbValue::STRING("test".to_string()),
///     HdbValue::NULL,
/// ]);
///
/// assert_eq!(row.len(), 3);
/// assert!(!row.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct MockRow {
    values: Vec<HdbValue<'static>>,
}

impl MockRow {
    /// Create a new mock row with the specified values.
    #[must_use]
    pub const fn new(values: Vec<HdbValue<'static>>) -> Self {
        Self { values }
    }

    /// Create an empty mock row.
    #[must_use]
    pub const fn empty() -> Self {
        Self { values: Vec::new() }
    }

    /// Create a mock row with all NULL values.
    #[must_use]
    pub fn nulls(count: usize) -> Self {
        Self {
            values: vec![HdbValue::NULL; count],
        }
    }
}

impl RowLike for MockRow {
    fn len(&self) -> usize {
        self.values.len()
    }

    fn get(&self, index: usize) -> &HdbValue<'static> {
        &self.values[index]
    }
}

impl std::ops::Index<usize> for MockRow {
    type Output = HdbValue<'static>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

/// Builder for creating `MockRow` instances fluently.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::traits::row::MockRowBuilder;
///
/// let row = MockRowBuilder::new()
///     .int(42)
///     .string("hello")
///     .null()
///     .double(3.14)
///     .build();
///
/// assert_eq!(row.len(), 4);
/// ```
#[derive(Debug, Default)]
pub struct MockRowBuilder {
    values: Vec<HdbValue<'static>>,
}

impl MockRowBuilder {
    /// Create a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an integer value.
    #[must_use]
    pub fn int(mut self, value: i32) -> Self {
        self.values.push(HdbValue::INT(value));
        self
    }

    /// Add a bigint value.
    #[must_use]
    pub fn bigint(mut self, value: i64) -> Self {
        self.values.push(HdbValue::BIGINT(value));
        self
    }

    /// Add a smallint value.
    #[must_use]
    pub fn smallint(mut self, value: i16) -> Self {
        self.values.push(HdbValue::SMALLINT(value));
        self
    }

    /// Add a tinyint value.
    #[must_use]
    pub fn tinyint(mut self, value: u8) -> Self {
        self.values.push(HdbValue::TINYINT(value));
        self
    }

    /// Add a double value.
    #[must_use]
    pub fn double(mut self, value: f64) -> Self {
        self.values.push(HdbValue::DOUBLE(value));
        self
    }

    /// Add a real (float32) value.
    #[must_use]
    pub fn real(mut self, value: f32) -> Self {
        self.values.push(HdbValue::REAL(value));
        self
    }

    /// Add a string value.
    #[must_use]
    pub fn string(mut self, value: impl Into<String>) -> Self {
        self.values.push(HdbValue::STRING(value.into()));
        self
    }

    /// Add a boolean value.
    #[must_use]
    pub fn boolean(mut self, value: bool) -> Self {
        self.values.push(HdbValue::BOOLEAN(value));
        self
    }

    /// Add a NULL value.
    #[must_use]
    pub fn null(mut self) -> Self {
        self.values.push(HdbValue::NULL);
        self
    }

    /// Add a binary value.
    #[must_use]
    pub fn binary(mut self, value: Vec<u8>) -> Self {
        self.values.push(HdbValue::BINARY(value));
        self
    }

    /// Add a raw `HdbValue`.
    #[must_use]
    pub fn value(mut self, value: HdbValue<'static>) -> Self {
        self.values.push(value);
        self
    }

    /// Build the `MockRow`.
    #[must_use]
    pub fn build(self) -> MockRow {
        MockRow::new(self.values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_row_new() {
        let row = MockRow::new(vec![HdbValue::INT(1), HdbValue::STRING("test".to_string())]);
        assert_eq!(row.len(), 2);
        assert!(!row.is_empty());
    }

    #[test]
    fn test_mock_row_empty() {
        let row = MockRow::empty();
        assert_eq!(row.len(), 0);
        assert!(row.is_empty());
    }

    #[test]
    fn test_mock_row_nulls() {
        let row = MockRow::nulls(5);
        assert_eq!(row.len(), 5);
        for i in 0..5 {
            assert!(matches!(row.get(i), HdbValue::NULL));
        }
    }

    #[test]
    fn test_mock_row_index() {
        let row = MockRow::new(vec![HdbValue::INT(42)]);
        assert!(matches!(row[0], HdbValue::INT(42)));
    }

    #[test]
    fn test_mock_row_builder() {
        let row = MockRowBuilder::new()
            .int(42)
            .string("hello")
            .null()
            .double(3.14)
            .boolean(true)
            .build();

        assert_eq!(row.len(), 5);
        assert!(matches!(row.get(0), HdbValue::INT(42)));
        assert!(matches!(row.get(2), HdbValue::NULL));
        assert!(matches!(row.get(4), HdbValue::BOOLEAN(true)));
    }

    #[test]
    fn test_mock_row_builder_all_types() {
        let row = MockRowBuilder::new()
            .tinyint(1)
            .smallint(2)
            .int(3)
            .bigint(4)
            .real(1.5)
            .double(2.5)
            .string("test")
            .boolean(false)
            .null()
            .binary(vec![1, 2, 3])
            .build();

        assert_eq!(row.len(), 10);
    }

    #[test]
    fn test_row_like_trait() {
        fn process<R: RowLike>(row: &R) -> usize {
            row.len()
        }

        let mock = MockRow::new(vec![HdbValue::INT(1)]);
        assert_eq!(process(&mock), 1);
    }
}
