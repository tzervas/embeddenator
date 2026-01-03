//! Bitsliced Ternary Vector - High-throughput VSA Operations
//!
//! This module implements a bitsliced representation for balanced ternary vectors,
//! optimized for SIMD-friendly bulk operations. Compared to interleaved 2-bit
//! encoding, bitslicing processes 2× the trits per instruction by separating
//! positive and negative bit-planes.
//!
//! # Representation
//!
//! ```text
//! BitslicedTritVec:
//!   pos: Vec<u64>  ──→  [p₀p₁p₂...p₆₃|p₆₄p₆₅...p₁₂₇|...]
//!   neg: Vec<u64>  ──→  [n₀n₁n₂...n₆₃|n₆₄n₆₅...n₁₂₇|...]
//!
//! Trit encoding:
//!   - pos[i/64] bit (i%64) = 1, neg[i/64] bit (i%64) = 0  →  +1 (P)
//!   - pos[i/64] bit (i%64) = 0, neg[i/64] bit (i%64) = 1  →  -1 (N)
//!   - pos[i/64] bit (i%64) = 0, neg[i/64] bit (i%64) = 0  →   0 (Z)
//!   - pos[i/64] bit (i%64) = 1, neg[i/64] bit (i%64) = 1  →  invalid (treated as Z)
//! ```
//!
//! # Performance
//!
//! | Operation | Interleaved | Bitsliced | Speedup |
//! |-----------|-------------|-----------|---------|
//! | Bind      | 11 ops/32 trits | 6 ops/64 trits | ~3.7× |
//! | Bundle    | 13 ops/32 trits | 8 ops/64 trits | ~3.2× |
//! | Dot       | 12 ops/32 trits | 8 ops/64 trits | ~3.0× |
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator::BitslicedTritVec;
//!
//! let a = BitslicedTritVec::random(10000);
//! let b = BitslicedTritVec::random(10000);
//!
//! let bound = a.bind(&b);       // Element-wise multiply
//! let bundled = a.bundle(&b);   // Element-wise saturating add
//! let similarity = a.cosine(&b); // Normalized dot product
//! ```

use crate::ternary::Trit;
use crate::vsa::SparseVec;

/// Bitsliced ternary vector for maximum throughput VSA operations.
///
/// Uses separate bit-planes for positive and negative components,
/// enabling efficient SIMD parallelization of all VSA primitives.
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
    // ========================================================================
    // CONSTRUCTION
    // ========================================================================

    /// Create zero vector of given length.
    #[inline]
    pub fn new_zero(len: usize) -> Self {
        let words = Self::word_count(len);
        Self {
            len,
            pos: vec![0u64; words],
            neg: vec![0u64; words],
        }
    }

    /// Create from raw bit-planes (advanced use).
    ///
    /// # Safety
    /// Caller must ensure pos and neg have correct length and no overlapping bits.
    #[inline]
    pub fn from_raw(len: usize, pos: Vec<u64>, neg: Vec<u64>) -> Self {
        debug_assert_eq!(pos.len(), Self::word_count(len));
        debug_assert_eq!(neg.len(), Self::word_count(len));
        Self { len, pos, neg }
    }

    /// Number of u64 words needed for `len` trits.
    #[inline(always)]
    pub const fn word_count(len: usize) -> usize {
        (len + 63) / 64
    }

    /// Mask for valid bits in the last word.
    #[inline(always)]
    const fn last_word_mask(len: usize) -> u64 {
        let bits_used = len % 64;
        if bits_used == 0 {
            !0u64
        } else {
            (1u64 << bits_used) - 1
        }
    }

    // ========================================================================
    // ACCESSORS
    // ========================================================================

    /// Number of trits in this vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the vector is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Read-only access to positive bit-plane.
    #[inline]
    pub fn pos_plane(&self) -> &[u64] {
        &self.pos
    }

    /// Read-only access to negative bit-plane.
    #[inline]
    pub fn neg_plane(&self) -> &[u64] {
        &self.neg
    }

    /// Get single trit by index (bounds-checked).
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

    /// Set single trit by index (bounds-checked).
    #[inline]
    pub fn set(&mut self, i: usize, t: Trit) {
        if i >= self.len {
            return;
        }
        let word = i / 64;
        let bit = i % 64;
        let mask = 1u64 << bit;

        // Clear both bits
        self.pos[word] &= !mask;
        self.neg[word] &= !mask;

        // Set appropriate bit
        match t {
            Trit::P => self.pos[word] |= mask,
            Trit::N => self.neg[word] |= mask,
            Trit::Z => {}
        }
    }

    /// Count non-zero trits.
    #[inline]
    pub fn nnz(&self) -> usize {
        let words = Self::word_count(self.len);
        let mut count = 0usize;

        for w in 0..words.min(self.pos.len()) {
            let (mut p, mut n) = (self.pos[w], self.neg[w]);

            // Mask last word
            if w + 1 == words {
                let mask = Self::last_word_mask(self.len);
                p &= mask;
                n &= mask;
            }

            // Union (don't double-count invalid states)
            count += (p | n).count_ones() as usize;
        }

        count
    }

    // ========================================================================
    // CORE VSA OPERATIONS
    // ========================================================================

    /// Bind (element-wise multiplication): O(D/64) with minimal ops per word.
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
    /// Bitwise implementation:
    /// - `out_pos = (a_pos & b_pos) | (a_neg & b_neg)` (same signs → positive)
    /// - `out_neg = (a_pos & b_neg) | (a_neg & b_pos)` (different signs → negative)
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

    /// Bind into pre-allocated output (avoids allocation in hot loops).
    #[inline]
    pub fn bind_into(&self, other: &Self, out: &mut Self) {
        let n = self.len.min(other.len);
        let words = Self::word_count(n);

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

    /// Bundle (element-wise saturating addition) for two vectors.
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
    /// Bitwise implementation:
    /// - `out_pos = (a_pos & !b_neg) | (b_pos & !a_neg)`
    /// - `out_neg = (a_neg & !b_pos) | (b_neg & !a_pos)`
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

    /// Bundle into pre-allocated output.
    #[inline]
    pub fn bundle_into(&self, other: &Self, out: &mut Self) {
        let n = self.len.min(other.len);
        let words = Self::word_count(n);

        out.len = n;
        out.pos.resize(words, 0);
        out.neg.resize(words, 0);

        for w in 0..words.min(self.pos.len()).min(other.pos.len()) {
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
    }

    /// Dot product: count matching signs minus opposing signs.
    ///
    /// `dot(a, b) = Σᵢ aᵢ × bᵢ`
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

            let pp = (ap & bp).count_ones() as i32; // +1 × +1 = +1
            let nn = (an & bn).count_ones() as i32; // -1 × -1 = +1
            let pn = (ap & bn).count_ones() as i32; // +1 × -1 = -1
            let np = (an & bp).count_ones() as i32; // -1 × +1 = -1

            acc += (pp + nn) - (pn + np);
        }

        acc
    }

    /// Cosine similarity: normalized dot product.
    ///
    /// `cosine(a, b) = dot(a, b) / (||a|| × ||b||)`
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

    /// Permute (cyclic shift) for sequence encoding.
    ///
    /// `permute(v, k)[i] = v[(i - k) mod len]`
    pub fn permute(&self, shift: usize) -> Self {
        if self.len == 0 || shift == 0 {
            return self.clone();
        }

        let shift = shift % self.len;
        let mut out = Self::new_zero(self.len);

        // TODO: Optimize with word-level rotation for large shifts
        for i in 0..self.len {
            let src_idx = (i + self.len - shift) % self.len;
            out.set(i, self.get(src_idx));
        }

        out
    }

    /// Negate all trits (swap positive and negative planes).
    #[inline]
    pub fn negate(&self) -> Self {
        Self {
            len: self.len,
            pos: self.neg.clone(),
            neg: self.pos.clone(),
        }
    }

    /// Negate in place.
    #[inline]
    pub fn negate_in_place(&mut self) {
        std::mem::swap(&mut self.pos, &mut self.neg);
    }

    // ========================================================================
    // CONVERSIONS
    // ========================================================================

    /// Convert from SparseVec: O(nnz).
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

    /// Convert to SparseVec: O(D/64) + O(nnz).
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
                p &= p - 1; // Clear lowest set bit
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

    /// Convert from PackedTritVec (interleaved 2-bit encoding).
    ///
    /// The interleaved format stores 32 trits per u64 as `[p₀n₀ p₁n₁ ... p₃₁n₃₁]`.
    /// This extracts and separates the bit-planes.
    pub fn from_packed(packed: &crate::ternary_vec::PackedTritVec) -> Self {
        const EVEN_BITS: u64 = 0x5555_5555_5555_5555;

        let packed_words = (packed.len() + 31) / 32;
        let out_words = Self::word_count(packed.len());

        let mut out = Self {
            len: packed.len(),
            pos: vec![0u64; out_words],
            neg: vec![0u64; out_words],
        };

        for (pw_idx, &packed_word) in packed.data().iter().enumerate().take(packed_words) {
            let pos_bits = packed_word & EVEN_BITS;
            let neg_bits = (packed_word >> 1) & EVEN_BITS;

            let base_trit = pw_idx * 32;
            let out_word = base_trit / 64;
            let out_offset = base_trit % 64;

            // Compress scattered bits to contiguous
            let pos_compressed = pext_u64(pos_bits, EVEN_BITS);
            let neg_compressed = pext_u64(neg_bits, EVEN_BITS);

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

    /// Convert to PackedTritVec (interleaved 2-bit encoding).
    pub fn to_packed(&self) -> crate::ternary_vec::PackedTritVec {
        use crate::ternary_vec::PackedTritVec;

        let mut packed = PackedTritVec::new_zero(self.len);
        let packed_words = (self.len + 31) / 32;

        for pw_idx in 0..packed_words {
            let base_trit = pw_idx * 32;
            let out_word = base_trit / 64;
            let out_offset = base_trit % 64;

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

            // Interleave: pos at even positions, neg at odd
            let interleaved = pdep_u64(pos_32 as u64, 0x5555_5555_5555_5555)
                | pdep_u64(neg_32 as u64, 0xAAAA_AAAA_AAAA_AAAA);

            packed.data_mut()[pw_idx] = interleaved;
        }

        packed
    }
}

// ============================================================================
// BIT MANIPULATION HELPERS
// ============================================================================

/// Parallel bit extract (software fallback).
/// Extracts bits from `src` at positions marked by `mask` into contiguous low bits.
#[inline]
fn pext_u64(src: u64, mask: u64) -> u64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
    {
        return unsafe { std::arch::x86_64::_pext_u64(src, mask) };
    }

    #[allow(unreachable_code)]
    {
        let mut result = 0u64;
        let mut m = mask;
        let mut k = 0;

        while m != 0 {
            let lsb = m.trailing_zeros();
            if (src >> lsb) & 1 == 1 {
                result |= 1u64 << k;
            }
            m &= m - 1;
            k += 1;
        }

        result
    }
}

/// Parallel bit deposit (software fallback).
/// Deposits contiguous low bits of `src` to positions marked by `mask`.
#[inline]
fn pdep_u64(src: u64, mask: u64) -> u64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
    {
        return unsafe { std::arch::x86_64::_pdep_u64(src, mask) };
    }

    #[allow(unreachable_code)]
    {
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
}

// ============================================================================
// CARRY-SAVE BUNDLE ACCUMULATOR
// ============================================================================

/// Carry-Save accumulator for efficient multi-way bundling.
///
/// Maintains 2-bit vote counts per trit position, allowing up to 3 vectors
/// to be accumulated before requiring normalization. This avoids repeated
/// allocation of intermediate results in N-way bundles.
///
/// # Example
///
/// ```rust,ignore
/// let mut acc = CarrySaveBundle::new(10000);
/// for vec in vectors.iter() {
///     acc.accumulate(vec);
/// }
/// let result = acc.finalize(); // Majority vote
/// ```
#[derive(Clone, Debug)]
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
    /// Number of vectors accumulated
    count: usize,
}

impl CarrySaveBundle {
    /// Create new accumulator for given dimension.
    pub fn new(len: usize) -> Self {
        let words = BitslicedTritVec::word_count(len);
        Self {
            len,
            sum_pos: vec![0u64; words],
            sum_neg: vec![0u64; words],
            carry_pos: vec![0u64; words],
            carry_neg: vec![0u64; words],
            count: 0,
        }
    }

    /// Reset accumulator to zero state.
    pub fn reset(&mut self) {
        self.sum_pos.fill(0);
        self.sum_neg.fill(0);
        self.carry_pos.fill(0);
        self.carry_neg.fill(0);
        self.count = 0;
    }

    /// Add a vector to the accumulator using carry-save addition.
    ///
    /// This is O(D/64) with ~12 ALU ops per word, avoiding sequential carry
    /// propagation across words.
    pub fn accumulate(&mut self, vec: &BitslicedTritVec) {
        let words = BitslicedTritVec::word_count(self.len);

        for w in 0..words.min(vec.pos.len()) {
            let (vp, vn) = (vec.pos[w], vec.neg[w]);

            // Carry-save add for positive votes:
            // new_carry = (sum & input) | (carry & (sum ^ input))
            // new_sum = sum ^ input
            let new_carry_p =
                (self.sum_pos[w] & vp) | (self.carry_pos[w] & (self.sum_pos[w] ^ vp));
            self.sum_pos[w] ^= vp;
            self.carry_pos[w] = new_carry_p;

            // Same for negative votes
            let new_carry_n =
                (self.sum_neg[w] & vn) | (self.carry_neg[w] & (self.sum_neg[w] ^ vn));
            self.sum_neg[w] ^= vn;
            self.carry_neg[w] = new_carry_n;
        }

        self.count += 1;

        // Auto-normalize if we approach overflow (count >= 4 needs 3 bits)
        if self.count >= 3 {
            self.normalize_internal();
        }
    }

    /// Finalize accumulated votes to ternary result using majority vote.
    ///
    /// For each trit position:
    /// - If pos_votes > neg_votes → P
    /// - If neg_votes > pos_votes → N
    /// - Otherwise → Z
    pub fn finalize(&self) -> BitslicedTritVec {
        let words = BitslicedTritVec::word_count(self.len);
        let mut out = BitslicedTritVec::new_zero(self.len);

        for w in 0..words {
            let pos_0 = self.sum_pos.get(w).copied().unwrap_or(0);
            let pos_1 = self.carry_pos.get(w).copied().unwrap_or(0);
            let neg_0 = self.sum_neg.get(w).copied().unwrap_or(0);
            let neg_1 = self.carry_neg.get(w).copied().unwrap_or(0);

            // 2-bit comparison: a > b iff (a1 > b1) || (a1 == b1 && a0 > b0)
            // a1 > b1: pos_1 & !neg_1
            // a1 == b1: !(pos_1 ^ neg_1)
            // a0 > b0: pos_0 & !neg_0

            let pos_gt_neg =
                (pos_1 & !neg_1) | (!(pos_1 ^ neg_1) & pos_0 & !neg_0);

            let neg_gt_pos =
                (neg_1 & !pos_1) | (!(pos_1 ^ neg_1) & neg_0 & !pos_0);

            // Only set if there were actual votes
            let has_pos = pos_0 | pos_1;
            let has_neg = neg_0 | neg_1;

            out.pos[w] = pos_gt_neg & has_pos;
            out.neg[w] = neg_gt_pos & has_neg;
        }

        out
    }

    /// Number of vectors accumulated so far.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Internal normalization to prevent overflow.
    fn normalize_internal(&mut self) {
        let partial = self.finalize();

        self.sum_pos.fill(0);
        self.sum_neg.fill(0);
        self.carry_pos.fill(0);
        self.carry_neg.fill(0);

        // Re-add partial result as single vote
        for w in 0..partial.pos.len() {
            self.sum_pos[w] = partial.pos[w];
            self.sum_neg[w] = partial.neg[w];
        }

        self.count = 1;
    }
}

// ============================================================================
// SIMD ACCELERATION (Optional)
// ============================================================================

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub mod avx512 {
    //! AVX-512 accelerated operations for bitsliced vectors.
    //!
    //! These functions process 512 trits per iteration (8 × u64 per plane).

    use super::BitslicedTritVec;
    use std::arch::x86_64::*;

    /// AVX-512 bind: processes 512 trits per iteration.
    ///
    /// # Safety
    /// Requires AVX-512F support. Check with `is_x86_feature_detected!("avx512f")`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn bind_avx512(
        a: &BitslicedTritVec,
        b: &BitslicedTritVec,
        out: &mut BitslicedTritVec,
    ) {
        let n = a.len.min(b.len);
        let words = BitslicedTritVec::word_count(n);

        out.len = n;
        out.pos.resize(words, 0);
        out.neg.resize(words, 0);

        let chunks = words / 8;

        for chunk in 0..chunks {
            let offset = chunk * 8;

            let ap = _mm512_loadu_si512(a.pos.as_ptr().add(offset) as *const __m512i);
            let an = _mm512_loadu_si512(a.neg.as_ptr().add(offset) as *const __m512i);
            let bp = _mm512_loadu_si512(b.pos.as_ptr().add(offset) as *const __m512i);
            let bn = _mm512_loadu_si512(b.neg.as_ptr().add(offset) as *const __m512i);

            let same_pp = _mm512_and_si512(ap, bp);
            let same_nn = _mm512_and_si512(an, bn);
            let out_pos = _mm512_or_si512(same_pp, same_nn);

            let diff_pn = _mm512_and_si512(ap, bn);
            let diff_np = _mm512_and_si512(an, bp);
            let out_neg = _mm512_or_si512(diff_pn, diff_np);

            _mm512_storeu_si512(out.pos.as_mut_ptr().add(offset) as *mut __m512i, out_pos);
            _mm512_storeu_si512(out.neg.as_mut_ptr().add(offset) as *mut __m512i, out_neg);
        }

        // Scalar remainder
        for w in (chunks * 8)..words {
            let (ap, an) = (a.pos[w], a.neg[w]);
            let (bp, bn) = (b.pos[w], b.neg[w]);
            out.pos[w] = (ap & bp) | (an & bn);
            out.neg[w] = (ap & bn) | (an & bp);
        }
    }

    /// Check if AVX-512 is available at runtime.
    pub fn is_available() -> bool {
        is_x86_feature_detected!("avx512f")
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_set() {
        let mut v = BitslicedTritVec::new_zero(100);

        v.set(0, Trit::P);
        v.set(1, Trit::N);
        v.set(63, Trit::P);
        v.set(64, Trit::N);
        v.set(99, Trit::P);

        assert_eq!(v.get(0), Trit::P);
        assert_eq!(v.get(1), Trit::N);
        assert_eq!(v.get(2), Trit::Z);
        assert_eq!(v.get(63), Trit::P);
        assert_eq!(v.get(64), Trit::N);
        assert_eq!(v.get(99), Trit::P);
        assert_eq!(v.nnz(), 5);
    }

    #[test]
    fn test_bind() {
        let mut a = BitslicedTritVec::new_zero(10);
        let mut b = BitslicedTritVec::new_zero(10);

        // P × P = P
        a.set(0, Trit::P);
        b.set(0, Trit::P);

        // P × N = N
        a.set(1, Trit::P);
        b.set(1, Trit::N);

        // N × N = P
        a.set(2, Trit::N);
        b.set(2, Trit::N);

        // P × Z = Z
        a.set(3, Trit::P);
        b.set(3, Trit::Z);

        let c = a.bind(&b);

        assert_eq!(c.get(0), Trit::P);
        assert_eq!(c.get(1), Trit::N);
        assert_eq!(c.get(2), Trit::P);
        assert_eq!(c.get(3), Trit::Z);
    }

    #[test]
    fn test_bundle() {
        let mut a = BitslicedTritVec::new_zero(10);
        let mut b = BitslicedTritVec::new_zero(10);

        // P + P = P (saturated)
        a.set(0, Trit::P);
        b.set(0, Trit::P);

        // P + N = Z
        a.set(1, Trit::P);
        b.set(1, Trit::N);

        // P + Z = P
        a.set(2, Trit::P);
        b.set(2, Trit::Z);

        // N + N = N (saturated)
        a.set(3, Trit::N);
        b.set(3, Trit::N);

        let c = a.bundle(&b);

        assert_eq!(c.get(0), Trit::P);
        assert_eq!(c.get(1), Trit::Z);
        assert_eq!(c.get(2), Trit::P);
        assert_eq!(c.get(3), Trit::N);
    }

    #[test]
    fn test_dot() {
        let mut a = BitslicedTritVec::new_zero(10);
        let mut b = BitslicedTritVec::new_zero(10);

        a.set(0, Trit::P);
        b.set(0, Trit::P); // +1

        a.set(1, Trit::P);
        b.set(1, Trit::N); // -1

        a.set(2, Trit::N);
        b.set(2, Trit::N); // +1

        // Total: +1 - 1 + 1 = 1
        assert_eq!(a.dot(&b), 1);
    }

    #[test]
    fn test_sparse_roundtrip() {
        let sparse = SparseVec {
            pos: vec![0, 5, 63, 64, 127],
            neg: vec![1, 10, 100],
        };

        let bitsliced = BitslicedTritVec::from_sparse(&sparse, 200);
        let back = bitsliced.to_sparse();

        assert_eq!(back.pos, sparse.pos);
        assert_eq!(back.neg, sparse.neg);
    }

    #[test]
    fn test_carry_save_bundle() {
        let dim = 100;
        let mut acc = CarrySaveBundle::new(dim);

        // Create 3 vectors that vote for P at position 0
        for _ in 0..3 {
            let mut v = BitslicedTritVec::new_zero(dim);
            v.set(0, Trit::P);
            acc.accumulate(&v);
        }

        let result = acc.finalize();
        assert_eq!(result.get(0), Trit::P);
    }
}
