# Hybrid Tryte Precision: Algebraic Density in Bitsliced VSA

**Branch**: `exploration/hybrid-tryte-precision`  
**Parent**: `exploration/bitsliced-ternary`  
**Date**: 2026-01-03  
**Status**: 🔬 **EXPLORATION IN PROGRESS**

---

## 1. Motivation: Beyond Binary Bitslicing

The bitsliced representation (`pos_bits`, `neg_bits`) achieves 2.3-2.7× speedup via ganged binary operations. But we're leaving precision on the table.

### The Core Insight

**Standard ternary**: 3 states (-1, 0, +1) encoded in 2 bits per position  
**Tryte (3 trits)**: 27 states, still sparse but algebraically richer  
**Soft ternary**: Multi-bit vote counts enabling fractional confidence

**Question**: Can we increase **effective precision** while maintaining bitsliced parallelism?

---

## 2. Hybrid Approaches to Explore

### 2.1 Multi-Bit Soft Ternary (MBST)

Instead of hard {-1, 0, +1}, use multiple bit planes for **vote counts**:

```
Standard (2 bits):  pos[i], neg[i]  →  3 states
Soft-3 (4 bits):    pos[1:0][i], neg[1:0][i]  →  5 effective levels
Soft-7 (6 bits):    pos[2:0][i], neg[2:0][i]  →  9 effective levels
```

**Encoding**:
```
Level -4: neg = 0b100  (strong negative)
Level -3: neg = 0b011
Level -2: neg = 0b010
Level -1: neg = 0b001
Level  0: pos = 0, neg = 0
Level +1: pos = 0b001
Level +2: pos = 0b010
Level +3: pos = 0b011
Level +4: pos = 0b100  (strong positive)
```

**Advantage**: Bundle operations preserve more information before collapsing to hard ternary.

### 2.2 Tryte Packing (Algebraic Groups)

Pack 3 trits into a "tryte" (27 values, fits in 5 bits):

```
Trit₀ × 1 + Trit₁ × 3 + Trit₂ × 9 = value ∈ [0, 26]

Where each Trit ∈ {0, 1, 2} maps to {-1, 0, +1}:
  0 → -1
  1 →  0
  2 → +1
```

**Why 27 ≈ 32?** A tryte fits in 5 bits (2^5 = 32), wasting only 5/32 = 15.6% capacity.

**Algebraic Operations**:
```rust
// Tryte multiplication via lookup or direct computation
fn tryte_bind(a: u8, b: u8) -> u8 {
    // Decompose, multiply component-wise, recompose
    let (a0, a1, a2) = decompose_tryte(a);
    let (b0, b1, b2) = decompose_tryte(b);
    compose_tryte(
        trit_mul(a0, b0),
        trit_mul(a1, b1),
        trit_mul(a2, b2)
    )
}
```

### 2.3 Redundant Binary Representation (RBR)

Use signed-digit representation where each position has 3 possible values but stored with redundancy:

```
Standard binary:  bit ∈ {0, 1}
Signed digit:     digit ∈ {-1, 0, +1}

Key insight: -1 and +1 are distinct from 0, enabling...
- Carry-free addition (no carry propagation!)
- Parallel subtraction
- Constant-time comparisons
```

**Implementation**:
```rust
struct RedundantBinaryVec {
    // Each digit stored as (negative_bit, positive_bit)
    // 00 = 0, 01 = +1, 10 = -1, 11 = invalid
    neg: Vec<u64>,
    pos: Vec<u64>,
    // Invariant: neg & pos == 0 (no position is both)
}
```

Wait - this is exactly what we already have! The insight is to **reinterpret** it as redundant binary, enabling:
- Subtraction without borrow chains
- Parallel magnitude comparisons
- Direct signed accumulation

### 2.4 Residue Number System (RNS) Encoding

Encode values modulo small coprimes for carry-free operations:

```
Value V encoded as (V mod 3, V mod 5, V mod 7)

Example: V = 17
  17 mod 3 = 2
  17 mod 5 = 2
  17 mod 7 = 3
  Encoded: (2, 2, 3)

Addition: (a₁, a₂, a₃) + (b₁, b₂, b₃) = ((a₁+b₁) mod 3, (a₂+b₂) mod 5, (a₃+b₃) mod 7)
```

**Advantage**: Parallel, carry-free operations for accumulation.  
**Challenge**: Comparison and conversion are expensive.

---

## 3. The "Hybrid Tryte" Proposal

### Core Idea: Bitsliced Operations on Grouped Trits

Instead of processing individual trits, group them into **algebraically meaningful units**:

```
Standard bitsliced: 64 trits per u64 word
Hybrid tryte:       21 trytes per u64 word (21 × 3 = 63 trits, wastes 1)
                    OR
                    12 trytes per u64 word using 5-bit encoding (12 × 5 = 60 bits)
```

### Data Structure

```rust
/// Hybrid representation mixing bitsliced and tryte-packed encodings.
/// 
/// The key insight: Use bitsliced for **compute** (bind/bundle),
/// but tryte-packed for **storage** (compression) and **precision** (soft values).
pub struct HybridTryteVec {
    /// Dimension in trits (not trytes)
    len: usize,
    
    /// Primary representation: 3-bit soft ternary per position
    /// Bits [2:0] of each position: magnitude (0-7)
    /// With sign stored separately in sign plane
    magnitude: Vec<u64>,  // 3 planes interleaved OR separate
    sign: Vec<u64>,       // 0 = positive, 1 = negative
    
    /// Alternative: Direct tryte packing for memory efficiency
    /// 12 trytes per u64 (5 bits each)
    // tryte_packed: Vec<u64>,
}
```

### Operation Semantics

**Soft Bundle** (sum with extended precision):
```rust
fn soft_bundle(&self, other: &Self) -> Self {
    // Instead of saturating at {-1, 0, +1}, accumulate magnitudes
    // Result magnitude: min(|a| + |b|, 7)
    // Result sign: determined by majority or larger magnitude
}
```

**Hard Threshold** (convert soft to hard):
```rust
fn harden(&self, threshold: u8) -> BitslicedTritVec {
    // For each position:
    //   if magnitude < threshold: → 0
    //   if magnitude >= threshold && sign == 0: → +1
    //   if magnitude >= threshold && sign == 1: → -1
}
```

---

## 4. Dimensionality Scaling Analysis

### Target: 100,000,000 Dimensions

```
D = 100M trits

BitslicedTritVec:
  Words per plane: ⌈100M / 64⌉ = 1,562,500
  Total memory: 1,562,500 × 2 × 8 = 25 MB per vector
  
HybridTryteVec (4 planes for soft-7):
  Words per plane: 1,562,500
  Total memory: 1,562,500 × 4 × 8 = 50 MB per vector

Tryte-packed (5-bit):
  Trytes: ⌈100M / 3⌉ = 33,333,334
  Words: ⌈33,333,334 × 5 / 64⌉ = 2,604,167
  Total memory: 2,604,167 × 8 = 20.8 MB per vector
```

### Memory Bandwidth Considerations

**DDR4-3200**: ~50 GB/s (single channel)  
**DDR5-6400**: ~100 GB/s  
**HBM2e**: ~460 GB/s

For D = 100M:
- Read 2 vectors (bitsliced): 50 MB
- Write 1 vector: 25 MB
- Total per operation: 75 MB
- At DDR4 bandwidth: 75 MB / 50 GB/s = **1.5 ms per bind**
- At HBM2e: 75 MB / 460 GB/s = **163 µs per bind**

**Conclusion**: Memory bandwidth dominates at 100M dimensions. GPU with HBM is essential.

---

## 5. Mathematical Foundations

### 5.1 Soft Ternary Algebra

**Soft value**: s ∈ [-K, +K] where K is max magnitude

**Operations**:
```
Soft bind: s₁ ⊙ s₂ = sgn(s₁ × s₂) × min(|s₁| × |s₂| / K, K)
Soft bundle: s₁ ⊕ s₂ = clamp(s₁ + s₂, -K, K)
Dot product: Σᵢ s₁[i] × s₂[i]  (sum of products, not clamped)
```

**Hardening threshold τ**:
```
hard(s) = +1 if s ≥ τ
        = -1 if s ≤ -τ
        =  0 otherwise
```

### 5.2 Information-Theoretic Density

**Standard ternary at 2% sparsity**:
- Non-zero positions: 0.02 × D
- Information per position: log₂(3) = 1.58 bits
- Total info: 0.02 × D × 1.58 bits

**Soft-7 at 2% sparsity**:
- Non-zero positions: 0.02 × D
- Information per position: log₂(15) = 3.91 bits (9 levels + sign)
- Total info: 0.02 × D × 3.91 bits

**Density increase**: 3.91 / 1.58 = **2.47× more information** per non-zero position!

### 5.3 Precision-Performance Trade-off

| Representation | Bits/Trit | Levels | Bind Ops/Word | Memory Ratio |
|----------------|-----------|--------|---------------|--------------|
| Bitsliced      | 2         | 3      | 4 AND + 2 OR  | 1.0×         |
| Soft-3         | 4         | 5      | 8 AND + 4 OR  | 2.0×         |
| Soft-7         | 6         | 9      | 12 AND + 6 OR | 3.0×         |
| Tryte-5        | 5         | 27     | ~15 ops       | 2.5×         |

---

## 6. Proposed Implementation Strategy

### Phase 1: Extended Testing Infrastructure

```rust
#[cfg(test)]
mod scalability_tests {
    const DIMENSIONS: &[usize] = &[
        1_000,           // 1K - cache-local
        10_000,          // 10K - current default
        100_000,         // 100K - validated
        1_000_000,       // 1M - L3 cache boundary
        10_000_000,      // 10M - RAM-bound
        100_000_000,     // 100M - target extreme
    ];
    
    #[test]
    fn test_bind_at_scale() { ... }
    
    #[test]
    fn test_bundle_at_scale() { ... }
    
    #[test]
    fn test_dot_at_scale() { ... }
}
```

### Phase 2: Soft Ternary Implementation

```rust
/// 3-bit magnitude + 1-bit sign = 4 bits per position
pub struct SoftTernaryVec {
    len: usize,
    mag_lo: Vec<u64>,   // Magnitude bit 0
    mag_mi: Vec<u64>,   // Magnitude bit 1
    mag_hi: Vec<u64>,   // Magnitude bit 2
    sign: Vec<u64>,     // 0 = positive, 1 = negative
}

impl SoftTernaryVec {
    /// Soft bundle: accumulate magnitudes with carry
    pub fn soft_bundle(&self, other: &Self) -> Self {
        // 3-bit add with saturation at 7
    }
    
    /// Convert to hard ternary with threshold
    pub fn harden(&self, threshold: u8) -> BitslicedTritVec {
        // threshold typically 1 (any vote) or 2 (majority)
    }
}
```

### Phase 3: Hybrid Operations

```rust
impl HybridTryteVec {
    /// Accumulate N vectors with full precision, then harden
    pub fn precise_bundle<'a>(vectors: impl Iterator<Item = &'a BitslicedTritVec>) -> BitslicedTritVec {
        let mut soft = SoftTernaryVec::new_zero(dim);
        for v in vectors {
            soft.accumulate_bitsliced(v);
        }
        soft.harden(THRESHOLD)
    }
}
```

### Phase 4: GPU Acceleration (Future)

```rust
#[cfg(feature = "gpu")]
pub mod gpu {
    use wgpu::*;
    
    pub struct GpuBitslicedVec {
        pos_buffer: Buffer,
        neg_buffer: Buffer,
        dim: usize,
    }
    
    impl GpuBitslicedVec {
        pub async fn bind(&self, other: &Self) -> Self {
            // Dispatch compute shader
        }
    }
}
```

---

## 7. Testing Strategy for 100M Dimensions

### Memory Management

```rust
fn test_at_dimension(dim: usize) {
    // Estimate memory
    let bytes_per_vec = (dim / 64 + 1) * 2 * 8;
    let available_ram = sys_info::mem_info().unwrap().avail * 1024;
    
    if bytes_per_vec * 3 > available_ram {
        eprintln!("Skipping dim={} (need {}MB, have {}MB)", 
            dim, bytes_per_vec * 3 / 1_000_000, available_ram / 1_000_000);
        return;
    }
    
    // Proceed with test...
}
```

### Correctness vs Performance Tests

```rust
// Correctness: Sample random positions, verify algebraic properties
#[test]
fn test_bind_correctness_100m() {
    let dim = 100_000_000;
    let a = random_sparse_bitsliced(dim, 0.001); // 0.1% density
    let b = random_sparse_bitsliced(dim, 0.001);
    
    let result = a.bind(&b);
    
    // Sample 10,000 random positions
    for _ in 0..10_000 {
        let idx = rand::random::<usize>() % dim;
        let expected = trit_multiply(a.get(idx), b.get(idx));
        assert_eq!(result.get(idx), expected);
    }
}

// Performance: Measure throughput
#[bench]
fn bench_bind_100m(b: &mut Bencher) {
    let dim = 100_000_000;
    let a = random_sparse_bitsliced(dim, 0.001);
    let other = random_sparse_bitsliced(dim, 0.001);
    
    b.iter(|| {
        let result = black_box(&a).bind(black_box(&other));
        black_box(result)
    });
}
```

---

## 8. Expected Outcomes

### Performance Projections (D = 100M)

| Operation | Time (DDR4) | Time (DDR5) | Time (HBM) |
|-----------|-------------|-------------|------------|
| Bind      | ~1.5 ms     | ~750 µs     | ~163 µs    |
| Bundle    | ~1.5 ms     | ~750 µs     | ~163 µs    |
| Dot       | ~2.0 ms     | ~1.0 ms     | ~220 µs    |
| Soft Bundle | ~3.0 ms   | ~1.5 ms     | ~330 µs    |

### Precision Improvements

**Standard N-way bundle** (binary vote):
- Information loss: ~30% per bundle operation
- Equivalent vectors needed for same fidelity: ~1.4×

**Soft-7 N-way bundle** (7-level accumulation):
- Information loss: ~12% per bundle operation
- Equivalent vectors: ~1.1×

**Net effect**: Soft bundle requires **fewer vectors** to achieve same retrieval quality.

---

## 9. Open Questions

1. **Optimal magnitude bits**: 3 bits (7 levels) vs 4 bits (15 levels)?
2. **Hardening threshold**: Fixed τ=2 or adaptive per-query?
3. **GPU dispatch granularity**: Per-operation or batched?
4. **Memory layout for GPU**: Structure-of-arrays vs array-of-structures?
5. **Sparse soft bundle**: Skip zero-magnitude positions?

---

## 10. Next Steps

1. ✅ Create exploration branch
2. 🔄 Implement extended dimension tests (1K → 100M)
3. ⬜ Implement `SoftTernaryVec` with 4-bit precision
4. ⬜ Benchmark soft vs hard bundle on precision/recall
5. ⬜ Profile memory bandwidth at scale
6. ⬜ Design GPU compute shader architecture
7. ⬜ Prototype wgpu-based acceleration

---

**Document Version**: 0.1 (Draft)  
**Last Updated**: 2026-01-03
