use std::net::IpAddr;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::PathBuf;

use clap::Parser;
use hdbconnect_mcp::config::{self, TransportMode};
use hdbconnect_mcp::observability::{init_observability, shutdown_observability};
use hdbconnect_mcp::security::SchemaFilter;
use hdbconnect_mcp::transport::run_transport;
use hdbconnect_mcp::{ServerHandler, create_pool};
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "hdbconnect-mcp")]
#[command(about = "MCP server for SAP HANA database", long_about = None)]
#[command(version)]
struct Args {
    /// HANA connection URL (hdbsql://user:password@host:port)
    #[arg(short, long, env = "HANA_URL")]
    url: Option<String>,

    /// Enable read-only mode (blocks DML/DDL)
    #[arg(short, long, default_value_t = true)]
    read_only: bool,

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
}

#[tokio::main]
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
        .read_only(args.read_only)
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

    // Build configuration
    let config = builder.build()?;

    // Initialize observability
    init_observability(&config.telemetry)?;

    // Create connection pool
    let pool = create_pool(config.connection_url.to_string(), config.pool_size.get());

    // Initialize server handler
    let handler = ServerHandler::new(pool, config.clone());

    // Log startup info
    tracing::info!("Starting MCP server for SAP HANA");
    tracing::info!("Transport: {:?}", config.transport.mode);
    tracing::info!("Read-only mode: {}", config.read_only);
    tracing::info!("Row limit: {:?}", config.row_limit);
    tracing::info!("Query timeout: {:?}", config.query_timeout);

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
