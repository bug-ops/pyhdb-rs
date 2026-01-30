//! Boolean type builder for Arrow boolean arrays.

use std::sync::Arc;

use arrow_array::ArrayRef;
use arrow_array::builder::BooleanBuilder;

use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::sealed::private::Sealed;

/// Builder for Arrow Boolean arrays (HANA BOOLEAN).
#[derive(Debug)]
pub struct BooleanBuilderWrapper {
    builder: BooleanBuilder,
    len: usize,
}

impl BooleanBuilderWrapper {
    /// Create a new boolean builder.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            builder: BooleanBuilder::with_capacity(capacity),
            len: 0,
        }
    }

    /// Create with default capacity.
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(1024)
    }
}

impl Sealed for BooleanBuilderWrapper {}

impl HanaCompatibleBuilder for BooleanBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::BOOLEAN(b) => {
                self.builder.append_value(*b);
            }
            other => {
                return Err(crate::ArrowConversionError::value_conversion(
                    "boolean",
                    format!("expected BOOLEAN, got {other:?}"),
                ));
            }
        }
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
    use arrow_array::{Array, BooleanArray};
    use hdbconnect::HdbValue;

    use super::*;

    #[test]
    fn test_boolean_builder() {
        let mut builder = BooleanBuilderWrapper::new(10);

        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        builder.append_null();
        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();

        assert_eq!(builder.len(), 3);
        let array = builder.finish();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_boolean_builder_default_capacity() {
        let builder = BooleanBuilderWrapper::default_capacity();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
        assert!(builder.capacity().is_some());
    }

    #[test]
    fn test_boolean_builder_new_with_capacity() {
        let builder = BooleanBuilderWrapper::new(100);
        assert_eq!(builder.len(), 0);
        // Capacity may be rounded up by the underlying builder
        assert!(builder.capacity().is_some());
        assert!(builder.capacity().unwrap() >= 100);
    }

    #[test]
    fn test_boolean_builder_true_values() {
        let mut builder = BooleanBuilderWrapper::new(5);
        for _ in 0..5 {
            builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        }
        assert_eq!(builder.len(), 5);
        let array = builder.finish();
        let bool_array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
        for i in 0..5 {
            assert!(bool_array.value(i));
        }
    }

    #[test]
    fn test_boolean_builder_false_values() {
        let mut builder = BooleanBuilderWrapper::new(5);
        for _ in 0..5 {
            builder
                .append_hana_value(&HdbValue::BOOLEAN(false))
                .unwrap();
        }
        let array = builder.finish();
        let bool_array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
        for i in 0..5 {
            assert!(!bool_array.value(i));
        }
    }

    #[test]
    fn test_boolean_builder_mixed_values() {
        let mut builder = BooleanBuilderWrapper::new(4);
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();

        let array = builder.finish();
        let bool_array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(bool_array.value(0));
        assert!(!bool_array.value(1));
        assert!(bool_array.value(2));
        assert!(!bool_array.value(3));
    }

    #[test]
    fn test_boolean_builder_null_values() {
        let mut builder = BooleanBuilderWrapper::new(3);
        builder.append_null();
        builder.append_null();
        builder.append_null();

        let array = builder.finish();
        let bool_array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(bool_array.is_null(0));
        assert!(bool_array.is_null(1));
        assert!(bool_array.is_null(2));
    }

    #[test]
    fn test_boolean_builder_mixed_with_nulls() {
        let mut builder = BooleanBuilderWrapper::new(5);
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        builder.append_null();
        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();
        builder.append_null();
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();

        let array = builder.finish();
        let bool_array = array.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(bool_array.value(0) && !bool_array.is_null(0));
        assert!(bool_array.is_null(1));
        assert!(!bool_array.value(2) && !bool_array.is_null(2));
        assert!(bool_array.is_null(3));
        assert!(bool_array.value(4) && !bool_array.is_null(4));
    }

    #[test]
    fn test_boolean_builder_wrong_type_int() {
        let mut builder = BooleanBuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::INT(42));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_value_conversion());
        assert!(err.to_string().contains("BOOLEAN"));
    }

    #[test]
    fn test_boolean_builder_wrong_type_string() {
        let mut builder = BooleanBuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::STRING("true".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    #[test]
    fn test_boolean_builder_wrong_type_bigint() {
        let mut builder = BooleanBuilderWrapper::new(1);
        let result = builder.append_hana_value(&HdbValue::BIGINT(1));
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    #[test]
    fn test_boolean_builder_finish_resets_len() {
        let mut builder = BooleanBuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        assert_eq!(builder.len(), 2);

        let _ = builder.finish();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_boolean_builder_reuse_after_finish() {
        let mut builder = BooleanBuilderWrapper::new(10);
        builder.append_hana_value(&HdbValue::BOOLEAN(true)).unwrap();
        let array1 = builder.finish();
        assert_eq!(array1.len(), 1);

        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();
        builder
            .append_hana_value(&HdbValue::BOOLEAN(false))
            .unwrap();
        let array2 = builder.finish();
        assert_eq!(array2.len(), 2);
    }

    #[test]
    fn test_boolean_builder_debug() {
        let builder = BooleanBuilderWrapper::new(10);
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("BooleanBuilderWrapper"));
    }

    #[test]
    fn test_boolean_builder_empty_finish() {
        let mut builder = BooleanBuilderWrapper::new(10);
        let array = builder.finish();
        assert_eq!(array.len(), 0);
    }
}
