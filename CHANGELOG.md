# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Performance**: Replaced `Box<dyn HanaCompatibleBuilder>` with enum-based builder dispatch in `hdbconnect-arrow`
  - Eliminates vtable overhead and pointer indirection for ~10-20% performance improvement
  - Introduced `BuilderEnum` with 16 variants for all supported Arrow types
  - Added `SchemaProfile` for detecting homogeneous vs mixed schemas
  - Better cache locality through contiguous memory layout in `Vec<BuilderEnum>`
  - New public API: `BuilderFactory::create_builder_enum()`, `BuilderEnum`, `BuilderKind`, `SchemaProfile`

### Added

- **Developer Tools**: Profiling infrastructure for performance analysis
  - Added optional `profiling` feature flag with dhat heap profiler integration
  - Zero impact on release builds through conditional compilation
  - Baseline profiling identified BigInt clone as optimization target (8MB per 1M decimals)
  - Comprehensive profiling methodology documentation

- **Performance**: Zero-copy decimal conversion via Cow::Borrowed optimization
  - Eliminated BigInt clone in decimal conversion using `as_bigint_and_scale()` instead of `as_bigint_and_exponent()`
  - Decimal conversion throughput: +222% (55 → 177 Melem/s)
  - Analytics workload throughput: +30% (18 → 23.7 Melem/s)
  - Memory savings: 8 MB per 1M decimals (999,990 allocations eliminated)
  - Zero-cost abstraction via std::borrow::Cow with automatic deref coercion

- **Performance**: String capacity pre-sizing via field metadata
  - Added metadata-based capacity calculation for VARCHAR/NVARCHAR builders
  - Extracts max_length from HANA field metadata for optimal buffer sizing
  - Reduces reallocation overhead during batch building (2-3 reallocations eliminated per column)
  - Safe overflow protection with saturating arithmetic and [4KB, 256MB] clamping
  - Unicode-aware: 4x multiplier for NVARCHAR (UTF-8 worst-case), 1x for VARCHAR
  - Backward compatible: graceful fallback to default capacity when metadata missing
  - Expected improvement: +5-10% on string-heavy workloads

- **Performance**: Homogeneous loop hoisting for type-specialized processing
  - Added SchemaProfile infrastructure for detecting homogeneous schemas (all columns same type)
  - Specialized processing paths for top 5 types covering 80% usage: Int64, Decimal128, Utf8, Int32, Float64
  - Eliminates per-value enum dispatch overhead by hoisting BuilderEnum match outside inner loop
  - Zero allocation overhead confirmed via dhat profiling
  - Performance gains: +4-8% on wide tables (100+ columns), +1-2% on moderate tables (10-50 columns)
  - Foundation for SIMD vectorization (Phase 8) - homogeneous type detection enables vectorized operations
  - Helper function `append_value_to_builder` reduces code duplication across specialized paths
  - Comprehensive testing: 14 new tests, 97.83% coverage in processor.rs

### Fixed

- **Performance**: Box wrapping optimization for large `BuilderEnum` variants
  - Wrapped 6 large variants (Decimal128, String, Binary builders) in `Box` to reduce enum size
  - Improved cache locality for temporal and primitive types
  - Eliminated temporal types performance regression from Phase 2 (-24% → <1%)
  - Analytics workload gains maintained (+54-60%)
  - Box indirection overhead negligible (<1%) on string/decimal operations

## [0.3.1] - 2026-01-29

### Removed

- **BREAKING**: `pyhdb_rs.pandas` module - Use `execute_arrow()` + PyArrow instead
- **BREAKING**: `pyhdb_rs.polars` module - Use `execute_arrow()` + `pl.from_arrow()` instead
- **BREAKING**: `pyhdb_rs.aio.polars` module - Use async `execute_arrow()` + `pl.from_arrow()` instead
- **BREAKING**: Convenience functions: `read_hana`, `write_hana`, `to_hana`, `scan_hana`
- **BREAKING**: `AsyncConnection.connect()` classmethod - Use `AsyncConnectionBuilder.build()` instead
- **BREAKING**: `ConnectionPool.__init__()` - Use `ConnectionPoolBuilder.build()` instead

### Changed

- **BREAKING**: Config-first API for `execute_arrow()` - `batch_size` parameter replaced with `config: ArrowConfig | None`
  - Before: `conn.execute_arrow("SELECT ...", batch_size=10000)`
  - After: `conn.execute_arrow("SELECT ...", config=ArrowConfig(batch_size=10000))`
  - Applies to: `Connection.execute_arrow()`, `Cursor.execute_arrow()`, `Cursor.fetch_arrow()`,
    `AsyncConnection.execute_arrow()`, `PooledConnection.execute_arrow()`
- **Internal Refactoring:** Comprehensive DRY (Don't Repeat Yourself) violations elimination
  - Created centralized `utils/` module to consolidate duplicate code across sync and async implementations
  - Eliminated 303 lines of duplicated code across 8 files
  - New module structure:
    - `utils/validation.rs` - Centralized `VALIDATION_QUERY` constant and validation helpers (`validate_positive_u32`, `validate_non_negative_f64`)
    - `utils/url_parser.rs` - Single `ParsedConnectionUrl` struct replacing 3 duplicate URL parsing implementations
    - `utils/tls.rs` - Unified TLS application helpers for sync/async builders
  - Added 23 comprehensive unit tests for new utilities
  - No breaking changes to public API

### Security

- **Enhanced Password Security:** `ParsedConnectionUrl` now implements custom `Debug` trait that redacts password field
  - Debug output shows `password: "[REDACTED]"` instead of plaintext password
  - Prevents accidental password exposure in logs and error messages
  - Added test coverage for password redaction

### Migration Guide

See README.md "Migration Guide: v0.3.0 to v0.3.1" section.

## [0.3.0]

### Added

- **Builder-based connection API** with full TLS support
  - `ConnectionBuilder` for sync connections with method chaining
  - `AsyncConnectionBuilder` for async connections
  - `PyConnectionPoolBuilder` for pool configuration
- **TlsConfig class** for flexible certificate configuration
  - `TlsConfig.from_directory(path)` - Load certificates from directory
  - `TlsConfig.from_environment(env_var)` - Load certificate from environment variable
  - `TlsConfig.from_certificate(pem_content)` - Load certificate from PEM string
  - `TlsConfig.with_system_roots()` - Use system root certificates
  - `TlsConfig.insecure()` - Skip verification (development only)
- Auto-TLS detection: `hdbsqls://` scheme automatically enables system root certificates
- **CursorHoldability enum** for transaction control
  - `CursorHoldability.None` - Cursor closed on commit and rollback (default)
  - `CursorHoldability.Commit` - Cursor held across commits
  - `CursorHoldability.Rollback` - Cursor held across rollbacks
  - `CursorHoldability.CommitAndRollback` - Cursor held across both operations
  - Integrated with `ConnectionBuilder`, `AsyncConnectionBuilder`
- **network_group parameter** for HANA Scale-Out and HA deployments
  - Available in `ConnectionBuilder`, `AsyncConnectionBuilder`, `ConnectionPoolBuilder`
  - Enables routing connections to specific HANA nodes in clustered environments

### Changed

- **BREAKING**: Removed deprecated `statement_cache_size` parameter from `pyhdb_rs.aio.connect()`
  - Migration: Use `ConnectionConfig(max_cached_statements=N)` instead
  - The parameter was ignored since v0.2.5 and existed only for backward compatibility

### Fixed

- Fix database field ignored in internal typestate builder (builder.rs)
  - Database name from URL or `.database()` method was not passed to hdbconnect

### Removed

- **BREAKING**: `statement_cache_size` parameter from async `connect()` function

## [0.2.5]

### Added

- `is_valid(check_connection=True)` method on all connection types for
  connection health checking. When `check_connection=True` (default),
  executes lightweight ping query (`SELECT 1 FROM DUMMY`). Available on:
  - `Connection` (sync)
  - `AsyncConnection` (async)
  - `PooledConnection` (pooled async)

### Changed

- **BREAKING**: Removed `execute_polars()` convenience method from async API (`AsyncConnection`, `PooledConnection`)
  - Aligns async API with synchronous API design
  - Use `execute_arrow()` instead and convert manually: `df = pl.from_arrow(await conn.execute_arrow(sql))`
  - Improves API consistency and reduces maintenance burden

### Deprecated

- `statement_cache_size` parameter in `AsyncConnection.connect()` is deprecated
  and ignored. Statement caching was not providing actual performance benefit
  due to hdbconnect API limitations. Will be removed in 0.3.0.
- `cache_stats()` method now returns None and is deprecated.
- `PreparedStatementCache` and `CacheStats` types are deprecated.

### Documentation

- **Async API Memory Warning:** Added prominent documentation about async API
  memory behavior. `execute_arrow()` in async mode loads all rows into memory
  before streaming batches. Use sync API for large datasets (>100K rows).
  - Module-level warning in `reader/wrapper.rs`
  - Function-level warnings on `AsyncStreamingReader::new` and `from_resultset_async`
  - README.md warning callout in async section
  - Python `pyhdb_rs.aio` module docstring warning
- **Async API Documentation:** Comprehensive documentation for async support including:
  - Connection pooling with deadpool (configurable max_size, connection_timeout)
  - Concurrent query execution patterns with asyncio.gather()
  - Transaction support (commit/rollback) in async context
  - Performance notes and best practices for high-concurrency workloads
  - Note: Statement caching documentation removed (feature deprecated)
- Expanded README.md async section with detailed usage examples (500+ words)
- Added async feature documentation to crate README (crates/hdbconnect-py/README.md)
- Python async examples with connection pooling patterns (python/README.md)
- Added `is_valid()` usage examples in README Connection Validation section

## [0.2.4] - 2026-01-28

### Changed

- **Refactoring:** Comprehensive technical debt elimination across 5 phases
  - **Connection Builder (Phase 1):** Eliminated HIGH RISK `expect()` calls in connection builder
    - Changed from panic-inducing `expect()` to defensive `ok_or_else()` with proper error handling
    - Typestate pattern enforcement ensures compile-time safety
    - Zero breaking changes, backward compatible
  - **Unsafe Send Hardening (Phase 2):** Added safety review infrastructure for unsafe Send implementations
    - Added SAFETY REVIEW dates (2026-01-28) with 6-month review cycle
    - Implemented IterationGuard for concurrent access detection in debug builds
    - Created CI workflow to monitor safety review staleness (180-day threshold)
    - Added Send verification tests and comprehensive safety documentation
    - Zero performance overhead in release builds
  - **Connection State Safety (Phase 3):** Eliminated HIGH RISK `expect()` in connection state
    - Changed `params` field from `Option<ConnectParams>` to `ConnectParams`
    - Compile-time safety guarantees (params always present by construction)
    - Reduced memory footprint (~8 bytes per connection)
    - Zero breaking changes to public API
  - **Lint Suppression Cleanup (Phase 4):** Documented and refined module-level lint suppressions
    - Reduced module-level suppressions from 11 to 6 (-45%)
    - Added 100% documentation coverage for all suppressions
    - Narrowed suppressions to 13 precise item-level locations
    - Fixed `clippy::doc_markdown` warnings
    - Improved code maintainability and reduced risk of masking real issues
  - **PyO3 API Migration (Phase 5):** Migrated from deprecated API to PyO3 0.27+ compliance
    - Replaced deprecated `PySequence::downcast()` with modern `Bound::cast()`
    - Eliminated deprecation warnings
    - Zero performance impact (semantically identical)
    - Type safety and error handling maintained

### Added

- CI workflow for safety review monitoring (`.github/workflows/safety-audit.yml`)
  - Monitors SAFETY REVIEW dates in unsafe Send implementations
  - Warns when reviews exceed 180 days (non-blocking)
  - Cross-platform date parsing (GNU and BSD)
- Compile-time Send verification tests for StreamingReader types
- Comprehensive safety documentation for async_support module

### Fixed

- Eliminated 2 HIGH RISK production panic paths in connection builder and state
- Removed all deprecated API usage for PyO3 0.27+ compatibility

### CI/CD

- Added cargo caching for GitHub Actions workflows
  - **build-wheels job (CI):** Separate caches per cross-platform target (7 architectures)
  - **build-wheels job (release):** Same optimization for release builds
  - **publish-crates job:** Cargo registry operations with cache
  - Achieves ~80% build time reduction on cache hit (~8-12 min → ~1-2 min per target)
  - Workspace mapping ensures compiled binaries are cached correctly

## [0.2.3] - 2026-01-22

### Changed

- **BREAKING**: Removed `execute_polars()` convenience methods from Connection and Cursor
  - Use universal `execute_arrow()` API instead: `reader = cursor.execute_arrow(sql)`
  - Convert to desired format: `df = pl.from_arrow(reader)` for Polars, `pa.RecordBatchReader.from_stream(reader)` for PyArrow
  - More extensible and maintains cleaner API surface
- Added `execute_arrow()` method to Cursor for consistency with Connection API
- Updated all example notebooks to use universal Arrow API
- Updated README documentation to reflect new API patterns

### Fixed

- `__arrow_c_stream__()` method signature now compatible with Polars via `pl.from_arrow()`
  - Added default parameter `requested_schema=None` for proper Arrow C Stream protocol
  - Fixes TypeError when Polars invokes the method without arguments
  - Enables seamless zero-copy integration: `df = pl.from_arrow(cursor.execute_arrow(sql))`

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

[0.3.1]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.5...v0.3.0
[0.2.5]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/bug-ops/pyhdb-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bug-ops/pyhdb-rs/releases/tag/v0.1.0
