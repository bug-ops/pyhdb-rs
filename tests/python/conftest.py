"""Pytest fixtures for HANA connection tests."""

from __future__ import annotations

import os
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

    import pyhdb_rs


def get_test_uri() -> str | None:
    """Get test HANA URI from environment variable."""
    return os.environ.get("HANA_TEST_URI")


def is_hana_available() -> bool:
    """Check if HANA test connection is available."""
    return get_test_uri() is not None


skip_no_hana = pytest.mark.skipif(
    not is_hana_available(),
    reason="HANA_TEST_URI environment variable not set",
)


@pytest.fixture
def hana_uri() -> str:
    """Get the HANA test URI.

    Raises:
        pytest.skip: If HANA_TEST_URI is not set
    """
    uri = get_test_uri()
    if uri is None:
        pytest.skip("HANA_TEST_URI environment variable not set")
    return uri


@pytest.fixture
def connection(hana_uri: str) -> Generator[pyhdb_rs.Connection, None, None]:
    """Create a HANA connection for tests.

    Yields:
        Connection object

    After the test completes, the connection is closed.
    """
    import pyhdb_rs

    conn = pyhdb_rs.connect(hana_uri)
    try:
        yield conn
    finally:
        conn.close()


@pytest.fixture
def cursor(connection: pyhdb_rs.Connection) -> Generator[pyhdb_rs.Cursor, None, None]:
    """Create a cursor for tests.

    Yields:
        Cursor object

    After the test completes, the cursor is closed.
    """
    cur = connection.cursor()
    try:
        yield cur
    finally:
        cur.close()


def create_pool(url: str, **kwargs):
    """Helper function for tests to create connection pool using builder.

    This is a test helper that wraps ConnectionPoolBuilder for backward compatibility
    with existing tests.
    """
    from pyhdb_rs.aio import ConnectionPoolBuilder

    builder = ConnectionPoolBuilder().url(url)

    if "max_size" in kwargs:
        builder = builder.max_size(kwargs["max_size"])
    if "connection_timeout" in kwargs:
        builder = builder.connection_timeout(kwargs["connection_timeout"])
    if "config" in kwargs:
        builder = builder.config(kwargs["config"])
    if "tls_config" in kwargs:
        builder = builder.tls(kwargs["tls_config"])

    return builder.build()


@pytest.fixture
def connection_url(hana_uri: str) -> str:
    """Get the HANA connection URL for tests.

    Returns:
        HANA connection URL string
    """
    return hana_uri


@pytest.fixture
def sync_connection(hana_uri: str) -> Generator[pyhdb_rs.Connection, None, None]:
    """Create a sync HANA connection for tests.

    Yields:
        Connection object

    After the test completes, the connection is closed.
    """
    import pyhdb_rs

    conn = pyhdb_rs.connect(hana_uri)
    try:
        yield conn
    finally:
        conn.close()


@pytest.fixture
async def async_connection(hana_uri: str):
    """Create an async HANA connection for tests.

    Yields:
        AsyncConnection object

    After the test completes, the connection is closed.
    """
    import pyhdb_rs.aio

    conn = await pyhdb_rs.aio.connect(hana_uri)
    try:
        yield conn
    finally:
        await conn.close()
