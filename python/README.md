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
    cursor.execute("SELECT * FROM MY_TABLE")
    for row in cursor:
        print(row)
```

## Usage

### Polars integration

```python
import pyhdb_rs.polars as hdb

df = hdb.read_hana(
    "SELECT * FROM sales WHERE year = 2024",
    "hdbsql://USER:PASSWORD@HOST:39017"
)
print(df.head())
```

### pandas integration

```python
import pyhdb_rs.pandas as hdb

df = hdb.read_hana(
    "SELECT * FROM sales",
    "hdbsql://USER:PASSWORD@HOST:39017"
)
```

### Async support

```python
from pyhdb_rs.aio import connect

async with await connect("hdbsql://USER:PASSWORD@HOST:39017") as conn:
    df = await conn.execute_polars("SELECT * FROM sales")
    print(df)
```

> [!NOTE]
> Use `async with` for proper resource cleanup. The context manager handles connection pooling automatically.

### Connection pooling

```python
from pyhdb_rs.aio import create_pool

pool = create_pool(
    "hdbsql://USER:PASSWORD@HOST:39017",
    max_size=10,
    connection_timeout=30
)

async with pool.acquire() as conn:
    df = await conn.execute_polars("SELECT * FROM sales")
```

## Error handling

```python
from pyhdb_rs import connect, DatabaseError, InterfaceError

try:
    with connect("hdbsql://USER:PASSWORD@HOST:39017") as conn:
        cursor = conn.cursor()
        cursor.execute("SELECT * FROM nonexistent")
except DatabaseError as e:
    print(f"Database error: {e}")
except InterfaceError as e:
    print(f"Connection error: {e}")
```

## Type hints

This package is fully typed and includes inline type stubs:

```python
from pyhdb_rs import connect, Connection, Cursor

def query_data(uri: str) -> list[tuple[int, str]]:
    with connect(uri) as conn:
        cursor: Cursor = conn.cursor()
        cursor.execute("SELECT id, name FROM users")
        return cursor.fetchall()
```

## Requirements

- Python >= 3.11

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
