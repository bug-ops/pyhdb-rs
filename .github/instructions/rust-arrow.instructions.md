---
applyTo: "crates/hdbconnect-arrow/**/*"
---

# Arrow Integration Development Guidelines

## HANA to Arrow Type Mappings

Ensure HANA types map correctly to Arrow types:

| HANA Type | Arrow Type |
|-----------|------------|
| `INT` | `Int32` |
| `BIGINT` | `Int64` |
| `DECIMAL` | `Decimal128(precision, scale)` |
| `VARCHAR` / `NVARCHAR` | `Utf8` |
| `CLOB` / `NCLOB` | `LargeUtf8` |
| `BLOB` | `LargeBinary` |
| `DATE` | `Date32` |
| `TIMESTAMP` | `Timestamp(Nanosecond)` |
| `GEOMETRY` / `POINT` | `Binary` (WKB format) |

## Zero-Copy Guarantees

- **No unnecessary allocations** in Arrow buffer construction
- **Use Arrow builders correctly** - verify proper capacity preallocation with `with_capacity()`
- **Check for data copies** - ensure zero-copy where promised
- Prefer `RecordBatch` over single-row operations

## Canonical Error Pattern

Use the canonical error struct pattern:

```rust
pub struct ArrowConversionError {
    kind: ErrorKind,
    column: Option<String>,
    source_type: Option<String>,
    target_type: Option<String>,
    context: Option<String>,
}

impl ArrowConversionError {
    pub fn is_type_mismatch(&self) -> bool { ... }
    pub fn is_overflow(&self) -> bool { ... }
    pub fn is_null_violation(&self) -> bool { ... }
}
```

## Error Handling

- Provide error context - include column names, type info in error messages
- Classify errors correctly - configuration vs data vs recoverable errors
- Use `thiserror` for error type definitions

## Performance

- Preallocate capacity using `with_capacity()` for builders
- Batch processing - prefer `RecordBatch` over single-row operations
- Minimize schema operations - cache schema derivation
- Avoid allocations in hot paths - use iterators, slices
