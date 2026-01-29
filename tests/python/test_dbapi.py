"""DB-API 2.0 compliance tests for pyhdb_rs."""

from __future__ import annotations

import datetime
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    import pyhdb_rs


class TestModuleAttributes:
    """Tests for DB-API 2.0 module-level attributes."""

    def test_apilevel(self) -> None:
        """Test that apilevel is '2.0'."""
        import pyhdb_rs

        assert pyhdb_rs.apilevel == "2.0"

    def test_threadsafety(self) -> None:
        """Test that threadsafety is 2 (connections can be shared)."""
        import pyhdb_rs

        assert pyhdb_rs.threadsafety == 2

    def test_paramstyle(self) -> None:
        """Test that paramstyle is 'qmark' (question mark style)."""
        import pyhdb_rs

        assert pyhdb_rs.paramstyle == "qmark"

    def test_version(self) -> None:
        """Test that __version__ is a valid semver string."""
        import pyhdb_rs

        assert isinstance(pyhdb_rs.__version__, str)
        parts = pyhdb_rs.__version__.split(".")
        assert len(parts) >= 2
        assert all(p.isdigit() for p in parts[:2])


class TestExceptions:
    """Tests for DB-API 2.0 exception hierarchy."""

    def test_error_is_exception(self) -> None:
        """Test that Error inherits from Exception."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.Error, Exception)

    def test_warning_is_exception(self) -> None:
        """Test that Warning inherits from Exception."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.Warning, Exception)

    def test_interface_error_inherits_error(self) -> None:
        """Test that InterfaceError inherits from Error."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.InterfaceError, pyhdb_rs.Error)

    def test_database_error_inherits_error(self) -> None:
        """Test that DatabaseError inherits from Error."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.DatabaseError, pyhdb_rs.Error)

    def test_data_error_inherits_database_error(self) -> None:
        """Test that DataError inherits from DatabaseError."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.DataError, pyhdb_rs.DatabaseError)

    def test_operational_error_inherits_database_error(self) -> None:
        """Test that OperationalError inherits from DatabaseError."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.OperationalError, pyhdb_rs.DatabaseError)

    def test_integrity_error_inherits_database_error(self) -> None:
        """Test that IntegrityError inherits from DatabaseError."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.IntegrityError, pyhdb_rs.DatabaseError)

    def test_internal_error_inherits_database_error(self) -> None:
        """Test that InternalError inherits from DatabaseError."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.InternalError, pyhdb_rs.DatabaseError)

    def test_programming_error_inherits_database_error(self) -> None:
        """Test that ProgrammingError inherits from DatabaseError."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.ProgrammingError, pyhdb_rs.DatabaseError)

    def test_not_supported_error_inherits_database_error(self) -> None:
        """Test that NotSupportedError inherits from DatabaseError."""
        import pyhdb_rs

        assert issubclass(pyhdb_rs.NotSupportedError, pyhdb_rs.DatabaseError)


class TestTypeConstructors:
    """Tests for DB-API 2.0 type constructors."""

    def test_date(self) -> None:
        """Test Date constructor."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 6, 15)
        assert isinstance(d, datetime.date)
        assert d.year == 2024
        assert d.month == 6
        assert d.day == 15

    def test_time(self) -> None:
        """Test Time constructor."""
        import pyhdb_rs

        t = pyhdb_rs.Time(14, 30, 45)
        assert isinstance(t, datetime.time)
        assert t.hour == 14
        assert t.minute == 30
        assert t.second == 45

    def test_timestamp(self) -> None:
        """Test Timestamp constructor."""
        import pyhdb_rs

        ts = pyhdb_rs.Timestamp(2024, 6, 15, 14, 30, 45)
        assert isinstance(ts, datetime.datetime)
        assert ts.year == 2024
        assert ts.month == 6
        assert ts.day == 15
        assert ts.hour == 14
        assert ts.minute == 30
        assert ts.second == 45

    def test_date_from_ticks(self) -> None:
        """Test DateFromTicks constructor."""
        import pyhdb_rs

        ticks = 1718470245.0  # 2024-06-15 14:30:45 UTC
        d = pyhdb_rs.DateFromTicks(ticks)
        assert isinstance(d, datetime.date)

    def test_time_from_ticks(self) -> None:
        """Test TimeFromTicks constructor."""
        import pyhdb_rs

        ticks = 1718470245.0
        t = pyhdb_rs.TimeFromTicks(ticks)
        assert isinstance(t, datetime.time)

    def test_timestamp_from_ticks(self) -> None:
        """Test TimestampFromTicks constructor."""
        import pyhdb_rs

        ticks = 1718470245.0
        ts = pyhdb_rs.TimestampFromTicks(ticks)
        assert isinstance(ts, datetime.datetime)

    def test_binary(self) -> None:
        """Test Binary constructor."""
        import pyhdb_rs

        b = pyhdb_rs.Binary(b"\x00\x01\x02")
        assert isinstance(b, bytes)
        assert b == b"\x00\x01\x02"

    def test_binary_from_bytearray(self) -> None:
        """Test Binary constructor with bytearray."""
        import pyhdb_rs

        b = pyhdb_rs.Binary(bytearray([0, 1, 2]))
        assert isinstance(b, bytes)
        assert b == b"\x00\x01\x02"


class TestTypeObjects:
    """Tests for DB-API 2.0 type objects."""

    def test_string_comparison(self) -> None:
        """Test STRING type object comparison."""
        import pyhdb_rs

        assert pyhdb_rs.STRING == 9
        assert pyhdb_rs.STRING == 11
        assert pyhdb_rs.STRING != 1

    def test_binary_comparison(self) -> None:
        """Test BINARY type object comparison."""
        import pyhdb_rs

        assert pyhdb_rs.BINARY == 12
        assert pyhdb_rs.BINARY == 13
        assert pyhdb_rs.BINARY != 1

    def test_number_comparison(self) -> None:
        """Test NUMBER type object comparison."""
        import pyhdb_rs

        assert pyhdb_rs.NUMBER == 3
        assert pyhdb_rs.NUMBER == 4
        assert pyhdb_rs.NUMBER != 9

    def test_datetime_comparison(self) -> None:
        """Test DATETIME type object comparison."""
        import pyhdb_rs

        assert pyhdb_rs.DATETIME == 14
        assert pyhdb_rs.DATETIME == 15
        assert pyhdb_rs.DATETIME != 1

    def test_rowid_comparison(self) -> None:
        """Test ROWID type object comparison."""
        import pyhdb_rs

        assert pyhdb_rs.ROWID == 4
        assert pyhdb_rs.ROWID != 9


class TestConnectFunction:
    """Tests for the connect function."""

    def test_connect_returns_connection(
        self, hana_uri: str, connection: pyhdb_rs.Connection
    ) -> None:
        """Test that connect returns a Connection object."""
        import pyhdb_rs

        assert isinstance(connection, pyhdb_rs.Connection)

    def test_connect_invalid_url(self) -> None:
        """Test that invalid URL raises InterfaceError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.InterfaceError):
            pyhdb_rs.connect("invalid://url")


class TestConnectionContextManager:
    """Tests for Connection context manager."""

    def test_context_manager(self, hana_uri: str) -> None:
        """Test that Connection works as context manager."""
        import pyhdb_rs

        with pyhdb_rs.connect(hana_uri) as conn:
            assert conn.is_connected

        assert not conn.is_connected


class TestCursor:
    """Tests for Cursor operations."""

    def test_cursor_description_none_before_execute(
        self, cursor: pyhdb_rs.Cursor
    ) -> None:
        """Test that description is None before execute."""
        assert cursor.description is None

    def test_cursor_rowcount_minus_one_for_select(
        self, cursor: pyhdb_rs.Cursor
    ) -> None:
        """Test that rowcount is -1 for SELECT."""
        cursor.execute("SELECT 1 FROM DUMMY")
        assert cursor.rowcount == -1

    def test_cursor_arraysize_default(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test that arraysize has a sensible default."""
        assert cursor.arraysize > 0

    def test_cursor_arraysize_setter(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test that arraysize can be set."""
        cursor.arraysize = 100
        assert cursor.arraysize == 100

    def test_fetchone(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test fetchone returns a tuple."""
        cursor.execute("SELECT 1, 'hello' FROM DUMMY")
        row = cursor.fetchone()
        assert row is not None
        assert isinstance(row, tuple)
        assert len(row) == 2

    def test_fetchone_exhausted(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test fetchone returns None when exhausted."""
        cursor.execute("SELECT 1 FROM DUMMY")
        cursor.fetchone()
        row = cursor.fetchone()
        assert row is None

    def test_fetchmany(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test fetchmany returns list of tuples."""
        cursor.execute("SELECT 1 FROM DUMMY")
        rows = cursor.fetchmany(2)
        assert isinstance(rows, list)
        assert len(rows) >= 1
        assert all(isinstance(row, tuple) for row in rows)

    def test_fetchall(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test fetchall returns list of tuples."""
        cursor.execute("SELECT 1 FROM DUMMY")
        rows = cursor.fetchall()
        assert isinstance(rows, list)
        assert len(rows) == 1
        assert all(isinstance(row, tuple) for row in rows)

    def test_cursor_iterator(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test cursor is iterable."""
        cursor.execute("SELECT 1 FROM DUMMY")
        rows = list(cursor)
        assert len(rows) == 1
        assert isinstance(rows[0], tuple)

    def test_cursor_description_after_execute(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test description is populated after execute."""
        cursor.execute("SELECT 1 AS col1, 'test' AS col2 FROM DUMMY")
        desc = cursor.description
        assert desc is not None
        assert len(desc) == 2
        assert desc[0][0].upper() == "COL1"
        assert desc[1][0].upper() == "COL2"


class TestCursorContextManager:
    """Tests for Cursor context manager."""

    def test_context_manager(self, connection: pyhdb_rs.Connection) -> None:
        """Test that Cursor works as context manager."""
        with connection.cursor() as cursor:
            cursor.execute("SELECT 1 FROM DUMMY")
            row = cursor.fetchone()
            assert row is not None
