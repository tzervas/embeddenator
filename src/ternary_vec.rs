//! Packed ternary vector representation.
//!
//! Phase-1 wraps [`trit_vsa::PackedTritVec`] (plus/minus `u32` planes).
//! Holographic bind is element-wise trit **multiplication**, not
//! [`trit_vsa::vsa::bind`] (subtraction mod 3).
//!
//! # Public API used by `vsa.rs`
//!
//! - [`PackedTritVec::from_sparsevec`] / [`PackedTritVec::fill_from_sparsevec`]
//! - [`PackedTritVec::to_sparsevec`]
//! - [`PackedTritVec::bind`] / [`PackedTritVec::bind_into`] (mul)
//! - [`PackedTritVec::bundle`] / [`PackedTritVec::bundle_into`]
//! - [`PackedTritVec::dot`] / [`PackedTritVec::len`]
//!
//! Scalar get/set is the phase-1 conversion path. SIMD on the wrap is deferred.

use crate::ternary::Trit;
use crate::vsa::SparseVec;

/// Bitsliced packed trit vector wrapping `trit_vsa::PackedTritVec`.
///
/// Layout is plus/minus `u32` planes (not the historical interleaved `u64`
/// 2-bits-per-trit encoding). Do not type-alias: layouts differ.
#[derive(Clone, PartialEq, Eq)]
pub struct PackedTritVec {
    inner: trit_vsa::PackedTritVec,
}

impl std::fmt::Debug for PackedTritVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackedTritVec")
            .field("len", &self.len())
            .finish()
    }
}

impl PackedTritVec {
    /// Allocate a zero vector of `len` trits.
    pub fn new_zero(len: usize) -> Self {
        Self {
            inner: trit_vsa::PackedTritVec::new(len),
        }
    }

    #[inline]
    fn last_word_mask(len: usize) -> u32 {
        let rem = len % 32;
        if rem == 0 {
            !0u32
        } else {
            (1u32 << rem) - 1
        }
    }

    fn from_planes(plus: Vec<u32>, minus: Vec<u32>, len: usize) -> Self {
        let inner = trit_vsa::PackedTritVec::from_planes(plus, minus, len)
            .expect("plus/minus plane word count matches len");
        Self { inner }
    }

    fn mask_last_word(plus: &mut [u32], minus: &mut [u32], len: usize) {
        if plus.is_empty() {
            return;
        }
        let last = plus.len() - 1;
        let mask = Self::last_word_mask(len);
        plus[last] &= mask;
        minus[last] &= mask;
    }

    fn ensure_len_and_clear(&mut self, len: usize) {
        self.inner = trit_vsa::PackedTritVec::new(len);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Scalar get. Out-of-range returns [`Trit::Z`] (does not panic).
    pub fn get(&self, i: usize) -> Trit {
        if i >= self.len() {
            return Trit::Z;
        }
        Trit::from_inner(self.inner.get(i))
    }

    /// Scalar set. Out-of-range is a no-op (does not panic).
    pub fn set(&mut self, i: usize, t: Trit) {
        if i >= self.len() {
            return;
        }
        self.inner.set(i, t.inner());
    }

    pub fn from_sparsevec(vec: &SparseVec, len: usize) -> Self {
        let mut out = Self::new_zero(len);
        out.fill_from_sparsevec(vec, len);
        out
    }

    /// Fill this packed vector from a SparseVec.
    ///
    /// Overlapping `pos`/`neg` indices cancel to zero (same as the historical
    /// interleaved layout treating 0b11 as Z).
    pub fn fill_from_sparsevec(&mut self, vec: &SparseVec, len: usize) {
        let words = len.div_ceil(32);
        let mut plus = vec![0u32; words];
        let mut minus = vec![0u32; words];

        for &idx in &vec.pos {
            if idx < len {
                plus[idx / 32] |= 1u32 << (idx % 32);
            }
        }
        for &idx in &vec.neg {
            if idx < len {
                minus[idx / 32] |= 1u32 << (idx % 32);
            }
        }

        for i in 0..words {
            let conflict = plus[i] & minus[i];
            plus[i] &= !conflict;
            minus[i] &= !conflict;
        }
        Self::mask_last_word(&mut plus, &mut minus, len);
        *self = Self::from_planes(plus, minus, len);
    }

    pub fn to_sparsevec(&self) -> SparseVec {
        let mut pos: Vec<usize> = Vec::new();
        let mut neg: Vec<usize> = Vec::new();
        let n = self.len();
        let plus = self.inner.plus_plane();
        let minus = self.inner.minus_plane();

        for (word_idx, (&p_raw, &m_raw)) in plus.iter().zip(minus.iter()).enumerate() {
            let mut p = p_raw;
            let mut m = m_raw;
            if word_idx + 1 == plus.len() {
                let mask = Self::last_word_mask(n);
                p &= mask;
                m &= mask;
            }
            // Both planes set → treat as Z (cancel).
            let conflict = p & m;
            p &= !conflict;
            m &= !conflict;

            let mut bits = p;
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                let idx = word_idx * 32 + b;
                if idx < n {
                    pos.push(idx);
                }
                bits &= bits - 1;
            }

            let mut bits = m;
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                let idx = word_idx * 32 + b;
                if idx < n {
                    neg.push(idx);
                }
                bits &= bits - 1;
            }
        }

        SparseVec { pos, neg }
    }

    /// Sparse ternary dot product: sum over i of a_i * b_i.
    pub fn dot(&self, other: &Self) -> i32 {
        let n = self.len().min(other.len());
        if n == 0 {
            return 0;
        }
        let words = n
            .div_ceil(32)
            .min(self.inner.num_words())
            .min(other.inner.num_words());
        let a_plus = self.inner.plus_plane();
        let a_minus = self.inner.minus_plane();
        let b_plus = other.inner.plus_plane();
        let b_minus = other.inner.minus_plane();

        let mut acc: i32 = 0;
        for w in 0..words {
            let mut ap = a_plus[w];
            let mut am = a_minus[w];
            let mut bp = b_plus[w];
            let mut bm = b_minus[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                ap &= mask;
                am &= mask;
                bp &= mask;
                bm &= mask;
            }
            let pp = (ap & bp).count_ones() as i32;
            let nn = (am & bm).count_ones() as i32;
            let pn = (ap & bm).count_ones() as i32;
            let np = (am & bp).count_ones() as i32;
            acc += (pp + nn) - (pn + np);
        }
        acc
    }

    /// Element-wise ternary multiplication (holographic bind).
    ///
    /// `P*P=P` (self-inverse). This is **not** `trit_vsa::vsa::bind`.
    pub fn bind(&self, other: &Self) -> Self {
        let n = self.len().min(other.len());
        if n == 0 {
            return Self::new_zero(0);
        }
        let mut out = Self::new_zero(n);
        self.bind_into(other, &mut out);
        out
    }

    /// Element-wise ternary multiplication into an existing output buffer.
    pub fn bind_into(&self, other: &Self, out: &mut Self) {
        let n = self.len().min(other.len());
        if n == 0 {
            out.ensure_len_and_clear(0);
            return;
        }

        let words = n
            .div_ceil(32)
            .min(self.inner.num_words())
            .min(other.inner.num_words());
        let a_plus = self.inner.plus_plane();
        let a_minus = self.inner.minus_plane();
        let b_plus = other.inner.plus_plane();
        let b_minus = other.inner.minus_plane();

        let mut plus = vec![0u32; n.div_ceil(32)];
        let mut minus = vec![0u32; n.div_ceil(32)];
        for w in 0..words {
            let mut ap = a_plus[w];
            let mut am = a_minus[w];
            let mut bp = b_plus[w];
            let mut bm = b_minus[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                ap &= mask;
                am &= mask;
                bp &= mask;
                bm &= mask;
            }
            // mul: same signs → P, opposite → N, either zero → Z
            plus[w] = (ap & bp) | (am & bm);
            minus[w] = (ap & bm) | (am & bp);
        }
        Self::mask_last_word(&mut plus, &mut minus, n);
        *out = Self::from_planes(plus, minus, n);
    }

    /// Element-wise saturating ternary addition (bundle primitive).
    pub fn bundle(&self, other: &Self) -> Self {
        let n = self.len().min(other.len());
        if n == 0 {
            return Self::new_zero(0);
        }
        let mut out = Self::new_zero(n);
        self.bundle_into(other, &mut out);
        out
    }

    /// Element-wise saturating ternary addition into an existing output buffer.
    pub fn bundle_into(&self, other: &Self, out: &mut Self) {
        let n = self.len().min(other.len());
        if n == 0 {
            out.ensure_len_and_clear(0);
            return;
        }

        let words = n
            .div_ceil(32)
            .min(self.inner.num_words())
            .min(other.inner.num_words());
        let a_plus = self.inner.plus_plane();
        let a_minus = self.inner.minus_plane();
        let b_plus = other.inner.plus_plane();
        let b_minus = other.inner.minus_plane();

        let mut plus = vec![0u32; n.div_ceil(32)];
        let mut minus = vec![0u32; n.div_ceil(32)];
        for w in 0..words {
            let mut ap = a_plus[w];
            let mut am = a_minus[w];
            let mut bp = b_plus[w];
            let mut bm = b_minus[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                ap &= mask;
                am &= mask;
                bp &= mask;
                bm &= mask;
            }
            // Saturating add: pos = (a+ & !b-) | (b+ & !a-); cancel opposite signs.
            plus[w] = (ap & !bm) | (bp & !am);
            minus[w] = (am & !bp) | (bm & !ap);
        }
        Self::mask_last_word(&mut plus, &mut minus, n);
        *out = Self::from_planes(plus, minus, n);
    }
}
