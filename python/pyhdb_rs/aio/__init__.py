"""Async support for pyhdb_rs - High-performance Python driver for SAP HANA.

This module provides async/await support for HANA database operations.
Requires the package to be built with the 'async' feature.

.. warning::
    The async ``execute_arrow()`` method loads ALL rows into memory before
    returning the RecordBatchReader. For large datasets (>100K rows), use the
    sync API instead which provides true streaming with O(batch_size) memory.

Basic async usage (builder-first API)::

    import asyncio
    from pyhdb_rs.aio import AsyncConnectionBuilder
    import polars as pl

    async def main():
        conn = await (AsyncConnectionBuilder()
            .host("hana.example.com")
            .credentials("SYSTEM", "password")
            .build())

        async with conn:
            reader = await conn.execute_arrow("SELECT * FROM sales")
            df = pl.from_arrow(reader)
            print(df)

    asyncio.run(main())

Connection pooling::

    from pyhdb_rs import TlsConfig
    from pyhdb_rs.aio import ConnectionPoolBuilder
    import polars as pl

    pool = (ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:39017")
        .max_size(20)
        .tls(TlsConfig.with_system_roots())
        .build())

    async with pool.acquire() as conn:
        reader = await conn.execute_arrow("SELECT * FROM sales")
        df = pl.from_arrow(reader)
"""

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


__all__ = [
    # Feature flag
    "ASYNC_AVAILABLE",
    # Classes
    "AsyncConnection",
    "AsyncConnectionBuilder",
    "AsyncCursor",
    "ConnectionPool",
    "ConnectionPoolBuilder",
    "PooledConnection",
    "PoolStatus",
]
