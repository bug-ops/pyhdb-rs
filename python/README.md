# pyhdb-rs

[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs)](https://pypi.org/project/pyhdb_rs)
[![Python](https://img.shields.io/pypi/pyversions/pyhdb_rs)](https://pypi.org/project/pyhdb_rs)
[![CI](https://img.shields.io/github/actions/workflow/status/bug-ops/pyhdb-rs/ci.yml)](https://github.com/bug-ops/pyhdb-rs/actions)
[![License](https://img.shields.io/pypi/l/pyhdb_rs)](https://github.com/bug-ops/pyhdb-rs/blob/main/LICENSE-MIT)

High-performance Python driver for SAP HANA with native Apache Arrow support.

## Features

- **DB-API 2.0 compliant** — Drop-in replacement for existing HANA drivers
- **Zero-copy Arrow integration** — Direct data transfer to Polars and pandas
- **Async support** — Native async/await with connection pooling
- **Type-safe** — Full type hints and strict typing
- **Fast** — Built with Rust for 2x+ performance over hdbcli

## Installation

```bash
pip install pyhdb_rs
```

With optional dependencies:

```bash
pip install pyhdb_rs[polars]    # Polars integration
pip install pyhdb_rs[pandas]    # pandas + PyArrow
pip install pyhdb_rs[async]     # Async support
pip install pyhdb_rs[all]       # All integrations
```

> [!TIP]
> Use `uv pip install pyhdb_rs` for faster installation.

## Quick start

```python
from pyhdb_rs import connect

with connect("hdbsql://USER:PASSWORD@HOST:39017") as conn:
    cursor = conn.cursor()
    cursor.execute(
        "SELECT ORDER_ID, CUSTOMER_NAME, ORDER_DATE, TOTAL_AMOUNT FROM SALES_ORDERS WHERE ORDER_STATUS = ?",
        ["SHIPPED"]
    )
    for row in cursor:
        print(row)
```

## Usage

### Polars integration

```python
import pyhdb_rs.polars as hdb

df = hdb.read_hana(
    """SELECT PRODUCT_NAME, SUM(QUANTITY) AS TOTAL_SOLD, SUM(NET_AMOUNT) AS REVENUE
       FROM SALES_ITEMS
       WHERE FISCAL_YEAR = 2025 AND REGION = 'EMEA'
       GROUP BY PRODUCT_NAME
       ORDER BY REVENUE DESC""",
    "hdbsql://USER:PASSWORD@HOST:39017"
)
print(df.head())
```

### pandas integration

```python
import pyhdb_rs.pandas as hdb

df = hdb.read_hana(
    """SELECT c.CUSTOMER_NAME, COUNT(o.ORDER_ID) AS ORDER_COUNT, SUM(o.TOTAL_AMOUNT) AS TOTAL_SPENT
       FROM CUSTOMERS c
       JOIN SALES_ORDERS o ON c.CUSTOMER_ID = o.CUSTOMER_ID
       WHERE o.ORDER_DATE >= '2025-01-01'
       GROUP BY c.CUSTOMER_NAME
       HAVING SUM(o.TOTAL_AMOUNT) > 5000""",
    "hdbsql://USER:PASSWORD@HOST:39017"
)
```

### Async support

The async API provides full async/await support with connection pooling and statement caching.

#### Basic async connection

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import connect

async def main():
    async with await connect("hdbsql://USER:PASSWORD@HOST:39017") as conn:
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

#### Async context managers

The async API uses Python's async context manager protocol for resource management:

```python
import polars as pl

# Connection context manager
async with await connect(url) as conn:
    reader = await conn.execute_arrow(
        "SELECT ORDER_ID, CUSTOMER_ID, TOTAL_AMOUNT FROM SALES_ORDERS WHERE ORDER_DATE >= '2025-06-01'"
    )
    df = pl.from_arrow(reader)
# Connection automatically closed

# Pool context manager
async with pool.acquire() as conn:
    reader = await conn.execute_arrow(
        "SELECT PRODUCT_ID, PRODUCT_NAME, STOCK_QUANTITY FROM INVENTORY WHERE STOCK_QUANTITY < 100"
    )
    df = pl.from_arrow(reader)
# Connection automatically returned to pool
```

#### Connection pooling

For applications that execute many queries concurrently, use connection pooling:

```python
import asyncio
import polars as pl
from pyhdb_rs.aio import create_pool

# Create pool at application startup
pool = create_pool(
    "hdbsql://USER:PASSWORD@HOST:39017",
    max_size=10,
    connection_timeout=30
)

# Use pool throughout application lifetime
async def handle_request(customer_id: int):
    async with pool.acquire() as conn:
        reader = await conn.execute_arrow(
            f"""SELECT o.ORDER_ID, o.ORDER_DATE, o.TOTAL_AMOUNT, o.ORDER_STATUS
                FROM SALES_ORDERS o
                WHERE o.CUSTOMER_ID = {customer_id} AND o.ORDER_DATE >= '2025-01-01'
                ORDER BY o.ORDER_DATE DESC"""
        )
        df = pl.from_arrow(reader)
        return df

# Run concurrent queries
results = await asyncio.gather(
    handle_request(),
    handle_request(),
    handle_request()
)

# Close pool at shutdown
await pool.close()
```

#### Statement caching

Enable prepared statement caching for frequently executed queries:

```python
import polars as pl
from pyhdb_rs.aio import connect

async with await connect(
    "hdbsql://USER:PASSWORD@HOST:39017",
    statement_cache_size=100  # Cache up to 100 prepared statements
) as conn:
    # First execution - cache miss
    reader = await conn.execute_arrow(
        "SELECT PRODUCT_ID, PRODUCT_NAME, UNIT_PRICE, STOCK_QUANTITY FROM PRODUCTS WHERE PRODUCT_ID = 1001"
    )
    df = pl.from_arrow(reader)

    # Subsequent executions - cache hit (faster)
    for product_id in range(1002, 1100):
        reader = await conn.execute_arrow(
            f"SELECT PRODUCT_ID, PRODUCT_NAME, UNIT_PRICE, STOCK_QUANTITY FROM PRODUCTS WHERE PRODUCT_ID = {product_id}"
        )
        df = pl.from_arrow(reader)

    # View cache statistics
    stats = await conn.cache_stats()
    if stats:
        print(f"Cache hit rate: {stats['hit_rate']:.2%}")
```

## Error handling

```python
from pyhdb_rs import connect, DatabaseError, InterfaceError

try:
    with connect("hdbsql://USER:PASSWORD@HOST:39017") as conn:
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
from pyhdb_rs import connect, Connection, Cursor

def query_data(uri: str, status: str) -> list[tuple[int, str, str]]:
    with connect(uri) as conn:
        cursor: Cursor = conn.cursor()
        cursor.execute(
            "SELECT ORDER_ID, CUSTOMER_NAME, ORDER_STATUS FROM SALES_ORDERS WHERE ORDER_STATUS = ?",
            [status]
        )
        return cursor.fetchall()
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
