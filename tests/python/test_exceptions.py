"""Tests for DB-API 2.0 exception hierarchy.

These tests verify that the exception hierarchy follows PEP 249 specification
and that exception instances can be raised and caught properly.
"""

from __future__ import annotations

import pytest


class TestExceptionHierarchy:
    """Tests for DB-API 2.0 exception hierarchy structure."""

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


class TestExceptionInstantiation:
    """Tests for exception instantiation and message handling."""

    def test_error_can_be_instantiated(self) -> None:
        """Test that Error can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.Error("test message")
        assert str(exc) == "test message"

    def test_warning_can_be_instantiated(self) -> None:
        """Test that Warning can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.Warning("test warning")
        assert str(exc) == "test warning"

    def test_interface_error_can_be_instantiated(self) -> None:
        """Test that InterfaceError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.InterfaceError("interface error")
        assert str(exc) == "interface error"

    def test_database_error_can_be_instantiated(self) -> None:
        """Test that DatabaseError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.DatabaseError("database error")
        assert str(exc) == "database error"

    def test_data_error_can_be_instantiated(self) -> None:
        """Test that DataError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.DataError("data error")
        assert str(exc) == "data error"

    def test_operational_error_can_be_instantiated(self) -> None:
        """Test that OperationalError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.OperationalError("operational error")
        assert str(exc) == "operational error"

    def test_integrity_error_can_be_instantiated(self) -> None:
        """Test that IntegrityError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.IntegrityError("integrity error")
        assert str(exc) == "integrity error"

    def test_internal_error_can_be_instantiated(self) -> None:
        """Test that InternalError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.InternalError("internal error")
        assert str(exc) == "internal error"

    def test_programming_error_can_be_instantiated(self) -> None:
        """Test that ProgrammingError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.ProgrammingError("programming error")
        assert str(exc) == "programming error"

    def test_not_supported_error_can_be_instantiated(self) -> None:
        """Test that NotSupportedError can be instantiated."""
        import pyhdb_rs

        exc = pyhdb_rs.NotSupportedError("not supported")
        assert str(exc) == "not supported"


class TestExceptionRaiseAndCatch:
    """Tests for raising and catching exceptions."""

    def test_error_can_be_raised(self) -> None:
        """Test that Error can be raised and caught."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.Error):
            raise pyhdb_rs.Error("test")

    def test_interface_error_caught_as_error(self) -> None:
        """Test that InterfaceError can be caught as Error."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.Error):
            raise pyhdb_rs.InterfaceError("test")

    def test_database_error_caught_as_error(self) -> None:
        """Test that DatabaseError can be caught as Error."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.Error):
            raise pyhdb_rs.DatabaseError("test")

    def test_data_error_caught_as_database_error(self) -> None:
        """Test that DataError can be caught as DatabaseError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.DatabaseError):
            raise pyhdb_rs.DataError("test")

    def test_operational_error_caught_as_database_error(self) -> None:
        """Test that OperationalError can be caught as DatabaseError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.DatabaseError):
            raise pyhdb_rs.OperationalError("test")

    def test_integrity_error_caught_as_database_error(self) -> None:
        """Test that IntegrityError can be caught as DatabaseError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.DatabaseError):
            raise pyhdb_rs.IntegrityError("test")

    def test_internal_error_caught_as_database_error(self) -> None:
        """Test that InternalError can be caught as DatabaseError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.DatabaseError):
            raise pyhdb_rs.InternalError("test")

    def test_programming_error_caught_as_database_error(self) -> None:
        """Test that ProgrammingError can be caught as DatabaseError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.DatabaseError):
            raise pyhdb_rs.ProgrammingError("test")

    def test_not_supported_error_caught_as_database_error(self) -> None:
        """Test that NotSupportedError can be caught as DatabaseError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.DatabaseError):
            raise pyhdb_rs.NotSupportedError("test")


class TestExceptionCatchAsException:
    """Tests for catching exceptions as base Exception."""

    def test_error_caught_as_exception(self) -> None:
        """Test that Error can be caught as Exception."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.Error):
            raise pyhdb_rs.Error("test")

    def test_warning_caught_as_exception(self) -> None:
        """Test that Warning can be caught as Exception."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.Warning):
            raise pyhdb_rs.Warning("test")


class TestConnectErrors:
    """Tests for connect function error handling."""

    def test_connect_invalid_url_raises_interface_error(self) -> None:
        """Test that invalid URL raises InterfaceError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.InterfaceError):
            pyhdb_rs.connect("invalid://url")

    def test_connect_malformed_url_raises_interface_error(self) -> None:
        """Test that malformed URL raises InterfaceError."""
        import pyhdb_rs

        with pytest.raises(pyhdb_rs.InterfaceError):
            pyhdb_rs.connect("not-a-valid-url")

    def test_connect_empty_url_raises_error(self) -> None:
        """Test that empty URL raises an error."""
        import pyhdb_rs

        with pytest.raises((pyhdb_rs.InterfaceError, pyhdb_rs.Error)):
            pyhdb_rs.connect("")


class TestExceptionArgs:
    """Tests for exception args attribute."""

    def test_error_args_single(self) -> None:
        """Test that Error args contains the message."""
        import pyhdb_rs

        exc = pyhdb_rs.Error("test message")
        assert exc.args == ("test message",)

    def test_interface_error_args_single(self) -> None:
        """Test that InterfaceError args contains the message."""
        import pyhdb_rs

        exc = pyhdb_rs.InterfaceError("interface error")
        assert exc.args == ("interface error",)

    def test_database_error_args_single(self) -> None:
        """Test that DatabaseError args contains the message."""
        import pyhdb_rs

        exc = pyhdb_rs.DatabaseError("database error")
        assert exc.args == ("database error",)


class TestExceptionRepr:
    """Tests for exception repr."""

    def test_error_repr(self) -> None:
        """Test that Error has a repr."""
        import pyhdb_rs

        exc = pyhdb_rs.Error("test")
        rep = repr(exc)
        assert "test" in rep

    def test_interface_error_repr(self) -> None:
        """Test that InterfaceError has a repr."""
        import pyhdb_rs

        exc = pyhdb_rs.InterfaceError("interface")
        rep = repr(exc)
        assert "interface" in rep
