# embeddenator-fs Quick Start (After Gap Closure)

**Note**: This is a **future-looking** quick start guide. Most features listed here are not yet implemented.
See [GAP_ANALYSIS.md](GAP_ANALYSIS.md) and [ROADMAP.md](ROADMAP.md) for current status and implementation plan.

---

## What is embeddenator-fs?

A FUSE-based holographic filesystem that stores file trees using Vector Symbolic Architecture (VSA), providing:
- **100% bit-perfect reconstruction** via algebraic correction layer
- **Hierarchical bundling** for efficient storage and retrieval
- **Content-addressable queries** using semantic similarity
- **Incremental updates** without full re-ingestion

---

## Installation (Future)

```bash
# Install CLI
cargo install embeddenator-fs --features fuse

# Or build from source
git clone https://github.com/tzervas/embeddenator-fs
cd embeddenator-fs
cargo build --release --features fuse
```

---

## CLI Usage (Planned)

### Ingest a Directory

```bash
# Create engram from directory
embrfs ingest ./my_project -o project.engram -m project.json

# With verbose output
embrfs ingest ./my_project -o project.engram -m project.json --verbose

# With compression
embrfs ingest ./my_project -o project.engram -m project.json --compress zstd
```

### Extract Files

```bash
# Extract to directory
embrfs extract project.engram -m project.json -o ./restored

# Extract with resonator recovery
embrfs extract project.engram -m project.json -o ./restored --resonator
```

### Mount as Filesystem

```bash
# Mount read-only
embrfs mount project.engram -m project.json /mnt/project

# Access files
ls /mnt/project
cat /mnt/project/README.md

# Unmount
fusermount -u /mnt/project
```

### Query Content

```bash
# Search for files containing "authentication"
embrfs query project.engram -m project.json "authentication"

# Hierarchical query (faster for large datasets)
embrfs query project.engram -m project.json "authentication" --hierarchical

# JSON output
embrfs query project.engram -m project.json "authentication" --format json
```

### Incremental Updates

```bash
# Add new file
embrfs add project.engram -m project.json new_file.txt

# Remove file
embrfs remove project.engram -m project.json old_file.txt

# Modify file
embrfs modify project.engram -m project.json updated_file.txt

# Compact (remove deleted files)
embrfs compact project.engram -m project.json
```

### Statistics

```bash
# Show engram info
embrfs stats project.engram -m project.json

# Example output:
# Files: 1,234
# Total size: 45.3 MB
# Chunks: 11,520
# Correction overhead: 2.4%
# Perfect chunks: 97.6%
```

---

## Library Usage (Current)

### Basic Example

```rust
use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};

fn main() -> std::io::Result<()> {
    let config = ReversibleVSAConfig::default();

    // Create filesystem
    let mut fs = EmbrFS::new();

    // Ingest directory
    fs.ingest_directory("./data", true, &config)?;

    // Save engram and manifest
    fs.save_engram("data.engram")?;
    fs.save_manifest("data.json")?;

    // Extract files
    let engram = EmbrFS::load_engram("data.engram")?;
    let manifest = EmbrFS::load_manifest("data.json")?;
    EmbrFS::extract(&engram, &manifest, "./restored", true, &config)?;

    Ok(())
}
```

### Hierarchical Example

```rust
use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};

fn main() -> std::io::Result<()> {
    let config = ReversibleVSAConfig::default();
    let mut fs = EmbrFS::new();

    // Ingest files
    fs.ingest_directory("./large_dataset", false, &config)?;

    // Create hierarchical structure
    let hierarchical = fs.bundle_hierarchically(500, false, &config)?;

    // Save hierarchical manifest
    embeddenator_fs::save_hierarchical_manifest(&hierarchical, "hier.json")?;

    // Query hierarchical structure
    let query = /* create query vector */;
    let bounds = embeddenator_fs::HierarchicalQueryBounds::default();
    let results = embeddenator_fs::query_hierarchical_codebook(
        &hierarchical,
        &fs.engram.codebook,
        &query,
        &bounds
    );

    println!("Found {} matching chunks", results.len());

    Ok(())
}
```

### Incremental Updates Example

```rust
use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};

fn main() -> std::io::Result<()> {
    let config = ReversibleVSAConfig::default();
    let mut fs = EmbrFS::new();

    // Initial ingestion
    fs.ingest_directory("./project", false, &config)?;

    // Add new file
    fs.add_file("new_feature.rs", "new_feature.rs".to_string(), true, &config)?;

    // Modify existing file
    fs.modify_file("updated.rs", "updated.rs".to_string(), true, &config)?;

    // Remove old file
    fs.remove_file("deprecated.rs", true)?;

    // Compact to reclaim space
    fs.compact(true, &config)?;

    // Save updated engram
    fs.save_engram("project.engram")?;
    fs.save_manifest("project.json")?;

    Ok(())
}
```

---

## Performance Characteristics (Expected)

### Ingestion
- **Throughput**: 200-300 MB/s (single-threaded)
- **Memory**: ~4x chunk size per worker
- **Chunk size**: 4 KB default (tunable)

### Query
- **Flat query**: O(N) where N = total chunks
- **Hierarchical query**: O(log N) with beam search
- **Latency**: <100ms for 1M chunks (P50)

### Extraction
- **Throughput**: 300-400 MB/s (correction overhead minimal)
- **Memory**: Streaming, ~16 MB typical

### FUSE Operations
- **Read latency**: <1ms (cached), <10ms (uncached)
- **Throughput**: 100-200 MB/s sequential reads

---

## Configuration Tuning

### Chunk Size
```rust
// Default 4KB, tune based on file sizes
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

// Larger chunks: better for large files, higher memory
// Smaller chunks: better for small files, more overhead
```

### Hierarchical Query Bounds
```rust
let bounds = HierarchicalQueryBounds {
    k: 10,               // Top-k results
    candidate_k: 100,    // Candidates before reranking
    beam_width: 32,      // Frontier nodes
    max_depth: 4,        // Tree depth
    max_expansions: 128, // Node expansions
    max_open_indices: 16,  // LRU cache size
    max_open_engrams: 16,  // LRU cache size
};
```

### Correction Store
```rust
// Automatic - no tuning needed
// Tracks:
// - BitFlips: sparse differences
// - BlockReplace: clustered differences
// - Verbatim: complete mismatches
```

---

## Troubleshooting

### "Out of memory during ingestion"
→ Use streaming ingestion (planned): `--stream` flag

### "Query is slow"
→ Use hierarchical bundling: `bundle_hierarchically()`
→ Tune `HierarchicalQueryBounds` parameters

### "High correction overhead"
→ Check data characteristics (high entropy → more corrections)
→ Consider compression before ingestion

### "FUSE mount fails"
→ Check FUSE is installed: `fusermount --version`
→ Check permissions: may need `user_allow_other` in `/etc/fuse.conf`

---

## What's Next?

See [ROADMAP.md](ROADMAP.md) for planned features and timeline.

**Current Phase**: Phase 2A - Component Extraction (Alpha)
**Next Milestone**: v0.20.0-beta.1 (CLI + Examples + Tests)

---

## Resources

- [GAP_ANALYSIS.md](GAP_ANALYSIS.md) - Detailed gap analysis
- [ROADMAP.md](ROADMAP.md) - Implementation roadmap
- [API Docs](https://docs.rs/embeddenator-fs) - Library documentation
- [Main Repo](https://github.com/tzervas/embeddenator) - Parent project

---

**Note**: This quick start describes the **intended** user experience after gap closure. Most CLI features shown here do not yet exist. Current version (v0.20.0-alpha.3) is library-only. See [GAP_ANALYSIS.md](GAP_ANALYSIS.md) for implementation status.
