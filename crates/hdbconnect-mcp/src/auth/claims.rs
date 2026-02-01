//! JWT claims types

use serde::Deserialize;

/// Audience can be a single string or array of strings
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    pub fn contains(&self, value: &str) -> bool {
        match self {
            Self::One(s) => s == value,
            Self::Many(v) => v.iter().any(|s| s == value),
        }
    }

    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::One(s) => vec![s.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

/// Standard JWT claims we validate
#[derive(Debug, Clone, Deserialize)]
pub struct StandardClaims {
    pub sub: String,
    pub iss: String,
    #[serde(default)]
    pub aud: Option<OneOrMany>,
    pub exp: i64,
    #[serde(default)]
    pub nbf: Option<i64>,
    #[serde(default)]
    pub iat: Option<i64>,
    #[serde(default)]
    pub jti: Option<String>,
}

/// Custom claims we extract
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CustomClaims {
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Complete JWT payload
#[derive(Debug, Clone, Deserialize)]
pub struct JwtClaims {
    #[serde(flatten)]
    pub standard: StandardClaims,
    #[serde(flatten)]
    pub custom: CustomClaims,
}

/// Authenticated user context
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// User subject (from JWT sub claim)
    pub sub: String,
    /// User email (optional)
    pub email: Option<String>,
    /// User display name (optional)
    pub name: Option<String>,
    /// Tenant ID (if multi-tenancy enabled)
    pub tenant_id: Option<String>,
    /// Resolved tenant schema
    pub tenant_schema: Option<String>,
    /// User roles (if RBAC enabled)
    pub roles: Vec<String>,
}

impl AuthenticatedUser {
    pub fn from_claims(claims: &JwtClaims, tenant_schema: Option<String>) -> Self {
        Self {
            sub: claims.standard.sub.clone(),
            email: claims.custom.email.clone(),
            name: claims.custom.name.clone(),
            tenant_id: claims.custom.tenant_id.clone(),
            tenant_schema,
            roles: claims.custom.roles.clone(),
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_or_many_one() {
        let aud = OneOrMany::One("api".to_string());
        assert!(aud.contains("api"));
        assert!(!aud.contains("other"));
        assert_eq!(aud.to_vec(), vec!["api"]);
    }

    #[test]
    fn test_one_or_many_many() {
        let aud = OneOrMany::Many(vec!["api".to_string(), "web".to_string()]);
        assert!(aud.contains("api"));
        assert!(aud.contains("web"));
        assert!(!aud.contains("other"));
        assert_eq!(aud.to_vec(), vec!["api", "web"]);
    }

    #[test]
    fn test_deserialize_claims_single_audience() {
        let json = r#"{
            "sub": "user123",
            "iss": "https://auth.example.com",
            "aud": "api",
            "exp": 1700000000
        }"#;
        let claims: JwtClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.standard.sub, "user123");
        assert!(claims.standard.aud.as_ref().unwrap().contains("api"));
    }

    #[test]
    fn test_deserialize_claims_multiple_audience() {
        let json = r#"{
            "sub": "user123",
            "iss": "https://auth.example.com",
            "aud": ["api", "web"],
            "exp": 1700000000
        }"#;
        let claims: JwtClaims = serde_json::from_str(json).unwrap();
        assert!(claims.standard.aud.as_ref().unwrap().contains("api"));
        assert!(claims.standard.aud.as_ref().unwrap().contains("web"));
    }

    #[test]
    fn test_deserialize_custom_claims() {
        let json = r#"{
            "sub": "user123",
            "iss": "https://auth.example.com",
            "exp": 1700000000,
            "tenant_id": "tenant1",
            "roles": ["admin", "user"],
            "email": "user@example.com",
            "name": "Test User"
        }"#;
        let claims: JwtClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.custom.tenant_id, Some("tenant1".to_string()));
        assert_eq!(claims.custom.roles, vec!["admin", "user"]);
        assert_eq!(claims.custom.email, Some("user@example.com".to_string()));
    }

    #[test]
    fn test_deserialize_minimal_claims() {
        let json = r#"{
            "sub": "user123",
            "iss": "https://auth.example.com",
            "exp": 1700000000
        }"#;
        let claims: JwtClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.standard.sub, "user123");
        assert!(claims.custom.tenant_id.is_none());
        assert!(claims.custom.roles.is_empty());
    }

    #[test]
    fn test_authenticated_user_from_claims() {
        let json = r#"{
            "sub": "user123",
            "iss": "https://auth.example.com",
            "exp": 1700000000,
            "tenant_id": "tenant1",
            "roles": ["admin"],
            "email": "user@example.com"
        }"#;
        let claims: JwtClaims = serde_json::from_str(json).unwrap();
        let user = AuthenticatedUser::from_claims(&claims, Some("TENANT1".to_string()));

        assert_eq!(user.sub, "user123");
        assert_eq!(user.tenant_id, Some("tenant1".to_string()));
        assert_eq!(user.tenant_schema, Some("TENANT1".to_string()));
        assert!(user.has_role("admin"));
        assert!(!user.has_role("user"));
    }
}
