# Bitsliced Balanced Ternary Design

**Date**: 2026-01-14  
**Version**: 0.20.0-alpha.1  
**Status**: Implementation Complete

---

## Executive Summary

This document describes the bitsliced balanced ternary representation used in `PackedTritVec`, which is the preferred implementation for dense ternary vectors in embeddenator-vsa. This approach enables SIMD-friendly operations and efficient vectorization.

---

## Balanced Ternary Overview

Balanced ternary uses three values: {-1, 0, +1}, denoted as {N, Z, P}.

### Algebraic Properties

**Multiplication (Bind)**:
- P × P = P, N × N = P (self-inverse)
- P × N = N, N × P = N
- Any × Z = Z

**Addition (Bundle - saturating)**:
- P + P = P, N + N = N
- P + N = Z, N + P = Z
- Any + Z = Any

---

## Representation Comparison

### 1. Packed Trit Representation (NOT USED)

**Encoding**: 3 trits per byte using base-3
- Range: 0-26 per byte (3^3 = 27 states)
- Inefficient: 5.08 bits per byte used (62.5% efficiency)
- Complex arithmetic for operations

**Problems**:
- Not SIMD-friendly
- Requires unpacking for operations
- Complex carry logic for bundling
- Poor cache utilization

### 2. Bitsliced Balanced Ternary (PREFERRED - PackedTritVec)

**Encoding**: 2 bits per trit
```
00 = Z (zero)
01 = P (positive, +1)
10 = N (negative, -1)
11 = unused (reserved, treated as Z)
```

**Storage**:
- 32 trits per u64 word
- 2 bits per trit = 100% bit utilization (no wasted bits vs 3-valued encoding)
- Direct word-level operations

**Key Insight**: By separating the encoding into two bitplanes (even and odd bits), we can perform parallel operations on 32 trits at once using standard 64-bit operations.

---

## Bitslicing Technique

### Bitplane Separation

Each u64 word stores 32 trits in bitsliced form:

```
Word layout (64 bits):
[t31_hi|t31_lo|t30_hi|t30_lo|...|t1_hi|t1_lo|t0_hi|t0_lo]

Even bits (0x5555_5555_5555_5555): P bitplane
  bit_i*2 = 1 if trit_i is P
  
Odd bits  (shifted right 1): N bitplane  
  bit_i*2+1 = 1 if trit_i is N
```

### Extraction

```rust
const MASK_EVEN_BITS: u64 = 0x5555_5555_5555_5555;

let word = data[i];
let pos_plane = word & MASK_EVEN_BITS;           // Extract P trits
let neg_plane = (word >> 1) & MASK_EVEN_BITS;   // Extract N trits
```

---

## SIMD-Optimized Operations

### 1. Dot Product (Cosine Similarity)

**Algorithm**: `dot(a, b) = sum(a_i * b_i)`

**Truth Table**:
- P × P = +1, N × N = +1 (matching signs)
- P × N = -1, N × P = -1 (opposing signs)
- Any × Z = 0 (zero contribution)

**Bitsliced Implementation**:
```rust
pub fn dot(&self, other: &Self) -> i32 {
    let mut acc: i32 = 0;
    for w in 0..words {
        let a = self.data[w];
        let b = other.data[w];
        
        // Separate bitplanes
        let a_pos = a & MASK_EVEN_BITS;
        let a_neg = (a >> 1) & MASK_EVEN_BITS;
        let b_pos = b & MASK_EVEN_BITS;
        let b_neg = (b >> 1) & MASK_EVEN_BITS;
        
        // Count matching and opposing signs
        let pp = (a_pos & b_pos).count_ones() as i32;  // Both positive
        let nn = (a_neg & b_neg).count_ones() as i32;  // Both negative
        let pn = (a_pos & b_neg).count_ones() as i32;  // Opposite signs
        let np = (a_neg & b_pos).count_ones() as i32;  // Opposite signs
        
        // Accumulate: +1 for matching, -1 for opposing
        acc += (pp + nn) - (pn + np);
    }
    acc
}
```

**SIMD Benefits**:
- Processes 32 trits per word (32x speedup potential)
- Uses hardware popcount (`count_ones()`)
- No branches in inner loop
- Excellent vectorization by compiler

### 2. Bind Operation (Elementwise Multiplication)

**Algorithm**: `c[i] = a[i] × b[i]`

**Truth Table**:
- Same sign (PP or NN) → P
- Opposite signs (PN or NP) → N
- Any zero → Z

**Bitsliced Implementation**:
```rust
pub fn bind(&self, other: &Self) -> Self {
    for w in 0..words {
        let a = self.data[w];
        let b = other.data[w];
        
        // Separate bitplanes
        let a_pos = a & MASK_EVEN_BITS;
        let a_neg = (a >> 1) & MASK_EVEN_BITS;
        let b_pos = b & MASK_EVEN_BITS;
        let b_neg = (b >> 1) & MASK_EVEN_BITS;
        
        // Compute result bitplanes
        let same = (a_pos & b_pos) | (a_neg & b_neg);  // Both have same sign → P
        let opp = (a_pos & b_neg) | (a_neg & b_pos);    // Opposite signs → N
        
        // Reconstruct word: P in even bits, N in odd bits
        out.data[w] = same | (opp << 1);
    }
}
```

**SIMD Benefits**:
- 32 multiplications per word
- Pure bitwise operations (AND, OR, shift)
- No conditional logic
- Perfect for SIMD vectorization

### 3. Bundle Operation (Elementwise Saturating Addition)

**Algorithm**: `c[i] = saturate(a[i] + b[i])`

**Truth Table** (saturating to ternary range):
- P + P = P, N + N = N
- P + N = Z, N + P = Z
- P + Z = P, Z + P = P
- N + Z = N, Z + N = N

**Bitsliced Implementation**:
```rust
pub fn bundle(&self, other: &Self) -> Self {
    for w in 0..words {
        let a = self.data[w];
        let b = other.data[w];
        
        // Separate bitplanes
        let a_pos = a & MASK_EVEN_BITS;
        let a_neg = (a >> 1) & MASK_EVEN_BITS;
        let b_pos = b & MASK_EVEN_BITS;
        let b_neg = (b >> 1) & MASK_EVEN_BITS;
        
        // Result is P if: (a is P and b is not N) OR (b is P and a is not N)
        let mask = MASK_EVEN_BITS;
        let pos = (a_pos & (!b_neg & mask)) | (b_pos & (!a_neg & mask));
        
        // Result is N if: (a is N and b is not P) OR (b is N and a is not P)
        let neg = (a_neg & (!b_pos & mask)) | (b_neg & (!a_pos & mask));
        
        out.data[w] = pos | (neg << 1);
    }
}
```

**SIMD Benefits**:
- 32 additions per word
- Saturating behavior via bitwise logic
- No arithmetic operations needed
- Highly vectorizable

---

## Performance Characteristics

### Space Efficiency

**PackedTritVec**:
- 2 bits per trit
- 32 trits per u64
- Overhead: ~16 bytes (len + Vec header)
- Example: 10,000 trits = 2,500 bytes + 16 = ~2.5 KB

**SparseVec** (for comparison):
- 8 bytes per non-zero trit (usize index)
- Example: 10,000 trits at 10% density = 1,000 × 8 = 8 KB

**Crossover Point**: ~25% density
- Below 25%: SparseVec more efficient
- Above 25%: PackedTritVec more efficient

### Time Complexity

| Operation | PackedTritVec | SparseVec | Speedup |
|-----------|---------------|-----------|---------|
| Dot product | O(n/32) | O(k log k) | 32x for dense |
| Bind | O(n/32) | O(k) | 32x for dense |
| Bundle | O(n/32) | O(k) | 32x for dense |
| Get/Set | O(1) | O(log k) | Varies |

Where:
- n = vector dimension
- k = number of non-zero trits

---

## SIMD Vectorization Opportunities

### Current State (Scalar)

The current implementation uses scalar operations that are auto-vectorized by the compiler. On modern CPUs:

**x86_64 with AVX2**:
- 256-bit SIMD registers
- Process 4 × u64 = 128 trits per SIMD instruction
- Theoretical 128x speedup over naive scalar

**ARM64 with NEON**:
- 128-bit SIMD registers
- Process 2 × u64 = 64 trits per SIMD instruction
- Theoretical 64x speedup over naive scalar

### Future SIMD Enhancement

**Explicit SIMD Implementation** (Future Work):

```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn dot_avx2(a: &[u64], b: &[u64]) -> i32 {
    use std::arch::x86_64::*;
    
    let mut acc = _mm256_setzero_si256();
    
    for i in (0..a.len()).step_by(4) {
        // Load 4 × u64 = 256 bits
        let va = _mm256_loadu_si256(a[i..].as_ptr() as *const __m256i);
        let vb = _mm256_loadu_si256(b[i..].as_ptr() as *const __m256i);
        
        // Perform 128 trit multiplications in parallel
        // ... bitplane separation and counting ...
        
        acc = _mm256_add_epi32(acc, result);
    }
    
    // Horizontal sum
    // ... reduce acc to scalar ...
}
```

**Expected Speedup**: 2-4x over current auto-vectorized code

---

## GPU Acceleration (Future Work)

### CUDA/OpenCL Implementation

PackedTritVec is ideal for GPU acceleration:

**Advantages**:
1. **Coalesced Memory Access**: Contiguous u64 arrays
2. **Warp-Level Operations**: 32 threads = 32 words = 1024 trits in parallel
3. **No Divergence**: Bitwise operations avoid conditional branches
4. **Shared Memory**: Cache bitplanes for repeated operations

**Example CUDA Kernel**:
```cuda
__global__ void dot_product_kernel(
    const uint64_t* a,
    const uint64_t* b,
    int32_t* partial_sums,
    size_t n_words
) {
    const uint64_t MASK = 0x5555555555555555ULL;
    
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_words) return;
    
    uint64_t a_word = a[idx];
    uint64_t b_word = b[idx];
    
    // Bitplane extraction
    uint64_t a_pos = a_word & MASK;
    uint64_t a_neg = (a_word >> 1) & MASK;
    uint64_t b_pos = b_word & MASK;
    uint64_t b_neg = (b_word >> 1) & MASK;
    
    // Count matching/opposing
    int pp = __popcll(a_pos & b_pos);
    int nn = __popcll(a_neg & b_neg);
    int pn = __popcll(a_pos & b_neg);
    int np = __popcll(a_neg & b_pos);
    
    partial_sums[idx] = (pp + nn) - (pn + np);
}
```

**Expected Performance**: 
- 10-100x speedup for large vectors (>10M dimensions)
- Most beneficial for batch operations

---

## Integration with Sparse Representation

### Adaptive Strategy

The VSA system uses both representations adaptively:

```rust
pub enum TernaryVecRepr {
    Sparse(SparseVec),      // For < 25% density
    Packed(PackedTritVec),  // For ≥ 25% density
}
```

**Conversion Logic**:
```rust
impl SparseVec {
    pub fn to_packed(&self, len: usize) -> PackedTritVec {
        let density = (self.pos.len() + self.neg.len()) as f64 / len as f64;
        if density < 0.25 {
            // Warn: sparse might be better
        }
        PackedTritVec::from_sparsevec(self, len)
    }
}

impl PackedTritVec {
    pub fn to_sparse(&self) -> SparseVec {
        let density = self.count_nonzero() as f64 / self.len as f64;
        if density > 0.25 {
            // Warn: packed might be better
        }
        self.to_sparsevec()
    }
}
```

### When to Use Each

**Use SparseVec when**:
- Density < 25%
- Random access patterns
- Incremental construction
- Memory is constrained

**Use PackedTritVec when**:
- Density ≥ 25%
- Bulk operations (dot, bind, bundle)
- SIMD/GPU acceleration needed
- Throughput over latency

---

## Testing and Validation

### Correctness Tests

The implementation includes comprehensive tests:

1. **Roundtrip Tests**: `SparseVec → PackedTritVec → SparseVec`
2. **Operation Correctness**: Compare against reference implementation
3. **Bitplane Integrity**: Verify even/odd bit separation
4. **Boundary Conditions**: Edge cases (empty, single trit, etc.)

### Performance Benchmarks

Benchmark results (to be added):
- Dot product: SparseVec vs PackedTritVec at various densities
- Bind operation throughput
- Bundle operation throughput
- Memory usage comparison

---

## References

### Academic Background

1. **Bitslicing Technique**: Originally from cryptography (DES implementation)
   - Biham, E. (1997). "A Fast New DES Implementation in Software"
   
2. **Vector Symbolic Architectures**:
   - Kanerva, P. (2009). "Hyperdimensional Computing"
   - Plate, T. (2003). "Holographic Reduced Representation"

3. **Balanced Ternary**:
   - Knuth, D. (1998). "The Art of Computer Programming, Vol 2"
   - Stakhov, A. (2009). "The Mathematics of Harmony"

### Implementation Notes

- Bitslicing enables branchless ternary arithmetic
- Hardware popcount (POPCNT instruction) is critical for performance
- Modern CPUs have 1-cycle POPCNT latency
- GPU warp intrinsics (`__popc`) provide massive parallelism

---

## Future Enhancements

### Roadmap

**Phase 1** (Current): Scalar bitsliced implementation 
- PackedTritVec with word-level operations
- Automatic compiler vectorization
- Comprehensive test coverage

**Phase 2** (v0.21.0): Explicit SIMD
- Hand-written AVX2/AVX-512 kernels
- NEON optimizations for ARM64
- Runtime feature detection
- Target: 2-4x speedup

**Phase 3** (v0.22.0): GPU Acceleration
- CUDA kernels for NVIDIA GPUs
- OpenCL for cross-platform GPU support
- Vulkan compute shaders
- CPU/GPU hybrid execution
- Target: 10-100x speedup for large batches

**Phase 4** (v1.0.0): Production Optimization
- Cache-aware algorithms
- NUMA-aware memory allocation
- Multi-threading for large operations
- Benchmark suite and profiling tools

---

## Conclusion

The bitsliced balanced ternary representation in `PackedTritVec` provides:

 **Correctness**: Mathematically sound ternary operations  
 **Efficiency**: 2 bits per trit, no wasted space  
 **Performance**: Word-level parallelism (32 trits per op)  
 **SIMD-Ready**: Vectorization-friendly bitwise operations  
 **GPU-Ready**: Coalesced memory, no divergence  
 **Flexibility**: Easy conversion to/from sparse representation  

This design positions embeddenator-vsa for high-performance VSA operations on modern hardware, with clear paths to SIMD and GPU acceleration.

---

**Document Version**: 1.0  
**Last Updated**: 2026-01-14  
**Next Review**: After Phase 2 SIMD implementation
