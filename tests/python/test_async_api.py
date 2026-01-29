"""Tests for async module API surface.

These tests verify the async module structure, class availability,
and error handling without requiring a database connection.
"""

from __future__ import annotations

import pytest


def async_available() -> bool:
    """Check if async support is available."""
    try:
        from pyhdb_rs import ASYNC_AVAILABLE

        return ASYNC_AVAILABLE
    except ImportError:
        return False


# Skip all tests if async not available
pytestmark = pytest.mark.skipif(
    not async_available(),
    reason="Async support not available (rebuild with 'async' feature)",
)


class TestAsyncModuleImports:
    """Tests for async module imports."""

    def test_import_aio_module(self) -> None:
        """Test that aio module can be imported."""
        from pyhdb_rs import aio

        assert aio is not None

    def test_aio_has_async_available_flag(self) -> None:
        """Test that aio module has ASYNC_AVAILABLE flag."""
        from pyhdb_rs import aio

        assert hasattr(aio, "ASYNC_AVAILABLE")



class TestAsyncClassExports:
    """Tests for async class exports."""

    def test_async_connection_class_exists(self) -> None:
        """Test that AsyncConnection class is exported."""
        from pyhdb_rs.aio import AsyncConnection

        assert AsyncConnection is not None

    def test_async_cursor_class_exists(self) -> None:
        """Test that AsyncCursor class is exported."""
        from pyhdb_rs.aio import AsyncCursor

        assert AsyncCursor is not None

    def test_connection_pool_class_exists(self) -> None:
        """Test that ConnectionPool class is exported."""
        from pyhdb_rs.aio import ConnectionPool

        assert ConnectionPool is not None

    def test_pooled_connection_class_exists(self) -> None:
        """Test that PooledConnection class is exported."""
        from pyhdb_rs.aio import PooledConnection

        assert PooledConnection is not None

    def test_pool_status_class_exists(self) -> None:
        """Test that PoolStatus class is exported."""
        from pyhdb_rs.aio import PoolStatus

        assert PoolStatus is not None


class TestAsyncConnectionClassMethods:
    """Tests for AsyncConnection class method existence."""

    def test_async_connection_has_cursor_method(self) -> None:
        """Test that AsyncConnection has cursor method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "cursor")

    def test_async_connection_has_close_method(self) -> None:
        """Test that AsyncConnection has close method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "close")

    def test_async_connection_has_commit_method(self) -> None:
        """Test that AsyncConnection has commit method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "commit")

    def test_async_connection_has_rollback_method(self) -> None:
        """Test that AsyncConnection has rollback method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "rollback")

    def test_async_connection_has_execute_arrow_method(self) -> None:
        """Test that AsyncConnection has execute_arrow method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "execute_arrow")

    def test_async_connection_has_is_connected_property(self) -> None:
        """Test that AsyncConnection has is_connected property."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "is_connected")

    def test_async_connection_has_autocommit_property(self) -> None:
        """Test that AsyncConnection has autocommit property."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "autocommit")

    def test_async_connection_has_cache_stats_method(self) -> None:
        """Test that AsyncConnection has cache_stats method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "cache_stats")


class TestAsyncConnectionContextManager:
    """Tests for AsyncConnection async context manager protocol."""

    def test_async_connection_has_aenter_method(self) -> None:
        """Test that AsyncConnection has __aenter__ method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "__aenter__")

    def test_async_connection_has_aexit_method(self) -> None:
        """Test that AsyncConnection has __aexit__ method."""
        from pyhdb_rs.aio import AsyncConnection

        assert hasattr(AsyncConnection, "__aexit__")


class TestAsyncCursorClassMethods:
    """Tests for AsyncCursor class method existence."""

    def test_async_cursor_has_execute_method(self) -> None:
        """Test that AsyncCursor has execute method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "execute")

    def test_async_cursor_has_fetchone_method(self) -> None:
        """Test that AsyncCursor has fetchone method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "fetchone")

    def test_async_cursor_has_fetchmany_method(self) -> None:
        """Test that AsyncCursor has fetchmany method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "fetchmany")

    def test_async_cursor_has_fetchall_method(self) -> None:
        """Test that AsyncCursor has fetchall method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "fetchall")

    def test_async_cursor_has_close_method(self) -> None:
        """Test that AsyncCursor has close method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "close")

    def test_async_cursor_has_description_property(self) -> None:
        """Test that AsyncCursor has description property."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "description")

    def test_async_cursor_has_rowcount_property(self) -> None:
        """Test that AsyncCursor has rowcount property."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "rowcount")

    def test_async_cursor_has_arraysize_property(self) -> None:
        """Test that AsyncCursor has arraysize property."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "arraysize")


class TestAsyncCursorIterator:
    """Tests for AsyncCursor async iterator protocol."""

    def test_async_cursor_has_aiter_method(self) -> None:
        """Test that AsyncCursor has __aiter__ method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "__aiter__")

    def test_async_cursor_has_anext_method(self) -> None:
        """Test that AsyncCursor has __anext__ method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "__anext__")


class TestAsyncCursorContextManager:
    """Tests for AsyncCursor async context manager protocol."""

    def test_async_cursor_has_aenter_method(self) -> None:
        """Test that AsyncCursor has __aenter__ method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "__aenter__")

    def test_async_cursor_has_aexit_method(self) -> None:
        """Test that AsyncCursor has __aexit__ method."""
        from pyhdb_rs.aio import AsyncCursor

        assert hasattr(AsyncCursor, "__aexit__")


class TestConnectionPoolClassMethods:
    """Tests for ConnectionPool class method existence."""

    def test_pool_has_acquire_method(self) -> None:
        """Test that ConnectionPool has acquire method."""
        from pyhdb_rs.aio import ConnectionPool

        assert hasattr(ConnectionPool, "acquire")

    def test_pool_has_status_property(self) -> None:
        """Test that ConnectionPool has status property."""
        from pyhdb_rs.aio import ConnectionPool

        assert hasattr(ConnectionPool, "status")

    def test_pool_has_max_size_property(self) -> None:
        """Test that ConnectionPool has max_size property."""
        from pyhdb_rs.aio import ConnectionPool

        assert hasattr(ConnectionPool, "max_size")

    def test_pool_has_close_method(self) -> None:
        """Test that ConnectionPool has close method."""
        from pyhdb_rs.aio import ConnectionPool

        assert hasattr(ConnectionPool, "close")


class TestPooledConnectionClassMethods:
    """Tests for PooledConnection class method existence."""

    def test_pooled_connection_has_execute_arrow_method(self) -> None:
        """Test that PooledConnection has execute_arrow method."""
        from pyhdb_rs.aio import PooledConnection

        assert hasattr(PooledConnection, "execute_arrow")

    def test_pooled_connection_has_cursor_method(self) -> None:
        """Test that PooledConnection has cursor method."""
        from pyhdb_rs.aio import PooledConnection

        assert hasattr(PooledConnection, "cursor")

    def test_pooled_connection_has_commit_method(self) -> None:
        """Test that PooledConnection has commit method."""
        from pyhdb_rs.aio import PooledConnection

        assert hasattr(PooledConnection, "commit")

    def test_pooled_connection_has_rollback_method(self) -> None:
        """Test that PooledConnection has rollback method."""
        from pyhdb_rs.aio import PooledConnection

        assert hasattr(PooledConnection, "rollback")


class TestPooledConnectionContextManager:
    """Tests for PooledConnection async context manager protocol."""

    def test_pooled_connection_has_aenter_method(self) -> None:
        """Test that PooledConnection has __aenter__ method."""
        from pyhdb_rs.aio import PooledConnection

        assert hasattr(PooledConnection, "__aenter__")

    def test_pooled_connection_has_aexit_method(self) -> None:
        """Test that PooledConnection has __aexit__ method."""
        from pyhdb_rs.aio import PooledConnection

        assert hasattr(PooledConnection, "__aexit__")


class TestPoolStatusAttributes:
    """Tests for PoolStatus class attributes."""

    def test_pool_status_has_size_attribute(self) -> None:
        """Test that PoolStatus has size attribute."""
        from pyhdb_rs.aio import PoolStatus

        assert hasattr(PoolStatus, "size")

    def test_pool_status_has_available_attribute(self) -> None:
        """Test that PoolStatus has available attribute."""
        from pyhdb_rs.aio import PoolStatus

        assert hasattr(PoolStatus, "available")

    def test_pool_status_has_max_size_attribute(self) -> None:
        """Test that PoolStatus has max_size attribute."""
        from pyhdb_rs.aio import PoolStatus

        assert hasattr(PoolStatus, "max_size")


class TestAioModuleAll:
    """Tests for aio module __all__ attribute."""

    def test_aio_all_contains_async_available(self) -> None:
        """Test that __all__ contains ASYNC_AVAILABLE."""
        from pyhdb_rs import aio

        assert "ASYNC_AVAILABLE" in aio.__all__

    def test_aio_all_contains_connect(self) -> None:
        """Test that __all__ contains connect."""
        from pyhdb_rs import aio

        assert "connect" in aio.__all__

    def test_aio_all_contains_async_connection(self) -> None:
        """Test that __all__ contains AsyncConnection."""
        from pyhdb_rs import aio

        assert "AsyncConnection" in aio.__all__

    def test_aio_all_contains_async_cursor(self) -> None:
        """Test that __all__ contains AsyncCursor."""
        from pyhdb_rs import aio

        assert "AsyncCursor" in aio.__all__

    def test_aio_all_contains_connection_pool(self) -> None:
        """Test that __all__ contains ConnectionPool."""
        from pyhdb_rs import aio

        assert "ConnectionPool" in aio.__all__

    def test_aio_all_contains_pooled_connection(self) -> None:
        """Test that __all__ contains PooledConnection."""
        from pyhdb_rs import aio

        assert "PooledConnection" in aio.__all__

    def test_aio_all_contains_pool_status(self) -> None:
        """Test that __all__ contains PoolStatus."""
        from pyhdb_rs import aio

        assert "PoolStatus" in aio.__all__


class TestConnectWithoutAsync:
    """Tests for connect function behavior without async runtime."""

    @pytest.mark.asyncio
    async def test_connect_invalid_url_raises_interface_error(self) -> None:
        """Test that connect with invalid URL raises InterfaceError."""
        import pyhdb_rs
        from pyhdb_rs.aio import connect

        with pytest.raises(pyhdb_rs.InterfaceError):
            await connect("invalid://url")


class TestCreatePoolErrors:
    """Tests for create_pool function error handling."""

    def test_create_pool_invalid_url_raises_error(self) -> None:
        """Test that create_pool with invalid URL raises error."""
        import pyhdb_rs
        from conftest import create_pool

        with pytest.raises((pyhdb_rs.InterfaceError, pyhdb_rs.OperationalError)):
            create_pool("invalid://url")
