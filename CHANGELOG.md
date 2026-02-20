# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.6] - 2026-02-20

### Fixed

- **hdbconnect-py**: Migrate tests from removed PyO3 0.28 APIs (`prepare_freethreaded_python`, `Python::with_gil`) to `Python::try_attach`
- **python**: Sync pyproject.toml version to 0.3.6 (was missed in release)

### Changed

- **Dependencies**: Bump rmcp 0.15 → 0.16

## [0.3.5] - 2026-02-16

### Fixed

- **hdbconnect-mcp**: Resolve runtime, logging, and error tracing issues (#91)
  - Add explicit Tokio runtime to deadpool pool builder (fixes `NoRuntimeSpecified` panic)
  - Redirect tracing logs to stderr (fixes Claude Desktop JSON-RPC parsing errors)
  - Replace `--read-only` flag with `--no-read-only` (read-only is now default)
  - Handle empty schema input from elicitation as `CURRENT_SCHEMA`
  - Add error tracing for database operations diagnostics

### Changed

- **Dependencies**: Bump pyo3 0.27 → 0.28, pyo3-arrow 0.15 → 0.16, pyo3-async-runtimes 0.27 → 0.28
- **Dependencies**: Bump rmcp 0.14 → 0.15, toml 0.9 → 1.0, and transitive deps (time, bytes, rust-minor-patch group)
- **PyO3 0.28 migration**: Add explicit `from_py_object`/`skip_from_py_object` annotations to all `#[pyclass]` types with `Clone`
- **Documentation**: Restructure main README for target audience (SAP consultants, data engineers, ML practitioners)

### Added

- **CI**: `CI Gate` aggregation job for branch protection required status checks
- **CI**: CodSpeed benchmarks switched to weekly schedule instead of per-PR runs (#90)
- **Branch protection**: Enforce `CI Gate` as required check on `main`

## [0.3.4] - 2026-02-02

### Added

- **CodSpeed Integration**: Continuous performance benchmarking
  - CPU simulation benchmarks (instruction counting for deterministic results)
  - Memory profiling (heap allocation tracking)
  - Walltime benchmarks (real execution time measurement)
  - Python benchmarks via pytest-codspeed
  - CodSpeed badge in README

- **hdbconnect-mcp Phase 4.5**: Configuration reload infrastructure
  - `RuntimeConfig` type for hot-reloadable parameters (row_limit, query_timeout, log_level, cache TTL)
  - `RuntimeConfigHolder` with lock-free reads via `arc-swap`
  - `POST /admin/reload` HTTP endpoint (requires authentication)
  - `ReloadTrigger` enum for audit logging (Signal, HttpEndpoint, Manual)
  - Separation of static vs runtime config parameters

- **hdbconnect-mcp Phase 4**: Prometheus metrics infrastructure (`metrics` feature)
  - Server metrics: uptime, version info, request counter
  - Query performance: duration histogram, query count, row count, error tracking
  - Cache observability: hit/miss counters, eviction tracking, size gauge
  - Connection pool statistics: pool size (max/available/in_use/waiting), wait time histogram, error counter
  - HTTP `/metrics` endpoint for Prometheus scraping (requires `http` feature)
  - Zero-cost when feature disabled

- **CI/CD**: MCP server binary releases with trusted publishing
  - Cross-platform binaries (Linux x86_64/aarch64, macOS x86_64/ARM64, Windows x86_64)
  - Static musl builds for Linux (no glibc dependency)
  - SHA256 checksums for all binary artifacts
  - Automated crates.io publication via OIDC trusted publishing
  - Binary stripping for reduced file size
  - Cross-compilation support via `cross` tool for Linux ARM64
  - Installation instructions auto-appended to GitHub Releases
  - Version validation (tag must match Cargo.toml)

### Changed

- **CI**: Migrate to moonrepo/setup-rust for unified Rust toolchain management
- **Tests**: Improved code coverage to ~87% with comprehensive unit tests for config, auth, helpers, security, and observability modules

### Dependencies

- Updated GitHub Actions dependencies (actions/checkout, actions/setup-python, actions/upload-artifact)
- Added `pytest-codspeed` for Python benchmark integration
- Replaced `criterion` with `codspeed-criterion-compat` for Rust benchmarks

## [0.3.3] - 2026-02-02

### Added

- **Cursor.nextset()**: Full multiple result sets support for stored procedures (#58)
  - `nextset()` returns `True` when next result set available, `False` otherwise
  - Works with `callproc()` for procedures returning multiple result sets
  - Supports forward-only navigation through result sets
  - Type stubs updated with comprehensive documentation and examples

- **hdbconnect-mcp Phase 2**: Production-ready MCP server with HTTP transport, security hardening, and observability
  - **Configuration Module** (`src/config/`): Layered configuration with precedence (env > file > CLI > defaults)
    - TOML config file support with auto-discovery (`./hdbconnect-mcp.toml`, `~/.config/`, `/etc/`)
    - `ConfigBuilder` with fluent API for programmatic configuration
    - Environment variables: `HANA_URL`, `MCP_*`, `OTEL_*`, `CORS_ALLOW_ORIGIN`
  - **Security Module** (`src/security/`): Schema access control and query protection
    - `SchemaFilter` enum with Whitelist/Blacklist/AllowAll modes
    - `QueryGuard` with timeout wrapper and schema validation
    - SQL identifier validation (prevents injection via schema/table names)
    - Comment stripping (prevents read-only bypass via `--` or `/* */`)
    - CTE-aware read-only validation (detects DML after `WITH` clauses)
    - Case-insensitive schema matching
  - **HTTP Transport** (`src/transport/http.rs`): Feature-gated HTTP/SSE transport (`http` feature)
    - Axum HTTP server with rmcp `StreamableHttpService` integration
    - Health endpoint at `/health`, MCP over SSE at `/mcp`
    - Bearer token authentication via `MCP_HTTP_BEARER_TOKEN` env var
    - Restrictive CORS (default: `http://localhost:3000`, configurable via `MCP_CORS_ORIGIN`)
    - Security warnings for non-loopback binding without authentication
    - CORS and timeout middleware via tower-http
  - **Observability** (`src/observability/`): Feature-gated telemetry (`telemetry` feature)
    - OpenTelemetry OTLP trace export
    - JSON logging option
    - Graceful fallback to basic logging when telemetry disabled
  - **Deployment Artifacts**: Production-ready container and orchestration configs
    - Multi-stage Dockerfile with distroless runtime
    - Default bind to `127.0.0.1` (localhost) for security
    - Security entrypoint warnings for `0.0.0.0` binding
    - systemd service unit with hardening (`deploy/hdbconnect-mcp.service`)
    - Kubernetes manifests: Deployment, Service, ConfigMap (`deploy/k8s/`)
  - **SQL Validation**: Extended write keyword detection (MERGE, UPSERT, CALL, EXEC, EXECUTE)
  - **Error Types**: Added `QueryTimeout`, `SchemaAccessDenied`, `Transport`, `InvalidIdentifier` error variants

- **hdbconnect-mcp Phase 3.1**: DML operations support (Issue #67)
  - **New Tools**: `insert_rows`, `update_rows`, `delete_rows` for database modifications
  - **Security**: DML disabled by default, explicit `allow_dml = true` required
  - **Configuration**: `DmlConfig` with granular operation control (INSERT/UPDATE/DELETE)
  - **Validation**: SQL injection prevention via control character filtering
  - **Error Handling**: 6 new error types (`DmlDisabled`, `DmlOperationNotAllowed`, etc.)
  - **Testing**: 27 unit tests for sanitization and validation
  - **Documentation**: Comprehensive security notes and usage examples

- **hdbconnect-mcp Phase 3.2**: Stored procedure execution support (Issue #67)
  - **New Tools**: `list_procedures`, `describe_procedure`, `call_procedure`
  - **Security**: Procedures disabled by default, explicit `allow_procedures = true` required
  - **Parameter Support**: IN/OUT/INOUT parameters with type conversion
  - **Multi-Result Sets**: Support for procedures returning multiple result sets
  - **Validation**: LIKE pattern validation, procedure name sanitization
  - **Configuration**: `ProcedureConfig` with security documentation
  - **Error Handling**: 6 new error types (`ProcedureDisabled`, `ProcedureNotFound`, etc.)
  - **Testing**: 27 unit tests for security and validation

- **hdbconnect-mcp Phase 3.3**: Pluggable cache abstraction layer (Issue #67)
  - **Cache Feature**: Optional `cache` feature flag (disabled by default, zero overhead when disabled)
  - **CacheProvider Trait**: Async trait with 9 operations (get, set, delete, exists, delete_by_prefix, metadata, clear, health_check, stats)
  - **Built-in Providers**: `NoopCache` (disabled), `InMemoryCache` (TTL + LRU eviction), `TracedCache` (observability wrapper)
  - **Type-Safe Keys**: `CacheKey` with namespace isolation (TableSchema, ProcedureList, QueryResult, etc.)
  - **Security**: Value size limit (1MB default), config validation warnings, debug-level logging to prevent schema exposure
  - **Configuration**: `CacheConfig` with TTL strategies, max entries, max value size
  - **Performance**: 6.15M ops/sec throughput, <5ns trait overhead, zero regressions
  - **Testing**: 91% coverage (303 tests), comprehensive benchmarks with Criterion
  - **Documentation**: Complete API docs, performance characteristics, security notes

- **hdbconnect-mcp Phase 3.4**: Cache integration with MCP server tools (Issue #67)
  - **Cached Tools**: Integrated cache with `list_tables`, `describe_table`, `list_procedures`, `describe_procedure`, `execute_sql` (read-only queries)
  - **cached_or_fetch Helper**: Generic async pattern for cache-first lookup with database fallback
  - **TTL Strategy**: Schema metadata (1 hour), query results (1 minute)
  - **Graceful Degradation**: Cache errors never fail operations, always fall back to fresh database queries
  - **Collision Mitigation**: Query cache keys include sql_hash + sql_len for disambiguation
  - **Performance**: Cache hit 3-6μs (170x better than target), miss overhead ~3.6μs (27x better than target), NoopCache 45ns
  - **Feature Gating**: All cache integration behind `#[cfg(feature = "cache")]` with zero overhead when disabled
  - **Documentation**: Single-user deployment limitation documented in README, multi-user considerations, schema staleness behavior
  - **Testing**: 309 tests passing, 98-100% cache module coverage, 5 tests for cached_or_fetch helper
  - **Benchmarks**: Comprehensive Criterion benchmarks for cache hit/miss scenarios and concurrent access patterns

- **hdbconnect-mcp Phase 3.5**: Enhanced authentication with OIDC/JWT support (Issue #67)
  - **Authentication Module** (`src/auth/`): Comprehensive OIDC and JWT authentication
    - OIDC discovery via `openidconnect` crate (CoreProviderMetadata)
    - JWT validation with `jsonwebtoken` supporting RS256, ES256, HS256 algorithms
    - Custom claims via `AdditionalClaims` trait (tenant_id, roles)
    - JWKS caching with automatic background refresh
    - Configurable clock skew tolerance and audience validation
  - **Multi-Tenant Support**: Tenant resolution from JWT claims
    - Extract tenant_id from configurable claim path
    - Role-based access control (RBAC) foundations
    - Tenant-aware schema filtering (future integration)
  - **HTTP Middleware** (`auth/middleware.rs`): Bearer token authentication
    - Axum middleware for JWT validation on HTTP requests
    - AuthenticatedUser context injection into request extensions
    - 401 Unauthorized on missing/invalid tokens
  - **BREAKING CHANGE**: Cache keys now require `user_id` parameter
    - `CacheKey::query_result(sql, limit, user_id)` - user_id is mandatory
    - Multi-tenant cache isolation when auth enabled
    - Falls back to `CACHE_SYSTEM_USER` for single-tenant deployments
  - **Dependencies**:
    - `openidconnect = "4.0.1"` for OIDC protocol
    - `jsonwebtoken = "9.3"` for JWT validation
  - **Performance**: JWT validation 50-500μs, JWKS fetch 50-100ms, middleware overhead 50-500μs
  - **Testing**: 389 tests passing, auth module 82-100% coverage (excluding network-dependent code)
  - **Security**: All JWT validation checks pass (signature, expiration, issuer, audience)

- **hdbconnect-mcp Phase 3.6**: Per-user cache isolation via RequestContext extensions (Issue #67)
  - **Multi-Tenant Cache Isolation**: Automatic user_id extraction from MCP RequestContext for per-user cache keys
    - Leverages rmcp 0.14 native extension propagation (HTTP request Parts → MCP extensions)
    - `extract_user_id()` helper extracts `AuthenticatedUser.sub` from nested extensions
    - `execute_sql` uses dynamic user_id in `CacheKey::query_result()` calls
    - Different users get different cache keys (hash includes user_id)
  - **Security Guarantees**:
    - User A cannot read User B's cached query results (different cache keys)
    - Cache poisoning affects only attacker's own cache entries
    - Zero vulnerabilities found in security audit
  - **Performance**:
    - `extract_user_id()` fully inlined by compiler (zero overhead)
    - User_id hashing adds ~90ns overhead (acceptable for security)
    - Cache remains enabled (no performance degradation vs disabling cache)
  - **Fallback Behavior**:
    - Non-HTTP transports (stdio) use `CACHE_SYSTEM_USER` constant
    - HTTP without auth uses `CACHE_SYSTEM_USER` (single-user scenario)
    - HTTP with auth extracts `user.sub` for per-user isolation
  - **New Module**: `auth/user_context.rs` with extraction helper and 7 unit tests
  - **Documentation**: Updated cache module docs to explain per-user isolation guarantees
  - **Testing**: 396 tests passing (+4 edge case tests), 92.99% coverage on user_context.rs
  - **Validation**: Security audit PASS, performance validation PASS (zero-cost abstraction), code review APPROVED

### Changed

- **hdbconnect-mcp Phase 3.7**: Documentation updates for Phase 3 completion (Issue #67)
  - **README Updates**: Removed outdated multi-user cache limitation warnings
    - Added per-user cache isolation documentation (Phase 3.6 feature)
    - Added `auth` feature section with security capabilities
    - Updated cache deployment notes with multi-tenant safety guarantees
  - **Project Status**: All Phase 3 enterprise features completed
    - Phase 3.1: DML operations (PR #72)
    - Phase 3.2: Stored procedures (PR #73)
    - Phase 3.3: Cache abstraction (PR #75)
    - Phase 3.4: Cache integration (PR #76)
    - Phase 3.5: OIDC/JWT authentication (PR #77)
    - Phase 3.6: Per-user cache isolation (PR #78)
    - Phase 3.7: Documentation cleanup (current)

### Security

- **hdbconnect-mcp**: Fixed 5 security vulnerabilities identified in Phase 2 review
  - Fixed permissive CORS configuration (CVE-style: SEC-001)
  - Added Bearer token authentication for HTTP transport (SEC-002)
  - Fixed SQL injection via identifier validation (SEC-003)
  - Fixed read-only bypass via CTE and comments (SEC-004)
  - Fixed Docker image binding to all interfaces by default (SEC-005)

- **Connection Statistics API - Full** (Issue #57 / Issue #54 Phase 4): Comprehensive connection performance monitoring
  - `Connection.statistics()` - Get performance statistics snapshot (`ConnectionStatistics` object)
  - `Connection.reset_statistics()` - Reset statistics counters to zero
  - `ConnectionStatistics` dataclass with 8 fields:
    - `call_count` - Number of roundtrips to server
    - `accumulated_wait_time` - Total wait time in milliseconds
    - `compressed_requests_count`, `compressed_requests_compressed_size`, `compressed_requests_uncompressed_size`
    - `compressed_replies_count`, `compressed_replies_compressed_size`, `compressed_replies_uncompressed_size`
  - `ConnectionStatistics` computed properties:
    - `avg_wait_time` - Average latency per roundtrip (ms)
    - `request_compression_ratio` - Request compression efficiency (0.0-1.0)
    - `reply_compression_ratio` - Reply compression efficiency (0.0-1.0)
  - Full async support: `AsyncConnection`, `PooledConnection`
  - Comprehensive type stubs for IDE support in both `_core.pyi` and `aio/_core.pyi`
  - Zero-copy statistics retrieval (<1μs overhead per call)
  - Thread-safe with proper lock patterns for sync/async/pooled connections

- **Stored Procedures API** (Issue #54 Phase 3): DB-API 2.0 compliant stored procedure support
  - `Cursor.callproc(procname, parameters)` - Execute stored procedures with optional parameters
  - `Cursor.nextset()` - Skip to next result set (stub returning False, Phase 4 will implement full support)
  - Sync cursor: Full parameter support via prepared statements
  - Async cursor: Parameterless procedure calls only (parameters raise `NotSupportedError`)
  - Procedure name validation to prevent SQL injection
  - Returns input parameters unchanged per DB-API 2.0 spec
  - Comprehensive type stubs for IDE support in both `_core.pyi` and `aio/_core.pyi`
  - 12 tests covering validation, sync, async, and integration scenarios

- **Connection Statistics API** (Issue #54 Phase 2): Expose server-side connection statistics for performance monitoring
  - `Connection.connection_id()` - Get server-assigned connection ID
  - `Connection.server_memory_usage()` - Get current memory usage in bytes
  - `Connection.server_processing_time()` - Get cumulative processing time in microseconds
  - Full async support: `AsyncConnection`, `PooledConnection`
  - Comprehensive type stubs for IDE support in both `_core.pyi` and `aio/_core.pyi`
  - 15 tests covering sync, async, and pooled connections
  - Note: `server_cpu_time()` was omitted because hdbconnect's `ServerUsage.accum_cpu_time` field is private

- **Application Metadata API** (Issue #54 Phase 1): Expose application metadata features for production monitoring
  - `Connection.set_application(name)` - Set application name visible in SAP HANA `M_CONNECTIONS`
  - `Connection.set_application_user(user)` - Set application-level user distinct from DB user
  - `Connection.set_application_version(version)` - Set application version for monitoring
  - `Connection.set_application_source(source)` - Set source location for debugging
  - `Connection.client_info()` - Get client context information as `dict[str, str]`
  - `ConnectionBuilder.application(name, version, user, source)` - Set metadata during connection setup
  - Full async support: `AsyncConnection`, `AsyncConnectionBuilder`, `PooledConnection`
  - Comprehensive type stubs for IDE support in both `_core.pyi` and `aio/_core.pyi`
  - 24 tests covering sync, async, pooled connections, and builders

## [0.3.2] - 2026-01-30

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

[Unreleased]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.6...HEAD
[0.3.6]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/bug-ops/pyhdb-rs/compare/v0.3.0...v0.3.1
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
