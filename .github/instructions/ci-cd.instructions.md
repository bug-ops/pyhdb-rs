---
applyTo: ".github/workflows/**/*"
---

# CI/CD Workflow Guidelines

## Workflow Quality

- **All jobs must have timeouts** - prevent hanging workflows
- **Pin action versions** - use specific versions (e.g., `@v4`), not `@latest`
- **Fail-fast strategy** - appropriate for lint jobs, disabled for test matrix
- **Use concurrency control** - cancel in-progress runs for same PR

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

## Caching Strategies

Use two-tier caching for Rust builds:

1. **Swatinem/rust-cache** - Cargo registry + target directory
2. **sccache** - Compiled artifacts

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "build"
    save-if: ${{ github.ref == 'refs/heads/main' }}
- uses: mozilla-actions/sccache-action@v0.0.4
```

## Security

- **No secrets in logs** - verify secrets are masked
- **Minimal permissions** - use least-privilege `permissions` block
- **Dependency review** - verify new dependencies are audited
- **cargo-deny checks** - advisories, licenses, bans, sources

```yaml
permissions:
  contents: read
  pull-requests: read
```

## Cross-Platform Testing

- Test on all platforms: `ubuntu-latest`, `macos-latest`, `windows-latest`
- Python version matrix: 3.12, 3.13, 3.14
- MSRV check - verify minimum supported Rust version

```yaml
strategy:
  fail-fast: false
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
```

## Standard Job Timeouts

| Job | Timeout |
|-----|---------|
| check (fmt + clippy) | 10 min |
| test (cargo nextest) | 30 min |
| coverage (cargo llvm-cov) | 20 min |
| security (cargo deny) | 10 min |
| msrv | 15 min |

## Tooling

- Use `dtolnay/rust-toolchain` for Rust setup
- Use `taiki-e/install-action` for tool installation (nextest, cargo-llvm-cov, cargo-deny)
- Use `codecov/codecov-action` for coverage reporting
