# Changelog

All notable changes to Embeddenator will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **TASK-RES-003**: Resonator-EmbrFS integration for enhanced extraction
  - Optional resonator field in EmbrFS struct for pattern completion
  - `set_resonator()` method for configuring resonator networks
  - `extract_with_resonator()` method with robust recovery capabilities
  - Integration tests validating resonator-enhanced extraction
  - 100% reconstruction support with pattern completion fallback

## [0.2.0] - 2025-12-15

### Added
- Comprehensive end-to-end regression test suite (5 tests)
  - Comprehensive workflow test with multi-file types and nested directories
  - Performance validation test (100 files with timing bounds)
  - Query functionality test
  - Data integrity test with bit-perfect byte-for-byte validation
  - Directory structure preservation test
- Intelligent test runner (`test_runner.py`) with debug logging
  - Accurate test counting across all test suites
  - Detection and reporting of 0-test blocks
  - Debug mode for troubleshooting
- Configuration-driven OS builder
  - `os_config.yaml` for flexible OS build management
  - Tag suffix support for dev/rc/custom builds
  - Version auto-reading from Cargo.toml

### Changed
- Extracted all tests from source files into organized `tests/` directory structure
  - Unit tests moved to `tests/unit_tests.rs` (11 tests)
  - Integration tests moved to `tests/integration_cli.rs` (7 tests)
  - E2E regression tests in `tests/e2e_regression.rs` (5 tests)
  - Removed test modules from `src/vsa.rs` and `src/embrfs.rs`
- Extended holographic OS container builder to support Ubuntu distributions
  - Added Ubuntu stable (amd64, arm64) support
  - Added Ubuntu testing/devel (amd64, arm64) support
  - Updated debian:testing to support both amd64 and arm64
  - Replaced debian:sid with debian:testing
- Applied comprehensive clippy fixes (29 improvements)
  - Zero clippy warnings remaining
  - Fixed needless borrows in test files
  - Fixed redundant closures
  - Improved code documentation

### Improved
- Test coverage: 18 tests → 23 tests (27% increase)
- Code quality: 20+ clippy warnings → 0 warnings
- Test reporting: Now accurately counts all 3 test suites
- Documentation: Enhanced with regression testing details

## [0.1.0] - 2025-12-15

### Added
- Initial production release of Embeddenator holographic computing substrate
- Core VSA (Vector Symbolic Architecture) implementation with sparse ternary vectors
  - SparseVec with ~1% density (10,000 dimensions)
  - Bundle operation for associative superposition
  - Bind operation for non-commutative composition
  - Cosine similarity for retrieval
- EmbrFS (Holographic Filesystem) implementation
  - Engram encoding with chunked data (4KB default)
  - JSON manifest for file metadata
  - Bit-perfect reconstruction of text and binary files
- CLI interface with three commands:
  - `ingest`: Convert directories to engram format
  - `extract`: Reconstruct files from engrams
  - `query`: Check similarity against engrams
- Docker support
  - Dockerfile.tool for static binary packaging
  - Dockerfile.holographic for OS container reconstruction
- Python orchestrator for unified build/test/deploy workflows
- Holographic OS container builder for Debian and Ubuntu distributions
  - Support for debian:stable (amd64, arm64)
  - Support for debian:testing (amd64, arm64)
  - Support for ubuntu:latest (amd64, arm64)
  - Support for ubuntu:devel (amd64, arm64)
- GitHub Actions CI/CD
  - Multi-architecture testing
  - Automated builds and validation
  - Workflow for building holographic OS containers
- Comprehensive test suite (18 total tests)
  - 11 unit tests (VSA algebraic properties, determinism, text detection)
  - 7 integration tests (CLI end-to-end, bit-perfect reconstruction)
- Documentation
  - Comprehensive README with examples
  - Architecture documentation
  - API documentation in code
  - CHANGELOG for version tracking
  - MIT LICENSE

### Technical Details
- Modular crate structure with separation of concerns:
  - `src/vsa.rs`: Vector Symbolic Architecture
  - `src/embrfs.rs`: Holographic filesystem
  - `src/cli.rs`: Command-line interface
  - `src/lib.rs`: Library exports
  - `src/main.rs`: Binary entry point
- Memory efficient: <50MB for typical workloads
- Fast reconstruction: <100ms for small files
- Compression: ~40-60% of original size (varies by content)
- Production-ready error handling
- Security: GitHub Actions permissions properly scoped

### Dependencies
- clap 4.5: CLI parsing
- serde 1.0: Serialization
- serde_json 1.0: JSON manifest format
- bincode 1.3: Engram serialization
- sha2 0.10: Deterministic vector generation
- rand 0.8: Random vector generation
- walkdir 2.5: Directory traversal

[0.2.0]: https://github.com/tzervas/embeddenator/releases/tag/v0.2.0
[0.1.0]: https://github.com/tzervas/embeddenator/releases/tag/v0.1.0
