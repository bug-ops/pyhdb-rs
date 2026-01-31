//! Type definitions for MCP tools

use rmcp::{ErrorData, handler::server::wrapper::Json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result type for MCP tool handlers returning structured JSON data
pub type ToolResult<T> = Result<Json<T>, ErrorData>;

/// Connection health check result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PingResult {
    /// Connection status: "ok" or "error"
    #[schemars(description = "Connection status: ok or error")]
    pub status: String,
    /// Query latency in milliseconds
    #[schemars(description = "Query latency in milliseconds")]
    pub latency_ms: u64,
}

/// HANA table information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableInfo {
    /// Table name
    #[schemars(description = "Table name")]
    pub name: String,
    /// Table type (TABLE, VIEW, etc.)
    #[schemars(description = "Table type: TABLE, VIEW, SYNONYM, etc.")]
    pub table_type: String,
}

/// Table column information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnInfo {
    /// Column name
    #[schemars(description = "Column name")]
    pub name: String,
    /// HANA data type (VARCHAR, INTEGER, DECIMAL, etc.)
    #[schemars(description = "HANA data type: VARCHAR, INTEGER, DECIMAL, TIMESTAMP, etc.")]
    pub data_type: String,
    /// Whether column accepts NULL values
    #[schemars(description = "Whether column accepts NULL values")]
    pub nullable: bool,
}

/// Table schema with columns
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableSchema {
    /// Table name
    #[schemars(description = "Table name")]
    pub table_name: String,
    /// List of columns
    #[schemars(description = "List of column definitions")]
    pub columns: Vec<ColumnInfo>,
}

/// SQL query execution result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryResult {
    /// Column names in result set
    #[schemars(description = "Column names in result set")]
    pub columns: Vec<String>,
    /// Result rows as JSON arrays
    #[schemars(description = "Result rows as JSON arrays")]
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows returned
    #[schemars(description = "Number of rows returned")]
    pub row_count: usize,
}

/// Parameters for listing tables
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTablesParams {
    /// Optional schema name filter. If not provided, uses `CURRENT_SCHEMA`.
    /// Ask the user which schema to use if not specified.
    #[serde(default)]
    #[schemars(description = "Schema name to filter tables. Leave empty to use CURRENT_SCHEMA (default behavior). Example: 'SYSTEM', 'MY_SCHEMA'")]
    pub schema: Option<String>,
}

/// Parameters for describing a table
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DescribeTableParams {
    /// Table name to describe. Ask the user which table they want to inspect.
    #[schemars(description = "Name of the table to describe. Example: 'EMPLOYEES', 'ORDERS'")]
    pub table: String,
    /// Optional schema name. If not provided, uses `CURRENT_SCHEMA`.
    /// Ask the user which schema the table belongs to if not specified.
    #[serde(default)]
    #[schemars(description = "Schema name where the table is located. Leave empty to use CURRENT_SCHEMA. Example: 'SYSTEM', 'MY_SCHEMA'")]
    pub schema: Option<String>,
}

/// Parameters for SQL query execution
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteSqlParams {
    /// SQL query to execute (SELECT only in read-only mode)
    #[schemars(
        description = "SQL query to execute. In read-only mode, only SELECT, WITH, EXPLAIN, and CALL are allowed"
    )]
    pub sql: String,
    /// Optional row limit (overrides server default)
    #[serde(default)]
    #[schemars(description = "Optional row limit. Server may enforce maximum limit")]
    pub limit: Option<u32>,
}
