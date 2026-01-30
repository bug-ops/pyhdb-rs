# pyhdb-rs

[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs)](https://pypi.org/project/pyhdb_rs)
[![Python](https://img.shields.io/pypi/pyversions/pyhdb_rs)](https://pypi.org/project/pyhdb_rs)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI&flag=python)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![CI](https://img.shields.io/github/actions/workflow/status/bug-ops/pyhdb-rs/ci.yml)](https://github.com/bug-ops/pyhdb-rs/actions)
[![License](https://img.shields.io/pypi/l/pyhdb_rs)](https://github.com/bug-ops/pyhdb-rs/blob/main/LICENSE-MIT)

High-performance Python driver for SAP HANA with native Apache Arrow support.

## Features

- **DB-API 2.0 compliant** - Drop-in replacement for existing HANA drivers
- **Zero-copy Arrow integration** - Direct data transfer to Polars and pandas
- **Async support** - Native async/await with connection pooling
- **Type-safe** - Full type hints and strict typing
- **Fast** - Built with Rust for 2x+ performance over hdbcli

## Installation

```bash
pip install pyhdb_rs
```

With optional dependencies:

```bash
pip install pyhdb_rs[async]     # Async support
```

For DataFrame libraries, install separately:

```bash
pip install polars              # Polars DataFrame library
pip install pandas pyarrow      # pandas with Arrow support
```

> [!TIP]
> Use `uv pip install pyhdb_rs` for faster installation.

## Quick start

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
reader = conn.execute_arrow("SELECT * FROM SALES_ORDERS WHERE ORDER_STATUS = 'SHIPPED'")
df = pl.from_arrow(reader)
print(df)
conn.close()
```

## Usage

### Polars integration

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
reader = conn.execute_arrow(
    """SELECT PRODUCT_NAME, SUM(QUANTITY) AS TOTAL_SOLD, SUM(NET_AMOUNT) AS REVENUE
       FROM SALES_ITEMS
       WHERE FISCAL_YEAR = 2025 AND REGION = 'EMEA'
       GROUP BY PRODUCT_NAME
       ORDER BY REVENUE DESC"""
)
df = pl.from_arrow(reader)
print(df.head())
conn.close()
```

### pandas integration

```python
from pyhdb_rs import ConnectionBuilder
import pyarrow as pa

conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
reader = conn.execute_arrow(
    """SELECT c.CUSTOMER_NAME, COUNT(o.ORDER_ID) AS ORDER_COUNT, SUM(o.TOTAL_AMOUNT) AS TOTAL_SPENT
       FROM CUSTOMERS c
       JOIN SALES_ORDERS o ON c.CUSTOMER_ID = o.CUSTOMER_ID
       WHERE o.ORDER_DATE >= '2025-01-01'
       GROUP BY c.CUSTOMER_NAME
       HAVING SUM(o.TOTAL_AMOUNT) > 5000"""
)
pa_reader = pa.RecordBatchReader.from_stream(reader)
df = pa_reader.read_all().to_pandas()
print(df)
conn.close()
```

### Async support

The async API provides full async/await support with connection pooling.

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
            """SELECT PRODUCT_CATEGORY, COUNT(*) AS ITEM_COUNT, SUM(NET_AMOUNT) AS TOTAL_REVENUE
               FROM SALES_ITEMS
               WHERE ORDER_DATE >= '2025-01-01'
               GROUP BY PRODUCT_CATEGORY"""
        )
        df = pl.from_arrow(reader)
        print(df)

asyncio.run(main())
```

> [!NOTE]
> Use `async with` for proper resource cleanup. The context manager automatically closes the connection on exit.

### Connection pooling

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import create_pool

pool = create_pool(
    "hdbsql://USER:PASSWORD@HOST:39017",
    max_size=10,
    connection_timeout=30
)

async def handle_request(customer_id: int):
    async with pool.acquire() as conn:
        reader = await conn.execute_arrow(
            f"""SELECT o.ORDER_ID, o.ORDER_DATE, o.TOTAL_AMOUNT, o.ORDER_STATUS
                FROM SALES_ORDERS o
                WHERE o.CUSTOMER_ID = {customer_id} AND o.ORDER_DATE >= '2025-01-01'
                ORDER BY o.ORDER_DATE DESC"""
        )
        return pl.from_arrow(reader)

# Run concurrent queries
results = await asyncio.gather(
    handle_request(1001),
    handle_request(1002),
    handle_request(1003)
)
```

## Error handling

```python
from pyhdb_rs import ConnectionBuilder, DatabaseError, InterfaceError

try:
    conn = ConnectionBuilder.from_url("hdbsql://USER:PASSWORD@HOST:39017").build()
    cursor = conn.cursor()
    cursor.execute(
        "SELECT CUSTOMER_NAME, EMAIL FROM CUSTOMERS WHERE REGISTRATION_DATE >= ?",
        ["2025-01-01"]
    )
except DatabaseError as e:
    print(f"Database error: {e}")
except InterfaceError as e:
    print(f"Connection error: {e}")
```

## Type hints

This package is fully typed and includes inline type stubs:

```python
from pyhdb_rs import ConnectionBuilder, Connection, Cursor

def query_data(uri: str, status: str) -> list[tuple[int, str, str]]:
    conn = ConnectionBuilder.from_url(uri).build()
    cursor: Cursor = conn.cursor()
    cursor.execute(
        "SELECT ORDER_ID, CUSTOMER_NAME, ORDER_STATUS FROM SALES_ORDERS WHERE ORDER_STATUS = ?",
        [status]
    )
    result = cursor.fetchall()
    conn.close()
    return result
```

## Requirements

- Python >= 3.12

## Development

```bash
git clone https://github.com/bug-ops/pyhdb-rs
cd pyhdb-rs/python

pip install -e ".[dev]"

pytest
ruff check .
mypy .
```

## Documentation

See the [main repository](https://github.com/bug-ops/pyhdb-rs) for full documentation.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
