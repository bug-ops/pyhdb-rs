# hdbconnect-arrow

Apache Arrow integration for the hdbconnect SAP HANA driver.

## Features

- Type-safe HANA to Arrow type mapping
- Streaming RecordBatch iteration for large result sets
- Sealed traits for API stability
- Generic Associated Types (GATs) for lending iterators

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hdbconnect-arrow = "0.1"
```

## Usage

```rust,ignore
use hdbconnect_arrow::{Result, StreamingRecordBatchReader, BatchConfig};

// Convert HANA ResultSet to Arrow RecordBatches
let reader = StreamingRecordBatchReader::new(result_set, BatchConfig::default())?;
for batch in reader {
    let batch = batch?;
    // Process Arrow RecordBatch...
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
