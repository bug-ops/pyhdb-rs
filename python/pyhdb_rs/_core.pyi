"""Type stubs for the pyhdb_rs Rust extension module.

This file provides type hints for IDE support and static analysis.
"""

from __future__ import annotations

from collections.abc import Awaitable, Iterator, Sequence
from enum import IntEnum
from types import TracebackType
from typing import Any, Literal, Self

# =====================================================================
# DB-API 2.0 Module Attributes
# =====================================================================

apilevel: Literal["2.0"]
"""DB-API 2.0 compliance level."""

threadsafety: Literal[2]
"""Thread safety level: connections can be shared between threads."""

paramstyle: Literal["qmark"]
"""Parameter marker style: ? (question mark)."""

__version__: str
"""Package version string."""

# =====================================================================
# CursorHoldability
# =====================================================================

class CursorHoldability(IntEnum):
    """Controls result set behavior across transaction boundaries.

    Determines whether cursors remain open after COMMIT or ROLLBACK operations.
    This affects how result sets behave in transaction-heavy applications.

    Example::

        from pyhdb_rs import ConnectionBuilder, CursorHoldability

        conn = (ConnectionBuilder()
            .host("hana.example.com")
            .credentials("SYSTEM", "password")
            .cursor_holdability(CursorHoldability.CommitAndRollback)
            .build())
    """

    # Cursor closed on commit and rollback (default).
    None_ = 0
    # Cursor held across commits.
    Commit = 1
    # Cursor held across rollbacks.
    Rollback = 2
    # Cursor held across both commit and rollback.
    CommitAndRollback = 3

# =====================================================================
# TlsConfig
# =====================================================================

class TlsConfig:
    """TLS configuration for secure SAP HANA connections.

    Choose ONE of the factory methods to specify the certificate source.
    This class is immutable once created.

    Example::

        # Load certificates from a directory
        tls = TlsConfig.from_directory("/etc/hana/certs")

        # Load from environment variable
        tls = TlsConfig.from_environment("HANA_CA_CERT")

        # Use certificate content directly
        with open("ca.pem") as f:
            tls = TlsConfig.from_certificate(f.read())

        # Use system root certificates
        tls = TlsConfig.with_system_roots()

        # Development only: skip verification (INSECURE)
        tls = TlsConfig.insecure()
    """

    @classmethod
    def from_directory(cls, path: str) -> TlsConfig:
        """Load server certificates from PEM files in a directory.

        The directory should contain one or more `.pem` files with CA certificates.

        Args:
            path: Path to directory containing PEM certificate files.

        Returns:
            TlsConfig configured to use certificates from the directory.

        Example::

            tls = TlsConfig.from_directory("/etc/hana/certs")
        """
        ...

    @classmethod
    def from_environment(cls, env_var: str) -> TlsConfig:
        """Load server certificate from an environment variable.

        The environment variable should contain the PEM-encoded certificate content.

        Args:
            env_var: Name of environment variable containing PEM certificate.

        Returns:
            TlsConfig configured to read certificate from environment.

        Example::

            # Set HANA_CA_CERT="-----BEGIN CERTIFICATE-----..."
            tls = TlsConfig.from_environment("HANA_CA_CERT")
        """
        ...

    @classmethod
    def from_certificate(cls, pem_content: str) -> TlsConfig:
        """Use certificate directly from a PEM string.

        Args:
            pem_content: PEM-encoded certificate content.

        Returns:
            TlsConfig configured with the provided certificate.

        Example::

            with open("ca.pem") as f:
                tls = TlsConfig.from_certificate(f.read())
        """
        ...

    @classmethod
    def with_system_roots(cls) -> TlsConfig:
        """Use system root certificates (webpki-roots / mkcert.org).

        This uses the bundled Mozilla root certificates, suitable for
        connections to HANA instances with certificates signed by
        well-known certificate authorities.

        Returns:
            TlsConfig configured to use system root certificates.

        Example::

            tls = TlsConfig.with_system_roots()
        """
        ...

    @classmethod
    def insecure(cls) -> TlsConfig:
        """TLS without server certificate verification (INSECURE).

        Warning:
            This disables server certificate verification and should
            only be used for development/testing with self-signed certificates.
            Never use in production.

        Returns:
            TlsConfig configured for insecure TLS.

        Example::

            # Development only!
            tls = TlsConfig.insecure()
        """
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# ConnectionBuilder
# =====================================================================

class ConnectionBuilder:
    """Builder for sync SAP HANA connections with TLS support.

    Use this builder when you need:
    - TLS configuration with custom certificates
    - Programmatic connection parameters
    - Fine-grained control over connection settings

    For simple URL-based connections, use `connect()` function instead.

    Example::

        # Simple connection
        conn = (ConnectionBuilder()
            .host("hana.example.com")
            .port(30015)
            .credentials("SYSTEM", "password")
            .build())

        # TLS with custom certificates
        conn = (ConnectionBuilder()
            .host("hana.example.com")
            .credentials("SYSTEM", "password")
            .tls(TlsConfig.from_directory("/path/to/certs"))
            .build())

        # From URL with TLS override
        conn = (ConnectionBuilder.from_url("hdbsql://user:pass@host:30015")
            .tls(TlsConfig.from_certificate(cert_pem))
            .build())
    """

    def __init__(self) -> None:
        """Create a new connection builder with default settings.

        Default port is 30015 (SAP HANA standard port).
        """
        ...

    @classmethod
    def from_url(cls, url: str) -> ConnectionBuilder:
        """Create builder from a connection URL.

        The URL provides initial values that can be overridden with builder methods.
        If the URL scheme is `hdbsqls://`, TLS with system roots is automatically enabled.

        Args:
            url: Connection URL in format `hdbsql://user:pass@host:port[/database]`

        Returns:
            ConnectionBuilder initialized with URL values.

        Raises:
            InterfaceError: If URL is invalid or missing required components.

        Example::

            # Parse URL, then override TLS
            builder = (ConnectionBuilder.from_url("hdbsql://user:pass@host:30015")
                .tls(TlsConfig.from_certificate(cert_pem)))
        """
        ...

    def host(self, hostname: str) -> Self:
        """Set the database host.

        Args:
            hostname: Database server hostname or IP address.

        Returns:
            Self for method chaining.
        """
        ...

    def port(self, port: int) -> Self:
        """Set the database port.

        Args:
            port: Database port (default: 30015).

        Returns:
            Self for method chaining.
        """
        ...

    def credentials(self, user: str, password: str) -> Self:
        """Set authentication credentials.

        Args:
            user: Database username.
            password: Database password.

        Returns:
            Self for method chaining.
        """
        ...

    def database(self, name: str) -> Self:
        """Set the database/tenant name.

        Args:
            name: Database or tenant name.

        Returns:
            Self for method chaining.
        """
        ...

    def tls(self, config: TlsConfig) -> Self:
        """Configure TLS for secure connection.

        Args:
            config: TLS configuration (use TlsConfig factory methods).

        Returns:
            Self for method chaining.

        Example::

            builder.tls(TlsConfig.from_directory("/path/to/certs"))
            builder.tls(TlsConfig.with_system_roots())
            builder.tls(TlsConfig.insecure())  # Development only!
        """
        ...

    def config(self, config: ConnectionConfig) -> Self:
        """Apply connection configuration (fetch_size, timeouts, etc.).

        Args:
            config: Connection configuration.

        Returns:
            Self for method chaining.

        Example::

            config = ConnectionConfig(fetch_size=50000, read_timeout=60.0)
            builder.config(config)
        """
        ...

    def cursor_holdability(self, holdability: CursorHoldability) -> Self:
        """Set cursor holdability for transaction behavior.

        Controls whether result set cursors remain open after COMMIT
        or ROLLBACK operations.

        Args:
            holdability: Cursor holdability setting.

        Returns:
            Self for method chaining.

        Example::

            builder.cursor_holdability(CursorHoldability.CommitAndRollback)
        """
        ...

    def network_group(self, group: str) -> Self:
        """Set network group for HANA Scale-Out/HA deployments.

        Specifies which network interface to use when connecting
        to HANA systems with multiple network configurations.

        Args:
            group: Network group name.

        Returns:
            Self for method chaining.

        Example::

            builder.network_group("internal")
        """
        ...

    def build(self) -> Connection:
        """Build and connect synchronously.

        Returns:
            Connection object.

        Raises:
            InterfaceError: If required parameters (host, credentials) not set.
            OperationalError: If connection fails.

        Example::

            conn = (ConnectionBuilder()
                .host("localhost")
                .credentials("user", "pass")
                .build())
        """
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# AsyncConnectionBuilder
# =====================================================================

class AsyncConnectionBuilder:
    """Builder for async SAP HANA connections with TLS support.

    Same API as `ConnectionBuilder` but produces async connections.

    Example::

        conn = await (AsyncConnectionBuilder()
            .host("hana.example.com")
            .credentials("SYSTEM", "password")
            .tls(TlsConfig.with_system_roots())
            .build())
    """

    def __init__(self) -> None:
        """Create a new async connection builder with default settings.

        Default port is 30015 (SAP HANA standard port).
        Autocommit is enabled by default.
        """
        ...

    @classmethod
    def from_url(cls, url: str) -> AsyncConnectionBuilder:
        """Create builder from a connection URL.

        The URL provides initial values that can be overridden with builder methods.
        If the URL scheme is `hdbsqls://`, TLS with system roots is automatically enabled.

        Args:
            url: Connection URL in format `hdbsql://user:pass@host:port[/database]`

        Returns:
            AsyncConnectionBuilder initialized with URL values.

        Raises:
            InterfaceError: If URL is invalid or missing required components.
        """
        ...

    def host(self, hostname: str) -> Self:
        """Set the database host.

        Args:
            hostname: Database server hostname or IP address.

        Returns:
            Self for method chaining.
        """
        ...

    def port(self, port: int) -> Self:
        """Set the database port.

        Args:
            port: Database port (default: 30015).

        Returns:
            Self for method chaining.
        """
        ...

    def credentials(self, user: str, password: str) -> Self:
        """Set authentication credentials.

        Args:
            user: Database username.
            password: Database password.

        Returns:
            Self for method chaining.
        """
        ...

    def database(self, name: str) -> Self:
        """Set the database/tenant name.

        Args:
            name: Database or tenant name.

        Returns:
            Self for method chaining.
        """
        ...

    def tls(self, config: TlsConfig) -> Self:
        """Configure TLS for secure connection.

        Args:
            config: TLS configuration (use TlsConfig factory methods).

        Returns:
            Self for method chaining.
        """
        ...

    def config(self, config: ConnectionConfig) -> Self:
        """Apply connection configuration (fetch_size, timeouts, etc.).

        Args:
            config: Connection configuration.

        Returns:
            Self for method chaining.
        """
        ...

    def autocommit(self, enabled: bool) -> Self:
        """Set auto-commit mode (default: True).

        Args:
            enabled: Whether to enable auto-commit.

        Returns:
            Self for method chaining.
        """
        ...

    def cursor_holdability(self, holdability: CursorHoldability) -> Self:
        """Set cursor holdability for transaction behavior.

        Controls whether result set cursors remain open after COMMIT
        or ROLLBACK operations.

        Args:
            holdability: Cursor holdability setting.

        Returns:
            Self for method chaining.

        Example::

            builder.cursor_holdability(CursorHoldability.CommitAndRollback)
        """
        ...

    def network_group(self, group: str) -> Self:
        """Set network group for HANA Scale-Out/HA deployments.

        Specifies which network interface to use when connecting
        to HANA systems with multiple network configurations.

        Args:
            group: Network group name.

        Returns:
            Self for method chaining.

        Example::

            builder.network_group("internal")
        """
        ...

    def build(self) -> Awaitable[AsyncConnection]:
        """Build and connect asynchronously.

        Returns:
            Awaitable that resolves to AsyncConnection.

        Raises:
            InterfaceError: If required parameters (host, credentials) not set.
            OperationalError: If connection fails.

        Example::

            conn = await (AsyncConnectionBuilder()
                .host("localhost")
                .credentials("user", "pass")
                .build())
        """
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# ArrowConfig
# =====================================================================

class ArrowConfig:
    """Configuration for Arrow-based query execution.

    Controls batch processing behavior for execute_arrow() methods.

    Example::

        from pyhdb_rs import ArrowConfig

        # Custom batch size for large result sets
        config = ArrowConfig(batch_size=10000)
        reader = conn.execute_arrow("SELECT * FROM T", config=config)
    """

    DEFAULT_BATCH_SIZE: int
    """Default batch size for Arrow conversions (65536 rows)."""

    def __init__(self, batch_size: int = 65536) -> None:
        """Create Arrow configuration.

        Args:
            batch_size: Number of rows per Arrow batch (default: 65536).
                Higher values reduce overhead but increase memory usage per batch.
                Recommended range: 1,000 - 100,000.

        Raises:
            ProgrammingError: If batch_size is 0.
        """
        ...

    @property
    def batch_size(self) -> int:
        """Number of rows per Arrow batch."""
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# ConnectionConfig
# =====================================================================

class ConnectionConfig:
    """Configuration for SAP HANA connection tuning.

    Use with connect() or ConnectionPool() to customize connection behavior.
    All parameters have sensible defaults matching hdbconnect behavior.

    Example::

        config = ConnectionConfig(
            fetch_size=50000,           # Larger batches for bulk reads
            lob_read_length=10_000_000, # 10MB LOB chunks
            read_timeout=60.0,          # 60 second timeout
        )
        conn = connect("hdbsql://...", config=config)
    """

    DEFAULT_FETCH_SIZE: int
    """Default fetch size (rows per network round-trip)."""

    DEFAULT_LOB_READ_LENGTH: int
    """Default LOB read length in bytes."""

    DEFAULT_LOB_WRITE_LENGTH: int
    """Default LOB write length in bytes."""

    DEFAULT_MAX_BUFFER_SIZE: int
    """Default maximum buffer size."""

    DEFAULT_MIN_COMPRESSION_SIZE: int
    """Default minimum compression size threshold."""

    MIN_BUFFER_SIZE: int
    """Minimum buffer size (cannot go below this)."""

    DEFAULT_CACHE_CAPACITY: int
    """Default prepared statement cache size."""

    def __init__(
        self,
        *,
        fetch_size: int | None = None,
        lob_read_length: int | None = None,
        lob_write_length: int | None = None,
        max_buffer_size: int | None = None,
        min_compression_size: int | None = None,
        read_timeout: float | None = None,
        max_cached_statements: int | None = None,
    ) -> None:
        """Create connection configuration.

        Args:
            fetch_size: Rows fetched per network round-trip (default: 10,000).
                Higher values reduce round-trips but increase memory.
                Recommended range: 1,000 - 100,000.
            lob_read_length: Bytes (or chars for NCLOB) per LOB read (default: ~16MB).
                Controls LOB fetch chunk size.
            lob_write_length: Bytes per LOB write (default: ~16MB).
                Controls LOB upload chunk size.
            max_buffer_size: Max connection buffer size in bytes (default: 128KB).
                Oversized buffers shrink back to this after use.
            min_compression_size: Threshold for request compression (default: 400 bytes).
                Requests larger than this may be compressed.
            read_timeout: Network read timeout in seconds (default: None = no timeout).
                Connection dropped if response takes longer.
            max_cached_statements: Maximum prepared statements to cache per connection
                (default: 16). Set to 0 to disable caching.

        Raises:
            ProgrammingError: If any parameter is invalid.
        """
        ...

    @property
    def fetch_size(self) -> int | None:
        """Rows fetched per network round-trip (None = use default)."""
        ...

    @property
    def lob_read_length(self) -> int | None:
        """Bytes per LOB read (None = use default)."""
        ...

    @property
    def lob_write_length(self) -> int | None:
        """Bytes per LOB write (None = use default)."""
        ...

    @property
    def max_buffer_size(self) -> int | None:
        """Max connection buffer size in bytes (None = use default)."""
        ...

    @property
    def min_compression_size(self) -> int | None:
        """Threshold for request compression (None = use default)."""
        ...

    @property
    def read_timeout(self) -> float | None:
        """Network read timeout in seconds (None = no timeout)."""
        ...

    @property
    def max_cached_statements(self) -> int | None:
        """Maximum prepared statements to cache per connection (None = use default)."""
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# CacheStats
# =====================================================================

class CacheStats:
    """Prepared statement cache statistics.

    Example::

        stats = conn.cache_stats()
        print(f"Cache hit rate: {stats.hit_rate:.2%}")
        print(f"Size: {stats.size}/{stats.capacity}")
    """

    @property
    def size(self) -> int:
        """Current number of cached statements."""
        ...

    @property
    def capacity(self) -> int:
        """Maximum cache capacity."""
        ...

    @property
    def hits(self) -> int:
        """Total number of cache hits."""
        ...

    @property
    def misses(self) -> int:
        """Total number of cache misses."""
        ...

    @property
    def evictions(self) -> int:
        """Total number of evictions."""
        ...

    @property
    def hit_rate(self) -> float:
        """Cache hit rate (0.0 - 1.0)."""
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# Connection
# =====================================================================

class Connection:
    """SAP HANA database connection.

    Thread-safe connection object supporting DB-API 2.0 operations
    and Arrow-based data transfer.

    Example::

        conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")
        cursor = conn.cursor()
        cursor.execute("SELECT * FROM DUMMY")
        print(cursor.fetchone())
        conn.close()
    """

    @property
    def autocommit(self) -> bool:
        """Auto-commit mode (default: True)."""
        ...

    @autocommit.setter
    def autocommit(self, value: bool) -> None: ...
    @property
    def is_connected(self) -> bool:
        """Check if connection is open."""
        ...

    @property
    def fetch_size(self) -> int:
        """Current fetch size (rows per network round-trip)."""
        ...

    @fetch_size.setter
    def fetch_size(self, value: int) -> None:
        """Set fetch size at runtime.

        Raises:
            ProgrammingError: If value is 0
            OperationalError: If connection is closed
        """
        ...

    @property
    def read_timeout(self) -> float | None:
        """Current read timeout in seconds (None = no timeout)."""
        ...

    @read_timeout.setter
    def read_timeout(self, value: float | None) -> None:
        """Set read timeout at runtime.

        Args:
            value: Timeout in seconds, or None to disable

        Raises:
            ProgrammingError: If value is negative
            OperationalError: If connection is closed
        """
        ...

    @property
    def lob_read_length(self) -> int:
        """Current LOB read length."""
        ...

    @lob_read_length.setter
    def lob_read_length(self, value: int) -> None:
        """Set LOB read length at runtime.

        Raises:
            ProgrammingError: If value is 0
            OperationalError: If connection is closed
        """
        ...

    @property
    def lob_write_length(self) -> int:
        """Current LOB write length."""
        ...

    @lob_write_length.setter
    def lob_write_length(self, value: int) -> None:
        """Set LOB write length at runtime.

        Raises:
            ProgrammingError: If value is 0
            OperationalError: If connection is closed
        """
        ...

    def __init__(self, url: str) -> None:
        """Create a new connection.

        Args:
            url: Connection URL in format hdbsql://user:pass@host:port

        Raises:
            InterfaceError: If URL is invalid
            OperationalError: If connection fails
        """
        ...

    def cursor(self) -> Cursor:
        """Create a new cursor object."""
        ...

    def commit(self) -> None:
        """Commit the current transaction."""
        ...

    def rollback(self) -> None:
        """Rollback the current transaction."""
        ...

    def close(self) -> None:
        """Close the connection."""
        ...

    def is_valid(self, check_connection: bool = True) -> bool:
        """Check if connection is valid.

        Args:
            check_connection: If True (default), executes SELECT 1 FROM DUMMY
                to verify the connection is alive.

        Returns:
            True if connection is valid, False otherwise.
        """
        ...

    def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader:
        """Execute query and return Arrow RecordBatchReader.

        This is the high-performance path for analytics workloads.
        Data is transferred zero-copy to Python Arrow/Polars.

        Args:
            sql: SQL query string
            config: Optional Arrow configuration (batch_size, etc.)

        Returns:
            RecordBatchReader for streaming Arrow results

        Example::

            import polars as pl

            # With default config
            reader = conn.execute_arrow("SELECT * FROM sales")
            df = pl.from_arrow(reader)

            # With custom batch size
            config = ArrowConfig(batch_size=10000)
            reader = conn.execute_arrow("SELECT * FROM sales", config=config)
        """
        ...

    def cache_stats(self) -> CacheStats:
        """Get prepared statement cache statistics.

        Returns:
            CacheStats with size, capacity, hits, misses, evictions, hit_rate

        Example::

            stats = conn.cache_stats()
            print(f"Cache hit rate: {stats.hit_rate:.2%}")
        """
        ...

    def clear_cache(self) -> None:
        """Clear the prepared statement cache.

        Drops all cached prepared statements. Useful after schema changes
        or to free server resources.
        """
        ...

    def __enter__(self) -> Connection: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    def __repr__(self) -> str: ...

# =====================================================================
# Cursor
# =====================================================================

class Cursor:
    """Database cursor for executing queries.

    DB-API 2.0 compliant cursor with Arrow extensions.

    Example::

        cursor = conn.cursor()
        cursor.execute("SELECT id, name FROM users WHERE active = ?", (True,))
        for row in cursor:
            print(row)
    """

    @property
    def description(
        self,
    ) -> list[tuple[str, int, None, int | None, int | None, int | None, bool]] | None:
        """Column descriptions from the last query.

        Returns a list of 7-tuples:
        (name, type_code, display_size, internal_size, precision, scale, null_ok)
        """
        ...

    @property
    def rowcount(self) -> int:
        """Number of rows affected by the last DML operation.

        Returns -1 for SELECT statements.
        """
        ...

    @property
    def arraysize(self) -> int:
        """Number of rows to fetch with fetchmany()."""
        ...

    @arraysize.setter
    def arraysize(self, value: int) -> None: ...
    def execute(
        self,
        sql: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
    ) -> None:
        """Execute a SQL query.

        Args:
            sql: SQL statement with ? placeholders for parameters
            parameters: Optional parameters for parameterized queries (sequence or dict)

        Raises:
            ProgrammingError: If SQL syntax is invalid
            OperationalError: If connection is closed
            DataError: If parameter types are invalid

        Example::

            cursor.execute("SELECT * FROM users WHERE id = ?", (123,))
            cursor.execute("INSERT INTO users (name, age) VALUES (?, ?)", ("Alice", 30))
        """
        ...

    def executemany(
        self,
        sql: str,
        seq_of_parameters: Sequence[Sequence[Any]] | None = None,
    ) -> None:
        """Execute a DML statement with multiple parameter sets.

        Args:
            sql: SQL statement with ? placeholders for parameters
            seq_of_parameters: Sequence of parameter sequences for batch operations

        Raises:
            ProgrammingError: If SQL syntax is invalid
            OperationalError: If connection is closed
            DataError: If parameter types are invalid

        Example::

            # Batch insert multiple rows
            cursor.executemany(
                "INSERT INTO users (name, age) VALUES (?, ?)",
                [
                    ("Alice", 30),
                    ("Bob", 25),
                    ("Charlie", 35),
                ]
            )
        """
        ...

    def fetchone(self) -> tuple[Any, ...] | None:
        """Fetch the next row from the result set.

        Returns:
            Single row as tuple, or None if no more rows
        """
        ...

    def fetchmany(self, size: int | None = None) -> list[tuple[Any, ...]]:
        """Fetch multiple rows from the result set.

        Args:
            size: Number of rows to fetch (defaults to arraysize)

        Returns:
            List of rows as tuples
        """
        ...

    def fetchall(self) -> list[tuple[Any, ...]]:
        """Fetch all remaining rows from the result set.

        Returns:
            List of all remaining rows as tuples
        """
        ...

    def close(self) -> None:
        """Close the cursor and release resources."""
        ...

    def fetch_arrow(self, config: ArrowConfig | None = None) -> RecordBatchReader:
        """Fetch remaining results as Arrow RecordBatchReader.

        Consumes the result set for zero-copy Arrow transfer.

        Args:
            config: Optional Arrow configuration (batch_size, etc.)

        Returns:
            RecordBatchReader

        Raises:
            ProgrammingError: If no active result set
        """
        ...

    def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader:
        """Execute query and return Arrow RecordBatchReader.

        This is the high-performance path for analytics workloads.
        Data is transferred zero-copy to Python Arrow/Polars.

        Args:
            sql: SQL query string
            config: Optional Arrow configuration (batch_size, etc.)

        Returns:
            RecordBatchReader for streaming Arrow results

        Example::

            import polars as pl

            # With default config
            reader = cursor.execute_arrow("SELECT * FROM sales")
            df = pl.from_arrow(reader)

            # With custom batch size
            config = ArrowConfig(batch_size=10000)
            reader = cursor.execute_arrow("SELECT * FROM sales", config=config)
        """
        ...

    def __iter__(self) -> Iterator[tuple[Any, ...]]: ...
    def __next__(self) -> tuple[Any, ...]: ...
    def __enter__(self) -> Cursor: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    def __repr__(self) -> str: ...

# =====================================================================
# RecordBatchReader
# =====================================================================

class RecordBatchReader:
    """Arrow RecordBatch reader with PyCapsule interface.

    Implements __arrow_c_stream__ for zero-copy data transfer
    to Polars, PyArrow, pandas, and other Arrow-compatible libraries.

    Example::

        import polars as pl

        reader = conn.execute_arrow("SELECT * FROM sales")
        df = pl.from_arrow(reader)  # Zero-copy!
    """

    def to_pyarrow(self) -> Any:
        """Export to PyArrow RecordBatchReader.

        Consumes this reader.

        Returns:
            pyarrow.RecordBatchReader
        """
        ...

    def schema(self) -> Any:
        """Get the Arrow schema.

        Returns:
            pyarrow.Schema
        """
        ...

    def __arrow_c_stream__(self, requested_schema: int | None = None) -> object:
        """Export via Arrow PyCapsule interface (zero-copy).

        Implements the Arrow PyCapsule Interface for seamless integration
        with Polars, PyArrow, pandas, and other Arrow-compatible libraries.

        Note:
            This method consumes the reader. After calling, the reader
            cannot be used again.

        Args:
            requested_schema: Optional schema capsule for cast request.
                Most consumers pass None.

        Returns:
            PyCapsule containing ArrowArrayStream pointer.

        Raises:
            ProgrammingError: If reader was already consumed.

        Example::

            import polars as pl

            reader = conn.execute_arrow("SELECT * FROM sales")
            df = pl.from_arrow(reader)  # Uses __arrow_c_stream__ internally

            # Or explicitly with PyArrow
            import pyarrow as pa

            reader = conn.execute_arrow("SELECT * FROM sales")
            pa_reader = pa.RecordBatchReader.from_stream(reader)
            table = pa_reader.read_all()

        See Also:
            Arrow PyCapsule Interface: https://arrow.apache.org/docs/format/CDataInterface/PyCapsuleInterface.html
        """
        ...

    def __repr__(self) -> str: ...

# =====================================================================
# Module-level connect function
# =====================================================================

def connect(
    url: str,
    *,
    config: ConnectionConfig | None = None,
) -> Connection:
    """Connect to a SAP HANA database.

    Args:
        url: Connection URL in format hdbsql://user:pass@host:port
        config: Optional connection configuration for tuning performance

    Returns:
        Connection object

    Raises:
        InterfaceError: If URL is invalid
        OperationalError: If connection fails

    Example::

        conn = pyhdb_rs.connect("hdbsql://SYSTEM:password@localhost:39017")

        # With configuration
        config = ConnectionConfig(fetch_size=50000, read_timeout=60.0)
        conn = pyhdb_rs.connect("hdbsql://...", config=config)
    """
    ...

# =====================================================================
# Exceptions (DB-API 2.0)
# =====================================================================

class Error(Exception):
    """Base class for all database errors."""

    ...

class Warning(Exception):
    """Database warning."""

    ...

class InterfaceError(Error):
    """Error related to the database interface.

    Raised for connection parameter issues, driver problems, etc.
    """

    ...

class DatabaseError(Error):
    """Error related to the database.

    Base class for data-related errors.
    """

    ...

class DataError(DatabaseError):
    """Error due to problems with processed data.

    Raised for type conversion issues, value overflow, etc.
    """

    ...

class OperationalError(DatabaseError):
    """Error related to database operation.

    Raised for connection loss, timeout, authentication failure, etc.
    """

    ...

class IntegrityError(DatabaseError):
    """Error when relational integrity is affected.

    Raised for constraint violations, duplicate keys, etc.
    """

    ...

class InternalError(DatabaseError):
    """Internal database error.

    Raised for unexpected internal errors.
    """

    ...

class ProgrammingError(DatabaseError):
    """Error in programming logic.

    Raised for SQL syntax errors, missing tables, etc.
    """

    ...

class NotSupportedError(DatabaseError):
    """Feature not supported by database.

    Raised when using features not implemented or supported.
    """

    ...

# =====================================================================
# Async support (when 'async' feature enabled)
# =====================================================================

ASYNC_AVAILABLE: bool
"""Whether async support is available."""

class AsyncConnection:
    """Async SAP HANA database connection."""

    @property
    def autocommit(self) -> bool: ...
    @autocommit.setter
    def autocommit(self, value: bool) -> None: ...
    @property
    def is_connected(self) -> Awaitable[bool]: ...
    @property
    def fetch_size(self) -> Awaitable[int]: ...
    async def set_fetch_size(self, value: int) -> None: ...
    @property
    def read_timeout(self) -> Awaitable[float | None]: ...
    async def set_read_timeout(self, value: float | None) -> None: ...
    @property
    def lob_read_length(self) -> Awaitable[int]: ...
    async def set_lob_read_length(self, value: int) -> None: ...
    @property
    def lob_write_length(self) -> Awaitable[int]: ...
    async def set_lob_write_length(self, value: int) -> None: ...
    def cursor(self) -> AsyncCursor: ...
    async def close(self) -> None: ...
    async def commit(self) -> None: ...
    async def rollback(self) -> None: ...
    async def is_valid(self, check_connection: bool = True) -> bool: ...
    async def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader: ...
    async def cache_stats(self) -> CacheStats: ...
    async def clear_cache(self) -> None: ...
    async def __aenter__(self) -> AsyncConnection: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    def __repr__(self) -> str: ...

class AsyncCursor:
    """Async database cursor."""

    @property
    def description(
        self,
    ) -> list[tuple[str, int, None, int | None, int | None, int | None, bool]] | None: ...
    @property
    def rowcount(self) -> int: ...
    @property
    def arraysize(self) -> int: ...
    @arraysize.setter
    def arraysize(self, value: int) -> None: ...
    async def execute(
        self,
        sql: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
    ) -> None: ...
    async def executemany(
        self,
        sql: str,
        seq_of_parameters: Sequence[Sequence[Any]] | None = None,
    ) -> None: ...
    async def fetchone(self) -> tuple[Any, ...] | None: ...
    async def fetchmany(self, size: int | None = None) -> list[tuple[Any, ...]]: ...
    async def fetchall(self) -> list[tuple[Any, ...]]: ...
    async def close(self) -> None: ...
    async def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader: ...
    def __aiter__(self) -> AsyncCursor: ...
    async def __anext__(self) -> tuple[Any, ...]: ...
    async def __aenter__(self) -> AsyncCursor: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    def __repr__(self) -> str: ...

class ConnectionPool:
    """Async connection pool."""

    def __init__(
        self,
        url: str,
        *,
        max_size: int = 10,
        min_idle: int | None = None,
        connection_timeout: int = 30,
        config: ConnectionConfig | None = None,
        tls_config: TlsConfig | None = None,
    ) -> None: ...
    async def acquire(self) -> PooledConnection: ...
    @property
    def status(self) -> PoolStatus: ...
    @property
    def max_size(self) -> int: ...
    async def close(self) -> None: ...
    def __repr__(self) -> str: ...

class ConnectionPoolBuilder:
    """Builder for async connection pools.

    Example::

        pool = (ConnectionPoolBuilder()
            .url("hdbsql://user:pass@host:30015")
            .max_size(20)
            .network_group("ha-group")
            .build())
    """

    def __init__(self) -> None:
        """Create a new pool builder with default settings."""
        ...

    def url(self, url: str) -> Self:
        """Set the connection URL.

        Args:
            url: Connection URL in format hdbsql://user:pass@host:port

        Returns:
            Self for method chaining.
        """
        ...

    def max_size(self, size: int) -> Self:
        """Set maximum pool size.

        Args:
            size: Maximum number of connections in the pool.

        Returns:
            Self for method chaining.
        """
        ...

    def min_idle(self, size: int) -> Self:
        """Set minimum idle connections.

        Args:
            size: Minimum idle connections to maintain.

        Returns:
            Self for method chaining.
        """
        ...

    def connection_timeout(self, seconds: int) -> Self:
        """Set connection acquisition timeout.

        Args:
            seconds: Timeout in seconds.

        Returns:
            Self for method chaining.
        """
        ...

    def config(self, config: ConnectionConfig) -> Self:
        """Apply connection configuration.

        Args:
            config: Connection configuration.

        Returns:
            Self for method chaining.
        """
        ...

    def tls(self, config: TlsConfig) -> Self:
        """Configure TLS for pool connections.

        Args:
            config: TLS configuration.

        Returns:
            Self for method chaining.
        """
        ...

    def network_group(self, group: str) -> Self:
        """Set network group for HANA Scale-Out/HA deployments.

        Args:
            group: Network group name.

        Returns:
            Self for method chaining.
        """
        ...

    def build(self) -> ConnectionPool:
        """Build the connection pool.

        Returns:
            ConnectionPool instance.

        Raises:
            InterfaceError: If URL not set.
        """
        ...

    def __repr__(self) -> str: ...

class PooledConnection:
    """A connection borrowed from the pool."""

    @property
    def fetch_size(self) -> Awaitable[int]: ...
    async def set_fetch_size(self, value: int) -> None: ...
    @property
    def read_timeout(self) -> Awaitable[float | None]: ...
    async def set_read_timeout(self, value: float | None) -> None: ...
    @property
    def lob_read_length(self) -> Awaitable[int]: ...
    async def set_lob_read_length(self, value: int) -> None: ...
    @property
    def lob_write_length(self) -> Awaitable[int]: ...
    async def set_lob_write_length(self, value: int) -> None: ...
    async def execute_arrow(
        self,
        sql: str,
        config: ArrowConfig | None = None,
    ) -> RecordBatchReader: ...
    async def cursor(self) -> AsyncCursor: ...
    async def commit(self) -> None: ...
    async def rollback(self) -> None: ...
    async def is_valid(self, check_connection: bool = True) -> bool: ...
    async def cache_stats(self) -> CacheStats: ...
    async def clear_cache(self) -> None: ...
    async def __aenter__(self) -> PooledConnection: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...
    async def __repr__(self) -> str: ...

class PoolStatus:
    """Pool status information."""

    @property
    def size(self) -> int: ...
    @property
    def available(self) -> int: ...
    @property
    def max_size(self) -> int: ...
    def __repr__(self) -> str: ...
