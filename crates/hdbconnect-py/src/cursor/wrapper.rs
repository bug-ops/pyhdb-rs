//! `PyO3` Cursor wrapper for Python.
//!
//! Provides DB-API 2.0 compliant cursor.

use parking_lot::Mutex;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};

use crate::connection::{ConnectionInner, SharedConnection};
use crate::cursor::state::ColumnDescription;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;
use crate::types::hana_value_to_python;

/// Internal cursor state.
#[derive(Debug)]
pub enum CursorInner {
    /// Idle - no active result set.
    Idle,
    /// Active - has result set.
    Active {
        result_set: hdbconnect::ResultSet,
        description: Vec<ColumnDescription>,
    },
}

/// Python Cursor class.
///
/// DB-API 2.0 compliant cursor object.
#[pyclass(name = "Cursor", module = "hdbconnect")]
#[derive(Debug)]
pub struct PyCursor {
    /// Shared connection reference.
    connection: SharedConnection,
    /// Internal cursor state.
    inner: Mutex<CursorInner>,
    /// Number of rows affected by last DML.
    #[pyo3(get)]
    rowcount: i64,
    /// Array size for fetchmany.
    #[pyo3(get, set)]
    arraysize: usize,
}

impl PyCursor {
    /// Create a new cursor from a shared connection.
    pub const fn new(connection: SharedConnection) -> Self {
        Self {
            connection,
            inner: Mutex::new(CursorInner::Idle),
            rowcount: -1,
            arraysize: 1,
        }
    }
}

#[pymethods]
impl PyCursor {
    /// Column descriptions from the last query.
    #[getter]
    fn description<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyList>>> {
        let guard = self.inner.lock();
        match &*guard {
            CursorInner::Active { description, .. } => {
                let desc_list: Vec<_> = description
                    .iter()
                    .map(|col| {
                        (
                            col.name.clone(),
                            col.type_code,
                            col.display_size,
                            col.internal_size,
                            col.precision,
                            col.scale,
                            col.nullable,
                        )
                    })
                    .collect();
                Ok(Some(PyList::new(py, desc_list)?))
            }
            CursorInner::Idle => Ok(None),
        }
    }

    /// Execute a SQL query.
    #[pyo3(signature = (sql, parameters=None))]
    fn execute(&mut self, sql: &str, parameters: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        if parameters.is_some() {
            return Err(PyHdbError::not_supported("parameters not yet supported").into());
        }

        let mut conn_guard = self.connection.lock();
        match &mut *conn_guard {
            ConnectionInner::Connected(conn) => {
                let rs = conn.query(sql).map_err(PyHdbError::from)?;

                // Build description from metadata
                let description: Vec<ColumnDescription> = rs
                    .metadata()
                    .iter()
                    .map(|f| {
                        let precision = f.precision();
                        let scale = f.scale();
                        ColumnDescription {
                            name: f.columnname().to_string(),
                            type_code: f.type_id() as i16,
                            display_size: None,
                            internal_size: None,
                            precision: if precision > 0 { Some(precision) } else { None },
                            scale: if scale > 0 { Some(scale) } else { None },
                            nullable: f.is_nullable(),
                        }
                    })
                    .collect();

                drop(conn_guard);

                *self.inner.lock() = CursorInner::Active {
                    result_set: rs,
                    description,
                };

                self.rowcount = -1;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Execute a DML statement.
    #[pyo3(signature = (sql, seq_of_parameters=None))]
    fn executemany(
        &mut self,
        sql: &str,
        seq_of_parameters: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        if seq_of_parameters.is_some() {
            return Err(
                PyHdbError::not_supported("executemany parameters not yet supported").into(),
            );
        }

        let mut conn_guard = self.connection.lock();
        match &mut *conn_guard {
            ConnectionInner::Connected(conn) => {
                let affected = conn.dml(sql).map_err(PyHdbError::from)?;
                drop(conn_guard);

                let mut inner_guard = self.inner.lock();
                *inner_guard = CursorInner::Idle;
                drop(inner_guard);

                self.rowcount = affected as i64;
                Ok(())
            }
            ConnectionInner::Disconnected => {
                Err(PyHdbError::operational("connection is closed").into())
            }
        }
    }

    /// Fetch one row from the result set.
    fn fetchone<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let mut guard = self.inner.lock();
        match &mut *guard {
            CursorInner::Active { result_set, .. } => match result_set.next() {
                Some(Ok(row)) => {
                    let values = row_to_python(py, &row)?;
                    Ok(Some(PyTuple::new(py, values)?))
                }
                Some(Err(e)) => Err(PyHdbError::from(e).into()),
                None => Ok(None),
            },
            CursorInner::Idle => Ok(None),
        }
    }

    /// Fetch multiple rows from the result set.
    #[pyo3(signature = (size=None))]
    #[allow(clippy::significant_drop_tightening)]
    fn fetchmany<'py>(&self, py: Python<'py>, size: Option<usize>) -> PyResult<Bound<'py, PyList>> {
        let size = size.unwrap_or(self.arraysize);
        let mut rows = Vec::with_capacity(size);

        let mut guard = self.inner.lock();
        if let CursorInner::Active { result_set, .. } = &mut *guard {
            for _ in 0..size {
                match result_set.next() {
                    Some(Ok(row)) => {
                        let values = row_to_python(py, &row)?;
                        rows.push(PyTuple::new(py, values)?);
                    }
                    Some(Err(e)) => return Err(PyHdbError::from(e).into()),
                    None => break,
                }
            }
        }

        PyList::new(py, rows)
    }

    /// Fetch all remaining rows from the result set.
    #[allow(clippy::significant_drop_tightening)]
    fn fetchall<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let mut rows = Vec::new();

        let mut guard = self.inner.lock();
        if let CursorInner::Active { result_set, .. } = &mut *guard {
            for row_result in result_set.by_ref() {
                match row_result {
                    Ok(row) => {
                        let values = row_to_python(py, &row)?;
                        rows.push(PyTuple::new(py, values)?);
                    }
                    Err(e) => return Err(PyHdbError::from(e).into()),
                }
            }
        }

        PyList::new(py, rows)
    }

    /// Close the cursor.
    fn close(&self) {
        *self.inner.lock() = CursorInner::Idle;
    }

    /// Get results as Arrow `RecordBatchReader`.
    #[pyo3(signature = (batch_size=65536))]
    fn fetch_arrow(&self, batch_size: usize) -> PyResult<PyRecordBatchReader> {
        let mut guard = self.inner.lock();
        match std::mem::replace(&mut *guard, CursorInner::Idle) {
            CursorInner::Active { result_set, .. } => {
                PyRecordBatchReader::from_resultset(result_set, batch_size)
            }
            CursorInner::Idle => Err(PyHdbError::programming("no active result set").into()),
        }
    }

    // Iterator protocol
    const fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyTuple>>> {
        self.fetchone(py)
    }

    // Context manager protocol
    const fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }

    fn __repr__(&self) -> String {
        format!(
            "Cursor(rowcount={}, arraysize={})",
            self.rowcount, self.arraysize
        )
    }
}

/// Convert a HANA row to Python values.
fn row_to_python<'py>(py: Python<'py>, row: &hdbconnect::Row) -> PyResult<Vec<Bound<'py, PyAny>>> {
    let mut values = Vec::with_capacity(row.len());

    for i in 0..row.len() {
        let value = &row[i];
        let py_value = hana_value_to_python(py, value)?;
        values.push(py_value);
    }

    Ok(values)
}
