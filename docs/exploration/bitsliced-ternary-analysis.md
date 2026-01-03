# Bitsliced Ternary VSA Analysis

**Date:** January 3, 2026  
**Branch:** `exploration/bitsliced-ternary`  
**Goal:** Determine optimal representation for holographic computational substrate

---

## The Debate: Mathematician vs. Implementer

### VSA Mathematician's Position

**Core Argument:** Ternary VSA requires algebraic closure over {-1, 0, +1} with specific properties:

1. **Bundle (⊕):** Majority voting must be associative across N vectors
2. **Bind (⊙):** Element-wise multiplication must be self-inverse
3. **Cosine:** Inner product must respect ternary arithmetic

**Current Problem with 2-bit Interleaved Encoding:**

The current `PackedTritVec` uses:
```
u64: |t0|t1|t2|...|t31|
     |01|10|00|...|01|  (2 bits each)
```

**Issue:** Binary addition on these fields creates carries that corrupt adjacent trits.

Example:
```
Trit A: +1 (01) at position i
Trit B: +1 (01) at position i
Bundle: Should be +1 (majority of two +1s)
Binary: 01 + 01 = 10 = -1 in our encoding! ❌
```

**Mathematical Requirement:** Operations must be **ganged** across all trits simultaneously without interference.

---

### Rust Implementer's Position

**Core Argument:** CPU efficiency requires cache-line alignment and SIMD vectorization.

**Current State:**
- `SparseVec`: Heap-allocated `Vec<usize>` — pointer chasing
- `PackedTritVec`: Dense but requires bit masking/shifting per trit

**Performance Constraints:**
1. **Cache Lines:** 64 bytes on modern CPUs
2. **SIMD Width:** AVX-512 = 512 bits = 8 × u64
3. **Register Pressure:** Limited to 16 vector registers

**Desired:** Operations that compile to pure register arithmetic with no branching.

---

## The Bitsliced Solution

### Core Insight: Separate Polarity Planes

Instead of interleaving trit bits, use **two parallel bit arrays**:

```rust
pub struct BitslicedTritVec {
    pos_bits: Vec<u64>,  // 1 = trit is +1
    neg_bits: Vec<u64>,  // 1 = trit is -1
    len: usize,          // Number of trits
}
```

**Encoding:**
- Trit +1: `pos=1, neg=0`
- Trit -1: `pos=0, neg=1`
- Trit 0:  `pos=0, neg=0`
- Invalid: `pos=1, neg=1` (never created)

**Capacity:** 64 trits per u64 word (vs 32 in interleaved)

---

## Mathematical Operations (Ganged Binary Logic)

### Operation 1: Bind (Ternary Multiplication)

**Truth Table:**
```
   × | -1   0  +1
-----+-----------
  -1 | +1   0  -1
   0 |  0   0   0
  +1 | -1   0  +1
```

**Bitsliced Implementation:**
```rust
pub fn bind(&self, other: &Self) -> Self {
    let mut result_pos = vec![0u64; self.pos_bits.len()];
    let mut result_neg = vec![0u64; self.neg_bits.len()];
    
    for i in 0..self.pos_bits.len() {
        let a_pos = self.pos_bits[i];
        let a_neg = self.neg_bits[i];
        let b_pos = other.pos_bits[i];
        let b_neg = other.neg_bits[i];
        
        // +1 result: (a_pos && b_pos) || (a_neg && b_neg)
        result_pos[i] = (a_pos & b_pos) | (a_neg & b_neg);
        
        // -1 result: (a_pos && b_neg) || (a_neg && b_pos)
        result_neg[i] = (a_pos & b_neg) | (a_neg & b_pos);
    }
    
    BitslicedTritVec { pos_bits: result_pos, neg_bits: result_neg, len: self.len }
}
```

**Key Property:** Self-inverse holds: `A ⊙ A = (+1 everywhere A is non-zero)`

---

### Operation 2: Bundle (Pairwise Majority)

For **two** vectors, bundle is conflict-cancel:

**Truth Table:**
```
  ⊕ | -1   0  +1
----+-----------
 -1 | -1  -1   0
  0 | -1   0  +1
 +1 |  0  +1  +1
```

**Bitsliced Implementation:**
```rust
pub fn bundle(&self, other: &Self) -> Self {
    let mut result_pos = vec![0u64; self.pos_bits.len()];
    let mut result_neg = vec![0u64; self.neg_bits.len()];
    
    for i in 0..self.pos_bits.len() {
        let a_pos = self.pos_bits[i];
        let a_neg = self.neg_bits[i];
        let b_pos = other.pos_bits[i];
        let b_neg = other.neg_bits[i];
        
        // Result is +1 if: (a is +1 and b is not -1) OR (b is +1 and a is not -1)
        result_pos[i] = (a_pos & !b_neg) | (b_pos & !a_neg);
        
        // Result is -1 if: (a is -1 and b is not +1) OR (b is -1 and a is not +1)
        result_neg[i] = (a_neg & !b_pos) | (b_neg & !a_pos);
    }
    
    BitslicedTritVec { pos_bits: result_pos, neg_bits: result_neg, len: self.len }
}
```

---

### Operation 3: Cosine Similarity (Dot Product)

**Formula:** `dot(A, B) = Σ(a_i × b_i)`

**Bitsliced Implementation:**
```rust
pub fn dot(&self, other: &Self) -> i64 {
    let mut dot_product = 0i64;
    
    for i in 0..self.pos_bits.len() {
        let a_pos = self.pos_bits[i];
        let a_neg = self.neg_bits[i];
        let b_pos = other.pos_bits[i];
        let b_neg = other.neg_bits[i];
        
        // +1 contribution: both positive or both negative
        let positive_matches = (a_pos & b_pos) | (a_neg & b_neg);
        
        // -1 contribution: opposite signs
        let negative_matches = (a_pos & b_neg) | (a_neg & b_pos);
        
        dot_product += positive_matches.count_ones() as i64;
        dot_product -= negative_matches.count_ones() as i64;
    }
    
    dot_product
}
```

**Norm:** `||A|| = √(count_ones(pos_bits) + count_ones(neg_bits))`

---

## The 32-Trit vs. 27-Trit Decision

### Mathematician's View: 27 Trits (3³)

**Arguments:**
1. **Algebraic Beauty:** 3^27 = 7,625,597,484,987 states
2. **Ternary Native:** Powers of 3 align with base-3 arithmetic
3. **Fits in 54 bits:** 27 × 2 bits = 54, leaving 10 bits for metadata

### Implementer's View: 32 Trits (2⁵)

**Arguments:**
1. **Cache Alignment:** 32 trits = 1 u64 per plane = 64 bits
2. **SIMD Efficiency:** AVX-512 operates on 512 bits = 8 words × 64 trits = 512 trits/cycle
3. **No Bit Shifting:** Access trit i = word[i/64] bit [i%64]

### Consensus: **64 Trits per u64 Pair**

**Compromise:** Neither 27 nor 32, use the natural bitsliced unit:

```rust
pub const TRITS_PER_WORD: usize = 64;  // One bit per trit in each plane
pub const WORDS_PER_VECTOR: usize = 157;  // 157 × 64 = 10,048 trits ≈ 10K
```

**Rationale:**
- Aligns with CPU word size
- Maximizes density (64 trits in 128 bits total)
- Simplifies indexing (no bit extraction)

---

## Memory Hierarchy Integration

### Register-Resident Operations

**Target:** Keep entire vector in registers during hot operations

**AVX-512 Capacity:**
- 32 ZMM registers × 512 bits = 16 KB
- Enough for: 16,384 bits / 2 planes = 8,192 trits

**Strategy:**
```rust
#[target_feature(enable = "avx512f")]
unsafe fn bind_register_resident(a: &[__m512i; 16], b: &[__m512i; 16]) -> [__m512i; 16] {
    let mut result = [_mm512_setzero_si512(); 16];
    for i in 0..16 {
        // Ganged bind across 512 trits per iteration
        result[i] = /* bitsliced bind logic */
    }
    result
}
```

---

## Benchmark Predictions

### Current SparseVec (Vec<usize> indices)
- Bundle: ~2-5 µs (200 elements)
- Memory: ~1.6 KB per vector

### Bitsliced (u64 pairs)
- Bundle: ~50-100 ns (entire 10K vector in registers)
- Memory: 2.5 KB per vector (10K trits × 2 bits / 8)

**Expected Speedup:** 20-50× for dense operations

---

## Open Questions for Experimentation

### Q1: Sparsity Representation

**Problem:** Bitsliced is dense. How do we handle sparse vectors efficiently?

**Option A:** Hybrid
```rust
pub enum TritVec {
    Sparse { pos: Vec<usize>, neg: Vec<usize> },
    Bitsliced { pos_bits: Vec<u64>, neg_bits: Vec<u64> },
}
```

**Option B:** Blocked Bitsliced
```rust
pub struct BlockedBitsliced {
    blocks: HashMap<u32, BitslicedBlock>,  // Only store non-zero blocks
}
```

### Q2: Associative Bundle for N Vectors

Current pairwise bundle is not fully associative. For true holographic superposition:

**Accumulator Method:**
```rust
pub fn bundle_many<'a, I>(vecs: I) -> BitslicedTritVec 
where I: IntoIterator<Item = &'a BitslicedTritVec> {
    // Accumulate trit-wise counts: -1, 0, +1
    // Then threshold to sign
    ...
}
```

### Q3: NTT Integration

Number Theoretic Transform requires polynomial representation. Does bitsliced format help or hinder?

---

## Implementation Plan

### Phase 1: Core Bitsliced Type (This Branch)

1. [ ] Implement `BitslicedTritVec` struct
2. [ ] Implement bind, bundle, dot operations
3. [ ] Comprehensive test suite (property-based)
4. [ ] Benchmark against current `PackedTritVec`

### Phase 2: Sparse-Dense Hybrid

1. [ ] Define crossover threshold (when to use bitsliced)
2. [ ] Implement automatic conversion
3. [ ] Benchmark hybrid vs pure sparse

### Phase 3: AVX-512 Intrinsics

1. [ ] Implement register-resident operations
2. [ ] Validate algebraic properties hold
3. [ ] Measure actual speedup

---

## Validation Criteria

For this exploration to be successful:

1. **Correctness:** All algebraic properties must hold
2. **Performance:** >10× speedup on dense operations
3. **Composability:** Must integrate with existing VSA pipeline
4. **Memory:** Comparable or better than current encoding

---

*This is a living document. Update as we discover insights.*
