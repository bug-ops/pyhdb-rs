"""Async connection with AsyncConnectionBuilder.

Demonstrates asynchronous connection creation and query execution
using the builder pattern.
"""

import asyncio

import polars as pl

from pyhdb_rs import TlsConfig
from pyhdb_rs.aio import AsyncConnectionBuilder


async def basic_async_connection():
    """Basic async connection with builder."""
    print("Basic Async Connection")
    print("-" * 50)

    conn = await (
        AsyncConnectionBuilder()
        .host("hana.example.com")
        .port(30015)
        .credentials("SYSTEM", "password")
        .database("SYSTEMDB")
        .build()
    )

    try:
        cursor = conn.cursor()
        await cursor.execute("SELECT * FROM DUMMY")
        result = await cursor.fetchone()
        print(f"Query result: {result}")
    finally:
        await conn.close()


async def async_with_tls():
    """Async connection with TLS configuration."""
    print("\nAsync Connection with TLS")
    print("-" * 50)

    conn = await (
        AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .autocommit(True)
        .build()
    )

    try:
        cursor = conn.cursor()
        await cursor.execute("SELECT CURRENT_USER, CURRENT_SCHEMA FROM DUMMY")
        user, schema = await cursor.fetchone()
        print(f"Connected as user: {user}")
        print(f"Current schema: {schema}")
    finally:
        await conn.close()


async def async_with_arrow():
    """Async query with Arrow result."""
    print("\nAsync Query with Arrow")
    print("-" * 50)

    conn = await (
        AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    try:
        reader = await conn.execute_arrow(
            """SELECT SCHEMA_NAME, COUNT(*) AS TABLE_COUNT
               FROM SYS.TABLES
               GROUP BY SCHEMA_NAME
               ORDER BY TABLE_COUNT DESC
               LIMIT 10"""
        )
        df = pl.from_arrow(reader)
        print(df)
    finally:
        await conn.close()


async def async_context_manager():
    """Use async context manager for automatic cleanup."""
    print("\nAsync Context Manager")
    print("-" * 50)

    async with await (
        AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .build()
    ) as conn:
        cursor = conn.cursor()
        await cursor.execute("SELECT DATABASE_NAME FROM SYS.M_DATABASE")
        db_name = await cursor.fetchone()
        print(f"Database name: {db_name[0]}")


async def async_from_url():
    """Build async connection from URL."""
    print("\nAsync Connection from URL")
    print("-" * 50)

    conn = await (
        AsyncConnectionBuilder.from_url("hdbsqls://user:pass@host:30015")
        .autocommit(False)
        .build()
    )

    try:
        cursor = conn.cursor()
        await cursor.execute("SELECT * FROM DUMMY")
        result = await cursor.fetchone()
        print(f"Query result: {result}")
    finally:
        await conn.close()


async def main():
    """Run all async examples."""
    print("Async ConnectionBuilder Examples")
    print("=" * 50)

    await basic_async_connection()
    await async_with_tls()
    await async_with_arrow()
    await async_context_manager()
    await async_from_url()

    print("\n" + "=" * 50)
    print("Async connections support the same configuration options as sync")


if __name__ == "__main__":
    asyncio.run(main())
