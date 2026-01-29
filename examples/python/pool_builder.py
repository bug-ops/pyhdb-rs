"""Connection pooling with ConnectionPoolBuilder.

Demonstrates async connection pool configuration and usage
for high-concurrency applications.
"""

import asyncio

import polars as pl

from pyhdb_rs import TlsConfig
from pyhdb_rs.aio import ConnectionPoolBuilder


async def basic_pool():
    """Basic connection pool usage."""
    print("Basic Connection Pool")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:30015")
        .max_size(10)
        .build()
    )

    async with pool.acquire() as conn:
        reader = await conn.execute_arrow("SELECT * FROM DUMMY")
        df = pl.from_arrow(reader)
        print(df)

    status = pool.status
    print(f"Pool size: {status.size}, available: {status.available}")


async def pool_with_tls():
    """Connection pool with TLS configuration."""
    print("\nConnection Pool with TLS")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:30015")
        .max_size(20)
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    async with pool.acquire() as conn:
        cursor = conn.cursor()
        await cursor.execute("SELECT CURRENT_USER FROM DUMMY")
        user = await cursor.fetchone()
        print(f"Connected as: {user[0]}")

    status = pool.status
    print(f"Pool stats - size: {status.size}, available: {status.available}")


async def pool_with_network_group():
    """Connection pool with network group for HA deployments."""
    print("\nConnection Pool with Network Group")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@hana-ha.example.com:30015")
        .max_size(15)
        .network_group("production")
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    async with pool.acquire() as conn:
        reader = await conn.execute_arrow(
            """SELECT HOST, PORT, SQL_PORT
               FROM SYS.M_SERVICES
               WHERE SERVICE_NAME = 'indexserver'"""
        )
        df = pl.from_arrow(reader)
        print("Connected to HANA cluster:")
        print(df)


async def concurrent_queries():
    """Execute multiple queries concurrently using pool."""
    print("\nConcurrent Query Execution")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:30015")
        .max_size(5)
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    async def fetch_schema_info(schema: str):
        async with pool.acquire() as conn:
            reader = await conn.execute_arrow(
                f"""SELECT '{schema}' AS SCHEMA_NAME,
                           COUNT(*) AS TABLE_COUNT
                    FROM SYS.TABLES
                    WHERE SCHEMA_NAME = '{schema}'"""
            )
            return pl.from_arrow(reader)

    # Run queries concurrently
    schemas = ["SYS", "SYSTEM", "_SYS_BIC"]
    results = await asyncio.gather(*[fetch_schema_info(s) for s in schemas])

    for df in results:
        print(df)

    status = pool.status
    print(f"\nPool stats after concurrent execution:")
    print(f"  Total connections: {status.size}")
    print(f"  Available: {status.available}")


async def pool_lifecycle():
    """Demonstrate pool lifecycle management."""
    print("\nPool Lifecycle Management")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:30015")
        .max_size(5)
        .build()
    )

    print(f"Initial pool status: {pool.status}")

    # Acquire multiple connections
    conns = []
    for i in range(3):
        conn = await pool.acquire()
        conns.append(conn)
        print(f"Acquired connection {i + 1}, available: {pool.status.available}")

    # Release connections
    for i, conn in enumerate(conns):
        await pool.release(conn)
        print(f"Released connection {i + 1}, available: {pool.status.available}")


async def pool_error_handling():
    """Handle errors with connection pool."""
    print("\nPool Error Handling")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:30015")
        .max_size(10)
        .build()
    )

    try:
        async with pool.acquire() as conn:
            cursor = conn.cursor()
            # This will fail if table doesn't exist
            await cursor.execute("SELECT * FROM NONEXISTENT_TABLE")
    except Exception as e:
        print(f"Query error (expected): {e}")

    # Pool should still be usable
    async with pool.acquire() as conn:
        cursor = conn.cursor()
        await cursor.execute("SELECT * FROM DUMMY")
        result = await cursor.fetchone()
        print(f"Pool still works after error: {result}")


async def main():
    """Run all connection pool examples."""
    print("Connection Pool Builder Examples")
    print("=" * 50)

    await basic_pool()
    await pool_with_tls()
    await pool_with_network_group()
    await concurrent_queries()
    await pool_lifecycle()
    await pool_error_handling()

    print("\n" + "=" * 50)
    print("Best practices:")
    print("  - Set max_size based on expected concurrency")
    print("  - Use TLS in production")
    print("  - Configure network_group for HA/Scale-Out")
    print("  - Always use context manager (async with) for connection acquisition")


if __name__ == "__main__":
    asyncio.run(main())
