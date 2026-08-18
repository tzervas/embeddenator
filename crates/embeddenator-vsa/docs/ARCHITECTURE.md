# Embeddenator-VSA Architecture

**Date**: 2026-01-14
**Version**: 0.21.0-beta.1

---

## Overview

Embeddenator-VSA is a Vector Symbolic Architecture (VSA) library implementing sparse and dense balanced ternary vector operations for holographic computing. VSAs are computational models that represent and manipulate symbols using high-dimensional vectors.

### What is VSA?

Vector Symbolic Architectures encode symbolic information into high-dimensional vectors (typically 10,000+ dimensions) using three core operations:

1. **Bundle (⊕)**: Superposition - combines multiple items into a set-like representation
2. **Bind (⊙)**: Composition - creates associations between items
3. **Similarity**: Measures relatedness between vectors (cosine similarity)

### Why Balanced Ternary?

This library uses balanced ternary representation: {-1, 0, +1}

**Advantages**:
- Simple multiplication (sign multiplication)
- Self-inverse property: `A ⊙ A ≈ Identity`
- Efficient sparse representation
- Ideal for bundling operations (majority voting)

---

## Core Abstractions

### 1. SparseVec (Primary)

Sparse ternary vector representation storing only non-zero indices.

**Structure**:
```rust
pub struct SparseVec {
    pub pos: Vec<usize>,  // Indices with +1 value
    pub neg: Vec<usize>,  // Indices with -1 value
}
```

**Invariants**:
- `pos` and `neg` are always sorted (for efficient merge operations)
- No index appears in both `pos` and `neg`
- All indices are in range `[0, DIM)` where `DIM = 10,000`

**When to Use**:
- Sparse data (< 25% density)
- Memory-constrained environments
- Incremental vector construction

**Complexity**:
- Bundle: O(n) where n = total non-zero elements
- Bind: O(n log n) due to merge operations
- Cosine: O(k log k) where k = smaller vector's non-zero count

### 2. PackedTritVec (Dense)

Bitsliced balanced ternary representation for dense vectors.

**Structure**:
```rust
pub struct PackedTritVec {
    data: Vec<u64>,  // 32 trits per u64 word
    len: usize,      // Number of trits
}
```

**Encoding**: 2 bits per trit
- `00` = Zero (0)
- `01` = Positive (+1)
- `10` = Negative (-1)
- `11` = Reserved (treated as Zero)

**When to Use**:
- Dense vectors (≥ 25% density)
- Bulk operations (bundle, bind, dot product)
- SIMD/GPU acceleration targets

**Complexity**:
- All operations: O(n/32) where n = vector dimension
- Uses hardware popcount for counting

### 3. Codebook

Dictionary for differential encoding using VSA basis vectors.

**Structure**:
```rust
pub struct Codebook {
    pub dimensionality: usize,
    pub basis_vectors: Vec<BasisVector>,
    pub semantic_markers: Vec<SparseVec>,
}
```

**Purpose**:
- Project data onto learned basis vectors
- Store residuals for reconstruction
- Detect semantic outliers (high-entropy regions)

**Security Model**:
- Codebook acts as a "private key"
- Without the codebook, reconstruction is impossible
- Different codebooks = different "encryption keys"

---

## Operations

### Bundle Operation (⊕)

**Semantics**: Creates a superposition of multiple vectors, similar to a set.

**Properties**:
- Commutative: `A ⊕ B = B ⊕ A`
- Approximately associative (with thinning)
- Result is similar to all inputs

**Use Cases**:
- Combining related concepts
- Creating set representations
- Superimposing multiple memories

**Implementation**:
```rust
// Pairwise bundle with cancellation
let bundled = vec_a.bundle(&vec_b);

// Associative bundle for multiple vectors
let bundled = SparseVec::bundle_sum_many(&vectors);
```

### Bind Operation (⊙)

**Semantics**: Creates an association between two vectors.

**Properties**:
- Self-inverse: `A ⊙ A ≈ Identity`
- Element-wise multiplication
- Result is dissimilar to both inputs

**Use Cases**:
- Role-filler binding (e.g., "color: red")
- Sequence encoding (with permutation)
- Querying associations

**Implementation**:
```rust
// Create association
let role_filler = role.bind(&filler);

// Query (uses self-inverse property)
let recovered = role_filler.bind(&role);  // ≈ filler
```

### Permutation Operation

**Semantics**: Cyclic rotation of vector indices.

**Properties**:
- Invertible: `inverse_permute(permute(A, n), n) = A`
- Used for sequence encoding

**Use Cases**:
- Encoding position in sequences
- Creating ordered structures

**Implementation**:
```rust
let permuted = vec.permute(100);
let original = permuted.inverse_permute(100);
```

### Similarity (Cosine)

**Semantics**: Measures the angle between two vectors.

**Range**: [-1, 1] where:
- 1.0 = Identical vectors
- 0.0 = Orthogonal vectors
- -1.0 = Opposite vectors

**Implementation**:
```rust
let similarity = vec_a.cosine(&vec_b);
```

---

## Data Flow

### Encoding

```
Raw Data → encode_data() → SparseVec
                            |
                            ↓ (if dense)
                         PackedTritVec
```

### Decoding

```
SparseVec → decode_data() → Reconstructed Data
    |
    ↓ (with codebook)
ProjectionResult → residuals → Perfect Reconstruction
```

### Codebook Projection

```
Data → project() → ProjectionResult {
                     coefficients,    // Basis vector weights
                     residual,        // Reconstruction error
                     outliers,        // High-entropy regions
                     quality_score    // [0, 1] coverage metric
                   }
```

---

## Module Structure

```
src/
├── lib.rs           # Public API exports
├── vsa.rs           # SparseVec implementation
├── ternary_vec.rs   # PackedTritVec implementation
├── codebook.rs      # Differential encoding
├── simd_cosine.rs   # SIMD-accelerated similarity
├── ternary.rs       # Low-level ternary arithmetic
└── dimensional.rs   # Multi-dimensional ternary types
```

---

## Design Decisions

### Decision 1: Sparse vs Dense Representation

**Context**: Ternary vectors can be sparse (few non-zeros) or dense (many non-zeros).

**Decision**: Adaptive strategy with automatic switching at 25% density.

**Rationale**:
- Sparse: 8 bytes per non-zero element
- Dense: 2 bits per element (regardless of value)
- Crossover point is mathematically ~25%

**Consequences**:
- Need conversion functions between representations
- Operations check density and may convert internally
- Some overhead for density checking

### Decision 2: Sorted Indices for SparseVec

**Context**: SparseVec stores indices in `pos` and `neg` arrays.

**Decision**: Always keep indices sorted.

**Rationale**:
- Enables O(n) merge operations for bundle/bind
- Binary search for membership tests O(log n)
- Merge join for set operations

**Consequences**:
- Insertion requires maintaining sort (O(log n) binary search)
- Bulk construction can sort once at the end

### Decision 3: Scalar Fallback for SIMD

**Context**: SIMD implementations are complex and platform-specific.

**Decision**: SIMD infrastructure with scalar fallbacks.

**Rationale**:
- Correctness guaranteed on all platforms
- Auto-vectorization provides good performance
- Explicit SIMD can be added incrementally

**Consequences**:
- `simd` feature flag exists but uses scalar currently
- Future explicit SIMD can be added without API changes

### Decision 4: Multi-Block Decoding Deferred

**Context**: Decoding hierarchically bundled multi-block data requires VSA factorization.

**Decision**: Simplified linear decoding for multi-block; defer full factorization.

**Rationale**:
- Single-block works correctly
- Residual mechanism handles reconstruction
- Full factorization is research-intensive

**Consequences**:
- Multi-block (> block_size) may have reduced fidelity
- Full hierarchical decoding planned for v1.1.0

---

## Performance Characteristics

### Big-O Analysis

| Operation | SparseVec | PackedTritVec |
|-----------|-----------|---------------|
| Bundle    | O(n)      | O(n/32)       |
| Bind      | O(n log n)| O(n/32)       |
| Cosine    | O(k log k)| O(n/32)       |
| Permute   | O(k)      | O(n)          |

Where n = vector dimension (10,000), k = non-zero count

### Memory Usage

| Representation | Memory Formula | Example (10,000 dim) |
|----------------|----------------|---------------------|
| SparseVec @ 5% | 8 × k × 2      | ~8 KB              |
| SparseVec @ 25%| 8 × k × 2      | ~40 KB             |
| PackedTritVec  | n / 32 × 8     | ~2.5 KB            |

### Benchmarks

Current performance (single-threaded, scalar):
- Bundle (sparse): ~0.1ms for 10,000 dim
- Cosine (sparse): ~0.05ms for 10,000 dim
- Encode 1KB data: ~0.5ms

---

## Future Roadmap

### v0.21.0 (Current)
-  Core VSA operations
-  Bitsliced PackedTritVec
-  Codebook reconstruction
-  Quality scoring

### v0.22.0
- Explicit SIMD (AVX2, NEON)
- ARM64 platform support
- Comprehensive benchmarks

### v1.0.0
- Documentation complete
- API stability guarantee
- Production optimizations

### v1.1.0
- GPU acceleration (CUDA, OpenCL)
- Full hierarchical decoding
- Batch operations

---

## References

### Internal Documentation
- [Bitsliced Ternary Design](./BITSLICED_TERNARY_DESIGN.md)
- [SIMD Design](./SIMD_DESIGN.md)
- [Ternary Representation Guide](./TERNARY_REPRESENTATION_GUIDE.md)

### External References
- Kanerva, P. (2009). "Hyperdimensional Computing: An Introduction"
- Plate, T. (2003). "Holographic Reduced Representations"
- Knuth, D. (1998). "The Art of Computer Programming, Vol 2" (Balanced Ternary)

---

**Document Version**: 1.0
**Last Updated**: 2026-01-14
