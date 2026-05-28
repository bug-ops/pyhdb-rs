//! Cursor typestate definitions.
//!
//! Implements compile-time state tracking for cursors:
//! - Idle: No active result set
//! - Executed: Query executed, results available
//! - Fetching: Currently fetching rows
//! - Exhausted: All rows consumed

use std::marker::PhantomData;

use crate::private::sealed::Sealed;

/// Marker trait for cursor states.
///
/// Sealed to prevent external implementations.
pub trait CursorState: Sealed + Send + Sync + 'static {
    /// Human-readable state name for debugging.
    const STATE_NAME: &'static str;

    /// Whether fetch operations are allowed.
    const CAN_FETCH: bool;

    /// Whether the cursor has an active result set.
    const HAS_RESULT: bool;
}

/// Idle state - no active result set.
#[derive(Debug, Clone, Copy, Default)]
pub struct Idle;

impl Sealed for Idle {}

impl CursorState for Idle {
    const STATE_NAME: &'static str = "Idle";
    const CAN_FETCH: bool = false;
    const HAS_RESULT: bool = false;
}

/// Executed state - query executed, results available.
#[derive(Debug, Clone, Copy, Default)]
pub struct Executed;

impl Sealed for Executed {}

impl CursorState for Executed {
    const STATE_NAME: &'static str = "Executed";
    const CAN_FETCH: bool = true;
    const HAS_RESULT: bool = true;
}

/// Fetching state - currently fetching rows.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fetching;

impl Sealed for Fetching {}

impl CursorState for Fetching {
    const STATE_NAME: &'static str = "Fetching";
    const CAN_FETCH: bool = true;
    const HAS_RESULT: bool = true;
}

/// Exhausted state - all rows consumed.
#[derive(Debug, Clone, Copy, Default)]
pub struct Exhausted;

impl Sealed for Exhausted {}

impl CursorState for Exhausted {
    const STATE_NAME: &'static str = "Exhausted";
    const CAN_FETCH: bool = false;
    const HAS_RESULT: bool = false;
}

/// Column description from result set metadata.
#[derive(Debug, Clone)]
pub struct ColumnDescription {
    /// Column name.
    pub name: String,
    /// HANA type code.
    pub type_code: i16,
    /// Display size (optional).
    pub display_size: Option<usize>,
    /// Internal size (optional).
    pub internal_size: Option<usize>,
    /// Precision (for decimals).
    pub precision: Option<i16>,
    /// Scale (for decimals).
    pub scale: Option<i16>,
    /// Whether nullable.
    pub nullable: bool,
}

/// Typed cursor with compile-time state tracking.
#[derive(Debug)]
pub struct TypedCursor<S: CursorState> {
    /// Column descriptions.
    description: Option<Vec<ColumnDescription>>,
    /// Number of rows affected by last DML.
    rowcount: i64,
    /// Phantom marker for state.
    _state: PhantomData<S>,
}

impl<S: CursorState> TypedCursor<S> {
    /// Get the current state name.
    #[must_use]
    pub const fn state_name(&self) -> &'static str {
        S::STATE_NAME
    }

    /// Check if fetch operations are allowed.
    #[must_use]
    pub const fn can_fetch(&self) -> bool {
        S::CAN_FETCH
    }

    /// Check if cursor has an active result set.
    #[must_use]
    pub const fn has_result(&self) -> bool {
        S::HAS_RESULT
    }

    /// Get the row count.
    #[must_use]
    pub const fn rowcount(&self) -> i64 {
        self.rowcount
    }

    /// Get column descriptions.
    #[must_use]
    pub fn description(&self) -> Option<&[ColumnDescription]> {
        self.description.as_deref()
    }
}

impl Default for TypedCursor<Idle> {
    fn default() -> Self {
        Self::new()
    }
}

impl TypedCursor<Idle> {
    /// Create a new idle cursor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            description: None,
            rowcount: -1,
            _state: PhantomData,
        }
    }

    /// Execute a DML statement.
    ///
    /// Stays in Idle state with updated rowcount.
    #[must_use]
    pub fn execute_dml(mut self, affected_rows: usize) -> Self {
        self.rowcount = affected_rows as i64;
        self.description = None;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_properties() {
        assert!(!Idle::CAN_FETCH);
        assert!(!Idle::HAS_RESULT);

        assert!(Executed::CAN_FETCH);
        assert!(Executed::HAS_RESULT);

        assert!(Fetching::CAN_FETCH);
        assert!(Fetching::HAS_RESULT);

        assert!(!Exhausted::CAN_FETCH);
        assert!(!Exhausted::HAS_RESULT);
    }

    #[test]
    fn test_cursor_creation() {
        let cursor = TypedCursor::<Idle>::new();
        assert_eq!(cursor.state_name(), "Idle");
        assert_eq!(cursor.rowcount(), -1);
    }

    #[test]
    fn test_cursor_default() {
        let cursor = TypedCursor::<Idle>::default();
        assert_eq!(cursor.state_name(), "Idle");
        assert_eq!(cursor.rowcount(), -1);
        assert!(!cursor.can_fetch());
        assert!(!cursor.has_result());
        assert!(cursor.description().is_none());
    }

    #[test]
    fn test_cursor_execute_dml() {
        let cursor = TypedCursor::<Idle>::new();
        let cursor = cursor.execute_dml(42);
        assert_eq!(cursor.rowcount(), 42);
        assert!(cursor.description().is_none());
        assert_eq!(cursor.state_name(), "Idle");
    }

    #[test]
    fn test_cursor_execute_dml_zero_rows() {
        let cursor = TypedCursor::<Idle>::new();
        let cursor = cursor.execute_dml(0);
        assert_eq!(cursor.rowcount(), 0);
    }

    #[test]
    fn test_state_names() {
        assert_eq!(Idle::STATE_NAME, "Idle");
        assert_eq!(Executed::STATE_NAME, "Executed");
        assert_eq!(Fetching::STATE_NAME, "Fetching");
        assert_eq!(Exhausted::STATE_NAME, "Exhausted");
    }

    #[test]
    fn test_column_description_creation() {
        let desc = ColumnDescription {
            name: "test_col".to_string(),
            type_code: 1,
            display_size: Some(10),
            internal_size: Some(8),
            precision: Some(10),
            scale: Some(2),
            nullable: true,
        };

        assert_eq!(desc.name, "test_col");
        assert_eq!(desc.type_code, 1);
        assert_eq!(desc.display_size, Some(10));
        assert_eq!(desc.internal_size, Some(8));
        assert_eq!(desc.precision, Some(10));
        assert_eq!(desc.scale, Some(2));
        assert!(desc.nullable);
    }

    #[test]
    fn test_column_description_clone() {
        let desc = ColumnDescription {
            name: "col".to_string(),
            type_code: 5,
            display_size: None,
            internal_size: None,
            precision: None,
            scale: None,
            nullable: false,
        };

        let cloned = desc.clone();
        assert_eq!(cloned.name, desc.name);
        assert_eq!(cloned.type_code, desc.type_code);
        assert_eq!(cloned.nullable, desc.nullable);
    }

    #[test]
    fn test_state_debug_implementations() {
        assert_eq!(format!("{:?}", Idle), "Idle");
        assert_eq!(format!("{:?}", Executed), "Executed");
        assert_eq!(format!("{:?}", Fetching), "Fetching");
        assert_eq!(format!("{:?}", Exhausted), "Exhausted");
    }

    #[test]
    fn test_state_default_implementations() {
        let _idle: Idle = Default::default();
        let _executed: Executed = Default::default();
        let _fetching: Fetching = Default::default();
        let _exhausted: Exhausted = Default::default();
    }

    #[test]
    fn test_state_clone_copy() {
        let idle = Idle;
        let idle_copy = idle;
        let _idle_clone = idle_copy.clone();

        let executed = Executed;
        let executed_copy = executed;
        let _executed_clone = executed_copy.clone();
    }
}
