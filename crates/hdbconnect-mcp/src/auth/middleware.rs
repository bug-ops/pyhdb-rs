//! JWT authentication middleware for HTTP transport
//!
//! This module requires the `http` feature for axum integration.

use std::sync::Arc;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use super::claims::AuthenticatedUser;
use super::config::{AuthConfig, AuthMode};
use super::jwt::JwtValidator;
use super::tenant::TenantResolver;

/// Authentication state for middleware
#[derive(Clone)]
pub struct AuthState {
    pub config: Arc<AuthConfig>,
    pub jwt_validator: Option<Arc<JwtValidator>>,
    pub tenant_resolver: Option<Arc<TenantResolver>>,
}

impl std::fmt::Debug for AuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthState")
            .field("config", &self.config)
            .field("has_jwt_validator", &self.jwt_validator.is_some())
            .field("has_tenant_resolver", &self.tenant_resolver.is_some())
            .finish()
    }
}

impl AuthState {
    #[must_use]
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
            jwt_validator: None,
            tenant_resolver: None,
        }
    }

    #[must_use]
    pub fn with_jwt_validator(mut self, validator: Arc<JwtValidator>) -> Self {
        self.jwt_validator = Some(validator);
        self
    }

    #[must_use]
    pub fn with_tenant_resolver(mut self, resolver: Arc<TenantResolver>) -> Self {
        self.tenant_resolver = Some(resolver);
        self
    }
}

/// JWT authentication middleware
#[allow(clippy::future_not_send)]
pub async fn jwt_auth_middleware(
    axum::extract::State(state): axum::extract::State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.config.is_enabled() {
        return Ok(next.run(request).await);
    }

    match &state.config.mode {
        AuthMode::None => Ok(next.run(request).await),

        AuthMode::BearerToken(expected) => {
            validate_bearer_token(&request, expected)?;
            Ok(next.run(request).await)
        }

        AuthMode::Jwt(_) => {
            let user = validate_jwt_token(&request, &state).await?;
            request.extensions_mut().insert(user);
            Ok(next.run(request).await)
        }
    }
}

fn validate_bearer_token(request: &Request, expected: &str) -> Result<(), StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok());

    match auth_header {
        Some(h) if h.starts_with("Bearer ") && &h[7..] == expected => Ok(()),
        Some(_) => {
            tracing::warn!("Invalid bearer token");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!("Missing Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[allow(clippy::future_not_send)]
async fn validate_jwt_token(
    request: &Request,
    state: &AuthState,
) -> Result<AuthenticatedUser, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("Missing Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    if !auth_header.starts_with("Bearer ") {
        tracing::warn!("Invalid Authorization header format");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    let validator = state.jwt_validator.as_ref().ok_or_else(|| {
        tracing::error!("JWT validator not configured");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let claims = validator.validate(token).await.map_err(|_e| {
        // Do not log error details to avoid leaking token information
        tracing::warn!("JWT validation failed");
        StatusCode::UNAUTHORIZED
    })?;

    let tenant_schema = state
        .tenant_resolver
        .as_ref()
        .and_then(|r| r.resolve(&claims).ok())
        .flatten();

    Ok(AuthenticatedUser::from_claims(&claims, tenant_schema))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request as HttpRequest;

    use super::*;

    fn create_test_request(auth_header: Option<&str>) -> Request {
        let mut builder = HttpRequest::builder().uri("/test").method("GET");
        if let Some(header) = auth_header {
            builder = builder.header(header::AUTHORIZATION, header);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[test]
    fn test_validate_bearer_token_success() {
        let request = create_test_request(Some("Bearer test-token"));
        assert!(validate_bearer_token(&request, "test-token").is_ok());
    }

    #[test]
    fn test_validate_bearer_token_wrong_token() {
        let request = create_test_request(Some("Bearer wrong-token"));
        assert!(validate_bearer_token(&request, "test-token").is_err());
    }

    #[test]
    fn test_validate_bearer_token_missing_header() {
        let request = create_test_request(None);
        assert!(validate_bearer_token(&request, "test-token").is_err());
    }

    #[test]
    fn test_validate_bearer_token_wrong_format() {
        let request = create_test_request(Some("Basic dXNlcjpwYXNz"));
        assert!(validate_bearer_token(&request, "test-token").is_err());
    }

    #[test]
    fn test_auth_state_new() {
        let config = AuthConfig::default();
        let state = AuthState::new(config);
        assert!(state.jwt_validator.is_none());
        assert!(state.tenant_resolver.is_none());
    }
}
