"""Async support for pyhdb_rs - High-performance Python driver for SAP HANA.

This module provides async/await support for HANA database operations.
Requires the package to be built with the 'async' feature.

Basic async usage::

    import asyncio
    from pyhdb_rs.aio import connect

    async def main():
        async with await connect("hdbsql://user:pass@host:39017") as conn:
            df = await conn.execute_polars("SELECT * FROM sales")
            print(df)

    asyncio.run(main())

Connection pooling::

    from pyhdb_rs.aio import create_pool

    pool = create_pool("hdbsql://user:pass@host:39017", max_size=10)

    async def query():
        async with pool.acquire() as conn:
            cursor = conn.cursor()
            await cursor.execute("SELECT * FROM products")
            async for row in cursor:
                print(row)

    asyncio.run(query())
"""

from __future__ import annotations

try:
    from pyhdb_rs._core import (
        ASYNC_AVAILABLE,
        AsyncConnection,
        AsyncCursor,
        ConnectionPool,
        PooledConnection,
        PoolStatus,
    )
except ImportError:
    ASYNC_AVAILABLE = False
    AsyncConnection = None
    AsyncCursor = None
    ConnectionPool = None
    PooledConnection = None
    PoolStatus = None


async def connect(
    url: str,
    *,
    autocommit: bool = True,
    statement_cache_size: int = 0,
) -> AsyncConnection:
    """Connect to a HANA database asynchronously.

    Args:
        url: Connection URL (hdbsql://user:pass@host:port[/database])
        autocommit: Enable auto-commit mode (default: True)
        statement_cache_size: Size of prepared statement cache (default: 0, disabled)

    Returns:
        AsyncConnection object

    Raises:
        InterfaceError: If URL is invalid
        OperationalError: If connection fails
        RuntimeError: If async support is not available

    Example:
        >>> async with await connect("hdbsql://user:pass@host:30015") as conn:
        ...     df = await conn.execute_polars("SELECT * FROM sales")
    """
    if not ASYNC_AVAILABLE:
        raise RuntimeError(
            "Async support is not available. Rebuild the package with the 'async' feature enabled."
        )

    return await AsyncConnection.connect(
        url,
        autocommit=autocommit,
        statement_cache_size=statement_cache_size,
    )


def create_pool(
    url: str,
    *,
    max_size: int = 10,
    connection_timeout: int = 30,
) -> ConnectionPool:
    """Create a connection pool.

    Args:
        url: Connection URL (hdbsql://user:pass@host:port[/database])
        max_size: Maximum pool size (default: 10)
        connection_timeout: Connection timeout in seconds (default: 30)

    Returns:
        ConnectionPool object

    Raises:
        InterfaceError: If URL is invalid
        OperationalError: If pool creation fails
        RuntimeError: If async support is not available

    Example:
        >>> pool = create_pool("hdbsql://user:pass@host:30015", max_size=20)
        >>> async with pool.acquire() as conn:
        ...     df = await conn.execute_polars("SELECT * FROM sales")
    """
    if not ASYNC_AVAILABLE:
        raise RuntimeError(
            "Async support is not available. Rebuild the package with the 'async' feature enabled."
        )

    return ConnectionPool(
        url,
        max_size=max_size,
        connection_timeout=connection_timeout,
    )


__all__ = [
    # Feature flag
    "ASYNC_AVAILABLE",
    # Factory functions
    "connect",
    "create_pool",
    # Classes
    "AsyncConnection",
    "AsyncCursor",
    "ConnectionPool",
    "PooledConnection",
    "PoolStatus",
]
