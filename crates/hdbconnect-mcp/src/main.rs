use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;
use hdbconnect_mcp::config::{self, AllowedOperations, TransportMode};
#[cfg(feature = "cache")]
use hdbconnect_mcp::create_cache;
use hdbconnect_mcp::observability::{init_observability, shutdown_observability};
use hdbconnect_mcp::security::SchemaFilter;
use hdbconnect_mcp::transport::run_transport;
use hdbconnect_mcp::{ServerHandler, create_pool};
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "hdbconnect-mcp")]
#[command(about = "MCP server for SAP HANA database", long_about = None)]
#[command(version)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// HANA connection URL (hdbsql://user:password@host:port)
    #[arg(short, long, env = "HANA_URL")]
    url: Option<String>,

    /// Disable read-only mode (allows DML/DDL)
    #[arg(long)]
    no_read_only: bool,

    /// Maximum rows per query
    #[arg(short = 'l', long, default_value = "10000")]
    row_limit: u32,

    /// Connection pool size
    #[arg(short, long, default_value = "4")]
    pool_size: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Transport mode (stdio or http)
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// HTTP bind host (when transport=http)
    #[arg(long, default_value = "127.0.0.1")]
    http_host: IpAddr,

    /// HTTP bind port (when transport=http)
    #[arg(long, default_value = "8080")]
    http_port: u16,

    /// Schema filter mode (none, whitelist, blacklist)
    #[arg(long, default_value = "none")]
    schema_filter_mode: String,

    /// Schema filter schemas (comma-separated)
    #[arg(long)]
    schema_filter_schemas: Option<String>,

    /// Query timeout in seconds
    #[arg(long, default_value = "30")]
    query_timeout: u64,

    /// Enable JSON logging output
    #[arg(long)]
    json_logs: bool,

    // DML configuration
    /// Enable DML operations (INSERT, UPDATE, DELETE)
    #[arg(long)]
    allow_dml: bool,

    /// Skip DML confirmation prompt
    #[arg(long)]
    no_dml_confirm: bool,

    /// Maximum affected rows for DML operations
    #[arg(long, default_value = "1000")]
    dml_max_rows: u32,

    /// Allow UPDATE/DELETE without WHERE clause
    #[arg(long)]
    no_where_clause: bool,

    /// Allowed DML operations (comma-separated: insert,update,delete)
    #[arg(long)]
    dml_ops: Option<String>,

    // Procedure configuration
    /// Enable stored procedure execution
    #[arg(long)]
    allow_procedures: bool,

    /// Skip procedure confirmation prompt
    #[arg(long)]
    no_procedure_confirm: bool,

    /// Maximum result sets from procedures
    #[arg(long, default_value = "10")]
    procedure_max_result_sets: u32,

    /// Maximum rows per procedure result set
    #[arg(long, default_value = "1000")]
    procedure_max_rows: u32,
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Load configuration with precedence: env > file > CLI > defaults
    let mut builder = if let Some(ref path) = args.config {
        config::load_config_from_path(path)?
    } else {
        config::load_config()?
    };

    // Apply CLI arguments (lowest priority, only if not set by env/file)
    if let Some(ref url_str) = args.url {
        let url =
            Url::parse(url_str).map_err(|e| anyhow::anyhow!("Invalid connection URL: {e}"))?;
        builder = builder.connection_url(url);
    }

    let transport_mode: TransportMode = args.transport.parse().unwrap_or_default();

    builder = builder
        .pool_size(NonZeroUsize::new(args.pool_size).unwrap_or(NonZeroUsize::MIN.saturating_add(3)))
        .read_only(!args.no_read_only)
        .row_limit(NonZeroU32::new(args.row_limit))
        .query_timeout(std::time::Duration::from_secs(args.query_timeout))
        .transport_mode(transport_mode)
        .http_host(args.http_host)
        .http_port(args.http_port)
        .json_logs(args.json_logs);

    // Schema filter from CLI
    if args.schema_filter_mode != "none" {
        let schemas: Vec<String> = args
            .schema_filter_schemas
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();

        let filter = SchemaFilter::from_config(&args.schema_filter_mode, &schemas)?;
        builder = builder.schema_filter(filter);
    }

    // Set log level based on verbose flag
    if args.verbose {
        builder = builder.log_level("debug".to_string());
    }

    // DML configuration from CLI
    if args.allow_dml {
        builder = builder.allow_dml(true);
    }

    if args.no_dml_confirm {
        builder = builder.require_dml_confirmation(false);
    }

    builder = builder.max_affected_rows(NonZeroU32::new(args.dml_max_rows));

    if args.no_where_clause {
        builder = builder.require_where_clause(false);
    }

    if let Some(ref ops_str) = args.dml_ops {
        let ops = AllowedOperations::from_str(ops_str).unwrap_or_default();
        builder = builder.allowed_operations(ops);
    }

    // Procedure configuration from CLI
    if args.allow_procedures {
        builder = builder.allow_procedures(true);
    }

    if args.no_procedure_confirm {
        builder = builder.require_procedure_confirmation(false);
    }

    builder = builder.max_result_sets(NonZeroU32::new(args.procedure_max_result_sets));
    builder = builder.max_rows_per_result_set(NonZeroU32::new(args.procedure_max_rows));

    // Build configuration
    let config = builder.build()?;

    // Initialize observability
    init_observability(&config.telemetry)?;

    // Create connection pool
    let pool = create_pool(config.connection_url.to_string(), config.pool_size.get());

    // Initialize server handler
    #[cfg(feature = "cache")]
    let handler = {
        let cache = create_cache(config.cache());
        tracing::info!(
            "Cache enabled: {}, backend: {:?}",
            config.cache.enabled,
            config.cache.backend
        );
        ServerHandler::new(pool, config.clone(), cache)
    };

    #[cfg(not(feature = "cache"))]
    let handler = ServerHandler::new(pool, config.clone());

    // Log startup info
    tracing::info!("Starting MCP server for SAP HANA");
    tracing::info!("Transport: {:?}", config.transport.mode);
    tracing::info!("Read-only mode: {}", config.read_only);
    tracing::info!("Row limit: {:?}", config.row_limit);
    tracing::info!("Query timeout: {:?}", config.query_timeout);
    tracing::info!("DML enabled: {}", config.dml.allow_dml);
    if config.dml.allow_dml {
        tracing::info!(
            "DML confirmation required: {}",
            config.dml.require_confirmation
        );
        tracing::info!("DML max affected rows: {:?}", config.dml.max_affected_rows);
        tracing::info!(
            "DML WHERE clause required: {}",
            config.dml.require_where_clause
        );
    }
    tracing::info!("Procedures enabled: {}", config.procedure.allow_procedures);
    if config.procedure.allow_procedures {
        tracing::info!(
            "Procedure confirmation required: {}",
            config.procedure.require_confirmation
        );
        tracing::info!(
            "Procedure max result sets: {:?}",
            config.procedure.max_result_sets
        );
        tracing::info!(
            "Procedure max rows per result set: {:?}",
            config.procedure.max_rows_per_result_set
        );
    }

    // Setup shutdown signal
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        tracing::info!("Shutdown signal received");
    };

    // Run transport
    let result = run_transport(handler, &config, shutdown).await;

    // Shutdown observability
    shutdown_observability();

    result.map_err(Into::into)
}
