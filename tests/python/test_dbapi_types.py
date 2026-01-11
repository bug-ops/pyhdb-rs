"""Tests for DB-API 2.0 type constructors and type objects.

These tests verify PEP 249 type constructors and type object comparison behavior.
"""

from __future__ import annotations

import datetime
import time


class TestDateConstructor:
    """Tests for Date type constructor."""

    def test_date_returns_date_object(self) -> None:
        """Test that Date returns a datetime.date object."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 6, 15)
        assert isinstance(d, datetime.date)

    def test_date_year(self) -> None:
        """Test that Date preserves year."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 6, 15)
        assert d.year == 2024

    def test_date_month(self) -> None:
        """Test that Date preserves month."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 6, 15)
        assert d.month == 6

    def test_date_day(self) -> None:
        """Test that Date preserves day."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 6, 15)
        assert d.day == 15

    def test_date_leap_year(self) -> None:
        """Test Date with leap year date."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 2, 29)
        assert d.month == 2
        assert d.day == 29

    def test_date_first_of_year(self) -> None:
        """Test Date with first day of year."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 1, 1)
        assert d == datetime.date(2024, 1, 1)

    def test_date_last_of_year(self) -> None:
        """Test Date with last day of year."""
        import pyhdb_rs

        d = pyhdb_rs.Date(2024, 12, 31)
        assert d == datetime.date(2024, 12, 31)


class TestTimeConstructor:
    """Tests for Time type constructor."""

    def test_time_returns_time_object(self) -> None:
        """Test that Time returns a datetime.time object."""
        import pyhdb_rs

        t = pyhdb_rs.Time(14, 30, 45)
        assert isinstance(t, datetime.time)

    def test_time_hour(self) -> None:
        """Test that Time preserves hour."""
        import pyhdb_rs

        t = pyhdb_rs.Time(14, 30, 45)
        assert t.hour == 14

    def test_time_minute(self) -> None:
        """Test that Time preserves minute."""
        import pyhdb_rs

        t = pyhdb_rs.Time(14, 30, 45)
        assert t.minute == 30

    def test_time_second(self) -> None:
        """Test that Time preserves second."""
        import pyhdb_rs

        t = pyhdb_rs.Time(14, 30, 45)
        assert t.second == 45

    def test_time_midnight(self) -> None:
        """Test Time with midnight."""
        import pyhdb_rs

        t = pyhdb_rs.Time(0, 0, 0)
        assert t == datetime.time(0, 0, 0)

    def test_time_end_of_day(self) -> None:
        """Test Time with end of day."""
        import pyhdb_rs

        t = pyhdb_rs.Time(23, 59, 59)
        assert t == datetime.time(23, 59, 59)


class TestTimestampConstructor:
    """Tests for Timestamp type constructor."""

    def test_timestamp_returns_datetime_object(self) -> None:
        """Test that Timestamp returns a datetime.datetime object."""
        import pyhdb_rs

        ts = pyhdb_rs.Timestamp(2024, 6, 15, 14, 30, 45)
        assert isinstance(ts, datetime.datetime)

    def test_timestamp_date_parts(self) -> None:
        """Test that Timestamp preserves date parts."""
        import pyhdb_rs

        ts = pyhdb_rs.Timestamp(2024, 6, 15, 14, 30, 45)
        assert ts.year == 2024
        assert ts.month == 6
        assert ts.day == 15

    def test_timestamp_time_parts(self) -> None:
        """Test that Timestamp preserves time parts."""
        import pyhdb_rs

        ts = pyhdb_rs.Timestamp(2024, 6, 15, 14, 30, 45)
        assert ts.hour == 14
        assert ts.minute == 30
        assert ts.second == 45

    def test_timestamp_epoch(self) -> None:
        """Test Timestamp with Unix epoch."""
        import pyhdb_rs

        ts = pyhdb_rs.Timestamp(1970, 1, 1, 0, 0, 0)
        assert ts == datetime.datetime(1970, 1, 1, 0, 0, 0)


class TestDateFromTicks:
    """Tests for DateFromTicks type constructor."""

    def test_date_from_ticks_returns_date(self) -> None:
        """Test that DateFromTicks returns a datetime.date object."""
        import pyhdb_rs

        ticks = time.time()
        d = pyhdb_rs.DateFromTicks(ticks)
        assert isinstance(d, datetime.date)

    def test_date_from_ticks_known_value(self) -> None:
        """Test DateFromTicks with a known timestamp."""
        import pyhdb_rs

        ticks = 1718470245.0
        d = pyhdb_rs.DateFromTicks(ticks)
        assert isinstance(d, datetime.date)
        assert d.year >= 2024

    def test_date_from_ticks_epoch(self) -> None:
        """Test DateFromTicks with epoch."""
        import pyhdb_rs

        d = pyhdb_rs.DateFromTicks(0.0)
        assert isinstance(d, datetime.date)


class TestTimeFromTicks:
    """Tests for TimeFromTicks type constructor."""

    def test_time_from_ticks_returns_time(self) -> None:
        """Test that TimeFromTicks returns a datetime.time object."""
        import pyhdb_rs

        ticks = time.time()
        t = pyhdb_rs.TimeFromTicks(ticks)
        assert isinstance(t, datetime.time)

    def test_time_from_ticks_known_value(self) -> None:
        """Test TimeFromTicks with a known timestamp."""
        import pyhdb_rs

        ticks = 1718470245.0
        t = pyhdb_rs.TimeFromTicks(ticks)
        assert isinstance(t, datetime.time)


class TestTimestampFromTicks:
    """Tests for TimestampFromTicks type constructor."""

    def test_timestamp_from_ticks_returns_datetime(self) -> None:
        """Test that TimestampFromTicks returns a datetime.datetime object."""
        import pyhdb_rs

        ticks = time.time()
        ts = pyhdb_rs.TimestampFromTicks(ticks)
        assert isinstance(ts, datetime.datetime)

    def test_timestamp_from_ticks_known_value(self) -> None:
        """Test TimestampFromTicks with a known timestamp."""
        import pyhdb_rs

        ticks = 1718470245.0
        ts = pyhdb_rs.TimestampFromTicks(ticks)
        assert isinstance(ts, datetime.datetime)
        assert ts.year >= 2024


class TestBinaryConstructor:
    """Tests for Binary type constructor."""

    def test_binary_returns_bytes(self) -> None:
        """Test that Binary returns a bytes object."""
        import pyhdb_rs

        b = pyhdb_rs.Binary(b"\x00\x01\x02")
        assert isinstance(b, bytes)

    def test_binary_preserves_data(self) -> None:
        """Test that Binary preserves binary data."""
        import pyhdb_rs

        b = pyhdb_rs.Binary(b"\x00\x01\x02")
        assert b == b"\x00\x01\x02"

    def test_binary_from_bytearray(self) -> None:
        """Test Binary constructor with bytearray."""
        import pyhdb_rs

        b = pyhdb_rs.Binary(bytearray([0, 1, 2]))
        assert isinstance(b, bytes)
        assert b == b"\x00\x01\x02"

    def test_binary_empty(self) -> None:
        """Test Binary with empty bytes."""
        import pyhdb_rs

        b = pyhdb_rs.Binary(b"")
        assert b == b""

    def test_binary_large(self) -> None:
        """Test Binary with larger data."""
        import pyhdb_rs

        data = bytes(range(256))
        b = pyhdb_rs.Binary(data)
        assert b == data
        assert len(b) == 256


class TestStringTypeObject:
    """Tests for STRING type object."""

    def test_string_equals_varchar_code(self) -> None:
        """Test STRING compares equal to VARCHAR type code."""
        import pyhdb_rs

        assert pyhdb_rs.STRING == 9

    def test_string_equals_nvarchar_code(self) -> None:
        """Test STRING compares equal to NVARCHAR type code."""
        import pyhdb_rs

        assert pyhdb_rs.STRING == 11

    def test_string_not_equal_int_code(self) -> None:
        """Test STRING not equal to INT type code."""
        import pyhdb_rs

        assert pyhdb_rs.STRING != 1

    def test_string_not_equal_binary_code(self) -> None:
        """Test STRING not equal to BINARY type code."""
        import pyhdb_rs

        assert pyhdb_rs.STRING != 12


class TestBinaryTypeObject:
    """Tests for BINARY type object."""

    def test_binary_equals_binary_code(self) -> None:
        """Test BINARY compares equal to BINARY type code."""
        import pyhdb_rs

        assert pyhdb_rs.BINARY == 12

    def test_binary_equals_varbinary_code(self) -> None:
        """Test BINARY compares equal to VARBINARY type code."""
        import pyhdb_rs

        assert pyhdb_rs.BINARY == 13

    def test_binary_not_equal_int_code(self) -> None:
        """Test BINARY not equal to INT type code."""
        import pyhdb_rs

        assert pyhdb_rs.BINARY != 1

    def test_binary_not_equal_string_code(self) -> None:
        """Test BINARY not equal to STRING type code."""
        import pyhdb_rs

        assert pyhdb_rs.BINARY != 9


class TestNumberTypeObject:
    """Tests for NUMBER type object."""

    def test_number_equals_int_code(self) -> None:
        """Test NUMBER compares equal to INT type code."""
        import pyhdb_rs

        assert pyhdb_rs.NUMBER == 3

    def test_number_equals_bigint_code(self) -> None:
        """Test NUMBER compares equal to BIGINT type code."""
        import pyhdb_rs

        assert pyhdb_rs.NUMBER == 4

    def test_number_not_equal_string_code(self) -> None:
        """Test NUMBER not equal to STRING type code."""
        import pyhdb_rs

        assert pyhdb_rs.NUMBER != 9


class TestDatetimeTypeObject:
    """Tests for DATETIME type object."""

    def test_datetime_equals_date_code(self) -> None:
        """Test DATETIME compares equal to DATE type code."""
        import pyhdb_rs

        assert pyhdb_rs.DATETIME == 14

    def test_datetime_equals_time_code(self) -> None:
        """Test DATETIME compares equal to TIME type code."""
        import pyhdb_rs

        assert pyhdb_rs.DATETIME == 15

    def test_datetime_not_equal_int_code(self) -> None:
        """Test DATETIME not equal to INT type code."""
        import pyhdb_rs

        assert pyhdb_rs.DATETIME != 1


class TestRowidTypeObject:
    """Tests for ROWID type object."""

    def test_rowid_equals_bigint_code(self) -> None:
        """Test ROWID compares equal to BIGINT type code."""
        import pyhdb_rs

        assert pyhdb_rs.ROWID == 4

    def test_rowid_not_equal_string_code(self) -> None:
        """Test ROWID not equal to STRING type code."""
        import pyhdb_rs

        assert pyhdb_rs.ROWID != 9


class TestTypeObjectRepr:
    """Tests for type object repr."""

    def test_string_has_repr(self) -> None:
        """Test STRING has a repr."""
        import pyhdb_rs

        rep = repr(pyhdb_rs.STRING)
        assert "TypeObject" in rep

    def test_binary_has_repr(self) -> None:
        """Test BINARY has a repr."""
        import pyhdb_rs

        rep = repr(pyhdb_rs.BINARY)
        assert "TypeObject" in rep

    def test_number_has_repr(self) -> None:
        """Test NUMBER has a repr."""
        import pyhdb_rs

        rep = repr(pyhdb_rs.NUMBER)
        assert "TypeObject" in rep

    def test_datetime_has_repr(self) -> None:
        """Test DATETIME has a repr."""
        import pyhdb_rs

        rep = repr(pyhdb_rs.DATETIME)
        assert "TypeObject" in rep

    def test_rowid_has_repr(self) -> None:
        """Test ROWID has a repr."""
        import pyhdb_rs

        rep = repr(pyhdb_rs.ROWID)
        assert "TypeObject" in rep


class TestTypeObjectHash:
    """Tests for type object hash."""

    def test_string_is_hashable(self) -> None:
        """Test STRING is hashable."""
        import pyhdb_rs

        h = hash(pyhdb_rs.STRING)
        assert isinstance(h, int)

    def test_binary_is_hashable(self) -> None:
        """Test BINARY is hashable."""
        import pyhdb_rs

        h = hash(pyhdb_rs.BINARY)
        assert isinstance(h, int)

    def test_type_objects_can_be_set_members(self) -> None:
        """Test type objects can be added to sets."""
        import pyhdb_rs

        s = {pyhdb_rs.STRING, pyhdb_rs.BINARY, pyhdb_rs.NUMBER}
        assert len(s) == 3

    def test_type_objects_can_be_dict_keys(self) -> None:
        """Test type objects can be used as dict keys."""
        import pyhdb_rs

        d = {
            pyhdb_rs.STRING: "string",
            pyhdb_rs.BINARY: "binary",
            pyhdb_rs.NUMBER: "number",
        }
        assert len(d) == 3
        assert d[pyhdb_rs.STRING] == "string"


class TestTypeObjectNotImplemented:
    """Tests for type object comparison with incompatible types."""

    def test_string_equality_with_string_returns_not_implemented(self) -> None:
        """Test STRING == string returns NotImplemented (via False)."""
        import pyhdb_rs

        result = pyhdb_rs.STRING == "not an int"
        assert result is NotImplemented or result is False

    def test_string_equality_with_none(self) -> None:
        """Test STRING == None returns NotImplemented (via False)."""
        import pyhdb_rs

        result = pyhdb_rs.STRING == None  # noqa: E711
        assert result is NotImplemented or result is False

    def test_string_inequality_with_string(self) -> None:
        """Test STRING != string."""
        import pyhdb_rs

        result = pyhdb_rs.STRING != "not an int"
        assert result is NotImplemented or result is True
