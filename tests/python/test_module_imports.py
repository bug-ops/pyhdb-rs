"""Tests for module imports and class instantiation.

These tests verify that pyhdb_rs module structure is correct
and all expected classes/functions are accessible.
"""

from __future__ import annotations


class TestCoreModuleImports:
    """Tests for core module imports."""

    def test_import_pyhdb_rs(self) -> None:
        """Test that pyhdb_rs can be imported."""
        import pyhdb_rs

        assert pyhdb_rs is not None

    def test_import_core_module(self) -> None:
        """Test that _core module can be imported."""
        import pyhdb_rs._core

        assert pyhdb_rs._core is not None

    def test_connection_class_exists(self) -> None:
        """Test that Connection class is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "Connection")
        assert pyhdb_rs.Connection is not None

    def test_cursor_class_exists(self) -> None:
        """Test that Cursor class is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "Cursor")
        assert pyhdb_rs.Cursor is not None

    def test_record_batch_reader_class_exists(self) -> None:
        """Test that RecordBatchReader class is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "RecordBatchReader")
        assert pyhdb_rs.RecordBatchReader is not None

    def test_connect_function_exists(self) -> None:
        """Test that connect function is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "connect")
        assert callable(pyhdb_rs.connect)


class TestModuleAttributes:
    """Tests for DB-API 2.0 module-level attributes."""

    def test_apilevel_value(self) -> None:
        """Test that apilevel is '2.0'."""
        import pyhdb_rs

        assert pyhdb_rs.apilevel == "2.0"

    def test_apilevel_type(self) -> None:
        """Test that apilevel is a string."""
        import pyhdb_rs

        assert isinstance(pyhdb_rs.apilevel, str)

    def test_threadsafety_value(self) -> None:
        """Test that threadsafety is 2 (connections shareable)."""
        import pyhdb_rs

        assert pyhdb_rs.threadsafety == 2

    def test_threadsafety_type(self) -> None:
        """Test that threadsafety is an integer."""
        import pyhdb_rs

        assert isinstance(pyhdb_rs.threadsafety, int)

    def test_paramstyle_value(self) -> None:
        """Test that paramstyle is 'qmark'."""
        import pyhdb_rs

        assert pyhdb_rs.paramstyle == "qmark"

    def test_paramstyle_type(self) -> None:
        """Test that paramstyle is a string."""
        import pyhdb_rs

        assert isinstance(pyhdb_rs.paramstyle, str)

    def test_version_exists(self) -> None:
        """Test that __version__ is defined."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "__version__")

    def test_version_is_string(self) -> None:
        """Test that __version__ is a string."""
        import pyhdb_rs

        assert isinstance(pyhdb_rs.__version__, str)

    def test_version_is_semver(self) -> None:
        """Test that __version__ follows semver pattern."""
        import pyhdb_rs

        parts = pyhdb_rs.__version__.split(".")
        assert len(parts) >= 2, "Version should have at least major.minor"
        assert all(p.isdigit() for p in parts[:2])


class TestAsyncAvailability:
    """Tests for async feature availability flag."""

    def test_async_available_flag_exists(self) -> None:
        """Test that ASYNC_AVAILABLE flag is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "ASYNC_AVAILABLE")

    def test_async_available_is_bool(self) -> None:
        """Test that ASYNC_AVAILABLE is a boolean."""
        import pyhdb_rs

        assert isinstance(pyhdb_rs.ASYNC_AVAILABLE, bool)


class TestDbapiModule:
    """Tests for dbapi module imports."""

    def test_import_dbapi_module(self) -> None:
        """Test that dbapi module can be imported."""
        import pyhdb_rs.dbapi

        assert pyhdb_rs.dbapi is not None

    def test_date_constructor_exists(self) -> None:
        """Test that Date constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "Date")
        assert callable(pyhdb_rs.Date)

    def test_time_constructor_exists(self) -> None:
        """Test that Time constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "Time")
        assert callable(pyhdb_rs.Time)

    def test_timestamp_constructor_exists(self) -> None:
        """Test that Timestamp constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "Timestamp")
        assert callable(pyhdb_rs.Timestamp)

    def test_date_from_ticks_exists(self) -> None:
        """Test that DateFromTicks constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "DateFromTicks")
        assert callable(pyhdb_rs.DateFromTicks)

    def test_time_from_ticks_exists(self) -> None:
        """Test that TimeFromTicks constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "TimeFromTicks")
        assert callable(pyhdb_rs.TimeFromTicks)

    def test_timestamp_from_ticks_exists(self) -> None:
        """Test that TimestampFromTicks constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "TimestampFromTicks")
        assert callable(pyhdb_rs.TimestampFromTicks)

    def test_binary_constructor_exists(self) -> None:
        """Test that Binary constructor is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "Binary")
        assert callable(pyhdb_rs.Binary)


class TestTypeObjectsExported:
    """Tests for DB-API 2.0 type objects exports."""

    def test_string_type_exists(self) -> None:
        """Test that STRING type object is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "STRING")

    def test_binary_type_exists(self) -> None:
        """Test that BINARY type object is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "BINARY")

    def test_number_type_exists(self) -> None:
        """Test that NUMBER type object is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "NUMBER")

    def test_datetime_type_exists(self) -> None:
        """Test that DATETIME type object is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "DATETIME")

    def test_rowid_type_exists(self) -> None:
        """Test that ROWID type object is exported."""
        import pyhdb_rs

        assert hasattr(pyhdb_rs, "ROWID")


class TestAllAttribute:
    """Tests for module __all__ attribute."""

    def test_all_contains_connect(self) -> None:
        """Test that __all__ contains 'connect'."""
        import pyhdb_rs

        assert "connect" in pyhdb_rs.__all__

    def test_all_contains_connection(self) -> None:
        """Test that __all__ contains 'Connection'."""
        import pyhdb_rs

        assert "Connection" in pyhdb_rs.__all__

    def test_all_contains_cursor(self) -> None:
        """Test that __all__ contains 'Cursor'."""
        import pyhdb_rs

        assert "Cursor" in pyhdb_rs.__all__

    def test_all_contains_exceptions(self) -> None:
        """Test that __all__ contains exception classes."""
        import pyhdb_rs

        expected = [
            "Error",
            "Warning",
            "InterfaceError",
            "DatabaseError",
            "DataError",
            "OperationalError",
            "IntegrityError",
            "InternalError",
            "ProgrammingError",
            "NotSupportedError",
        ]
        for exc in expected:
            assert exc in pyhdb_rs.__all__, f"{exc} not in __all__"

    def test_all_contains_type_constructors(self) -> None:
        """Test that __all__ contains type constructors."""
        import pyhdb_rs

        expected = [
            "Date",
            "Time",
            "Timestamp",
            "DateFromTicks",
            "TimeFromTicks",
            "TimestampFromTicks",
            "Binary",
        ]
        for tc in expected:
            assert tc in pyhdb_rs.__all__, f"{tc} not in __all__"

    def test_all_contains_type_objects(self) -> None:
        """Test that __all__ contains type objects."""
        import pyhdb_rs

        expected = ["STRING", "BINARY", "NUMBER", "DATETIME", "ROWID"]
        for to in expected:
            assert to in pyhdb_rs.__all__, f"{to} not in __all__"
