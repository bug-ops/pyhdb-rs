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
    use super::*;

    // Test that the trait is object-safe
    fn _assert_object_safe(_: &dyn HanaCompatibleBuilder) {}
}
