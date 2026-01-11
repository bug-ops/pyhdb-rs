"""Type stubs for the pyhdb_rs Rust extension module.

This file provides type hints for IDE support and static analysis.
"""

from __future__ import annotations

from collections.abc import Iterator, Sequence
from types import TracebackType
from typing import Any, Literal

# =====================================================================
# DB-API 2.0 Module Attributes
# =====================================================================

apilevel: Literal["2.0"]
"""DB-API 2.0 compliance level."""

threadsafety: Literal[2]
"""Thread safety level: connections can be shared between threads."""

paramstyle: Literal["qmark"]
"""Parameter marker style: ? (question mark)."""

__version__: str
"""Package version string."""

# =====================================================================
# Connection
# =====================================================================

class Connection:
    """SAP HANA database connection.

    Thread-safe connection object supporting DB-API 2.0 operations
    and Arrow-based data transfer.

    Example::

        conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")
        cursor = conn.cursor()
        cursor.execute("SELECT * FROM DUMMY")
        print(cursor.fetchone())
        conn.close()
    """

    @property
    def autocommit(self) -> bool:
        """Auto-commit mode (default: True)."""
        ...

    @autocommit.setter
    def autocommit(self, value: bool) -> None: ...
    @property
    def is_connected(self) -> bool:
        """Check if connection is open."""
        ...

    def __init__(self, url: str) -> None:
        """Create a new connection.

        Args:
            url: Connection URL in format hdbsql://user:pass@host:port

        Raises:
            InterfaceError: If URL is invalid
            OperationalError: If connection fails
        """
        ...

    def cursor(self) -> Cursor:
        """Create a new cursor object."""
        ...

    def commit(self) -> None:
        """Commit the current transaction."""
        ...

    def rollback(self) -> None:
        """Rollback the current transaction."""
        ...

    def close(self) -> None:
        """Close the connection."""
        ...

    def execute_arrow(
        self,
        sql: str,
        batch_size: int = 65536,
    ) -> RecordBatchReader:
        """Execute query and return Arrow RecordBatchReader.

        This is the high-performance path for analytics workloads.
        Data is transferred zero-copy to Python Arrow/Polars.

        Args:
            sql: SQL query string
            batch_size: Number of rows per batch (default: 65536)

        Returns:
            RecordBatchReader for streaming Arrow results

        Example::

            import polars as pl
            reader = conn.execute_arrow("SELECT * FROM sales")
            df = pl.from_arrow(reader)
        """
        ...

    def execute_polars(self, sql: str) -> Any:
        """Execute query and return Polars DataFrame.

        Convenience method that wraps execute_arrow with pl.from_arrow.
        Requires polars to be installed.

        Args:
            sql: SQL query string

        Returns:
            polars.DataFrame
        """
        ...

    def __enter__(self) -> Connection: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    def __repr__(self) -> str: ...

# =====================================================================
# Cursor
# =====================================================================

class Cursor:
    """Database cursor for executing queries.

    DB-API 2.0 compliant cursor with Arrow extensions.

    Example::

        cursor = conn.cursor()
        cursor.execute("SELECT id, name FROM users WHERE active = ?", (True,))
        for row in cursor:
            print(row)
    """

    @property
    def description(
        self,
    ) -> list[tuple[str, int, None, int | None, int | None, int | None, bool]] | None:
        """Column descriptions from the last query.

        Returns a list of 7-tuples:
        (name, type_code, display_size, internal_size, precision, scale, null_ok)
        """
        ...

    @property
    def rowcount(self) -> int:
        """Number of rows affected by the last DML operation.

        Returns -1 for SELECT statements.
        """
        ...

    @property
    def arraysize(self) -> int:
        """Number of rows to fetch with fetchmany()."""
        ...

    @arraysize.setter
    def arraysize(self, value: int) -> None: ...
    def execute(
        self,
        sql: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
    ) -> None:
        """Execute a SQL query.

        Args:
            sql: SQL statement
            parameters: Optional parameters (not yet supported)

        Raises:
            NotSupportedError: If parameters are provided (not yet implemented)
            ProgrammingError: If SQL syntax is invalid
            OperationalError: If connection is closed
        """
        ...

    def executemany(
        self,
        sql: str,
        seq_of_parameters: Sequence[Sequence[Any]] | None = None,
    ) -> None:
        """Execute a DML statement with multiple parameter sets.

        Args:
            sql: SQL statement
            seq_of_parameters: Sequence of parameter sequences (not yet supported)

        Raises:
            NotSupportedError: If parameters are provided (not yet implemented)
        """
        ...

    def fetchone(self) -> tuple[Any, ...] | None:
        """Fetch the next row from the result set.

        Returns:
            Single row as tuple, or None if no more rows
        """
        ...

    def fetchmany(self, size: int | None = None) -> list[tuple[Any, ...]]:
        """Fetch multiple rows from the result set.

        Args:
            size: Number of rows to fetch (defaults to arraysize)

        Returns:
            List of rows as tuples
        """
        ...

    def fetchall(self) -> list[tuple[Any, ...]]:
        """Fetch all remaining rows from the result set.

        Returns:
            List of all remaining rows as tuples
        """
        ...

    def close(self) -> None:
        """Close the cursor and release resources."""
        ...

    def fetch_arrow(self, batch_size: int = 65536) -> RecordBatchReader:
        """Fetch remaining results as Arrow RecordBatchReader.

        Consumes the result set for zero-copy Arrow transfer.

        Args:
            batch_size: Rows per batch

        Returns:
            RecordBatchReader

        Raises:
            ProgrammingError: If no active result set
        """
        ...

    def __iter__(self) -> Iterator[tuple[Any, ...]]: ...
    def __next__(self) -> tuple[Any, ...]: ...
    def __enter__(self) -> Cursor: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    def __repr__(self) -> str: ...

# =====================================================================
# RecordBatchReader
# =====================================================================

class RecordBatchReader:
    """Arrow RecordBatch reader with PyCapsule interface.

    Implements __arrow_c_stream__ for zero-copy data transfer
    to Polars, PyArrow, pandas, and other Arrow-compatible libraries.

    Example::

        import polars as pl

        reader = conn.execute_arrow("SELECT * FROM sales")
        df = pl.from_arrow(reader)  # Zero-copy!
    """

    def to_pyarrow(self) -> Any:
        """Export to PyArrow RecordBatchReader.

        Consumes this reader.

        Returns:
            pyarrow.RecordBatchReader
        """
        ...

    def schema(self) -> Any:
        """Get the Arrow schema.

        Returns:
            pyarrow.Schema
        """
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# Module-level connect function
# =====================================================================

def connect(url: str) -> Connection:
    """Connect to a SAP HANA database.

    Args:
        url: Connection URL in format hdbsql://user:pass@host:port

    Returns:
        Connection object

    Raises:
        InterfaceError: If URL is invalid
        OperationalError: If connection fails

    Example::

        conn = pyhdb_rs.connect("hdbsql://SYSTEM:password@localhost:39017")
    """
    ...

# =====================================================================
# Exceptions (DB-API 2.0)
# =====================================================================

class Error(Exception):
    """Base class for all database errors."""

    ...

class Warning(Exception):
    """Database warning."""

    ...

class InterfaceError(Error):
    """Error related to the database interface.

    Raised for connection parameter issues, driver problems, etc.
    """

    ...

class DatabaseError(Error):
    """Error related to the database.

    Base class for data-related errors.
    """

    ...

class DataError(DatabaseError):
    """Error due to problems with processed data.

    Raised for type conversion issues, value overflow, etc.
    """

    ...

class OperationalError(DatabaseError):
    """Error related to database operation.

    Raised for connection loss, timeout, authentication failure, etc.
    """

    ...

class IntegrityError(DatabaseError):
    """Error when relational integrity is affected.

    Raised for constraint violations, duplicate keys, etc.
    """

    ...

class InternalError(DatabaseError):
    """Internal database error.

    Raised for unexpected internal errors.
    """

    ...

class ProgrammingError(DatabaseError):
    """Error in programming logic.

    Raised for SQL syntax errors, missing tables, etc.
    """

    ...

class NotSupportedError(DatabaseError):
    """Feature not supported by database.

    Raised when using features not implemented or supported.
    """

    ...
