//! Type definitions for MCP tools

use rmcp::handler::server::wrapper::Json;
use rmcp::{ErrorData, elicit_safe};
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

/// Schema name for elicitation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Schema name")]
pub struct SchemaName {
    /// Schema name
    #[schemars(description = "Name of the schema")]
    pub name: String,
}

elicit_safe!(SchemaName);

/// Parameters for listing tables
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTablesParams {
    /// Optional schema name filter. If not provided, uses `CURRENT_SCHEMA`.
    #[serde(default)]
    #[schemars(
        description = "Schema name to filter tables. Leave empty to use CURRENT_SCHEMA (default behavior). Example: 'SYSTEM', 'MY_SCHEMA'"
    )]
    pub schema: Option<SchemaName>,
}

/// Parameters for describing a table
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DescribeTableParams {
    /// Table name to describe. Ask the user which table they want to inspect.
    #[schemars(description = "Name of the table to describe. Example: 'EMPLOYEES', 'ORDERS'")]
    pub table: String,
    /// Optional schema name. If not provided, uses `CURRENT_SCHEMA`.
    #[serde(default)]
    #[schemars(
        description = "Schema name where the table is located. Leave empty to use CURRENT_SCHEMA. Example: 'SYSTEM', 'MY_SCHEMA'"
    )]
    pub schema: Option<SchemaName>,
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

/// Parameters for DML execution
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteDmlParams {
    /// DML statement (INSERT, UPDATE, DELETE)
    #[schemars(description = "SQL DML statement. Allowed: INSERT, UPDATE, DELETE")]
    pub sql: String,

    /// Optional: schema context for the operation
    #[serde(default)]
    #[schemars(description = "Schema name. Leave empty to use CURRENT_SCHEMA")]
    pub schema: Option<SchemaName>,

    /// Force execution without confirmation (requires elevated permissions).
    ///
    /// # Security Warning
    ///
    /// Setting `force=true` bypasses the user confirmation prompt, allowing DML
    /// operations to execute without explicit user approval. This should only be
    /// used in automated pipelines or by trusted clients where:
    /// - The operation has been pre-validated
    /// - The caller has appropriate authorization
    /// - Audit logging is in place
    ///
    /// Using `force=true` in interactive contexts increases the risk of
    /// unintended data modifications.
    #[serde(default)]
    #[schemars(description = "Skip confirmation prompt (use with caution)")]
    pub force: bool,
}

/// DML execution result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DmlResult {
    /// Operation performed
    #[schemars(description = "Operation type: INSERT, UPDATE, or DELETE")]
    pub operation: String,

    /// Number of rows affected
    #[schemars(description = "Number of rows inserted, updated, or deleted")]
    pub affected_rows: u64,

    /// Execution status
    #[schemars(description = "Status: success or error")]
    pub status: String,

    /// Optional message (e.g., warning about row limit)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional message or warning")]
    pub message: Option<String>,
}

/// DML confirmation elicitation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Confirm DML operation execution")]
pub struct DmlConfirmation {
    /// User's confirmation response
    #[schemars(description = "Type 'yes' or 'confirm' to proceed")]
    pub confirm: String,
}

elicit_safe!(DmlConfirmation);

impl DmlConfirmation {
    #[must_use]
    pub fn is_confirmed(&self) -> bool {
        let normalized = self.confirm.trim().to_lowercase();
        matches!(normalized.as_str(), "yes" | "y" | "confirm" | "ok" | "true")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dml_confirmation_is_confirmed() {
        assert!(
            DmlConfirmation {
                confirm: "yes".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "YES".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "y".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "Y".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "confirm".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "CONFIRM".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "ok".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "OK".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "true".to_string()
            }
            .is_confirmed()
        );
        assert!(
            DmlConfirmation {
                confirm: "  yes  ".to_string()
            }
            .is_confirmed()
        );
    }

    #[test]
    fn test_dml_confirmation_not_confirmed() {
        assert!(
            !DmlConfirmation {
                confirm: "no".to_string()
            }
            .is_confirmed()
        );
        assert!(
            !DmlConfirmation {
                confirm: "false".to_string()
            }
            .is_confirmed()
        );
        assert!(
            !DmlConfirmation {
                confirm: "cancel".to_string()
            }
            .is_confirmed()
        );
        assert!(
            !DmlConfirmation {
                confirm: "".to_string()
            }
            .is_confirmed()
        );
        assert!(
            !DmlConfirmation {
                confirm: "n".to_string()
            }
            .is_confirmed()
        );
    }

    #[test]
    fn test_dml_result_serialization() {
        let result = DmlResult {
            operation: "INSERT".to_string(),
            affected_rows: 5,
            status: "success".to_string(),
            message: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("INSERT"));
        assert!(json.contains("5"));
        assert!(json.contains("success"));
        assert!(!json.contains("message"));

        let result_with_message = DmlResult {
            operation: "DELETE".to_string(),
            affected_rows: 100,
            status: "success".to_string(),
            message: Some("Deleted old records".to_string()),
        };

        let json = serde_json::to_string(&result_with_message).unwrap();
        assert!(json.contains("message"));
        assert!(json.contains("Deleted old records"));
    }

    #[test]
    fn test_execute_dml_params_deserialization() {
        let json = r#"{
            "sql": "INSERT INTO users VALUES (1, 'test')",
            "schema": {"name": "APP"}
        }"#;

        let params: ExecuteDmlParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.sql, "INSERT INTO users VALUES (1, 'test')");
        assert!(params.schema.is_some());
        assert_eq!(params.schema.unwrap().name, "APP");
        assert!(!params.force);
    }

    #[test]
    fn test_execute_dml_params_with_force() {
        let json = r#"{
            "sql": "DELETE FROM logs WHERE created_at < '2024-01-01'",
            "force": true
        }"#;

        let params: ExecuteDmlParams = serde_json::from_str(json).unwrap();
        assert!(params.force);
        assert!(params.schema.is_none());
    }
}
