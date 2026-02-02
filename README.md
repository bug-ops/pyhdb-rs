# pyhdb-rs

High-performance Python driver for SAP HANA with native Arrow support.

[![CI](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml)
[![Security](https://github.com/bug-ops/pyhdb-rs/actions/workflows/security.yml/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![Crates.io](https://img.shields.io/crates/v/hdbconnect-arrow.svg)](https://crates.io/crates/hdbconnect-arrow)
[![docs.rs](https://img.shields.io/docsrs/hdbconnect-arrow)](https://docs.rs/hdbconnect-arrow)
[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs.svg)](https://pypi.org/project/pyhdb_rs/)
[![Python](https://img.shields.io/pypi/pyversions/pyhdb_rs)](https://pypi.org/project/pyhdb_rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://github.com/bug-ops/pyhdb-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

## Features

- Full DB-API 2.0 (PEP 249) compliance
- Zero-copy Arrow data transfer via PyCapsule Interface
- Native Polars/pandas integration via `execute_arrow()`
- Async/await support with connection pooling
- Built with Rust and PyO3 for maximum performance

## Installation

```bash
uv pip install pyhdb_rs
```

With optional dependencies:

```bash
uv pip install pyhdb_rs[async]     # Async support
```

For DataFrame libraries, install separately:

```bash
pip install polars              # Polars DataFrame library
pip install pandas pyarrow      # pandas with Arrow support
```

> [!IMPORTANT]
> Requires Python 3.12 or later.

<details>
<summary><strong>Platform support</strong></summary>

| Platform | Architectures |
|----------|---------------|
| Linux (glibc) | x86_64, aarch64 |
| Linux (musl) | x86_64, aarch64 |
| macOS | x86_64, aarch64 |
| Windows | x86_64 |

</details>

<details>
<summary><strong>From source</strong></summary>

```bash
git clone https://github.com/bug-ops/pyhdb-rs.git
cd pyhdb-rs/python

uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

uv pip install maturin
maturin develop --release
```

</details>

## Quick start

### DB-API 2.0 usage

```python
from pyhdb_rs import ConnectionBuilder

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:30015").build()

with conn.cursor() as cursor:
    cursor.execute("SELECT * FROM CUSTOMERS WHERE IS_ACTIVE = ?", [True])

    rows = cursor.fetchall()
    for row in rows:
        print(row)

    cursor.execute("SELECT CUSTOMER_NAME, EMAIL_ADDRESS FROM CUSTOMERS")
    for name, email in cursor:
        print(f"{name}: {email}")

conn.close()
```

### Polars integration

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
reader = conn.execute_arrow("SELECT * FROM SALES_ITEMS WHERE FISCAL_YEAR = 2026")
df = pl.from_arrow(reader)
print(df.head())
conn.close()
```

### pandas integration

```python
from pyhdb_rs import ConnectionBuilder
import pyarrow as pa

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
reader = conn.execute_arrow("SELECT * FROM SALES_ITEMS")
pa_reader = pa.RecordBatchReader.from_stream(reader)
df = pa_reader.read_all().to_pandas()
print(df.head())
conn.close()
```

> [!TIP]
> Use `execute_arrow()` for best performance. Data flows directly from HANA to your DataFrame library without intermediate copies.

## Builder API

pyhdb-rs provides a builder pattern for flexible connection configuration.

### Basic connection

```python
from pyhdb_rs import ConnectionBuilder

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .port(30015)
    .credentials("SYSTEM", "password")
    .database("SYSTEMDB")
    .build())

with conn.cursor() as cursor:
    cursor.execute("SELECT * FROM DUMMY")
    print(cursor.fetchone())

conn.close()
```

### With TLS configuration

```python
from pyhdb_rs import ConnectionBuilder, TlsConfig

# System root certificates
tls = TlsConfig.with_system_roots()

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .tls(tls)
    .build())
```

> [!TIP]
> Use `TlsConfig.from_directory("/path/to/certs")` for custom CA certificates.

<details>
<summary><strong>Builder API: From URL with overrides</strong></summary>

```python
from pyhdb_rs import ConnectionBuilder, TlsConfig

# Start with URL, override specific settings
conn = (ConnectionBuilder.from_url("hdbsql://user:pass@host:30015")
    .tls(TlsConfig.with_system_roots())
    .autocommit(True)
    .build())
```

</details>

<details>
<summary><strong>Builder API: Async connections</strong></summary>

```python
import asyncio
from pyhdb_rs.aio import AsyncConnectionBuilder
from pyhdb_rs import TlsConfig
import polars as pl

async def main():
    conn = await (AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .autocommit(True)
        .build())

    async with conn:
        reader = await conn.execute_arrow("SELECT * FROM DUMMY")
        df = pl.from_arrow(reader)
        print(df)

asyncio.run(main())
```

</details>

<details>
<summary><strong>Advanced Polars Examples</strong></summary>

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()

# Get as Polars DataFrame (zero-copy via Arrow)
reader = conn.execute_arrow(
    "SELECT PRODUCT_ID, PRODUCT_NAME, CATEGORY, UNIT_PRICE FROM PRODUCTS WHERE IS_ACTIVE = 1"
)
df = pl.from_arrow(reader)

# For parameterized queries, use two-step pattern
cursor = conn.cursor()
cursor.execute(
    """SELECT p.PRODUCT_NAME, p.UNIT_PRICE, s.STOCK_QUANTITY
       FROM PRODUCTS p
       JOIN STOCK s ON p.PRODUCT_ID = s.PRODUCT_ID
       WHERE p.CATEGORY = ? AND s.STOCK_QUANTITY > ?""",
    ["Electronics", 10]
)
df = pl.from_arrow(cursor.fetch_arrow())

# Stream large datasets batch-by-batch with ArrowConfig
from pyhdb_rs import ArrowConfig

config = ArrowConfig(batch_size=10000)
reader = conn.execute_arrow(
    "SELECT ORDER_ID, CUSTOMER_ID, ORDER_DATE, TOTAL_AMOUNT FROM SALES_ORDERS WHERE ORDER_DATE >= '2024-01-01'",
    config=config
)
for batch in reader:
    process_batch(batch)

conn.close()
```

### Lazy evaluation with Polars

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
reader = conn.execute_arrow(
    "SELECT ORDER_ID, CUSTOMER_NAME, PRODUCT_CATEGORY, NET_AMOUNT FROM SALES_ITEMS WHERE YEAR(ORDER_DATE) = 2025"
)

# Convert to LazyFrame for deferred operations
lf = pl.from_arrow(reader).lazy()
result = (
    lf.filter(pl.col("NET_AMOUNT") > 1000)
    .select(["CUSTOMER_NAME", "PRODUCT_CATEGORY", "NET_AMOUNT"])
    .collect()
)

conn.close()
```

</details>

<details>
<summary><strong>TLS/SSL Configuration (5 methods)</strong></summary>

pyhdb-rs provides flexible TLS configuration via `TlsConfig` for secure connections.

### TLS Configuration Methods

`TlsConfig` provides five factory methods for different certificate sources:

#### 1. From Directory (recommended for production)

Load all `.pem`, `.crt`, and `.cer` files from a directory:

```python
from pyhdb_rs import TlsConfig, ConnectionBuilder

tls = TlsConfig.from_directory("/etc/hana/certs")

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .tls(tls)
    .build())
```

**Recommended for production:** Place all CA certificates in a single directory for easy management.

#### 2. From Environment Variable

Load certificate from an environment variable:

```python
import os
from pyhdb_rs import TlsConfig, ConnectionBuilder

# Set certificate in environment
os.environ["HANA_CA_CERT"] = """-----BEGIN CERTIFICATE-----
MIIDdzCCAl+gAwIBAgIEAgAAuTANBgkqhkiG9w0BAQUFADBaMQswCQYDVQQGEwJJ
...
-----END CERTIFICATE-----"""

tls = TlsConfig.from_environment("HANA_CA_CERT")

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .tls(tls)
    .build())
```

**Note:** Useful for containerized deployments where certificates are injected via environment variables.

#### 3. From Certificate String

Provide PEM-encoded certificate directly:

```python
from pyhdb_rs import TlsConfig, ConnectionBuilder

with open("/path/to/ca-bundle.pem") as f:
    cert_pem = f.read()

tls = TlsConfig.from_certificate(cert_pem)

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .tls(tls)
    .build())
```

#### 4. System Root Certificates

Use Mozilla's root certificates (bundled):

```python
from pyhdb_rs import TlsConfig, ConnectionBuilder

tls = TlsConfig.with_system_roots()

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .tls(tls)
    .build())
```

**Best choice:** Use this when your HANA server uses a certificate signed by a well-known CA (e.g., Let's Encrypt, DigiCert).

#### 5. Insecure (development only)

Skip certificate verification:

```python
from pyhdb_rs import TlsConfig, ConnectionBuilder

tls = TlsConfig.insecure()

conn = (ConnectionBuilder()
    .host("hana-dev.internal")
    .credentials("SYSTEM", "password")
    .tls(tls)
    .build())
```

**⚠️ SECURITY WARNING:** `TlsConfig.insecure()` disables certificate verification completely. **NEVER use in production.** This makes your connection vulnerable to man-in-the-middle attacks.

### URL Scheme for TLS

The `hdbsqls://` scheme automatically enables TLS with system roots:

```python
from pyhdb_rs import ConnectionBuilder

# Equivalent to using TlsConfig.with_system_roots()
conn = ConnectionBuilder.from_url("hdbsqls://user:pass@host:30015").build()

# Override with custom TLS config
conn = (ConnectionBuilder.from_url("hdbsqls://user:pass@host:30015")
    .tls(TlsConfig.from_directory("/custom/certs"))
    .build())
```

</details>

<details>
<summary><strong>Cursor Holdability (Transaction Control)</strong></summary>

Control result set behavior across transaction boundaries with `CursorHoldability`. This determines whether cursors remain open after `commit()` or `rollback()` operations.

```python
from pyhdb_rs import ConnectionBuilder, CursorHoldability

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .cursor_holdability(CursorHoldability.CommitAndRollback)
    .build())

conn.set_autocommit(False)
with conn.cursor() as cur:
    cur.execute("SELECT * FROM large_table")
    rows = cur.fetchmany(1000)

    # Process first batch
    process_batch(rows)

    conn.commit()  # Cursor remains open with CommitAndRollback

    # Continue reading from the same result set
    more_rows = cur.fetchmany(1000)
```

### Holdability Variants

| Variant | Behavior |
|---------|----------|
| `CursorHoldability.None` | Cursor closed on commit **and** rollback (default) |
| `CursorHoldability.Commit` | Cursor held across commits, closed on rollback |
| `CursorHoldability.Rollback` | Cursor held across rollbacks, closed on commit |
| `CursorHoldability.CommitAndRollback` | Cursor held across both operations |

**Use case:** Use `CommitAndRollback` when you need to iterate over large result sets while performing intermediate commits to free locks or manage transaction size.

</details>

<details>
<summary><strong>High Availability & Scale-Out Deployments</strong></summary>

Configure network groups for HANA HA and Scale-Out deployments to control connection routing.

### Network Group Configuration

```python
from pyhdb_rs import ConnectionBuilder

conn = (ConnectionBuilder()
    .host("hana-ha-cluster.example.com")
    .port(30015)
    .credentials("SYSTEM", "password")
    .network_group("ha-primary")
    .build())
```

**Important:** Network groups are essential for proper routing in multi-node HANA environments. They determine which network interface the driver uses when multiple options are available.

### Use Cases

**1. High Availability Clusters**

Direct connections to specific nodes in an HA setup:

```python
# Connect to primary node network
conn_primary = (ConnectionBuilder()
    .host("hana-ha.example.com")
    .credentials("SYSTEM", "password")
    .network_group("internal")
    .build())

# Connect to secondary node network
conn_secondary = (ConnectionBuilder()
    .host("hana-ha.example.com")
    .credentials("SYSTEM", "password")
    .network_group("external")
    .build())
```

**2. Scale-Out Systems**

Route to specific network groups in scale-out configurations:

```python
from pyhdb_rs import ConnectionBuilder

# Connect via data network
conn = (ConnectionBuilder()
    .host("hana-scaleout.example.com")
    .credentials("SYSTEM", "password")
    .network_group("data-network")
    .build())
```

### Async Connection Pools

Combine network groups with connection pooling for production deployments:

```python
from pyhdb_rs.aio import ConnectionPoolBuilder
from pyhdb_rs import TlsConfig
import polars as pl

pool = (ConnectionPoolBuilder()
    .url("hdbsql://user:pass@host:30015")
    .network_group("production")
    .max_size(20)
    .tls(TlsConfig.with_system_roots())
    .build())

async with pool.acquire() as conn:
    reader = await conn.execute_arrow("SELECT * FROM large_table")
    df = pl.from_arrow(reader)
```

</details>

<details>
<summary><strong>Async/Await Support</strong></summary>

pyhdb-rs supports async/await operations for non-blocking database access.

**Installation:** Async support requires the `async` extra: `uv pip install pyhdb_rs[async]`

**⚠️ Memory Warning:** The async `execute_arrow()` loads ALL rows into memory before streaming batches. For large datasets (>100K rows), use the sync API for true streaming with O(batch_size) memory usage.

### Basic async usage

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import AsyncConnectionBuilder

async def main():
    conn = await (AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("USER", "PASSWORD")
        .build())

    async with conn:
        reader = await conn.execute_arrow(
            """SELECT PRODUCT_NAME, SUM(QUANTITY) AS TOTAL_SOLD, SUM(NET_AMOUNT) AS REVENUE
               FROM SALES_ITEMS
               WHERE ORDER_DATE >= '2025-01-01'
               GROUP BY PRODUCT_NAME
               ORDER BY REVENUE DESC
               LIMIT 10"""
        )
        df = pl.from_arrow(reader)
        print(df)

asyncio.run(main())
```

<details>
<summary><strong>Connection pooling</strong></summary>

### Using ConnectionPoolBuilder (recommended)

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import ConnectionPoolBuilder
from pyhdb_rs import TlsConfig

async def main():
    # Builder pattern for pools
    pool = (ConnectionPoolBuilder()
        .url("hdbsql://USER:PASSWORD@HOST:30015")
        .max_size(10)
        .tls(TlsConfig.with_system_roots())
        .network_group("production")
        .build())

    async with pool.acquire() as conn:
        reader = await conn.execute_arrow(
            """SELECT CUSTOMER_ID, COUNT(ORDER_ID) AS ORDER_COUNT, SUM(TOTAL_AMOUNT) AS TOTAL_SPENT
               FROM SALES_ORDERS
               WHERE ORDER_DATE >= '2025-01-01' AND ORDER_STATUS = 'COMPLETED'
               GROUP BY CUSTOMER_ID
               HAVING SUM(TOTAL_AMOUNT) > 10000"""
        )
        df = pl.from_arrow(reader)
        print(df)

    status = pool.status
    print(f"Pool size: {status.size}, available: {status.available}")

asyncio.run(main())
```

### Using ConnectionPool directly

```python
import asyncio
from pyhdb_rs.aio import ConnectionPoolBuilder
from pyhdb_rs import TlsConfig

async def main():
    pool = (ConnectionPoolBuilder()
        .url("hdbsql://USER:PASSWORD@HOST:30015")
        .max_size(10)
        .connection_timeout(30)
        .tls(TlsConfig.with_system_roots())
        .build())

    async with pool.acquire() as conn:
        # Use connection
        pass

asyncio.run(main())
```

</details>

<details>
<summary><strong>Concurrent queries</strong></summary>

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import ConnectionPoolBuilder

async def fetch_sales_by_region(pool, region: str):
    async with pool.acquire() as conn:
        reader = await conn.execute_arrow(
            f"""SELECT PRODUCT_CATEGORY, SUM(NET_AMOUNT) AS REVENUE
                FROM SALES_ITEMS
                WHERE REGION = '{region}' AND FISCAL_YEAR = 2025
                GROUP BY PRODUCT_CATEGORY
                ORDER BY REVENUE DESC"""
        )
        return pl.from_arrow(reader)

async def main():
    pool = (ConnectionPoolBuilder()
        .url("hdbsql://USER:PASSWORD@HOST:30015")
        .max_size(5)
        .build())

    # Run multiple queries concurrently for different regions
    results = await asyncio.gather(
        fetch_sales_by_region(pool, "EMEA"),
        fetch_sales_by_region(pool, "AMERICAS"),
        fetch_sales_by_region(pool, "APAC"),
    )

    emea_df, americas_df, apac_df = results
    print(f"EMEA: {len(emea_df)} categories, AMERICAS: {len(americas_df)} categories")

asyncio.run(main())
```

</details>

</details>

<details>
<summary><strong>API Patterns & Best Practices</strong></summary>

### Arrow RecordBatchReader

`execute_arrow()` returns a `RecordBatchReader` that implements the Arrow PyCapsule Interface (`__arrow_c_stream__`):

```python
from pyhdb_rs import ArrowConfig, ConnectionBuilder
import polars as pl
import pyarrow as pa

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:30015").build()

# Pattern 1: Direct conversion to Polars (recommended)
reader = conn.execute_arrow(
    "SELECT CUSTOMER_ID, CUSTOMER_NAME, TOTAL_ORDERS FROM CUSTOMER_SUMMARY WHERE ACTIVE_FLAG = 1"
)
df = pl.from_arrow(reader)  # Zero-copy via PyCapsule

# Pattern 2: Convert to PyArrow Table first
reader = conn.execute_arrow(
    "SELECT ORDER_ID, ORDER_DATE, TOTAL_AMOUNT FROM SALES_ORDERS WHERE ORDER_DATE >= '2025-01-01'"
)
pa_reader = pa.RecordBatchReader.from_stream(reader)
table = pa_reader.read_all()

# Pattern 3: Stream large datasets with ArrowConfig
config = ArrowConfig(batch_size=10000)
reader = conn.execute_arrow(
    "SELECT TRANSACTION_ID, CUSTOMER_ID, AMOUNT, TRANSACTION_DATE FROM TRANSACTION_HISTORY WHERE YEAR(TRANSACTION_DATE) = 2025",
    config=config
)
for batch in reader:
    process_batch(batch)  # Each batch is a RecordBatch

conn.close()
```

**Note:** The reader is consumed after use (single-pass iterator). You cannot read from it twice.

### Parameterized Queries with Arrow

`execute_arrow()` does NOT support query parameters. For parameterized queries, use the two-step pattern:

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:30015").build()

# Two-step: execute() then fetch_arrow()
cursor = conn.cursor()
cursor.execute(
    """SELECT o.ORDER_ID, o.ORDER_DATE, c.CUSTOMER_NAME, o.TOTAL_AMOUNT
       FROM SALES_ORDERS o
       JOIN CUSTOMERS c ON o.CUSTOMER_ID = c.CUSTOMER_ID
       WHERE o.ORDER_STATUS = ? AND o.TOTAL_AMOUNT > ? AND o.ORDER_DATE >= ?""",
    ["COMPLETED", 5000, "2025-01-01"]
)
df = pl.from_arrow(cursor.fetch_arrow())

conn.close()
```

### Connection Validation

Check if a connection is still valid before use:

```python
from pyhdb_rs import ConnectionBuilder

# Sync API
conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:30015").build()
if not conn.is_valid():
    conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:30015").build()  # Reconnect

# Async API
async with conn:
    if not await conn.is_valid():
        # Handle invalid connection
        pass
```

The `is_valid(check_connection=True)` method:
- When `check_connection=True` (default): Executes `SELECT 1 FROM DUMMY` to verify connection is alive
- When `check_connection=False`: Only checks internal state (no network round-trip)

</details>

<details>
<summary><strong>Error Handling</strong></summary>

pyhdb-rs provides detailed error messages that include HANA server information for better diagnostics:

```python
from pyhdb_rs import ConnectionBuilder, ProgrammingError, DatabaseError, InterfaceError

try:
    conn = ConnectionBuilder.from_url("hdbsql://user:pass@host:30015").build()
    cursor = conn.cursor()
    cursor.execute("SELECT CUSTOMER_NAME, BALANCE FROM ACCOUNTS WHERE ACCOUNT_TYPE = ?", ["PREMIUM"])
except ProgrammingError as e:
    # Error message includes:
    # - Error code: [259] (HANA error number)
    # - Message: invalid table name
    # - Severity: Error
    # - SQLSTATE: 42000 (SQL standard code)
    # Example: "[259] invalid table name: NONEXISTENT_TABLE (severity: Error), SQLSTATE: 42000"
    print(f"SQL Error: {e}")
except DatabaseError as e:
    print(f"Database error: {e}")
except InterfaceError as e:
    print(f"Connection error: {e}")
```

**Exception hierarchy** (DB-API 2.0 compliant):

- `pyhdb_rs.Error` - Base exception
- `pyhdb_rs.InterfaceError` - Connection or driver issues
- `pyhdb_rs.DatabaseError` - Database server errors
  - `pyhdb_rs.ProgrammingError` - SQL syntax, missing table, wrong column
  - `pyhdb_rs.IntegrityError` - Constraint violations, duplicate keys
  - `pyhdb_rs.DataError` - Type conversion, value overflow
  - `pyhdb_rs.OperationalError` - Connection lost, timeout, server unavailable
  - `pyhdb_rs.NotSupportedError` - Unsupported operation

</details>

<details>
<summary><strong>Connection URL Format Reference</strong></summary>

```
hdbsql://[USER[:PASSWORD]@]HOST[:PORT][/DATABASE][?OPTIONS]
```

Examples:
- `hdbsql://user:pass@localhost:30015`
- `hdbsql://user:pass@hana.example.com:39017/HDB`
- `hdbsql://user:pass@host:30015?encrypt=true`

<details>
<summary><strong>Type mapping</strong></summary>

| HANA Type | Python Type | Arrow Type |
|-----------|-------------|------------|
| TINYINT, SMALLINT, INT | `int` | Int8, Int16, Int32 |
| BIGINT | `int` | Int64 |
| DECIMAL | `decimal.Decimal` | Decimal128 |
| REAL, DOUBLE | `float` | Float32, Float64 |
| VARCHAR, NVARCHAR | `str` | Utf8 |
| CLOB, NCLOB | `str` | LargeUtf8 |
| BLOB | `bytes` | LargeBinary |
| DATE | `datetime.date` | Date32 |
| TIME | `datetime.time` | Time64 |
| TIMESTAMP | `datetime.datetime` | Timestamp |
| BOOLEAN | `bool` | Boolean |

</details>

<details>
<summary><strong>Performance</strong></summary>

pyhdb-rs is designed for high-performance data access:

- **Zero-copy Arrow**: Data flows directly from HANA to Polars/pandas without intermediate copies
- **Rust core**: All heavy lifting happens in compiled Rust code
- **Connection pooling**: Async pool with configurable size for high-concurrency workloads
- **Batch processing**: Efficient handling of large result sets via streaming
- **Optimized conversions**: Direct BigInt arithmetic for decimals, builder reuse at batch boundaries
- **Type caching**: Thread-local Python type references minimize FFI overhead

Benchmarks show 2x+ performance improvement over hdbcli for bulk reads.

**Performance tip:** For maximum performance, use `execute_arrow()` with your Arrow-compatible library (Polars, PyArrow, pandas) for zero-copy data transfer.

</details>

</details>

<details>
<summary><strong>Arrow Ecosystem Integration</strong></summary>

Data is exported in [Apache Arrow](https://arrow.apache.org/) format, enabling zero-copy interoperability with:

- **DataFrames** - Polars, pandas, Vaex, Dask
- **Query engines** - DataFusion, DuckDB, ClickHouse
- **ML/AI** - Ray, Hugging Face Datasets, PyTorch
- **Data lakes** - Delta Lake, Apache Iceberg, Lance
- **Serialization** - Parquet, Arrow IPC (Feather)

For Rust integration examples (DataFusion, DuckDB, Parquet export), see [`hdbconnect-arrow`](crates/hdbconnect-arrow/README.md).

</details>

## MCP Server for AI Agents

**hdbconnect-mcp** provides AI assistants (Claude Desktop, Cline, VS Code) with secure, programmatic access to SAP HANA databases through the Model Context Protocol.

### Features

- **Read-only mode by default** — Blocks DML/DDL operations
- **Enterprise authentication** — OIDC/JWT support with multi-tenant isolation
- **Intelligent caching** — Schema metadata and query results with per-user isolation
- **HTTP/SSE transport** — Remote access with Bearer token authentication
- **Connection pooling** — Efficient resource management

### Installation

```bash
cargo install hdbconnect-mcp
```

### Quick Start

Add to Claude Desktop config (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "hana": {
      "command": "hdbconnect-mcp",
      "args": [
        "--url", "hdbsql://user:password@host:39017",
        "--read-only", "true",
        "--row-limit", "10000"
      ]
    }
  }
}
```

See [hdbconnect-mcp documentation](crates/hdbconnect-mcp/README.md) for HTTP transport, authentication, and deployment guides.

## MSRV policy

> [!NOTE]
> Minimum Supported Rust Version: **1.88**. MSRV increases are minor version bumps.

<details>
<summary><strong>Examples</strong></summary>

Interactive Jupyter notebooks are available in [`examples/notebooks/`](examples/notebooks/):

- **01_quickstart** - Basic connection and DataFrame integration
- **02_polars_analytics** - Advanced Polars analytics with LazyFrames
- **03_streaming_large_data** - Memory-efficient large dataset processing
- **04_performance_comparison** - Benchmarks vs hdbcli

</details>

## Repository

- [GitHub](https://github.com/bug-ops/pyhdb-rs)
- [Issue Tracker](https://github.com/bug-ops/pyhdb-rs/issues)
- [Changelog](CHANGELOG.md)
- [API Documentation (Rust)](https://docs.rs/hdbconnect-arrow)

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
