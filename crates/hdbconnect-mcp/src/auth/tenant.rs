//! Tenant resolution and schema mapping

use super::claims::JwtClaims;
use super::config::{SchemaMappingStrategy, TenantConfig};
use super::error::{AuthError, Result};
use crate::security::SchemaFilter;

/// Tenant resolver
#[derive(Debug, Clone)]
pub struct TenantResolver {
    config: TenantConfig,
}

impl TenantResolver {
    #[must_use]
    pub const fn new(config: TenantConfig) -> Self {
        Self { config }
    }

    /// Resolve tenant schema from JWT claims
    pub fn resolve(&self, claims: &JwtClaims) -> Result<Option<String>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let tenant_id = self.extract_tenant_id(claims)?;
        let schema = self.map_to_schema(&tenant_id);

        Ok(Some(schema))
    }

    fn extract_tenant_id(&self, claims: &JwtClaims) -> Result<String> {
        if let Some(tenant_id) = &claims.custom.tenant_id {
            return Ok(tenant_id.clone());
        }

        if let Some(default) = &self.config.default_schema {
            tracing::debug!(
                default_schema = %default,
                "Using default schema (tenant claim missing)"
            );
            return Ok(default.clone());
        }

        Err(AuthError::MissingTenantClaim)
    }

    fn map_to_schema(&self, tenant_id: &str) -> String {
        match &self.config.schema_mapping {
            SchemaMappingStrategy::Direct => tenant_id.to_uppercase(),
            SchemaMappingStrategy::Prefix(prefix) => format!("{prefix}_{tenant_id}").to_uppercase(),
            SchemaMappingStrategy::Suffix(suffix) => format!("{tenant_id}_{suffix}").to_uppercase(),
            SchemaMappingStrategy::Lookup(map) => map
                .get(tenant_id)
                .cloned()
                .unwrap_or_else(|| tenant_id.to_uppercase()),
        }
    }

    /// Create schema filter for tenant isolation
    #[must_use]
    pub fn create_schema_filter(&self, tenant_schema: &str) -> SchemaFilter {
        let mut allowed = std::collections::HashSet::new();
        allowed.insert(tenant_schema.to_uppercase());
        SchemaFilter::Whitelist(allowed)
    }
}

/// Get effective schema filter considering tenant isolation
#[must_use]
pub fn effective_schema_filter(
    server_filter: &SchemaFilter,
    tenant_schema: Option<&str>,
    is_admin: bool,
) -> SchemaFilter {
    if is_admin {
        return server_filter.clone();
    }

    if let Some(schema) = tenant_schema {
        let mut allowed = std::collections::HashSet::new();
        allowed.insert(schema.to_uppercase());
        return SchemaFilter::Whitelist(allowed);
    }

    server_filter.clone()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn create_claims(tenant_id: Option<&str>) -> JwtClaims {
        let json = if let Some(tid) = tenant_id {
            format!(
                r#"{{
                    "sub": "user123",
                    "iss": "https://auth.example.com",
                    "exp": 1700000000,
                    "tenant_id": "{tid}"
                }}"#
            )
        } else {
            r#"{
                "sub": "user123",
                "iss": "https://auth.example.com",
                "exp": 1700000000
            }"#
            .to_string()
        };
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn test_resolve_disabled() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: false,
            ..Default::default()
        });
        let claims = create_claims(Some("tenant1"));
        let result = resolver.resolve(&claims).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_direct_mapping() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            schema_mapping: SchemaMappingStrategy::Direct,
            ..Default::default()
        });
        let claims = create_claims(Some("tenant1"));
        let result = resolver.resolve(&claims).unwrap();
        assert_eq!(result, Some("TENANT1".to_string()));
    }

    #[test]
    fn test_resolve_prefix_mapping() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            schema_mapping: SchemaMappingStrategy::Prefix("APP".to_string()),
            ..Default::default()
        });
        let claims = create_claims(Some("tenant1"));
        let result = resolver.resolve(&claims).unwrap();
        assert_eq!(result, Some("APP_TENANT1".to_string()));
    }

    #[test]
    fn test_resolve_suffix_mapping() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            schema_mapping: SchemaMappingStrategy::Suffix("DATA".to_string()),
            ..Default::default()
        });
        let claims = create_claims(Some("tenant1"));
        let result = resolver.resolve(&claims).unwrap();
        assert_eq!(result, Some("TENANT1_DATA".to_string()));
    }

    #[test]
    fn test_resolve_lookup_mapping() {
        let mut map = HashMap::new();
        map.insert("tenant1".to_string(), "CUSTOM_SCHEMA".to_string());

        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            schema_mapping: SchemaMappingStrategy::Lookup(map),
            ..Default::default()
        });
        let claims = create_claims(Some("tenant1"));
        let result = resolver.resolve(&claims).unwrap();
        assert_eq!(result, Some("CUSTOM_SCHEMA".to_string()));
    }

    #[test]
    fn test_resolve_lookup_fallback() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            schema_mapping: SchemaMappingStrategy::Lookup(HashMap::new()),
            ..Default::default()
        });
        let claims = create_claims(Some("unknown_tenant"));
        let result = resolver.resolve(&claims).unwrap();
        assert_eq!(result, Some("UNKNOWN_TENANT".to_string()));
    }

    #[test]
    fn test_resolve_missing_claim_with_default() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            default_schema: Some("DEFAULT".to_string()),
            ..Default::default()
        });
        let claims = create_claims(None);
        let result = resolver.resolve(&claims).unwrap();
        assert_eq!(result, Some("DEFAULT".to_string()));
    }

    #[test]
    fn test_resolve_missing_claim_without_default() {
        let resolver = TenantResolver::new(TenantConfig {
            enabled: true,
            ..Default::default()
        });
        let claims = create_claims(None);
        let result = resolver.resolve(&claims);
        assert!(matches!(result, Err(AuthError::MissingTenantClaim)));
    }

    #[test]
    fn test_create_schema_filter() {
        let resolver = TenantResolver::new(TenantConfig::default());
        let filter = resolver.create_schema_filter("TENANT1");

        match filter {
            SchemaFilter::Whitelist(schemas) => {
                assert!(schemas.contains("TENANT1"));
                assert_eq!(schemas.len(), 1);
            }
            _ => panic!("Expected Whitelist filter"),
        }
    }

    #[test]
    fn test_effective_schema_filter_admin_bypass() {
        let server_filter = SchemaFilter::AllowAll;
        let result = effective_schema_filter(&server_filter, Some("TENANT1"), true);
        assert!(matches!(result, SchemaFilter::AllowAll));
    }

    #[test]
    fn test_effective_schema_filter_tenant_isolation() {
        let server_filter = SchemaFilter::AllowAll;
        let result = effective_schema_filter(&server_filter, Some("TENANT1"), false);

        match result {
            SchemaFilter::Whitelist(schemas) => {
                assert!(schemas.contains("TENANT1"));
                assert_eq!(schemas.len(), 1);
            }
            _ => panic!("Expected Whitelist filter"),
        }
    }

    #[test]
    fn test_effective_schema_filter_no_tenant() {
        let server_filter = SchemaFilter::AllowAll;
        let result = effective_schema_filter(&server_filter, None, false);
        assert!(matches!(result, SchemaFilter::AllowAll));
    }
}
