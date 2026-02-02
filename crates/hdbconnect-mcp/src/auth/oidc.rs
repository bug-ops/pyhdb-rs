//! OIDC support using openidconnect crate
//!
//! Provides OIDC discovery and token validation using `openidconnect` types directly.
//! Custom claims are implemented via `AdditionalClaims` trait.

use std::sync::Arc;
use std::time::Duration;

use openidconnect::core::CoreProviderMetadata;
use openidconnect::{ClientId, ClientSecret, IssuerUrl};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::claims::AuthenticatedUser;
use super::config::JwtConfig;
use super::error::{AuthError, Result};
use super::jwt::JwtValidator;

/// Custom claims for tenant and role information.
/// Standard claims (sub, iss, aud, exp, etc.) are accessed via `CoreIdTokenClaims` methods.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantClaims {
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub roles: Option<Vec<String>>,
}

impl openidconnect::AdditionalClaims for TenantClaims {}

/// ID token claims type alias for convenience
pub type IdTokenClaims =
    openidconnect::IdTokenClaims<TenantClaims, openidconnect::core::CoreGenderClaim>;

/// OIDC client configuration
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// OIDC issuer URL for discovery
    pub issuer_url: IssuerUrl,
    /// `OAuth2` client ID
    pub client_id: ClientId,
    /// `OAuth2` client secret (optional for public clients)
    pub client_secret: Option<ClientSecret>,
    /// Expected audience(s)
    pub audience: Vec<String>,
    /// Clock skew tolerance
    pub clock_skew: Duration,
    /// Nonce verification mode
    pub verify_nonce: bool,
}

impl OidcConfig {
    pub fn new(issuer: &str, client_id: &str) -> Result<Self> {
        let issuer_url = IssuerUrl::new(issuer.to_string())
            .map_err(|e| AuthError::Config(format!("Invalid issuer URL: {e}")))?;

        Ok(Self {
            issuer_url,
            client_id: ClientId::new(client_id.to_string()),
            client_secret: None,
            audience: vec![],
            clock_skew: Duration::from_secs(60),
            verify_nonce: false,
        })
    }

    #[must_use]
    pub fn with_client_secret(mut self, secret: String) -> Self {
        self.client_secret = Some(ClientSecret::new(secret));
        self
    }

    #[must_use]
    pub fn with_audience(mut self, audience: Vec<String>) -> Self {
        self.audience = audience;
        self
    }

    #[must_use]
    pub const fn with_nonce_verification(mut self, verify: bool) -> Self {
        self.verify_nonce = verify;
        self
    }

    /// Convert to `JwtConfig` for use with `JwtValidator`
    #[must_use]
    pub fn to_jwt_config(&self) -> JwtConfig {
        let issuer = url::Url::parse(self.issuer_url.as_str())
            .unwrap_or_else(|_| url::Url::parse("https://example.com").unwrap());

        JwtConfig {
            issuer,
            audience: self.audience.clone(),
            jwks_uri: None,
            clock_skew: self.clock_skew,
            hs_secret: None,
            jwks_cache_ttl: Duration::from_secs(3600),
            jwks_refresh_interval: Duration::from_secs(300),
        }
    }
}

/// OIDC client wrapper providing JWKS discovery.
/// Uses `openidconnect::reqwest::Client` internally (re-exported reqwest 0.12.x).
pub struct OidcClient {
    config: OidcConfig,
    jwt_validator: JwtValidator,
}

impl std::fmt::Debug for OidcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcClient")
            .field("issuer", &self.config.issuer_url)
            .field("client_id", &self.config.client_id)
            .finish_non_exhaustive()
    }
}

impl OidcClient {
    /// Create OIDC client via discovery
    pub async fn discover(config: OidcConfig) -> Result<Self> {
        tracing::info!(issuer = %config.issuer_url, "Discovering OIDC provider");

        // Use the reqwest client that openidconnect re-exports to avoid version conflicts
        let http_client = openidconnect::reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AuthError::Config(format!("Failed to create HTTP client: {e}")))?;

        let provider_metadata =
            CoreProviderMetadata::discover_async(config.issuer_url.clone(), &http_client)
                .await
                .map_err(|e| AuthError::DiscoveryFailed(e.to_string()))?;

        // Get JWKS URI from provider metadata
        let jwks_endpoint = provider_metadata.jwks_uri().clone();
        let jwks_endpoint_url = url::Url::parse(jwks_endpoint.as_str())
            .map_err(|e| AuthError::Config(format!("Invalid JWKS URI: {e}")))?;

        // Create JWT config with discovered JWKS URI
        let mut jwt_config = config.to_jwt_config();
        jwt_config.jwks_uri = Some(jwks_endpoint_url.clone());

        // Create JWKS cache and validator
        let jwks_cache = Arc::new(super::jwks::JwksCache::new(
            jwks_endpoint_url,
            jwt_config.jwks_cache_ttl,
        ));

        // Initial JWKS fetch
        jwks_cache.refresh().await?;

        let jwt_validator = JwtValidator::new(jwt_config, Some(jwks_cache));

        tracing::info!("OIDC discovery complete");

        Ok(Self {
            config,
            jwt_validator,
        })
    }

    /// Create OIDC client with a pre-configured JWKS URI (without discovery)
    pub async fn with_jwks_uri(config: OidcConfig, jwks_endpoint: url::Url) -> Result<Self> {
        tracing::info!(issuer = %config.issuer_url, jwks_uri = %jwks_endpoint, "Creating OIDC client with explicit JWKS URI");

        // Create JWT config with provided JWKS URI
        let mut jwt_config = config.to_jwt_config();
        jwt_config.jwks_uri = Some(jwks_endpoint.clone());

        // Create JWKS cache and validator
        let jwks_cache = Arc::new(super::jwks::JwksCache::new(
            jwks_endpoint,
            jwt_config.jwks_cache_ttl,
        ));

        // Initial JWKS fetch
        jwks_cache.refresh().await?;

        let jwt_validator = JwtValidator::new(jwt_config, Some(jwks_cache));

        tracing::info!("OIDC client created with explicit JWKS URI");

        Ok(Self {
            config,
            jwt_validator,
        })
    }

    /// Validate an ID token and extract claims
    pub async fn validate_token(&self, token_str: &str) -> Result<super::claims::JwtClaims> {
        self.jwt_validator.validate(token_str).await
    }

    /// Get the config
    #[must_use]
    pub const fn config(&self) -> &OidcConfig {
        &self.config
    }
}

/// Cached OIDC client with automatic refresh
pub struct CachedOidcClient {
    client: RwLock<Option<Arc<OidcClient>>>,
    config: OidcConfig,
    refresh_interval: Duration,
}

impl std::fmt::Debug for CachedOidcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedOidcClient")
            .field("issuer", &self.config.issuer_url)
            .field("refresh_interval", &self.refresh_interval)
            .finish_non_exhaustive()
    }
}

impl CachedOidcClient {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(config: OidcConfig, refresh_interval: Duration) -> Self {
        Self {
            client: RwLock::new(None),
            config,
            refresh_interval,
        }
    }

    /// Get or initialize the OIDC client
    pub async fn get(&self) -> Result<Arc<OidcClient>> {
        // Check if we have a cached client
        {
            let guard = self.client.read();
            if let Some(ref client) = *guard {
                return Ok(Arc::clone(client));
            }
        }

        // Initialize client
        let new_client = Arc::new(OidcClient::discover(self.config.clone()).await?);

        {
            let mut guard = self.client.write();
            *guard = Some(Arc::clone(&new_client));
        }

        Ok(new_client)
    }

    /// Spawn background refresh task
    pub fn spawn_refresh(
        self: Arc<Self>,
        shutdown: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let interval = self.refresh_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        tracing::debug!("Refreshing OIDC discovery");
                        match OidcClient::discover(self.config.clone()).await {
                            Ok(new_client) => {
                                *self.client.write() = Some(Arc::new(new_client));
                                tracing::debug!("OIDC discovery refreshed");
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "OIDC discovery refresh failed");
                            }
                        }
                    }
                    () = shutdown.cancelled() => {
                        tracing::debug!("OIDC refresh task shutting down");
                        break;
                    }
                }
            }
        })
    }
}

/// Extract `AuthenticatedUser` from OIDC claims
impl From<&IdTokenClaims> for AuthenticatedUser {
    fn from(claims: &IdTokenClaims) -> Self {
        let additional = claims.additional_claims();

        Self {
            sub: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string()),
            tenant_id: additional.tenant_id.clone(),
            tenant_schema: None, // Resolved by TenantResolver
            roles: additional.roles.clone().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oidc_config_new() {
        let config = OidcConfig::new("https://auth.example.com", "my-client-id").unwrap();
        assert_eq!(config.issuer_url.as_str(), "https://auth.example.com");
        assert!(config.client_secret.is_none());
        assert!(config.audience.is_empty());
    }

    #[test]
    fn test_oidc_config_with_secret() {
        let config = OidcConfig::new("https://auth.example.com", "my-client-id")
            .unwrap()
            .with_client_secret("secret123".to_string());
        assert!(config.client_secret.is_some());
    }

    #[test]
    fn test_oidc_config_with_audience() {
        let config = OidcConfig::new("https://auth.example.com", "my-client-id")
            .unwrap()
            .with_audience(vec!["api".to_string(), "web".to_string()]);
        assert_eq!(config.audience, vec!["api", "web"]);
    }

    #[test]
    fn test_oidc_config_invalid_url() {
        let result = OidcConfig::new("not-a-url", "client");
        assert!(result.is_err());
    }

    #[test]
    fn test_tenant_claims_default() {
        let claims = TenantClaims::default();
        assert!(claims.tenant_id.is_none());
        assert!(claims.roles.is_none());
    }

    #[test]
    fn test_tenant_claims_deserialize() {
        let json = r#"{"tenant_id": "tenant1", "roles": ["admin", "user"]}"#;
        let claims: TenantClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.tenant_id, Some("tenant1".to_string()));
        assert_eq!(
            claims.roles,
            Some(vec!["admin".to_string(), "user".to_string()])
        );
    }

    #[test]
    fn test_tenant_claims_deserialize_minimal() {
        let json = r#"{}"#;
        let claims: TenantClaims = serde_json::from_str(json).unwrap();
        assert!(claims.tenant_id.is_none());
        assert!(claims.roles.is_none());
    }

    #[test]
    fn test_oidc_config_to_jwt_config() {
        let config = OidcConfig::new("https://auth.example.com", "my-client-id")
            .unwrap()
            .with_audience(vec!["api".to_string()]);

        let jwt_config = config.to_jwt_config();
        assert_eq!(jwt_config.issuer.as_str(), "https://auth.example.com/");
        assert_eq!(jwt_config.audience, vec!["api"]);
    }
}
