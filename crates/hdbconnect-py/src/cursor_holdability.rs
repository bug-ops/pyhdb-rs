//! Cursor holdability configuration for transaction behavior.
//!
//! Controls whether result set cursors remain open after COMMIT or ROLLBACK operations.
//! This is important for applications that need to maintain open cursors across transaction
//! boundaries, especially when using pooled connections.

use pyo3::prelude::*;

/// Controls result set behavior across transaction boundaries.
///
/// Determines whether cursors remain open after COMMIT or ROLLBACK operations.
/// This affects how result sets behave in transaction-heavy applications.
///
/// # Variants
///
/// - `None` - Cursor closed on both commit and rollback (default behavior)
/// - `Commit` - Cursor held across commits but closed on rollback
/// - `Rollback` - Cursor held across rollbacks but closed on commit
/// - `CommitAndRollback` - Cursor held across both commit and rollback
///
/// # Example
///
/// ```python
/// from pyhdb_rs import ConnectionBuilder, CursorHoldability
///
/// conn = (ConnectionBuilder()
///     .host("hana.example.com")
///     .credentials("SYSTEM", "password")
///     .cursor_holdability(CursorHoldability.CommitAndRollback)
///     .build())
/// ```
#[pyclass(name = "CursorHoldability", module = "pyhdb_rs._core", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PyCursorHoldability {
    /// Cursor closed on commit and rollback (default).
    #[default]
    None = 0,
    /// Cursor held across commits.
    Commit = 1,
    /// Cursor held across rollbacks.
    Rollback = 2,
    /// Cursor held across both commit and rollback.
    CommitAndRollback = 3,
}

impl From<PyCursorHoldability> for hdbconnect::CursorHoldability {
    #[allow(clippy::use_self)]
    fn from(value: PyCursorHoldability) -> Self {
        match value {
            PyCursorHoldability::None => hdbconnect::CursorHoldability::None,
            PyCursorHoldability::Commit => hdbconnect::CursorHoldability::Commit,
            PyCursorHoldability::Rollback => hdbconnect::CursorHoldability::Rollback,
            PyCursorHoldability::CommitAndRollback => {
                hdbconnect::CursorHoldability::CommitAndRollback
            }
        }
    }
}

#[pymethods]
impl PyCursorHoldability {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __repr__(&self) -> String {
        match self {
            Self::None => "CursorHoldability.None".to_string(),
            Self::Commit => "CursorHoldability.Commit".to_string(),
            Self::Rollback => "CursorHoldability.Rollback".to_string(),
            Self::CommitAndRollback => "CursorHoldability.CommitAndRollback".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_holdability_default() {
        let holdability = PyCursorHoldability::default();
        assert_eq!(holdability, PyCursorHoldability::None);
    }

    #[test]
    fn test_cursor_holdability_repr() {
        assert_eq!(
            PyCursorHoldability::None.__repr__(),
            "CursorHoldability.None"
        );
        assert_eq!(
            PyCursorHoldability::Commit.__repr__(),
            "CursorHoldability.Commit"
        );
        assert_eq!(
            PyCursorHoldability::Rollback.__repr__(),
            "CursorHoldability.Rollback"
        );
        assert_eq!(
            PyCursorHoldability::CommitAndRollback.__repr__(),
            "CursorHoldability.CommitAndRollback"
        );
    }

    #[test]
    fn test_cursor_holdability_clone() {
        let holdability = PyCursorHoldability::Commit;
        let cloned = holdability;
        assert_eq!(holdability, cloned);
    }

    #[test]
    fn test_cursor_holdability_eq() {
        assert_eq!(PyCursorHoldability::None, PyCursorHoldability::None);
        assert_ne!(PyCursorHoldability::None, PyCursorHoldability::Commit);
    }

    #[test]
    fn test_cursor_holdability_to_hdbconnect() {
        let none: hdbconnect::CursorHoldability = PyCursorHoldability::None.into();
        assert!(matches!(none, hdbconnect::CursorHoldability::None));

        let commit: hdbconnect::CursorHoldability = PyCursorHoldability::Commit.into();
        assert!(matches!(commit, hdbconnect::CursorHoldability::Commit));

        let rollback: hdbconnect::CursorHoldability = PyCursorHoldability::Rollback.into();
        assert!(matches!(rollback, hdbconnect::CursorHoldability::Rollback));

        let both: hdbconnect::CursorHoldability = PyCursorHoldability::CommitAndRollback.into();
        assert!(matches!(
            both,
            hdbconnect::CursorHoldability::CommitAndRollback
        ));
    }

    #[test]
    fn test_cursor_holdability_debug() {
        let holdability = PyCursorHoldability::Commit;
        let debug_str = format!("{:?}", holdability);
        assert!(debug_str.contains("Commit"));
    }

    #[test]
    fn test_cursor_holdability_values() {
        assert_eq!(PyCursorHoldability::None as i32, 0);
        assert_eq!(PyCursorHoldability::Commit as i32, 1);
        assert_eq!(PyCursorHoldability::Rollback as i32, 2);
        assert_eq!(PyCursorHoldability::CommitAndRollback as i32, 3);
    }
}
