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
}
