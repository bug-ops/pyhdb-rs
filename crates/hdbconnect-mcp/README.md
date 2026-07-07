# hdbconnect-mcp

[![Crates.io](https://img.shields.io/crates/v/hdbconnect-mcp)](https://crates.io/crates/hdbconnect-mcp)
[![docs.rs](https://img.shields.io/docsrs/hdbconnect-mcp)](https://docs.rs/hdbconnect-mcp)
[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](../../LICENSE-APACHE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://www.rust-lang.org)
[![CI](https://github.com/bug-ops/pyhdb-rs/workflows/CI/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/branch/main/graph/badge.svg?flag=hdbconnect-mcp)](https://codecov.io/gh/bug-ops/pyhdb-rs)

MCP (Model Context Protocol) server providing AI assistants with secure, programmatic access to SAP HANA databases.

## Features

- **Interactive Parameter Collection** — Elicits missing schema names and DML/procedure confirmations from users during tool execution
- **8 Tools** — Schema exploration (`ping`, `list_tables`, `describe_table`), querying (`execute_sql`), guarded writes (`execute_dml`), and stored procedures (`list_procedures`, `describe_procedure`, `call_procedure`)
- **Security First** — Read-only by default; DML and procedure execution are opt-in with per-operation confirmation, row/result-set limits, and mandatory `WHERE` clauses for `UPDATE`/`DELETE`
- **Connection Pooling** — Efficient resource management with deadpool
- **Full JSON Schema** — AI-discoverable tool definitions with comprehensive descriptions
- **Zero Unsafe Code** — Memory-safe Rust implementation

## Installation

### From crates.io

```bash
cargo install hdbconnect-mcp
```

### From source

```bash
git clone https://github.com/bug-ops/pyhdb-rs.git
cd pyhdb-rs/crates/hdbconnect-mcp
cargo install --path .
```

> [!IMPORTANT]
> Requires Rust 1.88 or later. See the [`hdbconnect-arrow` MSRV policy](../hdbconnect-arrow/README.md#msrv-policy) (this crate shares the workspace MSRV).

## Quick Start

### Standalone Server

```bash
hdbconnect-mcp \
  --url "hdbsql://user:password@host:39017" \
  --row-limit 10000 \
  --pool-size 4
```

Read-only mode is the default — the server only accepts `SELECT`/`WITH`/`EXPLAIN`/`CALL` until you explicitly opt in to writes (see [Configuration](#configuration)).

### Claude Desktop Integration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "hana": {
      "command": "hdbconnect-mcp",
      "args": [
        "--url", "hdbsql://user:password@host:39017",
        "--row-limit", "10000"
      ]
    }
  }
}
```

> [!TIP]
> `--url` also reads from the `HANA_URL` environment variable, so credentials don't need to live in the args array: `"env": { "HANA_URL": "hdbsql://user:password@host:39017" }`

## Configuration

Precedence (highest to lowest): **environment variables** > **config file** (`--config`) > **CLI flags** > defaults.

### Connection & Query

| Flag | Default | Description |
|------|---------|-------------|
| `--url` / `-u` (env `HANA_URL`) | required | HANA connection URL (`hdbsql://user:password@host:port`) |
| `--row-limit` / `-l` | `10000` | Maximum rows per query result |
| `--pool-size` / `-p` | `4` | Database connection pool size |
| `--query-timeout` | `30` | Query timeout in seconds |
| `--schema-filter-mode` | `none` | Schema visibility: `none`, `whitelist`, or `blacklist` |
| `--schema-filter-schemas` | — | Comma-separated schema names for the filter mode above |

### Safety

| Flag | Default | Description |
|------|---------|-------------|
| `--no-read-only` | off (read-only) | Disable read-only mode — required before `execute_dml`/`call_procedure` can run |
| `--allow-dml` | `false` | Enable `execute_dml` (INSERT/UPDATE/DELETE) |
| `--no-dml-confirm` | `false` | Skip the interactive confirmation prompt before DML runs |
| `--dml-max-rows` | `1000` | Maximum rows a single DML statement may affect |
| `--no-where-clause` | `false` | Allow `UPDATE`/`DELETE` without a `WHERE` clause |
| `--dml-ops` | all allowed | Comma-separated allowlist: `insert,update,delete` |
| `--allow-procedures` | `false` | Enable `call_procedure` |
| `--no-procedure-confirm` | `false` | Skip the interactive confirmation prompt before a procedure call |
| `--procedure-max-result-sets` | `10` | Maximum result sets returned by a procedure call |
| `--procedure-max-rows` | `1000` | Maximum rows per procedure result set |

> [!WARNING]
> `--no-read-only` alone does nothing — DML and procedure calls stay disabled until you also pass `--allow-dml` / `--allow-procedures`. Only combine these on trusted, scoped deployments.

### Transport & Logging

| Flag | Default | Description |
|------|---------|-------------|
| `--transport` | `stdio` | `stdio` or `http` |
| `--http-host` | `127.0.0.1` | HTTP bind address (`transport=http`, requires the `http` feature) |
| `--http-port` | `8080` | HTTP bind port |
| `--config` / `-c` | — | Load configuration from a file (see [`config::file`](src/config/file.rs)) |
| `--verbose` / `-v` | `false` | Enable debug logging |
| `--json-logs` | `false` | Emit structured JSON logs instead of plain text |

## MCP Tools

### `ping`

Check database connection health and measure latency.

**Returns:**
```json
{
  "status": "ok",
  "latency_ms": 12
}
```

### `list_tables`

List tables in a schema with interactive elicitation for missing schema parameter.

**Parameters:**
- `schema` (optional) — Schema name (defaults to `CURRENT_SCHEMA`)

**Returns:**
```json
[
  { "name": "EMPLOYEES", "table_type": "TABLE" },
  { "name": "ORDERS", "table_type": "TABLE" }
]
```

### `describe_table`

Get column definitions for a table with type information.

**Parameters:**
- `table` (required) — Table name
- `schema` (optional) — Schema name (elicited if missing)

**Returns:**
```json
{
  "table_name": "EMPLOYEES",
  "columns": [
    { "name": "ID", "data_type": "INTEGER", "nullable": false },
    { "name": "NAME", "data_type": "NVARCHAR", "nullable": true }
  ]
}
```

### `execute_sql`

Execute SQL queries with configurable row limits.

**Parameters:**
- `sql` (required) — SQL query (SELECT/WITH/EXPLAIN/CALL in read-only mode)
- `limit` (optional) — Row limit (overrides server default)

**Returns:**
```json
{
  "columns": ["ID", "NAME", "DEPARTMENT"],
  "rows": [
    [1, "Alice", "Engineering"],
    [2, "Bob", "Sales"]
  ],
  "row_count": 2
}
```

> [!CAUTION]
> In read-only mode (default), only `SELECT`, `WITH`, `EXPLAIN`, and `CALL` statements are permitted. Other SQL commands will be rejected.

### `execute_dml`

Execute `INSERT`, `UPDATE`, or `DELETE` with safety checks. Disabled by default — requires `--no-read-only --allow-dml`.

**Parameters:**
- `sql` (required) — DML statement
- `schema` (optional) — Schema context (defaults to `CURRENT_SCHEMA`)
- `force` (optional) — Skip the interactive confirmation prompt

**Returns:**
```json
{
  "operation": "UPDATE",
  "affected_rows": 3,
  "status": "success"
}
```

> [!CAUTION]
> `UPDATE`/`DELETE` require a `WHERE` clause unless `--no-where-clause` is set. Statements are rejected outright if the operation isn't in `--dml-ops`, and (unless `--no-dml-confirm` or `force: true`) the client must confirm via MCP elicitation before anything runs.

### `list_procedures`

List stored procedures and functions in a schema.

**Parameters:**
- `schema` (optional) — Schema name (defaults to `CURRENT_SCHEMA`)
- `name_pattern` (optional) — SQL `LIKE` pattern, e.g. `"GET_%"`

**Returns:**
```json
[
  { "name": "GET_CUSTOMER_ORDERS", "schema": "SAPABAP1", "procedure_type": "PROCEDURE", "read_only": true }
]
```

### `describe_procedure`

Get parameter definitions for a stored procedure.

**Parameters:**
- `procedure` (required) — Procedure name
- `schema` (optional) — Schema name (defaults to `CURRENT_SCHEMA`)

**Returns:**
```json
{
  "name": "GET_CUSTOMER_ORDERS",
  "schema": "SAPABAP1",
  "parameters": [
    { "name": "CUSTOMER_ID", "position": 1, "data_type": "INTEGER", "direction": "IN" },
    { "name": "ORDER_COUNT", "position": 2, "data_type": "INTEGER", "direction": "OUT" }
  ]
}
```

### `call_procedure`

Execute a stored procedure with parameter binding. Disabled by default — requires `--no-read-only --allow-procedures`.

**Parameters:**
- `procedure` (required) — Procedure name, `SCHEMA.PROCEDURE` or `PROCEDURE`
- `parameters` (optional) — Input parameters as a JSON object
- `schema` (optional) — Schema context if not embedded in `procedure`
- `explicit_transaction` (optional) — Disable auto-commit for this call
- `force` (optional) — Skip the interactive confirmation prompt

**Returns:**
```json
{
  "procedure": "GET_CUSTOMER_ORDERS",
  "status": "success",
  "result_sets": [
    { "index": 0, "columns": ["ORDER_ID", "TOTAL"], "rows": [[1001, 249.99]], "row_count": 1 }
  ]
}
```

> [!CAUTION]
> Unless `--no-procedure-confirm` or `force: true` is set, the client must confirm via MCP elicitation before the procedure runs. Result sets are capped by `--procedure-max-result-sets` and `--procedure-max-rows`.

## Features

### Default

Standard stdio transport for MCP integration.

### `http`

Enable HTTP transport for remote MCP access.

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["http"] }
```

Run with `--transport http --http-host 0.0.0.0 --http-port 8080`. Combine with the `auth` feature for OIDC/JWT-protected remote access; see [`deploy/`](deploy/) for a sample systemd unit and Kubernetes manifests.

### `cache`

Enable in-memory caching for schema metadata and query results.

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["cache"] }
```

#### Cache Deployment Notes

**Performance benefits:**
- Schema metadata cached for 1 hour (configurable)
- Query results cached for 60 seconds (configurable)
- Cache hit latency: 3-6 microseconds vs database round-trip

**Multi-tenant safety:**
When combined with `auth` feature, cache implements per-user isolation:
- Different users get different cache keys (includes `user_id` hash)
- User A cannot read User B's cached query results
- Cache poisoning attacks affect only attacker's own cache entries

**Single-user deployments:**
For stdio transport or when auth is disabled, all queries use `_system` user. This is safe and recommended for personal AI assistants or service accounts.

**Schema staleness:**
DDL changes (ALTER TABLE, DROP COLUMN) may not be reflected until TTL expires. For environments with frequent schema changes, reduce TTL or disable caching.

### `auth`

Enable OIDC/JWT authentication and multi-tenant support.

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["auth", "http", "cache"] }
```

**Security features:**
- JWT validation with RS256/ES256/HS256 algorithms
- OIDC discovery support
- Multi-tenant schema isolation via JWT claims
- Per-user cache isolation when combined with `cache` feature
- Role-based access control (RBAC) foundations

### `metrics`

Expose Prometheus metrics via the `metrics`/`metrics-exporter-prometheus` crates (requires `http`).

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["metrics", "http"] }
```

### `telemetry`

Export distributed traces over OTLP using `opentelemetry`/`tracing-opentelemetry`.

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["telemetry"] }
```

## Architecture

```
┌──────────────────┐
│   MCP Client     │  (Claude Desktop, Cline, etc.)
└─────────┬────────┘
          │ stdio/HTTP
┌─────────▼────────┐
│  hdbconnect-mcp   │
│  ├─ server.rs     │  Tool handlers with elicitation
│  ├─ pool.rs       │  Connection pooling
│  ├─ config/       │  CLI/env/file configuration, precedence merging
│  ├─ types.rs      │  JSON Schema types for tool I/O
│  ├─ validation.rs │  SQL safety checks (read-only, DML, WHERE clause)
│  ├─ auth/         │  OIDC/JWT (feature "auth")
│  ├─ cache/        │  Per-user result caching (feature "cache")
│  └─ observability/│  Tracing, metrics, OTLP export
└─────────┬─────────┘
          │ hdbconnect_async
┌─────────▼────────┐
│   SAP HANA DB    │
└──────────────────┘
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development guidelines.

### Development

```bash
# Build
cargo build -p hdbconnect-mcp

# Run tests
cargo nextest run -p hdbconnect-mcp

# Lint
cargo clippy -p hdbconnect-mcp -- -D warnings

# Format
cargo +nightly fmt --check
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
