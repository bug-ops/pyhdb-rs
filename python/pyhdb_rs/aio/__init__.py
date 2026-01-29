"""Async support for pyhdb_rs - High-performance Python driver for SAP HANA.

This module provides async/await support for HANA database operations.
Requires the package to be built with the 'async' feature.

.. warning::
    The async ``execute_arrow()`` method loads ALL rows into memory before
    returning the RecordBatchReader. For large datasets (>100K rows), use the
    sync API instead which provides true streaming with O(batch_size) memory.

Basic async usage::

    import asyncio
    from pyhdb_rs.aio import connect

    async def main():
        async with await connect("hdbsql://user:pass@host:39017") as conn:
            reader = await conn.execute_arrow("SELECT * FROM sales")
            df = pl.from_arrow(reader)
            print(df)

    asyncio.run(main())

Connection with configuration::

    from pyhdb_rs import ConnectionConfig
    from pyhdb_rs.aio import connect

    config = ConnectionConfig(
        fetch_size=50000,
        read_timeout=60.0,
    )
    async with await connect("hdbsql://...", config=config) as conn:
        reader = await conn.execute_arrow("SELECT * FROM sales")

Builder-based async connection with TLS::

    from pyhdb_rs import TlsConfig
    from pyhdb_rs.aio import AsyncConnectionBuilder

    conn = await (AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .build())

Connection pooling::

    from pyhdb_rs import ConnectionConfig
    from pyhdb_rs.aio import create_pool

    config = ConnectionConfig(fetch_size=50000, read_timeout=30.0)
    pool = create_pool("hdbsql://user:pass@host:39017", max_size=10, config=config)

    async def query():
        async with pool.acquire() as conn:
            cursor = conn.cursor()
            await cursor.execute("SELECT * FROM products")
            async for row in cursor:
                print(row)

    asyncio.run(query())

Connection pool builder::

    from pyhdb_rs import TlsConfig
    from pyhdb_rs.aio import ConnectionPoolBuilder

    pool = (ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:39017")
        .max_size(20)
        .tls(TlsConfig.with_system_roots())
        .build())
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pyhdb_rs import ConnectionConfig

try:
    from pyhdb_rs._core import (
        ASYNC_AVAILABLE,
        AsyncConnection,
        AsyncConnectionBuilder,
        AsyncCursor,
        ConnectionPool,
        ConnectionPoolBuilder,
        PooledConnection,
        PoolStatus,
    )
except ImportError:
    ASYNC_AVAILABLE = False
    AsyncConnection = None
    AsyncConnectionBuilder = None
    AsyncCursor = None
    ConnectionPool = None
    ConnectionPoolBuilder = None
    PooledConnection = None
    PoolStatus = None


async def connect(
    url: str,
    *,
    autocommit: bool = True,
    config: ConnectionConfig | None = None,
) -> AsyncConnection:
    """Connect to a HANA database asynchronously.

    Args:
        url: Connection URL (hdbsql://user:pass@host:port[/database])
        autocommit: Enable auto-commit mode (default: True)
        config: Optional connection configuration for tuning performance

    Returns:
        AsyncConnection object

    Raises:
        InterfaceError: If URL is invalid
        OperationalError: If connection fails
        RuntimeError: If async support is not available

    Example:
        >>> async with await connect("hdbsql://user:pass@host:30015") as conn:
        ...     reader = await conn.execute_arrow("SELECT * FROM sales")
        ...     df = pl.from_arrow(reader)

        >>> # With configuration
        >>> config = ConnectionConfig(fetch_size=50000, read_timeout=60.0)
        >>> async with await connect("hdbsql://...", config=config) as conn:
        ...     reader = await conn.execute_arrow("SELECT * FROM sales")
    """
    if not ASYNC_AVAILABLE:
        raise RuntimeError(
            "Async support is not available. Rebuild the package with the 'async' feature enabled."
        )

    return await AsyncConnection.connect(
        url,
        autocommit=autocommit,
        config=config,
    )


def create_pool(
    url: str,
    *,
    max_size: int = 10,
    connection_timeout: int = 30,
    config: ConnectionConfig | None = None,
) -> ConnectionPool:
    """Create a connection pool.

    Args:
        url: Connection URL (hdbsql://user:pass@host:port[/database])
        max_size: Maximum pool size (default: 10)
        connection_timeout: Connection timeout in seconds (default: 30)
        config: Optional connection configuration applied to all pooled connections

    Returns:
        ConnectionPool object

    Raises:
        InterfaceError: If URL is invalid
        OperationalError: If pool creation fails
        RuntimeError: If async support is not available

    Example:
        >>> pool = create_pool("hdbsql://user:pass@host:30015", max_size=20)
        >>> async with pool.acquire() as conn:
        ...     reader = await conn.execute_arrow("SELECT * FROM sales")
        ...     df = pl.from_arrow(reader)

        >>> # With configuration
        >>> config = ConnectionConfig(fetch_size=50000, read_timeout=30.0)
        >>> pool = create_pool("hdbsql://...", max_size=20, config=config)
    """
    if not ASYNC_AVAILABLE:
        raise RuntimeError(
            "Async support is not available. Rebuild the package with the 'async' feature enabled."
        )

    return ConnectionPool(
        url,
        max_size=max_size,
        connection_timeout=connection_timeout,
        config=config,
    )


__all__ = [
    # Feature flag
    "ASYNC_AVAILABLE",
    # Factory functions
    "connect",
    "create_pool",
    # Classes
    "AsyncConnection",
    "AsyncConnectionBuilder",
    "AsyncCursor",
    "ConnectionPool",
    "ConnectionPoolBuilder",
    "PooledConnection",
    "PoolStatus",
]
