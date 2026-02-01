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

// Procedure-related types

/// Procedure parameter direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ParameterDirection {
    In,
    Out,
    InOut,
}

impl ParameterDirection {
    #[must_use]
    pub fn from_hana_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "IN" => Some(Self::In),
            "OUT" => Some(Self::Out),
            "INOUT" => Some(Self::InOut),
            _ => None,
        }
    }
}

/// Procedure parameter metadata
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureParameter {
    /// Parameter name
    #[schemars(description = "Parameter name")]
    pub name: String,
    /// Parameter position (1-indexed)
    #[schemars(description = "Parameter position (1-indexed)")]
    pub position: u32,
    /// HANA data type (VARCHAR, INTEGER, DECIMAL, etc.)
    #[schemars(description = "HANA data type")]
    pub data_type: String,
    /// Parameter direction (IN, OUT, INOUT)
    #[schemars(description = "Parameter direction: IN, OUT, or INOUT")]
    pub direction: ParameterDirection,
    /// Length for string/binary types
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Length for string/binary types")]
    pub length: Option<u32>,
    /// Precision for numeric types
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Precision for numeric types")]
    pub precision: Option<u32>,
    /// Scale for numeric types
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Scale for numeric types")]
    pub scale: Option<u32>,
    /// Whether parameter has default value
    #[schemars(description = "Whether parameter has a default value")]
    pub has_default: bool,
}

/// Procedure metadata (for listing)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureInfo {
    /// Procedure name
    #[schemars(description = "Procedure name")]
    pub name: String,
    /// Schema containing the procedure
    #[schemars(description = "Schema name")]
    pub schema: String,
    /// Procedure type (PROCEDURE or FUNCTION)
    #[schemars(description = "Procedure type: PROCEDURE or FUNCTION")]
    pub procedure_type: String,
    /// Whether procedure is read-only
    #[schemars(description = "Whether procedure only reads data")]
    pub is_read_only: bool,
}

/// Procedure schema with parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureSchema {
    /// Procedure name
    #[schemars(description = "Procedure name")]
    pub name: String,
    /// Schema containing the procedure
    #[schemars(description = "Schema name")]
    pub schema: String,
    /// List of parameters
    #[schemars(description = "List of procedure parameters")]
    pub parameters: Vec<ProcedureParameter>,
    /// Number of expected result sets (if determinable)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Number of result sets returned")]
    pub result_set_count: Option<u32>,
}

/// Parameters for listing procedures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListProceduresParams {
    /// Optional schema name filter
    #[serde(default)]
    #[schemars(description = "Schema name to filter procedures. Leave empty for CURRENT_SCHEMA")]
    pub schema: Option<SchemaName>,
    /// Filter by procedure name pattern (SQL LIKE syntax)
    #[serde(default)]
    #[schemars(description = "Filter by procedure name pattern (SQL LIKE syntax, e.g., 'GET_%')")]
    pub name_pattern: Option<String>,
}

/// Parameters for describing a procedure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DescribeProcedureParams {
    /// Procedure name
    #[schemars(description = "Procedure name to describe")]
    pub procedure: String,
    /// Optional schema name
    #[serde(default)]
    #[schemars(description = "Schema name. Leave empty for CURRENT_SCHEMA")]
    pub schema: Option<SchemaName>,
}

/// Parameters for calling a stored procedure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallProcedureParams {
    /// Procedure name (can be schema.procedure or just procedure)
    #[schemars(description = "Procedure name. Format: SCHEMA.PROCEDURE or PROCEDURE")]
    pub procedure: String,
    /// Input parameters as key-value pairs
    #[serde(default)]
    #[schemars(description = "Input parameters as JSON object")]
    pub parameters: Option<serde_json::Map<String, serde_json::Value>>,
    /// Schema context (optional, used if procedure name doesn't include schema)
    #[serde(default)]
    #[schemars(description = "Schema name. Leave empty to use CURRENT_SCHEMA")]
    pub schema: Option<SchemaName>,
    /// Use explicit transaction control (disable auto-commit)
    #[serde(default)]
    #[schemars(description = "Use explicit transaction control")]
    pub explicit_transaction: bool,
    /// Skip confirmation prompt
    #[serde(default)]
    #[schemars(description = "Skip confirmation prompt (use with caution)")]
    pub force: bool,
}

/// Single result set from procedure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureResultSet {
    /// Result set index (0-based)
    #[schemars(description = "Result set index (0-based)")]
    pub index: usize,
    /// Column names
    #[schemars(description = "Column names in result set")]
    pub columns: Vec<String>,
    /// Rows as JSON arrays
    #[schemars(description = "Result rows as JSON arrays")]
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows in this result set
    #[schemars(description = "Number of rows returned")]
    pub row_count: usize,
    /// Whether rows were truncated due to limit
    #[serde(default)]
    #[schemars(description = "Whether result was truncated due to row limit")]
    pub truncated: bool,
}

/// Output parameter value from procedure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OutputParameter {
    /// Parameter name
    #[schemars(description = "Parameter name")]
    pub name: String,
    /// Parameter value (JSON)
    #[schemars(description = "Output parameter value")]
    pub value: serde_json::Value,
    /// HANA data type
    #[schemars(description = "HANA data type")]
    pub data_type: String,
}

/// Procedure execution result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureResult {
    /// Procedure name that was executed
    #[schemars(description = "Executed procedure name")]
    pub procedure: String,
    /// Execution status
    #[schemars(description = "Status: success or error")]
    pub status: String,
    /// Result sets returned by procedure
    #[schemars(description = "Result sets from procedure")]
    pub result_sets: Vec<ProcedureResultSet>,
    /// Output parameters (OUT and INOUT)
    #[schemars(description = "Output parameter values")]
    pub output_parameters: Vec<OutputParameter>,
    /// Affected rows count (for procedures with DML)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Number of rows affected by DML operations")]
    pub affected_rows: Option<u64>,
    /// Execution message
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional execution message")]
    pub message: Option<String>,
}

/// Procedure confirmation elicitation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Confirm stored procedure execution")]
pub struct ProcedureConfirmation {
    /// User's confirmation response
    #[schemars(description = "Type 'yes' or 'confirm' to proceed")]
    pub confirm: String,
}

elicit_safe!(ProcedureConfirmation);

impl ProcedureConfirmation {
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

    // Procedure type tests
    #[test]
    fn test_parameter_direction_from_hana_str() {
        assert_eq!(
            ParameterDirection::from_hana_str("IN"),
            Some(ParameterDirection::In)
        );
        assert_eq!(
            ParameterDirection::from_hana_str("OUT"),
            Some(ParameterDirection::Out)
        );
        assert_eq!(
            ParameterDirection::from_hana_str("INOUT"),
            Some(ParameterDirection::InOut)
        );
        assert_eq!(
            ParameterDirection::from_hana_str("in"),
            Some(ParameterDirection::In)
        );
        assert_eq!(ParameterDirection::from_hana_str("INVALID"), None);
    }

    #[test]
    fn test_parameter_direction_serialization() {
        assert_eq!(
            serde_json::to_string(&ParameterDirection::In).unwrap(),
            r#""IN""#
        );
        assert_eq!(
            serde_json::to_string(&ParameterDirection::Out).unwrap(),
            r#""OUT""#
        );
        assert_eq!(
            serde_json::to_string(&ParameterDirection::InOut).unwrap(),
            r#""INOUT""#
        );
    }

    #[test]
    fn test_procedure_confirmation_is_confirmed() {
        assert!(
            ProcedureConfirmation {
                confirm: "yes".to_string()
            }
            .is_confirmed()
        );
        assert!(
            ProcedureConfirmation {
                confirm: "YES".to_string()
            }
            .is_confirmed()
        );
        assert!(
            ProcedureConfirmation {
                confirm: "y".to_string()
            }
            .is_confirmed()
        );
        assert!(
            ProcedureConfirmation {
                confirm: "confirm".to_string()
            }
            .is_confirmed()
        );
        assert!(
            ProcedureConfirmation {
                confirm: "  ok  ".to_string()
            }
            .is_confirmed()
        );
    }

    #[test]
    fn test_procedure_confirmation_not_confirmed() {
        assert!(
            !ProcedureConfirmation {
                confirm: "no".to_string()
            }
            .is_confirmed()
        );
        assert!(
            !ProcedureConfirmation {
                confirm: "cancel".to_string()
            }
            .is_confirmed()
        );
        assert!(
            !ProcedureConfirmation {
                confirm: "".to_string()
            }
            .is_confirmed()
        );
    }

    #[test]
    fn test_call_procedure_params_deserialization() {
        let json = r#"{
            "procedure": "GET_USER",
            "parameters": {"USER_ID": 123},
            "schema": {"name": "APP"}
        }"#;

        let params: CallProcedureParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.procedure, "GET_USER");
        assert!(params.parameters.is_some());
        let params_map = params.parameters.unwrap();
        assert_eq!(params_map.get("USER_ID").unwrap(), &serde_json::json!(123));
        assert_eq!(params.schema.unwrap().name, "APP");
        assert!(!params.explicit_transaction);
        assert!(!params.force);
    }

    #[test]
    fn test_procedure_result_serialization() {
        let result = ProcedureResult {
            procedure: "GET_USER".to_string(),
            status: "success".to_string(),
            result_sets: vec![ProcedureResultSet {
                index: 0,
                columns: vec!["ID".to_string(), "NAME".to_string()],
                rows: vec![vec![serde_json::json!(1), serde_json::json!("Alice")]],
                row_count: 1,
                truncated: false,
            }],
            output_parameters: vec![],
            affected_rows: None,
            message: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("GET_USER"));
        assert!(json.contains("success"));
        assert!(json.contains("Alice"));
        assert!(!json.contains("affected_rows"));
        assert!(!json.contains("message"));
    }

    #[test]
    fn test_procedure_info_serialization() {
        let info = ProcedureInfo {
            name: "MY_PROC".to_string(),
            schema: "APP".to_string(),
            procedure_type: "PROCEDURE".to_string(),
            is_read_only: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("MY_PROC"));
        assert!(json.contains("APP"));
        assert!(json.contains("PROCEDURE"));
        assert!(json.contains("true"));
    }
}
