//! Soft Ternary Vector: Multi-bit precision for enhanced VSA operations.
//!
//! This module extends the bitsliced representation with **soft ternary values**:
//! instead of hard {-1, 0, +1}, positions can accumulate magnitude votes.
//!
//! # Representation
//!
//! Each position has a 3-bit magnitude (0-7) and 1-bit sign:
//! - `magnitude = 0`: Zero (no vote)
//! - `magnitude > 0, sign = 0`: Positive with strength `magnitude`
//! - `magnitude > 0, sign = 1`: Negative with strength `magnitude`
//!
//! # Use Cases
//!
//! - **N-way bundling**: Accumulate many vectors before hardening
//! - **Confidence scores**: Track vote strength per position
//! - **Gradual forgetting**: Decay magnitudes over time
//! - **Weighted combinations**: Scale vectors by importance

use crate::bitsliced::BitslicedTritVec;
use crate::vsa::SparseVec;

/// Soft ternary vector with 3-bit magnitude per position.
///
/// Memory layout (4 planes):
/// - `mag_lo`: Bit 0 of magnitude
/// - `mag_mi`: Bit 1 of magnitude
/// - `mag_hi`: Bit 2 of magnitude
/// - `sign`: 0 = positive, 1 = negative (only meaningful when magnitude > 0)
///
/// # Example
///
/// ```rust,ignore
/// let mut soft = SoftTernaryVec::new_zero(10000);
/// soft.accumulate(&vec1);
/// soft.accumulate(&vec2);
/// soft.accumulate(&vec3);
/// let result = soft.harden(2);  // Threshold: need ≥2 votes
/// ```
#[derive(Clone, Debug)]
pub struct SoftTernaryVec {
    len: usize,
    mag_lo: Vec<u64>,  // Magnitude bit 0
    mag_mi: Vec<u64>,  // Magnitude bit 1
    mag_hi: Vec<u64>,  // Magnitude bit 2
    sign: Vec<u64>,    // Sign plane (0 = pos, 1 = neg)
}

impl SoftTernaryVec {
    /// Number of u64 words needed for a dimension.
    #[inline]
    fn word_count(len: usize) -> usize {
        (len + 63) / 64
    }

    /// Mask for the last word's valid bits.
    #[inline]
    fn last_word_mask(len: usize) -> u64 {
        if len % 64 == 0 {
            u64::MAX
        } else {
            (1u64 << (len % 64)) - 1
        }
    }

    /// Create a zero vector of given dimension.
    pub fn new_zero(len: usize) -> Self {
        let words = Self::word_count(len);
        Self {
            len,
            mag_lo: vec![0u64; words],
            mag_mi: vec![0u64; words],
            mag_hi: vec![0u64; words],
            sign: vec![0u64; words],
        }
    }

    /// Dimension (number of trit positions).
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the soft value at position `idx`: (magnitude, is_negative)
    #[inline]
    pub fn get(&self, idx: usize) -> (u8, bool) {
        assert!(idx < self.len, "index out of bounds");
        let word = idx / 64;
        let bit = idx % 64;

        let lo = ((self.mag_lo[word] >> bit) & 1) as u8;
        let mi = ((self.mag_mi[word] >> bit) & 1) as u8;
        let hi = ((self.mag_hi[word] >> bit) & 1) as u8;
        let sign = ((self.sign[word] >> bit) & 1) != 0;

        let magnitude = lo | (mi << 1) | (hi << 2);
        (magnitude, sign)
    }

    /// Set the soft value at position `idx`.
    #[inline]
    pub fn set(&mut self, idx: usize, magnitude: u8, is_negative: bool) {
        assert!(idx < self.len, "index out of bounds");
        assert!(magnitude <= 7, "magnitude must be 0-7");
        let word = idx / 64;
        let bit = idx % 64;
        let mask = 1u64 << bit;

        // Clear old bits
        self.mag_lo[word] &= !mask;
        self.mag_mi[word] &= !mask;
        self.mag_hi[word] &= !mask;
        self.sign[word] &= !mask;

        // Set new bits
        if magnitude & 1 != 0 {
            self.mag_lo[word] |= mask;
        }
        if magnitude & 2 != 0 {
            self.mag_mi[word] |= mask;
        }
        if magnitude & 4 != 0 {
            self.mag_hi[word] |= mask;
        }
        if is_negative && magnitude > 0 {
            self.sign[word] |= mask;
        }
    }

    /// Get signed value (-7 to +7).
    #[inline]
    pub fn get_signed(&self, idx: usize) -> i8 {
        let (mag, neg) = self.get(idx);
        if neg {
            -(mag as i8)
        } else {
            mag as i8
        }
    }

    /// Count non-zero positions.
    pub fn nnz(&self) -> usize {
        let words = Self::word_count(self.len);
        let mut count = 0;

        for w in 0..words {
            let any_mag = self.mag_lo[w] | self.mag_mi[w] | self.mag_hi[w];
            let masked = if w + 1 == words {
                any_mag & Self::last_word_mask(self.len)
            } else {
                any_mag
            };
            count += masked.count_ones() as usize;
        }
        count
    }

    // ========================================================================
    // SOFT OPERATIONS
    // ========================================================================

    /// Accumulate a hard ternary vector (BitslicedTritVec) into this soft vector.
    ///
    /// - Positive trits: increment positive magnitude (saturate at 7)
    /// - Negative trits: increment negative magnitude (saturate at 7)
    /// - Conflicting signs: cancel votes (reduce toward zero)
    pub fn accumulate(&mut self, hard: &BitslicedTritVec) {
        let words = Self::word_count(self.len.min(hard.len()));

        for w in 0..words {
            let h_pos = hard.pos_word(w);
            let h_neg = hard.neg_word(w);

            // Current magnitude bits
            let m0 = self.mag_lo[w];
            let m1 = self.mag_mi[w];
            let m2 = self.mag_hi[w];
            let s = self.sign[w];

            // Positions getting same-sign votes (reinforce)
            let reinforce_pos = h_pos & !s;  // Input pos, current pos (or zero)
            let reinforce_neg = h_neg & s;   // Input neg, current neg

            // Positions getting opposite-sign votes (cancel)
            let cancel_pos = h_pos & s;      // Input pos, but current is neg
            let cancel_neg = h_neg & !s;     // Input neg, but current is pos
            let cancel_mask = (cancel_pos | cancel_neg) & (m0 | m1 | m2); // Only cancel if magnitude > 0

            // Positions getting fresh votes (magnitude was 0)
            let fresh = (h_pos | h_neg) & !(m0 | m1 | m2);

            // Increment magnitude for reinforcing votes (saturating at 7)
            let reinforce = reinforce_pos | reinforce_neg;
            let (new_m0_inc, new_m1_inc, new_m2_inc) = Self::saturating_increment_3bit(m0, m1, m2, reinforce);

            // Decrement magnitude for canceling votes (floor at 0)
            let (new_m0_dec, new_m1_dec, new_m2_dec) = Self::saturating_decrement_3bit(new_m0_inc, new_m1_inc, new_m2_inc, cancel_mask);

            // Set magnitude to 1 for fresh votes
            let new_m0 = new_m0_dec | fresh;
            let new_m1 = new_m1_dec & !fresh;
            let new_m2 = new_m2_dec & !fresh;

            // Update sign for fresh votes
            let fresh_neg = fresh & h_neg;
            let new_sign = (s & !cancel_mask) | fresh_neg;

            // Handle sign flip when magnitude reaches 0 then increments from opposite side
            // (covered by fresh vote logic)

            self.mag_lo[w] = new_m0;
            self.mag_mi[w] = new_m1;
            self.mag_hi[w] = new_m2;
            self.sign[w] = new_sign;
        }
    }

    /// Saturating 3-bit increment: adds 1 to each position in `inc_mask`, saturates at 7.
    #[inline]
    fn saturating_increment_3bit(m0: u64, m1: u64, m2: u64, inc_mask: u64) -> (u64, u64, u64) {
        // Full adder: m + inc_mask
        // But only increment where inc_mask is set, and saturate at 7

        // Positions at max (7 = 111): don't increment
        let at_max = m0 & m1 & m2;
        let can_inc = inc_mask & !at_max;

        // Ripple add within each position (carry within 3 bits)
        let new_m0 = m0 ^ can_inc;
        let carry0 = m0 & can_inc;
        let new_m1 = m1 ^ carry0;
        let carry1 = m1 & carry0;
        let new_m2 = m2 ^ carry1;

        (new_m0, new_m1, new_m2)
    }

    /// Saturating 3-bit decrement: subtracts 1 from each position in `dec_mask`, floors at 0.
    #[inline]
    fn saturating_decrement_3bit(m0: u64, m1: u64, m2: u64, dec_mask: u64) -> (u64, u64, u64) {
        // Only decrement where magnitude > 0
        let non_zero = m0 | m1 | m2;
        let can_dec = dec_mask & non_zero;

        // Borrow-based subtraction within each position
        let new_m0 = m0 ^ can_dec;
        let borrow0 = !m0 & can_dec;
        let new_m1 = m1 ^ borrow0;
        let borrow1 = !m1 & borrow0;
        let new_m2 = m2 ^ borrow1;

        (new_m0, new_m1, new_m2)
    }

    /// Soft bundle: add two soft vectors with magnitude accumulation.
    ///
    /// For each position:
    /// - Same sign: add magnitudes (saturate at 7)
    /// - Opposite sign: subtract magnitudes (result sign = larger magnitude's sign)
    pub fn soft_bundle(&self, other: &Self) -> Self {
        let n = self.len.min(other.len);
        let words = Self::word_count(n);
        let mut out = Self::new_zero(n);

        for w in 0..words {
            // Get magnitudes and signs
            let (a_m0, a_m1, a_m2) = (self.mag_lo[w], self.mag_mi[w], self.mag_hi[w]);
            let (b_m0, b_m1, b_m2) = (other.mag_lo[w], other.mag_mi[w], other.mag_hi[w]);
            let (a_s, b_s) = (self.sign[w], other.sign[w]);

            // Same sign positions (add magnitudes)
            let same_sign = !(a_s ^ b_s);

            // Opposite sign positions (subtract magnitudes)
            let opp_sign = a_s ^ b_s;

            // For same sign: saturating add
            let (sum_m0, sum_m1, sum_m2, _overflow) =
                Self::saturating_add_3bit(a_m0, a_m1, a_m2, b_m0, b_m1, b_m2);

            // For opposite sign: compute |a| - |b| or |b| - |a|
            let (sub_m0, sub_m1, sub_m2, a_ge_b) =
                Self::abs_subtract_3bit(a_m0, a_m1, a_m2, b_m0, b_m1, b_m2);

            // Select result based on same/opposite sign
            let out_m0 = (sum_m0 & same_sign) | (sub_m0 & opp_sign);
            let out_m1 = (sum_m1 & same_sign) | (sub_m1 & opp_sign);
            let out_m2 = (sum_m2 & same_sign) | (sub_m2 & opp_sign);

            // Output sign:
            // - Same sign: use either (they're equal)
            // - Opposite sign: use sign of larger magnitude
            let out_sign = (a_s & same_sign) | ((a_s & a_ge_b | b_s & !a_ge_b) & opp_sign);

            out.mag_lo[w] = out_m0;
            out.mag_mi[w] = out_m1;
            out.mag_hi[w] = out_m2;
            out.sign[w] = out_sign;
        }

        out
    }

    /// Saturating 3-bit add: a + b, saturate at 7.
    #[inline]
    fn saturating_add_3bit(
        a0: u64, a1: u64, a2: u64,
        b0: u64, b1: u64, b2: u64,
    ) -> (u64, u64, u64, u64) {
        // Full 4-bit add, then saturate
        let sum0 = a0 ^ b0;
        let c0 = a0 & b0;
        let sum1 = a1 ^ b1 ^ c0;
        let c1 = (a1 & b1) | (c0 & (a1 ^ b1));
        let sum2 = a2 ^ b2 ^ c1;
        let c2 = (a2 & b2) | (c1 & (a2 ^ b2));

        // Overflow: c2 is set
        let overflow = c2;

        // Saturate: if overflow, set to 7 (111)
        let sat_m0 = sum0 | overflow;
        let sat_m1 = sum1 | overflow;
        let sat_m2 = sum2 | overflow;

        (sat_m0, sat_m1, sat_m2, overflow)
    }

    /// Absolute subtraction: |a - b|, returns (result, a >= b mask).
    #[inline]
    fn abs_subtract_3bit(
        a0: u64, a1: u64, a2: u64,
        b0: u64, b1: u64, b2: u64,
    ) -> (u64, u64, u64, u64) {
        // Compare: a >= b
        // 3-bit comparison: check from MSB to LSB
        let a_gt_2 = a2 & !b2;
        let eq_2 = !(a2 ^ b2);
        let a_gt_1 = a1 & !b1;
        let eq_1 = !(a1 ^ b1);
        let a_gt_0 = a0 & !b0;
        let eq_0 = !(a0 ^ b0);

        let a_ge_b = a_gt_2 | (eq_2 & a_gt_1) | (eq_2 & eq_1 & a_gt_0) | (eq_2 & eq_1 & eq_0);

        // Compute a - b where a >= b, else b - a
        let (big0, big1, big2) = (
            (a0 & a_ge_b) | (b0 & !a_ge_b),
            (a1 & a_ge_b) | (b1 & !a_ge_b),
            (a2 & a_ge_b) | (b2 & !a_ge_b),
        );
        let (small0, small1, small2) = (
            (b0 & a_ge_b) | (a0 & !a_ge_b),
            (b1 & a_ge_b) | (a1 & !a_ge_b),
            (b2 & a_ge_b) | (a2 & !a_ge_b),
        );

        // Subtract: big - small
        let diff0 = big0 ^ small0;
        let borrow0 = !big0 & small0;
        let diff1 = big1 ^ small1 ^ borrow0;
        let borrow1 = (!big1 & small1) | (borrow0 & !(big1 ^ small1));
        let diff2 = big2 ^ small2 ^ borrow1;

        (diff0, diff1, diff2, a_ge_b)
    }

    // ========================================================================
    // HARDENING (Convert to Hard Ternary)
    // ========================================================================

    /// Convert to hard ternary (BitslicedTritVec) with magnitude threshold.
    ///
    /// - `threshold = 1`: Any vote counts
    /// - `threshold = 2`: Need ≥2 votes (majority of 3)
    /// - `threshold = 4`: Need strong consensus
    pub fn harden(&self, threshold: u8) -> BitslicedTritVec {
        assert!(threshold >= 1 && threshold <= 7, "threshold must be 1-7");
        let words = Self::word_count(self.len);
        let mut out = BitslicedTritVec::new_zero(self.len);

        for w in 0..words {
            // Compute magnitude >= threshold
            let (m0, m1, m2) = (self.mag_lo[w], self.mag_mi[w], self.mag_hi[w]);
            let t0 = threshold & 1 != 0;
            let t1 = threshold & 2 != 0;
            let t2 = threshold & 4 != 0;

            // m >= t comparison
            let ge = Self::compare_ge_threshold(m0, m1, m2, t0, t1, t2);

            // Output: positive where ge and sign=0, negative where ge and sign=1
            let pos_mask = ge & !self.sign[w];
            let neg_mask = ge & self.sign[w];

            // Apply last word mask
            let mask = if w + 1 == words {
                Self::last_word_mask(self.len)
            } else {
                u64::MAX
            };

            out.set_pos_word(w, pos_mask & mask);
            out.set_neg_word(w, neg_mask & mask);
        }

        out
    }

    /// Compare 3-bit magnitude >= threshold.
    #[inline]
    fn compare_ge_threshold(m0: u64, m1: u64, m2: u64, t0: bool, t1: bool, t2: bool) -> u64 {
        // m >= t is equivalent to NOT(m < t) = NOT(t > m)
        // t > m when: t2 > m2, or (t2 == m2 and t1 > m1), or (t2 == m2 and t1 == m1 and t0 > m0)

        let t0_mask = if t0 { u64::MAX } else { 0 };
        let t1_mask = if t1 { u64::MAX } else { 0 };
        let t2_mask = if t2 { u64::MAX } else { 0 };

        let t_gt_m2 = t2_mask & !m2;
        let eq_2 = !(t2_mask ^ m2);
        let t_gt_m1 = t1_mask & !m1;
        let eq_1 = !(t1_mask ^ m1);
        let t_gt_m0 = t0_mask & !m0;

        let t_gt_m = t_gt_m2 | (eq_2 & t_gt_m1) | (eq_2 & eq_1 & t_gt_m0);

        !t_gt_m
    }

    /// Convert to hard ternary using simple majority (threshold = 1).
    pub fn harden_any(&self) -> BitslicedTritVec {
        self.harden(1)
    }

    // ========================================================================
    // CONVERSIONS
    // ========================================================================

    /// Create from BitslicedTritVec (all magnitudes = 1).
    pub fn from_bitsliced(hard: &BitslicedTritVec) -> Self {
        let words = Self::word_count(hard.len());
        let mut out = Self::new_zero(hard.len());

        for w in 0..words {
            let pos = hard.pos_word(w);
            let neg = hard.neg_word(w);

            // Magnitude bit 0 = any trit present
            out.mag_lo[w] = pos | neg;
            // Magnitude bits 1, 2 = 0 (magnitude = 1)
            // Sign = 1 for negative
            out.sign[w] = neg;
        }

        out
    }

    /// Create from SparseVec (all magnitudes = 1).
    pub fn from_sparse(sparse: &SparseVec, len: usize) -> Self {
        let hard = BitslicedTritVec::from_sparse(sparse, len);
        Self::from_bitsliced(&hard)
    }

    /// Dot product between soft vector and hard vector.
    /// Returns signed sum of (soft_value × hard_value).
    pub fn dot_with_hard(&self, hard: &BitslicedTritVec) -> i64 {
        let words = Self::word_count(self.len.min(hard.len()));
        let mut acc: i64 = 0;

        for w in 0..words {
            let h_pos = hard.pos_word(w);
            let h_neg = hard.neg_word(w);

            // Extract magnitudes for positions where hard is non-zero
            // This is expensive but accurate
            for bit in 0..64 {
                let idx = w * 64 + bit;
                if idx >= self.len {
                    break;
                }

                let h_val = if (h_pos >> bit) & 1 != 0 {
                    1i64
                } else if (h_neg >> bit) & 1 != 0 {
                    -1i64
                } else {
                    continue;
                };

                let (mag, neg) = self.get(idx);
                let s_val = if neg { -(mag as i64) } else { mag as i64 };

                acc += s_val * h_val;
            }
        }

        acc
    }

    /// Efficient dot product using bitwise operations.
    pub fn dot_with_hard_fast(&self, hard: &BitslicedTritVec) -> i64 {
        let n = self.len.min(hard.len());
        let words = Self::word_count(n);
        let mut acc: i64 = 0;

        for w in 0..words {
            let h_pos = hard.pos_word(w);
            let h_neg = hard.neg_word(w);
            let h_any = h_pos | h_neg;

            let m0 = self.mag_lo[w] & h_any;
            let m1 = self.mag_mi[w] & h_any;
            let m2 = self.mag_hi[w] & h_any;
            let s = self.sign[w];

            // Mask for last word
            let mask = if w + 1 == words {
                Self::last_word_mask(n)
            } else {
                u64::MAX
            };

            // Positions contributing positive to dot:
            // - soft positive (s=0) and hard positive (h_pos)
            // - soft negative (s=1) and hard negative (h_neg)
            let pos_contrib = ((!s & h_pos) | (s & h_neg)) & mask;

            // Positions contributing negative to dot:
            // - soft positive (s=0) and hard negative (h_neg)
            // - soft negative (s=1) and hard positive (h_pos)
            let neg_contrib = ((!s & h_neg) | (s & h_pos)) & mask;

            // Sum magnitudes for positive contributions
            let pos_m0 = (m0 & pos_contrib).count_ones() as i64;
            let pos_m1 = (m1 & pos_contrib).count_ones() as i64;
            let pos_m2 = (m2 & pos_contrib).count_ones() as i64;
            let pos_sum = pos_m0 + pos_m1 * 2 + pos_m2 * 4;

            // Sum magnitudes for negative contributions
            let neg_m0 = (m0 & neg_contrib).count_ones() as i64;
            let neg_m1 = (m1 & neg_contrib).count_ones() as i64;
            let neg_m2 = (m2 & neg_contrib).count_ones() as i64;
            let neg_sum = neg_m0 + neg_m1 * 2 + neg_m2 * 4;

            acc += pos_sum - neg_sum;
        }

        acc
    }

    /// Reset to zero.
    pub fn reset(&mut self) {
        self.mag_lo.fill(0);
        self.mag_mi.fill(0);
        self.mag_hi.fill(0);
        self.sign.fill(0);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Trit;

    #[test]
    fn test_soft_get_set() {
        let mut v = SoftTernaryVec::new_zero(100);

        v.set(0, 3, false);  // +3
        v.set(1, 7, true);   // -7
        v.set(50, 1, false); // +1
        v.set(99, 4, true);  // -4

        assert_eq!(v.get(0), (3, false));
        assert_eq!(v.get(1), (7, true));
        assert_eq!(v.get(50), (1, false));
        assert_eq!(v.get(99), (4, true));
        assert_eq!(v.get(2), (0, false));  // Zero

        assert_eq!(v.get_signed(0), 3);
        assert_eq!(v.get_signed(1), -7);
        assert_eq!(v.get_signed(2), 0);
    }

    #[test]
    fn test_soft_from_bitsliced() {
        let mut hard = BitslicedTritVec::new_zero(100);
        hard.set(0, Trit::P);
        hard.set(1, Trit::N);
        hard.set(50, Trit::P);

        let soft = SoftTernaryVec::from_bitsliced(&hard);

        assert_eq!(soft.get(0), (1, false));  // +1
        assert_eq!(soft.get(1), (1, true));   // -1
        assert_eq!(soft.get(2), (0, false));  // 0
        assert_eq!(soft.get(50), (1, false)); // +1
    }

    #[test]
    fn test_soft_accumulate() {
        let mut soft = SoftTernaryVec::new_zero(100);

        // Accumulate 3 vectors with same position positive
        for _ in 0..3 {
            let mut hard = BitslicedTritVec::new_zero(100);
            hard.set(0, Trit::P);
            soft.accumulate(&hard);
        }

        assert_eq!(soft.get(0), (3, false));  // Accumulated +3

        // Accumulate one negative vote
        let mut hard = BitslicedTritVec::new_zero(100);
        hard.set(0, Trit::N);
        soft.accumulate(&hard);

        assert_eq!(soft.get(0), (2, false));  // Now +2 (cancellation)
    }

    #[test]
    fn test_soft_saturation() {
        let mut soft = SoftTernaryVec::new_zero(100);

        // Accumulate many positive votes
        for _ in 0..10 {
            let mut hard = BitslicedTritVec::new_zero(100);
            hard.set(0, Trit::P);
            soft.accumulate(&hard);
        }

        // Should saturate at 7
        assert_eq!(soft.get(0), (7, false));
    }

    #[test]
    fn test_soft_harden() {
        let mut soft = SoftTernaryVec::new_zero(100);
        soft.set(0, 3, false);  // +3
        soft.set(1, 5, true);   // -5
        soft.set(2, 1, false);  // +1
        soft.set(3, 0, false);  // 0

        // Threshold = 2
        let hard = soft.harden(2);
        assert_eq!(hard.get(0), Trit::P);  // 3 >= 2
        assert_eq!(hard.get(1), Trit::N);  // 5 >= 2
        assert_eq!(hard.get(2), Trit::Z);  // 1 < 2
        assert_eq!(hard.get(3), Trit::Z);  // 0 < 2

        // Threshold = 1
        let hard = soft.harden(1);
        assert_eq!(hard.get(2), Trit::P);  // 1 >= 1
    }

    #[test]
    fn test_soft_bundle() {
        let mut a = SoftTernaryVec::new_zero(100);
        let mut b = SoftTernaryVec::new_zero(100);

        // Same sign: add magnitudes
        a.set(0, 3, false);  // +3
        b.set(0, 4, false);  // +4

        // Opposite sign: subtract
        a.set(1, 5, false);  // +5
        b.set(1, 3, true);   // -3

        let result = a.soft_bundle(&b);

        assert_eq!(result.get(0), (7, false));  // +3 + +4 = +7
        assert_eq!(result.get(1), (2, false));  // +5 + -3 = +2
    }

    #[test]
    fn test_dot_with_hard() {
        let mut soft = SoftTernaryVec::new_zero(100);
        soft.set(0, 3, false);  // +3
        soft.set(1, 2, true);   // -2

        let mut hard = BitslicedTritVec::new_zero(100);
        hard.set(0, Trit::P);  // +1
        hard.set(1, Trit::P);  // +1

        // Dot = (+3 × +1) + (-2 × +1) = 3 - 2 = 1
        assert_eq!(soft.dot_with_hard(&hard), 1);
        assert_eq!(soft.dot_with_hard_fast(&hard), 1);

        hard.set(1, Trit::N);  // -1
        // Dot = (+3 × +1) + (-2 × -1) = 3 + 2 = 5
        assert_eq!(soft.dot_with_hard(&hard), 5);
        assert_eq!(soft.dot_with_hard_fast(&hard), 5);
    }

    #[test]
    fn test_nnz() {
        let mut soft = SoftTernaryVec::new_zero(100);
        soft.set(0, 3, false);
        soft.set(1, 1, true);
        soft.set(50, 7, false);

        assert_eq!(soft.nnz(), 3);
    }
}
