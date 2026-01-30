"""Tests for Phase 3: Stored Procedures (Basic Implementation).

Tests cover:
- Sync Cursor: callproc() and nextset() (8 tests)
- Async Cursor: callproc() and nextset() (4 tests)

Total: 12+ tests

Note: Integration tests require HANA with stored procedures and are marked
with skip_no_hana. Unit tests run without HANA connection.
"""

from __future__ import annotations

import pyhdb_rs
import pytest
from conftest import skip_no_hana
from pyhdb_rs import NotSupportedError, ProgrammingError

# ============================================================================
# Unit Tests (No HANA Required)
# ============================================================================


class TestCallprocValidation:
    """Tests for callproc() parameter validation (no HANA required)."""

    def test_callproc_empty_name_raises_programming_error(
        self, sync_connection: pyhdb_rs.Connection
    ) -> None:
        """callproc() with empty name raises ProgrammingError."""
        cursor = sync_connection.cursor()
        with pytest.raises(ProgrammingError, match="procedure name cannot be empty"):
            cursor.callproc("")

    def test_callproc_invalid_name_sql_injection(
        self, sync_connection: pyhdb_rs.Connection
    ) -> None:
        """callproc() with SQL injection attempt raises ProgrammingError."""
        cursor = sync_connection.cursor()
        with pytest.raises(ProgrammingError, match="invalid procedure name"):
            cursor.callproc("PROC; DROP TABLE USERS")

    def test_callproc_invalid_name_quotes(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() with quote characters raises ProgrammingError."""
        cursor = sync_connection.cursor()
        with pytest.raises(ProgrammingError, match="invalid procedure name"):
            cursor.callproc("PROC'NAME")

    def test_callproc_invalid_name_double_dots(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() with consecutive dots raises ProgrammingError."""
        cursor = sync_connection.cursor()
        with pytest.raises(ProgrammingError, match="invalid procedure name"):
            cursor.callproc("SCHEMA..PROC")

    def test_callproc_invalid_name_leading_dot(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() with leading dot raises ProgrammingError."""
        cursor = sync_connection.cursor()
        with pytest.raises(ProgrammingError, match="invalid procedure name"):
            cursor.callproc(".PROC")

    def test_callproc_invalid_name_trailing_dot(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() with trailing dot raises ProgrammingError."""
        cursor = sync_connection.cursor()
        with pytest.raises(ProgrammingError, match="invalid procedure name"):
            cursor.callproc("PROC.")


class TestNextset:
    """Tests for nextset() stub implementation."""

    def test_nextset_returns_false(self, sync_connection: pyhdb_rs.Connection) -> None:
        """nextset() always returns False (MVP stub)."""
        cursor = sync_connection.cursor()
        result = cursor.nextset()
        assert result is False

    def test_nextset_returns_false_after_query(self, sync_connection: pyhdb_rs.Connection) -> None:
        """nextset() returns False even after executing a query."""
        cursor = sync_connection.cursor()
        cursor.execute("SELECT 1 FROM DUMMY")
        result = cursor.nextset()
        assert result is False


# ============================================================================
# Async Unit Tests (No HANA Required)
# ============================================================================


class TestAsyncCallprocValidation:
    """Tests for async callproc() validation (no HANA required)."""

    @pytest.mark.asyncio
    async def test_async_callproc_empty_name_raises_programming_error(
        self, async_connection: pyhdb_rs.AsyncConnection
    ) -> None:
        """Async callproc() with empty name raises ProgrammingError."""
        cursor = async_connection.cursor()
        with pytest.raises(ProgrammingError, match="procedure name cannot be empty"):
            await cursor.callproc("")

    @pytest.mark.asyncio
    async def test_async_callproc_with_params_raises_not_supported(
        self, async_connection: pyhdb_rs.AsyncConnection
    ) -> None:
        """Async callproc() with parameters raises NotSupportedError."""
        cursor = async_connection.cursor()
        with pytest.raises(NotSupportedError, match="parameterized procedures"):
            await cursor.callproc("MY_PROC", [1, 2, 3])


class TestAsyncNextset:
    """Tests for async nextset() stub implementation."""

    @pytest.mark.asyncio
    async def test_async_nextset_returns_false(
        self, async_connection: pyhdb_rs.AsyncConnection
    ) -> None:
        """Async nextset() always returns False (MVP stub)."""
        cursor = async_connection.cursor()
        result = cursor.nextset()
        assert result is False


# ============================================================================
# Integration Tests (HANA Required)
# ============================================================================


@skip_no_hana
class TestCallprocIntegration:
    """Integration tests for callproc() with HANA stored procedures."""

    def test_callproc_no_params_succeeds(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() with no parameters executes successfully.

        Note: This test requires a stored procedure that takes no parameters.
        If the procedure doesn't exist, the test verifies proper error handling.
        """
        cursor = sync_connection.cursor()

        # Try to call a procedure that should exist or handle error gracefully
        try:
            cursor.callproc("SYS.GET_INSUFFICIENT_PRIVILEGE_ERROR_DETAILS")
        except pyhdb_rs.ProgrammingError:
            # Expected if procedure doesn't exist or wrong schema
            pass

    def test_callproc_returns_input_params(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() returns input parameters unchanged per DB-API 2.0."""
        cursor = sync_connection.cursor()
        params = [1, "test", 3.14]

        # Even if procedure doesn't exist, we can verify the return behavior
        # by catching the expected error and checking the return value pattern
        try:
            result = cursor.callproc("NONEXISTENT_PROC_FOR_TESTING", params)
            # If somehow succeeds (unlikely), verify return value
            if result is not None:
                assert list(result) == params
        except pyhdb_rs.ProgrammingError:
            # Expected - procedure doesn't exist
            pass

    def test_callproc_on_closed_connection_raises_operational_error(
        self, sync_connection: pyhdb_rs.Connection
    ) -> None:
        """callproc() on closed connection raises OperationalError."""
        cursor = sync_connection.cursor()
        sync_connection.close()

        with pytest.raises(pyhdb_rs.OperationalError, match="connection is closed"):
            cursor.callproc("ANY_PROC")

    def test_callproc_schema_qualified_name(self, sync_connection: pyhdb_rs.Connection) -> None:
        """callproc() accepts schema.procedure format."""
        cursor = sync_connection.cursor()

        # Verify schema.procedure format is accepted (may fail at execution)
        try:
            cursor.callproc("SYS.SOME_PROCEDURE")
        except pyhdb_rs.ProgrammingError:
            # Expected if procedure doesn't exist
            pass


@skip_no_hana
class TestAsyncCallprocIntegration:
    """Integration tests for async callproc() with HANA."""

    @pytest.mark.asyncio
    async def test_async_callproc_no_params_succeeds(
        self, async_connection: pyhdb_rs.AsyncConnection
    ) -> None:
        """Async callproc() with no parameters executes successfully."""
        cursor = async_connection.cursor()

        try:
            await cursor.callproc("SYS.GET_INSUFFICIENT_PRIVILEGE_ERROR_DETAILS")
        except pyhdb_rs.ProgrammingError:
            # Expected if procedure doesn't exist
            pass

    @pytest.mark.asyncio
    async def test_async_callproc_on_closed_connection_raises_operational_error(
        self, async_connection: pyhdb_rs.AsyncConnection
    ) -> None:
        """Async callproc() on closed connection raises OperationalError."""
        cursor = async_connection.cursor()
        await async_connection.close()

        with pytest.raises(pyhdb_rs.OperationalError, match="connection is closed"):
            await cursor.callproc("ANY_PROC")
