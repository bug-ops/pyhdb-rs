//! User context extraction from MCP `RequestContext`
//!
//! Provides helper functions to extract authenticated user information
//! from rmcp `RequestContext` extensions for per-user cache isolation.

use rmcp::service::{RequestContext, ServiceRole};

#[cfg(feature = "http")]
use super::AuthenticatedUser;
#[cfg(feature = "cache")]
use crate::constants::CACHE_SYSTEM_USER;

/// Extract user identifier from `RequestContext` for cache key generation.
///
/// The extraction follows the rmcp 0.14 extension propagation chain:
/// 1. HTTP middleware inserts `AuthenticatedUser` into request extensions
/// 2. rmcp injects `http::request::Parts` into MCP extensions
/// 3. This function extracts Parts, then `AuthenticatedUser` from nested extensions
///
/// Returns `CACHE_SYSTEM_USER` when:
/// - No HTTP Parts in extensions (stdio transport)
/// - No `AuthenticatedUser` in Parts (auth disabled)
#[cfg(feature = "cache")]
pub fn extract_user_id<R: ServiceRole>(context: &RequestContext<R>) -> &str {
    #[cfg(feature = "http")]
    {
        context
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<AuthenticatedUser>())
            .map_or(CACHE_SYSTEM_USER, |user| user.sub.as_str())
    }

    #[cfg(not(feature = "http"))]
    {
        let _ = context;
        CACHE_SYSTEM_USER
    }
}

/// Extract user ID from extensions (testable helper).
///
/// This helper duplicates the extraction logic for use in unit tests where
/// `RequestContext` cannot be easily constructed.
#[cfg(all(feature = "cache", feature = "http", test))]
fn extract_user_id_from_extensions(extensions: &rmcp::model::Extensions) -> &str {
    extensions
        .get::<axum::http::request::Parts>()
        .and_then(|parts| parts.extensions.get::<AuthenticatedUser>())
        .map_or(CACHE_SYSTEM_USER, |user| user.sub.as_str())
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(all(feature = "http", feature = "cache"))]
    mod http_tests {
        use axum::body::Body;
        use axum::http::Request;
        use rmcp::model::Extensions;

        use super::*;

        fn create_http_parts_with_user(
            user: Option<AuthenticatedUser>,
        ) -> axum::http::request::Parts {
            let mut request = Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())
                .unwrap();

            if let Some(u) = user {
                request.extensions_mut().insert(u);
            }

            request.into_parts().0
        }

        fn create_extensions_with_user(user: Option<AuthenticatedUser>) -> Extensions {
            let parts = create_http_parts_with_user(user);
            let mut extensions = Extensions::new();
            extensions.insert(parts);
            extensions
        }

        #[test]
        fn test_extract_user_id_with_authenticated_user() {
            let user = AuthenticatedUser {
                sub: "user_a".to_string(),
                email: Some("user_a@example.com".to_string()),
                name: Some("User A".to_string()),
                tenant_id: None,
                tenant_schema: None,
                roles: vec![],
            };

            let extensions = create_extensions_with_user(Some(user));
            let extracted = extract_user_id_from_extensions(&extensions);

            assert_eq!(extracted, "user_a");
        }

        #[test]
        fn test_extract_user_id_without_authenticated_user() {
            let extensions = create_extensions_with_user(None);
            let extracted = extract_user_id_from_extensions(&extensions);

            assert_eq!(extracted, CACHE_SYSTEM_USER);
        }

        #[test]
        fn test_extract_user_id_without_parts() {
            let extensions = Extensions::new();
            let extracted = extract_user_id_from_extensions(&extensions);

            assert_eq!(extracted, CACHE_SYSTEM_USER);
        }

        #[test]
        fn test_extract_user_id_empty_sub() {
            let user = AuthenticatedUser {
                sub: String::new(),
                email: None,
                name: None,
                tenant_id: None,
                tenant_schema: None,
                roles: vec![],
            };

            let extensions = create_extensions_with_user(Some(user));
            let extracted = extract_user_id_from_extensions(&extensions);

            assert_eq!(extracted, "");
        }

        #[test]
        fn test_extract_user_id_with_special_characters() {
            let user = AuthenticatedUser {
                sub: "user@domain.com:tenant1".to_string(),
                email: Some("user@domain.com".to_string()),
                name: Some("Test User".to_string()),
                tenant_id: Some("tenant1".to_string()),
                tenant_schema: None,
                roles: vec![],
            };

            let extensions = create_extensions_with_user(Some(user));
            let extracted = extract_user_id_from_extensions(&extensions);

            assert_eq!(extracted, "user@domain.com:tenant1");
        }

        #[test]
        fn test_extract_user_id_with_unicode() {
            let user = AuthenticatedUser {
                sub: "user_\u{65E5}\u{672C}\u{8A9E}".to_string(),
                email: None,
                name: None,
                tenant_id: None,
                tenant_schema: None,
                roles: vec![],
            };

            let extensions = create_extensions_with_user(Some(user));
            let extracted = extract_user_id_from_extensions(&extensions);

            assert_eq!(extracted, "user_\u{65E5}\u{672C}\u{8A9E}");
        }

        #[test]
        fn test_extract_user_id_different_users_produce_different_results() {
            let user_a = AuthenticatedUser {
                sub: "user_a".to_string(),
                email: None,
                name: None,
                tenant_id: None,
                tenant_schema: None,
                roles: vec![],
            };

            let user_b = AuthenticatedUser {
                sub: "user_b".to_string(),
                email: None,
                name: None,
                tenant_id: None,
                tenant_schema: None,
                roles: vec![],
            };

            let ext_a = create_extensions_with_user(Some(user_a));
            let ext_b = create_extensions_with_user(Some(user_b));

            let id_a = extract_user_id_from_extensions(&ext_a);
            let id_b = extract_user_id_from_extensions(&ext_b);

            assert_ne!(id_a, id_b);
        }
    }

    #[cfg(all(not(feature = "http"), feature = "cache"))]
    mod non_http_tests {
        use rmcp::model::Extensions;

        use super::*;

        #[test]
        fn test_fallback_returns_system_user() {
            let extensions = Extensions::new();

            let extracted = extensions
                .get::<()>()
                .map_or(CACHE_SYSTEM_USER, |_| "never");

            assert_eq!(extracted, CACHE_SYSTEM_USER);
        }
    }
}
