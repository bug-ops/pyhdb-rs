//! Transport layer abstraction
//!
//! Supports stdio (default) and HTTP/SSE transports

#[cfg(feature = "http")]
mod auth;
#[cfg(feature = "http")]
mod http;

use std::future::Future;

use rmcp::ServiceExt;
use rmcp::transport::io::stdio;

use crate::config::{Config, TransportMode};
use crate::server::ServerHandler;
use crate::{Error, Result};

/// Run the MCP server with the configured transport
pub async fn run_transport(
    handler: ServerHandler,
    config: &Config,
    #[allow(unused_variables)] shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    match config.transport.mode {
        TransportMode::Stdio => run_stdio(handler).await,
        #[cfg(feature = "http")]
        TransportMode::Http => {
            http::run_http(
                handler,
                config.transport.http_host,
                config.transport.http_port,
                shutdown,
            )
            .await
        }
        #[cfg(not(feature = "http"))]
        TransportMode::Http => Err(Error::Transport(
            "HTTP transport requires the 'http' feature".into(),
        )),
    }
}

async fn run_stdio(handler: ServerHandler) -> Result<()> {
    let transport = stdio();
    let server = handler
        .serve(transport)
        .await
        .map_err(|e| Error::Transport(format!("Failed to start stdio transport: {e}")))?;

    server
        .waiting()
        .await
        .map_err(|e| Error::Transport(format!("Stdio transport error: {e}")))?;

    Ok(())
}
