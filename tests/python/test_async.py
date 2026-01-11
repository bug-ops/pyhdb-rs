"""Tests for async support in pyhdb_rs.

These tests verify the async API surface and basic functionality.
Integration tests require a running HANA instance.

TODO(TEST): Add unit tests that don't require HANA:
  - test_connect_raises_without_async - verify RuntimeError when async unavailable
  - test_create_pool_raises_without_async - verify RuntimeError when async unavailable
  - test_invalid_url_raises_interface_error - verify error for malformed URLs

TODO(TEST): Add integration tests (require HANA):
  - test_connection_with_statement_cache - verify cache is used
  - test_connection_autocommit_false - verify manual commit/rollback
  - test_execute_arrow_batch_size - verify custom batch sizes
  - test_execute_polars_large_result - verify memory handling
  - test_cursor_description_after_execute - verify column metadata
  - test_cursor_fetchone_returns_none - verify stub returns None
  - test_cursor_fetchmany_returns_empty - verify stub returns empty list
  - test_cursor_async_iteration - verify __aiter__/__anext__ protocol
  - test_connection_rollback - verify rollback behavior
  - test_concurrent_connections - verify multiple async connections
  - test_connection_context_manager_exception - verify cleanup on exception
"""

import os
import pytest

# Check if async is available
try:
    from pyhdb_rs import ASYNC_AVAILABLE
except ImportError:
    ASYNC_AVAILABLE = False

# Skip all tests if async not available
pytestmark = pytest.mark.skipif(
    not ASYNC_AVAILABLE,
    reason="Async support not available (rebuild with 'async' feature)",
)


class TestAsyncImports:
    """Test that async module imports correctly."""

    def test_import_aio_module(self):
        """Test importing the aio module."""
        from pyhdb_rs import aio

        assert hasattr(aio, "ASYNC_AVAILABLE")
        assert hasattr(aio, "connect")
        assert hasattr(aio, "create_pool")

    def test_import_async_classes(self):
        """Test importing async classes."""
        from pyhdb_rs.aio import (
            AsyncConnection,
            AsyncCursor,
            ConnectionPool,
            PooledConnection,
            PoolStatus,
        )

        assert AsyncConnection is not None
        assert AsyncCursor is not None
        assert ConnectionPool is not None
        assert PooledConnection is not None
        assert PoolStatus is not None

    def test_async_available_flag(self):
        """Test ASYNC_AVAILABLE flag."""
        from pyhdb_rs import ASYNC_AVAILABLE
        from pyhdb_rs.aio import ASYNC_AVAILABLE as AIO_ASYNC_AVAILABLE

        # Both should be True when async is available
        assert ASYNC_AVAILABLE is True
        assert AIO_ASYNC_AVAILABLE is True


class TestAsyncPolarsImports:
    """Test that async polars module imports correctly."""

    def test_import_polars_functions(self):
        """Test importing polars helper functions."""
        from pyhdb_rs.aio.polars import read_hana_async, read_hana_pooled

        assert callable(read_hana_async)
        assert callable(read_hana_pooled)


@pytest.mark.skipif(
    not os.environ.get("HANA_TEST_URI"),
    reason="HANA_TEST_URI not set",
)
class TestAsyncConnection:
    """Integration tests for async connection.

    Requires HANA_TEST_URI environment variable to be set.
    """

    @pytest.fixture
    def hana_url(self):
        """Get HANA connection URL from environment."""
        return os.environ["HANA_TEST_URI"]

    @pytest.mark.asyncio
    async def test_connect_and_query(self, hana_url):
        """Test basic async connection and query."""
        from pyhdb_rs.aio import connect

        async with await connect(hana_url) as conn:
            df = await conn.execute_polars("SELECT 1 AS value FROM DUMMY")
            assert df is not None
            assert len(df) == 1

    @pytest.mark.asyncio
    async def test_cursor_execute(self, hana_url):
        """Test async cursor execution."""
        from pyhdb_rs.aio import connect

        async with await connect(hana_url) as conn:
            cursor = conn.cursor()
            await cursor.execute("SELECT 1 AS value FROM DUMMY")
            # Cursor should be in active state after execute
            assert cursor.description is not None

    @pytest.mark.asyncio
    async def test_execute_arrow(self, hana_url):
        """Test async Arrow execution."""
        from pyhdb_rs.aio import connect

        async with await connect(hana_url) as conn:
            reader = await conn.execute_arrow("SELECT 1 AS value FROM DUMMY")
            assert reader is not None


@pytest.mark.skipif(
    not os.environ.get("HANA_TEST_URI"),
    reason="HANA_TEST_URI not set",
)
class TestAsyncPool:
    """Integration tests for connection pool.

    Requires HANA_TEST_URI environment variable to be set.
    """

    @pytest.fixture
    def hana_url(self):
        """Get HANA connection URL from environment."""
        return os.environ["HANA_TEST_URI"]

    def test_create_pool(self, hana_url):
        """Test pool creation."""
        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=5)
        assert pool is not None
        assert pool.max_size == 5

    @pytest.mark.asyncio
    async def test_pool_acquire_release(self, hana_url):
        """Test acquiring and releasing connections from pool."""
        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=2)

        async with pool.acquire() as conn:
            df = await conn.execute_polars("SELECT 1 AS value FROM DUMMY")
            assert df is not None

        # Connection should be returned to pool
        status = pool.status
        assert status.available >= 0

    @pytest.mark.asyncio
    async def test_pool_status(self, hana_url):
        """Test pool status reporting."""
        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=3)
        status = pool.status

        assert status.max_size == 3
        assert status.size >= 0
        assert status.available >= 0
