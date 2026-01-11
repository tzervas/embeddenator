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
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

// ============================================================================
// CPU FEATURE DETECTION (Runtime)
// ============================================================================

/// Cached AVX-512 detection result.
/// 0 = not checked, 1 = not available, 2 = available
static AVX512_AVAILABLE: AtomicU8 = AtomicU8::new(0);

/// Cached AVX2 detection result.
static AVX2_AVAILABLE: AtomicU8 = AtomicU8::new(0);

/// Check if AVX-512F is available at runtime (cached after first call).
///
/// This enables automatic dispatch to SIMD-optimized code paths without
/// requiring compile-time feature flags.
#[inline]
pub fn has_avx512() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        match AVX512_AVAILABLE.load(Ordering::Relaxed) {
            0 => {
                let available = std::arch::is_x86_feature_detected!("avx512f");
                AVX512_AVAILABLE.store(if available { 2 } else { 1 }, Ordering::Relaxed);
                available
            }
            2 => true,
            _ => false,
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if AVX2 is available at runtime (cached after first call).
#[inline]
pub fn has_avx2() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        match AVX2_AVAILABLE.load(Ordering::Relaxed) {
            0 => {
                let available = std::arch::is_x86_feature_detected!("avx2");
                AVX2_AVAILABLE.store(if available { 2 } else { 1 }, Ordering::Relaxed);
                available
            }
            2 => true,
            _ => false,
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Get a human-readable string describing available SIMD features.
pub fn simd_features_string() -> String {
    let mut features = Vec::new();
    if has_avx512() {
        features.push("AVX-512");
    }
    if has_avx2() {
        features.push("AVX2");
    }
    if features.is_empty() {
        "scalar only".to_string()
    } else {
        features.join(", ")
    }
}

// ============================================================================
// BITSLICED TERNARY VECTOR
// ============================================================================

/// Bitsliced ternary vector for maximum throughput VSA operations.
///
/// Uses separate bit-planes for positive and negative components,
/// enabling efficient SIMD parallelization of all VSA primitives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        len.div_ceil(64)
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

    /// Get a single word from positive plane (for soft ternary operations).
    #[inline]
    pub fn pos_word(&self, word_idx: usize) -> u64 {
        self.pos.get(word_idx).copied().unwrap_or(0)
    }

    /// Get a single word from negative plane (for soft ternary operations).
    #[inline]
    pub fn neg_word(&self, word_idx: usize) -> u64 {
        self.neg.get(word_idx).copied().unwrap_or(0)
    }

    /// Set a word in positive plane (advanced use).
    #[inline]
    pub fn set_pos_word(&mut self, word_idx: usize, value: u64) {
        if word_idx < self.pos.len() {
            self.pos[word_idx] = value;
        }
    }

    /// Set a word in negative plane (advanced use).
    #[inline]
    pub fn set_neg_word(&mut self, word_idx: usize, value: u64) {
        if word_idx < self.neg.len() {
            self.neg[word_idx] = value;
        }
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

        if words == 0 {
            return Self::new_zero(n);
        }

        let mut out = Self::new_zero(n);
        let last = words - 1;

        // Process all but last word without masking
        for w in 0..last {
            // Safety: w < last < words
            let (ap, an) = unsafe { (*self.pos.get_unchecked(w), *self.neg.get_unchecked(w)) };
            let (bp, bn) = unsafe { (*other.pos.get_unchecked(w), *other.neg.get_unchecked(w)) };

            unsafe {
                *out.pos.get_unchecked_mut(w) = (ap & bp) | (an & bn);
                *out.neg.get_unchecked_mut(w) = (ap & bn) | (an & bp);
            }
        }

        // Last word with masking
        let mask = Self::last_word_mask(n);
        let (ap, an) = unsafe { (*self.pos.get_unchecked(last), *self.neg.get_unchecked(last)) };
        let (bp, bn) = unsafe {
            (
                *other.pos.get_unchecked(last),
                *other.neg.get_unchecked(last),
            )
        };

        unsafe {
            *out.pos.get_unchecked_mut(last) = ((ap & bp) | (an & bn)) & mask;
            *out.neg.get_unchecked_mut(last) = ((ap & bn) | (an & bp)) & mask;
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

        if words == 0 {
            return Self::new_zero(n);
        }

        let mut out = Self::new_zero(n);
        let last = words - 1;

        // Process all but last word without masking
        for w in 0..last {
            // Safety: w < last < words
            let (ap, an) = unsafe { (*self.pos.get_unchecked(w), *self.neg.get_unchecked(w)) };
            let (bp, bn) = unsafe { (*other.pos.get_unchecked(w), *other.neg.get_unchecked(w)) };

            unsafe {
                *out.pos.get_unchecked_mut(w) = (ap & !bn) | (bp & !an);
                *out.neg.get_unchecked_mut(w) = (an & !bp) | (bn & !ap);
            }
        }

        // Last word with masking
        let mask = Self::last_word_mask(n);
        let (ap, an) = unsafe { (*self.pos.get_unchecked(last), *self.neg.get_unchecked(last)) };
        let (bp, bn) = unsafe {
            (
                *other.pos.get_unchecked(last),
                *other.neg.get_unchecked(last),
            )
        };

        unsafe {
            *out.pos.get_unchecked_mut(last) = ((ap & !bn) | (bp & !an)) & mask;
            *out.neg.get_unchecked_mut(last) = ((an & !bp) | (bn & !ap)) & mask;
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

    // ========================================================================
    // SIMD-DISPATCHING OPERATIONS
    // ========================================================================

    /// Bind with automatic SIMD dispatch.
    ///
    /// Automatically selects AVX-512 path when:
    /// 1. Running on x86_64 with AVX-512F support
    /// 2. Vector length >= 512 trits (worthwhile for SIMD overhead)
    ///
    /// Falls back to scalar implementation otherwise.
    #[inline]
    pub fn bind_dispatch(&self, other: &Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
        {
            if has_avx512() && self.len >= 512 {
                let mut out = Self::new_zero(self.len.min(other.len));
                // Safety: We verified AVX-512F support via runtime detection
                unsafe { avx512::bind_avx512(self, other, &mut out) };
                return out;
            }
        }
        // Scalar fallback
        self.bind(other)
    }

    /// Bundle with automatic SIMD dispatch.
    ///
    /// Automatically selects AVX-512 path when available and beneficial.
    #[inline]
    pub fn bundle_dispatch(&self, other: &Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
        {
            if has_avx512() && self.len >= 512 {
                let mut out = Self::new_zero(self.len.min(other.len));
                // Safety: We verified AVX-512F support via runtime detection
                unsafe { avx512::bundle_avx512(self, other, &mut out) };
                return out;
            }
        }
        // Scalar fallback
        self.bundle(other)
    }

    /// Dot product with automatic SIMD dispatch.
    #[inline]
    pub fn dot_dispatch(&self, other: &Self) -> i32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
        {
            if has_avx512() && self.len >= 512 {
                // Safety: We verified AVX-512F support via runtime detection
                return unsafe { avx512::dot_avx512(self, other) };
            }
        }
        // Scalar fallback
        self.dot(other)
    }

    // ========================================================================
    // DOT PRODUCT AND SIMILARITY
    // ========================================================================

    /// Dot product: count matching signs minus opposing signs.
    ///
    /// `dot(a, b) = Σᵢ aᵢ × bᵢ`
    ///
    /// Optimized: avoids branch per word for mask, uses unsafe slice access
    /// for bounds-checked loop elimination.
    #[inline]
    pub fn dot(&self, other: &Self) -> i32 {
        let n = self.len.min(other.len);
        let words = Self::word_count(n).min(self.pos.len()).min(other.pos.len());

        if words == 0 {
            return 0;
        }

        let mut acc: i32 = 0;
        let last = words - 1;
        let mask = Self::last_word_mask(n);

        // Process all but last word without masking
        for w in 0..last {
            // Safety: w < last < words <= self.pos.len() (and same for other)
            let (ap, an) = unsafe { (*self.pos.get_unchecked(w), *self.neg.get_unchecked(w)) };
            let (bp, bn) = unsafe { (*other.pos.get_unchecked(w), *other.neg.get_unchecked(w)) };

            let pp = (ap & bp).count_ones();
            let nn = (an & bn).count_ones();
            let pn = (ap & bn).count_ones();
            let np = (an & bp).count_ones();

            acc += (pp + nn) as i32 - (pn + np) as i32;
        }

        // Last word with masking
        let (ap, an) = unsafe {
            (
                *self.pos.get_unchecked(last) & mask,
                *self.neg.get_unchecked(last) & mask,
            )
        };
        let (bp, bn) = unsafe {
            (
                *other.pos.get_unchecked(last) & mask,
                *other.neg.get_unchecked(last) & mask,
            )
        };

        let pp = (ap & bp).count_ones();
        let nn = (an & bn).count_ones();
        let pn = (ap & bn).count_ones();
        let np = (an & bp).count_ones();

        acc + (pp + nn) as i32 - (pn + np) as i32
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
    ///
    /// Note: This is the naive O(D) implementation. For large vectors,
    /// use `permute_optimized` which achieves O(D/64) via word rotation.
    pub fn permute(&self, shift: usize) -> Self {
        if self.len == 0 || shift == 0 {
            return self.clone();
        }

        let shift = shift % self.len;
        let mut out = Self::new_zero(self.len);

        for i in 0..self.len {
            let src_idx = (i + self.len - shift) % self.len;
            out.set(i, self.get(src_idx));
        }

        out
    }

    /// Optimized permute using word-level bit rotation.
    ///
    /// # Mathematical Basis
    ///
    /// The naive permute does: `out[i] = src[(i + len - shift) % len]`
    /// This is a RIGHT rotation: elements shift to higher indices.
    ///
    /// Example with shift=1, len=1024:
    /// - out[0] = src[1023]  (last element wraps to first)
    /// - out[1] = src[0]
    /// - out[2] = src[1]
    ///   ...
    ///
    /// For bitsliced, we decompose shift k = 64q + r where:
    /// - q = k / 64 (word-level rotation)
    /// - r = k % 64 (intra-word bit shift)
    ///
    /// # Performance
    ///
    /// - Naive permute: O(D) with D get/set operations
    /// - Optimized permute: O(D/64) with 2 ops per word
    /// - Speedup: ~32-64× for large D
    pub fn permute_optimized(&self, shift: usize) -> Self {
        if self.len == 0 || shift == 0 {
            return self.clone();
        }

        let shift = shift % self.len;
        if shift == 0 {
            return self.clone();
        }

        // For non-64-aligned dimensions, fall back to naive to ensure correctness
        // at boundaries. The optimization is still valuable for the common case.
        if !self.len.is_multiple_of(64) {
            return self.permute(shift);
        }

        let words = Self::word_count(self.len);

        // Decompose: shift = word_shift * 64 + bit_shift
        let word_shift = shift / 64;
        let bit_shift = shift % 64;

        let mut out = Self::new_zero(self.len);

        if bit_shift == 0 {
            // Case 1: Aligned shift - pure word rotation (fastest path)
            // out[i] = src[(i - shift + len) % len]
            // For words: out_word[w] = src_word[(w - word_shift + words) % words]
            for w in 0..words {
                let src_w = (w + words - word_shift) % words;
                out.pos[w] = self.pos[src_w];
                out.neg[w] = self.neg[src_w];
            }
        } else {
            // Case 2: Unaligned shift - combine bits from adjacent words
            //
            // For shift=1, words=16, len=1024:
            //   out bit 0 = src bit 1023 = src[15] bit 63
            //   out bit 1 = src bit 0 = src[0] bit 0
            //   ...
            //   out bit 63 = src bit 62 = src[0] bit 62
            //
            // For out word 0:
            //   bit 0 comes from src[15] bit 63 (the wrap-around bit)
            //   bits 1-63 come from src[0] bits 0-62
            //
            // In general for out word w:
            //   - Low bits [0..bit_shift) come from src[(w - word_shift - 1 + words) % words]
            //     specifically the HIGH bits [64-bit_shift..64) of that word
            //   - High bits [bit_shift..64) come from src[(w - word_shift + words) % words]
            //     specifically the LOW bits [0..64-bit_shift) of that word
            //
            // Bitwise:
            //   out[w] = (src_prev >> (64 - bit_shift)) | (src_curr << bit_shift)
            let complement = 64 - bit_shift;

            for w in 0..words {
                let src_curr = (w + words - word_shift) % words;
                let src_prev = (w + words - word_shift - 1) % words;

                // src_prev's high bits become out's low bits
                // src_curr's low bits become out's high bits
                out.pos[w] = (self.pos[src_prev] >> complement) | (self.pos[src_curr] << bit_shift);
                out.neg[w] = (self.neg[src_prev] >> complement) | (self.neg[src_curr] << bit_shift);
            }
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

        let packed_words = packed.len().div_ceil(32);
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
        let packed_words = self.len.div_ceil(32);

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
            let new_carry_p = (self.sum_pos[w] & vp) | (self.carry_pos[w] & (self.sum_pos[w] ^ vp));
            self.sum_pos[w] ^= vp;
            self.carry_pos[w] = new_carry_p;

            // Same for negative votes
            let new_carry_n = (self.sum_neg[w] & vn) | (self.carry_neg[w] & (self.sum_neg[w] ^ vn));
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

            let pos_gt_neg = (pos_1 & !neg_1) | (!(pos_1 ^ neg_1) & pos_0 & !neg_0);

            let neg_gt_pos = (neg_1 & !pos_1) | (!(pos_1 ^ neg_1) & neg_0 & !pos_0);

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

    /// AVX-512 bundle: processes 512 trits per iteration.
    ///
    /// # Mathematical Basis
    /// out_pos = (a_pos & !b_neg) | (b_pos & !a_neg)
    /// out_neg = (a_neg & !b_pos) | (b_neg & !a_pos)
    ///
    /// # Safety
    /// Requires AVX-512F support. Check with `is_x86_feature_detected!("avx512f")`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn bundle_avx512(
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

            // out_pos = (ap & !bn) | (bp & !an)
            let not_bn = _mm512_xor_si512(bn, _mm512_set1_epi64(-1));
            let not_an = _mm512_xor_si512(an, _mm512_set1_epi64(-1));
            let out_pos =
                _mm512_or_si512(_mm512_and_si512(ap, not_bn), _mm512_and_si512(bp, not_an));

            // out_neg = (an & !bp) | (bn & !ap)
            let not_bp = _mm512_xor_si512(bp, _mm512_set1_epi64(-1));
            let not_ap = _mm512_xor_si512(ap, _mm512_set1_epi64(-1));
            let out_neg =
                _mm512_or_si512(_mm512_and_si512(an, not_bp), _mm512_and_si512(bn, not_ap));

            _mm512_storeu_si512(out.pos.as_mut_ptr().add(offset) as *mut __m512i, out_pos);
            _mm512_storeu_si512(out.neg.as_mut_ptr().add(offset) as *mut __m512i, out_neg);
        }

        // Scalar remainder
        for w in (chunks * 8)..words {
            let (ap, an) = (a.pos[w], a.neg[w]);
            let (bp, bn) = (b.pos[w], b.neg[w]);
            out.pos[w] = (ap & !bn) | (bp & !an);
            out.neg[w] = (an & !bp) | (bn & !ap);
        }
    }

    /// AVX-512 dot product: processes 512 trits per iteration.
    ///
    /// # Mathematical Basis
    /// dot = popcount(ap & bp) + popcount(an & bn) - popcount(ap & bn) - popcount(an & bp)
    ///
    /// # Safety
    /// Requires AVX-512F + AVX-512-VPOPCNTDQ support ideally.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn dot_avx512(a: &BitslicedTritVec, b: &BitslicedTritVec) -> i32 {
        let n = a.len.min(b.len);
        let words = BitslicedTritVec::word_count(n);

        let chunks = words / 8;
        let mut acc: i32 = 0;

        // Process 8 words at a time (512 trits)
        for chunk in 0..chunks {
            let offset = chunk * 8;

            let ap = _mm512_loadu_si512(a.pos.as_ptr().add(offset) as *const __m512i);
            let an = _mm512_loadu_si512(a.neg.as_ptr().add(offset) as *const __m512i);
            let bp = _mm512_loadu_si512(b.pos.as_ptr().add(offset) as *const __m512i);
            let bn = _mm512_loadu_si512(b.neg.as_ptr().add(offset) as *const __m512i);

            // Compute AND masks
            let pp = _mm512_and_si512(ap, bp);
            let nn = _mm512_and_si512(an, bn);
            let pn = _mm512_and_si512(ap, bn);
            let np = _mm512_and_si512(an, bp);

            // Extract and popcount each word (no AVX-512 POPCNT, use scalar)
            let pp_arr: [u64; 8] = std::mem::transmute(pp);
            let nn_arr: [u64; 8] = std::mem::transmute(nn);
            let pn_arr: [u64; 8] = std::mem::transmute(pn);
            let np_arr: [u64; 8] = std::mem::transmute(np);

            for i in 0..8 {
                acc += (pp_arr[i].count_ones() + nn_arr[i].count_ones()) as i32;
                acc -= (pn_arr[i].count_ones() + np_arr[i].count_ones()) as i32;
            }
        }

        // Scalar remainder
        for w in (chunks * 8)..words {
            let (mut ap, mut an) = (a.pos[w], a.neg[w]);
            let (mut bp, mut bn) = (b.pos[w], b.neg[w]);

            // Mask last word
            if w + 1 == words {
                let mask = BitslicedTritVec::last_word_mask(n);
                ap &= mask;
                an &= mask;
                bp &= mask;
                bn &= mask;
            }

            acc += ((ap & bp).count_ones() + (an & bn).count_ones()) as i32;
            acc -= ((ap & bn).count_ones() + (an & bp).count_ones()) as i32;
        }

        acc
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

    #[test]
    fn test_permute_optimized_equivalence() {
        // Test that permute_optimized produces same results as naive permute
        // Use 64-aligned dimension so optimized path is actually exercised
        let dim = 1024; // 16 words, aligned
        let sparse = SparseVec {
            pos: vec![0, 63, 64, 127, 128, 500, 1023],
            neg: vec![1, 62, 65, 126, 129, 501],
        };
        let v = BitslicedTritVec::from_sparse(&sparse, dim);

        // Test key shift amounts: 0, bit-only, word-aligned, mixed, wrap-around
        for shift in [0, 1, 63, 64, 65, 128, 512, 1024] {
            let naive = v.permute(shift);
            let optimized = v.permute_optimized(shift);

            // Spot-check specific positions rather than all D for speed
            let check_positions = [0, 1, 62, 63, 64, 65, 126, 127, 128, 500, 501, 1022, 1023];
            for &i in &check_positions {
                assert_eq!(
                    naive.get(i),
                    optimized.get(i),
                    "Mismatch at i={} for shift={}",
                    i,
                    shift
                );
            }

            // Also verify nnz is preserved
            assert_eq!(
                naive.nnz(),
                optimized.nnz(),
                "nnz mismatch for shift={}",
                shift
            );
        }
    }

    #[test]
    fn test_permute_optimized_unaligned_fallback() {
        // Test that unaligned dimensions correctly fall back to naive
        let dim = 1000; // Not 64-aligned
        let sparse = SparseVec {
            pos: vec![0, 500, 999],
            neg: vec![1, 501],
        };
        let v = BitslicedTritVec::from_sparse(&sparse, dim);

        // Should fall back to naive permute - just verify it doesn't crash
        // and preserves nnz
        let result = v.permute_optimized(123);
        assert_eq!(result.nnz(), v.nnz());
        assert_eq!(result.len(), dim);
    }

    #[test]
    fn test_permute_optimized_word_boundary() {
        // Test permutation at exact word boundaries (64-aligned dim)
        let dim = 256; // 4 words exactly
        let mut v = BitslicedTritVec::new_zero(dim);

        // Set trits at word boundaries
        v.set(0, Trit::P);
        v.set(63, Trit::N);
        v.set(64, Trit::P);
        v.set(127, Trit::N);
        v.set(128, Trit::P);
        v.set(191, Trit::N);
        v.set(192, Trit::P);
        v.set(255, Trit::N);

        // Shift by exactly 64 (one word) - RIGHT rotation
        // out[i] = src[(i - 64 + 256) % 256] = src[(i + 192) % 256]
        let shifted = v.permute_optimized(64);

        // Position 0 should now have what was at position (0 - 64 + 256) % 256 = 192
        assert_eq!(shifted.get(0), Trit::P, "pos 0 should have src[192]=P");
        // Position 64 should now have what was at position (64 - 64 + 256) % 256 = 0
        assert_eq!(shifted.get(64), Trit::P, "pos 64 should have src[0]=P");
        // Position 128 should have what was at position (128 - 64 + 256) % 256 = 64
        assert_eq!(shifted.get(128), Trit::P, "pos 128 should have src[64]=P");
    }

    #[test]
    fn test_permute_optimized_bit_shift() {
        // Test non-word-aligned shifts (bit rotation within words)
        let dim = 128; // 2 words
        let mut v = BitslicedTritVec::new_zero(dim);

        // Set specific bits to track rotation
        v.set(0, Trit::P); // Word 0, bit 0
        v.set(32, Trit::N); // Word 0, bit 32
        v.set(64, Trit::P); // Word 1, bit 0
        v.set(96, Trit::N); // Word 1, bit 32

        // Shift by 32 bits (half a word)
        let shifted = v.permute_optimized(32);

        // Position 0 should have src[(0 - 32 + 128) % 128] = src[96]
        assert_eq!(shifted.get(0), Trit::N, "pos 0 should have src[96]=N");
        // Position 32 should have src[(32 - 32 + 128) % 128] = src[0]
        assert_eq!(shifted.get(32), Trit::P, "pos 32 should have src[0]=P");
    }

    #[test]
    fn test_simd_feature_detection() {
        // Just verify the detection doesn't panic
        let _ = super::has_avx512();
        let _ = super::has_avx2();
        let features = super::simd_features_string();
        assert!(!features.is_empty());
    }
}
