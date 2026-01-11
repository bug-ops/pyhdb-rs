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
