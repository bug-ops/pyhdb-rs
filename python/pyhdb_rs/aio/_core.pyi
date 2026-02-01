"""Type stubs for async Rust extension module.

This module provides type hints for the async API components.
All async methods use ArrowConfig for batch configuration (not batch_size parameter).
"""

from __future__ import annotations

from collections.abc import Awaitable, Sequence
from types import TracebackType
from typing import Any, Literal, Self

from pyhdb_rs._core import (
    ArrowConfig,
    CacheStats,
    ConnectionConfig,
    ConnectionStatistics,
    RecordBatchReader,
    TlsConfig,
)

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

    async def set_application(self, name: str) -> None:
        """Set application name for monitoring.

        Visible in SAP HANA `M_CONNECTIONS` system view as `APPLICATION` column.

        Args:
            name: Application name (e.g., `OrderProcessingService`).

        Raises:
            OperationalError: If connection is closed.
        """
        ...

    async def set_application_user(self, user: str) -> None:
        """Set application user for monitoring.

        Typically the end-user making the request, distinct from database user.
        Visible in `M_CONNECTIONS` as `APPLICATIONUSER`.

        Args:
            user: Application-level user identifier.
        """
        ...

    async def set_application_version(self, version: str) -> None:
        """Set application version for monitoring.

        Args:
            version: Version string (e.g., "2.3.1").
        """
        ...

    async def set_application_source(self, source: str) -> None:
        """Set application source location for debugging.

        Args:
            source: Source identifier (e.g., "orders/process.py:42").
        """
        ...

    async def client_info(self) -> dict[str, str]:
        """Get client context information sent to server.

        Returns:
            Dictionary of client info key-value pairs.
        """
        ...

    async def connection_id(self) -> int:
        """Get connection ID assigned by SAP HANA server.

        Returns:
            Server-assigned connection ID for this session.

        Raises:
            OperationalError: If connection is closed.
        """
        ...

    async def server_memory_usage(self) -> int:
        """Get current server memory usage in bytes.

        Returns:
            Memory usage in bytes.

        Raises:
            OperationalError: If connection is closed.
        """
        ...

    async def server_processing_time(self) -> int:
        """Get cumulative server processing time in microseconds.

        Returns:
            Cumulative processing time in microseconds.

        Raises:
            OperationalError: If connection is closed.
        """
        ...

    async def statistics(self) -> ConnectionStatistics:
        """Get connection performance statistics.

        Returns snapshot of connection performance metrics including:
        - Roundtrip count and average latency
        - Request/reply compression ratios
        - Total accumulated wait time

        Returns:
            ConnectionStatistics object with performance metrics.

        Raises:
            OperationalError: If connection is closed.

        Example::

            stats = await conn.statistics()
            print(f"Roundtrips: {stats.call_count}")
            print(f"Avg latency: {stats.avg_wait_time:.2f}ms")
        """
        ...

    async def reset_statistics(self) -> None:
        """Reset connection statistics to zero.

        Useful for measuring specific operations or time windows.

        Raises:
            OperationalError: If connection is closed.

        Example::

            await conn.reset_statistics()
            # Execute some queries
            stats = await conn.statistics()
        """
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
    def application(
        self,
        name: str,
        version: str | None = None,
        user: str | None = None,
        source: str | None = None,
    ) -> Self:
        """Set application metadata for monitoring.

        All values are set on the connection after it's established.
        Visible in SAP HANA `M_CONNECTIONS` system view.

        Args:
            name: Application name (required).
            version: Application version (optional).
            user: Application-level user (optional).
            source: Source location for debugging (optional).

        Returns:
            Self for method chaining.
        """
        ...
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

    async def callproc(
        self,
        procname: str,
        parameters: Sequence[Any] | None = None,
    ) -> None:
        """Call a stored database procedure (async).

        Note: Parameters not supported in async cursor.
        Use connection.execute_arrow() for data retrieval.

        Args:
            procname: Procedure name (can include schema: "SCHEMA.PROC")
            parameters: Not supported, raises NotSupportedError if provided

        Returns:
            None (parameters not supported in async cursor)

        Raises:
            NotSupportedError: If parameters provided
            ProgrammingError: If procedure name is invalid or empty

        Example::

            >>> await cursor.callproc("CLEANUP_OLD_RECORDS")
        """
        ...

    def nextset(self) -> bool:
        """Skip to next result set.

        Returns:
            False (always - stub implementation for async cursor)

        Note:
            This is a stub implementation. Async cursor does not support
            multiple result sets in the current release. Use the sync Cursor
            for procedures returning multiple result sets.

            Full async multiple result set support is planned for a future release.
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

    async def set_application(self, name: str) -> None:
        """Set application name for monitoring.

        Visible in SAP HANA `M_CONNECTIONS` system view as `APPLICATION` column.

        Args:
            name: Application name (e.g., `OrderProcessingService`).

        Raises:
            OperationalError: If connection returned to pool.
        """
        ...

    async def set_application_user(self, user: str) -> None:
        """Set application user for monitoring.

        Typically the end-user making the request, distinct from database user.
        Visible in `M_CONNECTIONS` as `APPLICATIONUSER`.

        Args:
            user: Application-level user identifier.
        """
        ...

    async def set_application_version(self, version: str) -> None:
        """Set application version for monitoring.

        Args:
            version: Version string (e.g., "2.3.1").
        """
        ...

    async def set_application_source(self, source: str) -> None:
        """Set application source location for debugging.

        Args:
            source: Source identifier (e.g., "orders/process.py:42").
        """
        ...

    async def client_info(self) -> dict[str, str]:
        """Get client context information sent to server.

        Returns:
            Dictionary of client info key-value pairs.
        """
        ...

    async def connection_id(self) -> int:
        """Get connection ID assigned by SAP HANA server.

        Returns:
            Server-assigned connection ID for this session.

        Raises:
            OperationalError: If connection returned to pool.
        """
        ...

    async def server_memory_usage(self) -> int:
        """Get current server memory usage in bytes.

        Returns:
            Memory usage in bytes.

        Raises:
            OperationalError: If connection returned to pool.
        """
        ...

    async def server_processing_time(self) -> int:
        """Get cumulative server processing time in microseconds.

        Returns:
            Cumulative processing time in microseconds.

        Raises:
            OperationalError: If connection returned to pool.
        """
        ...

    async def statistics(self) -> ConnectionStatistics:
        """Get connection performance statistics.

        Returns snapshot of connection performance metrics including:
        - Roundtrip count and average latency
        - Request/reply compression ratios
        - Total accumulated wait time

        Returns:
            ConnectionStatistics object with performance metrics.

        Raises:
            OperationalError: If connection returned to pool.

        Example::

            async with pool.acquire() as conn:
                stats = await conn.statistics()
                print(f"Roundtrips: {stats.call_count}")
        """
        ...

    async def reset_statistics(self) -> None:
        """Reset connection statistics to zero.

        Useful for measuring specific operations or time windows.

        Raises:
            OperationalError: If connection returned to pool.

        Example::

            async with pool.acquire() as conn:
                await conn.reset_statistics()
                # Execute some queries
                stats = await conn.statistics()
        """
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
