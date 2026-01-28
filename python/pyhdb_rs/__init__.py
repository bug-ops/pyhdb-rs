"""pyhdb_rs - High-performance Python driver for SAP HANA.

A Rust-based driver providing:
- Full DB-API 2.0 compliance (PEP 249)
- Native Apache Arrow support for zero-copy data transfer
- Direct Polars/pandas integration
- Thread-safe connection sharing

Basic usage::

    import pyhdb_rs

    conn = pyhdb_rs.connect("hdbsql://user:pass@host:39017")
    with conn.cursor() as cursor:
        cursor.execute("SELECT * FROM SALES_ITEMS")
        for row in cursor:
            print(row)
    conn.close()

Polars integration::

    import polars as pl
    reader = conn.execute_arrow("SELECT * FROM SALES_ITEMS")
    df = pl.from_arrow(reader)  # Zero-copy via Arrow PyCapsule

Pandas integration::

    import pyarrow as pa
    reader = conn.execute_arrow("SELECT * FROM SALES_ITEMS")
    pa_reader = pa.RecordBatchReader.from_stream(reader)
    df = pa_reader.read_all().to_pandas()
"""

from __future__ import annotations

# Import from Rust extension module
from pyhdb_rs._core import (
    # Classes
    Connection,
    Cursor,
    DatabaseError,
    DataError,
    # Exceptions
    Error,
    IntegrityError,
    InterfaceError,
    InternalError,
    NotSupportedError,
    OperationalError,
    ProgrammingError,
    RecordBatchReader,
    Warning,
    # Version
    __version__,
    # DB-API 2.0 attributes
    apilevel,
    # Module-level function
    connect,
    paramstyle,
    threadsafety,
)

# DB-API 2.0 type constructors
from pyhdb_rs.dbapi import (
    BINARY,
    DATETIME,
    NUMBER,
    ROWID,
    STRING,
    Binary,
    Date,
    DateFromTicks,
    Time,
    TimeFromTicks,
    Timestamp,
    TimestampFromTicks,
)

# Import async availability flag
try:
    from pyhdb_rs._core import ASYNC_AVAILABLE
except ImportError:
    ASYNC_AVAILABLE = False

__all__ = [
    # Connection
    "connect",
    "Connection",
    "Cursor",
    "RecordBatchReader",
    # Module attributes
    "apilevel",
    "threadsafety",
    "paramstyle",
    "__version__",
    # Async availability
    "ASYNC_AVAILABLE",
    # Exceptions
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
    # Type constructors
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
