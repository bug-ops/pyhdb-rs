//! JWT parsing and validation

use std::sync::Arc;

use jsonwebtoken::{DecodingKey, Validation, decode_header};

use super::claims::JwtClaims;
use super::config::JwtConfig;
use super::error::{AuthError, Result};
use super::jwks::JwksCache;

/// JWT validator
pub struct JwtValidator {
    config: JwtConfig,
    jwks_cache: Option<Arc<JwksCache>>,
    hs_key: Option<DecodingKey>,
}

impl std::fmt::Debug for JwtValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtValidator")
            .field("issuer", &self.config.issuer)
            .field("has_jwks_cache", &self.jwks_cache.is_some())
            .field("has_hs_key", &self.hs_key.is_some())
            .finish()
    }
}

impl JwtValidator {
    #[must_use]
    pub fn new(config: JwtConfig, jwks_cache: Option<Arc<JwksCache>>) -> Self {
        let hs_key = config
            .hs_secret
            .as_ref()
            .map(|s| DecodingKey::from_secret(s.as_bytes()));
        Self {
            config,
            jwks_cache,
            hs_key,
        }
    }

    pub async fn validate(&self, token: &str) -> Result<JwtClaims> {
        let header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;

        let key = self
            .get_decoding_key(header.kid.as_deref(), header.alg)
            .await?;

        let mut validation = Validation::new(header.alg);

        // Normalize issuer by removing trailing slash for comparison
        let issuer = self.config.issuer.as_str().trim_end_matches('/');
        validation.set_issuer(&[issuer]);

        if self.config.audience.is_empty() {
            validation.validate_aud = false;
        } else {
            validation.set_audience(&self.config.audience);
        }

        validation.leeway = self.config.clock_skew.as_secs();

        let token_data = jsonwebtoken::decode::<JwtClaims>(token, &key, &validation)?;

        Ok(token_data.claims)
    }

    async fn get_decoding_key(
        &self,
        kid: Option<&str>,
        alg: jsonwebtoken::Algorithm,
    ) -> Result<DecodingKey> {
        // For HS* algorithms, use the secret key
        if matches!(
            alg,
            jsonwebtoken::Algorithm::HS256
                | jsonwebtoken::Algorithm::HS384
                | jsonwebtoken::Algorithm::HS512
        ) {
            return self
                .hs_key
                .clone()
                .ok_or_else(|| AuthError::Config("HS secret not configured".into()));
        }

        // For RS*/ES* algorithms, use JWKS
        let jwks_cache = self.jwks_cache.as_ref().ok_or_else(|| {
            AuthError::Config("JWKS not configured for asymmetric algorithm".into())
        })?;

        jwks_cache.get_key(kid, alg).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;
    use url::Url;

    use super::*;

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        iss: String,
        exp: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        aud: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tenant_id: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        roles: Vec<String>,
    }

    fn create_test_token(claims: &TestClaims, secret: &str) -> String {
        encode(
            &Header::new(jsonwebtoken::Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    fn current_time() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[tokio::test]
    async fn test_validate_valid_token() {
        let secret = "test-secret-key-at-least-32-bytes-long";
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            audience: vec![],
            hs_secret: Some(secret.to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);

        let claims = TestClaims {
            sub: "user123".to_string(),
            iss: "https://auth.example.com".to_string(),
            exp: current_time() + 3600,
            aud: None,
            tenant_id: Some("tenant1".to_string()),
            roles: vec!["admin".to_string()],
        };

        let token = create_test_token(&claims, secret);
        let validated = validator.validate(&token).await.unwrap();

        assert_eq!(validated.standard.sub, "user123");
        assert_eq!(validated.custom.tenant_id, Some("tenant1".to_string()));
        assert_eq!(validated.custom.roles, vec!["admin"]);
    }

    #[tokio::test]
    async fn test_validate_expired_token() {
        let secret = "test-secret-key-at-least-32-bytes-long";
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            hs_secret: Some(secret.to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);

        let claims = TestClaims {
            sub: "user123".to_string(),
            iss: "https://auth.example.com".to_string(),
            exp: current_time() - 3600,
            aud: None,
            tenant_id: None,
            roles: vec![],
        };

        let token = create_test_token(&claims, secret);
        let result = validator.validate(&token).await;

        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_validate_wrong_issuer() {
        let secret = "test-secret-key-at-least-32-bytes-long";
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            hs_secret: Some(secret.to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);

        let claims = TestClaims {
            sub: "user123".to_string(),
            iss: "https://wrong-issuer.com".to_string(),
            exp: current_time() + 3600,
            aud: None,
            tenant_id: None,
            roles: vec![],
        };

        let token = create_test_token(&claims, secret);
        let result = validator.validate(&token).await;

        assert!(matches!(result, Err(AuthError::InvalidIssuer)));
    }

    #[tokio::test]
    async fn test_validate_wrong_secret() {
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            hs_secret: Some("correct-secret-key-at-least-32-bytes".to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);

        let claims = TestClaims {
            sub: "user123".to_string(),
            iss: "https://auth.example.com".to_string(),
            exp: current_time() + 3600,
            aud: None,
            tenant_id: None,
            roles: vec![],
        };

        let token = create_test_token(&claims, "wrong-secret-key-at-least-32-bytes");
        let result = validator.validate(&token).await;

        assert!(matches!(result, Err(AuthError::InvalidSignature)));
    }

    #[tokio::test]
    async fn test_validate_with_audience() {
        let secret = "test-secret-key-at-least-32-bytes-long";
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            audience: vec!["api".to_string()],
            hs_secret: Some(secret.to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);

        let claims = TestClaims {
            sub: "user123".to_string(),
            iss: "https://auth.example.com".to_string(),
            exp: current_time() + 3600,
            aud: Some("api".to_string()),
            tenant_id: None,
            roles: vec![],
        };

        let token = create_test_token(&claims, secret);
        let result = validator.validate(&token).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_invalid_audience() {
        let secret = "test-secret-key-at-least-32-bytes-long";
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            audience: vec!["api".to_string()],
            hs_secret: Some(secret.to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);

        let claims = TestClaims {
            sub: "user123".to_string(),
            iss: "https://auth.example.com".to_string(),
            exp: current_time() + 3600,
            aud: Some("wrong-audience".to_string()),
            tenant_id: None,
            roles: vec![],
        };

        let token = create_test_token(&claims, secret);
        let result = validator.validate(&token).await;

        assert!(matches!(result, Err(AuthError::InvalidAudience)));
    }

    #[tokio::test]
    async fn test_validate_malformed_token() {
        let config = JwtConfig {
            issuer: Url::parse("https://auth.example.com").unwrap(),
            hs_secret: Some("secret".to_string()),
            ..Default::default()
        };

        let validator = JwtValidator::new(config, None);
        let result = validator.validate("not.a.valid.token").await;

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }
}
