---
applyTo: "**/*"
---

# General Development Guidelines

## Code Quality Standards

- All documentation and comments must be in English
- No excessive comments - comments only for cyclomatic/cognitive complex code blocks
- Follow [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/)
- Commit messages: concise, expert-level, no emojis

## Error Handling Philosophy

- All errors must be properly propagated with context
- Use `thiserror` for error types in Rust
- Never swallow errors silently
- Provide meaningful error messages with relevant context (column names, type info, etc.)
- Classify errors correctly: configuration vs data vs recoverable errors

## Testing Requirements

- All public APIs must have test coverage
- Use `cargo nextest` for Rust tests
- Use `pytest` for Python tests
- Target 70% project coverage, 80% patch coverage

## Security Principles

- No `unsafe` blocks without detailed safety comments explaining invariants
- Validate all external inputs (URLs, SQL, file paths)
- No credential logging - passwords/tokens must never appear in logs
- Use secure defaults for TLS connections
- No hardcoded credentials - use environment variables or config files

## Breaking Change Guidelines

Changes to the following require explicit approval:

1. Public Rust API (`pub` items in `lib.rs`)
2. Python public API (items in `__all__`)
3. Error types and codes (exception hierarchy)
4. Type mappings (HANA to Arrow conversions)
5. Default configuration values
6. Minimum supported versions (Rust, Python, dependencies)

### Deprecation Process

1. Add deprecation warning in current version
2. Document migration path
3. Remove in next major version

## Review Checklist

For every PR, verify:

- CI passes (lint, test, coverage, security)
- No new `unsafe` without justification
- No new `.unwrap()` in library code
- Type hints complete for Python code
- Tests added for new functionality
- Documentation updated if API changes
- No breaking changes without version bump
- License-compliant dependencies only (MIT, Apache-2.0, BSD)
