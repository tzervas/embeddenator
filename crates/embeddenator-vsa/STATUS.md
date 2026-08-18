# embeddenator-vsa Status

**Version:** 0.21.0
**crates.io:** [embeddenator-vsa](https://crates.io/crates/embeddenator-vsa)
**Last Updated:** 2026-01-26

---

## Overview

Core Vector Symbolic Architecture (VSA) implementation with sparse ternary vector representation.

---

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `simd` | Yes | SIMD optimizations (AVX2/NEON) |
| `block-sparse` | No | Block-sparse representation |
| `cuda` | No | GPU acceleration via cudarc |

---

## Test Coverage

- **Unit Tests:** 35
- **Benchmarks:** 2 (vsa_ops, simd_cosine)

---

## Key Components

| Component | Status | Description |
|-----------|--------|-------------|
| SparseVec | Production | Sparse ternary vector implementation |
| PackedTritVec | Production | Dense bitsliced representation |
| Codebook | Production | Vector codebook management |
| SIMD Cosine | Production | Optimized similarity computation |

---

## Performance

- Bitsliced: 32 trits per u64 word
- SIMD: 2-4x speedup on supported hardware
- GPU: Optional CUDA acceleration

---

## Remaining Tasks

- [ ] Explicit SIMD intrinsics for ARM NEON
- [ ] GPU kernel optimization
- [ ] Block-sparse production hardening

---

## Links

- **Documentation:** https://docs.rs/embeddenator-vsa
- **Repository:** https://github.com/tzervas/embeddenator-vsa
