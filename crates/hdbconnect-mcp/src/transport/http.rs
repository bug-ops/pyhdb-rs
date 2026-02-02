//! HTTP/SSE transport implementation

use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::{Router, middleware};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use super::auth::{AuthConfig, bearer_auth_middleware};
use crate::server::ServerHandler;
use crate::{Error, Result};

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Admin reload request
#[derive(Debug, Deserialize)]
struct ReloadRequest {
    /// Force reload even if config unchanged
    #[serde(default)]
    force: bool,
}

/// Admin reload response
#[derive(Debug, Serialize)]
struct ReloadResponse {
    success: bool,
    message: String,
    changed: Vec<String>,
}

/// HTTP server configuration
#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub cors_origin: Option<String>,
    pub bearer_token: Option<String>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            cors_origin: Some("http://localhost:3000".to_string()),
            bearer_token: None,
        }
    }
}

impl HttpConfig {
    pub fn from_env() -> Self {
        Self {
            cors_origin: std::env::var("MCP_CORS_ORIGIN").ok(),
            bearer_token: std::env::var("MCP_HTTP_BEARER_TOKEN").ok(),
        }
    }
}

/// Run HTTP server with SSE transport
pub async fn run_http(
    handler: ServerHandler,
    host: IpAddr,
    port: u16,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let addr = SocketAddr::new(host, port);
    let cancellation_token = CancellationToken::new();
    let token_clone = cancellation_token.clone();

    // Load HTTP-specific config from environment
    let http_config = HttpConfig::from_env();
    let auth_config = AuthConfig::new(http_config.bearer_token.clone());

    // Warn about security configuration
    emit_security_warnings(host, &http_config, &auth_config);

    // Create SSE service for MCP
    let session_manager = Arc::new(LocalSessionManager::default());
    let config = StreamableHttpServerConfig {
        cancellation_token: token_clone,
        ..Default::default()
    };

    let mcp_service =
        StreamableHttpService::new(move || Ok(handler.clone()), session_manager, config);

    // Build CORS layer with configurable origin
    let cors = build_cors_layer(&http_config);

    #[allow(unused_mut)]
    let mut app = Router::new().route("/health", get(health_handler));

    #[cfg(feature = "metrics")]
    {
        app = app.route("/metrics", get(metrics_handler));
    }

    // Admin endpoints (require authentication via middleware)
    app = app.route("/admin/reload", post(admin_reload_handler));

    let app = app
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(
            axum::extract::Extension(auth_config.clone()),
            bearer_auth_middleware,
        ))
        .layer(axum::extract::Extension(auth_config))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(60),
        ))
        .layer(cors);

    tracing::info!("HTTP server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::Transport(format!("Failed to bind to {addr}: {e}")))?;

    // Spawn shutdown handler
    tokio::spawn(async move {
        shutdown.await;
        cancellation_token.cancel();
    });

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::Transport(format!("HTTP server error: {e}")))?;

    tracing::info!("HTTP server shutdown complete");
    Ok(())
}

fn build_cors_layer(config: &HttpConfig) -> CorsLayer {
    config
        .cors_origin
        .as_ref()
        .and_then(|o| o.parse::<HeaderValue>().ok())
        .map_or_else(
            || {
                // Restrictive default: only localhost
                CorsLayer::new()
                    .allow_origin(
                        "http://localhost:3000"
                            .parse::<HeaderValue>()
                            .expect("valid header"),
                    )
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
            },
            |origin_value| {
                CorsLayer::new()
                    .allow_origin(origin_value)
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
            },
        )
}

fn emit_security_warnings(host: IpAddr, http_config: &HttpConfig, auth_config: &AuthConfig) {
    let is_non_loopback = !host.is_loopback();
    let is_all_interfaces = host == IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        || host == IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED);

    if is_all_interfaces {
        tracing::warn!(
            "HTTP server binding to all interfaces (0.0.0.0). \
             This exposes the server to all network interfaces."
        );
    } else if is_non_loopback {
        tracing::warn!(
            "HTTP server binding to non-loopback address ({host}). \
             Ensure network security policies are in place."
        );
    }

    if !auth_config.is_enabled() && is_non_loopback {
        tracing::warn!(
            "SECURITY WARNING: HTTP server accessible from network without authentication. \
             Set MCP_HTTP_BEARER_TOKEN environment variable to enable authentication."
        );
    }

    if http_config.cors_origin.is_none() {
        tracing::info!(
            "CORS origin not configured (MCP_CORS_ORIGIN). \
             Using restrictive default: http://localhost:3000"
        );
    }
}

async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(feature = "metrics")]
async fn metrics_handler() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        crate::observability::render_metrics(),
    )
}

async fn admin_reload_handler(Json(payload): Json<ReloadRequest>) -> impl IntoResponse {
    use crate::config::{ReloadResult, ReloadTrigger};

    tracing::info!(
        trigger = %ReloadTrigger::HttpEndpoint { remote_addr: None },
        force = payload.force,
        "Configuration reload requested"
    );

    // For now, just acknowledge the request
    // Full implementation would reload config from file/env and update RuntimeConfigHolder
    let result = ReloadResult::success(vec![]);

    let response = ReloadResponse {
        success: result.success,
        message: if result.success {
            "Configuration reload acknowledged".to_string()
        } else {
            result.error.unwrap_or_default()
        },
        changed: result.changed,
    };

    if result.success {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok",
            version: "0.3.2",
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("0.3.2"));
    }

    #[test]
    fn test_http_config_default() {
        let config = HttpConfig::default();
        assert_eq!(
            config.cors_origin,
            Some("http://localhost:3000".to_string())
        );
        assert!(config.bearer_token.is_none());
    }

    #[test]
    fn test_build_cors_layer_with_origin() {
        let config = HttpConfig {
            cors_origin: Some("https://example.com".to_string()),
            bearer_token: None,
        };
        let _cors = build_cors_layer(&config);
    }

    #[test]
    fn test_build_cors_layer_without_origin() {
        let config = HttpConfig {
            cors_origin: None,
            bearer_token: None,
        };
        let _cors = build_cors_layer(&config);
    }

    #[test]
    fn test_security_warnings_emitted_for_non_loopback() {
        let host = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let http_config = HttpConfig::default();
        let auth_config = AuthConfig::new(None);
        emit_security_warnings(host, &http_config, &auth_config);
    }

    #[test]
    fn test_loopback_no_warning_needed() {
        let host = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let http_config = HttpConfig::default();
        let auth_config = AuthConfig::new(Some("token".to_string()));
        emit_security_warnings(host, &http_config, &auth_config);
    }
}
