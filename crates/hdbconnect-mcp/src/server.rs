//! MCP server implementation
//!
//! # Feature Gating Design
//!
//! Tool methods contain duplicate code paths between `#[cfg(feature = "cache")]` and
//! `#[cfg(not(feature = "cache"))]` blocks. This duplication is intentional:
//! - Ensures zero runtime overhead when cache is disabled
//! - Keeps each code path self-contained and easy to verify
//! - Avoids macro complexity that would reduce readability
//! - Compile-time elimination of unused paths

use std::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;

use hdbconnect_async::HdbValue;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler as RmcpServerHandler, tool, tool_handler, tool_router};

#[cfg(feature = "cache")]
use crate::cache::{CacheKey, CacheProvider};
#[cfg(feature = "cache")]
use crate::constants::CACHE_SYSTEM_USER;
use crate::constants::{
    DESCRIBE_PROCEDURE_CURRENT_SCHEMA, DESCRIBE_PROCEDURE_TEMPLATE, DESCRIBE_TABLE_CURRENT_SCHEMA,
    DESCRIBE_TABLE_TEMPLATE, DML_SQL_PLACEHOLDER, DML_STATUS_SUCCESS, ELICIT_DML_CONFIRMATION,
    ELICIT_PROCEDURE_CONFIRMATION, ELICIT_SCHEMA_DESCRIBE_TABLE, ELICIT_SCHEMA_LIST_PROCEDURES,
    ELICIT_SCHEMA_LIST_TABLES, HEALTH_CHECK_QUERY, LIST_PROCEDURES_CURRENT_SCHEMA,
    LIST_PROCEDURES_PATTERN_TEMPLATE, LIST_PROCEDURES_TEMPLATE, LIST_TABLES_CURRENT_SCHEMA,
    LIST_TABLES_TEMPLATE, PROCEDURE_NAME_PLACEHOLDER, PROCEDURE_PARAMS_PLACEHOLDER,
    PROCEDURE_STATUS_SUCCESS, SQL_TRUE, STATUS_OK,
};
#[cfg(feature = "cache")]
use crate::helpers::cached_or_fetch;
use crate::helpers::{get_connection, hdb_value_to_json};
use crate::pool::Pool;
use crate::security::QueryGuard;
use crate::types::{
    CallProcedureParams, ColumnInfo, DescribeProcedureParams, DescribeTableParams, DmlConfirmation,
    DmlResult, ExecuteDmlParams, ExecuteSqlParams, ListProceduresParams, ListTablesParams,
    OutputParameter, ParameterDirection, PingResult, ProcedureConfirmation, ProcedureInfo,
    ProcedureParameter, ProcedureResult, ProcedureResultSet, ProcedureSchema, QueryResult,
    SchemaName, TableInfo, TableSchema, ToolResult,
};
use crate::validation::{
    is_valid_identifier, parse_qualified_name, validate_dml_sql, validate_identifier,
    validate_like_pattern, validate_procedure_name, validate_read_only_sql, validate_where_clause,
};
use crate::{Config, Error};

pub struct ServerHandler {
    pool: Arc<Pool>,
    config: Arc<Config>,
    query_guard: QueryGuard,
    #[cfg(feature = "cache")]
    cache: Arc<dyn CacheProvider>,
    tool_router: ToolRouter<Self>,
}

impl Clone for ServerHandler {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            config: Arc::clone(&self.config),
            query_guard: self.query_guard.clone(),
            #[cfg(feature = "cache")]
            cache: Arc::clone(&self.cache),
            tool_router: Self::tool_router(),
        }
    }
}

impl fmt::Debug for ServerHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("ServerHandler");
        s.field("pool", &"<Pool>")
            .field("config", &self.config)
            .field("query_guard", &self.query_guard);
        #[cfg(feature = "cache")]
        s.field("cache", &"<CacheProvider>");
        s.field("tool_router", &"<ToolRouter>").finish()
    }
}

impl ServerHandler {
    #[cfg(feature = "cache")]
    pub fn new(pool: Pool, config: Config, cache: Arc<dyn CacheProvider>) -> Self {
        let query_guard = QueryGuard::new(
            config.query_timeout,
            config.schema_filter.clone(),
            config.row_limit,
        );

        Self {
            pool: Arc::new(pool),
            config: Arc::new(config),
            query_guard,
            cache,
            tool_router: Self::tool_router(),
        }
    }

    #[cfg(not(feature = "cache"))]
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

    async fn fetch_tables_from_db(
        &self,
        schema: Option<&SchemaName>,
    ) -> crate::Result<Vec<TableInfo>> {
        let conn = get_connection(&self.pool)
            .await
            .map_err(|e| Error::Query(e.message.to_string()))?;

        let query = schema.map_or_else(
            || LIST_TABLES_CURRENT_SCHEMA.to_string(),
            |schema_name| LIST_TABLES_TEMPLATE.replace("{SCHEMA}", &schema_name.name),
        );

        let result_set = self.query_guard.execute(conn.query(&query)).await?;

        let rows = self.query_guard.execute(result_set.into_rows()).await?;

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

        Ok(tables)
    }

    async fn fetch_table_schema_from_db(
        &self,
        table: &str,
        schema: Option<&SchemaName>,
    ) -> crate::Result<TableSchema> {
        let conn = get_connection(&self.pool)
            .await
            .map_err(|e| Error::Query(e.message.to_string()))?;

        let query = schema.map_or_else(
            || DESCRIBE_TABLE_CURRENT_SCHEMA.replace("{TABLE}", table),
            |schema_name| {
                DESCRIBE_TABLE_TEMPLATE
                    .replace("{SCHEMA}", &schema_name.name)
                    .replace("{TABLE}", table)
            },
        );

        let result_set = self.query_guard.execute(conn.query(&query)).await?;

        let rows = self.query_guard.execute(result_set.into_rows()).await?;

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

        Ok(TableSchema {
            table_name: table.to_string(),
            columns,
        })
    }

    async fn fetch_procedures_from_db(
        &self,
        schema: Option<&SchemaName>,
        name_pattern: Option<&str>,
    ) -> crate::Result<Vec<ProcedureInfo>> {
        let conn = get_connection(&self.pool)
            .await
            .map_err(|e| Error::Query(e.message.to_string()))?;

        let query = match (schema, name_pattern) {
            (Some(s), Some(pattern)) => LIST_PROCEDURES_PATTERN_TEMPLATE
                .replace("{SCHEMA}", &s.name)
                .replace("{PATTERN}", pattern),
            (Some(s), None) => LIST_PROCEDURES_TEMPLATE.replace("{SCHEMA}", &s.name),
            (None, _) => LIST_PROCEDURES_CURRENT_SCHEMA.to_string(),
        };

        let result_set = self.query_guard.execute(conn.query(&query)).await?;

        let rows = self.query_guard.execute(result_set.into_rows()).await?;

        let procedures: Vec<ProcedureInfo> = rows
            .into_iter()
            .filter_map(|mut row| {
                if let (
                    Some(HdbValue::STRING(name)),
                    Some(HdbValue::STRING(schema)),
                    Some(HdbValue::STRING(proc_type)),
                    Some(HdbValue::STRING(read_only)),
                ) = (
                    row.next_value(),
                    row.next_value(),
                    row.next_value(),
                    row.next_value(),
                ) {
                    Some(ProcedureInfo {
                        name,
                        schema,
                        procedure_type: proc_type,
                        is_read_only: read_only == SQL_TRUE,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(procedures)
    }

    async fn fetch_procedure_schema_from_db(
        &self,
        proc_name: &str,
        schema: Option<&SchemaName>,
    ) -> crate::Result<ProcedureSchema> {
        let conn = get_connection(&self.pool)
            .await
            .map_err(|e| Error::Query(e.message.to_string()))?;

        let query = schema.map_or_else(
            || DESCRIBE_PROCEDURE_CURRENT_SCHEMA.replace("{PROCEDURE}", proc_name),
            |s| {
                DESCRIBE_PROCEDURE_TEMPLATE
                    .replace("{SCHEMA}", &s.name)
                    .replace("{PROCEDURE}", proc_name)
            },
        );

        let result_set = self.query_guard.execute(conn.query(&query)).await?;

        let rows = self.query_guard.execute(result_set.into_rows()).await?;

        let parameters: Vec<ProcedureParameter> = rows
            .into_iter()
            .filter_map(|mut row| {
                let Some(HdbValue::STRING(name)) = row.next_value() else {
                    return None;
                };
                let position = match row.next_value() {
                    Some(HdbValue::INT(i)) => i as u32,
                    Some(HdbValue::BIGINT(i)) => i as u32,
                    _ => return None,
                };
                let Some(HdbValue::STRING(data_type)) = row.next_value() else {
                    return None;
                };
                let Some(HdbValue::STRING(direction_str)) = row.next_value() else {
                    return None;
                };
                let length = match row.next_value() {
                    Some(HdbValue::INT(i)) if i > 0 => Some(i as u32),
                    Some(HdbValue::BIGINT(i)) if i > 0 => Some(i as u32),
                    _ => None,
                };
                let precision = match row.next_value() {
                    Some(HdbValue::INT(i)) if i > 0 => Some(i as u32),
                    Some(HdbValue::BIGINT(i)) if i > 0 => Some(i as u32),
                    _ => None,
                };
                let scale = match row.next_value() {
                    Some(HdbValue::INT(i)) if i > 0 => Some(i as u32),
                    Some(HdbValue::BIGINT(i)) if i > 0 => Some(i as u32),
                    _ => None,
                };
                let has_default = match row.next_value() {
                    Some(HdbValue::STRING(s)) => s == SQL_TRUE,
                    Some(HdbValue::BOOLEAN(b)) => b,
                    _ => false,
                };

                let direction = ParameterDirection::from_hana_str(&direction_str)?;

                Some(ProcedureParameter {
                    name,
                    position,
                    data_type,
                    direction,
                    length,
                    precision,
                    scale,
                    has_default,
                })
            })
            .collect();

        let schema_name = schema.map_or_else(String::new, |s| s.name.clone());

        Ok(ProcedureSchema {
            name: proc_name.to_string(),
            schema: schema_name,
            parameters,
            result_set_count: None,
        })
    }

    async fn fetch_query_from_db(
        &self,
        sql: &str,
        row_limit: Option<u32>,
    ) -> crate::Result<QueryResult> {
        let conn = get_connection(&self.pool)
            .await
            .map_err(|e| Error::Query(e.message.to_string()))?;

        let result_set = self.query_guard.execute(conn.query(sql)).await?;

        let metadata = result_set.metadata().clone();
        let all_rows = self.query_guard.execute(result_set.into_rows()).await?;

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

        Ok(QueryResult {
            columns,
            rows,
            row_count: count,
        })
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

        #[cfg(feature = "cache")]
        {
            let cache_key = CacheKey::table_list(params.schema.as_ref().map(|s| s.name.as_str()));
            let ttl = self.config.cache().ttl.schema;
            let schema_ref = params.schema.as_ref();

            let tables = cached_or_fetch(self.cache.as_ref(), &cache_key, ttl, || async {
                self.fetch_tables_from_db(schema_ref).await
            })
            .await
            .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "list_tables",
                count = tables.len(),
                "Query completed"
            );

            return Ok(Json(tables));
        }

        #[cfg(not(feature = "cache"))]
        {
            let tables = self
                .fetch_tables_from_db(params.schema.as_ref())
                .await
                .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "list_tables",
                count = tables.len(),
                "Query completed"
            );

            Ok(Json(tables))
        }
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

        #[cfg(feature = "cache")]
        {
            let cache_key = CacheKey::table_schema(
                params.schema.as_ref().map(|s| s.name.as_str()),
                &params.table,
            );
            let ttl = self.config.cache().ttl.schema;
            let table = params.table.clone();
            let schema_ref = params.schema.as_ref();

            let schema_result = cached_or_fetch(self.cache.as_ref(), &cache_key, ttl, || async {
                self.fetch_table_schema_from_db(&table, schema_ref).await
            })
            .await
            .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "describe_table",
                table = %params.table,
                columns = schema_result.columns.len(),
                "Query completed"
            );

            return Ok(Json(schema_result));
        }

        #[cfg(not(feature = "cache"))]
        {
            let schema_result = self
                .fetch_table_schema_from_db(&params.table, params.schema.as_ref())
                .await
                .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "describe_table",
                table = %params.table,
                columns = schema_result.columns.len(),
                "Query completed"
            );

            Ok(Json(schema_result))
        }
    }

    #[tool(description = "Execute a SQL SELECT query")]
    async fn execute_sql(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<ExecuteSqlParams>,
    ) -> ToolResult<QueryResult> {
        if self.config.read_only() {
            validate_read_only_sql(&params.sql).map_err(ErrorData::from)?;
        }

        let row_limit = params
            .limit
            .or_else(|| self.query_guard.row_limit().map(NonZeroU32::get));

        // Cache query results when read_only mode is enabled.
        // TODO: For multi-tenant cache isolation, extract user from context when
        // MCP protocol supports user context propagation from HTTP layer. Current
        // implementation uses CACHE_SYSTEM_USER for all requests (single-tenant safe).
        #[cfg(feature = "cache")]
        if self.config.read_only() && self.config.cache().enabled {
            let cache_key = CacheKey::query_result(&params.sql, row_limit, CACHE_SYSTEM_USER);
            let ttl = self.config.cache().ttl.query;
            let sql = params.sql.clone();

            let result = cached_or_fetch(self.cache.as_ref(), &cache_key, ttl, || async {
                self.fetch_query_from_db(&sql, row_limit).await
            })
            .await
            .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "execute_sql",
                row_count = result.row_count,
                columns = result.columns.len(),
                "Query completed"
            );

            return Ok(Json(result));
        }

        // Non-cached path (DML enabled or cache disabled)
        let result = self
            .fetch_query_from_db(&params.sql, row_limit)
            .await
            .map_err(ErrorData::from)?;

        tracing::debug!(
            tool = "execute_sql",
            row_count = result.row_count,
            columns = result.columns.len(),
            "Query completed"
        );

        Ok(Json(result))
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
                    if let Err(rollback_err) = conn.rollback().await {
                        tracing::warn!(
                            tool = "execute_dml",
                            operation = %operation,
                            error = %rollback_err,
                            "Failed to rollback after DML error (in error recovery path)"
                        );
                    }
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

    // Procedure tools

    #[tool(description = "List stored procedures in a schema")]
    async fn list_procedures(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(mut params): Parameters<ListProceduresParams>,
    ) -> ToolResult<Vec<ProcedureInfo>> {
        let proc_config = self.config.procedure();

        // Check procedures are enabled
        if !proc_config.allow_procedures {
            return Err(ErrorData::from(Error::ProcedureDisabled));
        }

        // Schema elicitation if not provided
        if params.schema.is_none()
            && context.peer.supports_elicitation()
            && let Ok(Some(selection)) = context
                .peer
                .elicit::<SchemaName>(ELICIT_SCHEMA_LIST_PROCEDURES.to_string())
                .await
        {
            params.schema = Some(selection);
        }

        // Validate schema access
        if let Some(ref schema) = params.schema {
            validate_identifier(&schema.name, "schema name").map_err(ErrorData::from)?;
            self.query_guard
                .validate_schema(&schema.name)
                .map_err(ErrorData::from)?;
        }

        // Validate name_pattern to prevent SQL injection
        if let Some(ref pattern) = params.name_pattern {
            validate_like_pattern(pattern).map_err(ErrorData::from)?;
        }

        #[cfg(feature = "cache")]
        {
            let cache_key = CacheKey::procedure_list(
                params.schema.as_ref().map(|s| s.name.as_str()),
                params.name_pattern.as_deref(),
            );
            let ttl = self.config.cache().ttl.schema;
            let schema_ref = params.schema.as_ref();
            let pattern = params.name_pattern.as_deref();

            let procedures = cached_or_fetch(self.cache.as_ref(), &cache_key, ttl, || async {
                self.fetch_procedures_from_db(schema_ref, pattern).await
            })
            .await
            .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "list_procedures",
                count = procedures.len(),
                "Query completed"
            );

            return Ok(Json(procedures));
        }

        #[cfg(not(feature = "cache"))]
        {
            let procedures = self
                .fetch_procedures_from_db(params.schema.as_ref(), params.name_pattern.as_deref())
                .await
                .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "list_procedures",
                count = procedures.len(),
                "Query completed"
            );

            Ok(Json(procedures))
        }
    }

    #[tool(description = "Get parameter definitions for a stored procedure")]
    async fn describe_procedure(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(mut params): Parameters<DescribeProcedureParams>,
    ) -> ToolResult<ProcedureSchema> {
        let proc_config = self.config.procedure();

        // Check procedures are enabled
        if !proc_config.allow_procedures {
            return Err(ErrorData::from(Error::ProcedureDisabled));
        }

        // Validate procedure name
        validate_procedure_name(&params.procedure).map_err(ErrorData::from)?;

        // Parse qualified name
        let (schema_from_name, proc_name) =
            parse_qualified_name(&params.procedure, params.schema.as_ref());

        // Use schema from name if not explicitly provided
        if params.schema.is_none() && schema_from_name.is_some() {
            params.schema = schema_from_name.clone().map(|s| SchemaName { name: s });
        }

        // Schema elicitation if still not provided
        if params.schema.is_none()
            && context.peer.supports_elicitation()
            && let Ok(Some(selection)) = context
                .peer
                .elicit::<SchemaName>(ELICIT_SCHEMA_LIST_PROCEDURES.to_string())
                .await
        {
            params.schema = Some(selection);
        }

        // Validate schema access
        if let Some(ref schema) = params.schema {
            validate_identifier(&schema.name, "schema name").map_err(ErrorData::from)?;
            self.query_guard
                .validate_schema(&schema.name)
                .map_err(ErrorData::from)?;
        }

        #[cfg(feature = "cache")]
        {
            let cache_key = CacheKey::procedure_schema(
                params.schema.as_ref().map(|s| s.name.as_str()),
                &proc_name,
            );
            let ttl = self.config.cache().ttl.schema;
            let proc_name_clone = proc_name.clone();
            let schema_ref = params.schema.as_ref();

            let schema_result = cached_or_fetch(self.cache.as_ref(), &cache_key, ttl, || async {
                self.fetch_procedure_schema_from_db(&proc_name_clone, schema_ref)
                    .await
            })
            .await
            .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "describe_procedure",
                procedure = %proc_name,
                parameters = schema_result.parameters.len(),
                "Query completed"
            );

            return Ok(Json(schema_result));
        }

        #[cfg(not(feature = "cache"))]
        {
            let schema_result = self
                .fetch_procedure_schema_from_db(&proc_name, params.schema.as_ref())
                .await
                .map_err(ErrorData::from)?;

            tracing::debug!(
                tool = "describe_procedure",
                procedure = %proc_name,
                parameters = schema_result.parameters.len(),
                "Query completed"
            );

            Ok(Json(schema_result))
        }
    }

    #[tool(description = "Execute a stored procedure with parameter binding")]
    async fn call_procedure(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(params): Parameters<CallProcedureParams>,
    ) -> ToolResult<ProcedureResult> {
        let proc_config = self.config.procedure();

        // 1. Check procedures are enabled
        if !proc_config.allow_procedures {
            return Err(ErrorData::from(Error::ProcedureDisabled));
        }

        // 2. Validate procedure name
        validate_procedure_name(&params.procedure).map_err(ErrorData::from)?;

        // 3. Parse qualified name
        let (schema_name, proc_name) =
            parse_qualified_name(&params.procedure, params.schema.as_ref());

        // 4. Validate schema access
        if let Some(ref schema) = schema_name {
            self.query_guard
                .validate_schema(schema)
                .map_err(ErrorData::from)?;
        }

        // 5. Request confirmation (unless force)
        if proc_config.require_confirmation && !params.force && context.peer.supports_elicitation()
        {
            let params_json = params.parameters.as_ref().map_or_else(
                || "{}".to_string(),
                |p| serde_json::to_string(p).unwrap_or_else(|_| "{}".to_string()),
            );

            let confirmation_msg = ELICIT_PROCEDURE_CONFIRMATION
                .replace(PROCEDURE_NAME_PLACEHOLDER, &params.procedure)
                .replace(PROCEDURE_PARAMS_PLACEHOLDER, &params_json);

            let confirmation_result = context
                .peer
                .elicit::<ProcedureConfirmation>(confirmation_msg)
                .await;

            match confirmation_result {
                Ok(Some(confirmation)) if confirmation.is_confirmed() => {}
                _ => return Err(ErrorData::from(Error::ProcedureCancelled)),
            }
        }

        // 6. Build CALL statement with literal parameter values
        let qualified_name = schema_name.as_ref().map_or_else(
            || format!("\"{proc_name}\""),
            |s| format!("\"{s}\".\"{proc_name}\""),
        );

        let call_sql = params.parameters.as_ref().map_or_else(
            || format!("CALL {qualified_name}()"),
            |param_map| {
                if param_map.is_empty() {
                    format!("CALL {qualified_name}()")
                } else {
                    let param_values: Vec<String> =
                        param_map.values().map(json_value_to_sql_literal).collect();
                    format!("CALL {qualified_name}({})", param_values.join(", "))
                }
            },
        );

        // 7. Execute with transaction control
        let conn = get_connection(&self.pool).await?;

        if params.explicit_transaction {
            conn.set_auto_commit(false).await;
        }

        // 8. Execute procedure
        let response = self
            .query_guard
            .execute(conn.statement(&call_sql))
            .await
            .map_err(ErrorData::from)?;

        // 9. Process response - collect all return values
        let max_sets = proc_config
            .max_result_sets
            .map_or(u32::MAX, NonZeroU32::get) as usize;
        let max_rows = proc_config
            .max_rows_per_result_set
            .map_or(u32::MAX, NonZeroU32::get) as usize;

        let mut result_sets = Vec::new();
        let mut output_parameters = Vec::new();
        let mut affected_rows: Option<u64> = None;
        let mut result_set_index = 0;

        for return_value in response {
            match return_value {
                hdbconnect_async::HdbReturnValue::ResultSet(rs) => {
                    if result_set_index >= max_sets {
                        if params.explicit_transaction {
                            if let Err(e) = conn.rollback().await {
                                tracing::warn!(
                                    tool = "call_procedure",
                                    procedure = %params.procedure,
                                    error = %e,
                                    "Failed to rollback after result set limit exceeded (in error path)"
                                );
                            }
                            conn.set_auto_commit(true).await;
                        }
                        return Err(ErrorData::from(Error::ProcedureResultSetLimitExceeded {
                            actual: result_set_index + 1,
                            limit: max_sets as u32,
                        }));
                    }

                    let metadata = rs.metadata().clone();
                    let columns: Vec<String> = metadata
                        .iter()
                        .map(|col| col.columnname().to_string())
                        .collect();

                    let all_rows = self
                        .query_guard
                        .execute(rs.into_rows())
                        .await
                        .map_err(ErrorData::from)?;

                    let mut rows = Vec::new();
                    let mut truncated = false;

                    for (idx, row) in all_rows.into_iter().enumerate() {
                        if idx >= max_rows {
                            truncated = true;
                            break;
                        }
                        let row_data: Vec<serde_json::Value> =
                            row.into_iter().map(|v| hdb_value_to_json(&v)).collect();
                        rows.push(row_data);
                    }

                    let row_count = rows.len();
                    result_sets.push(ProcedureResultSet {
                        index: result_set_index,
                        columns,
                        rows,
                        row_count,
                        truncated,
                    });

                    result_set_index += 1;
                }
                hdbconnect_async::HdbReturnValue::OutputParameters(op) => {
                    // Output parameters return null values because hdbconnect's OutputParameters
                    // API does not expose a method to extract values by index. Future enhancement
                    // could use descriptor info + raw protocol access if needed.
                    for (idx, opar) in op.descriptors().iter().enumerate() {
                        let name = opar
                            .name()
                            .map_or_else(|| format!("OUT_{idx}"), ToString::to_string);
                        output_parameters.push(OutputParameter {
                            name,
                            value: serde_json::Value::Null,
                            data_type: opar.type_id().to_string(),
                        });
                    }
                }
                hdbconnect_async::HdbReturnValue::AffectedRows(counts) => {
                    let total: usize = counts.iter().sum();
                    affected_rows = Some(total as u64);
                }
                hdbconnect_async::HdbReturnValue::Success => {}
            }
        }

        // 10. Commit if explicit transaction
        if params.explicit_transaction {
            conn.commit().await.map_err(|e| {
                tracing::error!(
                    tool = "call_procedure",
                    procedure = %params.procedure,
                    error = %e,
                    "Failed to commit procedure execution"
                );
                ErrorData::from(Error::Connection(e))
            })?;
            conn.set_auto_commit(true).await;
        }

        tracing::info!(
            tool = "call_procedure",
            procedure = %params.procedure,
            result_sets = result_sets.len(),
            output_params = output_parameters.len(),
            affected_rows = ?affected_rows,
            "Procedure executed"
        );

        Ok(Json(ProcedureResult {
            procedure: params.procedure,
            status: PROCEDURE_STATUS_SUCCESS.to_string(),
            result_sets,
            output_parameters,
            affected_rows,
            message: None,
        }))
    }
}

/// Sanitize string for SQL literal embedding.
/// Removes control characters and null bytes to prevent injection attacks.
fn sanitize_string_for_sql(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && *c != '\0')
        .collect::<String>()
        .replace('\'', "''")
}

/// Convert JSON value to SQL literal for parameter embedding.
/// Strings are sanitized to remove control characters and properly escape quotes.
fn json_value_to_sql_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!("'{}'", sanitize_string_for_sql(s))
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let json_str = value.to_string();
            format!("'{}'", sanitize_string_for_sql(&json_str))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_sql_null() {
        let value = serde_json::Value::Null;
        assert_eq!(json_value_to_sql_literal(&value), "NULL");
    }

    #[test]
    fn test_json_to_sql_bool_true() {
        let value = serde_json::Value::Bool(true);
        assert_eq!(json_value_to_sql_literal(&value), "TRUE");
    }

    #[test]
    fn test_json_to_sql_bool_false() {
        let value = serde_json::Value::Bool(false);
        assert_eq!(json_value_to_sql_literal(&value), "FALSE");
    }

    #[test]
    fn test_json_to_sql_integer() {
        let value = serde_json::json!(42);
        assert_eq!(json_value_to_sql_literal(&value), "42");
    }

    #[test]
    fn test_json_to_sql_negative_integer() {
        let value = serde_json::json!(-123);
        assert_eq!(json_value_to_sql_literal(&value), "-123");
    }

    #[test]
    fn test_json_to_sql_float() {
        let value = serde_json::json!(3.14);
        assert_eq!(json_value_to_sql_literal(&value), "3.14");
    }

    #[test]
    fn test_json_to_sql_simple_string() {
        let value = serde_json::json!("hello world");
        assert_eq!(json_value_to_sql_literal(&value), "'hello world'");
    }

    #[test]
    fn test_json_to_sql_string_with_single_quotes() {
        let value = serde_json::json!("it's a test");
        assert_eq!(json_value_to_sql_literal(&value), "'it''s a test'");
    }

    #[test]
    fn test_json_to_sql_string_with_multiple_quotes() {
        let value = serde_json::json!("O'Brien's 'quoted' text");
        assert_eq!(
            json_value_to_sql_literal(&value),
            "'O''Brien''s ''quoted'' text'"
        );
    }

    #[test]
    fn test_json_to_sql_empty_string() {
        let value = serde_json::json!("");
        assert_eq!(json_value_to_sql_literal(&value), "''");
    }

    #[test]
    fn test_json_to_sql_string_with_control_chars() {
        let value = serde_json::json!("line1\nline2\ttab\rcarriage");
        assert_eq!(json_value_to_sql_literal(&value), "'line1line2tabcarriage'");
    }

    #[test]
    fn test_json_to_sql_string_with_null_byte() {
        let value = serde_json::Value::String("hello\0world".to_string());
        assert_eq!(json_value_to_sql_literal(&value), "'helloworld'");
    }

    #[test]
    fn test_json_to_sql_string_only_control_chars() {
        let value = serde_json::Value::String("\n\r\t\0".to_string());
        assert_eq!(json_value_to_sql_literal(&value), "''");
    }

    #[test]
    fn test_json_to_sql_unicode_string() {
        let value = serde_json::json!("日本語テスト");
        assert_eq!(json_value_to_sql_literal(&value), "'日本語テスト'");
    }

    #[test]
    fn test_json_to_sql_unicode_with_quotes() {
        let value = serde_json::json!("It's 日本語");
        assert_eq!(json_value_to_sql_literal(&value), "'It''s 日本語'");
    }

    #[test]
    fn test_json_to_sql_array() {
        let value = serde_json::json!([1, 2, 3]);
        assert_eq!(json_value_to_sql_literal(&value), "'[1,2,3]'");
    }

    #[test]
    fn test_json_to_sql_array_with_strings() {
        let value = serde_json::json!(["a", "b'c"]);
        assert_eq!(json_value_to_sql_literal(&value), "'[\"a\",\"b''c\"]'");
    }

    #[test]
    fn test_json_to_sql_object() {
        let value = serde_json::json!({"key": "value"});
        assert_eq!(json_value_to_sql_literal(&value), "'{\"key\":\"value\"}'");
    }

    #[test]
    fn test_json_to_sql_nested_object() {
        let value = serde_json::json!({"outer": {"inner": "val'ue"}});
        assert_eq!(
            json_value_to_sql_literal(&value),
            "'{\"outer\":{\"inner\":\"val''ue\"}}'"
        );
    }

    #[test]
    fn test_sanitize_string_normal() {
        assert_eq!(sanitize_string_for_sql("hello"), "hello");
    }

    #[test]
    fn test_sanitize_string_with_quotes() {
        assert_eq!(sanitize_string_for_sql("it's"), "it''s");
    }

    #[test]
    fn test_sanitize_string_with_newline() {
        assert_eq!(sanitize_string_for_sql("a\nb"), "ab");
    }

    #[test]
    fn test_sanitize_string_with_tab() {
        assert_eq!(sanitize_string_for_sql("a\tb"), "ab");
    }

    #[test]
    fn test_sanitize_string_with_carriage_return() {
        assert_eq!(sanitize_string_for_sql("a\rb"), "ab");
    }

    #[test]
    fn test_sanitize_string_with_null_byte() {
        assert_eq!(sanitize_string_for_sql("a\0b"), "ab");
    }

    #[test]
    fn test_sanitize_string_mixed() {
        assert_eq!(sanitize_string_for_sql("a\n'b\0c'd"), "a''bc''d");
    }
}
