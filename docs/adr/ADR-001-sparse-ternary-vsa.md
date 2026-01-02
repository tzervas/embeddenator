# ADR-001: Choice of Sparse Ternary Vector Symbolic Architecture

## Status

Accepted

## Date

2025-12-15

## Context

The Embeddenator project required a method for holographic encoding of filesystem data that could:
- Represent complex hierarchical structures in a single root state
- Enable algebraic operations on encoded data without decoding
- Provide bit-perfect reconstruction capabilities
- Scale efficiently to handle large datasets (TB-scale)
- Support compositional operations (bundling, binding, permutation)

Traditional approaches like dense vector representations or tree-based encodings had limitations:
- Dense vectors consume excessive memory for high-dimensional spaces
- Tree structures don't support algebraic composition naturally
- Conventional compression loses the ability to perform operations on compressed data

## Decision

We chose to implement a Sparse Ternary Vector Symbolic Architecture (VSA) as the core encoding mechanism with the following characteristics:

- **Vectors**: 10,000-dimensional sparse ternary vectors {-1, 0, +1}
- **Sparsity**: Approximately 1% density (100 non-zero elements)
- **Operations**:
  - Bundle (⊕): Element-wise summation for superposition
  - Bind (⊙): Element-wise multiplication for composition
  - Scalar multiplication: For weighted contributions
- **Cleanup**: Cosine similarity-based matching (threshold >0.75 for correct matches, <0.3 for noise)

The implementation uses Rust's `HashMap<usize, i8>` to efficiently store only non-zero elements.

## Consequences

## Update (2026-01-01)

The project now supports multiple bundling semantics:

- A fast pairwise conflict-cancel bundle (`SparseVec::bundle`) which is commutative but generally **not associative** across 3+ vectors.
- An explicit associative multiway bundle (`SparseVec::bundle_sum_many`) for order-independent aggregation.

See ADR-008 for the current bundling semantics and future cost-aware hybrid notes.

### Positive

- **Memory Efficiency**: Sparse storage dramatically reduces memory footprint
  - Only ~0.4-1KB per 10K dimensional vector vs 40KB for dense storage
  - Enables handling of millions of chunks in reasonable memory

- **Algebraic Properties**: Natural support for compositional operations
  - Associative bundle: (A ⊕ B) ⊕ C ≈ A ⊕ (B ⊕ C)
  - Self-inverse bind: A ⊙ A ≈ I (identity)
  - Enables post-encoding modifications without full reconstruction

- **Scalability**: Hierarchical chunking enables TB-scale datasets
  - 4KB chunks provide optimal granularity
  - Multi-level encoding (file → directory → root)

- **Bit-Perfect Reconstruction**: Codebook maintains exact original data
  - 100% ordered text reconstruction guaranteed
  - Binary files recovered exactly

### Negative

- **Approximate Matching**: Cosine similarity is probabilistic
  - Rare collisions possible (mitigated by 10K dimensions)
  - Requires cleanup thresholds tuning

- **Computational Overhead**: Sparse operations have cost
  - Slower than dense vector operations for very dense vectors
  - Acceptable trade-off given typical 1% density

- **Learning Curve**: VSA concepts less familiar than traditional encodings
  - Requires understanding of holographic/distributed representations
  - Documentation and examples critical for adoption

### Neutral

- **Rust Implementation**: Performance benefits come with language constraints
  - Excellent for production use
  - May limit contributor pool vs Python/JavaScript

## Hardware Optimization Considerations

### Balanced Ternary Representation

The choice of sparse ternary vectors {-1, 0, +1} aligns naturally with **balanced ternary** arithmetic, which offers significant hardware optimization opportunities:

#### Ternary Mathematics
- **Trits** (ternary digits): {-1, 0, +1} representing three states
- **Trytes** (ternary bytes): Groups of trits optimized for binary hardware
- **Balanced ternary**: Uses symmetric -1/0/+1 representation (vs unbalanced 0/1/2)

#### 64-Bit Register Optimization

Contemporary 64-bit CPUs can efficiently encode ternary data without requiring SIMD extensions:

```
Optimal encoding for balanced ternary: 39-40 trits per 64-bit register

Mathematical basis:
  For signed balanced ternary (recommended):
    3^39 = 4,052,555,153,018,976,267 < 2^63
    Range: -(3^39-1)/2 to +(3^39-1)/2
    Bits used: 61.8
  
  For unsigned ternary:
    3^40 = 12,157,665,459,056,928,801 < 2^64
    Range: 0 to 3^40-1
    Bits used: 63.4
  
This means:
  - 39 trits optimal for signed balanced ternary {-1, 0, +1}
  - 40 trits optimal for unsigned representation
  - Each trit encodes log₂(3) ≈ 1.585 bits
  - No wasted register capacity (>60 bits utilized)
  - No overflow risk with proper trit count
```

**Benefits**:
- Works on any 64-bit CPU (x86-64, ARM64, RISC-V) without extensions
- No AVX, AVX2, or AVX-512 required for basic operations
- Can leverage SIMD when available for 2-8× acceleration
- Scalar fallback always available
- Optimal information density for contemporary hardware

#### Compact Hologram Encoding

For sparse vectors with ~1% density (200 non-zero elements out of 10,000):
- Traditional: 200 indices × 8 bytes = 1,600 bytes
- Balanced ternary: 200 trits / 39 trits per register ≈ 5.1 registers × 8 bytes ≈ 41 bytes
- **Compression ratio: ~39×**

This compact representation enables:
- Hash-like storage format
- Two-way encoding (encode/decode without loss)
- Efficient network transfer
- Reduced memory footprint
- Algebraic operations on encoded form

See [ADR-005](ADR-005-hologram-package-isolation.md) for detailed implementation of balanced ternary encoding and hologram package isolation.

## References

- Vector Symbolic Architectures: A New Building Block for Artificial General Intelligence (Kleyko et al.)
- Sparse Distributed Memory (Kanerva, 1988)
- Embeddenator README.md - Core Concepts section
- src/vsa.rs - Implementation details
- [Balanced Ternary](https://en.wikipedia.org/wiki/Balanced_ternary)
- ADR-005: Hologram-Based Package Isolation (balanced ternary implementation)
