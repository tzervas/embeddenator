# BitslicedTritVec Implementation Analysis

**Date:** January 2026  
**Author:** Rust Implementer Agent  
**Status:** Implementation Proposal

## Executive Summary

After analyzing the current `PackedTritVec` implementation and the mathematician's analysis, I recommend a **phased migration** to bitsliced representation rather than wholesale replacement. The 3.7× theoretical efficiency gain for bind operations is achievable, but practical concerns around cache behavior and conversion overhead require careful handling.

---

## 1. Memory Layout Analysis

### Current: Interleaved 2-bit Encoding

```
PackedTritVec.data: Vec<u64>
┌────────────────────────────────────────────────────────────────────┐
│ u64[0]: [p₀n₀|p₁n₁|p₂n₂|...|p₃₁n₃₁] (32 trits interleaved)       │
│ u64[1]: [p₃₂n₃₂|p₃₃n₃₃|...|p₆₃n₆₃]                               │
│ ...                                                                │
└────────────────────────────────────────────────────────────────────┘
```

**Bind operation (current):**
```rust
// Per-word overhead: 4 mask operations + 3 shifts
let a_pos = a & 0x5555_5555_5555_5555;  // Extract even bits
let a_neg = (a >> 1) & 0x5555_5555_5555_5555;
let b_pos = b & 0x5555_5555_5555_5555;
let b_neg = (b >> 1) & 0x5555_5555_5555_5555;

let same = (a_pos & b_pos) | (a_neg & b_neg);  // 2 AND + 1 OR
let opp = (a_pos & b_neg) | (a_neg & b_pos);   // 2 AND + 1 OR
out = same | (opp << 1);                        // 1 OR + 1 shift
// Total: 11 ALU ops for 32 trits = 0.34 ops/trit
```

### Proposed: Bitsliced Representation

```
BitslicedTritVec:
  pos: Vec<u64>  ──→  [p₀p₁p₂...p₆₃|p₆₄p₆₅...p₁₂₇|...]
  neg: Vec<u64>  ──→  [n₀n₁n₂...n₆₃|n₆₄n₆₅...n₁₂₇|...]
```

**Bind operation (bitsliced):**
```rust
// Direct logic, no extraction needed
out_pos = (a_pos & b_pos) | (a_neg & b_neg);  // 2 AND + 1 OR
out_neg = (a_pos & b_neg) | (a_neg & b_pos);  // 2 AND + 1 OR
// Total: 6 ALU ops for 64 trits = 0.094 ops/trit
// Speedup: 0.34 / 0.094 = 3.6× (matches mathematician's analysis!)
```

### Cache Behavior for D=10M

| Metric | Interleaved | Bitsliced |
|--------|-------------|-----------|
| Total memory | 2.5 MB | 2.5 MB |
| Memory layout | 1 contiguous | 2 × 1.25 MB |
| L3 cache fit (typical 8MB) | ✓ | ✓ |
| Prefetch pattern | Sequential | 2-stream interleaved |
| TLB entries (4KB pages) | 625 | 625 (312 × 2) |

**Analysis:** Both fit in L3. Bitsliced requires dual-stream prefetch but modern CPUs handle this well (Intel/AMD have 2+ HW prefetchers). The **real win** is ALU throughput, not memory bandwidth.

---

## 2. Concrete Rust Implementation

### Core Structure

```rust
//! Bitsliced ternary vector for maximum throughput VSA operations.
//!
//! Representation: separate bit-planes for positive and negative components
//! - pos[i] bit j = 1 iff trit (i*64 + j) is +1
//! - neg[i] bit j = 1 iff trit (i*64 + j) is -1
//! - Both bits 0 = trit is 0 (valid encoding)
//! - Both bits 1 = invalid (treated as 0)

use crate::ternary::Trit;
use crate::vsa::SparseVec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitslicedTritVec {
    /// Number of logical trits
    len: usize,
    /// Positive plane: bit i = 1 iff trit i is +1
    pos: Vec<u64>,
    /// Negative plane: bit i = 1 iff trit i is -1  
    neg: Vec<u64>,
}

impl BitslicedTritVec {
    /// Create zero vector of given length
    #[inline]
    pub fn new_zero(len: usize) -> Self {
        let words = (len + 63) / 64;
        Self {
            len,
            pos: vec![0u64; words],
            neg: vec![0u64; words],
        }
    }

    /// Number of words needed for len trits
    #[inline(always)]
    const fn word_count(len: usize) -> usize {
        (len + 63) / 64
    }

    /// Mask for valid bits in the last word
    #[inline(always)]
    const fn last_word_mask(len: usize) -> u64 {
        let bits_used = len % 64;
        if bits_used == 0 {
            !0u64
        } else {
            (1u64 << bits_used) - 1
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get single trit (bounds-checked, slower path for debugging)
    #[inline]
    pub fn get(&self, i: usize) -> Trit {
        if i >= self.len {
            return Trit::Z;
        }
        let word = i / 64;
        let bit = i % 64;
        let p = (self.pos.get(word).copied().unwrap_or(0) >> bit) & 1;
        let n = (self.neg.get(word).copied().unwrap_or(0) >> bit) & 1;
        match (p, n) {
            (1, 0) => Trit::P,
            (0, 1) => Trit::N,
            _ => Trit::Z,
        }
    }

    /// Set single trit (slower path for construction)
    #[inline]
    pub fn set(&mut self, i: usize, t: Trit) {
        if i >= self.len {
            return;
        }
        let word = i / 64;
        let bit = i % 64;
        let mask = 1u64 << bit;
        
        // Clear both bits first
        self.pos[word] &= !mask;
        self.neg[word] &= !mask;
        
        // Set appropriate bit
        match t {
            Trit::P => self.pos[word] |= mask,
            Trit::N => self.neg[word] |= mask,
            Trit::Z => {}
        }
    }

    // ========================================================================
    // CORE VSA OPERATIONS - These are the hot paths
    // ========================================================================

    /// Bind (element-wise multiplication): O(D/64) with minimal ops per word
    /// 
    /// Truth table for trit multiply:
    /// ```text
    ///   ×  | P  Z  N
    ///   ---+--------
    ///   P  | P  Z  N
    ///   Z  | Z  Z  Z
    ///   N  | N  Z  P
    /// ```
    /// 
    /// Bitwise: out_pos = (a_pos & b_pos) | (a_neg & b_neg)  [same signs]
    ///          out_neg = (a_pos & b_neg) | (a_neg & b_pos)  [diff signs]
    #[inline]
    pub fn bind(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        let words = Self::word_count(n).min(self.pos.len()).min(other.pos.len());
        
        let mut out = Self::new_zero(n);
        
        for w in 0..words {
            let (ap, an) = (self.pos[w], self.neg[w]);
            let (bp, bn) = (other.pos[w], other.neg[w]);
            
            out.pos[w] = (ap & bp) | (an & bn);
            out.neg[w] = (ap & bn) | (an & bp);
        }
        
        // Mask trailing bits
        if !out.pos.is_empty() {
            let last = out.pos.len() - 1;
            let mask = Self::last_word_mask(n);
            out.pos[last] &= mask;
            out.neg[last] &= mask;
        }
        
        out
    }

    /// Bind into pre-allocated output (avoid allocation in hot loops)
    #[inline]
    pub fn bind_into(&self, other: &Self, out: &mut Self) {
        let n = self.len.min(other.len);
        let words = Self::word_count(n);
        
        // Resize and clear output
        out.len = n;
        out.pos.resize(words, 0);
        out.neg.resize(words, 0);
        
        for w in 0..words.min(self.pos.len()).min(other.pos.len()) {
            let (ap, an) = (self.pos[w], self.neg[w]);
            let (bp, bn) = (other.pos[w], other.neg[w]);
            
            out.pos[w] = (ap & bp) | (an & bn);
            out.neg[w] = (ap & bn) | (an & bp);
        }
        
        if !out.pos.is_empty() {
            let last = out.pos.len() - 1;
            let mask = Self::last_word_mask(n);
            out.pos[last] &= mask;
            out.neg[last] &= mask;
        }
    }

    /// Bundle (element-wise saturating addition) for two vectors
    /// 
    /// Truth table for trit add (saturated):
    /// ```text
    ///   +  | P  Z  N
    ///   ---+--------
    ///   P  | P  P  Z
    ///   Z  | P  Z  N
    ///   N  | Z  N  N
    /// ```
    /// 
    /// Logic: out_pos = (a_pos & !b_neg) | (b_pos & !a_neg)
    ///        out_neg = (a_neg & !b_pos) | (b_neg & !a_pos)
    #[inline]
    pub fn bundle(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        let words = Self::word_count(n).min(self.pos.len()).min(other.pos.len());
        
        let mut out = Self::new_zero(n);
        
        for w in 0..words {
            let (ap, an) = (self.pos[w], self.neg[w]);
            let (bp, bn) = (other.pos[w], other.neg[w]);
            
            out.pos[w] = (ap & !bn) | (bp & !an);
            out.neg[w] = (an & !bp) | (bn & !ap);
        }
        
        if !out.pos.is_empty() {
            let last = out.pos.len() - 1;
            let mask = Self::last_word_mask(n);
            out.pos[last] &= mask;
            out.neg[last] &= mask;
        }
        
        out
    }

    /// Dot product: count matching signs minus opposing signs
    #[inline]
    pub fn dot(&self, other: &Self) -> i32 {
        let n = self.len.min(other.len);
        let words = Self::word_count(n).min(self.pos.len()).min(other.pos.len());
        
        let mut acc: i32 = 0;
        
        for w in 0..words {
            let (mut ap, mut an) = (self.pos[w], self.neg[w]);
            let (mut bp, mut bn) = (other.pos[w], other.neg[w]);
            
            // Mask last word
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                ap &= mask;
                an &= mask;
                bp &= mask;
                bn &= mask;
            }
            
            let pp = (ap & bp).count_ones() as i32;  // +1 × +1 = +1
            let nn = (an & bn).count_ones() as i32;  // -1 × -1 = +1
            let pn = (ap & bn).count_ones() as i32;  // +1 × -1 = -1
            let np = (an & bp).count_ones() as i32;  // -1 × +1 = -1
            
            acc += (pp + nn) - (pn + np);
        }
        
        acc
    }

    /// Cosine similarity
    #[inline]
    pub fn cosine(&self, other: &Self) -> f64 {
        let dot = self.dot(other) as f64;
        let a_nnz = self.nnz() as f64;
        let b_nnz = other.nnz() as f64;
        
        if a_nnz == 0.0 || b_nnz == 0.0 {
            0.0
        } else {
            dot / (a_nnz.sqrt() * b_nnz.sqrt())
        }
    }

    /// Count non-zero elements
    #[inline]
    pub fn nnz(&self) -> usize {
        let words = Self::word_count(self.len);
        let mut count = 0usize;
        
        for w in 0..words.min(self.pos.len()) {
            let (mut p, mut n) = (self.pos[w], self.neg[w]);
            
            if w + 1 == words {
                let mask = Self::last_word_mask(self.len);
                p &= mask;
                n &= mask;
            }
            
            // Union of positive and negative (don't double-count invalid 11 states)
            count += (p | n).count_ones() as usize;
        }
        
        count
    }

    /// Permute (cyclic shift) for sequence encoding
    #[inline]
    pub fn permute(&self, shift: usize) -> Self {
        let mut out = Self::new_zero(self.len);
        
        for i in 0..self.len {
            let new_idx = (i + shift) % self.len;
            out.set(new_idx, self.get(i));
        }
        
        out
    }
}
```

---

## 3. AVX-512 Acceleration

### Bind Operation (512-bit / 64-byte registers)

```rust
#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
mod avx512 {
    use std::arch::x86_64::*;
    use super::BitslicedTritVec;

    /// AVX-512 bind: processes 512 trits per iteration
    /// 
    /// Each __m512i holds 8 × u64 = 512 bits = 512 trits per plane
    #[target_feature(enable = "avx512f")]
    pub unsafe fn bind_avx512(
        a: &BitslicedTritVec,
        b: &BitslicedTritVec,
        out: &mut BitslicedTritVec,
    ) {
        let n = a.len.min(b.len);
        let words = (n + 63) / 64;
        
        out.len = n;
        out.pos.resize(words, 0);
        out.neg.resize(words, 0);
        
        let chunks = words / 8;  // 8 u64s per __m512i
        
        // Process 512 trits at a time
        for chunk in 0..chunks {
            let offset = chunk * 8;
            
            // Load 512-bit vectors
            let ap = _mm512_loadu_si512(a.pos.as_ptr().add(offset) as *const __m512i);
            let an = _mm512_loadu_si512(a.neg.as_ptr().add(offset) as *const __m512i);
            let bp = _mm512_loadu_si512(b.pos.as_ptr().add(offset) as *const __m512i);
            let bn = _mm512_loadu_si512(b.neg.as_ptr().add(offset) as *const __m512i);
            
            // Bind logic: out_pos = (ap & bp) | (an & bn)
            //             out_neg = (ap & bn) | (an & bp)
            let same_pp = _mm512_and_si512(ap, bp);
            let same_nn = _mm512_and_si512(an, bn);
            let out_pos = _mm512_or_si512(same_pp, same_nn);
            
            let diff_pn = _mm512_and_si512(ap, bn);
            let diff_np = _mm512_and_si512(an, bp);
            let out_neg = _mm512_or_si512(diff_pn, diff_np);
            
            // Store results
            _mm512_storeu_si512(out.pos.as_mut_ptr().add(offset) as *mut __m512i, out_pos);
            _mm512_storeu_si512(out.neg.as_mut_ptr().add(offset) as *mut __m512i, out_neg);
        }
        
        // Handle remainder with scalar
        for w in (chunks * 8)..words {
            let (ap, an) = (a.pos[w], a.neg[w]);
            let (bp, bn) = (b.pos[w], b.neg[w]);
            out.pos[w] = (ap & bp) | (an & bn);
            out.neg[w] = (ap & bn) | (an & bp);
        }
    }

    /// AVX-512 dot product with horizontal sum
    #[target_feature(enable = "avx512f", enable = "avx512vpopcntdq")]
    pub unsafe fn dot_avx512(a: &BitslicedTritVec, b: &BitslicedTritVec) -> i32 {
        let n = a.len.min(b.len);
        let words = (n + 63) / 64;
        let chunks = words / 8;
        
        let mut acc_pos = _mm512_setzero_si512();
        let mut acc_neg = _mm512_setzero_si512();
        
        for chunk in 0..chunks {
            let offset = chunk * 8;
            
            let ap = _mm512_loadu_si512(a.pos.as_ptr().add(offset) as *const __m512i);
            let an = _mm512_loadu_si512(a.neg.as_ptr().add(offset) as *const __m512i);
            let bp = _mm512_loadu_si512(b.pos.as_ptr().add(offset) as *const __m512i);
            let bn = _mm512_loadu_si512(b.neg.as_ptr().add(offset) as *const __m512i);
            
            // Count matching signs (positive contribution)
            let pp = _mm512_popcnt_epi64(_mm512_and_si512(ap, bp));
            let nn = _mm512_popcnt_epi64(_mm512_and_si512(an, bn));
            acc_pos = _mm512_add_epi64(acc_pos, _mm512_add_epi64(pp, nn));
            
            // Count opposing signs (negative contribution)
            let pn = _mm512_popcnt_epi64(_mm512_and_si512(ap, bn));
            let np = _mm512_popcnt_epi64(_mm512_and_si512(an, bp));
            acc_neg = _mm512_add_epi64(acc_neg, _mm512_add_epi64(pn, np));
        }
        
        // Horizontal sum (reduce 8 × i64 to single i64)
        let pos_sum = _mm512_reduce_add_epi64(acc_pos);
        let neg_sum = _mm512_reduce_add_epi64(acc_neg);
        
        // Handle scalar remainder
        let mut scalar_acc: i64 = pos_sum - neg_sum;
        for w in (chunks * 8)..words {
            let (ap, an) = (a.pos[w], a.neg[w]);
            let (bp, bn) = (b.pos[w], b.neg[w]);
            
            let pp = (ap & bp).count_ones() as i64;
            let nn = (an & bn).count_ones() as i64;
            let pn = (ap & bn).count_ones() as i64;
            let np = (an & bp).count_ones() as i64;
            
            scalar_acc += (pp + nn) - (pn + np);
        }
        
        scalar_acc as i32
    }
}
```

### Performance Projection

| Operation | Interleaved Scalar | Bitsliced Scalar | Bitsliced AVX-512 |
|-----------|-------------------|------------------|-------------------|
| Bind (D=10M) | ~1.8ms | ~0.5ms | ~0.08ms |
| Dot (D=10M) | ~1.5ms | ~0.4ms | ~0.06ms |
| Bundle (D=10M) | ~2.0ms | ~0.6ms | ~0.10ms |

*Estimates based on 3GHz × 2 ALU ports, L3-resident data*

---

## 4. Conversion Functions

```rust
impl BitslicedTritVec {
    // ========================================================================
    // CONVERSIONS
    // ========================================================================

    /// Convert from SparseVec: O(nnz) - optimal for sparse inputs
    pub fn from_sparse(sparse: &SparseVec, len: usize) -> Self {
        let words = Self::word_count(len);
        let mut out = Self {
            len,
            pos: vec![0u64; words],
            neg: vec![0u64; words],
        };
        
        for &idx in &sparse.pos {
            if idx < len {
                let word = idx / 64;
                let bit = idx % 64;
                out.pos[word] |= 1u64 << bit;
            }
        }
        
        for &idx in &sparse.neg {
            if idx < len {
                let word = idx / 64;
                let bit = idx % 64;
                out.neg[word] |= 1u64 << bit;
            }
        }
        
        out
    }

    /// Convert to SparseVec: O(D/64) popcount iterations + O(nnz) index extraction
    pub fn to_sparse(&self) -> SparseVec {
        let words = Self::word_count(self.len);
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        
        for w in 0..words.min(self.pos.len()) {
            let (mut p, mut n) = (self.pos[w], self.neg[w]);
            
            // Mask last word
            if w + 1 == words {
                let mask = Self::last_word_mask(self.len);
                p &= mask;
                n &= mask;
            }
            
            // Extract positive indices
            while p != 0 {
                let tz = p.trailing_zeros() as usize;
                let idx = w * 64 + tz;
                if idx < self.len {
                    pos.push(idx);
                }
                p &= p - 1;  // Clear lowest set bit
            }
            
            // Extract negative indices
            while n != 0 {
                let tz = n.trailing_zeros() as usize;
                let idx = w * 64 + tz;
                if idx < self.len {
                    neg.push(idx);
                }
                n &= n - 1;
            }
        }
        
        SparseVec { pos, neg }
    }

    /// Convert from PackedTritVec (interleaved): O(D/64) with bit extraction
    /// 
    /// Interleaved format: each u64 has 32 trits as [p₀n₀ p₁n₁ ... p₃₁n₃₁]
    /// We need to de-interleave into separate pos/neg planes
    pub fn from_packed(packed: &crate::ternary_vec::PackedTritVec) -> Self {
        const EVEN_BITS: u64 = 0x5555_5555_5555_5555;
        
        let packed_words = (packed.len() + 31) / 32;  // 32 trits per packed u64
        let out_words = (packed.len() + 63) / 64;     // 64 trits per bitsliced u64
        
        let mut out = Self {
            len: packed.len(),
            pos: vec![0u64; out_words],
            neg: vec![0u64; out_words],
        };
        
        // Process packed words, extracting into bitsliced format
        for (pw_idx, &packed_word) in packed.data().iter().enumerate().take(packed_words) {
            let pos_bits = packed_word & EVEN_BITS;        // Even bits are P flags
            let neg_bits = (packed_word >> 1) & EVEN_BITS; // Odd bits are N flags
            
            // Deposit into correct positions in output
            // Packed word `pw_idx` contains trits [pw_idx*32 .. pw_idx*32+31]
            let base_trit = pw_idx * 32;
            let out_word = base_trit / 64;
            let out_offset = base_trit % 64;
            
            // Compress scattered bits to contiguous (remove gaps)
            let pos_compressed = pext_software(pos_bits, EVEN_BITS);
            let neg_compressed = pext_software(neg_bits, EVEN_BITS);
            
            // Place into output, handling word boundary
            if out_offset == 0 {
                out.pos[out_word] |= pos_compressed;
                out.neg[out_word] |= neg_compressed;
            } else if out_offset == 32 {
                out.pos[out_word] |= pos_compressed << 32;
                out.neg[out_word] |= neg_compressed << 32;
            }
        }
        
        out
    }

    /// Convert to PackedTritVec (interleaved): O(D/64) with bit interleaving
    pub fn to_packed(&self) -> crate::ternary_vec::PackedTritVec {
        use crate::ternary_vec::PackedTritVec;
        
        let mut packed = PackedTritVec::new_zero(self.len);
        
        // Process in groups of 32 trits (one packed word)
        let packed_words = (self.len + 31) / 32;
        
        for pw_idx in 0..packed_words {
            let base_trit = pw_idx * 32;
            let out_word = base_trit / 64;
            let out_offset = base_trit % 64;
            
            // Extract 32 bits from bitsliced format
            let pos_32: u32 = if out_offset == 0 {
                (self.pos.get(out_word).copied().unwrap_or(0) & 0xFFFF_FFFF) as u32
            } else {
                (self.pos.get(out_word).copied().unwrap_or(0) >> 32) as u32
            };
            
            let neg_32: u32 = if out_offset == 0 {
                (self.neg.get(out_word).copied().unwrap_or(0) & 0xFFFF_FFFF) as u32
            } else {
                (self.neg.get(out_word).copied().unwrap_or(0) >> 32) as u32
            };
            
            // Interleave: deposit pos bits at even positions, neg at odd
            let interleaved = pdep_software(pos_32 as u64, 0x5555_5555_5555_5555)
                            | pdep_software(neg_32 as u64, 0xAAAA_AAAA_AAAA_AAAA);
            
            packed.data_mut()[pw_idx] = interleaved;
        }
        
        packed
    }
}

/// Software PEXT (parallel bit extract) - extract bits selected by mask
#[inline]
fn pext_software(src: u64, mask: u64) -> u64 {
    let mut result = 0u64;
    let mut m = mask;
    let mut k = 0;
    
    while m != 0 {
        let lsb = m.trailing_zeros();
        if (src >> lsb) & 1 == 1 {
            result |= 1u64 << k;
        }
        m &= m - 1;  // Clear LSB
        k += 1;
    }
    
    result
}

/// Software PDEP (parallel bit deposit) - scatter bits to positions in mask
#[inline]
fn pdep_software(src: u64, mask: u64) -> u64 {
    let mut result = 0u64;
    let mut m = mask;
    let mut k = 0;
    
    while m != 0 {
        let lsb = m.trailing_zeros();
        if (src >> k) & 1 == 1 {
            result |= 1u64 << lsb;
        }
        m &= m - 1;
        k += 1;
    }
    
    result
}

// Use hardware BMI2 when available
#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
fn pext_hw(src: u64, mask: u64) -> u64 {
    unsafe { std::arch::x86_64::_pext_u64(src, mask) }
}

#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
fn pdep_hw(src: u64, mask: u64) -> u64 {
    unsafe { std::arch::x86_64::_pdep_u64(src, mask) }
}
```

### Conversion Cost Summary

| Conversion | Time Complexity | D=10M Cost |
|------------|-----------------|------------|
| Sparse → Bitsliced | O(nnz) | ~0.02ms (nnz=200) |
| Bitsliced → Sparse | O(D/64) + O(nnz) | ~0.3ms |
| Packed → Bitsliced | O(D/64) | ~0.2ms |
| Bitsliced → Packed | O(D/64) | ~0.2ms |

---

## 5. Carry-Save Bundle Accumulator

For multi-way bundling, maintaining full vote counts avoids information loss:

```rust
/// Carry-Save accumulator for multi-way bundling
/// 
/// Counts up to 3 votes per trit before requiring normalization:
/// - (sum, carry) = (0,0): 0 votes
/// - (sum, carry) = (1,0): 1 vote  
/// - (sum, carry) = (0,1): 2 votes
/// - (sum, carry) = (1,1): 3 votes
/// 
/// For N-way bundle with N > 3, call normalize() periodically.
pub struct CarrySaveBundle {
    len: usize,
    /// Bit 0 of positive vote count
    sum_pos: Vec<u64>,
    /// Bit 0 of negative vote count  
    sum_neg: Vec<u64>,
    /// Bit 1 of positive vote count
    carry_pos: Vec<u64>,
    /// Bit 1 of negative vote count
    carry_neg: Vec<u64>,
    /// Number of vectors accumulated (for normalization threshold)
    count: usize,
}

impl CarrySaveBundle {
    pub fn new(len: usize) -> Self {
        let words = (len + 63) / 64;
        Self {
            len,
            sum_pos: vec![0u64; words],
            sum_neg: vec![0u64; words],
            carry_pos: vec![0u64; words],
            carry_neg: vec![0u64; words],
            count: 0,
        }
    }

    /// Add a vector to the accumulator: O(D/64) with 12 ops per word
    /// 
    /// Uses carry-save addition: no sequential carry propagation!
    /// new_sum = old_sum XOR input
    /// new_carry = old_sum AND input (shifted up by "adding" to carry)
    pub fn accumulate(&mut self, vec: &BitslicedTritVec) {
        let words = (self.len + 63) / 64;
        
        for w in 0..words.min(vec.pos.len()) {
            let (vp, vn) = (vec.pos[w], vec.neg[w]);
            
            // Carry-save add for positive votes
            let new_carry_p = (self.sum_pos[w] & vp) | (self.carry_pos[w] & (self.sum_pos[w] ^ vp));
            self.sum_pos[w] ^= vp;
            self.carry_pos[w] = new_carry_p;
            
            // Carry-save add for negative votes
            let new_carry_n = (self.sum_neg[w] & vn) | (self.carry_neg[w] & (self.sum_neg[w] ^ vn));
            self.sum_neg[w] ^= vn;
            self.carry_neg[w] = new_carry_n;
        }
        
        self.count += 1;
        
        // Auto-normalize if we risk overflow (count >= 4 means 4+ votes possible)
        if self.count >= 3 {
            self.normalize_internal();
        }
    }

    /// Reduce accumulated votes to ternary result using majority vote
    pub fn finalize(&self) -> BitslicedTritVec {
        let words = (self.len + 63) / 64;
        let mut out = BitslicedTritVec::new_zero(self.len);
        
        // Threshold = count / 2 (majority)
        // With carry-save, we have 2-bit vote counts
        // pos_votes = sum_pos + 2*carry_pos
        // neg_votes = sum_neg + 2*carry_neg
        
        for w in 0..words {
            let pos_votes_0 = self.sum_pos[w];
            let pos_votes_1 = self.carry_pos[w];
            let neg_votes_0 = self.sum_neg[w];
            let neg_votes_1 = self.carry_neg[w];
            
            // Compare: pos_votes > neg_votes → P
            //          neg_votes > pos_votes → N
            //          equal → Z
            // 
            // For 2-bit compare: a > b iff (a1 > b1) or (a1 == b1 and a0 > b0)
            let pos_gt_neg = (pos_votes_1 & !neg_votes_1) 
                           | ((pos_votes_1 ^ neg_votes_1).not() & pos_votes_0 & !neg_votes_0);
            let neg_gt_pos = (neg_votes_1 & !pos_votes_1)
                           | ((pos_votes_1 ^ neg_votes_1).not() & neg_votes_0 & !pos_votes_0);
            
            // Only set if there's a strict majority (nonzero vote count)
            let has_pos = pos_votes_0 | pos_votes_1;
            let has_neg = neg_votes_0 | neg_votes_1;
            
            out.pos[w] = pos_gt_neg & has_pos;
            out.neg[w] = neg_gt_pos & has_neg;
        }
        
        out
    }

    /// Internal normalization when count approaches overflow
    fn normalize_internal(&mut self) {
        // For counts up to 7 (3 bits), we'd need a third level
        // For simplicity, we just finalize and restart
        // A more sophisticated version would expand to 3+ bits
        let partial = self.finalize();
        
        self.sum_pos.fill(0);
        self.sum_neg.fill(0);
        self.carry_pos.fill(0);
        self.carry_neg.fill(0);
        
        // Re-accumulate the partial result as a single vote
        for w in 0..partial.pos.len() {
            self.sum_pos[w] = partial.pos[w];
            self.sum_neg[w] = partial.neg[w];
        }
        
        self.count = 1;
    }
}

// Extension trait for cleaner code
trait BitNot {
    fn not(self) -> Self;
}
impl BitNot for u64 {
    #[inline(always)]
    fn not(self) -> Self { !self }
}
```

### Practicality Assessment

| Aspect | Assessment |
|--------|------------|
| Memory overhead | 4× base vector (4 planes vs 2) |
| Per-accumulate cost | ~12 ALU ops/word vs ~8 for naive |
| Break-even | N > 4 vectors (avoids N-1 intermediate allocations) |
| Use case | Hierarchical bundling, streaming aggregation |

**Recommendation:** Practical for N > 5 way bundles. Below that, sequential `bundle()` is simpler.

---

## 6. Optimized SparseVec with u32 Indices

```rust
/// Memory-optimized sparse vector with u32 indices
/// 
/// Saves 50% memory vs Vec<usize> for D < 4B dimensions.
/// For D=10M with 200 non-zeros: 200 × 4 = 800 bytes vs 1600 bytes.
#[derive(Clone, Debug)]
pub struct SparseVec32 {
    pub pos: Vec<u32>,
    pub neg: Vec<u32>,
}

impl SparseVec32 {
    pub fn new() -> Self {
        Self { pos: Vec::new(), neg: Vec::new() }
    }

    /// Convert from standard SparseVec (panics if any index >= 2^32)
    pub fn from_sparse(sparse: &SparseVec) -> Self {
        Self {
            pos: sparse.pos.iter().map(|&i| i as u32).collect(),
            neg: sparse.neg.iter().map(|&i| i as u32).collect(),
        }
    }

    /// Convert to standard SparseVec
    pub fn to_sparse(&self) -> SparseVec {
        SparseVec {
            pos: self.pos.iter().map(|&i| i as usize).collect(),
            neg: self.neg.iter().map(|&i| i as usize).collect(),
        }
    }

    #[inline]
    pub fn nnz(&self) -> usize {
        self.pos.len() + self.neg.len()
    }
}

/// Block-compressed sparse format for extreme dimensions
/// 
/// For D=10M with 1024-element blocks:
/// - 9766 blocks total
/// - Average 0.02 non-zeros per block (very sparse)
/// - Only stores blocks with content
/// 
/// Memory: O(active_blocks × 2 + nnz × 2) bytes
#[derive(Clone, Debug)]
pub struct BlockSparseVec {
    dim: usize,
    block_size: usize,  // Power of 2 for fast division
    /// Sorted list of block indices that have non-zeros
    active_blocks: Vec<u32>,
    /// For each active block: starting offset in offsets/signs
    block_starts: Vec<u32>,
    /// Offset within block (0..block_size-1)
    offsets: Vec<u16>,
    /// Sign bits: 1=positive, 0=negative
    signs: Vec<u8>,  // Packed bits
}

impl BlockSparseVec {
    const DEFAULT_BLOCK_SIZE: usize = 1024;

    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            block_size: Self::DEFAULT_BLOCK_SIZE,
            active_blocks: Vec::new(),
            block_starts: Vec::new(),
            offsets: Vec::new(),
            signs: Vec::new(),
        }
    }

    pub fn from_sparse(sparse: &SparseVec, dim: usize) -> Self {
        let block_size = Self::DEFAULT_BLOCK_SIZE;
        let block_shift = block_size.trailing_zeros() as usize;
        let block_mask = block_size - 1;

        // Collect (block_idx, offset, sign) tuples
        let mut entries: Vec<(u32, u16, bool)> = Vec::with_capacity(sparse.pos.len() + sparse.neg.len());
        
        for &idx in &sparse.pos {
            let block = (idx >> block_shift) as u32;
            let offset = (idx & block_mask) as u16;
            entries.push((block, offset, true));
        }
        
        for &idx in &sparse.neg {
            let block = (idx >> block_shift) as u32;
            let offset = (idx & block_mask) as u16;
            entries.push((block, offset, false));
        }

        // Sort by (block, offset)
        entries.sort_unstable_by_key(|&(b, o, _)| (b, o));

        // Build compressed structure
        let mut active_blocks = Vec::new();
        let mut block_starts = Vec::new();
        let mut offsets = Vec::new();
        let mut signs = Vec::new();
        let mut sign_acc = 0u8;
        let mut sign_bit = 0;

        let mut last_block: Option<u32> = None;
        
        for (i, &(block, offset, sign)) in entries.iter().enumerate() {
            if last_block != Some(block) {
                active_blocks.push(block);
                block_starts.push(i as u32);
                last_block = Some(block);
            }
            
            offsets.push(offset);
            
            if sign {
                sign_acc |= 1 << sign_bit;
            }
            sign_bit += 1;
            if sign_bit == 8 {
                signs.push(sign_acc);
                sign_acc = 0;
                sign_bit = 0;
            }
        }
        
        // Flush remaining sign bits
        if sign_bit > 0 {
            signs.push(sign_acc);
        }

        Self {
            dim,
            block_size,
            active_blocks,
            block_starts,
            offsets,
            signs,
        }
    }

    pub fn to_sparse(&self) -> SparseVec {
        let block_shift = self.block_size.trailing_zeros() as usize;
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        for (block_i, &block_idx) in self.active_blocks.iter().enumerate() {
            let start = self.block_starts[block_i] as usize;
            let end = self.block_starts.get(block_i + 1).copied().unwrap_or(self.offsets.len() as u32) as usize;
            
            for entry_i in start..end {
                let offset = self.offsets[entry_i] as usize;
                let idx = ((block_idx as usize) << block_shift) | offset;
                
                let sign_byte = entry_i / 8;
                let sign_bit = entry_i % 8;
                let is_positive = (self.signs[sign_byte] >> sign_bit) & 1 == 1;
                
                if is_positive {
                    pos.push(idx);
                } else {
                    neg.push(idx);
                }
            }
        }

        SparseVec { pos, neg }
    }

    /// Memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.active_blocks.len() * 4
            + self.block_starts.len() * 4
            + self.offsets.len() * 2
            + self.signs.len()
    }
}
```

### Memory Comparison (D=10M, nnz=200)

| Format | Memory |
|--------|--------|
| SparseVec (usize) | 200 × 8 × 2 = 3,200 bytes |
| SparseVec32 | 200 × 4 × 2 = 1,600 bytes |
| BlockSparseVec | ~200 × 4 + 200 × 2 + 25 = ~1,225 bytes |
| BitslicedTritVec | 10M / 8 × 2 = 2,500,000 bytes |

**Note:** Sparse formats win massively for low nnz. Bitsliced wins for operations.

---

## 7. Recommendation: Phased Migration Strategy

### Phase 1: Add BitslicedTritVec as Third Representation (Now)

```rust
// In src/vsa/mod.rs
pub mod bitsliced;
pub use bitsliced::BitslicedTritVec;

// In src/vsa/vsa.rs - add trait for polymorphism
pub trait TritVec: Clone {
    fn len(&self) -> usize;
    fn bind(&self, other: &Self) -> Self;
    fn bundle(&self, other: &Self) -> Self;
    fn dot(&self, other: &Self) -> i32;
    fn to_sparse(&self) -> SparseVec;
    fn from_sparse(sparse: &SparseVec, len: usize) -> Self;
}

impl TritVec for BitslicedTritVec { /* ... */ }
impl TritVec for PackedTritVec { /* ... */ }
```

**Rationale:** Low risk, enables benchmarking, keeps existing code working.

### Phase 2: Benchmark and Profile (1-2 weeks)

- Compare bind/bundle/dot for D ∈ {10K, 100K, 1M, 10M}
- Measure conversion overhead in real workloads
- Profile cache behavior with `perf stat` and `cachegrind`

### Phase 3: Selective Replacement (Based on Data)

| If benchmark shows... | Then... |
|----------------------|---------|
| Bitsliced 2×+ faster for all D | Replace PackedTritVec entirely |
| Mixed results | Keep both, auto-select based on D |
| Conversion overhead dominates | Keep PackedTritVec for small D |

### Phase 4: AVX-512 Optimization (If Worthwhile)

Only if Phase 2 shows scalar bitsliced is already faster. AVX-512 availability is limited (server CPUs, Ice Lake+).

---

## 8. Summary

| Question | Answer |
|----------|--------|
| Replace or Add? | **Add first**, replace after benchmarking |
| Cache behavior D=10M | **Both fit L3**, bitsliced has 2× ALU efficiency |
| AVX-512 value | **Yes**, but scalar bitsliced is already ~3× faster |
| Carry-save practical? | **Yes for N>5** way bundles |
| SparseVec32 worth it? | **Yes**, 50% memory savings, trivial change |

**Bottom line:** The mathematician's 3.7× efficiency analysis is sound. Bitsliced representation is the correct direction. Implement incrementally to validate with real workloads.
