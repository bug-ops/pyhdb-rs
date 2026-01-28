//! Connection typestate definitions.
//!
//! Implements compile-time state tracking for connections:
//! - Disconnected: Initial state, no active connection
//! - Connected: Active connection, ready for queries
//! - [`InTransaction`]: Inside a transaction

use std::marker::PhantomData;

use crate::private::sealed::Sealed;

/// Marker trait for connection states.
///
/// Sealed to prevent external implementations.
pub trait ConnectionState: Sealed + Send + Sync + 'static {
    /// Human-readable state name for debugging.
    const STATE_NAME: &'static str;

    /// Whether this state allows executing queries.
    const CAN_EXECUTE: bool;

    /// Whether this state is inside a transaction.
    const IN_TRANSACTION: bool;
}

/// Disconnected state - no active connection.
#[derive(Debug, Clone, Copy, Default)]
pub struct Disconnected;

impl Sealed for Disconnected {}

impl ConnectionState for Disconnected {
    const STATE_NAME: &'static str = "Disconnected";
    const CAN_EXECUTE: bool = false;
    const IN_TRANSACTION: bool = false;
}

/// Connected state - active connection ready for queries.
#[derive(Debug, Clone, Copy, Default)]
pub struct Connected;

impl Sealed for Connected {}

impl ConnectionState for Connected {
    const STATE_NAME: &'static str = "Connected";
    const CAN_EXECUTE: bool = true;
    const IN_TRANSACTION: bool = false;
}

/// In-transaction state - active transaction.
#[derive(Debug, Clone, Copy, Default)]
pub struct InTransaction;

impl Sealed for InTransaction {}

impl ConnectionState for InTransaction {
    const STATE_NAME: &'static str = "InTransaction";
    const CAN_EXECUTE: bool = true;
    const IN_TRANSACTION: bool = true;
}

/// Typed connection with compile-time state tracking.
///
/// Uses phantom data to track connection state at compile time.
#[derive(Debug)]
pub struct TypedConnection<S: ConnectionState> {
    /// The underlying hdbconnect connection (if connected).
    inner: Option<hdbconnect::Connection>,
    /// Connection parameters for reconnection.
    params: hdbconnect::ConnectParams,
    /// Phantom marker for state.
    _state: PhantomData<S>,
}

impl<S: ConnectionState> TypedConnection<S> {
    /// Get the current state name.
    #[must_use]
    pub const fn state_name(&self) -> &'static str {
        S::STATE_NAME
    }

    /// Check if queries can be executed in this state.
    #[must_use]
    pub const fn can_execute(&self) -> bool {
        S::CAN_EXECUTE
    }

    /// Check if currently in a transaction.
    #[must_use]
    pub const fn in_transaction(&self) -> bool {
        S::IN_TRANSACTION
    }
}

impl TypedConnection<Disconnected> {
    /// Create a new disconnected connection with parameters.
    #[must_use]
    pub const fn new(params: hdbconnect::ConnectParams) -> Self {
        Self {
            inner: None,
            params,
            _state: PhantomData,
        }
    }

    /// Connect to the database.
    ///
    /// Transitions from Disconnected to Connected state.
    ///
    /// # Errors
    ///
    /// Returns error if connection fails.
    pub fn connect(self) -> Result<TypedConnection<Connected>, hdbconnect::HdbError> {
        let conn = hdbconnect::Connection::new(self.params.clone())?;
        Ok(TypedConnection {
            inner: Some(conn),
            params: self.params,
            _state: PhantomData,
        })
    }
}

impl TypedConnection<Connected> {
    /// Begin a new transaction.
    ///
    /// Transitions from Connected to `InTransaction` state.
    ///
    /// # Errors
    ///
    /// Returns error if transaction start fails.
    pub fn begin_transaction(
        mut self,
    ) -> Result<TypedConnection<InTransaction>, hdbconnect::HdbError> {
        if let Some(ref mut conn) = self.inner {
            conn.set_auto_commit(false)?;
        }
        Ok(TypedConnection {
            inner: self.inner,
            params: self.params,
            _state: PhantomData,
        })
    }

    /// Close the connection.
    ///
    /// Transitions from Connected to Disconnected state.
    pub fn close(self) -> TypedConnection<Disconnected> {
        // Connection is dropped automatically
        TypedConnection {
            inner: None,
            params: self.params,
            _state: PhantomData,
        }
    }

    /// Get a reference to the underlying connection.
    #[must_use]
    pub const fn connection(&self) -> Option<&hdbconnect::Connection> {
        self.inner.as_ref()
    }

    /// Get a mutable reference to the underlying connection.
    pub const fn connection_mut(&mut self) -> Option<&mut hdbconnect::Connection> {
        self.inner.as_mut()
    }
}

impl TypedConnection<InTransaction> {
    /// Commit the current transaction.
    ///
    /// Transitions from `InTransaction` to Connected state.
    ///
    /// # Errors
    ///
    /// Returns error if commit fails.
    pub fn commit(mut self) -> Result<TypedConnection<Connected>, hdbconnect::HdbError> {
        if let Some(ref mut conn) = self.inner {
            conn.commit()?;
            conn.set_auto_commit(true)?;
        }
        Ok(TypedConnection {
            inner: self.inner,
            params: self.params,
            _state: PhantomData,
        })
    }

    /// Rollback the current transaction.
    ///
    /// Transitions from `InTransaction` to Connected state.
    ///
    /// # Errors
    ///
    /// Returns error if rollback fails.
    pub fn rollback(mut self) -> Result<TypedConnection<Connected>, hdbconnect::HdbError> {
        if let Some(ref mut conn) = self.inner {
            conn.rollback()?;
            conn.set_auto_commit(true)?;
        }
        Ok(TypedConnection {
            inner: self.inner,
            params: self.params,
            _state: PhantomData,
        })
    }

    /// Get a reference to the underlying connection.
    #[must_use]
    pub const fn connection(&self) -> Option<&hdbconnect::Connection> {
        self.inner.as_ref()
    }

    /// Get a mutable reference to the underlying connection.
    pub const fn connection_mut(&mut self) -> Option<&mut hdbconnect::Connection> {
        self.inner.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_properties() {
        assert!(!Disconnected::CAN_EXECUTE);
        assert!(!Disconnected::IN_TRANSACTION);

        assert!(Connected::CAN_EXECUTE);
        assert!(!Connected::IN_TRANSACTION);

        assert!(InTransaction::CAN_EXECUTE);
        assert!(InTransaction::IN_TRANSACTION);
    }

    #[test]
    fn test_state_names() {
        assert_eq!(Disconnected::STATE_NAME, "Disconnected");
        assert_eq!(Connected::STATE_NAME, "Connected");
        assert_eq!(InTransaction::STATE_NAME, "InTransaction");
    }

    #[test]
    fn test_state_debug_implementations() {
        assert_eq!(format!("{:?}", Disconnected), "Disconnected");
        assert_eq!(format!("{:?}", Connected), "Connected");
        assert_eq!(format!("{:?}", InTransaction), "InTransaction");
    }

    #[test]
    fn test_state_default_implementations() {
        let _disconnected: Disconnected = Default::default();
        let _connected: Connected = Default::default();
        let _in_transaction: InTransaction = Default::default();
    }

    #[test]
    fn test_state_clone_copy() {
        let disconnected = Disconnected;
        let disconnected_copy = disconnected;
        let _disconnected_clone = disconnected_copy.clone();

        let connected = Connected;
        let connected_copy = connected;
        let _connected_clone = connected_copy.clone();

        let in_tx = InTransaction;
        let in_tx_copy = in_tx;
        let _in_tx_clone = in_tx_copy.clone();
    }
}
