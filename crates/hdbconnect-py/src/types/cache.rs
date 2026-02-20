//! Cached Python type references for efficient conversions.
//!
//! Avoids repeated `py.import()` and `getattr()` calls by caching
//! commonly used Python types in thread-local storage.
//!
//! # Thread Safety
//!
//! The cache uses `thread_local!` storage with `RefCell` for interior mutability.
//! This is safe because:
//! - Each thread has its own independent cache instance
//! - All access requires a `Python<'py>` token (GIL held)
//! - Standard library imports (`datetime`, `decimal`) do not cause reentrancy
//!
//! We use `try_borrow_mut()` to gracefully handle the unlikely case of reentrant
//! access (e.g., if a custom import hook somehow calls back into our code).

use std::cell::RefCell;

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::error::PyHdbError;

/// Thread-local cache for Python datetime module types.
///
/// Each type is stored as `Option<Py<PyType>>` which is a GIL-independent
/// reference that can be safely stored and later bound to a GIL token.
struct DateTimeCache {
    datetime: RefCell<Option<Py<PyType>>>,
    date: RefCell<Option<Py<PyType>>>,
    time: RefCell<Option<Py<PyType>>>,
}

impl DateTimeCache {
    const fn new() -> Self {
        Self {
            datetime: RefCell::new(None),
            date: RefCell::new(None),
            time: RefCell::new(None),
        }
    }

    fn datetime<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
        let mut cache = self
            .datetime
            .try_borrow_mut()
            .map_err(|_| PyHdbError::internal("type cache reentrant access detected"))?;

        if let Some(cls) = cache.as_ref() {
            return Ok(cls.bind(py).clone());
        }
        let datetime_mod = py.import("datetime")?;
        let cls = datetime_mod.getattr("datetime")?;
        let cls: &Bound<'py, PyType> = cls.cast()?;
        *cache = Some(cls.clone().unbind());
        Ok(cls.clone())
    }

    fn date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
        let mut cache = self
            .date
            .try_borrow_mut()
            .map_err(|_| PyHdbError::internal("type cache reentrant access detected"))?;

        if let Some(cls) = cache.as_ref() {
            return Ok(cls.bind(py).clone());
        }
        let datetime_mod = py.import("datetime")?;
        let cls = datetime_mod.getattr("date")?;
        let cls: &Bound<'py, PyType> = cls.cast()?;
        *cache = Some(cls.clone().unbind());
        Ok(cls.clone())
    }

    fn time<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
        let mut cache = self
            .time
            .try_borrow_mut()
            .map_err(|_| PyHdbError::internal("type cache reentrant access detected"))?;

        if let Some(cls) = cache.as_ref() {
            return Ok(cls.bind(py).clone());
        }
        let datetime_mod = py.import("datetime")?;
        let cls = datetime_mod.getattr("time")?;
        let cls: &Bound<'py, PyType> = cls.cast()?;
        *cache = Some(cls.clone().unbind());
        Ok(cls.clone())
    }
}

/// Thread-local cache for Python decimal.Decimal type.
struct DecimalCache {
    decimal: RefCell<Option<Py<PyType>>>,
}

impl DecimalCache {
    const fn new() -> Self {
        Self {
            decimal: RefCell::new(None),
        }
    }

    fn decimal<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
        let mut cache = self
            .decimal
            .try_borrow_mut()
            .map_err(|_| PyHdbError::internal("type cache reentrant access detected"))?;

        if let Some(cls) = cache.as_ref() {
            return Ok(cls.bind(py).clone());
        }
        let decimal_mod = py.import("decimal")?;
        let cls = decimal_mod.getattr("Decimal")?;
        let cls: &Bound<'py, PyType> = cls.cast()?;
        *cache = Some(cls.clone().unbind());
        Ok(cls.clone())
    }
}

thread_local! {
    static DATETIME_CACHE: DateTimeCache = const { DateTimeCache::new() };
    static DECIMAL_CACHE: DecimalCache = const { DecimalCache::new() };
}

// Explicit lifetime 'py is kept for consistency with PyO3 patterns.
// The Python<'py> token and Bound<'py, T> share the same lifetime, making
// the relationship clear even though clippy suggests eliding it.

/// Get cached datetime.datetime class.
///
/// First call imports the module and caches the reference.
/// Subsequent calls return the cached reference with minimal overhead.
#[allow(clippy::elidable_lifetime_names)]
pub fn get_datetime_cls<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
    DATETIME_CACHE.with(|cache| cache.datetime(py))
}

/// Get cached datetime.date class.
#[allow(clippy::elidable_lifetime_names)]
pub fn get_date_cls<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
    DATETIME_CACHE.with(|cache| cache.date(py))
}

/// Get cached datetime.time class.
#[allow(clippy::elidable_lifetime_names)]
pub fn get_time_cls<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
    DATETIME_CACHE.with(|cache| cache.time(py))
}

/// Get cached decimal.Decimal class.
#[allow(clippy::elidable_lifetime_names)]
pub fn get_decimal_cls<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyType>> {
    DECIMAL_CACHE.with(|cache| cache.decimal(py))
}

#[cfg(test)]
mod tests {
    use pyo3::Python;

    use super::*;

    #[test]
    fn test_datetime_cache() {
        Python::try_attach(|py| {
            let cls1 = get_datetime_cls(py).unwrap();
            let cls2 = get_datetime_cls(py).unwrap();
            assert!(cls1.is(&cls2));
        })
        .unwrap();
    }

    #[test]
    fn test_date_cache() {
        Python::try_attach(|py| {
            let cls1 = get_date_cls(py).unwrap();
            let cls2 = get_date_cls(py).unwrap();
            assert!(cls1.is(&cls2));
        })
        .unwrap();
    }

    #[test]
    fn test_time_cache() {
        Python::try_attach(|py| {
            let cls1 = get_time_cls(py).unwrap();
            let cls2 = get_time_cls(py).unwrap();
            assert!(cls1.is(&cls2));
        })
        .unwrap();
    }

    #[test]
    fn test_decimal_cache() {
        Python::try_attach(|py| {
            let cls1 = get_decimal_cls(py).unwrap();
            let cls2 = get_decimal_cls(py).unwrap();
            assert!(cls1.is(&cls2));
        })
        .unwrap();
    }
}
