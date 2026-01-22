# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-01-22

### Fixed

- Ruff linting issues in test suite (B009: unnecessary getattr with constant attribute)
- Code formatting compliance in test files

## [0.2.1] - 2026-01-22

### Added

- Detailed HANA server error reporting with error code, severity, SQLSTATE, and query position
  - Replace generic "Database server responded with an error" messages
  - Include HANA error codes (e.g., [260] for table not found)
  - Show SQLSTATE for SQL standard error documentation
  - Display error position in SQL query for syntax errors
- Arrow PyCapsule Interface (__arrow_c_stream__) for zero-copy integration
  - Seamless integration with Polars, PyArrow, pandas, and other Arrow-compatible libraries
  - Delegates to pyo3_arrow for FFI safety
  - Implements consumption semantics (single-use pattern)

### Changed

- Enhanced error messages automatically visible in Python via PyO3 conversion
- Improved user debugging experience with actionable error information
- Errors now map to correct DB-API 2.0 exception types (ProgrammingError, IntegrityError, DataError)

### Fixed

- Error formatting now uses efficient `std::fmt::Write` instead of repeated `format!` + `push_str`

### Quality

- Performance review: PASS (cold path, zero impact on hot paths)
- Security review: PASS (0 vulnerabilities, DB-API 2.0 compliant)
- Code review: PASS (all lints resolved, rustfmt compliant)

## [0.2.0] - 2026-01-13

### Changed

- **BREAKING**: Minimum Python version raised from 3.11 to 3.12
- **BREAKING**: PyO3 ABI changed from abi3-py311 to abi3-py312
- Added Python 3.14 support to test matrix and classifiers
- Updated ruff and mypy target-version to py312

### CI/CD

- Python test matrix now covers 3.12, 3.13, 3.14 (9 combinations)
- Reduced artifact retention from 90 days to 3 days

## [0.1.3] - 2026-01-13

### Added

- Linux musl support (x86_64, aarch64) for Alpine and static builds
- Jupyter notebook examples in `examples/notebooks/`
  - Quickstart guide
  - Advanced Polars analytics
  - Streaming large datasets
  - Performance comparison vs hdbcli

### Changed

- **BREAKING**: Minimum Supported Rust Version raised from 1.85 to 1.88
- Refactored nested if-let patterns to use let chains (Rust 1.88 feature)
- README updated with platform support table and uv commands
- Installation examples now use uv instead of pip

### CI/CD

- Expanded build matrix to 9 cross-platform targets
- Added macOS and Windows to Python test matrix (9 combinations)
- Enabled sccache with GitHub Actions cache backend
- Added `generate-import-lib` feature for PyO3 cross-compilation
- Improved caching with `uv sync --frozen` and cache suffixes

## [0.1.2] - 2026-01-11

### Performance

#### Decimal Conversion
- Replaced string-based decimal parsing with direct `BigDecimal::as_bigint_and_exponent()`
- Eliminates ~100K heap allocations per 100K decimal values
- ~2x faster decimal conversion for bulk workloads

#### Arrow Builder Optimization
- Builder reuse at batch boundaries (Arrow builders reset after `finish()`)
- Removed unnecessary factory field from `HanaBatchProcessor`
- Added `Vec::with_capacity()` hints in `create_builders_for_schema()`
- 10-15% throughput improvement for batch processing

#### PyO3 Optimizations
- Thread-local caching for Python datetime/decimal types
- Eliminates repeated `py.import()` and `getattr()` calls per row
- Safe `RefCell` pattern with `try_borrow_mut()` for reentrancy protection

#### Build
- Added optimized release profile with LTO (`lto = "fat"`)
- Single codegen unit for maximum optimization
- Strip symbols for smaller binary size
- Added `bench` and `release-with-debug` profiles

### Added

- Comprehensive Criterion benchmark suite for Arrow conversions
- Benchmarks for Int32, Int64, String, Boolean, Float64, Decimal types
- Mixed schema and batch size comparison benchmarks
- Builder creation and null handling benchmarks

### Changed

- CI now excludes `hdbconnect-py` from cargo test (PyO3 abi3 requires Python at runtime)

## [0.1.1] - 2026-01-11

### Changed

#### Architecture
- Refactored Python utilities into shared `_utils.py` module for code deduplication
- Introduced `TypeCategory` enum for centralized HANA type classification
- Added `NonZeroUsize` for type-safe batch size configuration
- Implemented `RowLike` trait abstraction for testing without HANA connection
- Added `MockRow`/`MockRowBuilder` test utilities (behind `test-utils` feature flag)
- Created `impl_field_metadata_ext!` macro for DRY trait implementations

#### Testing
- Added comprehensive unit tests for `_utils.py` module (25 tests)
- Improved test coverage for PyO3 bindings
- Added pytest tests for module imports, exceptions, and DB-API types

#### CI/CD
- Enabled sccache for maturin builds (faster CI compilation)
- Added ruff format check to CI pipeline

### Security
- MockRow/MockRowBuilder now properly gated behind `test-utils` feature
- Added SQL identifier length validation (127 char HANA limit)

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

[Unreleased]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bug-ops/pyhdb-rs/releases/tag/v0.1.0
