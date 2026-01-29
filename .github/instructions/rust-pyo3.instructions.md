---
applyTo: "crates/hdbconnect-py/**/*"
---

# PyO3 Bindings Development Guidelines

## Safety and Memory

- **No `unsafe` blocks without justification** - if `unsafe` is used, require detailed safety comments explaining invariants
- **Avoid `.unwrap()` and `.expect()` in library code** - use proper error handling with `?` operator
- **Check for panic paths** - library code should not panic; use `Result` types
- **GIL safety** - verify proper `Python<'_>` lifetime handling
- **No memory leaks in FFI boundaries** - ensure proper cleanup of Python objects

## PyO3 Patterns

- Use `#[pyo3(signature = ...)]` for functions with optional/keyword arguments
- Implement `__repr__` and `__str__` for all Python-visible types
- Use `Bound<'py, T>` over deprecated `PyCell`/`PyRef` patterns
- Use `PyResult<T>` for all fallible operations

## Exception Mapping to DB-API 2.0

Map Rust errors to appropriate DB-API 2.0 exception types:

| Exception | Use Case |
|-----------|----------|
| `InterfaceError` | Connection parameters, driver issues |
| `OperationalError` | Connection lost, timeout |
| `ProgrammingError` | SQL syntax errors |
| `IntegrityError` | Constraint violations |
| `DataError` | Value conversion issues |
| `NotSupportedError` | Unsupported features |

## Performance Concerns

- **Minimize GIL acquisition** - release GIL during I/O operations using `py.allow_threads()`
- **Use `parking_lot::Mutex`** over `std::sync::Mutex` for better performance
- **Batch operations** - prefer bulk data transfer over row-by-row
- **Check for unnecessary clones** - prefer references where possible
- Use `PyBytes`/`PyString` directly instead of converting through `Vec<u8>`/`String`
- Minimize Python/Rust boundary crossings

## API Stability

- Breaking changes in public API require justification
- Maintain backward compatibility with existing Python code
- Version bump required for API changes
