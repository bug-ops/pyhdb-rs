# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial release of pyhdb-rs
- Full DB-API 2.0 (PEP 249) compliance
- Zero-copy Arrow data transfer via PyCapsule Interface
- Native Polars integration (`pyhdb_rs.polars`)
- Native pandas integration (`pyhdb_rs.pandas`)
- Async/await support (`pyhdb_rs.aio`)
- Connection pooling for async operations
- Support for Python 3.11, 3.12, 3.13
- Cross-platform support (Linux, macOS, Windows)

### Changed

- N/A

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- N/A

### Security

- N/A

## [0.1.0] - TBD

Initial release.

[Unreleased]: https://github.com/bug-ops/pyhdb-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bug-ops/pyhdb-rs/releases/tag/v0.1.0
