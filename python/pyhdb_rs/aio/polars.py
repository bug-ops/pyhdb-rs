"""Async Polars integration for pyhdb_rs.

Provides convenience functions for reading HANA data into Polars DataFrames
using async connections and connection pools.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from . import ASYNC_AVAILABLE, connect

if TYPE_CHECKING:
    import polars as pl

    from . import ConnectionPool


async def read_hana_async(
    url: str,
    sql: str,
) -> pl.DataFrame:
    """Read HANA query results into a Polars DataFrame.

    Creates a temporary connection for the query.

    Args:
        url: Connection URL (hdbsql://user:pass@host:port[/database])
        sql: SQL query string

    Returns:
        Polars DataFrame

    Raises:
        RuntimeError: If async support is not available

    Example:
        >>> df = await read_hana_async(
        ...     "hdbsql://user:pass@host:30015",
        ...     "SELECT * FROM sales WHERE year = 2024"
        ... )
    """
    if not ASYNC_AVAILABLE:
        raise RuntimeError(
            "Async support is not available. Rebuild the package with the 'async' feature enabled."
        )

    async with await connect(url) as conn:
        return await conn.execute_polars(sql)


async def read_hana_pooled(
    pool: ConnectionPool,
    sql: str,
) -> pl.DataFrame:
    """Read HANA query results into a Polars DataFrame using a pool.

    Uses a connection from the provided pool.

    Args:
        pool: Connection pool to use
        sql: SQL query string

    Returns:
        Polars DataFrame

    Example:
        >>> pool = create_pool("hdbsql://user:pass@host:30015", max_size=10)
        >>> df = await read_hana_pooled(pool, "SELECT * FROM sales")
    """
    async with pool.acquire() as conn:
        return await conn.execute_polars(sql)


__all__ = [
    "read_hana_async",
    "read_hana_pooled",
]
