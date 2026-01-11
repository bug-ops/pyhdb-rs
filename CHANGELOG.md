# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-11

Initial release of pyhdb-rs — high-performance Python driver for SAP HANA.

### Added

#### Core
- Full DB-API 2.0 (PEP 249) compliance with `Connection`, `Cursor` classes
- Zero-copy Apache Arrow data transfer via PyCapsule Interface (PEP 3118)
- Support for Python 3.11, 3.12, 3.13
- Cross-platform wheels for Linux (x86_64, aarch64), macOS (x86_64, ARM64), Windows (x86_64)

#### Data Integration
- Native Polars integration via `pyhdb_rs.polars.read_hana()`
- Native pandas integration via `pyhdb_rs.pandas.read_hana()`
- Direct Arrow RecordBatch streaming for memory-efficient large result sets

#### Async Support
- Async/await API via `pyhdb_rs.aio` module
- Connection pooling with configurable min/max connections
- Prepared statement caching for improved performance

#### Type Mapping
- HANA INT/BIGINT → Arrow Int32/Int64
- HANA DECIMAL → Arrow Decimal128 with precision/scale
- HANA VARCHAR/NVARCHAR → Arrow Utf8
- HANA CLOB/NCLOB → Arrow LargeUtf8
- HANA BLOB → Arrow LargeBinary
- HANA DATE → Arrow Date32
- HANA TIMESTAMP → Arrow Timestamp(Nanosecond)
- HANA GEOMETRY/POINT → Arrow Binary (WKB format)

#### Developer Experience
- Full type hints with inline stubs (PEP 561)
- Comprehensive error hierarchy (`DatabaseError`, `InterfaceError`, `OperationalError`)
- Context manager support for automatic resource cleanup

### Security

- Trusted publishing for PyPI and crates.io via OIDC
- Build provenance attestations for all release artifacts
- Dependency auditing with cargo-deny

[Unreleased]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bug-ops/pyhdb-rs/releases/tag/v0.1.0
