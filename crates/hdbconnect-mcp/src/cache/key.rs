//! Cache key types and factory methods
//!
//! # Multi-Tenant Cache Safety (Phase 3.5)
//!
//! Cache keys now require `user_id` for query results to ensure multi-tenant
//! isolation. This is a **breaking change** from Phase 3.4 where `user_id`
//! was optional.

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Cache key namespace for tool categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheNamespace {
    /// Table metadata
    TableSchema,
    /// Table list cache
    TableList,
    /// Procedure metadata
    ProcedureSchema,
    /// Procedure list cache
    ProcedureList,
    /// Query results
    QueryResult,
    /// Custom namespace for extensions
    Custom,
}

impl CacheNamespace {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TableSchema => "tbl_schema",
            Self::TableList => "tbl_list",
            Self::ProcedureSchema => "proc_schema",
            Self::ProcedureList => "proc_list",
            Self::QueryResult => "query",
            Self::Custom => "custom",
        }
    }
}

/// Structured cache key with namespace isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    namespace: CacheNamespace,
    schema: Option<String>,
    identifier: String,
    variant: Option<String>,
    user_id: Option<String>,
}

impl CacheKey {
    /// Create key for table schema cache
    #[must_use]
    pub fn table_schema(schema: Option<&str>, table: &str) -> Self {
        Self {
            namespace: CacheNamespace::TableSchema,
            schema: schema.map(str::to_uppercase),
            identifier: table.to_uppercase(),
            variant: None,
            user_id: None,
        }
    }

    /// Create key for table list cache
    #[must_use]
    pub fn table_list(schema: Option<&str>) -> Self {
        Self {
            namespace: CacheNamespace::TableList,
            schema: schema.map(str::to_uppercase),
            identifier: "_all".to_string(),
            variant: None,
            user_id: None,
        }
    }

    /// Create key for procedure schema cache
    #[must_use]
    pub fn procedure_schema(schema: Option<&str>, procedure: &str) -> Self {
        Self {
            namespace: CacheNamespace::ProcedureSchema,
            schema: schema.map(str::to_uppercase),
            identifier: procedure.to_uppercase(),
            variant: None,
            user_id: None,
        }
    }

    /// Create key for procedure list cache
    #[must_use]
    pub fn procedure_list(schema: Option<&str>, pattern: Option<&str>) -> Self {
        Self {
            namespace: CacheNamespace::ProcedureList,
            schema: schema.map(str::to_uppercase),
            identifier: "_all".to_string(),
            variant: pattern.map(str::to_uppercase),
            user_id: None,
        }
    }

    /// Create key for query result cache with required user context.
    ///
    /// Uses SQL hash + length as discriminator to reduce collision probability.
    /// The 64-bit hash combined with SQL length provides sufficient uniqueness
    /// for typical workloads while keeping key size small.
    ///
    /// # Multi-Tenant Safety (Phase 3.5 Breaking Change)
    ///
    /// `user_id` is now **required** to ensure cache isolation between users.
    /// This prevents cross-user data leakage in multi-tenant deployments.
    ///
    /// For single-user deployments, use a constant user identifier:
    /// ```ignore
    /// CacheKey::query_result(sql, limit, "_system")
    /// ```
    #[must_use]
    pub fn query_result(sql: &str, limit: Option<u32>, user_id: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        user_id.hash(&mut hasher);
        let hash = hasher.finish();
        let sql_len = sql.len();

        Self {
            namespace: CacheNamespace::QueryResult,
            schema: None,
            identifier: format!("{hash:016x}:{sql_len}"),
            variant: limit.map(|l| l.to_string()),
            user_id: Some(user_id.to_string()),
        }
    }

    /// Create a custom cache key
    #[must_use]
    pub fn custom(identifier: &str, variant: Option<&str>) -> Self {
        Self {
            namespace: CacheNamespace::Custom,
            schema: None,
            identifier: identifier.to_string(),
            variant: variant.map(ToString::to_string),
            user_id: None,
        }
    }

    /// Add user context to cache key for multi-tenant isolation.
    ///
    /// This method allows adding `user_id` to any cache key type when
    /// authentication is enabled, ensuring cache isolation per user.
    #[must_use]
    pub fn with_user(mut self, user_id: Option<&str>) -> Self {
        self.user_id = user_id.map(String::from);
        self
    }

    /// Get the namespace of this key
    #[must_use]
    pub const fn namespace(&self) -> CacheNamespace {
        self.namespace
    }

    /// Get namespace prefix for bulk operations
    #[must_use]
    pub fn namespace_prefix(&self) -> String {
        self.schema.as_ref().map_or_else(
            || self.namespace.as_str().to_string(),
            |s| format!("{}:{}", self.namespace.as_str(), s),
        )
    }

    /// Convert to string key for storage
    #[must_use]
    pub fn to_key_string(&self) -> String {
        let mut parts = vec![self.namespace.as_str().to_string()];

        if let Some(ref schema) = self.schema {
            parts.push(schema.clone());
        }

        parts.push(self.identifier.clone());

        if let Some(ref variant) = self.variant {
            parts.push(variant.clone());
        }

        if let Some(ref user) = self.user_id {
            parts.push(format!("u:{user}"));
        }

        parts.join(":")
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_key_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_schema_key() {
        let key = CacheKey::table_schema(Some("myschema"), "users");
        assert_eq!(key.namespace(), CacheNamespace::TableSchema);
        assert_eq!(key.to_key_string(), "tbl_schema:MYSCHEMA:USERS");
    }

    #[test]
    fn test_table_schema_key_no_schema() {
        let key = CacheKey::table_schema(None, "users");
        assert_eq!(key.to_key_string(), "tbl_schema:USERS");
    }

    #[test]
    fn test_table_list_key() {
        let key = CacheKey::table_list(Some("myschema"));
        assert_eq!(key.to_key_string(), "tbl_list:MYSCHEMA:_all");
    }

    #[test]
    fn test_table_list_key_no_schema() {
        let key = CacheKey::table_list(None);
        assert_eq!(key.to_key_string(), "tbl_list:_all");
    }

    #[test]
    fn test_procedure_schema_key() {
        let key = CacheKey::procedure_schema(Some("app"), "my_proc");
        assert_eq!(key.to_key_string(), "proc_schema:APP:MY_PROC");
    }

    #[test]
    fn test_procedure_list_key() {
        let key = CacheKey::procedure_list(Some("app"), None);
        assert_eq!(key.to_key_string(), "proc_list:APP:_all");
    }

    #[test]
    fn test_procedure_list_key_with_pattern() {
        let key = CacheKey::procedure_list(Some("app"), Some("get%"));
        assert_eq!(key.to_key_string(), "proc_list:APP:_all:GET%");
    }

    #[test]
    fn test_query_result_key_with_user() {
        let sql = "SELECT * FROM users";
        let key = CacheKey::query_result(sql, Some(100), "user_a");
        let key_str = key.to_key_string();

        assert!(key_str.starts_with("query:"));
        assert!(key_str.contains("u:user_a"));
        assert!(key_str.contains(":100:"));
    }

    #[test]
    fn test_query_result_key_no_limit() {
        let sql = "SELECT * FROM users";
        let key = CacheKey::query_result(sql, None, "user123");
        let key_str = key.to_key_string();

        assert!(key_str.starts_with("query:"));
        assert!(key_str.contains("u:user123"));
    }

    #[test]
    fn test_query_result_deterministic() {
        let key1 = CacheKey::query_result("SELECT * FROM users", Some(100), "user_a");
        let key2 = CacheKey::query_result("SELECT * FROM users", Some(100), "user_a");
        assert_eq!(key1.to_key_string(), key2.to_key_string());
    }

    #[test]
    fn test_query_result_different_sql() {
        let key1 = CacheKey::query_result("SELECT * FROM users", None, "user_a");
        let key2 = CacheKey::query_result("SELECT * FROM orders", None, "user_a");
        assert_ne!(key1.to_key_string(), key2.to_key_string());
    }

    #[test]
    fn test_query_result_same_hash_different_length() {
        let key1 = CacheKey::query_result("abc", None, "user");
        let key2 = CacheKey::query_result("abcdef", None, "user");
        assert_ne!(key1.to_key_string(), key2.to_key_string());
    }

    #[test]
    fn test_query_result_different_users() {
        let sql = "SELECT * FROM users";
        let key1 = CacheKey::query_result(sql, None, "user_a");
        let key2 = CacheKey::query_result(sql, None, "user_b");

        assert_ne!(key1.to_key_string(), key2.to_key_string());
        assert!(key1.to_key_string().contains("u:user_a"));
        assert!(key2.to_key_string().contains("u:user_b"));
    }

    #[test]
    fn test_query_result_user_affects_hash() {
        let sql = "SELECT * FROM users";
        let key1 = CacheKey::query_result(sql, None, "user_a");
        let key2 = CacheKey::query_result(sql, None, "user_b");

        let str1 = key1.to_key_string();
        let str2 = key2.to_key_string();

        let hash1 = str1.split(':').nth(1).unwrap();
        let hash2 = str2.split(':').nth(1).unwrap();

        assert_ne!(hash1, hash2, "Hash should differ with different user_id");
    }

    #[test]
    fn test_with_user_method() {
        let key = CacheKey::table_schema(Some("test"), "users").with_user(Some("user123"));
        assert!(key.to_key_string().contains("u:user123"));
    }

    #[test]
    fn test_with_user_none() {
        let key = CacheKey::table_schema(Some("test"), "users").with_user(None);
        assert!(!key.to_key_string().contains("u:"));
    }

    #[test]
    fn test_custom_key() {
        let key = CacheKey::custom("my-data", Some("v1"));
        assert_eq!(key.to_key_string(), "custom:my-data:v1");
    }

    #[test]
    fn test_namespace_prefix_with_schema() {
        let key = CacheKey::table_schema(Some("myschema"), "users");
        assert_eq!(key.namespace_prefix(), "tbl_schema:MYSCHEMA");
    }

    #[test]
    fn test_namespace_prefix_without_schema() {
        let key = CacheKey::table_list(None);
        assert_eq!(key.namespace_prefix(), "tbl_list");
    }

    #[test]
    fn test_display_impl() {
        let key = CacheKey::table_schema(Some("test"), "users");
        assert_eq!(format!("{key}"), "tbl_schema:TEST:USERS");
    }

    #[test]
    fn test_key_equality() {
        let key1 = CacheKey::table_schema(Some("test"), "users");
        let key2 = CacheKey::table_schema(Some("TEST"), "USERS");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_inequality_different_namespace() {
        let key1 = CacheKey::table_schema(Some("test"), "users");
        let key2 = CacheKey::procedure_schema(Some("test"), "users");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_inequality_different_user() {
        let key1 = CacheKey::table_schema(Some("test"), "users").with_user(Some("user_a"));
        let key2 = CacheKey::table_schema(Some("test"), "users").with_user(Some("user_b"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_namespace_as_str() {
        assert_eq!(CacheNamespace::TableSchema.as_str(), "tbl_schema");
        assert_eq!(CacheNamespace::TableList.as_str(), "tbl_list");
        assert_eq!(CacheNamespace::ProcedureSchema.as_str(), "proc_schema");
        assert_eq!(CacheNamespace::ProcedureList.as_str(), "proc_list");
        assert_eq!(CacheNamespace::QueryResult.as_str(), "query");
        assert_eq!(CacheNamespace::Custom.as_str(), "custom");
    }

    #[test]
    fn test_system_user_for_single_tenant() {
        let key = CacheKey::query_result("SELECT 1", None, "_system");
        assert!(key.to_key_string().contains("u:_system"));
    }
}
