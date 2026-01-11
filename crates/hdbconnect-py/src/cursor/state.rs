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
    /// The result set (if executed).
    result_set: Option<hdbconnect::ResultSet>,
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
            result_set: None,
            description: None,
            rowcount: -1,
            _state: PhantomData,
        }
    }

    /// Execute a query.
    ///
    /// Transitions from Idle to Executed state.
    pub fn execute(
        self,
        result_set: hdbconnect::ResultSet,
        description: Vec<ColumnDescription>,
    ) -> TypedCursor<Executed> {
        TypedCursor {
            result_set: Some(result_set),
            description: Some(description),
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

impl TypedCursor<Executed> {
    /// Fetch one row.
    ///
    /// Transitions to Fetching or Exhausted state.
    pub fn fetchone(mut self) -> FetchResult {
        if let Some(ref mut rs) = self.result_set {
            match rs.next() {
                Some(Ok(row)) => FetchResult::Row(
                    row,
                    TypedCursor {
                        result_set: self.result_set,
                        description: self.description,
                        rowcount: self.rowcount,
                        _state: PhantomData,
                    },
                ),
                Some(Err(e)) => FetchResult::Error(e),
                None => FetchResult::Empty(TypedCursor {
                    result_set: None,
                    description: self.description,
                    rowcount: self.rowcount,
                    _state: PhantomData,
                }),
            }
        } else {
            FetchResult::Empty(TypedCursor {
                result_set: None,
                description: self.description,
                rowcount: self.rowcount,
                _state: PhantomData,
            })
        }
    }

    /// Reset to idle state.
    pub fn reset(self) -> TypedCursor<Idle> {
        TypedCursor {
            result_set: None,
            description: None,
            rowcount: -1,
            _state: PhantomData,
        }
    }

    /// Get mutable reference to result set.
    pub const fn result_set_mut(&mut self) -> Option<&mut hdbconnect::ResultSet> {
        self.result_set.as_mut()
    }
}

impl TypedCursor<Fetching> {
    /// Fetch one more row.
    pub fn fetchone(mut self) -> FetchResult {
        if let Some(ref mut rs) = self.result_set {
            match rs.next() {
                Some(Ok(row)) => FetchResult::Row(
                    row,
                    Self {
                        result_set: self.result_set,
                        description: self.description,
                        rowcount: self.rowcount,
                        _state: PhantomData,
                    },
                ),
                Some(Err(e)) => FetchResult::Error(e),
                None => FetchResult::Empty(TypedCursor {
                    result_set: None,
                    description: self.description,
                    rowcount: self.rowcount,
                    _state: PhantomData,
                }),
            }
        } else {
            FetchResult::Empty(TypedCursor {
                result_set: None,
                description: self.description,
                rowcount: self.rowcount,
                _state: PhantomData,
            })
        }
    }

    /// Reset to idle state.
    pub fn reset(self) -> TypedCursor<Idle> {
        TypedCursor {
            result_set: None,
            description: None,
            rowcount: -1,
            _state: PhantomData,
        }
    }
}

impl TypedCursor<Exhausted> {
    /// Reset to idle state.
    pub fn reset(self) -> TypedCursor<Idle> {
        TypedCursor {
            result_set: None,
            description: None,
            rowcount: -1,
            _state: PhantomData,
        }
    }
}

/// Result of a fetch operation.
#[derive(Debug)]
pub enum FetchResult {
    /// Row fetched successfully.
    Row(hdbconnect::Row, TypedCursor<Fetching>),
    /// No more rows.
    Empty(TypedCursor<Exhausted>),
    /// Error occurred.
    Error(hdbconnect::HdbError),
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
}
