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

## References

- Vector Symbolic Architectures: A New Building Block for Artificial General Intelligence (Kleyko et al.)
- Sparse Distributed Memory (Kanerva, 1988)
- Embeddenator README.md - Core Concepts section
- src/vsa.rs - Implementation details
