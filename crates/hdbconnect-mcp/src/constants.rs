//! Constants for MCP server

/// SQL query to check database connection health
pub const HEALTH_CHECK_QUERY: &str = "SELECT 1 FROM DUMMY";

/// SQL query to list tables in current schema
pub const LIST_TABLES_CURRENT_SCHEMA: &str =
    "SELECT TABLE_NAME, TABLE_TYPE FROM SYS.TABLES WHERE SCHEMA_NAME = CURRENT_SCHEMA";

/// SQL query template to list tables in specific schema (use .replace("{SCHEMA}", `schema_name`))
pub const LIST_TABLES_TEMPLATE: &str =
    "SELECT TABLE_NAME, TABLE_TYPE FROM SYS.TABLES WHERE SCHEMA_NAME = '{SCHEMA}'";

/// SQL query template to describe table in current schema (use .replace("{TABLE}", `table_name`))
pub const DESCRIBE_TABLE_CURRENT_SCHEMA: &str = "SELECT COLUMN_NAME, DATA_TYPE_NAME, IS_NULLABLE FROM SYS.TABLE_COLUMNS WHERE SCHEMA_NAME = CURRENT_SCHEMA AND TABLE_NAME = '{TABLE}'";

/// SQL query template to describe table in specific schema (use .replace("{SCHEMA}",
/// schema).replace("{TABLE}", table))
pub const DESCRIBE_TABLE_TEMPLATE: &str = "SELECT COLUMN_NAME, DATA_TYPE_NAME, IS_NULLABLE FROM SYS.TABLE_COLUMNS WHERE SCHEMA_NAME = '{SCHEMA}' AND TABLE_NAME = '{TABLE}'";

/// Elicitation message for schema selection in `list_tables`
pub const ELICIT_SCHEMA_LIST_TABLES: &str =
    "Which schema do you want to list tables from? (Leave empty for current schema)";

/// Elicitation message for schema selection in `describe_table`
pub const ELICIT_SCHEMA_DESCRIBE_TABLE: &str =
    "Which schema is the table in? (Leave empty for current schema)";

/// Connection status: success
pub const STATUS_OK: &str = "ok";

/// SQL nullable value: TRUE
pub const SQL_TRUE: &str = "TRUE";

/// DML status: success
pub const DML_STATUS_SUCCESS: &str = "success";

/// Elicitation message template for DML confirmation
pub const ELICIT_DML_CONFIRMATION: &str = "You are about to execute a DML operation.\n\nStatement: <sql>\n\nThis operation may modify data in the database. Type 'yes' to confirm or 'no' to cancel.";

/// Placeholder for SQL in DML confirmation message
pub const DML_SQL_PLACEHOLDER: &str = "<sql>";

// Procedure-related constants

/// SQL query to list procedures in current schema
pub const LIST_PROCEDURES_CURRENT_SCHEMA: &str = r"
SELECT PROCEDURE_NAME, SCHEMA_NAME, PROCEDURE_TYPE, IS_READ_ONLY
FROM SYS.PROCEDURES
WHERE SCHEMA_NAME = CURRENT_SCHEMA
ORDER BY PROCEDURE_NAME
";

/// SQL query template to list procedures in specific schema
pub const LIST_PROCEDURES_TEMPLATE: &str = r"
SELECT PROCEDURE_NAME, SCHEMA_NAME, PROCEDURE_TYPE, IS_READ_ONLY
FROM SYS.PROCEDURES
WHERE SCHEMA_NAME = '{SCHEMA}'
ORDER BY PROCEDURE_NAME
";

/// SQL query template to list procedures with name pattern
pub const LIST_PROCEDURES_PATTERN_TEMPLATE: &str = r"
SELECT PROCEDURE_NAME, SCHEMA_NAME, PROCEDURE_TYPE, IS_READ_ONLY
FROM SYS.PROCEDURES
WHERE SCHEMA_NAME = '{SCHEMA}'
  AND PROCEDURE_NAME LIKE '{PATTERN}'
ORDER BY PROCEDURE_NAME
";

/// SQL query template for procedure parameters (current schema)
pub const DESCRIBE_PROCEDURE_CURRENT_SCHEMA: &str = r"
SELECT PARAMETER_NAME, POSITION, DATA_TYPE_NAME, PARAMETER_TYPE, LENGTH, PRECISION, SCALE, HAS_DEFAULT
FROM SYS.PROCEDURE_PARAMETERS
WHERE SCHEMA_NAME = CURRENT_SCHEMA AND PROCEDURE_NAME = '{PROCEDURE}'
ORDER BY POSITION
";

/// SQL query template for procedure parameters (specific schema)
pub const DESCRIBE_PROCEDURE_TEMPLATE: &str = r"
SELECT PARAMETER_NAME, POSITION, DATA_TYPE_NAME, PARAMETER_TYPE, LENGTH, PRECISION, SCALE, HAS_DEFAULT
FROM SYS.PROCEDURE_PARAMETERS
WHERE SCHEMA_NAME = '{SCHEMA}' AND PROCEDURE_NAME = '{PROCEDURE}'
ORDER BY POSITION
";

/// Elicitation message for schema selection in `list_procedures`
pub const ELICIT_SCHEMA_LIST_PROCEDURES: &str =
    "Which schema do you want to list procedures from? (Leave empty for current schema)";

/// Elicitation message template for procedure execution confirmation
pub const ELICIT_PROCEDURE_CONFIRMATION: &str = r"You are about to execute a stored procedure.

Procedure: <procedure>
Parameters: <parameters>

This operation may modify data or have side effects. Type 'yes' to confirm or 'no' to cancel.";

/// Placeholder for procedure name in confirmation message
pub const PROCEDURE_NAME_PLACEHOLDER: &str = "<procedure>";

/// Placeholder for parameters in confirmation message
pub const PROCEDURE_PARAMS_PLACEHOLDER: &str = "<parameters>";

/// Procedure status: success
pub const PROCEDURE_STATUS_SUCCESS: &str = "success";

// Cache-related constants

/// Default user identifier for single-tenant/anonymous cache keys.
/// Used when authentication is disabled but cache is enabled.
#[cfg(feature = "cache")]
pub const CACHE_SYSTEM_USER: &str = "_system";
