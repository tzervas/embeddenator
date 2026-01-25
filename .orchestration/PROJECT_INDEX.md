# Embeddenator Project Index

## Overview

Embeddenator is a **Rust-based holographic computing substrate** implementing Vector Symbolic Architecture (VSA) with sparse ternary representations. The project is undergoing **Phase 2A component decomposition** - transitioning from a monorepo to modular crates.

---

## Component Catalog

### Core Components

#### 1. embeddenator (Main Repository)
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator` |
| **Version** | v0.19.4 |
| **Role** | Meta-repo containing full VSA implementation |
| **Status** | Active - undergoing decomposition |
| **LOC** | ~18,013 |

**Key Modules:**
- `src/vsa/` - Vector Symbolic Architecture core
- `src/retrieval/` - Resonator network
- `src/fs/` - EmbrFS filesystem
- `src/cli.rs` - Command-line interface
- `benches/` - Benchmark suite

---

#### 2. embeddenator-vsa
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-vsa` |
| **Version** | v0.1.0 |
| **Role** | Foundational VSA operations |
| **Dependencies** | None (internal) |
| **LOC** | ~4,311 |

**Exports:**
- `HolographicVector` - Core 10,000-dim sparse ternary vectors
- `bind()` / `unbind()` - Holographic binding operations
- `superpose()` - Vector superposition
- `cosine_similarity_simd()` - SIMD-accelerated similarity
- `CodeBook` - Atomic symbol codebook

---

#### 3. embeddenator-retrieval
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-retrieval` |
| **Version** | v0.2.0 |
| **Role** | Retrieval and resonator network |
| **Dependencies** | `embeddenator-vsa` |
| **LOC** | ~1,425 |

**Exports:**
- `ResonatorNetwork` - Iterative factorization
- `HierarchicalRetrieval` - Multi-level retrieval
- `SignatureStore` - Content-addressable storage
- Correction algorithms (majority vote, soft threshold)

---

#### 4. embeddenator-fs
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-fs` |
| **Version** | v0.2.0 |
| **Role** | FUSE filesystem (EmbrFS) |
| **Dependencies** | `embeddenator-vsa`, `embeddenator-retrieval` |
| **LOC** | ~3,698 |

**Exports:**
- `EmbrFS` - Holographic filesystem
- `FuseShim` - FUSE interface (feature-gated)
- File encoding/decoding pipeline
- Hierarchical bundling

**Features:**
- `fuse` - Enable FUSE mount support

---

### Infrastructure Components

#### 5. embeddenator-obs
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-obs` |
| **Version** | v0.1.1 |
| **Role** | Observability infrastructure |
| **Dependencies** | None |
| **LOC** | ~969 |

**Exports:**
- `HighResTimer` - Nanosecond timing
- `MetricsCollector` - Statistics aggregation
- Structured logging macros
- Tracing integration (optional)

---

#### 6. embeddenator-io
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-io` |
| **Version** | v0.1.1 |
| **Role** | Serialization format |
| **Dependencies** | None |
| **LOC** | ~184 |

**Exports:**
- `Envelope` - Binary container format
- Engram serialization
- Version-tagged encoding

---

#### 7. embeddenator-interop
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-interop` |
| **Version** | v0.2.0 |
| **Role** | System integration |
| **Dependencies** | `embeddenator-vsa`, `embeddenator-fs` |
| **LOC** | ~328 |

**Exports:**
- Kernel-level integration hooks
- External system bridges
- FFI boundaries (planned)

---

### User-Facing Components

#### 8. embeddenator-cli
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-cli` |
| **Version** | v0.1.0 |
| **Role** | Command-line interface |
| **Dependencies** | vsa, retrieval, fs, io (git) |
| **LOC** | ~1,323 |

**Commands:**
- `ingest` - Import files into holographic store
- `extract` - Retrieve files from store
- `query` - Semantic search
- `mount` - FUSE mount (feature-gated)
- `bundle-hier` - Hierarchical bundling
- `update` - Add/remove/modify/compact (stub)

---

### Development Components

#### 9. embeddenator-contract-bench
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-contract-bench` |
| **Version** | v0.2.1 |
| **Role** | Deterministic benchmarks |
| **Dependencies** | `embeddenator` (path) |
| **LOC** | ~2,186 |

**Features:**
- Golden hash verification
- Deterministic RNG fixtures
- Performance regression tracking

---

#### 10. embeddenator-testkit
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-testkit` |
| **Version** | v0.1.1 |
| **Role** | Test utilities |
| **Dependencies** | `embeddenator` (git tag v0.20.0) |
| **LOC** | ~3 |
| **Status** | ⚠️ Minimal - needs expansion |

---

#### 11. embeddenator-workspace
| Property | Value |
|----------|-------|
| **Path** | `/home/kang/Documents/projects/embdntr/embeddenator-workspace` |
| **Version** | v0.1.0-alpha.1 |
| **Role** | Workspace management |
| **Dependencies** | None |
| **LOC** | ~63 |
| **Status** | ⚠️ Minimal implementation |

---

## Technology Stack

| Category | Technology |
|----------|------------|
| **Language** | Rust (2021 edition) |
| **Vector Dims** | 10,000 (sparse ternary) |
| **Serialization** | bincode, serde |
| **Hashing** | SHA-256 |
| **Filesystem** | FUSE (via `fuser` crate) |
| **CLI** | clap v4.x |
| **Async** | tokio (where needed) |
| **SIMD** | Native Rust SIMD |

---

## Version Matrix

| Component | Current | Target | Published |
|-----------|---------|--------|-----------|
| embeddenator | v0.19.4 | v0.20.0+ | ❌ |
| embeddenator-vsa | v0.1.0 | v0.2.0 | ❌ |
| embeddenator-retrieval | v0.2.0 | v0.2.0 | ❌ |
| embeddenator-fs | v0.2.0 | v0.2.0 | ❌ |
| embeddenator-obs | v0.1.1 | v0.2.0 | ❌ |
| embeddenator-io | v0.1.1 | v0.2.0 | ❌ |
| embeddenator-interop | v0.2.0 | v0.2.0 | ❌ |
| embeddenator-cli | v0.1.0 | v0.2.0 | ❌ |
| embeddenator-contract-bench | v0.2.1 | v0.3.0 | ❌ |
| embeddenator-testkit | v0.1.1 | v0.2.0 | ❌ |
| embeddenator-workspace | v0.1.0-alpha.1 | v0.1.0 | ❌ |

*None of the crates are published to crates.io yet.*
