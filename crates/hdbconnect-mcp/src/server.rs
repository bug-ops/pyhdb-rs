//! MCP server implementation

use std::fmt;
use std::sync::Arc;

use hdbconnect_async::HdbValue;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler as RmcpServerHandler, tool, tool_handler, tool_router};

use crate::Config;
use crate::constants::{
    DESCRIBE_TABLE_CURRENT_SCHEMA, DESCRIBE_TABLE_TEMPLATE, ELICIT_SCHEMA_DESCRIBE_TABLE,
    ELICIT_SCHEMA_LIST_TABLES, HEALTH_CHECK_QUERY, LIST_TABLES_CURRENT_SCHEMA,
    LIST_TABLES_TEMPLATE, SQL_TRUE, STATUS_OK,
};
use crate::helpers::{get_connection, hdb_value_to_json};
use crate::pool::Pool;
use crate::security::QueryGuard;
use crate::types::{
    ColumnInfo, DescribeTableParams, ExecuteSqlParams, ListTablesParams, PingResult, QueryResult,
    SchemaName, TableInfo, TableSchema, ToolResult,
};
use crate::validation::{validate_identifier, validate_read_only_sql};

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
