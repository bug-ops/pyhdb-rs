# hdbconnect-arrow

[![Crates.io](https://img.shields.io/crates/v/hdbconnect-arrow)](https://crates.io/crates/hdbconnect-arrow)
[![docs.rs](https://img.shields.io/docsrs/hdbconnect-arrow)](https://docs.rs/hdbconnect-arrow)
[![codecov](https://codecov.io/gh/bug-ops/pyhdb-rs/graph/badge.svg?token=75RR61N6FI&flag=hdbconnect-arrow)](https://codecov.io/gh/bug-ops/pyhdb-rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://github.com/bug-ops/pyhdb-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

Apache Arrow integration for the [hdbconnect](https://crates.io/crates/hdbconnect) SAP HANA driver, enabling zero-copy data transfer to analytics tools like Polars and pandas.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hdbconnect-arrow = "0.1"
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

## Features

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
hdbconnect-arrow = { version = "0.1", features = ["async", "test-utils"] }
```

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Async support via `hdbconnect_async` | No |
| `test-utils` | Expose `MockRow`/`MockRowBuilder` for testing | No |

> [!TIP]
> Enable `test-utils` in dev-dependencies for unit testing without a HANA connection.

## Type mapping

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

## API overview

### Core types

- **`HanaBatchProcessor`** — Converts HANA rows to Arrow `RecordBatch` with configurable batch sizes
- **`BatchConfig`** — Configuration for batch processing (uses `NonZeroUsize` for type-safe batch size)
- **`SchemaMapper`** — Maps HANA result set metadata to Arrow schemas
- **`BuilderFactory`** — Creates appropriate Arrow array builders for HANA types
- **`TypeCategory`** — Centralized HANA type classification enum

### Traits

- **`HanaCompatibleBuilder`** — Trait for Arrow builders that accept HANA values
- **`FromHanaValue`** — Sealed trait for type-safe value conversion
- **`BatchProcessor`** — Core batch processing interface
- **`LendingBatchIterator`** — GAT-based streaming iterator for large result sets
- **`RowLike`** — Row abstraction for testing without HANA connection

### Test utilities

When `test-utils` feature is enabled:

```rust,ignore
use hdbconnect_arrow::{MockRow, MockRowBuilder};

let row = MockRowBuilder::new()
    .push_i64(42)
    .push_string("test")
    .push_null()
    .build();

// Use with HanaBatchProcessor for unit tests
```

### Error handling

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

## Part of pyhdb-rs

This crate is part of the [pyhdb-rs](https://github.com/bug-ops/pyhdb-rs) workspace, providing the Arrow integration layer for the Python SAP HANA driver.

Related crates:
- [`hdbconnect-py`](https://github.com/bug-ops/pyhdb-rs/tree/main/crates/hdbconnect-py) — PyO3 bindings exposing Arrow data to Python

## MSRV policy

> [!NOTE]
> Minimum Supported Rust Version: **1.88**. MSRV increases are minor version bumps.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
