---
applyTo: "python/**/*.py,tests/python/**/*.py"
---

# Python Development Guidelines

## Type Hints (PEP 484)

- All public functions must have type annotations
- Use `from __future__ import annotations` for forward references
- Verify `.pyi` stub files match actual implementation
- Use `typing` module for complex types

## DB-API 2.0 Compliance (PEP 249)

- Verify cursor, connection, and type constructor interfaces
- Ensure type objects work correctly: `STRING`, `BINARY`, `NUMBER`, `DATETIME`, `ROWID`
- Maintain `threadsafety = 2` guarantees (connections shareable, cursors not)
- Implement required module globals: `apilevel`, `threadsafety`, `paramstyle`

## Code Style

- Follow ruff linting rules (see `python/ruff.toml`)
- Docstrings for public APIs using Google or NumPy style
- No star imports - explicit imports only
- Use `from __future__ import annotations` at top of files

## Testing with pytest

- Use fixtures for database connections
- Mark integration tests with `@pytest.mark.integration`
- Use `@pytest.mark.parametrize` for data-driven tests
- Ensure proper cleanup in teardown

## Security

- No `eval()` or `exec()` on untrusted input
- SQL injection prevention - use parameterized queries only
- No hardcoded credentials - use environment variables

## Linting Commands

```bash
ruff check --config python/ruff.toml python/ tests/python/
ruff format --config python/ruff.toml --check python/ tests/python/
```
