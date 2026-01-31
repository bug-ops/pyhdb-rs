use std::num::{NonZeroU32, NonZeroUsize};

use clap::Parser;
use hdbconnect_mcp::{Config, ServerHandler, create_pool};
use rmcp::ServiceExt;
use rmcp::transport::io::stdio;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "hdbconnect-mcp")]
#[command(about = "MCP server for SAP HANA database", long_about = None)]
struct Args {
    /// HANA connection URL (hdbsql://user:password@host:port)
    #[arg(short, long)]
    url: String,

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse URL
    let url = Url::parse(&args.url).map_err(|e| anyhow::anyhow!("Invalid connection URL: {e}"))?;

    // Build config
    let config = Config::builder()
        .connection_url(url)
        .pool_size(NonZeroUsize::new(args.pool_size).unwrap_or(NonZeroUsize::new(4).unwrap()))
        .read_only(args.read_only)
        .row_limit(NonZeroU32::new(args.row_limit))
        .build()?;

    // Create connection pool
    let pool = create_pool(args.url.clone(), args.pool_size);

    // Initialize server handler
    let handler = ServerHandler::new(pool, config);

    // Run stdio transport
    tracing::info!("Starting MCP server for SAP HANA");
    tracing::info!("Read-only mode: {}", args.read_only);
    tracing::info!("Row limit: {}", args.row_limit);

    let transport = stdio();
    let server = handler.serve(transport).await?;
    server.waiting().await?;

    Ok(())
}
