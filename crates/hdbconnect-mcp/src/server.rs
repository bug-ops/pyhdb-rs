//! MCP server implementation

use std::fmt;
use std::sync::Arc;

use hdbconnect_async::HdbValue;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler as RmcpServerHandler, tool, tool_handler, tool_router};

use crate::constants::{
    DESCRIBE_TABLE_CURRENT_SCHEMA, DESCRIBE_TABLE_TEMPLATE, DML_SQL_PLACEHOLDER,
    DML_STATUS_SUCCESS, ELICIT_DML_CONFIRMATION, ELICIT_SCHEMA_DESCRIBE_TABLE,
    ELICIT_SCHEMA_LIST_TABLES, HEALTH_CHECK_QUERY, LIST_TABLES_CURRENT_SCHEMA,
    LIST_TABLES_TEMPLATE, SQL_TRUE, STATUS_OK,
};
use crate::helpers::{get_connection, hdb_value_to_json};
use crate::pool::Pool;
use crate::security::QueryGuard;
use crate::types::{
    ColumnInfo, DescribeTableParams, DmlConfirmation, DmlResult, ExecuteDmlParams,
    ExecuteSqlParams, ListTablesParams, PingResult, QueryResult, SchemaName, TableInfo,
    TableSchema, ToolResult,
};
use crate::validation::{
    is_valid_identifier, validate_dml_sql, validate_identifier, validate_read_only_sql,
    validate_where_clause,
};
use crate::{Config, Error};

pub struct ServerHandler {
    pool: Arc<Pool>,
    config: Arc<Config>,
    query_guard: QueryGuard,
    tool_router: ToolRouter<Self>,
}

impl Clone for ServerHandler {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            config: Arc::clone(&self.config),
            query_guard: self.query_guard.clone(),
            tool_router: Self::tool_router(),
        }
    }
}

impl fmt::Debug for ServerHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerHandler")
            .field("pool", &"<Pool>")
            .field("config", &self.config)
            .field("query_guard", &self.query_guard)
            .field("tool_router", &"<ToolRouter>")
            .finish()
    }
}

impl ServerHandler {
    pub fn new(pool: Pool, config: Config) -> Self {
        let query_guard = QueryGuard::new(
            config.query_timeout,
            config.schema_filter.clone(),
            config.row_limit,
        );

        Self {
            pool: Arc::new(pool),
            config: Arc::new(config),
            query_guard,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ServerHandler {
    #[tool(description = "Check database connection health")]
    async fn ping(&self) -> ToolResult<PingResult> {
        let start = std::time::Instant::now();
        let conn = get_connection(&self.pool).await?;

        let query_result = self
            .query_guard
            .execute(conn.query(HEALTH_CHECK_QUERY))
            .await;

        match query_result {
            Ok(_) => Ok(Json(PingResult {
                status: STATUS_OK.to_string(),
                latency_ms: start.elapsed().as_millis() as u64,
            })),
            Err(e) => Err(ErrorData::from(e)),
        }
    }

    #[tool(description = "List all tables in the specified schema")]
    async fn list_tables(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(mut params): Parameters<ListTablesParams>,
    ) -> ToolResult<Vec<TableInfo>> {
        // If schema not provided and client supports elicitation, ask user
        if params.schema.is_none()
            && context.peer.supports_elicitation()
            && let Ok(Some(selection)) = context
                .peer
                .elicit::<SchemaName>(ELICIT_SCHEMA_LIST_TABLES.to_string())
                .await
        {
            params.schema = Some(selection);
        }

        // Validate schema access and identifier
        if let Some(ref schema) = params.schema {
            validate_identifier(&schema.name, "schema name").map_err(ErrorData::from)?;
            self.query_guard
                .validate_schema(&schema.name)
                .map_err(ErrorData::from)?;
        }

        let conn = get_connection(&self.pool).await?;

        let query = params.schema.as_ref().map_or_else(
            || LIST_TABLES_CURRENT_SCHEMA.to_string(),
            |schema_name| LIST_TABLES_TEMPLATE.replace("{SCHEMA}", &schema_name.name),
        );

        let result_set = self
            .query_guard
            .execute(conn.query(&query))
            .await
            .map_err(ErrorData::from)?;

        let rows = self
            .query_guard
            .execute(result_set.into_rows())
            .await
            .map_err(ErrorData::from)?;

        let tables: Vec<TableInfo> = rows
            .into_iter()
            .filter_map(|mut row| {
                if let (Some(HdbValue::STRING(name)), Some(HdbValue::STRING(table_type))) =
                    (row.next_value(), row.next_value())
                {
                    Some(TableInfo { name, table_type })
                } else {
                    None
                }
            })
            .collect();

        tracing::debug!(
            tool = "list_tables",
            count = tables.len(),
            "Query completed"
        );
        Ok(Json(tables))
    }

    #[tool(description = "Get column definitions for a table")]
    async fn describe_table(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(mut params): Parameters<DescribeTableParams>,
    ) -> ToolResult<TableSchema> {
        // Validate table name identifier
        validate_identifier(&params.table, "table name").map_err(ErrorData::from)?;

        // If schema not provided and client supports elicitation, ask user
        if params.schema.is_none()
            && context.peer.supports_elicitation()
            && let Ok(Some(selection)) = context
                .peer
                .elicit::<SchemaName>(ELICIT_SCHEMA_DESCRIBE_TABLE.to_string())
                .await
        {
            params.schema = Some(selection);
        }

        // Validate schema access and identifier
        if let Some(ref schema) = params.schema {
            validate_identifier(&schema.name, "schema name").map_err(ErrorData::from)?;
            self.query_guard
                .validate_schema(&schema.name)
                .map_err(ErrorData::from)?;
        }

        let conn = get_connection(&self.pool).await?;

        let query = params.schema.as_ref().map_or_else(
            || DESCRIBE_TABLE_CURRENT_SCHEMA.replace("{TABLE}", &params.table),
            |schema_name| {
                DESCRIBE_TABLE_TEMPLATE
                    .replace("{SCHEMA}", &schema_name.name)
                    .replace("{TABLE}", &params.table)
            },
        );

        let result_set = self
            .query_guard
            .execute(conn.query(&query))
            .await
            .map_err(ErrorData::from)?;

        let rows = self
            .query_guard
            .execute(result_set.into_rows())
            .await
            .map_err(ErrorData::from)?;

        let columns: Vec<ColumnInfo> = rows
            .into_iter()
            .filter_map(|mut row| {
                if let (
                    Some(HdbValue::STRING(name)),
                    Some(HdbValue::STRING(data_type)),
                    Some(HdbValue::STRING(nullable)),
                ) = (row.next_value(), row.next_value(), row.next_value())
                {
                    Some(ColumnInfo {
                        name,
                        data_type,
                        nullable: nullable == SQL_TRUE,
                    })
                } else {
                    None
                }
            })
            .collect();

        tracing::debug!(
            tool = "describe_table",
            table = %params.table,
            columns = columns.len(),
            "Query completed"
        );

        Ok(Json(TableSchema {
            table_name: params.table.clone(),
            columns,
        }))
    }

    #[tool(description = "Execute a SQL SELECT query")]
    async fn execute_sql(
        &self,
        Parameters(params): Parameters<ExecuteSqlParams>,
    ) -> ToolResult<QueryResult> {
        if self.config.read_only() {
            validate_read_only_sql(&params.sql).map_err(ErrorData::from)?;
        }

        let row_limit = params
            .limit
            .or_else(|| self.query_guard.row_limit().map(std::num::NonZeroU32::get));

        let conn = get_connection(&self.pool).await?;

        let result_set = self
            .query_guard
            .execute(conn.query(&params.sql))
            .await
            .map_err(ErrorData::from)?;

        let metadata = result_set.metadata().clone();
        let all_rows = self
            .query_guard
            .execute(result_set.into_rows())
            .await
            .map_err(ErrorData::from)?;

        let columns: Vec<String> = metadata
            .iter()
            .map(|col| col.columnname().to_string())
            .collect();

        let mut rows = Vec::new();
        let mut count: usize = 0;

        for row in all_rows {
            if let Some(limit_val) = row_limit
                && count >= limit_val as usize
            {
                break;
            }

            let row_data: Vec<serde_json::Value> =
                row.into_iter().map(|v| hdb_value_to_json(&v)).collect();

            rows.push(row_data);
            count += 1;
        }

        tracing::debug!(
            tool = "execute_sql",
            row_count = count,
            columns = columns.len(),
            "Query completed"
        );

        Ok(Json(QueryResult {
            columns,
            rows,
            row_count: count,
        }))
    }

    #[tool(description = "Execute a DML statement (INSERT, UPDATE, DELETE) with safety checks")]
    async fn execute_dml(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(params): Parameters<ExecuteDmlParams>,
    ) -> ToolResult<DmlResult> {
        let dml_config = self.config.dml();

        // 1. Check DML is enabled
        if !dml_config.allow_dml {
            return Err(ErrorData::from(Error::DmlDisabled));
        }

        // 2. Parse and validate statement
        let operation = validate_dml_sql(&params.sql).map_err(ErrorData::from)?;

        // 3. Check operation whitelist
        if !dml_config.allowed_operations.is_allowed(operation) {
            return Err(ErrorData::from(Error::DmlOperationNotAllowed(operation)));
        }

        // 4. WHERE clause validation for UPDATE/DELETE
        if dml_config.require_where_clause && operation.requires_where_clause() {
            validate_where_clause(&params.sql, operation).map_err(ErrorData::from)?;
        }

        // 5. Schema validation (reuse existing)
        if let Some(ref schema) = params.schema {
            validate_identifier(&schema.name, "schema name").map_err(ErrorData::from)?;
            debug_assert!(is_valid_identifier(&schema.name));
            self.query_guard
                .validate_schema(&schema.name)
                .map_err(ErrorData::from)?;
        }

        // 6. Request confirmation (unless force)
        if dml_config.require_confirmation && !params.force && context.peer.supports_elicitation() {
            let confirmation_msg =
                ELICIT_DML_CONFIRMATION.replace(DML_SQL_PLACEHOLDER, &params.sql);

            let confirmation_result = context
                .peer
                .elicit::<DmlConfirmation>(confirmation_msg)
                .await;

            match confirmation_result {
                Ok(Some(confirmation)) if confirmation.is_confirmed() => {
                    // User confirmed, proceed
                }
                _ => {
                    return Err(ErrorData::from(Error::DmlCancelled));
                }
            }
        }

        // 7. Execute DML statement with transaction control
        let conn = get_connection(&self.pool).await?;

        // Build the SQL with optional schema prefix
        let sql_to_execute = if let Some(ref schema) = params.schema {
            format!("SET SCHEMA \"{}\"; {}", schema.name, params.sql)
        } else {
            params.sql.clone()
        };

        // 8. If row limit is set, use explicit transaction control
        if let Some(limit) = dml_config.max_affected_rows {
            // Disable auto-commit to control transaction manually
            conn.set_auto_commit(false).await;

            let dml_result = self.query_guard.execute(conn.dml(&sql_to_execute)).await;

            let affected_rows = match dml_result {
                Ok(rows) => rows,
                Err(e) => {
                    // Rollback on error and re-enable auto-commit
                    let _ = conn.rollback().await;
                    conn.set_auto_commit(true).await;
                    return Err(ErrorData::from(e));
                }
            };

            let affected_rows_u64 = affected_rows as u64;

            // Check row limit BEFORE commit
            if affected_rows_u64 > u64::from(limit.get()) {
                // ROLLBACK: limit exceeded
                let rollback_result = conn.rollback().await;
                conn.set_auto_commit(true).await;

                if let Err(e) = rollback_result {
                    tracing::error!(
                        tool = "execute_dml",
                        operation = %operation,
                        affected_rows = affected_rows,
                        limit = limit.get(),
                        error = %e,
                        "Failed to rollback after row limit exceeded"
                    );
                }

                tracing::warn!(
                    tool = "execute_dml",
                    operation = %operation,
                    affected_rows = affected_rows,
                    limit = limit.get(),
                    "Row limit exceeded, operation rolled back"
                );

                return Err(ErrorData::from(Error::DmlRowLimitExceeded {
                    actual: affected_rows_u64,
                    limit: limit.get(),
                }));
            }

            // COMMIT: within limit
            let commit_result = conn.commit().await;
            conn.set_auto_commit(true).await;

            if let Err(e) = commit_result {
                tracing::error!(
                    tool = "execute_dml",
                    operation = %operation,
                    affected_rows = affected_rows,
                    error = %e,
                    "Failed to commit DML operation"
                );
                return Err(ErrorData::from(Error::Connection(e)));
            }

            tracing::info!(
                tool = "execute_dml",
                operation = %operation,
                affected_rows = affected_rows,
                "DML operation committed"
            );

            Ok(Json(DmlResult {
                operation: operation.to_string(),
                affected_rows: affected_rows_u64,
                status: DML_STATUS_SUCCESS.to_string(),
                message: None,
            }))
        } else {
            // No row limit: execute with auto-commit (default behavior)
            let affected_rows = self
                .query_guard
                .execute(conn.dml(&sql_to_execute))
                .await
                .map_err(ErrorData::from)?;

            let affected_rows_u64 = affected_rows as u64;

            tracing::info!(
                tool = "execute_dml",
                operation = %operation,
                affected_rows = affected_rows,
                "DML operation completed"
            );

            Ok(Json(DmlResult {
                operation: operation.to_string(),
                affected_rows: affected_rows_u64,
                status: DML_STATUS_SUCCESS.to_string(),
                message: None,
            }))
        }
    }
}

#[tool_handler]
impl RmcpServerHandler for ServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP server for SAP HANA database. Provides tools to query and explore HANA databases."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
