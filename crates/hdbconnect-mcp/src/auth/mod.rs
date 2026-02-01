//! Authentication and authorization module
//!
//! Provides JWT/OIDC authentication, multi-tenancy support, and RBAC.
//!
//! # Features
//!
//! - JWT validation with RS256/ES256/HS256 support via `jsonwebtoken`
//! - OIDC discovery support via `openidconnect` crate
//! - JWKS fetching with automatic key rotation
//! - Multi-tenant schema isolation via JWT claims
//! - Optional role-based access control
//!
//! # OIDC Support
//!
//! For OIDC-based authentication, use the `oidc` module which provides:
//! - `OidcConfig` - Configuration using `openidconnect` types directly
//! - `OidcClient` - Client with automatic OIDC discovery
//! - `TenantClaims` - Custom claims implementing `AdditionalClaims`
//! - `IdTokenClaims` - Type alias for `CoreIdTokenClaims<TenantClaims>`
//!
//! # Multi-User Cache Safety
//!
//! When authentication is enabled, user context MUST be included in cache keys
//! to prevent cross-user data leakage. The `CacheKey::query_result()` method
//! requires `user_id` parameter for multi-tenant deployments (breaking change
//! from Phase 3.4).

mod claims;
mod config;
mod error;
mod jwks;
mod jwt;
#[cfg(feature = "http")]
mod middleware;
mod oidc;
mod rbac;
mod tenant;

pub use claims::{AuthenticatedUser, CustomClaims, JwtClaims, OneOrMany, StandardClaims};
pub use config::{
    AuthConfig, AuthMode, JwtConfig, RbacConfig, SchemaMappingStrategy, TenantConfig,
};
pub use error::{AuthError, Result};
pub use jwks::{JwkSet, JwksCache, JwksRefreshTask};
pub use jwt::JwtValidator;
#[cfg(feature = "http")]
pub use middleware::{AuthState, jwt_auth_middleware};
pub use oidc::{CachedOidcClient, IdTokenClaims, OidcClient, OidcConfig, TenantClaims};
pub use rbac::{Permission, RbacEnforcer};
pub use tenant::{TenantResolver, effective_schema_filter};
