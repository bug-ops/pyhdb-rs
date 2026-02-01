# hdbconnect-mcp

[![Crates.io](https://img.shields.io/crates/v/hdbconnect-mcp)](https://crates.io/crates/hdbconnect-mcp)
[![docs.rs](https://img.shields.io/docsrs/hdbconnect-mcp)](https://docs.rs/hdbconnect-mcp)
[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](../LICENSE-APACHE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://www.rust-lang.org)
[![CI](https://github.com/bug-ops/pyhdb-rs/workflows/CI/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/branch/main/graph/badge.svg?flag=hdbconnect-mcp)](https://codecov.io/gh/bug-ops/pyhdb-rs)

MCP (Model Context Protocol) server providing AI assistants with secure, programmatic access to SAP HANA databases.

## Features

- **Interactive Parameter Collection** — Elicits missing schema names from users during tool execution
- **4 Core Tools** — `ping`, `list_tables`, `describe_table`, `execute_sql`
- **Security First** — Read-only mode blocks DML/DDL, configurable row limits prevent data exfiltration
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
> Requires Rust 1.88 or later. See [MSRV policy](../../README.md#msrv-policy).

## Quick Start

### Standalone Server

```bash
hdbconnect-mcp \
  --url "hdbsql://user:password@host:39017" \
  --read-only true \
  --row-limit 10000 \
  --pool-size 4
```

### Claude Desktop Integration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "hana": {
      "command": "hdbconnect-mcp",
      "args": [
        "--url", "hdbsql://user:password@host:39017",
        "--read-only", "true",
        "--row-limit", "10000"
      ]
    }
  }
}
```

> [!TIP]
> Use environment variables for credentials: `--url "hdbsql://${HANA_USER}:${HANA_PASSWORD}@${HANA_HOST}:39017"`

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--url` | required | HANA connection URL (`hdbsql://user:password@host:port`) |
| `--read-only` | `true` | Block DML/DDL operations (SELECT/WITH/EXPLAIN only) |
| `--row-limit` | `10000` | Maximum rows per query result |
| `--pool-size` | `4` | Database connection pool size |
| `--verbose` | `false` | Enable debug logging (RUST_LOG=debug) |

> [!WARNING]
> Setting `--read-only false` allows INSERT/UPDATE/DELETE operations. Only use in trusted environments.

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

## Features

### Default

Standard stdio transport for MCP integration.

### `http`

Enable HTTP transport for remote MCP access.

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["http"] }
```

> [!NOTE]
> HTTP transport requires additional authentication configuration. See [HTTP Transport Guide](docs/http-transport.md) for setup.

### `cache`

Enable in-memory caching for schema metadata and query results.

```toml
[dependencies]
hdbconnect-mcp = { version = "0.3", features = ["cache"] }
```

## Cache Deployment Notes

The cache feature is designed for **single-user MCP deployments** where all queries run under the same database user or service account.

> [!WARNING]
> **Multi-User Limitation**: Cache keys do not include user context. In multi-tenant deployments with per-user database permissions (row-level security), disable the cache feature or implement user-scoped cache keys to prevent authorization bypass.

For single-user scenarios (typical MCP usage with personal AI assistant or service account), cache is safe and recommended for performance:
- Schema metadata cached for 1 hour (configurable)
- Query results cached for 60 seconds
- Cache hit latency: 3-6 microseconds vs database round-trip

**Schema staleness**: DDL changes (ALTER TABLE, DROP COLUMN) may not be reflected until TTL expires. For environments with frequent schema changes, reduce TTL or disable caching.

## Architecture

```
┌─────────────────┐
│  MCP Client     │ (Claude Desktop, Cline, etc.)
└────────┬────────┘
         │ stdio/HTTP
┌────────▼────────┐
│ hdbconnect-mcp  │
│  ├─ server.rs   │ (Tool handlers with elicitation)
│  ├─ pool.rs     │ (Connection pooling)
│  ├─ config.rs   │ (CLI configuration)
│  ├─ types.rs    │ (JSON Schema types)
│  └─ validation  │ (SQL safety checks)
└────────┬────────┘
         │ hdbconnect_async
┌────────▼────────┐
│   SAP HANA DB   │
└─────────────────┘
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
