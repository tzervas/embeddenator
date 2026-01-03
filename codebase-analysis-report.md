# Embeddenator Codebase Analysis Report

**Analysis Date:** January 2, 2026  
**Version Analyzed:** 0.19.3  
**Report Type:** Deep Code Analysis (Implementation-First)

---

## Executive Summary

Embeddenator is a **holographic computing substrate** implementing **Vector Symbolic Architecture (VSA)** with sparse ternary vectors. The system encodes filesystems into "engrams"—holographic representations that enable:

- 100% bit-perfect reconstruction via correction layer
- Algebraic operations (bundle, bind) on encoded data
- Hierarchical chunked encoding for large datasets
- Semantic similarity search via inverted indices

**Current Implementation State:** The codebase has a functional VSA substrate with a hybrid sparse/packed representation, but uses **fixed 10,000 dimensions** and **1% sparsity target**. The ternary encoding uses `usize` index lists (not balanced ternary integers) with optional packed 2-bit-per-trit representation.

---

## 1. Core Architecture Analysis

### 1.1 Module Organization

```
src/
├── vsa/                    # Vector Symbolic Architecture core
│   ├── vsa.rs             # SparseVec: primary vector type
│   ├── ternary.rs         # Foundational Trit/Tryte3/Word6 types
│   ├── ternary_vec.rs     # PackedTritVec: dense 2-bit encoding
│   ├── dimensional.rs     # Variable-depth hyperdimensional config
│   └── simd_cosine.rs     # Platform-specific SIMD acceleration
├── core/
│   ├── codebook.rs        # BalancedTernaryWord, basis vectors
│   ├── resonator.rs       # Pattern completion/factorization
│   └── correction.rs      # 100% reconstruction guarantee layer
├── retrieval/
│   ├── retrieval.rs       # TernaryInvertedIndex for search
│   └── signature.rs       # Signature-based candidate generation
├── fs/
│   ├── embrfs.rs          # Holographic filesystem layer
│   └── fuse_shim.rs       # FUSE mount support
├── io/
│   └── envelope.rs        # Binary serialization with compression
└── cli/
    └── mod.rs             # CLI commands: ingest, extract, query
```

### 1.2 Primary Data Flow

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Raw Files   │ ──> │ CDC Chunking     │ ──> │ SparseVec       │
│             │     │ (4KB default)    │     │ Encoding        │
└─────────────┘     └──────────────────┘     └─────────────────┘
                                                     │
                                                     ▼
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Engram      │ <── │ Hierarchical     │ <── │ Bundle/Bind     │
│ (root.engram)     │ Bundle           │     │ Operations      │
└─────────────┘     └──────────────────┘     └─────────────────┘
        │
        │ + Manifest (JSON) + Corrections + Codebook
        ▼
┌─────────────────────────────────────────────────────────────┐
│ Reconstruction: decode + corrections = 100% original        │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. VSA Implementation Details

### 2.1 SparseVec ([vsa.rs](src/vsa/vsa.rs))

The primary vector type stores **positive and negative index lists**, not actual ternary values:

```rust
pub struct SparseVec {
    pub pos: Vec<usize>,  // Indices with +1 value
    pub neg: Vec<usize>,  // Indices with -1 value
}
```

**Key Constants:**
- `DIM: usize = 10_000` — Fixed dimensionality
- Sparsity target: ~1% (100 positive + 100 negative indices)

**Operations Implemented:**

| Operation | Method | Complexity | Notes |
|-----------|--------|------------|-------|
| **Bundle** | `bundle()` | O(n) | Pairwise conflict-cancel superposition |
| **Bind** | `bind()` | O(n log n) | Element-wise multiplication (merge-join) |
| **Cosine** | `cosine()` | O(n) | Sorted intersection counting |
| **Permute** | `permute(shift)` | O(n log n) | Cyclic index shift + re-sort |
| **Thin** | `thin(target)` | O(n log n) | Density reduction with ratio preservation |

**Bundle Variants:**
1. `bundle()` — Pairwise, fast but non-associative for 3+ vectors
2. `bundle_sum_many()` — Associative, accumulates all contributions before threshold
3. `bundle_hybrid_many()` — Auto-selects based on collision probability

### 2.2 PackedTritVec ([ternary_vec.rs](src/vsa/ternary_vec.rs))

Dense representation using **2 bits per trit**:

```rust
pub struct PackedTritVec {
    len: usize,
    data: Vec<u64>,  // 32 trits per u64
}
```

**Encoding:**
- `00` = Zero (0)
- `01` = Positive (+1)
- `10` = Negative (-1)
- `11` = Unused/invalid

**Feature-Gated Usage:** Under `bt-phase-2` feature, operations automatically switch to packed representation when vectors exceed 25% density:

```rust
#[cfg(feature = "bt-phase-2")]
if total > DIM / 4 && min_nnz > DIM / 32 {
    // Use thread-local packed scratch buffers
    return packed_bundle_path(...);
}
```

### 2.3 Foundational Ternary Types ([ternary.rs](src/vsa/ternary.rs))

Mathematically rigorous single-trit layer:

```rust
#[repr(i8)]
pub enum Trit {
    N = -1,  // Negative
    Z = 0,   // Zero
    P = 1,   // Positive
}
```

**Algebraic Properties (Proven in Code):**

| Property | Addition | Multiplication |
|----------|----------|----------------|
| Commutative | ✓ | ✓ |
| Associative | ✓ | ✓ |
| Identity | Z (0) | P (+1) |
| Self-inverse | a + (-a) = Z | a × a = P (for non-zero) |
| Zero behavior | a + Z = a | a × Z = Z |

**Compound Types:**

| Type | Trits | States | Range | Bits |
|------|-------|--------|-------|------|
| `Trit` | 1 | 3 | {-1, 0, +1} | 1.585 |
| `Tryte3` | 3 | 27 | [-13, +13] | 4.75 |
| `Word6` | 6 | 729 | [-364, +364] | 9.51 |

### 2.4 BalancedTernaryWord ([codebook.rs](src/core/codebook.rs))

64-bit balanced ternary encoding unit:

```rust
pub struct BalancedTernaryWord {
    packed: u64,  // 61 data bits + 3 metadata bits
}
```

**Layout:**
- Bits 0-60: 38 trits of data payload
- Bits 61-63: Metadata tag (Data/SemanticOutlier/Residual/etc.)

**Range:** ±675,425,858,836,496,044 (±6.75 × 10¹⁷)

---

## 3. Encoding/Decoding Pipeline

### 3.1 Data Encoding ([vsa.rs#encode_data](src/vsa/vsa.rs))

```rust
pub fn encode_data(data: &[u8], config: &ReversibleVSAConfig, path: Option<&str>) -> Self
```

**Algorithm:**
1. **Path-Based Shift:** SHA256 hash of path → shift offset
2. **Block Splitting:** Chunk data by `config.block_size` (default 256 bytes)
3. **Per-Block Encoding:**
   - Each byte maps to an index: `base_idx = (position + shift) % DIM`
   - High bit determines polarity: `0x80` → negative, else positive
   - Index offset: `(base_idx + (byte & 0x7F)) % DIM`
4. **Hierarchical Bundle:** Combine blocks into single vector

### 3.2 Data Decoding ([vsa.rs#decode_data](src/vsa/vsa.rs))

```rust
pub fn decode_data(&self, config: &ReversibleVSAConfig, path: Option<&str>, expected_size: usize) -> Vec<u8>
```

**Algorithm:**
1. Reverse path-based shift calculation
2. For each expected byte position:
   - Compute base index
   - Binary search pos/neg lists for offsets 0-127
   - Reconstruct byte from found index + polarity

**Limitation:** Current decode is lossy for multi-block data; relies on `CorrectionStore` for 100% fidelity.

### 3.3 Correction Layer ([correction.rs](src/core/correction.rs))

Guarantees bit-perfect reconstruction:

```rust
pub enum CorrectionType {
    None,                              // Exact match
    BitFlips(Vec<(u64, u8)>),          // Position + XOR mask
    TritFlips(Vec<(u64, Trit, Trit)>), // Position + was + should_be
    BlockReplace { offset, original }, // Wholesale replacement
    Verbatim(Vec<u8>),                 // Full data (high-entropy)
}
```

**Invariant:** `original = decode(encode(original)) + correction`

---

## 4. Retrieval System

### 4.1 TernaryInvertedIndex ([retrieval.rs](src/retrieval/retrieval.rs))

Sub-linear candidate generation for similarity search:

```rust
pub struct TernaryInvertedIndex {
    pos_postings: Vec<Vec<usize>>,  // DIM posting lists for +1
    neg_postings: Vec<Vec<usize>>,  // DIM posting lists for -1
    max_id: usize,
}
```

**Query Algorithm:**
1. For each query dimension d with value +1:
   - Accumulate +1 to scores of IDs in `pos_postings[d]`
   - Accumulate -1 to scores of IDs in `neg_postings[d]`
2. For each query dimension d with value -1:
   - Opposite accumulation
3. Sort by score, return top-k

**Complexity:** O(k·d) where k = average posting list size, d = query density

### 4.2 Signature-Based Index ([signature.rs](src/retrieval/signature.rs))

Alternative candidate generation using locality-sensitive hashing:

```rust
pub struct TernarySignatureIndex {
    probe_dims: Vec<usize>,           // 24 fixed probe dimensions
    buckets: HashMap<u64, Vec<usize>>, // signature → IDs
}
```

**Signature Encoding:** 2 bits per probe dimension → 48-bit signature
**Multi-Probe:** Radius-1 variants for bucket boundary softening

### 4.3 Hierarchical Query ([embrfs.rs](src/fs/embrfs.rs))

Selective unfolding for large engrams:

```rust
pub struct HierarchicalQueryBounds {
    k: usize,            // 10 results
    candidate_k: usize,  // 100 candidates per node
    beam_width: usize,   // 32 frontier nodes
    max_depth: usize,    // 4 levels
    max_expansions: usize, // 128 node expansions
}
```

**Algorithm:** Beam search over sub-engram tree, expanding only high-similarity nodes.

---

## 5. Filesystem Layer (EmbrFS)

### 5.1 Engram Structure ([embrfs.rs](src/fs/embrfs.rs))

```rust
pub struct Engram {
    pub root: SparseVec,
    pub codebook: HashMap<usize, SparseVec>,
}
```

**Storage:**
- `root.engram` — Bincode serialized, optionally compressed (Zstd/LZ4)
- `manifest.json` — File metadata + chunk mappings
- `corrections.bin` — CorrectionStore for 100% fidelity

### 5.2 Chunk Size

```rust
pub const DEFAULT_CHUNK_SIZE: usize = 4096;  // 4KB
```

No Content-Defined Chunking (CDC) currently implemented; uses fixed-size blocks.

### 5.3 FUSE Mount ([fuse_shim.rs](src/fs/fuse_shim.rs))

Optional userspace filesystem mount (feature-gated):

```rust
#[cfg(feature = "fuse")]
pub struct EngramFS { ... }
```

---

## 6. Performance Characteristics

### 6.1 Current Benchmarks (from [benches/vsa_ops.rs](benches/vsa_ops.rs))

| Operation | Time | Notes |
|-----------|------|-------|
| Bundle (sparse) | ~2-5 µs | Two ~200-element vectors |
| Bind (sparse) | ~5-10 µs | Merge-join algorithm |
| Cosine (sparse) | ~1-2 µs | Four intersection counts |
| Encode 4KB | ~50-100 µs | Including hierarchical bundle |
| Decode 4KB | ~100-200 µs | Binary search per byte |

### 6.2 Memory Usage

| Component | Size | Notes |
|-----------|------|-------|
| SparseVec (typical) | ~1.6 KB | 200 indices × 8 bytes |
| PackedTritVec (DIM=10K) | 2.5 KB | 10K × 2 bits / 8 |
| Codebook (10K chunks) | ~16 MB | + overhead |
| Inverted Index (DIM=10K) | Variable | Empty → posting lists |

### 6.3 SIMD Status ([simd_cosine.rs](src/vsa/simd_cosine.rs))

- **x86_64 AVX2:** Defined but falls back to scalar (sorted merge not SIMD-friendly)
- **aarch64 NEON:** Stub implementation, falls back to scalar
- **Effective:** Scalar intersection counting for sparse vectors

---

## 7. Feature Flags & Migration Status

### 7.1 Current Features

| Feature | Status | Description |
|---------|--------|-------------|
| `fuse` | Optional | FUSE filesystem mount |
| `compression-zstd` | Optional | Zstd envelope compression |
| `compression-lz4` | Optional | LZ4 envelope compression |
| `metrics` | Optional | Runtime metrics collection |
| `logging` | Optional | Tracing-based logging |
| `bt-phase-1` | Testing | Ternary refactor invariant tests |
| `bt-phase-2` | Testing | Packed representation fast-path |
| `bt-phase-3` | Testing | Future: full ternary substrate |

### 7.2 Balanced Ternary Migration

The codebase is mid-migration toward a balanced-ternary-first implementation:

| Phase | Status | Contents |
|-------|--------|----------|
| Phase 1 | ✅ Complete | Foundational `Trit`/`Tryte3`/`Word6` types |
| Phase 2 | 🔄 In Progress | `PackedTritVec` with conditional fast-paths |
| Phase 3 | 📋 Planned | Replace `SparseVec` internals with packed |

---

## 8. Test Coverage Analysis

### 8.1 Test Categories

| Category | File | Coverage |
|----------|------|----------|
| E2E Workflow | `e2e_regression.rs` | Ingest/extract/query cycles |
| Reconstruction | `reconstruction_guarantee.rs` | 100% fidelity verification |
| Ternary Math | `exhaustive_trit_tests.rs` | All trit operations |
| Packed Equivalence | `bt_phase1_packed_equivalence.rs` | SparseVec ↔ Packed |
| Hierarchical | `hierarchical_artifacts_e2e.rs` | Multi-level engrams |
| Property-Based | `properties.rs` | Proptest fuzzing |

### 8.2 Verified Guarantees

1. **Bit-Perfect Reconstruction:** Tests verify exact byte equality after round-trip
2. **Algebraic Closure:** Bundle/bind preserve ternary domain
3. **Cosine Bounds:** Results in [-1, 1]
4. **Permute Invertibility:** `permute(n).inverse_permute(n) = identity`

---

## 9. Known Limitations

### 9.1 Dimension Constraints

- **Fixed at 10,000:** No dynamic scaling based on data characteristics
- **No entropy-based adjustment:** All data gets same dimensionality
- **Memory inefficient for small data:** 10K dimensions for single-byte input

### 9.2 Encoding Limitations

- **Lossy multi-block decode:** Relies on corrections for complex data
- **No CDC:** Fixed 4KB chunks, not content-aware boundaries
- **No quantization:** Dense data (images) doesn't compress well

### 9.3 SIMD Gaps

- **Sparse intersection:** Not SIMD-friendly (irregular access patterns)
- **Dense operations:** PackedTritVec is word-wise but not vectorized
- **No AVX-512:** Only AVX2/NEON stubs

### 9.4 Missing Features (vs refactor.md)

- ❌ 27-trit integers in 64-bit registers
- ❌ Dynamic dimensionality (100K-10M)
- ❌ Quantization + outlier sidecar
- ❌ Register-resident operations
- ❌ Selective unfold via Merkle-DAG
- ❌ NTT for exact bind/unbind
- ❌ GPU acceleration

---

## 10. Architecture Strengths

### 10.1 Design Quality

1. **Clean Separation:** VSA core decoupled from filesystem layer
2. **Correction Guarantee:** Mathematically sound 100% reconstruction
3. **Algebraic Rigor:** Trit operations properly tested
4. **Feature Gates:** Progressive migration without breaking changes
5. **Serialization:** Envelope format supports versioning + compression

### 10.2 Performance Wins

1. **Sparse Representation:** O(k) where k << DIM for most operations
2. **Sorted Indices:** Binary search O(log n) for membership tests
3. **Packed Fast-Path:** Auto-switches to dense when beneficial
4. **LRU Caches:** Sub-engram and index caching in hierarchical queries

### 10.3 Extensibility

1. **Trait Abstractions:** `SubEngramStore`, `CandidateGenerator`
2. **Configurable Params:** `ReversibleVSAConfig`, `HierarchicalQueryBounds`
3. **Multiple Retrievers:** Inverted index vs signature index

---

## 11. Code Quality Metrics

### 11.1 Lines of Code (Core)

| Module | Lines | Complexity |
|--------|-------|------------|
| vsa/vsa.rs | 1,070 | Medium |
| vsa/ternary.rs | 789 | Low |
| vsa/ternary_vec.rs | 399 | Low |
| vsa/dimensional.rs | 970 | Medium |
| core/codebook.rs | 701 | Medium |
| core/correction.rs | 531 | Low |
| fs/embrfs.rs | 1,567 | High |
| cli/mod.rs | 889 | Low |
| **Total Core** | **~7,000** | — |

### 11.2 Documentation

- **Module-level docs:** ✅ All modules have `//!` headers
- **Function docs:** ✅ Most public functions documented
- **Examples:** ✅ Doc tests for core operations
- **Architecture docs:** ✅ docs/adr/*.md decision records

---

## 12. Conclusion

Embeddenator has a **solid foundational architecture** for VSA-based holographic storage. The current implementation correctly handles the core operations with mathematically proven guarantees.

**Key Gaps vs. refactor.md Vision:**
1. Fixed 10K dimensions vs. dynamic 100K-10M
2. `usize` index lists vs. 27-trit packed integers
3. No quantization layer for dense data
4. No register-resident / cache-aware tiering
5. No NTT for algebraic invertibility
6. No GPU acceleration

The codebase is well-positioned for incremental improvement via the `bt-phase-*` feature gates. The next logical steps involve implementing the 27-trit integer type and dynamic dimension selection before approaching compression and encryption layers.

---

*Generated by Embeddenator Documentation Writer Agent*
