//! Packed ternary vector representation.
//!
//! This module introduces a ternary-native vector type intended to become the
//! internal substrate for fast dot/bind/bundle operations as the project moves
//! toward a balanced-ternary-first implementation.
//!
//! Representation: 2 bits per dimension
//! - 00 = Z (0)
//! - 01 = P (+1)
//! - 10 = N (-1)
//! - 11 = unused (treated as Z)

use crate::ternary::Trit;
use crate::vsa::SparseVec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedTritVec {
    len: usize,
    data: Vec<u64>,
}

impl PackedTritVec {
    const MASK_EVEN_BITS: u64 = 0x5555_5555_5555_5555;

    #[inline]
    fn ensure_len_and_clear(&mut self, len: usize) {
        self.len = len;
        let words = Self::word_count_for_len(len);
        if self.data.len() != words {
            self.data.resize(words, 0u64);
        }
        self.data.fill(0u64);
    }

    pub fn new_zero(len: usize) -> Self {
        let bits = len.saturating_mul(2);
        let words = (bits + 63) / 64;
        Self {
            len,
            data: vec![0u64; words],
        }
    }

    #[inline]
    fn word_count_for_len(len: usize) -> usize {
        let bits = len.saturating_mul(2);
        (bits + 63) / 64
    }

    #[inline]
    fn last_word_mask(len: usize) -> u64 {
        let lanes_in_last = len % 32;
        if lanes_in_last == 0 {
            !0u64
        } else {
            let used_bits = lanes_in_last * 2;
            if used_bits >= 64 {
                !0u64
            } else {
                (1u64 << used_bits) - 1
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn word_bit_index(i: usize) -> (usize, usize) {
        let bit = i * 2;
        (bit / 64, bit % 64)
    }

    pub fn get(&self, i: usize) -> Trit {
        if i >= self.len {
            return Trit::Z;
        }
        let (word, bit) = Self::word_bit_index(i);
        let w = self.data.get(word).copied().unwrap_or(0);
        let v = (w >> bit) & 0b11;
        match v {
            0b01 => Trit::P,
            0b10 => Trit::N,
            _ => Trit::Z,
        }
    }

    pub fn set(&mut self, i: usize, t: Trit) {
        if i >= self.len {
            return;
        }
        let (word, bit) = Self::word_bit_index(i);
        if let Some(w) = self.data.get_mut(word) {
            *w &= !(0b11u64 << bit);
            let enc = match t {
                Trit::Z => 0b00u64,
                Trit::P => 0b01u64,
                Trit::N => 0b10u64,
            };
            *w |= enc << bit;
        }
    }

    pub fn from_sparsevec(vec: &SparseVec, len: usize) -> Self {
        let mut out = Self::new_zero(len);
        out.fill_from_sparsevec(vec, len);
        out
    }

    /// Fill this packed vector from a SparseVec, reusing existing allocation.
    ///
    /// This is a hot-path helper for bt-phase-2 operations to avoid repeated allocations.
    pub fn fill_from_sparsevec(&mut self, vec: &SparseVec, len: usize) {
        self.ensure_len_and_clear(len);

        // Fast set: output is already zeroed, so we can OR lane encodings.
        // 01 => P (+1) sets even bit; 10 => N (-1) sets odd bit.
        for &idx in &vec.pos {
            if idx < len {
                let bit = idx * 2;
                let word = bit / 64;
                let shift = bit % 64;
                self.data[word] |= 1u64 << shift;
            }
        }

        for &idx in &vec.neg {
            if idx < len {
                let bit = idx * 2;
                let word = bit / 64;
                let shift = bit % 64;
                self.data[word] |= 1u64 << (shift + 1);
            }
        }

        if !self.data.is_empty() {
            let last = self.data.len() - 1;
            self.data[last] &= Self::last_word_mask(self.len);
        }
    }

    pub fn to_sparsevec(&self) -> SparseVec {
        let mut pos: Vec<usize> = Vec::new();
        let mut neg: Vec<usize> = Vec::new();

        // Word-wise extraction: each u64 holds 32 trits (2 bits each).
        for (word_idx, &word_raw) in self.data.iter().enumerate() {
            let mut word = word_raw;
            if word_idx + 1 == self.data.len() {
                word &= Self::last_word_mask(self.len);
            }

            // P lanes have the even bit set; N lanes have the odd bit set.
            // Shift odd bits down to even positions to get per-lane masks.
            let pos_bits = word & Self::MASK_EVEN_BITS;
            let neg_bits = (word >> 1) & Self::MASK_EVEN_BITS;

            // Extract indices for P lanes.
            let mut m = pos_bits;
            while m != 0 {
                let tz = m.trailing_zeros() as usize;
                let lane = tz / 2;
                let idx = word_idx * 32 + lane;
                if idx < self.len {
                    pos.push(idx);
                }
                m &= m - 1;
            }

            // Extract indices for N lanes.
            let mut n = neg_bits;
            while n != 0 {
                let tz = n.trailing_zeros() as usize;
                let lane = tz / 2;
                let idx = word_idx * 32 + lane;
                if idx < self.len {
                    neg.push(idx);
                }
                n &= n - 1;
            }
        }

        SparseVec { pos, neg }
    }

    /// Sparse ternary dot product: sum over i of a_i * b_i.
    pub fn dot(&self, other: &Self) -> i32 {
        let n = self.len.min(other.len);
        if n == 0 {
            return 0;
        }

        let words = Self::word_count_for_len(n)
            .min(self.data.len())
            .min(other.data.len());

        let mut acc: i32 = 0;
        for w in 0..words {
            let mut a = self.data[w];
            let mut b = other.data[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                a &= mask;
                b &= mask;
            }

            let a_pos = a & Self::MASK_EVEN_BITS;
            let a_neg = (a >> 1) & Self::MASK_EVEN_BITS;
            let b_pos = b & Self::MASK_EVEN_BITS;
            let b_neg = (b >> 1) & Self::MASK_EVEN_BITS;

            let pp = (a_pos & b_pos).count_ones() as i32;
            let nn = (a_neg & b_neg).count_ones() as i32;
            let pn = (a_pos & b_neg).count_ones() as i32;
            let np = (a_neg & b_pos).count_ones() as i32;

            acc += (pp + nn) - (pn + np);
        }

        acc
    }

    /// Element-wise ternary multiplication (bind primitive).
    pub fn bind(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        if n == 0 {
            return Self::new_zero(0);
        }

        let words = Self::word_count_for_len(n)
            .min(self.data.len())
            .min(other.data.len());
        let mut out = Self::new_zero(n);

        for w in 0..words {
            let mut a = self.data[w];
            let mut b = other.data[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                a &= mask;
                b &= mask;
            }

            let a_pos = a & Self::MASK_EVEN_BITS;
            let a_neg = (a >> 1) & Self::MASK_EVEN_BITS;
            let b_pos = b & Self::MASK_EVEN_BITS;
            let b_neg = (b >> 1) & Self::MASK_EVEN_BITS;

            let same = (a_pos & b_pos) | (a_neg & b_neg);
            let opp = (a_pos & b_neg) | (a_neg & b_pos);

            out.data[w] = same | (opp << 1);
        }

        // Ensure any unused tail bits stay zero.
        if !out.data.is_empty() {
            let last = out.data.len() - 1;
            out.data[last] &= Self::last_word_mask(out.len);
        }

        out
    }

    /// Element-wise ternary multiplication into an existing output buffer.
    pub fn bind_into(&self, other: &Self, out: &mut Self) {
        let n = self.len.min(other.len);
        out.ensure_len_and_clear(n);
        if n == 0 {
            return;
        }

        let words = Self::word_count_for_len(n)
            .min(self.data.len())
            .min(other.data.len())
            .min(out.data.len());

        for w in 0..words {
            let mut a = self.data[w];
            let mut b = other.data[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                a &= mask;
                b &= mask;
            }

            let a_pos = a & Self::MASK_EVEN_BITS;
            let a_neg = (a >> 1) & Self::MASK_EVEN_BITS;
            let b_pos = b & Self::MASK_EVEN_BITS;
            let b_neg = (b >> 1) & Self::MASK_EVEN_BITS;

            let same = (a_pos & b_pos) | (a_neg & b_neg);
            let opp = (a_pos & b_neg) | (a_neg & b_pos);

            out.data[w] = same | (opp << 1);
        }

        if !out.data.is_empty() {
            let last = out.data.len() - 1;
            out.data[last] &= Self::last_word_mask(out.len);
        }
    }

    /// Element-wise saturating ternary addition (bundle primitive for two vectors).
    pub fn bundle(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        if n == 0 {
            return Self::new_zero(0);
        }

        let words = Self::word_count_for_len(n)
            .min(self.data.len())
            .min(other.data.len());
        let mut out = Self::new_zero(n);

        for w in 0..words {
            let mut a = self.data[w];
            let mut b = other.data[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                a &= mask;
                b &= mask;
            }

            let a_pos = a & Self::MASK_EVEN_BITS;
            let a_neg = (a >> 1) & Self::MASK_EVEN_BITS;
            let b_pos = b & Self::MASK_EVEN_BITS;
            let b_neg = (b >> 1) & Self::MASK_EVEN_BITS;

            let mask = Self::MASK_EVEN_BITS;
            let not_b_neg = (!b_neg) & mask;
            let not_a_neg = (!a_neg) & mask;
            let not_b_pos = (!b_pos) & mask;
            let not_a_pos = (!a_pos) & mask;

            let pos = (a_pos & not_b_neg) | (b_pos & not_a_neg);
            let neg = (a_neg & not_b_pos) | (b_neg & not_a_pos);

            out.data[w] = pos | (neg << 1);
        }

        if !out.data.is_empty() {
            let last = out.data.len() - 1;
            out.data[last] &= Self::last_word_mask(out.len);
        }

        out
    }

    /// Element-wise saturating ternary addition into an existing output buffer.
    pub fn bundle_into(&self, other: &Self, out: &mut Self) {
        let n = self.len.min(other.len);
        out.ensure_len_and_clear(n);
        if n == 0 {
            return;
        }

        let words = Self::word_count_for_len(n)
            .min(self.data.len())
            .min(other.data.len())
            .min(out.data.len());

        for w in 0..words {
            let mut a = self.data[w];
            let mut b = other.data[w];
            if w + 1 == words {
                let mask = Self::last_word_mask(n);
                a &= mask;
                b &= mask;
            }

            let a_pos = a & Self::MASK_EVEN_BITS;
            let a_neg = (a >> 1) & Self::MASK_EVEN_BITS;
            let b_pos = b & Self::MASK_EVEN_BITS;
            let b_neg = (b >> 1) & Self::MASK_EVEN_BITS;

            let mask = Self::MASK_EVEN_BITS;
            let not_b_neg = (!b_neg) & mask;
            let not_a_neg = (!a_neg) & mask;
            let not_b_pos = (!b_pos) & mask;
            let not_a_pos = (!a_pos) & mask;

            let pos = (a_pos & not_b_neg) | (b_pos & not_a_neg);
            let neg = (a_neg & not_b_pos) | (b_neg & not_a_pos);

            out.data[w] = pos | (neg << 1);
        }

        if !out.data.is_empty() {
            let last = out.data.len() - 1;
            out.data[last] &= Self::last_word_mask(out.len);
        }
    }
}
