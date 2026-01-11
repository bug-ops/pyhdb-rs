# pyhdb-rs

High-performance Python driver for SAP HANA with native Arrow support.

[![CI](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml)
[![Security](https://github.com/bug-ops/pyhdb-rs/actions/workflows/security.yml/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![Crates.io](https://img.shields.io/crates/v/hdbconnect-arrow.svg)](https://crates.io/crates/hdbconnect-arrow)
[![docs.rs](https://img.shields.io/docsrs/hdbconnect-arrow)](https://docs.rs/hdbconnect-arrow)
[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs.svg)](https://pypi.org/project/pyhdb_rs/)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://github.com/bug-ops/pyhdb-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

## Features

- Full DB-API 2.0 (PEP 249) compliance
- Zero-copy Arrow data transfer via PyCapsule Interface
- Native Polars/pandas integration
- Async/await support with connection pooling
- Built with Rust and PyO3 for maximum performance

## Installation

### From PyPI

```bash
pip install pyhdb_rs
```

### With optional dependencies

```bash
# For Polars integration
pip install pyhdb_rs[polars]

# For pandas integration
pip install pyhdb_rs[pandas]

# For async support
pip install pyhdb_rs[async]

# All optional dependencies
pip install pyhdb_rs[all]
```

> [!IMPORTANT]
> Requires Python 3.11 or later.

### From source

```bash
git clone https://github.com/bug-ops/pyhdb-rs.git
cd pyhdb-rs

python -m venv .venv
source .venv/bin/activate

pip install maturin
cd python
maturin develop
```

## Quick start

### DB-API 2.0 usage

```python
import pyhdb_rs

conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")

with conn.cursor() as cursor:
    cursor.execute("SELECT * FROM USERS WHERE active = ?", [True])

    rows = cursor.fetchall()
    for row in rows:
        print(row)

    cursor.execute("SELECT name, email FROM USERS")
    for name, email in cursor:
        print(f"{name}: {email}")

conn.close()
```

### Polars integration

```python
import pyhdb_rs.polars as hdb

# Read directly into Polars DataFrame (zero-copy)
df = hdb.read_hana(
    "SELECT * FROM sales WHERE year = 2024",
    "hdbsql://analyst:secret@hana.corp:39017"
)

print(df.head())
```

> [!TIP]
> Use `execute_polars()` for best performance. Data flows directly from HANA to Polars without intermediate copies.

Or using the connection object:

```python
import pyhdb_rs

conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")

# Get as Polars DataFrame
df = conn.execute_polars("SELECT * FROM products")

# Get as Arrow RecordBatchReader for streaming large datasets
reader = conn.execute_arrow("SELECT * FROM large_table")
for batch in reader:
    process_batch(batch)

conn.close()
```

### pandas integration

```python
import pyhdb_rs.pandas as hdb

df = hdb.read_hana(
    "SELECT * FROM sales",
    "hdbsql://user:pass@host:39017"
)

print(df.head())
```

## Async support

pyhdb-rs supports async/await operations for non-blocking database access.

> [!NOTE]
> Async support requires the package to be built with the `async` feature: `pip install pyhdb_rs[async]`

### Basic async usage

```python
import asyncio
from pyhdb_rs.aio import connect

async def main():
    async with await connect("hdbsql://user:pass@host:30015") as conn:
        df = await conn.execute_polars("SELECT * FROM sales")
        print(df)

asyncio.run(main())
```

### Connection pooling

```python
import asyncio
from pyhdb_rs.aio import create_pool

async def main():
    pool = create_pool(
        "hdbsql://user:pass@host:30015",
        max_size=10,
        connection_timeout=30
    )

    async with pool.acquire() as conn:
        df = await conn.execute_polars("SELECT * FROM sales")
        print(df)

    status = pool.status
    print(f"Pool size: {status.size}, available: {status.available}")

asyncio.run(main())
```

### Concurrent queries

```python
import asyncio
from pyhdb_rs.aio import create_pool

async def fetch_data(pool, table: str):
    async with pool.acquire() as conn:
        return await conn.execute_polars(f"SELECT * FROM {table}")

async def main():
    pool = create_pool("hdbsql://user:pass@host:30015", max_size=5)

    # Run multiple queries concurrently
    results = await asyncio.gather(
        fetch_data(pool, "customers"),
        fetch_data(pool, "orders"),
        fetch_data(pool, "products"),
    )

    customers_df, orders_df, products_df = results

asyncio.run(main())
```

## Connection URL format

```
hdbsql://[user[:password]@]host[:port][/database][?options]
```

Examples:
- `hdbsql://user:pass@localhost:30015`
- `hdbsql://user:pass@hana.example.com:39017/HDB`
- `hdbsql://user:pass@host:30015?encrypt=true`

## Type mapping

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

## Performance

pyhdb-rs is designed for high-performance data access:

- **Zero-copy Arrow**: Data flows directly from HANA to Polars/pandas without intermediate copies
- **Rust core**: All heavy lifting happens in compiled Rust code
- **Connection pooling**: Async pool with configurable size for high-concurrency workloads
- **Batch processing**: Efficient handling of large result sets via streaming

Benchmarks show 2x+ performance improvement over hdbcli for bulk reads.

## MSRV policy

> [!NOTE]
> Minimum Supported Rust Version: **1.85**. MSRV increases are minor version bumps.

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
