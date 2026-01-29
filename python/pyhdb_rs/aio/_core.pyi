"""Type stubs for async Rust extension module.

This module provides type hints for the async API components.
All async methods use ArrowConfig for batch configuration (not batch_size parameter).
"""

from __future__ import annotations

from collections.abc import Awaitable, Sequence
from types import TracebackType
from typing import Any, Literal, Self

from pyhdb_rs._core import ArrowConfig, CacheStats, ConnectionConfig, RecordBatchReader, TlsConfig

# Feature flag
ASYNC_AVAILABLE: bool

class PoolStatus:
    """Pool status information."""

    @property
    def size(self) -> int: ...
    @property
    def available(self) -> int: ...
    @property
    def max_size(self) -> int: ...
    def __repr__(self) -> str: ...

class AsyncConnection:
    """Async connection to SAP HANA database.

    Use AsyncConnectionBuilder to create instances.
    """

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
    def autocommit(self, value: bool) -> None:
        """Set autocommit mode."""
        ...

    @property
    def is_connected(self) -> Awaitable[bool]:
        """Check if connection is open (async property)."""
        ...

    @property
    def fetch_size(self) -> Awaitable[int]:
        """Current fetch size (async property)."""
        ...

    async def set_fetch_size(self, value: int) -> None:
        """Set fetch size at runtime."""
        ...

    @property
    def read_timeout(self) -> Awaitable[float | None]:
        """Current read timeout in seconds (async property)."""
        ...

    async def set_read_timeout(self, value: float | None) -> None:
        """Set read timeout at runtime."""
        ...

    @property
    def lob_read_length(self) -> Awaitable[int]:
        """Current LOB read length (async property)."""
        ...

    async def set_lob_read_length(self, value: int) -> None:
        """Set LOB read length at runtime."""
        ...

    @property
    def lob_write_length(self) -> Awaitable[int]:
        """Current LOB write length (async property)."""
        ...

    async def set_lob_write_length(self, value: int) -> None:
        """Set LOB write length at runtime."""
        ...

    async def is_valid(self, check_connection: bool = True) -> bool:
        """Check if connection is valid.

        Args:
            check_connection: If True, executes SELECT 1 FROM DUMMY to verify.
        """
        ...

    async def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader:
        """Execute query and return Arrow RecordBatchReader.

        Args:
            sql: SQL query string
            config: Optional ArrowConfig for batch size configuration
        """
        ...

    async def cache_stats(self) -> CacheStats:
        """Get prepared statement cache statistics."""
        ...

    async def clear_cache(self) -> None:
        """Clear the prepared statement cache."""
        ...

    async def __aenter__(self) -> AsyncConnection:
        """Async context manager entry."""
        ...

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]:
        """Async context manager exit."""
        ...

    def __repr__(self) -> str: ...

class AsyncConnectionBuilder:
    """Builder for async SAP HANA connections with TLS support.

    Example::

        conn = await (AsyncConnectionBuilder()
            .host("hana.example.com")
            .credentials("SYSTEM", "password")
            .build())
    """

    def __init__(self) -> None: ...
    @classmethod
    def from_url(cls, url: str) -> AsyncConnectionBuilder: ...
    def host(self, hostname: str) -> Self: ...
    def port(self, port: int) -> Self: ...
    def credentials(self, user: str, password: str) -> Self: ...
    def database(self, name: str) -> Self: ...
    def tls(self, config: TlsConfig) -> Self: ...
    def config(self, config: ConnectionConfig) -> Self: ...
    def autocommit(self, enabled: bool) -> Self: ...
    def network_group(self, group: str) -> Self: ...
    def build(self) -> Awaitable[AsyncConnection]: ...
    def __repr__(self) -> str: ...

class AsyncCursor:
    """Async cursor for query execution.

    Note: fetch methods (fetchone, fetchmany, fetchall) raise NotSupportedError.
    Use connection.execute_arrow() for data retrieval.
    """

    @property
    def rowcount(self) -> int: ...
    @property
    def arraysize(self) -> int: ...
    @arraysize.setter
    def arraysize(self, value: int) -> None: ...
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

    async def __anext__(self) -> tuple[Any, ...]:
        """Fetch next row."""
        ...

    async def __aenter__(self) -> AsyncCursor:
        """Async context manager entry."""
        ...

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]:
        """Async context manager exit."""
        ...

    def __repr__(self) -> str: ...

class ConnectionPool:
    """Connection pool for async HANA connections.

    Use ConnectionPoolBuilder to create instances.

    Example::

        from pyhdb_rs.aio import ConnectionPoolBuilder

        pool = (ConnectionPoolBuilder()
            .url("hdbsql://user:pass@host:30015")
            .max_size(10)
            .tls(TlsConfig.with_system_roots())
            .build())
    """

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

    def __repr__(self) -> str: ...

class ConnectionPoolBuilder:
    """Builder for async connection pools.

    Example::

        pool = (ConnectionPoolBuilder()
            .url("hdbsql://user:pass@host:30015")
            .max_size(20)
            .tls(TlsConfig.with_system_roots())
            .build())
    """

    def __init__(self) -> None: ...
    def url(self, url: str) -> Self: ...
    def max_size(self, size: int) -> Self: ...
    def min_idle(self, size: int) -> Self: ...
    def connection_timeout(self, seconds: int) -> Self: ...
    def config(self, config: ConnectionConfig) -> Self: ...
    def tls(self, config: TlsConfig) -> Self: ...
    def network_group(self, group: str) -> Self: ...
    def build(self) -> ConnectionPool: ...
    def __repr__(self) -> str: ...

class PooledConnection:
    """A connection borrowed from the pool.

    Connection is automatically returned to the pool when __aexit__ is called.
    """

    @property
    def fetch_size(self) -> Awaitable[int]:
        """Current fetch size (async property)."""
        ...

    async def set_fetch_size(self, value: int) -> None:
        """Set fetch size at runtime."""
        ...

    @property
    def read_timeout(self) -> Awaitable[float | None]:
        """Current read timeout in seconds (async property)."""
        ...

    async def set_read_timeout(self, value: float | None) -> None:
        """Set read timeout at runtime."""
        ...

    @property
    def lob_read_length(self) -> Awaitable[int]:
        """Current LOB read length (async property)."""
        ...

    async def set_lob_read_length(self, value: int) -> None:
        """Set LOB read length at runtime."""
        ...

    @property
    def lob_write_length(self) -> Awaitable[int]:
        """Current LOB write length (async property)."""
        ...

    async def set_lob_write_length(self, value: int) -> None:
        """Set LOB write length at runtime."""
        ...

    async def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader:
        """Execute query and return Arrow RecordBatchReader.

        Args:
            sql: SQL query string
            config: Optional ArrowConfig for batch size configuration
        """
        ...

    async def cursor(self) -> AsyncCursor:
        """Create a cursor for this connection."""
        ...

    async def commit(self) -> None:
        """Commit the current transaction."""
        ...

    async def rollback(self) -> None:
        """Rollback the current transaction."""
        ...

    async def is_valid(self, check_connection: bool = True) -> bool:
        """Check if pooled connection is valid."""
        ...

    async def cache_stats(self) -> CacheStats:
        """Get prepared statement cache statistics."""
        ...

    async def clear_cache(self) -> None:
        """Clear the prepared statement cache."""
        ...

    async def __aenter__(self) -> PooledConnection:
        """Async context manager entry."""
        ...

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]:
        """Async context manager exit - returns connection to pool."""
        ...

    async def __repr__(self) -> str: ...
