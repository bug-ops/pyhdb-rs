//! Parameter validation utilities.

use pyo3::prelude::*;

use crate::error::PyHdbError;

/// Lightweight validation query for connection health checks.
///
/// SAP HANA's `DUMMY` table is equivalent to Oracle's `DUAL` - a special
/// single-row, single-column table designed for this purpose.
pub const VALIDATION_QUERY: &str = "SELECT 1 FROM DUMMY";

/// Validates that a u32 parameter is positive (greater than 0).
///
/// # Arguments
///
/// * `value` - The value to validate
/// * `param_name` - The parameter name for error messages
///
/// # Errors
///
/// Returns `ProgrammingError` if value is 0.
pub fn validate_positive_u32(value: u32, param_name: &str) -> PyResult<()> {
    if value == 0 {
        return Err(PyHdbError::programming(format!("{param_name} must be > 0")).into());
    }
    Ok(())
}

/// Validates that an optional f64 parameter is non-negative.
///
/// # Arguments
///
/// * `value` - The optional value to validate
/// * `param_name` - The parameter name for error messages
///
/// # Errors
///
/// Returns `ProgrammingError` if value is negative.
pub fn validate_non_negative_f64(value: Option<f64>, param_name: &str) -> PyResult<()> {
    if let Some(v) = value
        && v < 0.0
    {
        return Err(PyHdbError::programming(format!("{param_name} cannot be negative")).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_query_constant() {
        assert_eq!(VALIDATION_QUERY, "SELECT 1 FROM DUMMY");
    }

    #[test]
    fn test_validate_positive_u32_valid() {
        assert!(validate_positive_u32(1, "test").is_ok());
        assert!(validate_positive_u32(100, "test").is_ok());
    }

    #[test]
    fn test_validate_positive_u32_zero() {
        assert!(validate_positive_u32(0, "test").is_err());
    }

    #[test]
    fn test_validate_non_negative_f64_valid() {
        assert!(validate_non_negative_f64(None, "test").is_ok());
        assert!(validate_non_negative_f64(Some(0.0), "test").is_ok());
        assert!(validate_non_negative_f64(Some(1.5), "test").is_ok());
    }

    #[test]
    fn test_validate_non_negative_f64_negative() {
        assert!(validate_non_negative_f64(Some(-1.0), "test").is_err());
    }
}
