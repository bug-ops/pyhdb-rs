"""Type stubs for async Rust extension module."""

from collections.abc import Sequence
from typing import Any

# Feature flag
ASYNC_AVAILABLE: bool

class PoolStatus:
    """Pool status information."""

    size: int
    available: int
    max_size: int

class AsyncConnection:
    """Async connection to SAP HANA database."""

    @classmethod
    async def connect(
        cls,
        url: str,
        *,
        autocommit: bool = True,
        statement_cache_size: int = 0,
    ) -> AsyncConnection:
        """Connect to HANA database asynchronously."""
        ...

    def cursor(self) -> AsyncCursor:
        """Create a new cursor."""
        ...

    async def close(self) -> None:
        """Close the connection."""
        ...

    async def commit(self) -> None:
        """Commit the current transaction."""
        ...

    async def rollback(self) -> None:
        """Rollback the current transaction."""
        ...

    @property
    def autocommit(self) -> bool:
        """Get autocommit mode."""
        ...

    @autocommit.setter
    async def autocommit(self, value: bool) -> None:
        """Set autocommit mode."""
        ...

    async def execute_arrow(
        self,
        sql: str,
        batch_size: int = 65536,
    ) -> Any:
        """Execute query and return Arrow RecordBatchReader."""
        ...

    async def execute_polars(self, sql: str) -> Any:
        """Execute query and return Polars DataFrame."""
        ...

    async def __aenter__(self) -> AsyncConnection:
        """Async context manager entry."""
        ...

    async def __aexit__(
        self,
        exc_type: type | None,
        exc_val: BaseException | None,
        exc_tb: Any | None,
    ) -> bool:
        """Async context manager exit."""
        ...

class AsyncCursor:
    """Async cursor for query execution.

    Note: fetch methods (fetchone, fetchmany, fetchall) raise NotSupportedError.
    Use connection.execute_arrow() or execute_polars() for data retrieval.
    """

    rowcount: int
    arraysize: int

    @property
    def description(self) -> None:
        """Column descriptions - always None in async cursor."""
        ...

    async def execute(
        self,
        sql: str,
        parameters: Sequence[Any] | None = None,
    ) -> None:
        """Execute a SQL query.

        Note: parameters argument raises NotSupportedError if provided.
        """
        ...

    def fetchone(self) -> None:
        """Fetch one row.

        Raises:
            NotSupportedError: Always - use execute_arrow() instead.
        """
        ...

    def fetchmany(self, size: int | None = None) -> None:
        """Fetch multiple rows.

        Raises:
            NotSupportedError: Always - use execute_arrow() instead.
        """
        ...

    def fetchall(self) -> None:
        """Fetch all rows.

        Raises:
            NotSupportedError: Always - use execute_arrow() instead.
        """
        ...

    def close(self) -> None:
        """Close the cursor."""
        ...

    def __aiter__(self) -> AsyncCursor:
        """Async iterator protocol."""
        ...

    def __anext__(self) -> tuple[Any, ...] | None:
        """Fetch next row - always returns None (StopAsyncIteration)."""
        ...

    def __aenter__(self) -> AsyncCursor:
        """Async context manager entry."""
        ...

    async def __aexit__(
        self,
        exc_type: type | None,
        exc_val: BaseException | None,
        exc_tb: Any | None,
    ) -> bool:
        """Async context manager exit."""
        ...

class ConnectionPool:
    """Connection pool for async HANA connections."""

    def __init__(
        self,
        url: str,
        *,
        max_size: int = 10,
        connection_timeout: int = 30,
    ) -> None:
        """Create a new connection pool."""
        ...

    async def acquire(self) -> PooledConnection:
        """Acquire a connection from the pool."""
        ...

    @property
    def status(self) -> PoolStatus:
        """Get current pool status."""
        ...

    @property
    def max_size(self) -> int:
        """Get maximum pool size."""
        ...

    async def close(self) -> None:
        """Close all connections in the pool."""
        ...

class PooledConnection:
    """A connection borrowed from the pool.

    Connection is automatically returned to the pool when __aexit__ is called
    or when the object is garbage collected.
    """

    async def execute_arrow(
        self,
        sql: str,
        batch_size: int = 65536,
    ) -> Any:
        """Execute query and return Arrow RecordBatchReader."""
        ...

    async def execute_polars(self, sql: str) -> Any:
        """Execute query and return Polars DataFrame."""
        ...

    async def cursor(self) -> AsyncCursor:
        """Create a cursor for this connection.

        Note: Cursor fetch methods raise NotSupportedError.
        Use execute_arrow() or execute_polars() instead.
        """
        ...

    async def commit(self) -> None:
        """Commit the current transaction."""
        ...

    async def rollback(self) -> None:
        """Rollback the current transaction."""
        ...

    def __aenter__(self) -> PooledConnection:
        """Async context manager entry."""
        ...

    async def __aexit__(
        self,
        exc_type: type | None,
        exc_val: BaseException | None,
        exc_tb: Any | None,
    ) -> bool:
        """Async context manager exit - returns connection to pool."""
        ...
