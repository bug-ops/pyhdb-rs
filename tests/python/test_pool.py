"""Tests for connection pool in pyhdb_rs.

These tests verify pool configuration and behavior.
Integration tests require a running HANA instance.

TODO(TEST): Add unit tests that don't require HANA:
  - test_invalid_url_raises_error - verify error handling for bad URLs
  - test_pool_zero_max_size_error - verify error for max_size=0

TODO(TEST): Add integration tests (require HANA):
  - test_pool_exhaustion_wait - verify waiting when pool is exhausted
  - test_pool_connection_health_check - verify health check on recycle
  - test_pool_acquire_timeout - verify timeout when pool exhausted
  - test_pooled_connection_rollback - verify rollback behavior
  - test_pooled_connection_execute_arrow - verify Arrow result
  - test_pooled_connection_cursor_execute - verify cursor through pool
  - test_pool_concurrent_acquire_release - verify thread safety
  - test_pool_close_returns_connections - verify all connections returned
  - test_pool_status_during_operations - verify status updates
  - test_pool_after_close_raises_error - verify error on closed pool
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


class TestPoolConfiguration:
    """Test pool configuration options."""

    @pytest.mark.skipif(
        not os.environ.get("HANA_TEST_URI"),
        reason="HANA_TEST_URI not set",
    )
    def test_default_pool_config(self):
        """Test default pool configuration."""
        from pyhdb_rs.aio import create_pool

        url = os.environ["HANA_TEST_URI"]
        pool = create_pool(url)

        assert pool.max_size == 10  # default

    @pytest.mark.skipif(
        not os.environ.get("HANA_TEST_URI"),
        reason="HANA_TEST_URI not set",
    )
    def test_custom_pool_config(self):
        """Test custom pool configuration."""
        from pyhdb_rs.aio import create_pool

        url = os.environ["HANA_TEST_URI"]
        pool = create_pool(url, max_size=20, connection_timeout=60)

        assert pool.max_size == 20


class TestPoolStatus:
    """Test pool status reporting."""

    @pytest.mark.skipif(
        not os.environ.get("HANA_TEST_URI"),
        reason="HANA_TEST_URI not set",
    )
    def test_pool_status_attributes(self):
        """Test that pool status has required attributes."""
        from pyhdb_rs.aio import create_pool

        url = os.environ["HANA_TEST_URI"]
        pool = create_pool(url, max_size=5)
        status = pool.status

        assert hasattr(status, "size")
        assert hasattr(status, "available")
        assert hasattr(status, "max_size")

        assert isinstance(status.size, int)
        assert isinstance(status.available, int)
        assert isinstance(status.max_size, int)
        assert status.max_size == 5


@pytest.mark.skipif(
    not os.environ.get("HANA_TEST_URI"),
    reason="HANA_TEST_URI not set",
)
class TestPoolOperations:
    """Integration tests for pool operations."""

    @pytest.fixture
    def hana_url(self):
        """Get HANA connection URL from environment."""
        return os.environ["HANA_TEST_URI"]

    @pytest.mark.asyncio
    async def test_concurrent_queries(self, hana_url):
        """Test running concurrent queries through pool."""
        import asyncio

        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=3)

        async def query():
            async with pool.acquire() as conn:
                return await conn.execute_polars("SELECT 1 AS value FROM DUMMY")

        # Run 5 concurrent queries with pool size 3
        results = await asyncio.gather(*[query() for _ in range(5)])

        assert len(results) == 5
        for df in results:
            assert len(df) == 1

    @pytest.mark.asyncio
    async def test_pool_connection_reuse(self, hana_url):
        """Test that connections are reused."""
        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=1)

        # First query
        async with pool.acquire() as conn:
            await conn.execute_polars("SELECT 1 FROM DUMMY")

        # Second query should reuse the connection
        async with pool.acquire() as conn:
            await conn.execute_polars("SELECT 2 FROM DUMMY")

        # Pool should have exactly 1 connection
        status = pool.status
        assert status.size <= 1

    @pytest.mark.asyncio
    async def test_pool_transaction(self, hana_url):
        """Test transaction operations through pooled connection."""
        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=1)

        async with pool.acquire() as conn:
            # Start a transaction
            cursor = conn.cursor()

            # Execute some queries
            await cursor.execute("SELECT 1 FROM DUMMY")

            # Commit
            await conn.commit()

    @pytest.mark.asyncio
    async def test_pool_close(self, hana_url):
        """Test closing the pool."""
        from pyhdb_rs.aio import create_pool

        pool = create_pool(hana_url, max_size=2)

        # Use the pool
        async with pool.acquire() as conn:
            await conn.execute_polars("SELECT 1 FROM DUMMY")

        # Close the pool
        await pool.close()

        # After close, pool should be empty
        status = pool.status
        assert status.size == 0
