"""Tests for Phase 2: Connection Statistics API.

Tests cover:
- Sync Connection: 3 methods + 2 closed state tests (5 tests)
- Async Connection: 3 methods + 2 closed state tests (5 tests)
- Pooled Connection: 3 methods + 2 returned-to-pool tests (5 tests)

Total: 15 tests

Note: server_cpu_time() was removed because hdbconnect's ServerUsage.accum_cpu_time
field is private and not exposed in the public API.
"""

from __future__ import annotations

import pyhdb_rs
import pytest
from pyhdb_rs import OperationalError

# ============================================================================
# Sync Connection Tests
# ============================================================================


class TestSyncConnectionStatistics:
    """Tests for Connection.connection_id/server_memory_usage/server_processing_time."""

    def test_connection_id(self, sync_connection: pyhdb_rs.Connection) -> None:
        """connection_id() returns positive integer."""
        conn_id = sync_connection.connection_id()
        assert isinstance(conn_id, int)
        assert conn_id > 0

    def test_server_memory_usage(self, sync_connection: pyhdb_rs.Connection) -> None:
        """server_memory_usage() returns non-negative integer."""
        memory = sync_connection.server_memory_usage()
        assert isinstance(memory, int)
        assert memory >= 0

    def test_server_processing_time(self, sync_connection: pyhdb_rs.Connection) -> None:
        """server_processing_time() returns non-negative integer."""
        proc_time = sync_connection.server_processing_time()
        assert isinstance(proc_time, int)
        assert proc_time >= 0

    def test_connection_id_closed(self, sync_connection: pyhdb_rs.Connection) -> None:
        """connection_id() raises OperationalError when closed."""
        sync_connection.close()
        with pytest.raises(OperationalError, match="connection is closed"):
            sync_connection.connection_id()

    def test_server_statistics_closed(self, sync_connection: pyhdb_rs.Connection) -> None:
        """Server statistics methods raise OperationalError when closed."""
        sync_connection.close()

        with pytest.raises(OperationalError, match="connection is closed"):
            sync_connection.server_memory_usage()

        with pytest.raises(OperationalError, match="connection is closed"):
            sync_connection.server_processing_time()


# ============================================================================
# Async Connection Tests
# ============================================================================


class TestAsyncConnectionStatistics:
    """Tests for AsyncConnection.connection_id/server_memory_usage/server_processing_time."""

    @pytest.mark.asyncio
    async def test_connection_id(self, async_connection: pyhdb_rs.AsyncConnection) -> None:
        """connection_id() returns positive integer."""
        conn_id = await async_connection.connection_id()
        assert isinstance(conn_id, int)
        assert conn_id > 0

    @pytest.mark.asyncio
    async def test_server_memory_usage(self, async_connection: pyhdb_rs.AsyncConnection) -> None:
        """server_memory_usage() returns non-negative integer."""
        memory = await async_connection.server_memory_usage()
        assert isinstance(memory, int)
        assert memory >= 0

    @pytest.mark.asyncio
    async def test_server_processing_time(self, async_connection: pyhdb_rs.AsyncConnection) -> None:
        """server_processing_time() returns non-negative integer."""
        proc_time = await async_connection.server_processing_time()
        assert isinstance(proc_time, int)
        assert proc_time >= 0

    @pytest.mark.asyncio
    async def test_connection_id_closed(self, async_connection: pyhdb_rs.AsyncConnection) -> None:
        """connection_id() raises OperationalError when closed."""
        await async_connection.close()
        with pytest.raises(OperationalError, match="connection is closed"):
            await async_connection.connection_id()

    @pytest.mark.asyncio
    async def test_server_statistics_closed(
        self, async_connection: pyhdb_rs.AsyncConnection
    ) -> None:
        """Server statistics methods raise OperationalError when closed."""
        await async_connection.close()

        with pytest.raises(OperationalError, match="connection is closed"):
            await async_connection.server_memory_usage()

        with pytest.raises(OperationalError, match="connection is closed"):
            await async_connection.server_processing_time()


# ============================================================================
# Pooled Connection Tests
# ============================================================================


class TestPooledConnectionStatistics:
    """Tests for PooledConnection.connection_id/server_memory_usage/server_processing_time."""

    @pytest.mark.asyncio
    async def test_connection_id(self, pooled_connection: pyhdb_rs.PooledConnection) -> None:
        """connection_id() returns positive integer."""
        conn_id = await pooled_connection.connection_id()
        assert isinstance(conn_id, int)
        assert conn_id > 0

    @pytest.mark.asyncio
    async def test_server_memory_usage(self, pooled_connection: pyhdb_rs.PooledConnection) -> None:
        """server_memory_usage() returns non-negative integer."""
        memory = await pooled_connection.server_memory_usage()
        assert isinstance(memory, int)
        assert memory >= 0

    @pytest.mark.asyncio
    async def test_server_processing_time(
        self, pooled_connection: pyhdb_rs.PooledConnection
    ) -> None:
        """server_processing_time() returns non-negative integer."""
        proc_time = await pooled_connection.server_processing_time()
        assert isinstance(proc_time, int)
        assert proc_time >= 0

    @pytest.mark.asyncio
    async def test_connection_id_returned_to_pool(
        self, connection_pool: pyhdb_rs.ConnectionPool
    ) -> None:
        """connection_id() raises OperationalError when returned to pool."""
        pooled = await connection_pool.acquire()
        await pooled.__aexit__(None, None, None)

        with pytest.raises(OperationalError, match="connection returned to pool"):
            await pooled.connection_id()

    @pytest.mark.asyncio
    async def test_server_statistics_returned_to_pool(
        self, connection_pool: pyhdb_rs.ConnectionPool
    ) -> None:
        """Server statistics methods raise OperationalError when returned to pool."""
        pooled = await connection_pool.acquire()
        await pooled.__aexit__(None, None, None)

        with pytest.raises(OperationalError, match="connection returned to pool"):
            await pooled.server_memory_usage()

        with pytest.raises(OperationalError, match="connection returned to pool"):
            await pooled.server_processing_time()
