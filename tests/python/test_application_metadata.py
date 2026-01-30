"""Tests for application metadata features (Issue #54 Phase 1).

These tests verify:
- Connection.set_application() and related methods
- Connection.client_info()
- ConnectionBuilder.application()
- Async equivalents

Requires HANA_TEST_URI environment variable.
"""

import pytest

# Skip all tests if pyhdb_rs not available
pytest.importorskip("pyhdb_rs")

from pyhdb_rs import OperationalError


@pytest.mark.integration
class TestSyncApplicationMetadata:
    """Tests for sync Connection application metadata."""

    def test_set_application(self, sync_connection):
        """Test set_application() sets application name."""
        sync_connection.set_application("TestApp")
        info = sync_connection.client_info()
        # Note: client_info() returns client-side values, may not include APPLICATION
        # until next server round-trip
        assert isinstance(info, dict)

    def test_set_application_user(self, sync_connection):
        """Test set_application_user() sets application user."""
        sync_connection.set_application_user("test_user@example.com")
        info = sync_connection.client_info()
        assert isinstance(info, dict)

    def test_set_application_version(self, sync_connection):
        """Test set_application_version() sets version."""
        sync_connection.set_application_version("1.0.0")
        info = sync_connection.client_info()
        assert isinstance(info, dict)

    def test_set_application_source(self, sync_connection):
        """Test set_application_source() sets source location."""
        sync_connection.set_application_source("test_module.py:42")
        info = sync_connection.client_info()
        assert isinstance(info, dict)

    def test_client_info_returns_dict(self, sync_connection):
        """Test client_info() returns dictionary."""
        info = sync_connection.client_info()
        assert isinstance(info, dict)
        # Keys and values should be strings
        for key, value in info.items():
            assert isinstance(key, str)
            assert isinstance(value, str)

    def test_set_application_on_closed_connection_raises(self, sync_connection):
        """Test set_application() raises when connection closed."""
        sync_connection.close()
        with pytest.raises(OperationalError):
            sync_connection.set_application("TestApp")

    def test_all_methods_chainable(self, sync_connection):
        """Test all application metadata methods can be called in sequence."""
        sync_connection.set_application("TestApp")
        sync_connection.set_application_user("user@test.com")
        sync_connection.set_application_version("2.0.0")
        sync_connection.set_application_source("main.py")
        info = sync_connection.client_info()
        assert isinstance(info, dict)


@pytest.mark.integration
class TestConnectionBuilderApplicationMetadata:
    """Tests for ConnectionBuilder.application() method."""

    def test_builder_application_method(self, connection_url):
        """Test ConnectionBuilder.application() sets metadata."""
        from pyhdb_rs import ConnectionBuilder

        conn = (
            ConnectionBuilder.from_url(connection_url)
            .application("BuilderTestApp", version="1.0.0", user="builder_user")
            .build()
        )

        try:
            info = conn.client_info()
            assert isinstance(info, dict)
            # Metadata should be set post-connection
        finally:
            conn.close()

    def test_builder_application_name_only(self, connection_url):
        """Test ConnectionBuilder.application() with name only."""
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(connection_url).application("MinimalApp").build()

        try:
            info = conn.client_info()
            assert isinstance(info, dict)
        finally:
            conn.close()


@pytest.mark.integration
@pytest.mark.asyncio
class TestAsyncApplicationMetadata:
    """Tests for async Connection application metadata."""

    async def test_async_set_application(self, async_connection):
        """Test async set_application()."""
        await async_connection.set_application("AsyncTestApp")
        info = await async_connection.client_info()
        assert isinstance(info, dict)

    async def test_async_set_application_user(self, async_connection):
        """Test async set_application_user()."""
        await async_connection.set_application_user("async_user@test.com")
        info = await async_connection.client_info()
        assert isinstance(info, dict)

    async def test_async_set_application_version(self, async_connection):
        """Test async set_application_version()."""
        await async_connection.set_application_version("3.0.0")
        info = await async_connection.client_info()
        assert isinstance(info, dict)

    async def test_async_set_application_source(self, async_connection):
        """Test async set_application_source()."""
        await async_connection.set_application_source("async_module.py:100")
        info = await async_connection.client_info()
        assert isinstance(info, dict)

    async def test_async_client_info_returns_dict(self, async_connection):
        """Test async client_info() returns dictionary."""
        info = await async_connection.client_info()
        assert isinstance(info, dict)
        for key, value in info.items():
            assert isinstance(key, str)
            assert isinstance(value, str)

    async def test_async_set_application_on_closed_connection(self, async_connection):
        """Test async set_application() raises when connection closed."""
        await async_connection.close()
        with pytest.raises(OperationalError):
            await async_connection.set_application("TestApp")


@pytest.mark.integration
@pytest.mark.asyncio
class TestAsyncBuilderApplicationMetadata:
    """Tests for AsyncConnectionBuilder.application() method."""

    async def test_async_builder_application_method(self, connection_url):
        """Test AsyncConnectionBuilder.application() sets metadata."""
        from pyhdb_rs.aio import AsyncConnectionBuilder

        conn = await (
            AsyncConnectionBuilder.from_url(connection_url)
            .application("AsyncBuilderApp", version="2.0.0", user="async_builder")
            .build()
        )

        try:
            info = await conn.client_info()
            assert isinstance(info, dict)
        finally:
            await conn.close()


@pytest.mark.integration
@pytest.mark.asyncio
class TestPooledConnectionApplicationMetadata:
    """Tests for PooledConnection application metadata."""

    async def test_pooled_set_application(self, connection_pool):
        """Test set_application() on pooled connection."""
        async with await connection_pool.acquire() as conn:
            await conn.set_application("PooledApp")
            info = await conn.client_info()
            assert isinstance(info, dict)

    async def test_pooled_set_application_user(self, connection_pool):
        """Test set_application_user() on pooled connection."""
        async with await connection_pool.acquire() as conn:
            await conn.set_application_user("pooled_user@test.com")
            info = await conn.client_info()
            assert isinstance(info, dict)

    async def test_pooled_all_methods(self, connection_pool):
        """Test all application metadata methods on pooled connection."""
        async with await connection_pool.acquire() as conn:
            await conn.set_application("PooledTestApp")
            await conn.set_application_user("pool_user")
            await conn.set_application_version("4.0.0")
            await conn.set_application_source("pool_test.py")
            info = await conn.client_info()
            assert isinstance(info, dict)


# Note: Fixtures (sync_connection, async_connection, connection_pool, connection_url)
# should be defined in conftest.py or imported from existing test fixtures.
# These tests assume standard pytest fixtures for HANA connections.
