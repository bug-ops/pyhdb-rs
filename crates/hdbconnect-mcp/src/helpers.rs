//! Helper utilities for MCP server

use hdbconnect_async::HdbValue;
use rmcp::ErrorData;

use crate::Error;
use crate::pool::{Pool, PooledConnection};

/// Get a connection from the pool, returning `ErrorData` on failure
pub async fn get_connection(pool: &Pool) -> Result<PooledConnection, ErrorData> {
    Box::pin(pool.get())
        .await
        .map_err(|_| Error::PoolExhausted.into())
}

/// Convert `HdbValue` to `serde_json::Value`
pub fn hdb_value_to_json(value: &HdbValue) -> serde_json::Value {
    match value {
        HdbValue::NULL => serde_json::Value::Null,
        HdbValue::TINYINT(v) => serde_json::json!(v),
        HdbValue::SMALLINT(v) => serde_json::json!(v),
        HdbValue::INT(v) => serde_json::json!(v),
        HdbValue::BIGINT(v) => serde_json::json!(v),
        HdbValue::DECIMAL(v) => serde_json::json!(v.to_string()),
        HdbValue::REAL(v) => serde_json::json!(v),
        HdbValue::DOUBLE(v) => serde_json::json!(v),
        HdbValue::STRING(v) => serde_json::json!(v),
        HdbValue::BOOLEAN(v) => serde_json::json!(v),
        _ => serde_json::json!(format!("{value:?}")),
    }
}
