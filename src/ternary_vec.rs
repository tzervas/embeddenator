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
    pub fn new_zero(len: usize) -> Self {
        let bits = len.saturating_mul(2);
        let words = (bits + 63) / 64;
        Self {
            len,
            data: vec![0u64; words],
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
        for &idx in &vec.pos {
            if idx < len {
                out.set(idx, Trit::P);
            }
        }
        for &idx in &vec.neg {
            if idx < len {
                out.set(idx, Trit::N);
            }
        }
        out
    }

    pub fn to_sparsevec(&self) -> SparseVec {
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        // Iterate dimensions. This is the correctness-first baseline; later we
        // can add word-wise fast paths.
        for i in 0..self.len {
            match self.get(i) {
                Trit::P => pos.push(i),
                Trit::N => neg.push(i),
                Trit::Z => {}
            }
        }
        SparseVec { pos, neg }
    }

    /// Sparse ternary dot product: sum over i of a_i * b_i.
    pub fn dot(&self, other: &Self) -> i32 {
        let n = self.len.min(other.len);
        let mut acc = 0i32;
        for i in 0..n {
            let a = self.get(i).to_i8() as i32;
            let b = other.get(i).to_i8() as i32;
            acc += a * b;
        }
        acc
    }

    /// Element-wise ternary multiplication (bind primitive).
    pub fn bind(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        let mut out = Self::new_zero(n);
        for i in 0..n {
            out.set(i, self.get(i).mul(other.get(i)));
        }
        out
    }

    /// Element-wise saturating ternary addition (bundle primitive for two vectors).
    pub fn bundle(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        let mut out = Self::new_zero(n);
        for i in 0..n {
            out.set(i, self.get(i).add_saturating(other.get(i)));
        }
        out
    }
}
