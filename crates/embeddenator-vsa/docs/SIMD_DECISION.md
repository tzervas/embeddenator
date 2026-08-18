# SIMD Implementation Decision

**Date**: 2026-01-14
**Version**: 0.21.0-beta.1

## Decision

**CHOSEN OPTION: Keep SIMD infrastructure with scalar fallbacks (Status Quo)**

## Rationale

After reviewing the codebase and implementation plan, we have decided to maintain the current SIMD implementation approach rather than removing it entirely:

1. **Correct Behavior**: The current SIMD functions (AVX2, NEON) correctly fall back to optimized scalar implementations. This provides correct results on all platforms.

2. **Infrastructure Ready**: The SIMD infrastructure is in place for future optimization:
   - Feature flags configured (`simd` in Cargo.toml)
   - Platform detection working (`target_arch`, `target_feature`)
   - Clear code organization in `simd_cosine.rs`

3. **No User Impact**: Users enabling the `simd` feature get correct results via scalar fallback. There's no broken functionality.

4. **Preferred Path**: The bitsliced `PackedTritVec` representation (see `ternary_vec.rs`) is the preferred approach for SIMD acceleration as it provides word-level parallelism (32 trits per u64 operation).

## Current Implementation

### What Works
- `cosine_simd()` - Dispatches to platform-specific or scalar implementation
- `cosine_scalar()` - Optimized scalar baseline (always available)
- `cosine_avx2()` - Infrastructure ready, falls back to scalar
- `cosine_neon()` - Infrastructure ready, falls back to scalar

### Why Scalar is Sufficient for Sparse Vectors

For `SparseVec` (sparse ternary vectors), SIMD optimization is challenging because:
- Sparse intersection counting requires sorted merge algorithms
- Variable-length index lists don't vectorize naturally
- Memory access patterns are irregular

The scalar `intersection_count_sorted()` function is already highly optimized for this use case.

### Dense Vector Path (Preferred for SIMD)

For `PackedTritVec` (dense bitsliced vectors):
- SIMD is highly effective due to contiguous memory layout
- Word-level operations process 32 trits per operation
- Hardware popcount acceleration available
- This path is used automatically when vector density exceeds threshold

## Performance Characteristics

Current benchmark results (approximate):
- **Sparse cosine similarity**: O(k log k) where k = non-zero elements
- **Dense cosine similarity (PackedTritVec)**: O(n/32) with compiler auto-vectorization
- **Threshold**: PackedTritVec activated when density > 25%

## Future Work (v1.1.0+)

See `IMPLEMENTATION_PLAN.md` Phase 5 for GPU acceleration plans:

1. **Explicit SIMD for PackedTritVec** (Phase 2)
   - AVX2/AVX-512 implementations for x86_64
   - NEON implementations for ARM64
   - Expected 2-4x speedup over auto-vectorized code

2. **GPU Acceleration** (Phase 5)
   - CUDA kernels for large-scale VSA workloads
   - OpenCL for cross-platform GPU support
   - Expected 10-100x speedup for vectors > 10M dimensions

## References

- `src/simd_cosine.rs` - Current SIMD implementation
- `src/ternary_vec.rs` - PackedTritVec (bitsliced) implementation
- `docs/BITSLICED_TERNARY_DESIGN.md` - Design details
- `IMPLEMENTATION_PLAN.md` - Full roadmap

---

**Document Version**: 1.0
**Last Updated**: 2026-01-14
