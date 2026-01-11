//! Builder traits for Arrow array construction.
//!
//! This module defines the [`HanaCompatibleBuilder`] trait that all Arrow
//! builders must implement to accept HANA values.

use arrow_array::ArrayRef;

use super::sealed::private::Sealed;

/// Marker trait for Arrow builders that can accept HANA values.
///
/// This trait is sealed to prevent external implementations that might
/// violate invariants around null handling and type safety.
///
/// # Implementors
///
/// This trait is implemented by wrapper types in the `builders` module:
/// - `UInt8BuilderWrapper`
/// - `Int16BuilderWrapper`
/// - `StringBuilderWrapper`
/// - etc.
///
/// # Thread Safety
///
/// Implementations must be `Send` to allow parallel batch processing.
pub trait HanaCompatibleBuilder: Sealed + Send {
    /// Append a HANA value to this builder.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to the target type.
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> crate::Result<()>;

    /// Append a null value to this builder.
    fn append_null(&mut self);

    /// Finish building and return the Arrow array.
    ///
    /// After calling this method, the builder is reset and can be reused.
    fn finish(&mut self) -> ArrayRef;

    /// Reset the builder, clearing all data while preserving capacity.
    ///
    /// This is more efficient than calling `finish()` when you want to
    /// reuse the builder without creating an array. Useful for batch
    /// boundary resets where the previous batch data is discarded.
    fn reset(&mut self) {
        // Default implementation: call finish() and discard the result
        let _ = self.finish();
    }

    /// Returns the number of values (including nulls) appended so far.
    fn len(&self) -> usize;

    /// Returns true if no values have been appended.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity hint for this builder, if known.
    fn capacity(&self) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use hdbconnect::HdbValue;

    use super::*;
    use crate::builders::boolean::BooleanBuilderWrapper;
    use crate::builders::primitive::{Float64BuilderWrapper, Int32BuilderWrapper};
    use crate::builders::string::StringBuilderWrapper;

    // Test that the trait is object-safe
    fn _assert_object_safe(_: &dyn HanaCompatibleBuilder) {}

    // ═══════════════════════════════════════════════════════════════════════════
    // is_empty Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_is_empty_initial() {
        let builder = Int32BuilderWrapper::new(10);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_is_empty_after_append() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(42)).unwrap();
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_is_empty_after_append_null() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_null();
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_is_empty_after_finish() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(42)).unwrap();
        let _ = builder.finish();
        assert!(builder.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // reset Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_reset_clears_builder() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(42)).unwrap();
        builder.append_hana_value(&HdbValue::INT(43)).unwrap();
        assert_eq!(builder.len(), 2);

        builder.reset();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_reset_on_empty_builder() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.reset();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_reset_allows_reuse() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(42)).unwrap();
        builder.reset();

        builder.append_hana_value(&HdbValue::INT(100)).unwrap();
        builder.append_hana_value(&HdbValue::INT(200)).unwrap();
        assert_eq!(builder.len(), 2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // capacity Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_capacity_returns_some_for_int_builder() {
        let builder = Int32BuilderWrapper::new(100);
        assert!(builder.capacity().is_some());
        assert_eq!(builder.capacity(), Some(100));
    }

    #[test]
    fn test_capacity_returns_some_for_float_builder() {
        let builder = Float64BuilderWrapper::new(50);
        assert!(builder.capacity().is_some());
    }

    #[test]
    fn test_capacity_returns_none_for_string_builder() {
        let builder = StringBuilderWrapper::new(10, 100);
        assert!(builder.capacity().is_none());
    }

    #[test]
    fn test_capacity_returns_some_for_boolean_builder() {
        let builder = BooleanBuilderWrapper::new(200);
        assert!(builder.capacity().is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Object Safety Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_object_safety_with_boxed_builder() {
        let builder: Box<dyn HanaCompatibleBuilder> = Box::new(Int32BuilderWrapper::new(10));
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_object_safety_multiple_types() {
        let mut builders: Vec<Box<dyn HanaCompatibleBuilder>> = vec![
            Box::new(Int32BuilderWrapper::new(10)),
            Box::new(Float64BuilderWrapper::new(10)),
            Box::new(StringBuilderWrapper::new(10, 100)),
            Box::new(BooleanBuilderWrapper::new(10)),
        ];

        for builder in &builders {
            assert!(builder.is_empty());
        }

        for builder in &mut builders {
            builder.append_null();
            assert!(!builder.is_empty());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // len Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_len_increments_with_values() {
        let mut builder = Int32BuilderWrapper::new(10);
        assert_eq!(builder.len(), 0);

        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        assert_eq!(builder.len(), 1);

        builder.append_hana_value(&HdbValue::INT(2)).unwrap();
        assert_eq!(builder.len(), 2);

        builder.append_hana_value(&HdbValue::INT(3)).unwrap();
        assert_eq!(builder.len(), 3);
    }

    #[test]
    fn test_len_includes_nulls() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        builder.append_null();
        builder.append_hana_value(&HdbValue::INT(3)).unwrap();
        assert_eq!(builder.len(), 3);
    }

    #[test]
    fn test_len_resets_after_finish() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        builder.append_hana_value(&HdbValue::INT(2)).unwrap();
        assert_eq!(builder.len(), 2);

        let _ = builder.finish();
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // finish Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_finish_returns_array_with_correct_length() {
        let mut builder = Int32BuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        builder.append_hana_value(&HdbValue::INT(2)).unwrap();
        builder.append_hana_value(&HdbValue::INT(3)).unwrap();

        let array = builder.finish();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_finish_empty_builder_returns_empty_array() {
        let mut builder = Int32BuilderWrapper::new(10);
        let array = builder.finish();
        assert_eq!(array.len(), 0);
    }

    #[test]
    fn test_finish_multiple_times() {
        let mut builder = Int32BuilderWrapper::new(10);

        builder.append_hana_value(&HdbValue::INT(1)).unwrap();
        let array1 = builder.finish();
        assert_eq!(array1.len(), 1);

        builder.append_hana_value(&HdbValue::INT(2)).unwrap();
        builder.append_hana_value(&HdbValue::INT(3)).unwrap();
        let array2 = builder.finish();
        assert_eq!(array2.len(), 2);
    }
}
