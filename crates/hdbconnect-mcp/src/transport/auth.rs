//! HTTP authentication middleware

use std::sync::Arc;

use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

/// Bearer token authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    token: Option<Arc<str>>,
}

impl AuthConfig {
    pub fn new(token: Option<String>) -> Self {
        Self {
            token: token.map(Into::into),
        }
    }

    pub const fn is_enabled(&self) -> bool {
        self.token.is_some()
    }
}

/// Extract and validate Bearer token from Authorization header
pub async fn bearer_auth_middleware(
    headers: HeaderMap,
    auth: axum::extract::Extension<AuthConfig>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(ref expected_token) = auth.token else {
        return Ok(next.run(request).await);
    };

    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header_value) if header_value.starts_with("Bearer ") => {
            let provided_token = &header_value[7..];
            if provided_token == expected_token.as_ref() {
                Ok(next.run(request).await)
            } else {
                tracing::warn!("Invalid bearer token provided");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Some(_) => {
            tracing::warn!("Invalid Authorization header format");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!("Missing Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_disabled() {
        let config = AuthConfig::new(None);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_auth_config_enabled() {
        let config = AuthConfig::new(Some("test-token".to_string()));
        assert!(config.is_enabled());
    }
}
