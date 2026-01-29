"""Tests for cursor state management and attributes.

These tests verify cursor initialization, attributes, and state transitions
without requiring a database connection.
"""

from __future__ import annotations


class TestCursorAttributes:
    """Tests for cursor attribute existence."""

    def test_cursor_has_description_attribute(self) -> None:
        """Test that Cursor class has description attribute."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "description")

    def test_cursor_has_rowcount_attribute(self) -> None:
        """Test that Cursor class has rowcount attribute."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "rowcount")

    def test_cursor_has_arraysize_attribute(self) -> None:
        """Test that Cursor class has arraysize attribute."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "arraysize")


class TestCursorMethods:
    """Tests for cursor method existence."""

    def test_cursor_has_execute_method(self) -> None:
        """Test that Cursor class has execute method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "execute")
        assert callable(pyhdb_rs.Cursor.execute)

    def test_cursor_has_executemany_method(self) -> None:
        """Test that Cursor class has executemany method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "executemany")
        assert callable(pyhdb_rs.Cursor.executemany)

    def test_cursor_has_fetchone_method(self) -> None:
        """Test that Cursor class has fetchone method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "fetchone")
        assert callable(pyhdb_rs.Cursor.fetchone)

    def test_cursor_has_fetchmany_method(self) -> None:
        """Test that Cursor class has fetchmany method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "fetchmany")
        assert callable(pyhdb_rs.Cursor.fetchmany)

    def test_cursor_has_fetchall_method(self) -> None:
        """Test that Cursor class has fetchall method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "fetchall")
        assert callable(pyhdb_rs.Cursor.fetchall)

    def test_cursor_has_close_method(self) -> None:
        """Test that Cursor class has close method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "close")
        assert callable(pyhdb_rs.Cursor.close)

    def test_cursor_has_fetch_arrow_method(self) -> None:
        """Test that Cursor class has fetch_arrow method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "fetch_arrow")
        assert callable(pyhdb_rs.Cursor.fetch_arrow)


class TestCursorContextManager:
    """Tests for cursor context manager protocol."""

    def test_cursor_has_enter_method(self) -> None:
        """Test that Cursor class has __enter__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "__enter__")

    def test_cursor_has_exit_method(self) -> None:
        """Test that Cursor class has __exit__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "__exit__")


class TestCursorIterator:
    """Tests for cursor iterator protocol."""

    def test_cursor_has_iter_method(self) -> None:
        """Test that Cursor class has __iter__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "__iter__")

    def test_cursor_has_next_method(self) -> None:
        """Test that Cursor class has __next__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "__next__")


class TestCursorRepr:
    """Tests for cursor repr."""

    def test_cursor_has_repr_method(self) -> None:
        """Test that Cursor class has __repr__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Cursor, "__repr__")


class TestConnectionAttributes:
    """Tests for connection attribute existence."""

    def test_connection_has_cursor_method(self) -> None:
        """Test that Connection class has cursor method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "cursor")
        assert callable(pyhdb_rs.Connection.cursor)

    def test_connection_has_close_method(self) -> None:
        """Test that Connection class has close method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "close")
        assert callable(pyhdb_rs.Connection.close)

    def test_connection_has_commit_method(self) -> None:
        """Test that Connection class has commit method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "commit")
        assert callable(pyhdb_rs.Connection.commit)

    def test_connection_has_rollback_method(self) -> None:
        """Test that Connection class has rollback method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "rollback")
        assert callable(pyhdb_rs.Connection.rollback)

    def test_connection_has_is_connected_property(self) -> None:
        """Test that Connection class has is_connected property."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "is_connected")

    def test_connection_has_autocommit_property(self) -> None:
        """Test that Connection class has autocommit property."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "autocommit")

    def test_connection_has_execute_arrow_method(self) -> None:
        """Test that Connection class has execute_arrow method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "execute_arrow")
        assert callable(pyhdb_rs.Connection.execute_arrow)


class TestConnectionContextManager:
    """Tests for connection context manager protocol."""

    def test_connection_has_enter_method(self) -> None:
        """Test that Connection class has __enter__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "__enter__")

    def test_connection_has_exit_method(self) -> None:
        """Test that Connection class has __exit__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "__exit__")


class TestConnectionRepr:
    """Tests for connection repr."""

    def test_connection_has_repr_method(self) -> None:
        """Test that Connection class has __repr__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.Connection, "__repr__")


class TestRecordBatchReaderAttributes:
    """Tests for RecordBatchReader attribute existence."""

    def test_reader_has_to_pyarrow_method(self) -> None:
        """Test that RecordBatchReader has to_pyarrow method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.RecordBatchReader, "to_pyarrow")
        assert callable(pyhdb_rs.RecordBatchReader.to_pyarrow)

    def test_reader_has_schema_method(self) -> None:
        """Test that RecordBatchReader has schema method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.RecordBatchReader, "schema")
        assert callable(pyhdb_rs.RecordBatchReader.schema)

    def test_reader_has_repr_method(self) -> None:
        """Test that RecordBatchReader has __repr__ method."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs.RecordBatchReader, "__repr__")


class TestDbapiModuleExports:
    """Tests for dbapi module direct exports."""

    def test_dbapi_exports_date(self) -> None:
        """Test that dbapi module exports Date."""
        from pyhdb_rs.dbapi import Date

        assert callable(Date)

    def test_dbapi_exports_time(self) -> None:
        """Test that dbapi module exports Time."""
        from pyhdb_rs.dbapi import Time

        assert callable(Time)

    def test_dbapi_exports_timestamp(self) -> None:
        """Test that dbapi module exports Timestamp."""
        from pyhdb_rs.dbapi import Timestamp

        assert callable(Timestamp)

    def test_dbapi_exports_binary(self) -> None:
        """Test that dbapi module exports Binary."""
        from pyhdb_rs.dbapi import Binary

        assert callable(Binary)

    def test_dbapi_exports_string_type(self) -> None:
        """Test that dbapi module exports STRING type."""
        from pyhdb_rs.dbapi import STRING

        assert STRING is not None

    def test_dbapi_exports_binary_type(self) -> None:
        """Test that dbapi module exports BINARY type."""
        from pyhdb_rs.dbapi import BINARY

        assert BINARY is not None

    def test_dbapi_exports_number_type(self) -> None:
        """Test that dbapi module exports NUMBER type."""
        from pyhdb_rs.dbapi import NUMBER

        assert NUMBER is not None

    def test_dbapi_exports_datetime_type(self) -> None:
        """Test that dbapi module exports DATETIME type."""
        from pyhdb_rs.dbapi import DATETIME

        assert DATETIME is not None

    def test_dbapi_exports_rowid_type(self) -> None:
        """Test that dbapi module exports ROWID type."""
        from pyhdb_rs.dbapi import ROWID

        assert ROWID is not None
