"""DB-API 2.0 type constructors and type objects.

This module provides the standard type constructors required by PEP 249:
- Date, Time, Timestamp: Construct date/time objects
- DateFromTicks, TimeFromTicks, TimestampFromTicks: From Unix timestamps
- Binary: Construct binary objects
- STRING, BINARY, NUMBER, DATETIME, ROWID: Type objects for description
"""

from __future__ import annotations

import datetime
from typing import Final

__all__ = [
    "Date",
    "Time",
    "Timestamp",
    "DateFromTicks",
    "TimeFromTicks",
    "TimestampFromTicks",
    "Binary",
    "STRING",
    "BINARY",
    "NUMBER",
    "DATETIME",
    "ROWID",
]


# =====================================================================
# Type Constructors (PEP 249 Section)
# =====================================================================


def Date(year: int, month: int, day: int) -> datetime.date:
    """Construct a date value.

    Args:
        year: Year (e.g., 2024)
        month: Month (1-12)
        day: Day (1-31)

    Returns:
        datetime.date object
    """
    return datetime.date(year, month, day)


def Time(hour: int, minute: int, second: int) -> datetime.time:
    """Construct a time value.

    Args:
        hour: Hour (0-23)
        minute: Minute (0-59)
        second: Second (0-59)

    Returns:
        datetime.time object
    """
    return datetime.time(hour, minute, second)


def Timestamp(
    year: int,
    month: int,
    day: int,
    hour: int,
    minute: int,
    second: int,
) -> datetime.datetime:
    """Construct a timestamp value.

    Args:
        year: Year
        month: Month (1-12)
        day: Day (1-31)
        hour: Hour (0-23)
        minute: Minute (0-59)
        second: Second (0-59)

    Returns:
        datetime.datetime object
    """
    return datetime.datetime(year, month, day, hour, minute, second)


def DateFromTicks(ticks: float) -> datetime.date:
    """Construct a date from Unix timestamp.

    Args:
        ticks: Unix timestamp (seconds since epoch)

    Returns:
        datetime.date object
    """
    return datetime.date.fromtimestamp(ticks)


def TimeFromTicks(ticks: float) -> datetime.time:
    """Construct a time from Unix timestamp.

    Args:
        ticks: Unix timestamp (seconds since epoch)

    Returns:
        datetime.time object
    """
    return datetime.datetime.fromtimestamp(ticks).time()


def TimestampFromTicks(ticks: float) -> datetime.datetime:
    """Construct a timestamp from Unix timestamp.

    Args:
        ticks: Unix timestamp (seconds since epoch)

    Returns:
        datetime.datetime object
    """
    return datetime.datetime.fromtimestamp(ticks)


def Binary(data: bytes | bytearray | memoryview) -> bytes:
    """Construct a binary value.

    Args:
        data: Binary data

    Returns:
        bytes object
    """
    return bytes(data)


# =====================================================================
# Type Objects (PEP 249)
# =====================================================================


class _TypeObject:
    """Type object for comparing cursor.description type_code.

    Used to check if a column is of a certain type category.
    """

    __slots__ = ("_values",)

    def __init__(self, *values: int) -> None:
        self._values = frozenset(values)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, int):
            return other in self._values
        return NotImplemented

    def __ne__(self, other: object) -> bool:
        if isinstance(other, int):
            return other not in self._values
        return NotImplemented

    def __hash__(self) -> int:
        return hash(self._values)

    def __repr__(self) -> str:
        return f"TypeObject({sorted(self._values)})"


# HANA type IDs mapped to DB-API type objects
# Based on hdbconnect TypeId enum values

# String types: CHAR, VARCHAR, NCHAR, NVARCHAR, CLOB, NCLOB, TEXT, SHORTTEXT, ALPHANUM, STRING
STRING: Final[_TypeObject] = _TypeObject(8, 9, 10, 11, 25, 26, 51, 52, 55, 29)

# Binary types: BINARY, VARBINARY, BLOB
BINARY: Final[_TypeObject] = _TypeObject(12, 13, 27)

# Numeric types: TINYINT, SMALLINT, INT, BIGINT, REAL, DOUBLE, DECIMAL, SMALLDECIMAL
NUMBER: Final[_TypeObject] = _TypeObject(1, 2, 3, 4, 6, 7, 5, 47)

# Date/time types: DATE, TIME, TIMESTAMP, SECONDDATE, DAYDATE, SECONDTIME, LONGDATE
DATETIME: Final[_TypeObject] = _TypeObject(14, 15, 16, 61, 62, 63, 64)

# Row ID type (HANA doesn't have explicit ROWID, use BIGINT as proxy)
ROWID: Final[_TypeObject] = _TypeObject(4)
