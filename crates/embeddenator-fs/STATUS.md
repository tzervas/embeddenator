# embeddenator-fs Status

**Version:** 0.24.0
**crates.io:** [embeddenator-fs](https://crates.io/crates/embeddenator-fs)
**Last Updated:** 2026-01-27

---

## Overview

EmbrFS: FUSE-based holographic filesystem for encoding directory trees into high-dimensional sparse vectors with bit-perfect reconstruction.

---

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `fuse` | No | FUSE filesystem support (read/write) |
| `async-streaming` | No | Async streaming with tokio |
| `disk-image` | No | QCOW2/raw disk image support |
| `disk-image-portable` | No | Portable disk image (no io_uring) |

---

## Test Coverage

- **Unit Tests:** 74 (highest in workspace)
- **Benchmarks:** 4 (filesystem, ingest, query, incremental)

---

## Key Components

| Component | Status | Description |
|-----------|--------|-------------|
| EmbrFS | Production | Core filesystem abstraction |
| Engram | Production | Holographic encoding container |
| Manifest | Production | File metadata and chunk mapping |
| Hierarchical | Production | Multi-level chunking |
| FUSE Mount | Production | Read/write FUSE with signal handling |
| Signal Handlers | Production | Graceful unmount on SIGINT/SIGTERM/SIGHUP |
| Extended Attributes | Production | xattr support for engram metadata |
| Delta Encoding | Production | Incremental modification support |
| Streaming Decode | Production | Memory-efficient file reading |
| Range Queries | Production | O(log n) byte-offset index |

---

## Known Limitations

- Large file reconstruction (>10MB) has degraded quality without correction layer
- Deep hierarchy paths (>20 levels) may have reduced accuracy

---

## Remaining Tasks

- [x] FUSE write support
- [x] Production hardening for FUSE (signal handlers, xattr)
- [ ] Disk image support testing
- [ ] Large-scale (TB) validation

---

## Links

- **Documentation:** https://docs.rs/embeddenator-fs
- **Repository:** https://github.com/tzervas/embeddenator-fs
