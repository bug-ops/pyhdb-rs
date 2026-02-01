//! DML operations configuration

use std::num::NonZeroU32;
use std::str::FromStr;

/// DML operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmlOperation {
    Insert,
    Update,
    Delete,
}

impl DmlOperation {
    /// Parse DML operation from SQL statement
    #[must_use]
    pub fn from_sql(sql: &str) -> Option<Self> {
        let trimmed = sql.trim_start().to_uppercase();
        if trimmed.starts_with("INSERT") {
            Some(Self::Insert)
        } else if trimmed.starts_with("UPDATE") {
            Some(Self::Update)
        } else if trimmed.starts_with("DELETE") {
            Some(Self::Delete)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn requires_where_clause(&self) -> bool {
        matches!(self, Self::Update | Self::Delete)
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Insert => "INSERT",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
        }
    }
}

impl std::fmt::Display for DmlOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Allowed DML operation types
#[derive(Debug, Clone, Copy)]
pub struct AllowedOperations {
    pub insert: bool,
    pub update: bool,
    pub delete: bool,
}

impl AllowedOperations {
    #[must_use]
    pub const fn all() -> Self {
        Self {
            insert: true,
            update: true,
            delete: true,
        }
    }

    #[must_use]
    pub const fn none() -> Self {
        Self {
            insert: false,
            update: false,
            delete: false,
        }
    }

    #[must_use]
    pub const fn is_allowed(&self, op: DmlOperation) -> bool {
        match op {
            DmlOperation::Insert => self.insert,
            DmlOperation::Update => self.update,
            DmlOperation::Delete => self.delete,
        }
    }
}

impl FromStr for AllowedOperations {
    type Err = std::convert::Infallible;

    /// Parse from comma-separated string (e.g., "insert,update")
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_upper = s.to_uppercase();
        Ok(Self {
            insert: s_upper.contains("INSERT"),
            update: s_upper.contains("UPDATE"),
            delete: s_upper.contains("DELETE"),
        })
    }
}

impl Default for AllowedOperations {
    fn default() -> Self {
        Self::all()
    }
}

/// Default maximum affected rows limit for DML safety (1000)
const DEFAULT_MAX_AFFECTED_ROWS: NonZeroU32 = NonZeroU32::new(1000).unwrap();

/// DML operations configuration
#[derive(Debug, Clone, Copy)]
pub struct DmlConfig {
    /// Allow DML operations (INSERT, UPDATE, DELETE). Default: false
    pub allow_dml: bool,
    /// Require user confirmation before executing DML. Default: true
    pub require_confirmation: bool,
    /// Maximum affected rows allowed (None = unlimited). Default: 1000
    pub max_affected_rows: Option<NonZeroU32>,
    /// Require WHERE clause for UPDATE/DELETE. Default: true
    pub require_where_clause: bool,
    /// Allowed DML operations. Default: all
    pub allowed_operations: AllowedOperations,
}

impl Default for DmlConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DmlConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            allow_dml: false,
            require_confirmation: true,
            max_affected_rows: Some(DEFAULT_MAX_AFFECTED_ROWS),
            require_where_clause: true,
            allowed_operations: AllowedOperations::all(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dml_operation_from_sql_insert() {
        assert_eq!(
            DmlOperation::from_sql("INSERT INTO t VALUES (1)"),
            Some(DmlOperation::Insert)
        );
        assert_eq!(
            DmlOperation::from_sql("  INSERT INTO t VALUES (1)"),
            Some(DmlOperation::Insert)
        );
        assert_eq!(
            DmlOperation::from_sql("insert into t values (1)"),
            Some(DmlOperation::Insert)
        );
    }

    #[test]
    fn test_dml_operation_from_sql_update() {
        assert_eq!(
            DmlOperation::from_sql("UPDATE t SET x = 1"),
            Some(DmlOperation::Update)
        );
        assert_eq!(
            DmlOperation::from_sql("  update t set x = 1"),
            Some(DmlOperation::Update)
        );
    }

    #[test]
    fn test_dml_operation_from_sql_delete() {
        assert_eq!(
            DmlOperation::from_sql("DELETE FROM t WHERE id = 1"),
            Some(DmlOperation::Delete)
        );
        assert_eq!(
            DmlOperation::from_sql("  delete from t"),
            Some(DmlOperation::Delete)
        );
    }

    #[test]
    fn test_dml_operation_from_sql_not_dml() {
        assert_eq!(DmlOperation::from_sql("SELECT * FROM t"), None);
        assert_eq!(DmlOperation::from_sql("CREATE TABLE t (id INT)"), None);
        assert_eq!(DmlOperation::from_sql("DROP TABLE t"), None);
        assert_eq!(DmlOperation::from_sql(""), None);
    }

    #[test]
    fn test_dml_operation_requires_where_clause() {
        assert!(!DmlOperation::Insert.requires_where_clause());
        assert!(DmlOperation::Update.requires_where_clause());
        assert!(DmlOperation::Delete.requires_where_clause());
    }

    #[test]
    fn test_dml_operation_display() {
        assert_eq!(DmlOperation::Insert.to_string(), "INSERT");
        assert_eq!(DmlOperation::Update.to_string(), "UPDATE");
        assert_eq!(DmlOperation::Delete.to_string(), "DELETE");
    }

    #[test]
    fn test_allowed_operations_all() {
        let ops = AllowedOperations::all();
        assert!(ops.is_allowed(DmlOperation::Insert));
        assert!(ops.is_allowed(DmlOperation::Update));
        assert!(ops.is_allowed(DmlOperation::Delete));
    }

    #[test]
    fn test_allowed_operations_none() {
        let ops = AllowedOperations::none();
        assert!(!ops.is_allowed(DmlOperation::Insert));
        assert!(!ops.is_allowed(DmlOperation::Update));
        assert!(!ops.is_allowed(DmlOperation::Delete));
    }

    #[test]
    fn test_allowed_operations_from_str() {
        let ops: AllowedOperations = "insert,update".parse().unwrap();
        assert!(ops.insert);
        assert!(ops.update);
        assert!(!ops.delete);

        let ops2: AllowedOperations = "DELETE".parse().unwrap();
        assert!(!ops2.insert);
        assert!(!ops2.update);
        assert!(ops2.delete);

        let ops3: AllowedOperations = "INSERT, UPDATE, DELETE".parse().unwrap();
        assert!(ops3.insert);
        assert!(ops3.update);
        assert!(ops3.delete);
    }

    #[test]
    fn test_dml_config_default() {
        let config = DmlConfig::default();
        assert!(!config.allow_dml);
        assert!(config.require_confirmation);
        assert_eq!(config.max_affected_rows, NonZeroU32::new(1000));
        assert!(config.require_where_clause);
        assert!(config.allowed_operations.insert);
        assert!(config.allowed_operations.update);
        assert!(config.allowed_operations.delete);
    }

    #[test]
    fn test_dml_config_new() {
        let config = DmlConfig::new();
        assert!(!config.allow_dml);
        assert!(config.require_confirmation);
        assert_eq!(config.max_affected_rows, NonZeroU32::new(1000));
        assert!(config.require_where_clause);
        assert!(config.allowed_operations.insert);
        assert!(config.allowed_operations.update);
        assert!(config.allowed_operations.delete);
    }

    #[test]
    fn test_dml_config_new_and_default_are_identical() {
        let from_new = DmlConfig::new();
        let from_default = DmlConfig::default();

        assert_eq!(from_new.allow_dml, from_default.allow_dml);
        assert_eq!(
            from_new.require_confirmation,
            from_default.require_confirmation
        );
        assert_eq!(from_new.max_affected_rows, from_default.max_affected_rows);
        assert_eq!(
            from_new.require_where_clause,
            from_default.require_where_clause
        );
        assert_eq!(
            from_new.allowed_operations.insert,
            from_default.allowed_operations.insert
        );
        assert_eq!(
            from_new.allowed_operations.update,
            from_default.allowed_operations.update
        );
        assert_eq!(
            from_new.allowed_operations.delete,
            from_default.allowed_operations.delete
        );
    }
}
