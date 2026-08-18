# SIMD Design and Implementation Status

**Date**: 2026-01-14  
**Version**: 0.20.0-alpha.1  
**Status**: Scalar with Auto-Vectorization

---

## Executive Summary

This document describes the SIMD acceleration strategy for embeddenator-vsa, the current implementation status, and the roadmap for explicit SIMD optimizations.

---

## Current Implementation

### Status: Scalar with Compiler Auto-Vectorization

**What we have**:
-  Bitsliced `PackedTritVec` representation (word-level operations)
-  Scalar implementations that auto-vectorize well
-  Infrastructure for AVX2 and NEON (feature-gated)
-  Comprehensive test coverage

**What we don't have yet**:
-  Hand-written SIMD intrinsics (AVX2/AVX-512/NEON)
-  Runtime CPU feature detection
-  SIMD-specific benchmarks
-  GPU implementations

### Why Scalar First?

**Decision Rationale** (2026-01-14):

1. **Correctness over Speed**: Ensure mathematical correctness before optimization
2. **Bitsliced Foundation**: Proper representation design is more important than premature SIMD
3. **Compiler Auto-Vectorization**: Modern compilers (LLVM 16+) do excellent auto-vectorization
4. **Maintenance Burden**: Hand-written SIMD requires extensive testing and platform support
5. **Future-Proof**: Bitsliced design enables easy SIMD/GPU implementation later

---

## Architecture: Two-Level Approach

### Level 1: SparseVec (Sparse Representation)

**Current Implementation**: Sorted index lists
```rust
pub struct SparseVec {
    pub pos: Vec<usize>,  // Positive trit indices
    pub neg: Vec<usize>,  // Negative trit indices
}
```

**SIMD Challenges**:
- Sparse intersection requires sorted merge (irregular memory access)
- Variable-length lists don't vectorize naturally
- Branch-heavy algorithms (comparison-based merging)

**SIMD Strategy**: 
-  **Not pursued** for sparse operations (low ROI)
-  Use scalar merge algorithms (already optimal)
-  Consider converting to PackedTritVec for dense operations

**Current Performance**: 
- O(k log k) where k = non-zero count
- Excellent for sparse vectors (< 25% density)

---

### Level 2: PackedTritVec (Bitsliced Dense Representation)

**Current Implementation**: Bitsliced 2-bits-per-trit
```rust
pub struct PackedTritVec {
    len: usize,
    data: Vec<u64>,  // 32 trits per word
}
```

**SIMD Advantages**:
-  Contiguous memory (coalesced access)
-  Word-level parallelism (32 trits per u64)
-  Branchless operations (pure bitwise)
-  Hardware POPCNT acceleration

**SIMD Strategy**:
-  **Primary target** for SIMD optimization
-  Bitsliced design enables natural vectorization
-  Clear path to AVX2/AVX-512/NEON/GPU

**Current Performance**:
- O(n/32) where n = dimension
- Auto-vectorized by compiler (LLVM)
- 32x theoretical speedup vs naive scalar

---

## SIMD Implementation Plan

### Phase 1: Scalar + Auto-Vectorization (Current)

**Status**:  **COMPLETE**

**Implementation**:
```rust
// PackedTritVec::dot() - auto-vectorizes well
pub fn dot(&self, other: &Self) -> i32 {
    let mut acc: i32 = 0;
    for w in 0..words {
        let a = self.data[w];
        let b = other.data[w];
        
        // Bitplane separation
        let a_pos = a & MASK_EVEN_BITS;
        let a_neg = (a >> 1) & MASK_EVEN_BITS;
        let b_pos = b & MASK_EVEN_BITS;
        let b_neg = (b >> 1) & MASK_EVEN_BITS;
        
        // POPCNT on bitwise operations
        let pp = (a_pos & b_pos).count_ones() as i32;
        let nn = (a_neg & b_neg).count_ones() as i32;
        let pn = (a_pos & b_neg).count_ones() as i32;
        let np = (a_neg & b_pos).count_ones() as i32;
        
        acc += (pp + nn) - (pn + np);
    }
    acc
}
```

**Compiler Auto-Vectorization**:
- LLVM recognizes bitwise + POPCNT pattern
- Generates SSE4.2/AVX2 code automatically
- No manual intrinsics needed

**Performance**: 
- 2-4x faster than naive scalar
- Good enough for v1.0.0 release

---

### Phase 2: Explicit SIMD (v0.21.0 - Future)

**Status**:  **PLANNED**

**Target Platforms**:
1. **x86_64**: AVX2, AVX-512
2. **ARM64**: NEON, SVE (if available)
3. **WASM**: SIMD128 (for web deployment)

**Implementation Strategy**:

#### AVX2 (x86_64)

**Approach**: Process 4 × u64 = 128 trits per iteration

```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn dot_avx2(data_a: &[u64], data_b: &[u64]) -> i32 {
    use std::arch::x86_64::*;
    
    let mask = _mm256_set1_epi64x(0x5555_5555_5555_5555u64 as i64);
    let mut acc_pp = _mm256_setzero_si256();
    let mut acc_nn = _mm256_setzero_si256();
    let mut acc_pn = _mm256_setzero_si256();
    let mut acc_np = _mm256_setzero_si256();
    
    for i in (0..data_a.len()).step_by(4) {
        // Load 4 × u64
        let va = _mm256_loadu_si256(data_a[i..].as_ptr() as *const __m256i);
        let vb = _mm256_loadu_si256(data_b[i..].as_ptr() as *const __m256i);
        
        // Separate bitplanes
        let a_pos = _mm256_and_si256(va, mask);
        let a_neg = _mm256_and_si256(_mm256_srli_epi64(va, 1), mask);
        let b_pos = _mm256_and_si256(vb, mask);
        let b_neg = _mm256_and_si256(_mm256_srli_epi64(vb, 1), mask);
        
        // Compute intersections
        let pp = _mm256_and_si256(a_pos, b_pos);
        let nn = _mm256_and_si256(a_neg, b_neg);
        let pn = _mm256_and_si256(a_pos, b_neg);
        let np = _mm256_and_si256(a_neg, b_pos);
        
        // POPCNT (AVX-512) or horizontal accumulation
        // ... accumulate into acc_* ...
    }
    
    // Horizontal reduction
    // ... reduce 4 × 64-bit counts to single i32 ...
}
```

**Expected Speedup**: 2-4x over auto-vectorized scalar

**Challenges**:
- POPCNT in AVX2 requires emulation (AVX-512 has native `_mm256_popcnt_epi64`)
- Horizontal reduction overhead
- Need fallback for CPUs without AVX2

#### NEON (ARM64)

**Approach**: Process 2 × u64 = 64 trits per iteration

```rust
#[cfg(target_arch = "aarch64")]
unsafe fn dot_neon(data_a: &[u64], data_b: &[u64]) -> i32 {
    use std::arch::aarch64::*;
    
    let mask = vdupq_n_u64(0x5555_5555_5555_5555u64);
    let mut acc: i32 = 0;
    
    for i in (0..data_a.len()).step_by(2) {
        // Load 2 × u64
        let va = vld1q_u64(data_a[i..].as_ptr());
        let vb = vld1q_u64(data_b[i..].as_ptr());
        
        // Separate bitplanes
        let a_pos = vandq_u64(va, mask);
        let a_neg = vandq_u64(vshrq_n_u64(va, 1), mask);
        let b_pos = vandq_u64(vb, mask);
        let b_neg = vandq_u64(vshrq_n_u64(vb, 1), mask);
        
        // Compute intersections
        let pp = vandq_u64(a_pos, b_pos);
        let nn = vandq_u64(a_neg, b_neg);
        let pn = vandq_u64(a_pos, b_neg);
        let np = vandq_u64(a_neg, b_pos);
        
        // POPCNT (ARM has vcntq_u8 for bytes, need reduction)
        // ... accumulate counts ...
    }
    
    acc
}
```

**Expected Speedup**: 1.5-3x over auto-vectorized scalar

**Challenges**:
- NEON POPCNT operates on bytes (need to aggregate)
- Limited register count compared to AVX2

---

### Phase 3: GPU Acceleration (v0.22.0 - Future)

**Status**:  **PLANNED**

See `IMPLEMENTATION_PLAN.md` Phase 5 and `docs/BITSLICED_TERNARY_DESIGN.md` for details.

**Key Points**:
- CUDA for NVIDIA GPUs
- OpenCL for cross-platform
- Vulkan compute shaders (optional)
- Expected: 10-100x speedup for large vectors

---

## Runtime Selection Strategy

### Adaptive Execution (Future)

```rust
pub enum SimdLevel {
    Scalar,       // Baseline
    Sse42,        // SSE 4.2 (POPCNT)
    Avx2,         // AVX2 (256-bit)
    Avx512,       // AVX-512 (512-bit + native POPCNT)
    Neon,         // ARM NEON
    Sve,          // ARM SVE (scalable vectors)
    Gpu,          // GPU acceleration
}

pub struct SimdConfig {
    pub level: SimdLevel,
    pub threshold_for_gpu: usize,  // Min vector size for GPU
}

impl Default for SimdConfig {
    fn default() -> Self {
        Self {
            level: Self::detect_best(),
            threshold_for_gpu: 1_000_000,
        }
    }
}

impl SimdConfig {
    pub fn detect_best() -> SimdLevel {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx512f") {
                return SimdLevel::Avx512;
            }
            if is_x86_feature_detected!("avx2") {
                return SimdLevel::Avx2;
            }
            if is_x86_feature_detected!("sse4.2") {
                return SimdLevel::Sse42;
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            // NEON always available on aarch64
            return SimdLevel::Neon;
        }
        
        SimdLevel::Scalar
    }
}
```

---

## Testing Strategy

### Correctness Tests

**Requirements**:
1. **Bit-for-bit accuracy**: SIMD results must match scalar
2. **All platforms**: Test on x86_64, ARM64, WASM
3. **All feature levels**: SSE4.2, AVX2, AVX-512, NEON
4. **Edge cases**: Empty, single element, misaligned, etc.

**Test Framework**:
```rust
#[test]
fn test_simd_correctness() {
    let test_vectors = generate_test_cases();
    
    for (a, b) in test_vectors {
        let scalar_result = dot_scalar(a, b);
        let simd_result = dot_simd(a, b);
        
        assert_eq!(
            scalar_result, simd_result,
            "SIMD result differs from scalar"
        );
    }
}
```

### Performance Benchmarks

**Metrics**:
1. **Throughput**: Operations per second
2. **Latency**: Time per operation
3. **Speedup**: SIMD vs scalar ratio
4. **Efficiency**: % of theoretical peak

**Benchmark Suite**:
```rust
fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product");
    
    for size in [1_000, 10_000, 100_000, 1_000_000] {
        group.bench_with_input(
            BenchmarkId::new("scalar", size),
            &size,
            |b, &size| {
                let a = PackedTritVec::random(size);
                let b = PackedTritVec::random(size);
                b.iter(|| a.dot_scalar(&b));
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", size),
            &size,
            |b, &size| {
                let a = PackedTritVec::random(size);
                let b = PackedTritVec::random(size);
                b.iter(|| a.dot_simd(&b));
            }
        );
    }
}
```

---

## Performance Expectations

### Theoretical Analysis

**Current (Auto-Vectorized)**:
- Compiler LLVM 16+
- Auto-vectorizes bitwise + POPCNT
- Estimated: 2-4x speedup

**Phase 2 (Explicit SIMD)**:
- Hand-written AVX2/AVX-512
- Better register allocation
- Reduced memory traffic
- Estimated: 4-8x speedup

**Phase 3 (GPU)**:
- Massive parallelism (1000s of cores)
- Batch processing efficiency
- Estimated: 10-100x speedup

### Real-World Expectations

**Vector Size Impact**:
- Small (< 10K): Scalar best (overhead dominates)
- Medium (10K - 1M): SIMD wins (2-4x)
- Large (> 1M): GPU wins (10-100x)

**Density Impact**:
- Sparse (< 25%): Use SparseVec (no SIMD)
- Dense (≥ 25%): Use PackedTritVec (SIMD-friendly)

---

## Conclusion

### Current State (v0.20.0-alpha.1)

 **Bitsliced foundation** - Proper representation design  
 **Scalar implementation** - Correct and well-tested  
 **Auto-vectorization** - Compiler does good job  
 **Infrastructure** - Feature gates and scaffolding ready  

### Future Path

 **Phase 2** (v0.21.0): Explicit SIMD (2-4x speedup)  
 **Phase 3** (v0.22.0): GPU acceleration (10-100x speedup)  
 **Phase 4** (v1.0.0): Production optimization  

### Design Decision

**Prioritize correctness and good design over premature optimization.**

The bitsliced `PackedTritVec` design enables future SIMD and GPU acceleration without compromising the current implementation's correctness or maintainability.

---

**Document Version**: 1.0  
**Last Updated**: 2026-01-14  
**Next Review**: Before Phase 2 SIMD implementation
