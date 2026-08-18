# embeddenator-fs

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/embeddenator-fs/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-fs/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/embeddenator-fs/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-fs/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

[![Crates.io](https://img.shields.io/crates/v/embeddenator-fs.svg)](https://crates.io/crates/embeddenator-fs)
[![Documentation](https://docs.rs/embeddenator-fs/badge.svg)](https://docs.rs/embeddenator-fs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A holographic filesystem implementation using Vector Symbolic Architecture (VSA) for encoding entire directory trees into high-dimensional sparse vectors with bit-perfect reconstruction guarantees.

**Independent component** extracted from the Embeddenator monolithic repository. Part of the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

**Repository:** [https://github.com/tzervas/embeddenator-fs](https://github.com/tzervas/embeddenator-fs)

**Status:** Alpha - Core functionality complete, API may change. Suitable for experimental use and research.

## What is EmbrFS?

EmbrFS (Embeddenator Filesystem) is a novel approach to filesystem storage that encodes files and directories as holographic "engrams" - bundled high-dimensional sparse vectors. Unlike traditional filesystems that store files as sequential blocks, EmbrFS distributes file information across a holographic representation, enabling:

- **Bit-perfect reconstruction** through algebraic correction layers
- **Holographic properties** - complete information distributed across the representation
- **Hierarchical scalability** - sub-engrams for bounded memory usage
- **Read-only FUSE mounting** - kernel-level filesystem integration
- **Incremental operations** - add/modify/remove files without full rebuilds

### Realistic Scope & Limitations

**What EmbrFS IS:**
-  A research-grade holographic encoding system for filesystems
-  A read-only FUSE filesystem for browsing encoded directory trees
-  An experimental VSA application demonstrating bit-perfect reconstruction
-  A foundation for exploring holographic storage and retrieval patterns

**What EmbrFS IS NOT:**
-  A replacement for production filesystems (ext4, btrfs, ZFS)
-  A compression tool (overhead varies, typically 0-5% for correction layer)
-  A writable filesystem (holographic engrams are immutable snapshots)
-  A distributed storage system (single-machine only)

**Current Limitations:**
- Read-only FUSE operations (by design - engrams are immutable)
- No symbolic link support (returns ENOSYS)
- No extended attributes
- No write/modify operations through FUSE (modifications require re-encoding)
- Alpha API stability (breaking changes possible)

## Features

### Core Capabilities

- **Holographic Encoding**: Encodes files into SparseVec representations using VSA
- **Bit-Perfect Reconstruction**: 100% accurate file recovery via correction layer
  - Primary SparseVec encoding
  - Immediate verification on encode
  - Algebraic correction store for exact differences
- **Hierarchical Architecture**: Scales to large filesystems via sub-engram trees
- **Incremental Operations**:
  - `add_files` - Add new files without full rebuild
  - `modify_files` - Update existing files
  - `remove_files` - Soft-delete files
  - `compact` - Hard rebuild to reclaim space
- **FUSE Integration** (optional `fuse` feature):
  - Mount engrams as read-only filesystems
  - Standard Unix tools (ls, cat, grep) work transparently
  - Kernel-level integration via fuser library
- **Correction Strategies**:
  - `BitFlips` - Sparse bit-level corrections
  - `TritFlips` - Ternary value corrections  
  - `BlockReplace` - Contiguous region replacement
  - `Verbatim` - Full data storage (fallback)

### Architecture Highlights

```
User Tools (ls, cat, etc.)
         ↓
  FUSE Kernel Interface (fuse_shim.rs)
         ↓
  Holographic Filesystem Core (embrfs.rs)
         ↓
  Correction Layer (correction.rs)
         ↓
  VSA Primitives (embeddenator-vsa)
```

**File Structure:**
- `embrfs.rs` - Core filesystem logic (1,884 lines)
- `fuse_shim.rs` - FUSE integration (1,263 lines)
- `correction.rs` - Bit-perfect reconstruction (531 lines)

**Test Coverage:**
- 20 tests covering core functionality
- All tests passing in CI
- Unit tests for correction logic
- Integration tests for FUSE operations

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
embeddenator-fs = "0.20.0-alpha.3"

# Enable FUSE mounting support (Linux only)
embeddenator-fs = { version = "0.20.0-alpha.3", features = ["fuse"] }
```

## Usage

### Basic API

```rust
use embeddenator_fs::{EmbrFS, IngestOptions};
use std::path::Path;

// Create a new holographic filesystem
let mut fs = EmbrFS::new();

// Ingest a directory tree
let options = IngestOptions::default();
fs.ingest_directory(Path::new("/path/to/data"), &options)?;

// Save the engram
fs.save("filesystem.engram")?;

// Later: Load and extract
let fs = EmbrFS::load("filesystem.engram")?;
fs.extract_all(Path::new("/output/dir"))?;
```

### FUSE Mounting (Linux Only)

```rust
use embeddenator_fs::{EmbrFS, fuse::mount_embrfs};

// Load an engram
let fs = EmbrFS::load("filesystem.engram")?;

// Mount as read-only filesystem
let mountpoint = Path::new("/mnt/embrfs");
mount_embrfs(fs, mountpoint, &[])?;

// Now access files normally:
// $ ls /mnt/embrfs
// $ cat /mnt/embrfs/file.txt
// $ grep "pattern" /mnt/embrfs/**/*.log
```

**FUSE Limitations:**
- Read-only operations only (writes return EROFS)
- No symbolic links (readlink returns ENOSYS)
- Simplified permission model
- Requires root or user_allow_other in /etc/fuse.conf

## Command-Line Interface

The `embeddenator-fs` CLI provides convenient access to all filesystem operations:

### Installation

```bash
# From source
cargo install --path embeddenator-fs

# Or build locally
cargo build --release --manifest-path embeddenator-fs/Cargo.toml
```

### CLI Commands

#### Ingest files into engram
```bash
embeddenator-fs ingest -i ./mydata -e data.engram -v
embeddenator-fs ingest -i file1.txt -i file2.txt -e files.engram
```

#### Extract files from engram
```bash
embeddenator-fs extract -e data.engram -o ./restored -v
```

#### Query for similar files
```bash
embeddenator-fs query -e data.engram -q search.txt -k 10
```

#### List files in engram
```bash
embeddenator-fs list -e data.engram -v
```

#### Show engram information
```bash
embeddenator-fs info -e data.engram
```

#### Verify engram integrity
```bash
embeddenator-fs verify -e data.engram -v
```

#### Incremental updates
```bash
# Add a new file
embeddenator-fs update add -e data.engram -f newfile.txt

# Remove a file (soft delete)
embeddenator-fs update remove -e data.engram -p oldfile.txt

# Modify an existing file
embeddenator-fs update modify -e data.engram -f updated.txt

# Compact engram (hard rebuild)
embeddenator-fs update compact -e data.engram -v
```

### CLI Features

-  User-friendly progress indicators
-  Verbose mode for detailed output
-  Helpful error messages
-  Performance statistics
-  Bit-perfect verification
-  Incremental operations

## Examples

The `examples/` directory contains runnable examples:

### Basic Ingestion
```bash
cargo run --example basic_ingest
```
Demonstrates simple file ingestion and extraction with verification.

### Query Files
```bash
cargo run --example query_files
```
Shows how to query for similar files in an engram using VSA cosine similarity.

### Incremental Updates
```bash
cargo run --example incremental_update
```
Demonstrates add/modify/remove operations and compaction.

### Batch Processing
```bash
cargo run --example batch_processing --release
```
Tests performance with larger numbers of files (100+ files, 4KB each).

## Benchmarks

Performance benchmarks using Criterion:

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --manifest-path embeddenator-fs/Cargo.toml

# Run specific benchmark
cargo bench --bench ingest_benchmark
cargo bench --bench query_benchmark
cargo bench --bench incremental_benchmark
```

### Benchmark Coverage

- **Ingestion benchmarks**: Single files (1KB-10MB), multiple small files (10-100), large files, nested directories
- **Query benchmarks**: Codebook queries, path-sweep queries, scaling with file count, index build time
- **Incremental benchmarks**: Add file, remove file, modify file, compact, sequential adds

### Expected Performance

- **Ingestion**: 20-50 MB/s (debug), 50-100+ MB/s (release)
- **Extraction**: 50-100 MB/s (debug), 100-200+ MB/s (release)
- **Queries**: Sub-millisecond for small codebooks, milliseconds for large
- **Incremental adds**: ~1-5ms per file
- **Compaction**: Similar to full re-ingestion

### Hierarchical Sub-Engrams

For large filesystems, use hierarchical encoding:

```rust
let options = IngestOptions {
    max_files_per_engram: 1000,  // Split into sub-engrams
    beam_width: 10,               // Beam search for retrieval
    ..Default::default()
};

fs.ingest_directory(path, &options)?;
```

## Performance Characteristics

**Encoding Performance:**
- Time: O(N) where N = total file size
- Space: O(chunks) + correction overhead (typically 0-5%)
- Chunk size: 4KB default (configurable)

**Retrieval Performance:**
- Beam-limited hierarchical search: O(beam_width × max_depth)
- LRU caching reduces repeated disk I/O
- Inverted index enables sub-linear candidate generation

**Correction Overhead:**
- Observed: 0-5% typical for structured data
- Varies with data entropy and VSA dimensionality
- Statistics tracked per-engram

## Development

```bash
# Clone and build
git clone https://github.com/tzervas/embeddenator-fs
cd embeddenator-fs
cargo build

# Run tests
cargo test

# Run tests with FUSE support
cargo test --features fuse

# Build documentation
cargo doc --open

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check
```

For cross-component development with other Embeddenator crates:

```toml
# Add to workspace Cargo.toml
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
```

## Documentation

- [CHANGELOG.md](CHANGELOG.md) - Version history and migration guides
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines and PR process
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Detailed system architecture
- [docs/FUSE.md](docs/FUSE.md) - FUSE implementation details
- [docs/CORRECTION.md](docs/CORRECTION.md) - Bit-perfect reconstruction system
- [API Documentation](https://docs.rs/embeddenator-fs) - Complete rustdoc API reference

## Related Projects

- [embeddenator](https://github.com/tzervas/embeddenator) - Parent monorepo with CLI and additional tools
- [embeddenator-vsa](https://crates.io/crates/embeddenator-vsa) - Core VSA primitives
- [embeddenator-retrieval](https://crates.io/crates/embeddenator-retrieval) - Retrieval and indexing

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

This project is in active development. Expect API changes in minor versions until 1.0.

## License

MIT - See [LICENSE](LICENSE) file for details.

Copyright (c) 2024-2026 Tyler Zervas
