# hdbconnect-mcp

MCP (Model Context Protocol) server for SAP HANA database.

## Overview

Provides AI assistants with secure, read-only access to SAP HANA databases through the Model Context Protocol. Built with Rust for performance and safety.

## Features

- **4 Core Tools**:
  - `ping` - Check database connection health
  - `list_tables` - List tables in schema
  - `describe_table` - Get table column definitions
  - `execute_sql` - Execute SELECT queries (read-only by default)

- **Security**:
  - Read-only mode (blocks DML/DDL operations)
  - Configurable row limits
  - Query timeouts
  - Connection pooling with deadpool

- **Transport**:
  - stdio transport for Claude Desktop integration
  - HTTP transport support (feature: `http`)

## Installation

```bash
cargo install --path .
```

## Usage

```bash
hdbconnect-mcp \
  --url "hdbsql://user:password@host:39017" \
  --read-only true \
  --row-limit 10000 \
  --pool-size 4
```

### Claude Desktop Configuration

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

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--url` | required | HANA connection URL (hdbsql://user:password@host:port) |
| `--read-only` | `true` | Block DML/DDL operations |
| `--row-limit` | `10000` | Maximum rows per query |
| `--pool-size` | `4` | Connection pool size |
| `--verbose` | `false` | Enable debug logging |

## License

Licensed under Apache-2.0 OR MIT.
