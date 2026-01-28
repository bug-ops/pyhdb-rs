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
- Native Polars/pandas integration
- Async/await support with connection pooling
- Built with Rust and PyO3 for maximum performance

## Installation

```bash
uv pip install pyhdb_rs
```

With optional dependencies:

```bash
uv pip install pyhdb_rs[polars]    # Polars integration
uv pip install pyhdb_rs[pandas]    # pandas + PyArrow
uv pip install pyhdb_rs[async]     # Async support
uv pip install pyhdb_rs[all]       # All integrations
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
import pyhdb_rs

conn = pyhdb_rs.connect("hdbsql://USER:PASSWORD@HOST:30015")

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
import pyhdb_rs.polars as hdb

df = hdb.read_hana(
    """
    SELECT
        PRODUCT_CATEGORY,
        FISCAL_YEAR,
        SUM(NET_AMOUNT) AS TOTAL_REVENUE,
        COUNT(DISTINCT ORDER_ID) AS ORDER_COUNT,
        AVG(QUANTITY) AS AVG_QUANTITY
    FROM SALES_ITEMS
    WHERE FISCAL_YEAR BETWEEN 2024 AND 2026
        AND SALES_REGION IN ('EMEA', 'AMERICAS')
    GROUP BY PRODUCT_CATEGORY, FISCAL_YEAR
    ORDER BY TOTAL_REVENUE DESC
    """,
    "hdbsql://USER:PASSWORD@HOST:39017"
)

print(df.head())
```

> [!TIP]
> Use `execute_arrow()` with Polars for best performance. Data flows directly from HANA to Polars without intermediate copies.

Or using the connection object:

```python
import pyhdb_rs
import polars as pl

conn = pyhdb_rs.connect("hdbsql://USER:PASSWORD@HOST:30015")

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

# Stream large datasets batch-by-batch
reader = conn.execute_arrow(
    "SELECT ORDER_ID, CUSTOMER_ID, ORDER_DATE, TOTAL_AMOUNT FROM SALES_ORDERS WHERE ORDER_DATE >= '2024-01-01'"
)
for batch in reader:
    process_batch(batch)

conn.close()
```

### pandas integration

```python
import pyhdb_rs.pandas as hdb

df = hdb.read_hana(
    """SELECT ORDER_ID, CUSTOMER_NAME, PRODUCT_NAME, QUANTITY, NET_AMOUNT
       FROM SALES_ITEMS
       WHERE ORDER_STATUS = 'COMPLETED' AND ORDER_DATE >= ADD_MONTHS(CURRENT_DATE, -12)""",
    "hdbsql://USER:PASSWORD@HOST:39017"
)

print(df.head())
```

### Lazy evaluation with Polars

```python
import pyhdb_rs.polars as hdb
import polars as pl

# scan_hana() returns a LazyFrame - query executes on .collect()
lf = hdb.scan_hana(
    "SELECT ORDER_ID, CUSTOMER_NAME, PRODUCT_CATEGORY, NET_AMOUNT, ORDER_DATE FROM SALES_ITEMS WHERE YEAR(ORDER_DATE) = 2025",
    "hdbsql://USER:PASSWORD@HOST:39017"
)
result = lf.filter(pl.col("NET_AMOUNT") > 1000).select(["CUSTOMER_NAME", "PRODUCT_CATEGORY", "NET_AMOUNT"]).collect()
```

> [!TIP]
> Use `scan_hana()` for lazy evaluation when you need to apply filters or transformations before materializing data.

## Async support

pyhdb-rs supports async/await operations for non-blocking database access.

> [!NOTE]
> Async support requires the `async` extra: `uv pip install pyhdb_rs[async]`

### Basic async usage

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import connect

async def main():
    async with await connect("hdbsql://USER:PASSWORD@HOST:30015") as conn:
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

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import create_pool

async def main():
    pool = create_pool(
        "hdbsql://USER:PASSWORD@HOST:30015",
        max_size=10,
        connection_timeout=30
    )

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

</details>

<details>
<summary><strong>Concurrent queries</strong></summary>

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import create_pool

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
    pool = create_pool("hdbsql://USER:PASSWORD@HOST:30015", max_size=5)

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

## API Patterns

### Arrow RecordBatchReader

`execute_arrow()` returns a `RecordBatchReader` that implements the Arrow PyCapsule Interface (`__arrow_c_stream__`):

```python
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

# Pattern 3: Stream large datasets
reader = conn.execute_arrow(
    "SELECT TRANSACTION_ID, CUSTOMER_ID, AMOUNT, TRANSACTION_DATE FROM TRANSACTION_HISTORY WHERE YEAR(TRANSACTION_DATE) = 2025",
    batch_size=10000
)
for batch in reader:
    process_batch(batch)  # Each batch is a RecordBatch
```

> [!NOTE]
> The reader is consumed after use (single-pass iterator). You cannot read from it twice.

### Parameterized Queries with Arrow

`execute_arrow()` does NOT support query parameters. For parameterized queries, use the two-step pattern:

```python
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
```

### Write Methods

Write DataFrames back to HANA:

```python
# Polars
import pyhdb_rs.polars as hdb
df = pl.DataFrame({"id": [1, 2, 3], "name": ["a", "b", "c"]})
hdb.write_hana(df, "my_table", uri, if_table_exists="replace")

# pandas
import pyhdb_rs.pandas as hdb
df = pd.DataFrame({"id": [1, 2, 3], "name": ["a", "b", "c"]})
hdb.to_hana(df, "my_table", uri, if_exists="append")
```

> [!NOTE]
> Naming difference is intentional: `write_hana()` follows Polars conventions, `to_hana()` follows pandas conventions.

## Error handling

pyhdb-rs provides detailed error messages that include HANA server information for better diagnostics:

```python
import pyhdb_rs

try:
    conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")
    cursor = conn.cursor()
    cursor.execute("SELECT CUSTOMER_NAME, BALANCE FROM ACCOUNTS WHERE ACCOUNT_TYPE = ?", ["PREMIUM"])
except pyhdb_rs.ProgrammingError as e:
    # Error message includes:
    # - Error code: [259] (HANA error number)
    # - Message: invalid table name
    # - Severity: Error
    # - SQLSTATE: 42000 (SQL standard code)
    # Example: "[259] invalid table name: NONEXISTENT_TABLE (severity: Error), SQLSTATE: 42000"
    print(f"SQL Error: {e}")
except pyhdb_rs.DatabaseError as e:
    print(f"Database error: {e}")
except pyhdb_rs.InterfaceError as e:
    print(f"Connection error: {e}")
```

**Exception hierarchy** (DB-API 2.0 compliant):

- `pyhdb_rs.Error` — Base exception
- `pyhdb_rs.InterfaceError` — Connection or driver issues
- `pyhdb_rs.DatabaseError` — Database server errors
  - `pyhdb_rs.ProgrammingError` — SQL syntax, missing table, wrong column
  - `pyhdb_rs.IntegrityError` — Constraint violations, duplicate keys
  - `pyhdb_rs.DataError` — Type conversion, value overflow
  - `pyhdb_rs.OperationalError` — Connection lost, timeout, server unavailable
  - `pyhdb_rs.NotSupportedError` — Unsupported operation

## Connection URL format

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

> [!TIP]
> For maximum performance, use `execute_arrow()` with your Arrow-compatible library (Polars, PyArrow, pandas) for zero-copy data transfer.

</details>

## Arrow ecosystem

Data is exported in [Apache Arrow](https://arrow.apache.org/) format, enabling zero-copy interoperability with:

- **DataFrames** — Polars, pandas, Vaex, Dask
- **Query engines** — DataFusion, DuckDB, ClickHouse
- **ML/AI** — Ray, Hugging Face Datasets, PyTorch
- **Data lakes** — Delta Lake, Apache Iceberg, Lance
- **Serialization** — Parquet, Arrow IPC (Feather)

For Rust integration examples (DataFusion, DuckDB, Parquet export), see [`hdbconnect-arrow`](crates/hdbconnect-arrow/README.md).

## MSRV policy

> [!NOTE]
> Minimum Supported Rust Version: **1.88**. MSRV increases are minor version bumps.

<details>
<summary><strong>Examples</strong></summary>

Interactive Jupyter notebooks are available in [`examples/notebooks/`](examples/notebooks/):

- **01_quickstart** — Basic connection and DataFrame integration
- **02_polars_analytics** — Advanced Polars analytics with LazyFrames
- **03_streaming_large_data** — Memory-efficient large dataset processing
- **04_performance_comparison** — Benchmarks vs hdbcli

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
