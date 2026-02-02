//! Authentication error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("authentication required")]
    NotAuthenticated,

    #[error("invalid token")]
    InvalidToken,

    #[error("token expired")]
    TokenExpired,

    #[error("invalid issuer")]
    InvalidIssuer,

    #[error("invalid audience")]
    InvalidAudience,

    #[error("invalid signature")]
    InvalidSignature,

    #[error("key not found: {0}")]
    KeyNotFound(String),

    #[error("no matching key for algorithm")]
    NoMatchingKey,

    #[error("missing tenant claim")]
    MissingTenantClaim,

    #[error("insufficient permissions")]
    InsufficientPermissions,

    #[error("OIDC discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("JWKS fetch failed: {0}")]
    JwksFetch(#[from] reqwest::Error),

    #[error("JWKS parse failed: {0}")]
    JwksParse(String),

    #[error("token validation failed: {0}")]
    ValidationFailed(String),

    #[error("configuration error: {0}")]
    Config(String),
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => Self::TokenExpired,
            ErrorKind::InvalidIssuer => Self::InvalidIssuer,
            ErrorKind::InvalidAudience => Self::InvalidAudience,
            ErrorKind::InvalidSignature => Self::InvalidSignature,
            _ => Self::InvalidToken,
        }
    }
}

impl From<AuthError> for rmcp::ErrorData {
    fn from(err: AuthError) -> Self {
        match &err {
            AuthError::NotAuthenticated
            | AuthError::InvalidToken
            | AuthError::TokenExpired
            | AuthError::InvalidIssuer
            | AuthError::InvalidAudience
            | AuthError::InvalidSignature
            | AuthError::KeyNotFound(_)
            | AuthError::NoMatchingKey => Self::invalid_params(err.to_string(), None),

            AuthError::InsufficientPermissions => {
                Self::invalid_params(format!("Authorization failed: {err}"), None)
            }

            AuthError::MissingTenantClaim => {
                Self::invalid_params(format!("Tenant resolution failed: {err}"), None)
            }

            AuthError::DiscoveryFailed(_)
            | AuthError::JwksFetch(_)
            | AuthError::JwksParse(_)
            | AuthError::ValidationFailed(_)
            | AuthError::Config(_) => Self::internal_error(err.to_string(), None),
        }
    }
}

pub type Result<T> = std::result::Result<T, AuthError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        assert_eq!(
            AuthError::NotAuthenticated.to_string(),
            "authentication required"
        );
        assert_eq!(AuthError::InvalidToken.to_string(), "invalid token");
        assert_eq!(AuthError::TokenExpired.to_string(), "token expired");
    }

    #[test]
    fn test_key_not_found_error() {
        let err = AuthError::KeyNotFound("kid123".to_string());
        assert_eq!(err.to_string(), "key not found: kid123");
    }

    #[test]
    fn test_no_matching_key_error() {
        let err = AuthError::NoMatchingKey;
        assert_eq!(err.to_string(), "no matching key for algorithm");
    }

    #[test]
    fn test_jwks_parse_error() {
        let err = AuthError::JwksParse("invalid JSON".to_string());
        assert_eq!(err.to_string(), "JWKS parse failed: invalid JSON");
    }

    #[test]
    fn test_auth_error_to_error_data() {
        let err: rmcp::ErrorData = AuthError::NotAuthenticated.into();
        assert!(err.message.contains("authentication required"));
    }

    #[test]
    fn test_insufficient_permissions_error() {
        let err: rmcp::ErrorData = AuthError::InsufficientPermissions.into();
        assert!(err.message.contains("Authorization failed"));
    }

    #[test]
    fn test_missing_tenant_claim_error() {
        let err: rmcp::ErrorData = AuthError::MissingTenantClaim.into();
        assert!(err.message.contains("Tenant resolution failed"));
    }

    #[test]
    fn test_internal_error_conversion() {
        let err: rmcp::ErrorData = AuthError::DiscoveryFailed("failed".into()).into();
        assert!(err.message.contains("OIDC discovery failed"));

        let err: rmcp::ErrorData = AuthError::Config("bad config".into()).into();
        assert!(err.message.contains("configuration error"));
    }

    #[test]
    fn test_key_errors_conversion() {
        let err: rmcp::ErrorData = AuthError::KeyNotFound("kid".into()).into();
        assert!(err.message.contains("key not found"));

        let err: rmcp::ErrorData = AuthError::NoMatchingKey.into();
        assert!(err.message.contains("no matching key"));
    }
}
