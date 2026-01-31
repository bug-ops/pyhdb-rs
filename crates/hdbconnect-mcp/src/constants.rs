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
