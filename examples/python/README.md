# Python Examples for pyhdb-rs v0.3.0

This directory contains comprehensive examples demonstrating the new features in pyhdb-rs v0.3.0.

## Examples Overview

| File | Description |
|------|-------------|
| `builder_basic.py` | Basic ConnectionBuilder usage and patterns |
| `builder_tls.py` | All five TlsConfig factory methods with examples |
| `builder_async.py` | AsyncConnectionBuilder for async/await connections |
| `pool_builder.py` | Connection pooling with ConnectionPoolBuilder |
| `cursor_holdability.py` | Cursor behavior control across transactions |
| `ha_network_group.py` | Network group configuration for HA/Scale-Out |
| `migration_guide.py` | Migration guide from v0.2.x to v0.3.0 |

## Running the Examples

### Prerequisites

```bash
# Install pyhdb-rs with all features
uv pip install pyhdb_rs[all]

# Or with specific features
uv pip install pyhdb_rs[polars,async]
```

### Configuration

Before running examples, configure your HANA connection:

1. Edit the connection parameters in each example file
2. Replace placeholders:
   - `hana.example.com` → your HANA host
   - `SYSTEM` / `password` → your credentials
   - `30015` → your HANA port

### Run Individual Examples

```bash
# Basic builder usage
python examples/python/builder_basic.py

# TLS configuration
python examples/python/builder_tls.py

# Async connections
python examples/python/builder_async.py

# Connection pooling
python examples/python/pool_builder.py

# Cursor holdability
python examples/python/cursor_holdability.py

# HA/Network groups
python examples/python/ha_network_group.py

# Migration guide
python examples/python/migration_guide.py
```

## Key Features Demonstrated

### 1. ConnectionBuilder

The builder pattern provides type-safe, discoverable connection configuration:

```python
from pyhdb_rs import ConnectionBuilder, TlsConfig

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .port(30015)
    .credentials("SYSTEM", "password")
    .tls(TlsConfig.with_system_roots())
    .autocommit(False)
    .build())
```

### 2. TlsConfig (5 Methods)

Flexible TLS configuration for different deployment scenarios:

- `TlsConfig.from_directory(path)` - Load certs from directory
- `TlsConfig.from_environment(var)` - From environment variable
- `TlsConfig.from_certificate(pem)` - Direct PEM string
- `TlsConfig.with_system_roots()` - Mozilla root certificates
- `TlsConfig.insecure()` - Skip verification (dev only)

### 3. CursorHoldability

Control cursor behavior across transaction boundaries:

- `CursorHoldability.None_` - Closed on commit and rollback
- `CursorHoldability.Commit` - Held across commits
- `CursorHoldability.Rollback` - Held across rollbacks
- `CursorHoldability.CommitAndRollback` - Held across both

### 4. Network Groups

Essential for HANA HA and Scale-Out deployments:

```python
conn = (ConnectionBuilder()
    .host("hana-ha.example.com")
    .credentials("SYSTEM", "password")
    .network_group("production")
    .build())
```

### 5. Connection Pool Builder

Production-ready async connection pooling:

```python
from pyhdb_rs.aio import ConnectionPoolBuilder

pool = (ConnectionPoolBuilder()
    .url("hdbsql://user:pass@host:30015")
    .max_size(20)
    .tls(TlsConfig.with_system_roots())
    .network_group("production")
    .build())
```

## Best Practices

1. **Use TLS in Production**
   ```python
   tls = TlsConfig.from_directory("/etc/hana/certs")
   # or
   tls = TlsConfig.with_system_roots()
   ```

2. **Configure Network Groups for HA**
   ```python
   .network_group("production")
   ```

3. **Choose Appropriate Cursor Holdability**
   ```python
   # For large result sets with intermediate commits
   .cursor_holdability(CursorHoldability.CommitAndRollback)
   ```

4. **Size Connection Pools Appropriately**
   ```python
   .max_size(20)  # Based on expected concurrency
   ```

5. **Use Context Managers**
   ```python
   async with pool.acquire() as conn:
       # Connection automatically released
       pass
   ```

## Troubleshooting

### TLS Certificate Issues

```python
# For development/testing only
tls = TlsConfig.insecure()

# For production, use proper certificates
tls = TlsConfig.from_directory("/etc/hana/certs")
```

### Connection Pool Exhaustion

```python
# Increase pool size or reduce connection timeout
pool = (ConnectionPoolBuilder()
    .max_size(30)  # Increase from default
    .build())
```

### Cursor Closed After Commit

```python
# Use appropriate holdability
conn = (ConnectionBuilder()
    .cursor_holdability(CursorHoldability.CommitAndRollback)
    .build())
```

## Further Reading

- [Project README](../../README.md)
- [Jupyter Notebooks](../notebooks/)
- [Migration Guide](migration_guide.py)
- [API Documentation](https://github.com/bug-ops/pyhdb-rs)

## Support

For issues or questions:
- [GitHub Issues](https://github.com/bug-ops/pyhdb-rs/issues)
- [Changelog](../../CHANGELOG.md)
