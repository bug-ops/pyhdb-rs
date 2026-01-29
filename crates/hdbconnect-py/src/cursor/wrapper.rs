//! `PyO3` Cursor wrapper for Python.
//!
//! Provides DB-API 2.0 compliant cursor.

use parking_lot::Mutex;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyFloat, PyInt, PyList, PySequence, PyString, PyTuple};

use crate::config::{DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig};
use crate::connection::{ConnectionInner, SharedConnection};
use crate::cursor::state::ColumnDescription;
use crate::error::PyHdbError;
use crate::reader::PyRecordBatchReader;
use crate::types::{
    get_date_cls, get_datetime_cls, get_decimal_cls, get_time_cls, hana_value_to_python,
};

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
#[pyclass(name = "Cursor", module = "pyhdb_rs._core")]
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

    /// Execute a SQL query with optional parameters.
    ///
    /// Parameters are passed as a tuple or list and bound to ? placeholders in the SQL.
    #[pyo3(signature = (sql, parameters=None))]
    fn execute(&mut self, sql: &str, parameters: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        let mut conn_guard = self.connection.lock();
        match &mut *conn_guard {
            ConnectionInner::Connected(conn) => {
                let rs = match parameters {
                    Some(params) => {
                        // Convert Python params to serde-serializable values
                        let serializable_params = convert_to_serializable(params)?;
                        let mut stmt = conn.prepare(sql).map_err(PyHdbError::from)?;
                        stmt.execute(&serializable_params)
                            .map_err(PyHdbError::from)?
                            .into_result_set()
                            .map_err(PyHdbError::from)?
                    }
                    None => conn.query(sql).map_err(PyHdbError::from)?,
                };

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

    /// Execute a DML statement with optional batch parameters.
    ///
    /// For batch INSERT operations, accepts a sequence of parameter tuples/lists.
    #[pyo3(signature = (sql, seq_of_parameters=None))]
    fn executemany(
        &mut self,
        sql: &str,
        seq_of_parameters: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let mut conn_guard = self.connection.lock();
        match &mut *conn_guard {
            ConnectionInner::Connected(conn) => {
                let affected = match seq_of_parameters {
                    Some(seq) => {
                        // Use prepared statement with batch execution
                        let param_batches = convert_to_serializable_batch(seq)?;
                        let mut stmt = conn.prepare(sql).map_err(PyHdbError::from)?;

                        // Add all parameter sets to batch
                        for params in &param_batches {
                            stmt.add_batch(params).map_err(PyHdbError::from)?;
                        }

                        // Execute the batch
                        let response = stmt.execute_batch().map_err(PyHdbError::from)?;
                        response.count()
                    }
                    None => conn.dml(sql).map_err(PyHdbError::from)?,
                };
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
    ///
    /// Args:
    ///     config: Optional Arrow configuration (`batch_size`, etc.)
    ///
    /// Returns:
    ///     `RecordBatchReader` for streaming results
    ///
    /// Example:
    ///     ```python
    ///     from pyhdb_rs import ArrowConfig
    ///     import polars as pl
    ///
    ///     cursor.execute("SELECT * FROM T")
    ///
    ///     # With default config
    ///     reader = cursor.fetch_arrow()
    ///     df = pl.from_arrow(reader)
    ///
    ///     # With custom batch size
    ///     config = ArrowConfig(batch_size=10000)
    ///     reader = cursor.fetch_arrow(config=config)
    ///     ```
    #[pyo3(signature = (config=None))]
    fn fetch_arrow(
        &self,
        py: Python<'_>,
        config: Option<&PyArrowConfig>,
    ) -> PyResult<PyRecordBatchReader> {
        let batch_size = config.map_or(DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig::batch_size);

        // Extract result_set while holding lock briefly
        let result_set = {
            let mut guard = self.inner.lock();
            match std::mem::replace(&mut *guard, CursorInner::Idle) {
                CursorInner::Active { result_set, .. } => result_set,
                CursorInner::Idle => {
                    return Err(PyHdbError::programming("no active result set").into());
                }
            }
        };

        // Release GIL for CPU-bound schema building and processor creation
        py.detach(|| PyRecordBatchReader::from_resultset(result_set, batch_size))
    }

    /// Execute a query and return Arrow `RecordBatchReader`.
    ///
    /// Args:
    ///     sql: SQL query string
    ///     config: Optional Arrow configuration (`batch_size`, etc.)
    ///
    /// Returns:
    ///     `RecordBatchReader` for streaming results
    ///
    /// Example:
    ///     ```python
    ///     from pyhdb_rs import ArrowConfig
    ///     import polars as pl
    ///
    ///     # With default config
    ///     reader = cursor.execute_arrow("SELECT * FROM T")
    ///     df = pl.from_arrow(reader)
    ///
    ///     # With custom batch size
    ///     config = ArrowConfig(batch_size=10000)
    ///     reader = cursor.execute_arrow("SELECT * FROM T", config=config)
    ///     ```
    #[pyo3(signature = (sql, config=None))]
    fn execute_arrow(
        &self,
        py: Python<'_>,
        sql: &str,
        config: Option<&PyArrowConfig>,
    ) -> PyResult<PyRecordBatchReader> {
        let batch_size = config.map_or(DEFAULT_ARROW_BATCH_SIZE, PyArrowConfig::batch_size);

        let result_set = {
            let mut conn_guard = self.connection.lock();
            match &mut *conn_guard {
                ConnectionInner::Connected(conn) => {
                    let rs = conn.query(sql).map_err(PyHdbError::from)?;
                    drop(conn_guard);
                    rs
                }
                ConnectionInner::Disconnected => {
                    return Err(PyHdbError::operational("connection is closed").into());
                }
            }
        };

        // Release GIL for CPU-bound schema building and processor creation
        py.detach(|| PyRecordBatchReader::from_resultset(result_set, batch_size))
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

/// Serializable parameter value for hdbconnect prepared statements.
///
/// hdbconnect uses serde for parameter binding, so we convert Python values
/// to this enum which implements Serialize.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
enum SerializableValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
}

/// Convert Python parameters (tuple/list) to serializable values.
fn convert_to_serializable(params: &Bound<'_, PyAny>) -> PyResult<Vec<SerializableValue>> {
    let sequence = params.cast::<PySequence>()?;
    let len = sequence.len()?;
    let mut result = Vec::with_capacity(len);

    for i in 0..len {
        let item = sequence.get_item(i)?;
        let value = python_to_serializable(&item)?;
        result.push(value);
    }

    Ok(result)
}

/// Convert sequence of Python parameter tuples/lists to batch serializable values.
fn convert_to_serializable_batch(seq: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<SerializableValue>>> {
    let sequence = seq.cast::<PySequence>()?;
    let len = sequence.len()?;
    let mut result = Vec::with_capacity(len);

    for i in 0..len {
        let params = sequence.get_item(i)?;
        let values = convert_to_serializable(&params)?;
        result.push(values);
    }

    Ok(result)
}

/// Convert a single Python value to a serializable value.
fn python_to_serializable(obj: &Bound<'_, PyAny>) -> PyResult<SerializableValue> {
    if obj.is_none() {
        return Ok(SerializableValue::Null);
    }

    // Check for datetime types first (before generic checks)
    let py = obj.py();

    // Check if it's a datetime.datetime (must check before date since datetime is a subclass)
    let datetime_cls = get_datetime_cls(py)?;
    if obj.is_instance(&datetime_cls)? {
        let year: i32 = obj.getattr("year")?.extract()?;
        let month: u32 = obj.getattr("month")?.extract()?;
        let day: u32 = obj.getattr("day")?.extract()?;
        let hour: u32 = obj.getattr("hour")?.extract()?;
        let minute: u32 = obj.getattr("minute")?.extract()?;
        let second: u32 = obj.getattr("second")?.extract()?;
        let microsecond: u32 = obj.getattr("microsecond")?.extract()?;

        let ts_str = if microsecond > 0 {
            format!(
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{microsecond:06}"
            )
        } else {
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
        };
        return Ok(SerializableValue::String(ts_str));
    }

    // Check if it's a datetime.date
    let date_cls = get_date_cls(py)?;
    if obj.is_instance(&date_cls)? {
        let year: i32 = obj.getattr("year")?.extract()?;
        let month: u32 = obj.getattr("month")?.extract()?;
        let day: u32 = obj.getattr("day")?.extract()?;
        let date_str = format!("{year:04}-{month:02}-{day:02}");
        return Ok(SerializableValue::String(date_str));
    }

    // Check if it's a datetime.time
    let time_cls = get_time_cls(py)?;
    if obj.is_instance(&time_cls)? {
        let hour: u32 = obj.getattr("hour")?.extract()?;
        let minute: u32 = obj.getattr("minute")?.extract()?;
        let second: u32 = obj.getattr("second")?.extract()?;
        let time_str = format!("{hour:02}:{minute:02}:{second:02}");
        return Ok(SerializableValue::String(time_str));
    }

    // Check for Python Decimal
    let decimal_cls = get_decimal_cls(py)?;
    if obj.is_instance(&decimal_cls)? {
        let s: String = obj.str()?.extract()?;
        return Ok(SerializableValue::String(s));
    }

    // Boolean must be checked before int (bool is subclass of int in Python)
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(SerializableValue::Bool(b));
    }

    if obj.is_instance_of::<PyInt>() {
        let v: i64 = obj.extract()?;
        return Ok(SerializableValue::Int(v));
    }

    if obj.is_instance_of::<PyFloat>() {
        let v: f64 = obj.extract()?;
        return Ok(SerializableValue::Float(v));
    }

    if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        return Ok(SerializableValue::String(s));
    }

    if obj.is_instance_of::<PyBytes>() {
        let b: Vec<u8> = obj.extract()?;
        return Ok(SerializableValue::Bytes(b));
    }

    // Unsupported type
    Err(PyHdbError::data(format!(
        "cannot convert Python type {} to SQL parameter",
        obj.get_type().name()?
    ))
    .into())
}
