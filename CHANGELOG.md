# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.23.1] - 2026-01-26

### Changed
- **Supply Chain Security**: Documented maintained dependency ecosystem for unmaintained crates
  - `qlora-paste` (v1.0.20) - maintained fork of unmaintained `paste` crate
  - `qlora-gemm` (v0.20.0) - maintained fork of unmaintained `gemm` crate
  - See [MAINTAINED_DEPENDENCIES.md](../MAINTAINED_DEPENDENCIES.md) for integration guide
  - Upstream PR: https://github.com/huggingface/candle/pull/3335

## [0.23.1] - 2026-01-26

### Changed
- **Supply Chain Security**: Documented maintained dependency ecosystem
  - `qlora-paste` (v1.0.20) - maintained fork of unmaintained `paste`
  - `qlora-gemm` (v0.20.0) - maintained fork of unmaintained `gemm`
  - See [MAINTAINED_DEPENDENCIES.md](../MAINTAINED_DEPENDENCIES.md) for integration guide
- Upstream PR: https://github.com/huggingface/candle/pull/3335

## [Unreleased]

### Added
- **StreamingIngester**: Memory-efficient sync API for large file ingestion
  - `StreamingIngesterBuilder` with configurable chunk sizes and callbacks
  - `ingest_reader()`, `ingest_buffered()`, `ingest_bytes()` methods
  - `PendingChunk` for batched insertion to VersionedEmbrFS
  - Sub-MB memory footprint for multi-GB files
  
- **AsyncStreamingIngester**: Async variant for non-blocking I/O
  - `AsyncStreamingIngesterBuilder` mirroring sync API
  - `ingest_async_reader()` for `tokio::io::AsyncRead` sources
  - `ingest_async_buffered()` for `tokio::io::AsyncBufRead` sources
  - Feature-gated with `async-streaming` or `disk-image*` features
  
- **Large File Algorithm Improvements**: Hierarchical sub-engram encoding
  - `LargeFileHandler` with adaptive chunk sizing based on entropy
  - Sub-engrams bundled with MAX_BUNDLE_CAPACITY=100
  - Entropy-aware chunk sizes: 8KB (low), 16KB (medium), 32KB (high)
  - Hierarchical structure enables efficient partial retrieval

- **FUSE Read/Write Operations**: Full filesystem mutation support
  - `mkdir()`, `rmdir()`, `rename()` operations
  - Parent inode tracking for proper directory navigation
  - Fixed `readdir()` to use actual parent instead of ROOT_INO
  - Completes R/W FUSE support (was read-only for testing)

- **Disk Image Enhancements**:
  - QCOW2 full `read_at()` with L1/L2 table lookups and backing file chains
  - MBR extended partition parsing (EBR chain traversal)
  - ext4 filesystem traversal via walk_sync() with ext4 crate integration
  - `BlockDeviceSync` trait for synchronous filesystem libraries
  - `PartitionReader` implementing `positioned_io2::ReadAt`

### Changed
- Gated against `embeddenator-vsa` wrap of `trit-vsa` 0.3. Holographic `SparseVec::bind` is unchanged (trit multiplication: `P*P=P`, self-inverse unbind-by-rebind); not trit-vsa subtraction-mod-3. No direct `trit-vsa` dependency; default features only (no `fuse` / `disk-image` / `cuda`).
- `VersionedEmbrFS`: Made `config()`, `allocate_chunk_id()`, and 
  `bundle_chunks_to_root_streaming()` methods public for streaming API
- Added `positioned-io2` dependency for ext4 crate integration
- New feature flag: `async-streaming` for async API without disk-image deps

## [0.23.0] - 2026-01-25

### Added
- **Hybrid Journaling System**: Crash-safe write operations
  - `RecordHeader` with 32-byte format including CRC32 checksums
  - `JournalOp` enum for WriteChunk, DeleteChunk, WriteFile operations
  - Three durability modes: Immediate, GroupCommit, Relaxed
  - Background flush thread for group commit mode
  - Transaction replay and checkpoint functionality
- **VSA Validation Framework** (25 tests across 7 sections):
  - Byte-perfect reconstruction, SHA256 verification, size validation
  - VSA algebraic properties (bundle, bind, permutation)
  - Filesystem CRUD, concurrent reads, version conflicts
  - Memory system content-addressable retrieval
  - Computational paradigm (superposition, sequence encoding)
  - Integration tests with compression profiles
  - Stress tests (100 files, 50 updates)

### Changed
- Requires embeddenator-vsa >= 0.21.0

### Fixed
- Clippy warnings: truncate(false) for journal file, io::Error::other usage

## [0.22.0] - 2026-01-25

### Added
- Full CLI binary with 9 commands (ingest, extract, query, list, info, verify, update, compact, help)
- 4 production examples (basic_ingest, query_files, incremental_update, batch_processing)
- 3 comprehensive benchmark suites (13 groups total)

### Changed
- Performance improvements: 8.12 MB/s ingest, 9.31 MB/s extract
- API stabilized for production use

## [0.20.0-alpha.3] - 2026-01-14

### Added
- Comprehensive gap analysis documentation (GAP_ANALYSIS.md)
- Implementation roadmap with 3-milestone plan (ROADMAP.md)
- Future-looking quick start guide (QUICKSTART.md)
- Strategic merge analysis (MERGE_STRATEGY.md)

### Changed
- Merged documentation and tests from release/alpha-publish branch
- Version bump to alpha.3 (clean slate for ecosystem alignment)

## [0.20.0-alpha.2] - 2026-01-10

### Added
- Comprehensive documentation suite (LICENSE, CHANGELOG, CONTRIBUTING, docs/)
- Integration test suite with 10 end-to-end tests (all passing)
- Professional documentation with accurate limitations and capabilities
- Architecture documentation (ARCHITECTURE.md, FUSE.md, CORRECTION.md)
- Project status tracking (STATUS.md)

### Changed
- Enhanced README with detailed feature descriptions and realistic scope
- Updated inline documentation for accuracy and professionalism
- Code formatted with rustfmt for consistency

### Fixed
- Added missing MIT LICENSE file (previously only declared in Cargo.toml)
- Integration tests properly handle output directory creation
- Correction statistics test accounts for VSA variability

## [0.20.0-alpha.1] - 2026-01-09

### Changed
- **BREAKING**: Version bump from 0.2.0 to 0.20.0-alpha.1 for alignment with embeddenator ecosystem
- Converted path/git dependencies to published crates.io versions
  - `embeddenator-vsa = "0.20.0-alpha.1"`
  - `embeddenator-retrieval = "0.20.0-alpha.1"`
- Published to crates.io as alpha release

### Migration Guide
This is a significant version jump (0.2.0 → 0.20.0-alpha.1) to align versioning across the Embeddenator ecosystem. If you were using git dependencies, update to:

```toml
[dependencies]
embeddenator-fs = "0.20.0-alpha.1"
```

## [0.2.0] - 2024-12-XX (Monorepo Era)

### Added
- Complete holographic filesystem implementation (EmbrFS)
- Bit-perfect reconstruction with algebraic correction layer
- FUSE integration for kernel-level filesystem mounting (behind `fuse` feature flag)
- Hierarchical sub-engram architecture for scalability
- Incremental operations: add, modify, remove, compact
- Multiple correction strategies (BitFlips, TritFlips, BlockReplace, Verbatim)
- Comprehensive test suite (20 tests covering core functionality)
- Lock poisoning recovery for read operations

### Architecture
- `embrfs.rs`: Core holographic filesystem logic (1884 lines)
- `fuse_shim.rs`: FUSE kernel integration layer (1263 lines)
- `correction.rs`: Bit-perfect reconstruction system (531 lines)

### Limitations (Acknowledged)
- Read-only FUSE filesystem (write operations return EROFS by design)
- Holographic engrams are immutable snapshots; modifications require re-encoding
- No symbolic link support (returns ENOSYS)
- No extended attributes support

## [0.1.0] - 2024-XX-XX

### Added
- Initial extraction from [Embeddenator monorepo](https://github.com/tzervas/embeddenator)
- Phase 2A component decomposition (see ADR-016)
- Separated filesystem concerns from core VSA library
- Independent crate structure with modular dependencies

---

## Version Numbering Strategy

- **0.x.x-alpha.x**: Early alpha releases, API may change significantly
- **0.x.x-beta.x**: Feature-complete beta releases, API stabilizing
- **0.x.x**: Pre-1.0 releases with stable API within minor version
- **1.0.0**: First stable release with long-term API stability guarantees

## Semantic Versioning Notes

Given the innovative nature of holographic filesystems, we anticipate:
- **Minor version bumps** (0.x.0) for new encoding strategies or significant performance improvements
- **Patch version bumps** (0.x.y) for bug fixes and documentation improvements
- **Alpha/beta suffixes** during pre-release stabilization phases

---

[Unreleased]: https://github.com/tzervas/embeddenator-fs/compare/v0.20.0-alpha.2...HEAD
[0.20.0-alpha.2]: https://github.com/tzervas/embeddenator-fs/compare/v0.20.0-alpha.1...v0.20.0-alpha.2
[0.20.0-alpha.1]: https://github.com/tzervas/embeddenator-fs/compare/v0.2.0...v0.20.0-alpha.1
[0.2.0]: https://github.com/tzervas/embeddenator-fs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/tzervas/embeddenator-fs/releases/tag/v0.1.0
