# Contributing to pyhdb-rs

Thank you for your interest in contributing to pyhdb-rs! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Commit Messages](#commit-messages)

## Code of Conduct

This project adheres to the Contributor Covenant code of conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the maintainers.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Set up the development environment
4. Create a new branch for your changes
5. Make your changes
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.85+ (see `rust-version` in Cargo.toml for MSRV)
- Python 3.12+
- [maturin](https://github.com/PyO3/maturin) for building Python wheels
- [cargo-nextest](https://nexte.st/) for running tests
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for security audits

### Installation

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/pyhdb-rs.git
cd pyhdb-rs

# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain for formatting
rustup install nightly

# Install development tools
cargo install cargo-nextest cargo-deny cargo-llvm-cov

# Set up Python virtual environment
cd python
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install Python dependencies
pip install maturin pytest pytest-cov polars pandas pyarrow

# Build the Python extension in development mode
maturin develop
```

### Building

```bash
# Build Rust workspace
cargo build --workspace

# Build with all features (including async)
cargo build --workspace --all-features

# Build Python wheel (development)
cd python && maturin develop

# Build release wheel
maturin build --release
```

## Code Style

### Rust

We follow the [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/) and enforce strict linting.

```bash
# Format code (requires nightly)
cargo +nightly fmt

# Check formatting
cargo +nightly fmt --check

# Run clippy with strict warnings
cargo clippy --workspace --all-features -- -D warnings
```

Key style points:
- Use `rustfmt` for formatting
- All clippy warnings are treated as errors
- Add documentation for public APIs
- Avoid unnecessary `unsafe` code
- Use `thiserror` for error types

### Python

We use [ruff](https://github.com/astral-sh/ruff) for Python linting and formatting.

```bash
# Run ruff
ruff check python/
ruff format python/
```

## Testing

### Running Tests

```bash
# Run all Rust tests
cargo nextest run --workspace

# Run tests with all features
cargo nextest run --workspace --all-features

# Run Python tests
pytest tests/python/ -v

# Run with coverage
cargo llvm-cov --workspace --all-features
```

### Writing Tests

- All new features should include tests
- Bug fixes should include regression tests
- Tests should be placed in:
  - `tests/rust/` for Rust integration tests
  - `tests/python/` for Python tests
  - Inline `#[cfg(test)]` modules for unit tests

### Testing with SAP HANA

Some tests require a HANA instance. Set the connection via environment variable:

```bash
export HANA_TEST_URI="hdbsql://user:pass@host:39017"
```

Tests that require HANA are marked with `#[ignore]` by default.

## Pull Request Process

1. **Create a branch**: Use a descriptive branch name (e.g., `feature/add-batch-insert`, `fix/connection-timeout`)

2. **Make your changes**: Follow the code style guidelines

3. **Run the verification suite**:
   ```bash
   cargo +nightly fmt --check && \
   cargo clippy --workspace --all-features -- -D warnings && \
   cargo check --workspace && \
   cargo nextest run --workspace && \
   cargo deny check
   ```

4. **Update documentation**: If your changes affect the public API, update the relevant documentation

5. **Submit a PR**: Fill out the PR template completely

6. **Address review feedback**: Make any requested changes

7. **Merge**: Once approved, a maintainer will merge your PR

### PR Guidelines

- Keep PRs focused on a single change
- Include tests for new functionality
- Update documentation as needed
- Ensure all CI checks pass
- Respond to review comments promptly

## Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring without feature changes
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

### Examples

```
feat(arrow): add support for DECIMAL128 type conversion

fix(cursor): handle empty result sets correctly

docs(readme): add async usage examples

perf(conversion): optimize batch conversion for large result sets

Reduces memory allocation by 40% when converting result sets
with more than 10,000 rows.
```

## Questions?

If you have questions, feel free to:
- Open a [Discussion](https://github.com/bug-ops/pyhdb-rs/discussions)
- Open an issue with the question label

Thank you for contributing!
