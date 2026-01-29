"""pyhdb_rs - High-performance Python driver for SAP HANA.

A Rust-based driver providing:
- Full DB-API 2.0 compliance (PEP 249)
- Native Apache Arrow support for zero-copy data transfer
- Thread-safe connection sharing

Basic usage::

    from pyhdb_rs import ConnectionBuilder
    import polars as pl

    conn = ConnectionBuilder.from_url("hdbsql://user:pass@host:39017").build()
    reader = conn.execute_arrow("SELECT * FROM SALES_ITEMS")
    df = pl.from_arrow(reader)
    conn.close()

Builder-based connection with TLS::

    from pyhdb_rs import TlsConfig, ConnectionBuilder

    conn = (ConnectionBuilder()
        .host("hana.example.com")
        .port(30015)
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.from_directory("/path/to/certs"))
        .build())

Polars integration::

    import polars as pl
    reader = conn.execute_arrow("SELECT * FROM SALES_ITEMS")
    df = pl.from_arrow(reader)  # Zero-copy via Arrow PyCapsule

pandas integration::

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
    ConnectionBuilder,
    ConnectionConfig,
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
    TlsConfig,
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
    "ConnectionBuilder",
    "ConnectionConfig",
    "Cursor",
    "RecordBatchReader",
    "TlsConfig",
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
