//! Prometheus metrics for hdbconnect-mcp

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::Result;
use crate::error::Error;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
static START_TIME: OnceLock<Instant> = OnceLock::new();

// Server metrics
const METRIC_UPTIME: &str = "hdbconnect_mcp_uptime_seconds";
const METRIC_INFO: &str = "hdbconnect_mcp_info";
const METRIC_REQUESTS: &str = "hdbconnect_mcp_requests_total";

// Query metrics
const METRIC_QUERY_DURATION: &str = "hdbconnect_mcp_query_duration_seconds";
const METRIC_QUERY_TOTAL: &str = "hdbconnect_mcp_queries_total";
const METRIC_QUERY_ERRORS: &str = "hdbconnect_mcp_query_errors_total";
const METRIC_QUERY_ROWS: &str = "hdbconnect_mcp_query_rows_total";

// Cache metrics
const METRIC_CACHE_HITS: &str = "hdbconnect_mcp_cache_hits_total";
const METRIC_CACHE_MISSES: &str = "hdbconnect_mcp_cache_misses_total";
const METRIC_CACHE_EVICTIONS: &str = "hdbconnect_mcp_cache_evictions_total";
const METRIC_CACHE_SIZE: &str = "hdbconnect_mcp_cache_size";

// Connection pool metrics
const METRIC_POOL_SIZE: &str = "hdbconnect_mcp_pool_connections";
const METRIC_POOL_WAIT_TIME: &str = "hdbconnect_mcp_pool_wait_seconds";
const METRIC_POOL_ERRORS: &str = "hdbconnect_mcp_pool_errors_total";

/// Initialize Prometheus metrics recorder.
pub fn init_metrics() -> Result<()> {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| Error::Config(format!("Failed to install metrics recorder: {e}")))?;

    PROMETHEUS_HANDLE.set(handle).ok();
    START_TIME.set(Instant::now()).ok();

    register_metrics();
    tracing::info!("Prometheus metrics initialized");
    Ok(())
}

fn register_metrics() {
    // Server metrics
    describe_gauge!(METRIC_UPTIME, "Server uptime in seconds");
    describe_gauge!(METRIC_INFO, "Server information (always 1)");
    describe_counter!(METRIC_REQUESTS, "Total MCP requests processed");

    // Query metrics
    describe_histogram!(METRIC_QUERY_DURATION, "Query execution duration in seconds");
    describe_counter!(METRIC_QUERY_TOTAL, "Total queries executed");
    describe_counter!(METRIC_QUERY_ERRORS, "Total query errors");
    describe_counter!(METRIC_QUERY_ROWS, "Total rows returned by queries");

    // Cache metrics
    describe_counter!(METRIC_CACHE_HITS, "Total cache hits");
    describe_counter!(METRIC_CACHE_MISSES, "Total cache misses");
    describe_counter!(METRIC_CACHE_EVICTIONS, "Total cache evictions");
    describe_gauge!(METRIC_CACHE_SIZE, "Current cache size (entries)");

    // Connection pool metrics
    describe_gauge!(METRIC_POOL_SIZE, "Connection pool size by state");
    describe_histogram!(
        METRIC_POOL_WAIT_TIME,
        "Time waiting for a connection from pool"
    );
    describe_counter!(METRIC_POOL_ERRORS, "Total pool connection errors");

    gauge!(
        METRIC_INFO,
        "version" => env!("CARGO_PKG_VERSION"),
    )
    .set(1.0);
}

/// Render metrics in Prometheus text format.
#[must_use]
pub fn render_metrics() -> String {
    if let Some(start) = START_TIME.get() {
        gauge!(METRIC_UPTIME).set(start.elapsed().as_secs_f64());
    }

    PROMETHEUS_HANDLE
        .get()
        .map(PrometheusHandle::render)
        .unwrap_or_default()
}

/// Record an MCP request.
pub fn record_request(method: &str) {
    counter!(METRIC_REQUESTS, "method" => method.to_owned()).increment(1);
}

/// Record a successful query execution.
pub fn record_query(tool: &str, duration: Duration, row_count: u64, cached: bool) {
    let cached_label = if cached { "hit" } else { "miss" };

    histogram!(
        METRIC_QUERY_DURATION,
        "tool" => tool.to_owned(),
        "cached" => cached_label.to_owned(),
    )
    .record(duration.as_secs_f64());

    counter!(
        METRIC_QUERY_TOTAL,
        "tool" => tool.to_owned(),
        "status" => "success".to_owned(),
        "cached" => cached_label.to_owned(),
    )
    .increment(1);

    counter!(METRIC_QUERY_ROWS, "tool" => tool.to_owned()).increment(row_count);
}

/// Record a query error.
pub fn record_query_error(tool: &str, error_type: &str) {
    counter!(
        METRIC_QUERY_ERRORS,
        "tool" => tool.to_owned(),
        "error_type" => error_type.to_owned(),
    )
    .increment(1);

    counter!(
        METRIC_QUERY_TOTAL,
        "tool" => tool.to_owned(),
        "status" => "error".to_owned(),
        "cached" => "miss".to_owned(),
    )
    .increment(1);
}

/// Record a cache hit.
pub fn record_cache_hit(cache_type: &str) {
    counter!(METRIC_CACHE_HITS, "type" => cache_type.to_owned()).increment(1);
}

/// Record a cache miss.
pub fn record_cache_miss(cache_type: &str) {
    counter!(METRIC_CACHE_MISSES, "type" => cache_type.to_owned()).increment(1);
}

/// Record a cache eviction.
pub fn record_cache_eviction(cache_type: &str) {
    counter!(METRIC_CACHE_EVICTIONS, "type" => cache_type.to_owned()).increment(1);
}

/// Update cache size gauge.
#[allow(clippy::cast_precision_loss)]
pub fn set_cache_size(cache_type: &str, size: u64) {
    gauge!(METRIC_CACHE_SIZE, "type" => cache_type.to_owned()).set(size as f64);
}

/// Update pool size gauges.
#[allow(clippy::cast_precision_loss)]
pub fn set_pool_stats(max: usize, available: usize, waiting: usize) {
    gauge!(METRIC_POOL_SIZE, "state" => "max".to_owned()).set(max as f64);
    gauge!(METRIC_POOL_SIZE, "state" => "available".to_owned()).set(available as f64);
    gauge!(METRIC_POOL_SIZE, "state" => "in_use".to_owned()).set((max - available) as f64);
    gauge!(METRIC_POOL_SIZE, "state" => "waiting".to_owned()).set(waiting as f64);
}

/// Record time spent waiting for a connection.
pub fn record_pool_wait_time(duration: Duration) {
    histogram!(METRIC_POOL_WAIT_TIME).record(duration.as_secs_f64());
}

/// Record a pool connection error.
pub fn record_pool_error(error_type: &str) {
    counter!(METRIC_POOL_ERRORS, "type" => error_type.to_owned()).increment(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_metrics_without_init() {
        let output = render_metrics();
        assert!(output.is_empty());
    }
}
