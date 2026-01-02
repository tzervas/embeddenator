# Changelog

All notable changes to Embeddenator will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for 1.0.0
- Performance benchmarks for TB-scale hierarchical datasets
- Complete ARM64 CI validation and auto-trigger on main branch
- GPU runner support for VSA acceleration research
- SIMD optimization for hot-path operations
- Incremental update support for engrams
- Production monitoring and observability integration

## [0.3.0] - 2026-01-01

### Added
- **Deterministic hierarchical artifacts**
  - Stable JSON serialization for `HierarchicalManifest` using sorted keys
  - Deterministic sub-engram directory writes with sorted iteration
  - Sorted prefix/file iteration in `bundle_hierarchically` for reproducible output
- **Optional node sharding with deterministic caps**
  - New `EmbrFS::bundle_hierarchically_with_options(max_chunks_per_node)` API
  - CLI flag `bundle-hier --max-chunks-per-node` for bounded per-node indexing
  - Router+shard architecture for large nodes exceeding chunk caps
- **Multi-input ingest support**
  - CLI accepts multiple `-i/--input` arguments (files and/or directories)
  - Automatic namespacing for multiple directory roots to prevent collisions
  - Backward-compatible with single directory ingest behavior
- **Query performance improvements**
  - `Engram::build_codebook_index()` for reusable inverted index across queries
  - `Engram::query_codebook_with_index()` eliminates redundant index builds
  - Increased per-bucket candidate pool in shift-sweep for better global top-k
  - Hierarchical query now runs once using best shift instead of per-shift
- **Enhanced test coverage**
  - New `tests/hierarchical_determinism.rs` validates stable artifact generation
  - Existing E2E test `tests/hierarchical_artifacts_e2e.rs` covers full workflow
  - Query shift-sweep correctness test in `tests/query_shift_sweep.rs`

### Changed
- `ManifestLevel` and `ManifestItem` now derive `Clone` for deterministic serialization
- CLI ingest signature changed from single `PathBuf` to `Vec<PathBuf>` (repeatable `-i`)
- Query command now uses bucket-shift sweep terminology instead of "path shift"
- Updated all documentation to reflect v0.3.0 features and APIs

### Fixed
- Repaired `EmbrFS::new()` struct initialization after multi-input refactor
- Corrected `ingest_directory` implementation and added `ingest_directory_with_prefix`

### Documentation
- Updated README with v0.3.0 feature highlights and multi-input examples
- Enhanced CLI reference for `ingest`, `query`, `query-text`, and `bundle-hier`
- Updated `HIERARCHICAL_FORMAT.md` to reflect current prefix-grouping approach
- Completed `RECURSIVE_UNFOLDING.md` with directory-backed store status
- Updated `TASK_REGISTRY.md` to mark TASK-006 improvements as completed
- Marked TASK-HIE-006 as completed in master task tracker

## [0.2.0] - 2025-12-15
  - Randomized packed-vs-sparse semantic checks for dot/bind/bundle
  - Enables safe incremental migration under `bt-phase-1`

### Improved
- Reversible VSA encode/decode throughput
  - Removed per-block permutation vector allocations in `SparseVec::encode_block`
  - Bounded `decode_block` work by the caller’s `expected_size`
  - Replaced `Vec::contains` membership scans with `binary_search` on sorted indices

### Changed
- CLI `query` now reports top codebook chunk matches (in addition to root similarity)
- Test suite cleanup: removed unused imports/vars and addressed deprecated API warnings where practical

### Added
- **TASK-RES-003**: Resonator-EmbrFS integration for enhanced extraction
  - Optional resonator field in EmbrFS struct for pattern completion
  - `set_resonator()` method for configuring resonator networks
  - `extract_with_resonator()` method with robust recovery capabilities
  - Integration tests validating resonator-enhanced extraction
  - 100% reconstruction support with pattern completion fallback
- **TASK-HIE-003**: Multi-level bundling with path role binding and permutation
  - `bundle_hierarchically()` method for hierarchical engram creation
  - Path component encoding using permutation operations at each level
  - Level-by-level sparsity control for scalable hierarchical storage
  - Hierarchical manifest generation with sub-engram relationships
  - TB+ synthetic test validation for hierarchical bundling correctness
- **TASK-HIE-004**: Hierarchical extraction with manifest-guided traversal
  - `extract_hierarchically()` method for manifest-guided level traversal
  - Inverse permutation decoding for path-based reconstruction
  - Support for bit-perfect reconstruction from hierarchical structures
  - E2E test validation for complete hierarchical extraction workflow

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

[0.3.0]: https://github.com/tzervas/embeddenator/releases/tag/v0.3.0
[0.2.0]: https://github.com/tzervas/embeddenator/releases/tag/v0.2.0
[0.1.0]: https://github.com/tzervas/embeddenator/releases/tag/v0.1.0
