//! Conversion between Python and HANA types.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyFloat, PyInt, PyString};

use crate::error::PyHdbError;

/// Convert a HANA value to a Python object.
pub fn hana_value_to_python<'py>(
    py: Python<'py>,
    value: &hdbconnect::HdbValue,
) -> PyResult<Bound<'py, PyAny>> {
    use hdbconnect::HdbValue;

    match value {
        HdbValue::NULL => Ok(py.None().into_bound(py)),
        HdbValue::BOOLEAN(b) => Ok(b.into_pyobject(py)?.to_owned().into_any()),
        HdbValue::TINYINT(v) => Ok(v.into_pyobject(py)?.clone().into_any()),
        HdbValue::SMALLINT(v) => Ok(v.into_pyobject(py)?.clone().into_any()),
        HdbValue::INT(v) => Ok(v.into_pyobject(py)?.clone().into_any()),
        HdbValue::BIGINT(v) => Ok(v.into_pyobject(py)?.clone().into_any()),
        HdbValue::REAL(v) => Ok(v.into_pyobject(py)?.clone().into_any()),
        HdbValue::DOUBLE(v) => Ok(v.into_pyobject(py)?.clone().into_any()),
        HdbValue::STRING(s) => Ok(s.into_pyobject(py)?.clone().into_any()),
        HdbValue::BINARY(b) => Ok(PyBytes::new(py, b).clone().into_any()),
        // Decimal: convert to Python Decimal
        HdbValue::DECIMAL(d) => {
            let decimal_mod = py.import("decimal")?;
            let decimal_cls = decimal_mod.getattr("Decimal")?;
            let s = d.to_string();
            decimal_cls.call1((s,))
        }
        // Date/Time: convert to string for now
        // TODO: Convert to Python datetime objects
        other => {
            let s = format!("{other:?}");
            Ok(s.into_pyobject(py)?.clone().into_any())
        }
    }
}

/// Convert a Python object to a HANA value.
///
/// # Errors
///
/// Returns error if conversion is not possible.
pub fn python_to_hana_value(obj: &Bound<'_, PyAny>) -> PyResult<hdbconnect::HdbValue<'static>> {
    use hdbconnect::HdbValue;

    if obj.is_none() {
        return Ok(HdbValue::NULL);
    }

    // Check Python type and convert
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(HdbValue::BOOLEAN(b));
    }

    if obj.is_instance_of::<PyInt>() {
        let v: i64 = obj.extract()?;
        return Ok(HdbValue::BIGINT(v));
    }

    if obj.is_instance_of::<PyFloat>() {
        let v: f64 = obj.extract()?;
        return Ok(HdbValue::DOUBLE(v));
    }

    if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        return Ok(HdbValue::STRING(s));
    }

    if obj.is_instance_of::<PyBytes>() {
        let b: Vec<u8> = obj.extract()?;
        return Ok(HdbValue::BINARY(b));
    }

    // Unsupported type
    Err(PyHdbError::data(format!(
        "cannot convert Python type {} to HANA value",
        obj.get_type().name()?
    ))
    .into())
}

#[cfg(test)]
mod tests {
    // Tests require Python runtime
}
