# hdbconnect-arrow

[![Crates.io](https://img.shields.io/crates/v/hdbconnect-arrow)](https://crates.io/crates/hdbconnect-arrow)
[![docs.rs](https://img.shields.io/docsrs/hdbconnect-arrow)](https://docs.rs/hdbconnect-arrow)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI&flag=hdbconnect-arrow)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://github.com/bug-ops/pyhdb-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

Apache Arrow integration for the [hdbconnect](https://crates.io/crates/hdbconnect) SAP HANA driver. Converts HANA result sets to Arrow `RecordBatch` format, enabling zero-copy interoperability with the entire Arrow ecosystem.

## Why Arrow?

[Apache Arrow](https://arrow.apache.org/) is the universal columnar data format for analytics. By converting SAP HANA data to Arrow, you unlock seamless integration with:

| Category | Tools |
|----------|-------|
| **DataFrames** | Polars, pandas, Vaex, Dask |
| **Query engines** | DataFusion, DuckDB, ClickHouse, Ballista |
| **ML/AI** | Ray, Hugging Face Datasets, PyTorch, TensorFlow |
| **Data lakes** | Delta Lake, Apache Iceberg, Lance |
| **Visualization** | Perspective, Graphistry, Falcon |
| **Languages** | Rust, Python, R, Julia, Go, Java, C++ |

> [!TIP]
> Arrow's columnar format enables vectorized processing — operations run 10-100x faster than row-by-row iteration.

## Installation

```toml
[dependencies]
hdbconnect-arrow = "0.3"
```

Or with cargo-add:

```bash
cargo add hdbconnect-arrow
```

> [!IMPORTANT]
> Requires Rust 1.88 or later.

## Usage

### Basic batch processing

```rust,ignore
use hdbconnect_arrow::{HanaBatchProcessor, BatchConfig, Result};
use arrow_schema::{Schema, Field, DataType};
use std::sync::Arc;

fn process_results(result_set: hdbconnect::ResultSet) -> Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
    ]));

    let config = BatchConfig::default();
    let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);

    for row in result_set {
        if let Some(batch) = processor.process_row(&row?)? {
            println!("Batch with {} rows", batch.num_rows());
        }
    }

    // Flush remaining rows
    if let Some(batch) = processor.flush()? {
        println!("Final batch with {} rows", batch.num_rows());
    }

    Ok(())
}
```

### Schema mapping

```rust,ignore
use hdbconnect_arrow::{hana_type_to_arrow, hana_field_to_arrow};
use hdbconnect::TypeId;

// Convert individual types
let arrow_type = hana_type_to_arrow(TypeId::DECIMAL, Some(18), Some(2));
// Returns: DataType::Decimal128(18, 2)

// Convert entire field metadata
let arrow_field = hana_field_to_arrow(&hana_field_metadata);
```

### Custom batch size

```rust,ignore
use hdbconnect_arrow::BatchConfig;
use std::num::NonZeroUsize;

let config = BatchConfig::new(NonZeroUsize::new(10_000).unwrap());
```

## Performance

The crate is optimized for high-throughput data transfer with several performance enhancements in v0.3.2:

### Optimization Techniques

- **Enum-based dispatch** — Eliminates vtable overhead by replacing `Box<dyn HanaCompatibleBuilder>` with `BuilderEnum`, resulting in ~10-20% performance improvement through better cache locality and monomorphized dispatch
- **Homogeneous loop hoisting** — Detects schemas where all columns share the same type and hoists the dispatch match outside the row loop for +4-8% throughput on wide tables (100+ columns)
- **Zero-copy decimal conversion** — Uses `Cow::Borrowed` to avoid cloning BigInt during decimal conversion, improving decimal throughput by +222% (55 → 177 Melem/s) and saving 8MB per 1M decimals
- **String capacity pre-sizing** — Extracts max_length from HANA field metadata to pre-allocate optimal buffer sizes, reducing reallocation overhead by 2-3x per string column
- **Batch processing** — Configurable batch sizes to balance memory usage and throughput
- **Builder reuse** — Builders reset between batches, eliminating repeated allocations

> [!TIP]
> For large result sets, use `LendingBatchIterator` to stream data with constant memory usage.

### Profiling Support

Enable the optional `profiling` feature flag to integrate [dhat](https://docs.rs/dhat) heap profiler:

```toml
[dependencies]
hdbconnect-arrow = { version = "0.3", features = ["profiling"] }
```

This enables allocation tracking with zero impact on release builds through conditional compilation. See `src/profiling.rs` for usage examples.

## Ecosystem integration

<details>
<summary><strong>DataFusion</strong> — SQL queries on Arrow data</summary>

Query HANA data with SQL using [Apache DataFusion](https://datafusion.apache.org/):

```rust,ignore
use datafusion::prelude::*;

let batches = collect_batches_from_hana(result_set)?;
let ctx = SessionContext::new();
ctx.register_batch("hana_data", batches[0].clone())?;

let df = ctx.sql("SELECT * FROM hana_data WHERE amount > 1000").await?;
df.show().await?;
```

</details>

<details>
<summary><strong>DuckDB</strong> — analytical queries</summary>

Load Arrow data directly into [DuckDB](https://duckdb.org/):

```rust,ignore
use duckdb::{Connection, arrow::record_batch_to_duckdb};

let conn = Connection::open_in_memory()?;
conn.register_arrow("sales", batches)?;

let mut stmt = conn.prepare("SELECT region, SUM(amount) FROM sales GROUP BY region")?;
let result = stmt.query_arrow([])?;
```

</details>

<details>
<summary><strong>Polars</strong> — zero-copy DataFrame</summary>

Convert to [Polars](https://pola.rs/) DataFrame:

```rust,ignore
use polars::prelude::*;

let batch = processor.flush()?.unwrap();
let df = DataFrame::try_from(batch)?;

let result = df.lazy()
    .filter(col("status").eq(lit("active")))
    .group_by([col("region")])
    .agg([col("amount").sum()])
    .collect()?;
```

</details>

<details>
<summary><strong>Arrow IPC / Parquet</strong> — serialization</summary>

Serialize Arrow data for storage or network transfer:

```rust,ignore
use arrow_ipc::writer::FileWriter;
use parquet::arrow::ArrowWriter;
use std::fs::File;

// Arrow IPC (Feather) format
let file = File::create("data.arrow")?;
let mut writer = FileWriter::try_new(file, &schema)?;
writer.write(&batch)?;
writer.finish()?;

// Parquet format
let file = File::create("data.parquet")?;
let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
writer.write(&batch)?;
writer.close()?;
```

</details>

<details>
<summary><strong>Python interop</strong> — PyCapsule zero-copy</summary>

Export Arrow data to Python without copying (requires `pyo3`):

```rust,ignore
use pyo3_arrow::PyArrowType;
use pyo3::prelude::*;

#[pyfunction]
fn get_hana_data(py: Python<'_>) -> PyResult<PyArrowType<RecordBatch>> {
    let batch = fetch_from_hana()?;
    Ok(PyArrowType(batch))
}

// Python: df = pl.from_arrow(get_hana_data())
```

</details>

## Features

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
hdbconnect-arrow = { version = "0.3", features = ["async", "test-utils", "profiling"] }
```

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Async support via `hdbconnect_async` | No |
| `test-utils` | Expose `MockRow`/`MockRowBuilder` for testing | No |
| `profiling` | dhat heap profiler integration for performance analysis | No |

> [!TIP]
> Enable `test-utils` in dev-dependencies for unit testing without a HANA connection.

## Type mapping

<details>
<summary>HANA → Arrow type conversion table</summary>

| HANA Type | Arrow Type | Notes |
|-----------|------------|-------|
| TINYINT | UInt8 | Unsigned in HANA |
| SMALLINT | Int16 | |
| INT | Int32 | |
| BIGINT | Int64 | |
| REAL | Float32 | |
| DOUBLE | Float64 | |
| DECIMAL(p,s) | Decimal128(p,s) | Full precision preserved |
| CHAR, VARCHAR | Utf8 | |
| NCHAR, NVARCHAR | Utf8 | Unicode strings |
| CLOB, NCLOB | LargeUtf8 | Large text |
| BLOB | LargeBinary | Large binary |
| DATE | Date32 | Days since epoch |
| TIME | Time64(Nanosecond) | |
| TIMESTAMP | Timestamp(Nanosecond) | |
| BOOLEAN | Boolean | |
| GEOMETRY, POINT | Binary | WKB format |

</details>

## API overview

<details>
<summary><strong>Core types</strong></summary>

- **`HanaBatchProcessor`** — Converts HANA rows to Arrow `RecordBatch` with configurable batch sizes
- **`BatchConfig`** — Configuration for batch processing (uses `NonZeroUsize` for type-safe batch size)
- **`SchemaMapper`** — Maps HANA result set metadata to Arrow schemas
- **`BuilderFactory`** — Creates appropriate Arrow array builders for HANA types
- **`TypeCategory`** — Centralized HANA type classification enum

</details>

<details>
<summary><strong>Performance types (v0.3.2+)</strong></summary>

- **`BuilderEnum`** — Enum-wrapped builder for static dispatch (eliminates vtable overhead)
- **`BuilderKind`** — Discriminant identifying builder type for schema profiling
- **`SchemaProfile`** — Classifies schemas as homogeneous or mixed for optimized processing paths

</details>

<details>
<summary><strong>Traits</strong></summary>

- **`HanaCompatibleBuilder`** — Trait for Arrow builders that accept HANA values
- **`FromHanaValue`** — Sealed trait for type-safe value conversion
- **`BatchProcessor`** — Core batch processing interface
- **`LendingBatchIterator`** — GAT-based streaming iterator for large result sets
- **`RowLike`** — Row abstraction for testing without HANA connection

</details>

<details>
<summary><strong>Test utilities</strong></summary>

When `test-utils` feature is enabled:

```rust,ignore
use hdbconnect_arrow::{MockRow, MockRowBuilder};

let row = MockRowBuilder::new()
    .push_i64(42)
    .push_string("test")
    .push_null()
    .build();
```

</details>

<details>
<summary><strong>Error handling</strong></summary>

```rust,ignore
use hdbconnect_arrow::{ArrowConversionError, Result};

fn convert_data() -> Result<()> {
    // ArrowConversionError covers:
    // - Type mismatches
    // - Decimal overflow
    // - Schema incompatibilities
    // - Invalid batch configuration
    Ok(())
}
```

</details>

## Part of pyhdb-rs

This crate is part of the [pyhdb-rs](https://github.com/bug-ops/pyhdb-rs) workspace, providing the Arrow integration layer for the Python SAP HANA driver.

Related crates:
- [`hdbconnect-py`](https://github.com/bug-ops/pyhdb-rs/tree/main/crates/hdbconnect-py) — PyO3 bindings exposing Arrow data to Python

## Resources

- [Apache Arrow](https://arrow.apache.org/) — Official Arrow project
- [Arrow Rust](https://docs.rs/arrow) — Rust implementation
- [DataFusion](https://datafusion.apache.org/) — Query engine built on Arrow
- [Powered by Arrow](https://arrow.apache.org/powered_by/) — Projects using Arrow

## MSRV policy

> [!NOTE]
> Minimum Supported Rust Version: **1.88**. MSRV increases are minor version bumps.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
