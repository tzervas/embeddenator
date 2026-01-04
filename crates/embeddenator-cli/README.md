# embeddenator-cli

Command-line interface for the Embeddenator holographic computing substrate.

## Overview

This crate provides a modular CLI for Embeddenator operations, extracted from the main embeddenator repository as part of the Phase 2A component decomposition.

## Features

- **Ingest**: Encode files/directories into holographic engrams
- **Extract**: Bit-perfect reconstruction from engrams
- **Query**: Similarity search using VSA cosine similarity
- **Mount**: FUSE filesystem interface (requires `fuse` feature)
- **Update**: Incremental operations (add, remove, modify, compact)

## Installation

```bash
cargo build --release
```

With FUSE support:
```bash
cargo build --release --features fuse
```

## Usage

### Ingest files
```bash
embeddenator-cli ingest -i ./mydata -e data.engram -m data.json -v
```

### Extract files
```bash
embeddenator-cli extract -e data.engram -m data.json -o ./restored -v
```

### Query similarity
```bash
embeddenator-cli query -e data.engram -q ./testfile.txt -v
```

### Build hierarchical artifacts
```bash
embeddenator-cli bundle-hier -e data.engram -m data.json --out-hierarchical-manifest hier.json --out-sub-engrams-dir sub_engrams -v
```

### Mount as FUSE filesystem (requires `--features fuse`)
```bash
embeddenator-cli mount -e data.engram -m data.json /mnt/engram -v
```

## Architecture

The CLI is organized into modular components:

```
src/
├── lib.rs              # Public API and Clap definitions
├── main.rs             # Binary entry point
├── commands/           # Command implementations
│   ├── ingest.rs
│   ├── extract.rs
│   ├── query.rs
│   ├── bundle_hier.rs
│   ├── mount.rs
│   └── update.rs
└── utils/              # Helper utilities
    ├── path.rs
    └── mod.rs
```

## Dependencies

This crate uses the Phase 2A component libraries:
- `embeddenator-vsa`: Vector Symbolic Architecture operations
- `embeddenator-fs`: Filesystem and engram operations
- `embeddenator-retrieval`: Hierarchical retrieval
- `embeddenator-io`: I/O utilities

## Error Handling

All command handlers return `anyhow::Result<()>` for consistent error handling across the CLI.

## License

MIT
