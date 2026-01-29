# pyhdb-rs Jupyter Notebooks

Interactive examples demonstrating pyhdb-rs features and best practices.

## Notebooks Overview

| Notebook | Description | Level |
|----------|-------------|-------|
| [01_quickstart.ipynb](01_quickstart.ipynb) | Basic usage, DB-API 2.0, Arrow integration | Beginner |
| [02_polars_analytics.ipynb](02_polars_analytics.ipynb) | Advanced Polars analytics with LazyFrames | Intermediate |
| [03_streaming_large_data.ipynb](03_streaming_large_data.ipynb) | Memory-efficient large dataset processing | Intermediate |
| [04_performance_comparison.ipynb](04_performance_comparison.ipynb) | Benchmarks vs hdbcli | Advanced |
| [05_builder_api.ipynb](05_builder_api.ipynb) | ConnectionBuilder pattern (NEW in v0.3.0) | Beginner |
| [06_tls_configuration.ipynb](06_tls_configuration.ipynb) | Complete TLS/SSL setup guide (NEW) | Intermediate |
| [07_connection_pooling.ipynb](07_connection_pooling.ipynb) | Async connection pooling (NEW) | Intermediate |
| [08_advanced_features.ipynb](08_advanced_features.ipynb) | Cursor holdability, network groups (NEW) | Advanced |

## Getting Started

### Prerequisites

```bash
# Install pyhdb-rs with all features
uv pip install pyhdb_rs[all]

# Install Jupyter
uv pip install jupyter
```

### Running Notebooks

```bash
cd examples/notebooks
jupyter notebook
```

Or use VS Code with the Jupyter extension.

### Configuration

Before running notebooks, set your HANA connection details:

```bash
export HANA_TEST_URI="hdbsql://user:password@host:39017"
```

Or edit the connection strings directly in each notebook.

## What's New in v0.3.0

### Builder API (05_builder_api.ipynb)

Type-safe connection configuration:

```python
from pyhdb_rs import ConnectionBuilder, TlsConfig

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("SYSTEM", "password")
    .tls(TlsConfig.with_system_roots())
    .build())
```

### TLS Configuration (06_tls_configuration.ipynb)

Five methods for different deployment scenarios:

- `TlsConfig.from_directory(path)` - Production (recommended)
- `TlsConfig.from_environment(var)` - Containers/Kubernetes
- `TlsConfig.from_certificate(pem)` - Secrets management
- `TlsConfig.with_system_roots()` - Public CAs
- `TlsConfig.insecure()` - Development only

### Connection Pooling (07_connection_pooling.ipynb)

Production-ready async pools:

```python
from pyhdb_rs.aio import ConnectionPoolBuilder

pool = (ConnectionPoolBuilder()
    .url("hdbsql://user:pass@host:30015")
    .max_size(20)
    .tls(TlsConfig.with_system_roots())
    .build())
```

### Advanced Features (08_advanced_features.ipynb)

- **Cursor Holdability** - Keep cursors open across transactions
- **Network Groups** - HA and Scale-Out routing
- **Connection Validation** - Health checks

## Learning Path

### Beginners

1. Start with [01_quickstart.ipynb](01_quickstart.ipynb)
2. Learn the builder pattern: [05_builder_api.ipynb](05_builder_api.ipynb)
3. Explore Polars integration: [02_polars_analytics.ipynb](02_polars_analytics.ipynb)

### Intermediate Users

1. TLS setup: [06_tls_configuration.ipynb](06_tls_configuration.ipynb)
2. Large datasets: [03_streaming_large_data.ipynb](03_streaming_large_data.ipynb)
3. Async pools: [07_connection_pooling.ipynb](07_connection_pooling.ipynb)

### Advanced Users

1. Advanced features: [08_advanced_features.ipynb](08_advanced_features.ipynb)
2. Performance tuning: [04_performance_comparison.ipynb](04_performance_comparison.ipynb)
3. Production deployments (see all TLS and HA notebooks)

## Common Patterns

### Basic Query

```python
import pyhdb_rs
import polars as pl

conn = pyhdb_rs.connect("hdbsql://user:pass@host:30015")
reader = conn.execute_arrow("SELECT * FROM customers")
df = pl.from_arrow(reader)
```

### With Builder API

```python
from pyhdb_rs import ConnectionBuilder, TlsConfig

conn = (ConnectionBuilder()
    .host("hana.example.com")
    .credentials("user", "password")
    .tls(TlsConfig.with_system_roots())
    .build())
```

### Async Connection Pool

```python
from pyhdb_rs.aio import ConnectionPoolBuilder

pool = (ConnectionPoolBuilder()
    .url("hdbsql://user:pass@host:30015")
    .max_size(10)
    .build())

async with pool.acquire() as conn:
    reader = await conn.execute_arrow("SELECT * FROM orders")
    df = pl.from_arrow(reader)
```

## Tips

1. **Use Arrow for Performance**
   - `execute_arrow()` provides zero-copy data transfer
   - 2x+ faster than traditional row-by-row fetching

2. **Connection Pooling for Concurrency**
   - Use `ConnectionPoolBuilder` for async applications
   - Set `max_size` based on expected concurrency

3. **TLS in Production**
   - Always use TLS for production deployments
   - `TlsConfig.from_directory()` for custom CAs
   - `TlsConfig.with_system_roots()` for public CAs

4. **Cursor Holdability for Large Results**
   - Use `CursorHoldability.CommitAndRollback` for large datasets
   - Allows intermediate commits while reading

5. **Network Groups for HA**
   - Configure network groups for HANA HA/Scale-Out
   - Ensures proper connection routing

## Troubleshooting

### Connection Errors

Check your connection string format:
```
hdbsql://[USER[:PASSWORD]@]HOST[:PORT][/DATABASE]
```

### TLS Certificate Issues

Try different TLS methods:
```python
# For self-signed certs (dev only)
tls = TlsConfig.insecure()

# For public CAs
tls = TlsConfig.with_system_roots()
```

### Memory Issues with Large Datasets

Use streaming with sync API:
```python
reader = conn.execute_arrow("SELECT * FROM huge_table", batch_size=10000)
for batch in reader:
    process_batch(batch)
```

## Contributing

Found an issue or have a suggestion?
- [Report bugs](https://github.com/bug-ops/pyhdb-rs/issues)
- [Submit PRs](https://github.com/bug-ops/pyhdb-rs/pulls)

## Further Reading

- [Project README](../../README.md)
- [Python Examples](../python/)
- [Migration Guide](../python/migration_guide.py)
- [Changelog](../../CHANGELOG.md)
