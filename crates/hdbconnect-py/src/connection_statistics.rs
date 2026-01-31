use pyo3::prelude::*;

/// Connection performance statistics.
///
/// Provides metrics for monitoring connection performance, query latency,
/// and network compression efficiency.
#[pyclass(name = "ConnectionStatistics", module = "pyhdb_rs._core", frozen)]
#[derive(Debug, Clone)]
pub struct PyConnectionStatistics {
    /// Number of roundtrips to the server.
    #[pyo3(get)]
    pub call_count: u32,

    /// Total accumulated wait time in milliseconds.
    #[pyo3(get)]
    pub accumulated_wait_time: f64,

    /// Number of compressed requests.
    #[pyo3(get)]
    pub compressed_requests_count: u32,

    /// Total size of compressed requests in bytes.
    #[pyo3(get)]
    pub compressed_requests_compressed_size: u64,

    /// Total size of uncompressed requests in bytes.
    #[pyo3(get)]
    pub compressed_requests_uncompressed_size: u64,

    /// Number of compressed replies.
    #[pyo3(get)]
    pub compressed_replies_count: u32,

    /// Total size of compressed replies in bytes.
    #[pyo3(get)]
    pub compressed_replies_compressed_size: u64,

    /// Total size of uncompressed replies in bytes.
    #[pyo3(get)]
    pub compressed_replies_uncompressed_size: u64,
}

impl From<hdbconnect::ConnectionStatistics> for PyConnectionStatistics {
    #[inline]
    fn from(stats: hdbconnect::ConnectionStatistics) -> Self {
        Self {
            call_count: stats.call_count(),
            accumulated_wait_time: stats.accumulated_wait_time().as_secs_f64() * 1000.0,
            compressed_requests_count: stats.compressed_requests_count(),
            compressed_requests_compressed_size: stats.compressed_requests_compressed_size(),
            compressed_requests_uncompressed_size: stats.compressed_requests_uncompressed_size(),
            compressed_replies_count: stats.compressed_replies_count(),
            compressed_replies_compressed_size: stats.compressed_replies_compressed_size(),
            compressed_replies_uncompressed_size: stats.compressed_replies_uncompressed_size(),
        }
    }
}

#[pymethods]
impl PyConnectionStatistics {
    /// Average wait time per roundtrip in milliseconds.
    ///
    /// Returns 0.0 if no calls have been made.
    #[getter]
    fn avg_wait_time(&self) -> f64 {
        if self.call_count > 0 {
            self.accumulated_wait_time / f64::from(self.call_count)
        } else {
            0.0
        }
    }

    /// Request compression ratio (0.0-1.0, lower is better).
    ///
    /// Returns 1.0 (no compression) if no compressed requests.
    #[getter]
    #[allow(clippy::cast_precision_loss)]
    fn request_compression_ratio(&self) -> f64 {
        if self.compressed_requests_uncompressed_size > 0 {
            self.compressed_requests_compressed_size as f64
                / self.compressed_requests_uncompressed_size as f64
        } else {
            1.0
        }
    }

    /// Reply compression ratio (0.0-1.0, lower is better).
    ///
    /// Returns 1.0 (no compression) if no compressed replies.
    #[getter]
    #[allow(clippy::cast_precision_loss)]
    fn reply_compression_ratio(&self) -> f64 {
        if self.compressed_replies_uncompressed_size > 0 {
            self.compressed_replies_compressed_size as f64
                / self.compressed_replies_uncompressed_size as f64
        } else {
            1.0
        }
    }

    /// Human-readable representation for debugging.
    fn __repr__(&self) -> String {
        format!(
            "ConnectionStatistics(call_count={}, avg_wait_time={:.2}ms, request_compression={:.3}, reply_compression={:.3})",
            self.call_count,
            self.avg_wait_time(),
            self.request_compression_ratio(),
            self.reply_compression_ratio()
        )
    }
}
