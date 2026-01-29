//! Connection configuration for SAP HANA.
//!
//! Provides `PyConnectionConfig`, a Python-facing wrapper for tuning connection
//! parameters like `fetch_size`, LOB settings, buffer sizes, and timeouts.
//!
//! # Example
//!
//! ```python
//! from pyhdb_rs import ConnectionConfig, connect
//!
//! config = ConnectionConfig(
//!     fetch_size=50000,           # Larger batches for bulk reads
//!     lob_read_length=10_000_000, # 10MB LOB chunks
//!     read_timeout=60.0,          # 60 second timeout
//!     max_cached_statements=32,   # Statement cache size
//! )
//! conn = connect("hdbsql://...", config=config)
//! ```

use std::time::Duration;

use hdbconnect::ConnectionConfiguration;
use pyo3::prelude::*;

use crate::error::PyHdbError;
use crate::types::prepared_cache::DEFAULT_CACHE_CAPACITY;

/// Default batch size for Arrow conversions (65536 rows).
pub const DEFAULT_ARROW_BATCH_SIZE: usize = 65536;

/// Python-facing Arrow configuration for `execute_arrow()`.
///
/// Controls batch processing behavior for Arrow result streaming.
///
/// # Example
///
/// ```python
/// from pyhdb_rs import ArrowConfig
///
/// config = ArrowConfig(batch_size=10000)
/// reader = conn.execute_arrow("SELECT * FROM T", config=config)
/// ```
#[pyclass(name = "ArrowConfig", module = "pyhdb_rs._core", frozen)]
#[derive(Debug, Clone)]
pub struct PyArrowConfig {
    batch_size: usize,
}

impl PyArrowConfig {
    /// Get the batch size.
    #[must_use]
    pub const fn batch_size(&self) -> usize {
        self.batch_size
    }
}

impl Default for PyArrowConfig {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_ARROW_BATCH_SIZE,
        }
    }
}

#[pymethods]
impl PyArrowConfig {
    /// Default batch size for Arrow conversions.
    #[classattr]
    const DEFAULT_BATCH_SIZE: usize = DEFAULT_ARROW_BATCH_SIZE;

    /// Create Arrow configuration.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - Number of rows per Arrow batch (default: 65536). Higher values reduce
    ///   overhead but increase memory usage per batch. Recommended range: 1,000 - 100,000.
    ///
    /// # Raises
    ///
    /// `ProgrammingError`: If `batch_size` is 0.
    #[new]
    #[pyo3(signature = (batch_size=DEFAULT_ARROW_BATCH_SIZE))]
    fn new(batch_size: usize) -> PyResult<Self> {
        if batch_size == 0 {
            return Err(PyHdbError::programming("batch_size must be > 0").into());
        }
        Ok(Self { batch_size })
    }

    /// Number of rows per Arrow batch.
    #[getter]
    const fn get_batch_size(&self) -> usize {
        self.batch_size
    }

    fn __repr__(&self) -> String {
        format!("ArrowConfig(batch_size={})", self.batch_size)
    }
}

/// Python-facing connection configuration.
///
/// Wraps values that will be applied to `hdbconnect::ConnectionConfiguration`.
/// Uses `Option<T>` to distinguish "not set" (use default) from "explicitly set".
#[pyclass(name = "ConnectionConfig", module = "pyhdb_rs._core", frozen)]
#[derive(Debug, Clone)]
pub struct PyConnectionConfig {
    fetch_size: Option<u32>,
    lob_read_length: Option<u32>,
    lob_write_length: Option<u32>,
    max_buffer_size: Option<usize>,
    min_compression_size: Option<usize>,
    read_timeout_secs: Option<f64>,
    max_cached_statements: Option<usize>,
}

impl PyConnectionConfig {
    /// Convert to `hdbconnect::ConnectionConfiguration`.
    #[must_use]
    pub fn to_hdbconnect_config(&self) -> ConnectionConfiguration {
        let mut config = ConnectionConfiguration::default();

        if let Some(fs) = self.fetch_size {
            config.set_fetch_size(fs);
        }
        if let Some(lrl) = self.lob_read_length {
            config.set_lob_read_length(lrl);
        }
        if let Some(lwl) = self.lob_write_length {
            config.set_lob_write_length(lwl);
        }
        if let Some(mbs) = self.max_buffer_size {
            config.set_max_buffer_size(mbs);
        }
        if let Some(mcs) = self.min_compression_size {
            config.set_min_compression_size(mcs);
        }
        if let Some(rt) = self.read_timeout_secs {
            let duration = if rt > 0.0 {
                Some(Duration::from_secs_f64(rt))
            } else {
                None
            };
            config.set_read_timeout(duration);
        }

        config
    }

    /// Get the configured statement cache size.
    ///
    /// Returns the configured value or the default if not set.
    #[must_use]
    pub fn statement_cache_size(&self) -> usize {
        self.max_cached_statements.unwrap_or(DEFAULT_CACHE_CAPACITY)
    }
}

#[pymethods]
impl PyConnectionConfig {
    /// Default fetch size (rows per network round-trip).
    #[classattr]
    const DEFAULT_FETCH_SIZE: u32 = ConnectionConfiguration::DEFAULT_FETCH_SIZE;

    /// Default LOB read length in bytes.
    #[classattr]
    const DEFAULT_LOB_READ_LENGTH: u32 = ConnectionConfiguration::DEFAULT_LOB_READ_LENGTH;

    /// Default LOB write length in bytes.
    #[classattr]
    const DEFAULT_LOB_WRITE_LENGTH: u32 = ConnectionConfiguration::DEFAULT_LOB_WRITE_LENGTH;

    /// Default maximum buffer size.
    #[classattr]
    const DEFAULT_MAX_BUFFER_SIZE: usize = ConnectionConfiguration::DEFAULT_MAX_BUFFER_SIZE;

    /// Default minimum compression size threshold.
    #[classattr]
    const DEFAULT_MIN_COMPRESSION_SIZE: usize =
        ConnectionConfiguration::DEFAULT_MIN_COMPRESSION_SIZE;

    /// Minimum buffer size (cannot go below this).
    #[classattr]
    const MIN_BUFFER_SIZE: usize = ConnectionConfiguration::MIN_BUFFER_SIZE;

    /// Default prepared statement cache size.
    #[classattr]
    const DEFAULT_CACHE_CAPACITY: usize = DEFAULT_CACHE_CAPACITY;

    /// Create connection configuration.
    ///
    /// # Arguments
    ///
    /// * `fetch_size` - Rows fetched per network round-trip (default: 10,000). Higher values reduce
    ///   round-trips but increase memory. Recommended range: 1,000 - 100,000.
    /// * `lob_read_length` - Bytes (or chars for NCLOB) per LOB read (default: ~16MB). Controls LOB
    ///   fetch chunk size.
    /// * `lob_write_length` - Bytes per LOB write (default: ~16MB). Controls LOB upload chunk size.
    /// * `max_buffer_size` - Max connection buffer size in bytes (default: 128KB). Oversized
    ///   buffers shrink back to this after use.
    /// * `min_compression_size` - Threshold for request compression (default: 400 bytes). Requests
    ///   larger than this may be compressed.
    /// * `read_timeout` - Network read timeout in seconds (default: None = no timeout). Connection
    ///   dropped if response takes longer.
    /// * `max_cached_statements` - Maximum number of prepared statements to cache per connection
    ///   (default: 16). Set to 0 to disable caching.
    ///
    /// # Raises
    ///
    /// `ProgrammingError`: If any parameter is invalid.
    #[new]
    #[pyo3(signature = (
        *,
        fetch_size=None,
        lob_read_length=None,
        lob_write_length=None,
        max_buffer_size=None,
        min_compression_size=None,
        read_timeout=None,
        max_cached_statements=None,
    ))]
    fn new(
        fetch_size: Option<u32>,
        lob_read_length: Option<u32>,
        lob_write_length: Option<u32>,
        max_buffer_size: Option<usize>,
        min_compression_size: Option<usize>,
        read_timeout: Option<f64>,
        max_cached_statements: Option<usize>,
    ) -> PyResult<Self> {
        if let Some(fs) = fetch_size
            && fs == 0
        {
            return Err(PyHdbError::programming("fetch_size must be > 0").into());
        }
        if let Some(lrl) = lob_read_length
            && lrl == 0
        {
            return Err(PyHdbError::programming("lob_read_length must be > 0").into());
        }
        if let Some(lwl) = lob_write_length
            && lwl == 0
        {
            return Err(PyHdbError::programming("lob_write_length must be > 0").into());
        }
        if let Some(mbs) = max_buffer_size
            && mbs < Self::MIN_BUFFER_SIZE
        {
            return Err(PyHdbError::programming(format!(
                "max_buffer_size must be >= {} (MIN_BUFFER_SIZE)",
                Self::MIN_BUFFER_SIZE
            ))
            .into());
        }
        if let Some(rt) = read_timeout
            && rt < 0.0
        {
            return Err(PyHdbError::programming("read_timeout cannot be negative").into());
        }

        Ok(Self {
            fetch_size,
            lob_read_length,
            lob_write_length,
            max_buffer_size,
            min_compression_size,
            read_timeout_secs: read_timeout,
            max_cached_statements,
        })
    }

    /// Rows fetched per network round-trip (None = use default).
    #[getter]
    const fn fetch_size(&self) -> Option<u32> {
        self.fetch_size
    }

    /// Bytes per LOB read (None = use default).
    #[getter]
    const fn lob_read_length(&self) -> Option<u32> {
        self.lob_read_length
    }

    /// Bytes per LOB write (None = use default).
    #[getter]
    const fn lob_write_length(&self) -> Option<u32> {
        self.lob_write_length
    }

    /// Max connection buffer size in bytes (None = use default).
    #[getter]
    const fn max_buffer_size(&self) -> Option<usize> {
        self.max_buffer_size
    }

    /// Threshold for request compression (None = use default).
    #[getter]
    const fn min_compression_size(&self) -> Option<usize> {
        self.min_compression_size
    }

    /// Network read timeout in seconds (None = no timeout).
    #[getter]
    const fn read_timeout(&self) -> Option<f64> {
        self.read_timeout_secs
    }

    /// Maximum prepared statements to cache per connection (None = use default).
    #[getter]
    const fn max_cached_statements(&self) -> Option<usize> {
        self.max_cached_statements
    }

    fn __repr__(&self) -> String {
        format!(
            "ConnectionConfig(fetch_size={}, lob_read_length={}, lob_write_length={}, \
             max_buffer_size={}, min_compression_size={}, read_timeout={}, max_cached_statements={})",
            format_option(self.fetch_size),
            format_option(self.lob_read_length),
            format_option(self.lob_write_length),
            format_option(self.max_buffer_size),
            format_option(self.min_compression_size),
            format_option(self.read_timeout_secs),
            format_option(self.max_cached_statements),
        )
    }
}

fn format_option<T: std::fmt::Display>(opt: Option<T>) -> String {
    opt.map_or_else(|| "None".to_string(), |v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════════
    // PyArrowConfig Tests
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_arrow_config_default() {
        let config = PyArrowConfig::default();
        assert_eq!(config.batch_size(), DEFAULT_ARROW_BATCH_SIZE);
    }

    #[test]
    fn test_arrow_config_new() {
        let config = PyArrowConfig::new(10000).unwrap();
        assert_eq!(config.batch_size(), 10000);
    }

    #[test]
    fn test_arrow_config_zero_batch_size() {
        let result = PyArrowConfig::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_arrow_config_repr() {
        let config = PyArrowConfig::new(10000).unwrap();
        let repr = config.__repr__();
        assert!(repr.contains("ArrowConfig"));
        assert!(repr.contains("batch_size=10000"));
    }

    #[test]
    fn test_arrow_config_clone() {
        let config = PyArrowConfig::new(10000).unwrap();
        let cloned = config.clone();
        assert_eq!(cloned.batch_size(), config.batch_size());
    }

    #[test]
    fn test_arrow_config_debug() {
        let config = PyArrowConfig::new(10000).unwrap();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("PyArrowConfig"));
        assert!(debug_str.contains("10000"));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // PyConnectionConfig Tests
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_config_defaults() {
        let config = PyConnectionConfig::new(None, None, None, None, None, None, None).unwrap();
        assert!(config.fetch_size().is_none());
        assert!(config.lob_read_length().is_none());
        assert!(config.lob_write_length().is_none());
        assert!(config.max_buffer_size().is_none());
        assert!(config.min_compression_size().is_none());
        assert!(config.read_timeout().is_none());
        assert!(config.max_cached_statements().is_none());
    }

    #[test]
    fn test_config_with_values() {
        let config = PyConnectionConfig::new(
            Some(50000),
            Some(1000),
            Some(2000),
            None,
            None,
            Some(30.0),
            Some(32),
        )
        .unwrap();

        assert_eq!(config.fetch_size(), Some(50000));
        assert_eq!(config.lob_read_length(), Some(1000));
        assert_eq!(config.lob_write_length(), Some(2000));
        assert_eq!(config.read_timeout(), Some(30.0));
        assert_eq!(config.max_cached_statements(), Some(32));
    }

    #[test]
    fn test_config_validation_fetch_size_zero() {
        let result = PyConnectionConfig::new(Some(0), None, None, None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_lob_read_length_zero() {
        let result = PyConnectionConfig::new(None, Some(0), None, None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_lob_write_length_zero() {
        let result = PyConnectionConfig::new(None, None, Some(0), None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_max_buffer_size_too_small() {
        let result = PyConnectionConfig::new(None, None, None, Some(100), None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_max_buffer_size_at_minimum() {
        let result = PyConnectionConfig::new(
            None,
            None,
            None,
            Some(PyConnectionConfig::MIN_BUFFER_SIZE),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_negative_timeout() {
        let result = PyConnectionConfig::new(None, None, None, None, None, Some(-1.0), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_zero_timeout() {
        let result = PyConnectionConfig::new(None, None, None, None, None, Some(0.0), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_to_hdbconnect() {
        let config =
            PyConnectionConfig::new(Some(50000), None, None, None, None, Some(30.0), None).unwrap();
        let hdb_config = config.to_hdbconnect_config();

        assert_eq!(hdb_config.fetch_size(), 50000);
        assert_eq!(hdb_config.read_timeout(), Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_config_to_hdbconnect_zero_timeout_disables() {
        let config =
            PyConnectionConfig::new(None, None, None, None, None, Some(0.0), None).unwrap();
        let hdb_config = config.to_hdbconnect_config();

        assert_eq!(hdb_config.read_timeout(), None);
    }

    #[test]
    fn test_config_repr() {
        let config =
            PyConnectionConfig::new(Some(50000), None, None, None, None, None, Some(32)).unwrap();
        let repr = config.__repr__();

        assert!(repr.contains("ConnectionConfig"));
        assert!(repr.contains("fetch_size=50000"));
        assert!(repr.contains("lob_read_length=None"));
        assert!(repr.contains("max_cached_statements=32"));
    }

    #[test]
    fn test_config_clone() {
        let config =
            PyConnectionConfig::new(Some(50000), None, None, None, None, None, None).unwrap();
        let cloned = config.clone();

        assert_eq!(cloned.fetch_size(), config.fetch_size());
    }

    #[test]
    fn test_config_debug() {
        let config =
            PyConnectionConfig::new(Some(50000), None, None, None, None, None, None).unwrap();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("PyConnectionConfig"));
        assert!(debug_str.contains("50000"));
    }

    #[test]
    fn test_classattr_defaults() {
        assert_eq!(
            PyConnectionConfig::DEFAULT_FETCH_SIZE,
            ConnectionConfiguration::DEFAULT_FETCH_SIZE
        );
        assert_eq!(
            PyConnectionConfig::DEFAULT_LOB_READ_LENGTH,
            ConnectionConfiguration::DEFAULT_LOB_READ_LENGTH
        );
        assert_eq!(
            PyConnectionConfig::DEFAULT_LOB_WRITE_LENGTH,
            ConnectionConfiguration::DEFAULT_LOB_WRITE_LENGTH
        );
        assert_eq!(
            PyConnectionConfig::DEFAULT_MAX_BUFFER_SIZE,
            ConnectionConfiguration::DEFAULT_MAX_BUFFER_SIZE
        );
        assert_eq!(
            PyConnectionConfig::DEFAULT_MIN_COMPRESSION_SIZE,
            ConnectionConfiguration::DEFAULT_MIN_COMPRESSION_SIZE
        );
        assert_eq!(
            PyConnectionConfig::MIN_BUFFER_SIZE,
            ConnectionConfiguration::MIN_BUFFER_SIZE
        );
        assert_eq!(
            PyConnectionConfig::DEFAULT_CACHE_CAPACITY,
            DEFAULT_CACHE_CAPACITY
        );
    }

    #[test]
    fn test_statement_cache_size_default() {
        let config = PyConnectionConfig::new(None, None, None, None, None, None, None).unwrap();
        assert_eq!(config.statement_cache_size(), DEFAULT_CACHE_CAPACITY);
    }

    #[test]
    fn test_statement_cache_size_custom() {
        let config = PyConnectionConfig::new(None, None, None, None, None, None, Some(64)).unwrap();
        assert_eq!(config.statement_cache_size(), 64);
    }

    #[test]
    fn test_statement_cache_size_zero() {
        let config = PyConnectionConfig::new(None, None, None, None, None, None, Some(0)).unwrap();
        assert_eq!(config.statement_cache_size(), 0);
    }
}
