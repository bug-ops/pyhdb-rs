# hdbconnect-py

[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs)](https://pypi.org/project/pyhdb_rs/)
[![Python](https://img.shields.io/pypi/pyversions/pyhdb_rs)](https://pypi.org/project/pyhdb_rs)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI&flag=hdbconnect-py)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://github.com/bug-ops/pyhdb-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

PyO3 bindings for SAP HANA via hdbconnect, exposing the native Rust driver to Python with zero-copy Arrow data transfer.

## Features

- **DB-API 2.0 compliant** — `Connection` and `Cursor` classes following PEP 249
- **Zero-copy Arrow** — Data transfer via PyCapsule Interface (PEP 3118)
- **Native integrations** — Direct Polars/pandas support
- **Async support** — Non-blocking operations with connection pooling
- **Type-safe** — Full Python type hints via inline stubs

## Architecture

This crate provides the `pyhdb_rs._core` extension module:

```
Python Application
        ↓
pyhdb_rs (Python package)
  └── _core (this crate)
        ↓
hdbconnect-arrow
        ↓
hdbconnect (Rust)
        ↓
SAP HANA Database
```

## Exposed types

### Synchronous API

- `Connection` — Database connection with context manager support
- `Cursor` — Query execution with parameter binding
- `RecordBatchReader` — Arrow data streaming via PyCapsule

### Async API

- `AsyncConnection` — Async database connection
- `AsyncCursor` — Async query execution
- `ConnectionPool` — Managed connection pool with configurable size

### Exceptions

DB-API 2.0 exception hierarchy:

- `Error` — Base exception
  - `DatabaseError` — Database-related errors
    - `DataError` — Data processing errors
    - `OperationalError` — Connection/transaction errors
    - `IntegrityError` — Constraint violations
    - `InternalError` — Internal database errors
    - `ProgrammingError` — SQL syntax errors
    - `NotSupportedError` — Unsupported operations
  - `InterfaceError` — Driver interface errors
- `Warning` — Non-fatal warnings

## Building

```bash
cd crates/hdbconnect-py

# Development build
maturin develop

# Release build
maturin build --release
```

> [!NOTE]
> Uses PyO3 ABI3 for Python 3.11+ compatibility from a single wheel.

## Testing

```bash
# Rust unit tests
cargo nextest run -p hdbconnect-py

# Python integration tests (requires wheel)
cd python
maturin develop
pytest
```

## Part of pyhdb-rs

This crate is part of the [pyhdb-rs](https://github.com/bug-ops/pyhdb-rs) workspace.

Related crates:
- [`hdbconnect-arrow`](https://github.com/bug-ops/pyhdb-rs/tree/main/crates/hdbconnect-arrow) — Arrow conversion layer

Python package:
- [`pyhdb_rs`](https://pypi.org/project/pyhdb_rs/) — Published Python package on PyPI

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
