//! Authentication configuration types

use std::collections::HashMap;
use std::time::Duration;

use url::Url;

/// Authentication mode
#[derive(Debug, Clone, Default)]
pub enum AuthMode {
    /// No authentication required (default for backward compatibility)
    #[default]
    None,
    /// Simple bearer token (existing behavior)
    BearerToken(String),
    /// JWT validation with OIDC
    Jwt(Box<JwtConfig>),
}

/// JWT/OIDC configuration
#[derive(Clone)]
pub struct JwtConfig {
    /// OIDC issuer URL (used for discovery and `iss` validation)
    pub issuer: Url,
    /// Expected audience claims (must contain at least one match)
    pub audience: Vec<String>,
    /// JWKS URI (if not using OIDC discovery)
    pub jwks_uri: Option<Url>,
    /// Clock skew tolerance for exp/nbf validation
    pub clock_skew: Duration,
    /// HS256/384/512 secret for symmetric signing (testing/dev only)
    pub hs_secret: Option<String>,
    /// JWKS cache TTL
    pub jwks_cache_ttl: Duration,
    /// JWKS refresh interval for background refresh
    pub jwks_refresh_interval: Duration,
}

// Custom Debug impl that redacts hs_secret for security
impl std::fmt::Debug for JwtConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Completely omit hs_secret from debug output to prevent data flow analysis false positives
        f.debug_struct("JwtConfig")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("jwks_uri", &self.jwks_uri)
            .field("clock_skew", &self.clock_skew)
            .field("jwks_cache_ttl", &self.jwks_cache_ttl)
            .field("jwks_refresh_interval", &self.jwks_refresh_interval)
            .finish_non_exhaustive()
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            issuer: Url::parse("https://example.com").expect("valid default URL"),
            audience: vec![],
            jwks_uri: None,
            clock_skew: Duration::from_secs(60),
            hs_secret: None,
            jwks_cache_ttl: Duration::from_secs(3600),
            jwks_refresh_interval: Duration::from_secs(300),
        }
    }
}

impl JwtConfig {
    #[must_use]
    pub fn new(issuer: Url) -> Self {
        Self {
            issuer,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_audience(mut self, audience: Vec<String>) -> Self {
        self.audience = audience;
        self
    }

    #[must_use]
    pub fn with_jwks_uri(mut self, uri: Url) -> Self {
        self.jwks_uri = Some(uri);
        self
    }

    #[must_use]
    pub fn with_hs_secret(mut self, secret: String) -> Self {
        self.hs_secret = Some(secret);
        self
    }
}

/// How to map tenant ID to database schema
#[derive(Debug, Clone, Default)]
pub enum SchemaMappingStrategy {
    /// Use tenant ID directly as schema name (uppercase)
    #[default]
    Direct,
    /// Add prefix to tenant ID: "{prefix}_{tenant}"
    Prefix(String),
    /// Add suffix to tenant ID: "{tenant}_{suffix}"
    Suffix(String),
    /// Custom mapping via configuration
    Lookup(HashMap<String, String>),
}

/// Multi-tenancy configuration
#[derive(Debug, Clone)]
pub struct TenantConfig {
    /// Enable multi-tenancy
    pub enabled: bool,
    /// JWT claim name containing tenant ID
    pub tenant_claim: String,
    /// Tenant ID to schema mapping strategy
    pub schema_mapping: SchemaMappingStrategy,
    /// Default schema if tenant claim is missing (None = reject)
    pub default_schema: Option<String>,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tenant_claim: "tenant_id".to_string(),
            schema_mapping: SchemaMappingStrategy::Direct,
            default_schema: None,
        }
    }
}

/// Role-based access control configuration
#[derive(Debug, Clone)]
pub struct RbacConfig {
    /// Enable RBAC
    pub enabled: bool,
    /// JWT claim name containing roles
    pub roles_claim: String,
    /// Role required for read operations
    pub read_role: Option<String>,
    /// Role required for write operations (DML)
    pub write_role: Option<String>,
    /// Role required for procedure execution
    pub execute_role: Option<String>,
    /// Admin role (bypasses all checks)
    pub admin_role: Option<String>,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            roles_claim: "roles".to_string(),
            read_role: None,
            write_role: None,
            execute_role: None,
            admin_role: None,
        }
    }
}

/// Complete authentication configuration
#[derive(Debug, Clone, Default)]
pub struct AuthConfig {
    /// Authentication mode
    pub mode: AuthMode,
    /// Multi-tenancy settings
    pub tenant: TenantConfig,
    /// RBAC settings
    pub rbac: RbacConfig,
}

impl AuthConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mode: AuthMode::None,
            tenant: TenantConfig {
                enabled: false,
                tenant_claim: String::new(),
                schema_mapping: SchemaMappingStrategy::Direct,
                default_schema: None,
            },
            rbac: RbacConfig {
                enabled: false,
                roles_claim: String::new(),
                read_role: None,
                write_role: None,
                execute_role: None,
                admin_role: None,
            },
        }
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        !matches!(self.mode, AuthMode::None)
    }

    #[must_use]
    pub const fn is_jwt_mode(&self) -> bool {
        matches!(self.mode, AuthMode::Jwt(_))
    }

    #[must_use]
    pub const fn jwt_config(&self) -> Option<&JwtConfig> {
        match &self.mode {
            AuthMode::Jwt(config) => Some(config),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_mode_default() {
        let mode = AuthMode::default();
        assert!(matches!(mode, AuthMode::None));
    }

    #[test]
    fn test_jwt_config_new() {
        let issuer = Url::parse("https://auth.example.com").unwrap();
        let config = JwtConfig::new(issuer);

        assert!(config.audience.is_empty());
        assert_eq!(config.clock_skew, Duration::from_secs(60));
    }

    #[test]
    fn test_jwt_config_builder() {
        let issuer = Url::parse("https://auth.example.com").unwrap();
        let jwks_uri = Url::parse("https://auth.example.com/.well-known/jwks.json").unwrap();

        let config = JwtConfig::new(issuer)
            .with_audience(vec!["api".to_string()])
            .with_jwks_uri(jwks_uri.clone())
            .with_hs_secret("secret".to_string());

        assert_eq!(config.audience, vec!["api"]);
        assert_eq!(config.jwks_uri, Some(jwks_uri));
        assert_eq!(config.hs_secret, Some("secret".to_string()));
    }

    #[test]
    fn test_auth_config_is_enabled() {
        let config = AuthConfig::default();
        assert!(!config.is_enabled());

        let config = AuthConfig {
            mode: AuthMode::BearerToken("token".to_string()),
            ..Default::default()
        };
        assert!(config.is_enabled());
    }

    #[test]
    fn test_auth_config_is_jwt_mode() {
        let config = AuthConfig::default();
        assert!(!config.is_jwt_mode());

        let issuer = Url::parse("https://auth.example.com").unwrap();
        let config = AuthConfig {
            mode: AuthMode::Jwt(Box::new(JwtConfig::new(issuer))),
            ..Default::default()
        };
        assert!(config.is_jwt_mode());
    }

    #[test]
    fn test_tenant_config_default() {
        let config = TenantConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.tenant_claim, "tenant_id");
    }

    #[test]
    fn test_rbac_config_default() {
        let config = RbacConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.roles_claim, "roles");
    }

    #[test]
    fn test_schema_mapping_strategy() {
        let direct = SchemaMappingStrategy::Direct;
        assert!(matches!(direct, SchemaMappingStrategy::Direct));

        let prefix = SchemaMappingStrategy::Prefix("APP".to_string());
        assert!(matches!(prefix, SchemaMappingStrategy::Prefix(_)));

        let suffix = SchemaMappingStrategy::Suffix("DATA".to_string());
        assert!(matches!(suffix, SchemaMappingStrategy::Suffix(_)));
    }

    #[test]
    fn test_schema_mapping_lookup() {
        let mut map = HashMap::new();
        map.insert("tenant1".to_string(), "SCHEMA_1".to_string());
        map.insert("tenant2".to_string(), "SCHEMA_2".to_string());

        let lookup = SchemaMappingStrategy::Lookup(map);
        assert!(matches!(lookup, SchemaMappingStrategy::Lookup(_)));
    }

    #[test]
    fn test_jwt_config_debug_redacts_secret() {
        let issuer = Url::parse("https://auth.example.com").unwrap();
        let config = JwtConfig::new(issuer)
            .with_hs_secret("super_secret_key".to_string());

        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("super_secret_key"));
        assert!(debug_str.contains("issuer"));
    }

    #[test]
    fn test_jwt_config_default() {
        let config = JwtConfig::default();
        assert!(config.audience.is_empty());
        assert!(config.jwks_uri.is_none());
        assert!(config.hs_secret.is_none());
        assert_eq!(config.clock_skew, Duration::from_secs(60));
        assert_eq!(config.jwks_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.jwks_refresh_interval, Duration::from_secs(300));
    }

    #[test]
    fn test_auth_config_new_const() {
        let config = AuthConfig::new();
        assert!(!config.is_enabled());
        assert!(!config.is_jwt_mode());
        assert!(config.jwt_config().is_none());
    }

    #[test]
    fn test_auth_config_jwt_config_returns_some() {
        let issuer = Url::parse("https://auth.example.com").unwrap();
        let jwt_config = JwtConfig::new(issuer.clone());
        let config = AuthConfig {
            mode: AuthMode::Jwt(Box::new(jwt_config)),
            ..Default::default()
        };

        let returned = config.jwt_config();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().issuer, issuer);
    }

    #[test]
    fn test_auth_config_jwt_config_returns_none_for_bearer() {
        let config = AuthConfig {
            mode: AuthMode::BearerToken("token".to_string()),
            ..Default::default()
        };
        assert!(config.jwt_config().is_none());
    }

    #[test]
    fn test_tenant_config_with_custom_values() {
        let mut map = HashMap::new();
        map.insert("t1".to_string(), "S1".to_string());

        let config = TenantConfig {
            enabled: true,
            tenant_claim: "custom_tenant".to_string(),
            schema_mapping: SchemaMappingStrategy::Lookup(map),
            default_schema: Some("DEFAULT".to_string()),
        };

        assert!(config.enabled);
        assert_eq!(config.tenant_claim, "custom_tenant");
        assert_eq!(config.default_schema, Some("DEFAULT".to_string()));
    }

    #[test]
    fn test_rbac_config_with_all_roles() {
        let config = RbacConfig {
            enabled: true,
            roles_claim: "permissions".to_string(),
            read_role: Some("reader".to_string()),
            write_role: Some("writer".to_string()),
            execute_role: Some("executor".to_string()),
            admin_role: Some("admin".to_string()),
        };

        assert!(config.enabled);
        assert_eq!(config.read_role, Some("reader".to_string()));
        assert_eq!(config.write_role, Some("writer".to_string()));
        assert_eq!(config.execute_role, Some("executor".to_string()));
        assert_eq!(config.admin_role, Some("admin".to_string()));
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(matches!(config.mode, AuthMode::None));
        assert!(!config.tenant.enabled);
        assert!(!config.rbac.enabled);
    }
}
