//! Conversion between Python and HANA types.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyFloat, PyInt, PyString};

use super::cache::{get_date_cls, get_datetime_cls, get_decimal_cls, get_time_cls};
use crate::error::PyHdbError;

/// Parse ISO timestamp string to Python datetime.
///
/// Parses HANA timestamp format "YYYY-MM-DDTHH:MM:SS[.FFFFFFF]" to Python datetime.
/// On parse failures, returns the original string rather than failing, since this
/// is display/conversion code where partial results are acceptable.
fn parse_timestamp_to_python<'py>(py: Python<'py>, s: &str) -> PyResult<Bound<'py, PyAny>> {
    let datetime_cls = get_datetime_cls(py)?;

    // Display format: "YYYY-MM-DDTHH:MM:SS[.FFFFFFF]"
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() == 2 {
        let date_parts: Vec<&str> = parts[0].split('-').collect();
        let time_str = parts[1];
        let (time_part, microseconds) =
            time_str
                .split_once('.')
                .map_or((time_str, 0u32), |(t, frac)| {
                    // Convert fractional seconds (up to 7 digits) to microseconds (6 digits)
                    let padded = format!("{frac:0<6}");
                    let micros: u32 = padded[..6].parse().unwrap_or(0);
                    (t, micros)
                });
        let time_parts: Vec<&str> = time_part.split(':').collect();

        if date_parts.len() == 3 && time_parts.len() == 3 {
            // Fallback to epoch/zero values on parse failure - this is intentional
            // rather than failing the entire conversion. Invalid dates will produce
            // unusual but valid datetime objects (e.g., 1970-01-01 00:00:00).
            let year: i32 = date_parts[0].parse().unwrap_or(1970);
            let month: u32 = date_parts[1].parse().unwrap_or(1);
            let day: u32 = date_parts[2].parse().unwrap_or(1);
            let hour: u32 = time_parts[0].parse().unwrap_or(0);
            let minute: u32 = time_parts[1].parse().unwrap_or(0);
            let second: u32 = time_parts[2].parse().unwrap_or(0);
            return datetime_cls.call1((year, month, day, hour, minute, second, microseconds));
        }
    }
    // Fallback to string representation for unparseable formats
    Ok(s.into_pyobject(py)?.clone().into_any())
}

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
            let decimal_cls = get_decimal_cls(py)?;
            let s = d.to_string();
            decimal_cls.call1((s,))
        }
        // Date: convert to Python datetime.date
        HdbValue::DAYDATE(d) => {
            let date_cls = get_date_cls(py)?;
            // DayDate displays as "YYYY-MM-DD"
            let s = d.to_string();
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() == 3 {
                // Fallback to epoch date on parse failure (see parse_timestamp_to_python)
                let year: i32 = parts[0].parse().unwrap_or(1970);
                let month: u32 = parts[1].parse().unwrap_or(1);
                let day: u32 = parts[2].parse().unwrap_or(1);
                date_cls.call1((year, month, day))
            } else {
                // Fallback to string for non-standard formats
                Ok(s.into_pyobject(py)?.clone().into_any())
            }
        }
        // Time: convert to Python datetime.time
        HdbValue::SECONDTIME(t) => {
            let time_cls = get_time_cls(py)?;
            // SecondTime displays as "HH:MM:SS"
            let s = t.to_string();
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 3 {
                // Fallback to midnight on parse failure (see parse_timestamp_to_python)
                let hour: u32 = parts[0].parse().unwrap_or(0);
                let minute: u32 = parts[1].parse().unwrap_or(0);
                let second: u32 = parts[2].parse().unwrap_or(0);
                time_cls.call1((hour, minute, second))
            } else {
                // Fallback to string for non-standard formats
                Ok(s.into_pyobject(py)?.clone().into_any())
            }
        }
        // Timestamp types: convert to Python datetime.datetime
        HdbValue::LONGDATE(ts) => parse_timestamp_to_python(py, &ts.to_string()),
        HdbValue::SECONDDATE(sd) => parse_timestamp_to_python(py, &sd.to_string()),
        // LOB types: materialize and convert to Python str/bytes
        HdbValue::SYNC_CLOB(clob) => {
            let content = clob
                .clone()
                .into_string()
                .map_err(|e| PyHdbError::data(format!("CLOB read failed: {e}")))?;
            Ok(content.into_pyobject(py)?.clone().into_any())
        }
        HdbValue::SYNC_NCLOB(nclob) => {
            let content = nclob
                .clone()
                .into_string()
                .map_err(|e| PyHdbError::data(format!("NCLOB read failed: {e}")))?;
            Ok(content.into_pyobject(py)?.clone().into_any())
        }
        HdbValue::SYNC_BLOB(blob) => {
            let content = blob
                .clone()
                .into_bytes()
                .map_err(|e| PyHdbError::data(format!("BLOB read failed: {e}")))?;
            Ok(PyBytes::new(py, &content).clone().into_any())
        }
        // Fallback for other types: Debug representation
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

    // Check for datetime types first (before generic checks)
    let py = obj.py();

    // Check if it's a datetime.datetime (must check before date since datetime is a subclass)
    let datetime_cls = get_datetime_cls(py)?;
    if obj.is_instance(&datetime_cls)? {
        // Extract datetime components and format as ISO string for HANA
        let year: i32 = obj.getattr("year")?.extract()?;
        let month: u32 = obj.getattr("month")?.extract()?;
        let day: u32 = obj.getattr("day")?.extract()?;
        let hour: u32 = obj.getattr("hour")?.extract()?;
        let minute: u32 = obj.getattr("minute")?.extract()?;
        let second: u32 = obj.getattr("second")?.extract()?;
        let microsecond: u32 = obj.getattr("microsecond")?.extract()?;

        // Format as ISO timestamp string for HANA to parse
        let ts_str = if microsecond > 0 {
            format!(
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{microsecond:06}"
            )
        } else {
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
        };
        return Ok(HdbValue::STRING(ts_str));
    }

    // Check if it's a datetime.date
    let date_cls = get_date_cls(py)?;
    if obj.is_instance(&date_cls)? {
        let year: i32 = obj.getattr("year")?.extract()?;
        let month: u32 = obj.getattr("month")?.extract()?;
        let day: u32 = obj.getattr("day")?.extract()?;
        let date_str = format!("{year:04}-{month:02}-{day:02}");
        return Ok(HdbValue::STRING(date_str));
    }

    // Check if it's a datetime.time
    let time_cls = get_time_cls(py)?;
    if obj.is_instance(&time_cls)? {
        let hour: u32 = obj.getattr("hour")?.extract()?;
        let minute: u32 = obj.getattr("minute")?.extract()?;
        let second: u32 = obj.getattr("second")?.extract()?;
        let time_str = format!("{hour:02}:{minute:02}:{second:02}");
        return Ok(HdbValue::STRING(time_str));
    }

    // Check for Python Decimal
    let decimal_cls = get_decimal_cls(py)?;
    if obj.is_instance(&decimal_cls)? {
        let s: String = obj.str()?.extract()?;
        return Ok(HdbValue::STRING(s));
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

/// Convert an async HANA value to a Python object.
#[cfg(feature = "async")]
pub fn hana_value_to_python_async<'py>(
    py: Python<'py>,
    value: &hdbconnect_async::HdbValue,
) -> PyResult<Bound<'py, PyAny>> {
    use hdbconnect_async::HdbValue;

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
        HdbValue::DECIMAL(d) => {
            let decimal_cls = get_decimal_cls(py)?;
            let s = d.to_string();
            decimal_cls.call1((s,))
        }
        HdbValue::DAYDATE(d) => {
            let date_cls = get_date_cls(py)?;
            let s = d.to_string();
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() == 3 {
                // Fallback to epoch date on parse failure (see parse_timestamp_to_python)
                let year: i32 = parts[0].parse().unwrap_or(1970);
                let month: u32 = parts[1].parse().unwrap_or(1);
                let day: u32 = parts[2].parse().unwrap_or(1);
                date_cls.call1((year, month, day))
            } else {
                Ok(s.into_pyobject(py)?.clone().into_any())
            }
        }
        HdbValue::SECONDTIME(t) => {
            let time_cls = get_time_cls(py)?;
            let s = t.to_string();
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 3 {
                // Fallback to midnight on parse failure (see parse_timestamp_to_python)
                let hour: u32 = parts[0].parse().unwrap_or(0);
                let minute: u32 = parts[1].parse().unwrap_or(0);
                let second: u32 = parts[2].parse().unwrap_or(0);
                time_cls.call1((hour, minute, second))
            } else {
                Ok(s.into_pyobject(py)?.clone().into_any())
            }
        }
        HdbValue::LONGDATE(ts) => parse_timestamp_to_python(py, &ts.to_string()),
        HdbValue::SECONDDATE(sd) => parse_timestamp_to_python(py, &sd.to_string()),
        other => {
            let s = format!("{other:?}");
            Ok(s.into_pyobject(py)?.clone().into_any())
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests require Python runtime
}
