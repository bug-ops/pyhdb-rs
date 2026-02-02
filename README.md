# pyhdb-rs

**Unlock your SAP HANA data for modern analytics and AI workflows.**

Pure Rust toolkit — no SAP client installation required. Connect to HANA from anywhere, stream data directly into Polars, pandas, or DuckDB via Apache Arrow. Let AI assistants explore your schemas through a secure MCP interface.

[![CI](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/ci.yml)
[![Security](https://github.com/bug-ops/pyhdb-rs/actions/workflows/security.yml/badge.svg)](https://github.com/bug-ops/pyhdb-rs/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/bug-ops/pyhdb-rs)
[![PyPI](https://img.shields.io/pypi/v/pyhdb_rs.svg)](https://pypi.org/project/pyhdb_rs/)
[![Crates.io](https://img.shields.io/crates/v/hdbconnect-mcp.svg)](https://crates.io/crates/hdbconnect-mcp)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

## Why pyhdb-rs?

| Pain Point | Solution |
|------------|----------|
| SAP client installation & licensing | **Pure Rust** — no SAP dependencies, just `pip install` |
| Memory explodes with big datasets | **Zero-copy Arrow** streams data directly to DataFrames |
| Manual schema discovery for AI tools | **MCP server** gives Claude/Cline native HANA access |
| Complex ETL for analytics | **One-liner** to Polars LazyFrame or pandas |

## Quick Start

### For Data Engineers & Scientists

```bash
pip install pyhdb_rs
```

```python
from pyhdb_rs import ConnectionBuilder
import polars as pl

# Connect and extract — data flows directly to Polars without copies
conn = ConnectionBuilder.from_url("hdbsql://user:pass@hana:30015").build()
df = pl.from_arrow(conn.execute_arrow("""
    SELECT material, plant, SUM(quantity) as total
    FROM sapabap1.mard
    GROUP BY material, plant
"""))

# Lazy evaluation for memory-efficient analytics
result = df.lazy().filter(pl.col("total") > 1000).collect()
```

> [!TIP]
> Data exports via [Apache Arrow](https://arrow.apache.org/) — the universal columnar format. Integrates with the entire Arrow ecosystem out of the box.

### Arrow Ecosystem Compatibility

Stream HANA data directly into any Arrow-compatible tool:

| Category | Tools |
|----------|-------|
| **DataFrames** | [Polars](https://pola.rs/), [pandas](https://pandas.pydata.org/), [Vaex](https://vaex.io/), [Dask](https://www.dask.org/) |
| **Query Engines** | [DuckDB](https://duckdb.org/), [DataFusion](https://datafusion.apache.org/), [ClickHouse](https://clickhouse.com/) |
| **ETL / Streaming** | [Apache Spark](https://spark.apache.org/), [Apache Flink](https://flink.apache.org/), [Kafka + Arrow](https://arrow.apache.org/) |
| **ML / AI** | [Ray](https://www.ray.io/), [Hugging Face Datasets](https://huggingface.co/docs/datasets/), [PyTorch](https://pytorch.org/) |
| **Data Lakes** | [Delta Lake](https://delta.io/), [Apache Iceberg](https://iceberg.apache.org/), [Lance](https://lancedb.github.io/lance/) |
| **Serialization** | [Parquet](https://parquet.apache.org/), [Arrow IPC/Feather](https://arrow.apache.org/docs/python/ipc.html) |

Zero-copy data transfer means no serialization overhead between HANA and your analytics stack.

### For AI-Assisted Development

```bash
cargo install hdbconnect-mcp
```

Add to Claude Desktop config:

```json
{
  "mcpServers": {
    "hana": {
      "command": "hdbconnect-mcp",
      "args": ["--url", "hdbsql://user:pass@hana:30015"]
    }
  }
}
```

Now ask Claude: *"Show me the top 10 customers by revenue from VBAK/VBAP"* — it queries HANA directly.

## Components

| Package | Use Case | Install |
|---------|----------|---------|
| [**pyhdb_rs**](python/README.md) | Python analytics, ETL pipelines, ML feature extraction | `pip install pyhdb_rs` |
| [**hdbconnect-mcp**](crates/hdbconnect-mcp/README.md) | AI assistants, natural language queries, schema exploration | `cargo install hdbconnect-mcp` |
| [**hdbconnect-arrow**](crates/hdbconnect-arrow/README.md) | Rust applications, custom Arrow integrations | `cargo add hdbconnect-arrow` |

## Python Driver

Full DB-API 2.0 compliance with native Arrow integration. No SAP client required — works anywhere Python runs.

<details>
<summary><strong>Async ETL with connection pooling</strong></summary>

```python
from pyhdb_rs.aio import ConnectionPoolBuilder
import polars as pl

pool = ConnectionPoolBuilder().url("hdbsql://user:pass@hana:30015").max_size(10).build()

async def extract_sales(region: str) -> pl.DataFrame:
    async with pool.acquire() as conn:
        return pl.from_arrow(await conn.execute_arrow(f"""
            SELECT vbeln, erdat, netwr FROM sapabap1.vbak
            WHERE region = '{region}' AND erdat >= '20240101'
        """))

# Parallel extraction across regions
results = await asyncio.gather(*[extract_sales(r) for r in ["US", "EU", "APAC"]])
```

</details>

<details>
<summary><strong>pandas integration</strong></summary>

```python
import pyarrow as pa

reader = conn.execute_arrow("SELECT * FROM sapabap1.mara WHERE mtart = 'FERT'")
df = pa.RecordBatchReader.from_stream(reader).read_all().to_pandas()
```

</details>

<details>
<summary><strong>Streaming large datasets</strong></summary>

```python
from pyhdb_rs import ArrowConfig

# Process 100M rows with constant memory
config = ArrowConfig(batch_size=50_000)
reader = conn.execute_arrow("SELECT * FROM sapabap1.mseg", config=config)

for batch in reader:
    process_batch(batch)  # Each batch: 50K rows as Arrow RecordBatch
```

</details>

**Full documentation:** [Python package README](python/README.md) — TLS configuration, HA clusters, transaction control, error handling.

## MCP Server for AI Agents

Production-ready server that exposes SAP HANA to Claude, Cline, Cursor, and any MCP-compatible assistant.

**Why it matters:** Instead of copy-pasting schemas or writing boilerplate queries, let AI discover and query your data directly — with guardrails.

**Security by design:**
- Read-only mode blocks DML/DDL by default
- Row limits prevent data exfiltration
- OIDC/JWT authentication for enterprise deployments
- Per-user cache isolation in multi-tenant setups

**Available tools:**

| Tool | What AI can do |
|------|----------------|
| `list_tables` | Explore schemas: *"What tables exist in SAPABAP1?"* |
| `describe_table` | Understand structure: *"Show me VBAK columns"* |
| `execute_sql` | Query data: *"Get top customers by revenue"* |
| `ping` | Verify connectivity |

**Full documentation:** [MCP server README](crates/hdbconnect-mcp/README.md) — HTTP transport, Kubernetes deployment, Prometheus metrics.

## Architecture

```mermaid
flowchart TB
    subgraph apps["Your Applications"]
        direction LR
        subgraph python["Python Analytics"]
            p1["ETL Pipelines"]
            p2["ML Features"]
            p3["BI Dashboards"]
        end
        subgraph ai["AI Assistants"]
            m1["Claude Desktop"]
            m2["Cline / Cursor"]
            m3["Custom Agents"]
        end
    end

    subgraph core["Rust Core"]
        arrow["hdbconnect-arrow\nZero-copy HANA → Arrow"]
    end

    subgraph hana["SAP HANA"]
        db[("BW/4HANA\nS/4HANA\nHANA Cloud")]
    end

    apps --> core
    core --> hana

    style apps fill:#e3f2fd
    style core fill:#fff8e1
    style hana fill:#e8f5e9
```

## Requirements

- **Python** 3.12+ (for pyhdb_rs)
- **Rust** 1.88+ (for building from source or MCP server)

## Resources

| Resource | Link |
|----------|------|
| Python API | [python/README.md](python/README.md) |
| MCP Server | [crates/hdbconnect-mcp/README.md](crates/hdbconnect-mcp/README.md) |
| Arrow Integration | [crates/hdbconnect-arrow/README.md](crates/hdbconnect-arrow/README.md) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## License

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
