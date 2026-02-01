//! Cache key types and factory methods

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
        }
    }

    /// Create key for query result cache.
    ///
    /// Uses SQL hash + length as discriminator to reduce collision probability.
    /// The 64-bit hash combined with SQL length provides sufficient uniqueness
    /// for typical workloads while keeping key size small.
    #[must_use]
    pub fn query_result(sql: &str, limit: Option<u32>) -> Self {
        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        let hash = hasher.finish();
        let sql_len = sql.len();

        Self {
            namespace: CacheNamespace::QueryResult,
            schema: None,
            identifier: format!("{hash:016x}:{sql_len}"),
            variant: limit.map(|l| l.to_string()),
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
        }
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
    fn test_query_result_key() {
        let sql = "SELECT * FROM users";
        let key = CacheKey::query_result(sql, Some(100));
        let key_str = key.to_key_string();

        // Format: query:{hash}:{sql_len}:{limit}
        assert!(key_str.starts_with("query:"));
        assert!(key_str.ends_with(":100"));

        // Verify SQL length is included
        let sql_len = sql.len();
        assert!(
            key_str.contains(&format!(":{sql_len}:")),
            "Expected key to contain :{sql_len}:, got: {key_str}"
        );
    }

    #[test]
    fn test_query_result_key_no_limit() {
        let sql = "SELECT * FROM users";
        let key = CacheKey::query_result(sql, None);
        let key_str = key.to_key_string();

        // Format: query:{hash}:{sql_len}
        assert!(key_str.starts_with("query:"));

        // SQL length should be at end (no limit variant)
        let sql_len = sql.len();
        assert!(
            key_str.ends_with(&format!(":{sql_len}")),
            "Expected key to end with :{sql_len}, got: {key_str}"
        );
    }

    #[test]
    fn test_query_result_deterministic() {
        let key1 = CacheKey::query_result("SELECT * FROM users", Some(100));
        let key2 = CacheKey::query_result("SELECT * FROM users", Some(100));
        assert_eq!(key1.to_key_string(), key2.to_key_string());
    }

    #[test]
    fn test_query_result_different_sql() {
        let key1 = CacheKey::query_result("SELECT * FROM users", None);
        let key2 = CacheKey::query_result("SELECT * FROM orders", None);
        assert_ne!(key1.to_key_string(), key2.to_key_string());
    }

    #[test]
    fn test_query_result_same_hash_different_length() {
        // Even if two strings happened to produce the same hash,
        // different lengths would create different keys
        let key1 = CacheKey::query_result("abc", None);
        let key2 = CacheKey::query_result("abcdef", None);

        // Keys differ due to SQL length being part of identifier
        assert_ne!(key1.to_key_string(), key2.to_key_string());

        // Verify both contain their respective lengths
        assert!(key1.to_key_string().ends_with(":3"));
        assert!(key2.to_key_string().ends_with(":6"));
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
    fn test_namespace_as_str() {
        assert_eq!(CacheNamespace::TableSchema.as_str(), "tbl_schema");
        assert_eq!(CacheNamespace::TableList.as_str(), "tbl_list");
        assert_eq!(CacheNamespace::ProcedureSchema.as_str(), "proc_schema");
        assert_eq!(CacheNamespace::ProcedureList.as_str(), "proc_list");
        assert_eq!(CacheNamespace::QueryResult.as_str(), "query");
        assert_eq!(CacheNamespace::Custom.as_str(), "custom");
    }
}
