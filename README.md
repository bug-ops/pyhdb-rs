# pyhdb-rs

High-performance Python driver for SAP HANA with native Arrow support.

[![Crates.io](https://img.shields.io/crates/v/hdbconnect-arrow.svg)](https://crates.io/crates/hdbconnect-arrow)
[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs.svg)](https://pypi.org/project/pyhdb_rs/)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE)

## Features

- Full DB-API 2.0 (PEP 249) compliance
- Zero-copy Arrow data transfer via PyCapsule Interface
- Native Polars/pandas integration
- Built with Rust and PyO3 for maximum performance

## Installation

```bash
pip install pyhdb_rs
```

## Quick Start

```python
import pyhdb_rs

# Connect to HANA
conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")

# Execute query with cursor (DB-API 2.0)
cursor = conn.cursor()
cursor.execute("SELECT * FROM USERS")
for row in cursor:
    print(row)

# Or get as Polars DataFrame (zero-copy!)
df = conn.execute_polars("SELECT * FROM USERS")

conn.close()
```

## Polars Integration

```python
import pyhdb_rs.polars as hdb

# Read directly into Polars DataFrame
df = hdb.read_hana(
    "SELECT * FROM sales WHERE year = 2024",
    "hdbsql://analyst:secret@hana.corp:39017"
)
```

## pandas Integration

```python
import pyhdb_rs.pandas as hdb

# Read into pandas DataFrame
df = hdb.read_hana(
    "SELECT * FROM sales",
    "hdbsql://user:pass@host:39017"
)
```

## Repository

- **GitHub:** https://github.com/bug-ops/pyhdb-rs
- **Issues:** https://github.com/bug-ops/pyhdb-rs/issues

## License

Apache-2.0 OR MIT
