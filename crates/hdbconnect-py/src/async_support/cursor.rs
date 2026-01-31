//! Async cursor implementation for Python.
//!
//! Note: fetch methods raise `NotSupportedError`. Use `execute_arrow()` on connection.

use std::sync::Arc;

use pyo3::prelude::*;
use tokio::sync::Mutex as TokioMutex;

use super::common::{ConnectionState, execute_query_impl};
use super::connection::{AsyncConnectionInner, SharedAsyncConnection};
use super::pool::PooledObject;
use crate::error::PyHdbError;

enum CursorConnection {
    Direct(SharedAsyncConnection),
    Pooled(Arc<TokioMutex<Option<PooledObject>>>),
}

#[pyclass(name = "AsyncCursor", module = "hdbconnect.aio")]
pub struct AsyncPyCursor {
    connection: CursorConnection,
    #[pyo3(get)]
    rowcount: i64,
    #[pyo3(get, set)]
    arraysize: usize,
}

impl std::fmt::Debug for AsyncPyCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncPyCursor")
            .field("rowcount", &self.rowcount)
            .field("arraysize", &self.arraysize)
            .finish_non_exhaustive()
    }
}

impl AsyncPyCursor {
    pub fn new(connection: SharedAsyncConnection) -> Self {
        Self {
            connection: CursorConnection::Direct(connection),
            rowcount: -1,
            arraysize: 1,
        }
    }

    pub fn from_pooled(pooled: Arc<TokioMutex<Option<PooledObject>>>) -> Self {
        Self {
            connection: CursorConnection::Pooled(pooled),
            rowcount: -1,
            arraysize: 1,
        }
    }
}

/// Validate procedure name for safety and correctness.
fn validate_procedure_name(name: &str) -> PyResult<()> {
    if name.is_empty() {
        return Err(PyHdbError::programming("procedure name cannot be empty").into());
    }

    // Basic validation: allow alphanumeric, underscores, dots (for schema.procedure)
    // and reject SQL injection patterns
    let is_valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$' || c == '#');

    if !is_valid {
        return Err(PyHdbError::programming(format!("invalid procedure name: {name}")).into());
    }

    // Check for consecutive dots or starting/ending with dot
    if name.starts_with('.') || name.ends_with('.') || name.contains("..") {
        return Err(PyHdbError::programming(format!("invalid procedure name: {name}")).into());
    }

    Ok(())
}

#[pymethods]
impl AsyncPyCursor {
    // PyO3 requires &self for Python property getter binding.
    #[allow(clippy::unused_self)]
    #[getter]
    fn description<'py>(&self, py: Python<'py>) -> Bound<'py, PyAny> {
        py.None().into_bound(py)
    }

    #[pyo3(signature = (sql, parameters=None))]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        sql: String,
        parameters: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if parameters.is_some() {
            return Err(PyHdbError::not_supported(
                "parameterized queries are not supported in async cursor; \
                 use connection.execute_arrow() or construct SQL directly",
            )
            .into());
        }

        // Branches are structurally similar but handle different connection wrapper types:
        // Direct uses SharedAsyncConnection with AsyncConnectionInner enum,
        // Pooled uses Arc<TokioMutex<Option<PooledObject>>> with Option unwrapping.
        // Unifying would require a trait abstraction that adds complexity without benefit.
        match &self.connection {
            CursorConnection::Direct(conn) => {
                let connection = Arc::clone(conn);
                pyo3_async_runtimes::tokio::future_into_py(py, async move {
                    let mut conn_guard = connection.lock().await;
                    match &mut *conn_guard {
                        AsyncConnectionInner::Connected { connection, .. } => {
                            execute_query_impl(connection, &sql).await
                        }
                        AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
                    }
                })
            }
            CursorConnection::Pooled(pooled) => {
                let pooled = Arc::clone(pooled);
                pyo3_async_runtimes::tokio::future_into_py(py, async move {
                    let mut guard = pooled.lock().await;
                    let obj = guard
                        .as_mut()
                        .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;
                    execute_query_impl(&mut obj.connection, &sql).await
                })
            }
        }
    }

    /// Call a stored database procedure (async).
    ///
    /// Note: Parameters not supported in async cursor.
    /// Use `connection.execute_arrow()` for data retrieval.
    ///
    /// Args:
    ///     procname: Procedure name
    ///     parameters: Not supported, raises `NotSupportedError`
    ///
    /// Returns:
    ///     None (parameters not supported)
    ///
    /// Raises:
    ///     `NotSupportedError`: If parameters provided
    ///     `ProgrammingError`: If procedure name is invalid
    #[pyo3(signature = (procname, parameters=None))]
    fn callproc<'py>(
        &self,
        py: Python<'py>,
        procname: &str,
        parameters: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if parameters.is_some() {
            return Err(PyHdbError::not_supported(
                "parameterized procedures are not supported in async cursor; \
                 use connection.execute_arrow() or construct SQL directly",
            )
            .into());
        }

        validate_procedure_name(procname)?;
        let call_sql = format!("CALL {procname}()");

        match &self.connection {
            CursorConnection::Direct(conn) => {
                let connection = Arc::clone(conn);
                pyo3_async_runtimes::tokio::future_into_py(py, async move {
                    let mut conn_guard = connection.lock().await;
                    match &mut *conn_guard {
                        AsyncConnectionInner::Connected { connection, .. } => {
                            execute_query_impl(connection, &call_sql).await?;
                            Ok(())
                        }
                        AsyncConnectionInner::Disconnected => Err(ConnectionState::Closed.into()),
                    }
                })
            }
            CursorConnection::Pooled(pooled) => {
                let pooled = Arc::clone(pooled);
                pyo3_async_runtimes::tokio::future_into_py(py, async move {
                    let mut guard = pooled.lock().await;
                    let obj = guard
                        .as_mut()
                        .ok_or_else(|| ConnectionState::ReturnedToPool.into_error())?;
                    execute_query_impl(&mut obj.connection, &call_sql).await?;
                    Ok(())
                })
            }
        }
    }

    /// Skip to next result set.
    ///
    /// Returns:
    ///     False (stub implementation)
    // PyO3 #[pymethods] cannot be const fn; clippy suggestion not applicable.
    #[allow(clippy::unused_self, clippy::missing_const_for_fn)]
    fn nextset(&self) -> bool {
        // MVP: stub implementation - multiple result sets deferred to Phase 4
        false
    }

    // PyO3 requires &self for Python method binding; returns NotSupportedError.
    #[allow(clippy::unused_self)]
    fn fetchone(&self) -> PyResult<()> {
        Err(
            PyHdbError::not_supported("fetchone() not supported; use connection.execute_arrow()")
                .into(),
        )
    }

    // PyO3 requires &self for Python method binding; returns NotSupportedError.
    #[allow(clippy::unused_self)]
    #[pyo3(signature = (_size=None))]
    fn fetchmany(&self, _size: Option<usize>) -> PyResult<()> {
        Err(
            PyHdbError::not_supported("fetchmany() not supported; use connection.execute_arrow()")
                .into(),
        )
    }

    // PyO3 requires &self for Python method binding; returns NotSupportedError.
    #[allow(clippy::unused_self)]
    fn fetchall(&self) -> PyResult<()> {
        Err(
            PyHdbError::not_supported("fetchall() not supported; use connection.execute_arrow()")
                .into(),
        )
    }

    fn close(&mut self) {
        self.rowcount = -1;
    }

    // PyO3 requires &self for Python __aiter__ protocol binding.
    #[allow(clippy::unused_self)]
    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    // PyO3 requires &self for Python __anext__ protocol binding.
    #[allow(clippy::unused_self)]
    fn __anext__(&self) -> Option<()> {
        None
    }

    // PyO3 requires &self for Python __aenter__ protocol binding.
    #[allow(clippy::unused_self)]
    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __aexit__<'py>(
        &mut self,
        py: Python<'py>,
        _exc_type: Option<&Bound<'py, PyAny>>,
        _exc_val: Option<&Bound<'py, PyAny>>,
        _exc_tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.close();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(false) })
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncCursor(rowcount={}, arraysize={})",
            self.rowcount, self.arraysize
        )
    }
}
