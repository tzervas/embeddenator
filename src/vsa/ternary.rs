//! Foundational Balanced Ternary Primitives
//!
//! This module implements the mathematically rigorous single-trit layer.
//! Everything builds on these primitives - they MUST be proven correct.
//!
//! # Representation
//!
//! Single Trit: {N, Z, P} = {-1, 0, +1}
//! - N (Negative): -1
//! - Z (Zero): 0  
//! - P (Positive): +1
//!
//! # Algebraic Properties (Must Hold)
//!
//! ## Addition (Bundle primitive)
//! - Commutative: a + b = b + a
//! - Associative: (a + b) + c = a + (b + c)
//! - Identity: a + Z = a
//! - Inverse: a + (-a) = Z
//!
//! ## Multiplication (Bind primitive)
//! - Commutative: a × b = b × a
//! - Associative: (a × b) × c = a × (b × c)
//! - Identity: a × P = a
//! - Self-inverse: a × a = P (for non-zero)
//! - Zero annihilator: a × Z = Z
//!
//! # Reconstruction Guarantee
//!
//! The key insight: VSA operations are "approximate" only when you lose
//! information through superposition without tracking. We guarantee
//! reconstruction by:
//!
//! 1. Exact residual storage for anything basis can't capture
//! 2. Codebook lookup (not similarity matching) for known patterns
//! 3. Semantic markers for high-entropy regions
//! 4. Parity trits for error detection

use serde::{Deserialize, Serialize};
use std::fmt;

/// Single balanced ternary digit: the atomic unit
///
/// This is THE foundational type. All math builds on this.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[repr(i8)]
pub enum Trit {
    /// Negative: -1
    N = -1,
    /// Zero: 0
    #[default]
    Z = 0,
    /// Positive: +1
    P = 1,
}

impl fmt::Debug for Trit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trit::N => write!(f, "N"),
            Trit::Z => write!(f, "Z"),
            Trit::P => write!(f, "P"),
        }
    }
}

impl fmt::Display for Trit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trit::N => write!(f, "-"),
            Trit::Z => write!(f, "0"),
            Trit::P => write!(f, "+"),
        }
    }
}

impl Trit {
    /// All possible trit values in order
    pub const ALL: [Trit; 3] = [Trit::N, Trit::Z, Trit::P];

    /// Convert from i8 with clamping
    #[inline]
    pub const fn from_i8_clamped(v: i8) -> Self {
        match v {
            i8::MIN..=-1 => Trit::N,
            0 => Trit::Z,
            1..=i8::MAX => Trit::P,
        }
    }

    /// Convert from i8, returning None if out of range
    #[inline]
    pub const fn from_i8_exact(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Trit::N),
            0 => Some(Trit::Z),
            1 => Some(Trit::P),
            _ => None,
        }
    }

    /// Convert to i8
    #[inline]
    pub const fn to_i8(self) -> i8 {
        self as i8
    }

    /// Negate: -N = P, -Z = Z, -P = N
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub const fn neg(self) -> Trit {
        match self {
            Trit::N => Trit::P,
            Trit::Z => Trit::Z,
            Trit::P => Trit::N,
        }
    }

    /// Absolute value: |N| = P, |Z| = Z, |P| = P
    #[inline]
    pub const fn abs(self) -> Trit {
        match self {
            Trit::N => Trit::P,
            Trit::Z => Trit::Z,
            Trit::P => Trit::P,
        }
    }

    /// Sign: returns -1, 0, or 1
    #[inline]
    pub const fn sign(self) -> i8 {
        self as i8
    }

    /// Is zero?
    #[inline]
    pub const fn is_zero(self) -> bool {
        matches!(self, Trit::Z)
    }

    /// Is non-zero?
    #[inline]
    pub const fn is_nonzero(self) -> bool {
        !self.is_zero()
    }

    /// Trit multiplication (bind operation)
    ///
    /// Truth table:
    /// ```text
    ///   × | N  Z  P
    /// ----+--------
    ///   N | P  Z  N
    ///   Z | Z  Z  Z
    ///   P | N  Z  P
    /// ```
    ///
    /// Key property: a × a = P for a ∈ {N, P} (self-inverse)
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub const fn mul(self, other: Trit) -> Trit {
        match (self, other) {
            (Trit::Z, _) | (_, Trit::Z) => Trit::Z,
            (Trit::P, Trit::P) | (Trit::N, Trit::N) => Trit::P,
            (Trit::P, Trit::N) | (Trit::N, Trit::P) => Trit::N,
        }
    }

    /// Trit addition with carry (for multi-trit arithmetic)
    ///
    /// Returns (sum, carry) where both are trits
    ///
    /// In balanced ternary:
    /// - Sum of 3 trits ranges from -3 to +3
    /// - We express this as (sum_trit, carry_trit) where result = sum + 3*carry
    #[inline]
    pub const fn add_with_carry(self, other: Trit, carry_in: Trit) -> (Trit, Trit) {
        let sum = self.to_i8() + other.to_i8() + carry_in.to_i8();
        match sum {
            -3 => (Trit::Z, Trit::N), // -3 = 0 + 3×(-1)
            -2 => (Trit::P, Trit::N), // -2 = 1 + 3×(-1)
            -1 => (Trit::N, Trit::Z), // -1 = -1 + 3×0
            0 => (Trit::Z, Trit::Z),  // 0 = 0 + 3×0
            1 => (Trit::P, Trit::Z),  // 1 = 1 + 3×0
            2 => (Trit::N, Trit::P),  // 2 = -1 + 3×1
            3 => (Trit::Z, Trit::P),  // 3 = 0 + 3×1
            _ => unreachable!(),
        }
    }

    /// Simple trit addition (saturating to trit range)
    /// Used for bundle majority voting
    #[inline]
    pub const fn add_saturating(self, other: Trit) -> Trit {
        let sum = self.to_i8() + other.to_i8();
        Trit::from_i8_clamped(sum)
    }

    /// Majority of three trits (used in multi-way bundling)
    #[inline]
    pub const fn majority3(a: Trit, b: Trit, c: Trit) -> Trit {
        let sum = a.to_i8() + b.to_i8() + c.to_i8();
        Trit::from_i8_clamped(sum)
    }

    /// Encode two bits as a trit (with one invalid state)
    /// 00 -> Z, 01 -> P, 10 -> N, 11 -> invalid (returns None)
    #[inline]
    pub const fn from_bits(b1: bool, b0: bool) -> Option<Trit> {
        match (b1, b0) {
            (false, false) => Some(Trit::Z),
            (false, true) => Some(Trit::P),
            (true, false) => Some(Trit::N),
            (true, true) => None, // Invalid encoding
        }
    }

    /// Decode trit to two bits
    /// Z -> (0, 0), P -> (0, 1), N -> (1, 0)
    #[inline]
    pub const fn to_bits(self) -> (bool, bool) {
        match self {
            Trit::Z => (false, false),
            Trit::P => (false, true),
            Trit::N => (true, false),
        }
    }
}

impl std::ops::Neg for Trit {
    type Output = Trit;
    #[inline]
    fn neg(self) -> Trit {
        Trit::neg(self)
    }
}

impl std::ops::Mul for Trit {
    type Output = Trit;
    #[inline]
    fn mul(self, rhs: Trit) -> Trit {
        Trit::mul(self, rhs)
    }
}

impl std::ops::MulAssign for Trit {
    #[inline]
    fn mul_assign(&mut self, rhs: Trit) {
        *self = *self * rhs;
    }
}

/// A tryte: exactly 3 trits = 27 states
///
/// Range: -13 to +13 in balanced ternary
///
/// Layout: [trit0 (LST), trit1, trit2 (MST)]
/// Value = trit0 + 3×trit1 + 9×trit2
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Tryte3 {
    /// Three trits, index 0 is least significant
    pub trits: [Trit; 3],
}

impl fmt::Debug for Tryte3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tryte3[{}{}{} = {}]",
            self.trits[2],
            self.trits[1],
            self.trits[0],
            self.to_i8()
        )
    }
}

impl fmt::Display for Tryte3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.trits[2], self.trits[1], self.trits[0])
    }
}

impl Tryte3 {
    /// Zero tryte
    pub const ZERO: Tryte3 = Tryte3 {
        trits: [Trit::Z, Trit::Z, Trit::Z],
    };

    /// Maximum value: +++ = 13
    pub const MAX: Tryte3 = Tryte3 {
        trits: [Trit::P, Trit::P, Trit::P],
    };

    /// Minimum value: --- = -13
    pub const MIN: Tryte3 = Tryte3 {
        trits: [Trit::N, Trit::N, Trit::N],
    };

    /// Maximum representable value
    pub const MAX_VALUE: i8 = 13;

    /// Minimum representable value
    pub const MIN_VALUE: i8 = -13;

    /// Number of distinct states
    pub const NUM_STATES: u8 = 27;

    /// Create from three trits [LST, middle, MST]
    #[inline]
    pub const fn new(t0: Trit, t1: Trit, t2: Trit) -> Self {
        Tryte3 {
            trits: [t0, t1, t2],
        }
    }

    /// Create from integer value (-13 to 13)
    pub const fn from_i8(mut value: i8) -> Option<Self> {
        if value < Self::MIN_VALUE || value > Self::MAX_VALUE {
            return None;
        }

        let negative = value < 0;
        if negative {
            value = -value;
        }

        let mut trits = [Trit::Z; 3];
        let mut i = 0;

        while i < 3 && value != 0 {
            let remainder = value % 3;
            value /= 3;

            trits[i] = match remainder {
                0 => Trit::Z,
                1 => Trit::P,
                2 => {
                    value += 1; // Carry
                    Trit::N
                }
                _ => return None, // unreachable
            };
            i += 1;
        }

        if negative {
            trits[0] = trits[0].neg();
            trits[1] = trits[1].neg();
            trits[2] = trits[2].neg();
        }

        Some(Tryte3 { trits })
    }

    /// Convert to integer value
    #[inline]
    pub const fn to_i8(self) -> i8 {
        self.trits[0].to_i8() + 3 * self.trits[1].to_i8() + 9 * self.trits[2].to_i8()
    }

    /// Negate all trits
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub const fn neg(self) -> Self {
        Tryte3 {
            trits: [
                self.trits[0].neg(),
                self.trits[1].neg(),
                self.trits[2].neg(),
            ],
        }
    }

    /// Trit-wise multiplication (bind)
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub const fn mul(self, other: Tryte3) -> Tryte3 {
        Tryte3 {
            trits: [
                self.trits[0].mul(other.trits[0]),
                self.trits[1].mul(other.trits[1]),
                self.trits[2].mul(other.trits[2]),
            ],
        }
    }

    /// Trit-wise majority voting (bundle)
    #[inline]
    pub const fn bundle(self, other: Tryte3) -> Tryte3 {
        Tryte3 {
            trits: [
                self.trits[0].add_saturating(other.trits[0]),
                self.trits[1].add_saturating(other.trits[1]),
                self.trits[2].add_saturating(other.trits[2]),
            ],
        }
    }

    /// Arithmetic addition with carry out
    pub const fn add_with_carry(self, other: Tryte3, carry_in: Trit) -> (Tryte3, Trit) {
        let (t0, c0) = self.trits[0].add_with_carry(other.trits[0], carry_in);
        let (t1, c1) = self.trits[1].add_with_carry(other.trits[1], c0);
        let (t2, c2) = self.trits[2].add_with_carry(other.trits[2], c1);

        (
            Tryte3 {
                trits: [t0, t1, t2],
            },
            c2,
        )
    }

    /// Dot product (for similarity)
    #[inline]
    pub const fn dot(self, other: Tryte3) -> i8 {
        self.trits[0].to_i8() * other.trits[0].to_i8()
            + self.trits[1].to_i8() * other.trits[1].to_i8()
            + self.trits[2].to_i8() * other.trits[2].to_i8()
    }

    /// Count non-zero trits
    #[inline]
    pub const fn nnz(self) -> u8 {
        self.trits[0].is_nonzero() as u8
            + self.trits[1].is_nonzero() as u8
            + self.trits[2].is_nonzero() as u8
    }

    /// Pack into a single byte (5 bits needed for 27 states)
    /// Returns value 0-26
    #[inline]
    pub const fn pack(self) -> u8 {
        // Map each trit: N->0, Z->1, P->2, then compute base-3 number
        let t0 = (self.trits[0].to_i8() + 1) as u8; // 0, 1, 2
        let t1 = (self.trits[1].to_i8() + 1) as u8;
        let t2 = (self.trits[2].to_i8() + 1) as u8;
        t0 + 3 * t1 + 9 * t2
    }

    /// Unpack from byte (value 0-26)
    #[inline]
    pub const fn unpack(byte: u8) -> Option<Self> {
        if byte >= 27 {
            return None;
        }

        let t0 = (byte % 3) as i8 - 1;
        let t1 = ((byte / 3) % 3) as i8 - 1;
        let t2 = (byte / 9) as i8 - 1;

        Some(Tryte3 {
            trits: [
                Trit::from_i8_clamped(t0),
                Trit::from_i8_clamped(t1),
                Trit::from_i8_clamped(t2),
            ],
        })
    }
}

impl std::ops::Neg for Tryte3 {
    type Output = Tryte3;
    fn neg(self) -> Tryte3 {
        Tryte3::neg(self)
    }
}

impl std::ops::Mul for Tryte3 {
    type Output = Tryte3;
    fn mul(self, rhs: Tryte3) -> Tryte3 {
        Tryte3::mul(self, rhs)
    }
}

/// A word: 6 trits = 729 states ≈ 9.51 bits
///
/// Range: -364 to +364 in balanced ternary
///
/// This fits nicely in operations and provides good precision
/// for coefficients and residuals.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Word6 {
    /// Two trytes
    pub low: Tryte3,
    pub high: Tryte3,
}

impl fmt::Debug for Word6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Word6[{}{} = {}]", self.high, self.low, self.to_i16())
    }
}

impl Word6 {
    /// Zero word
    pub const ZERO: Word6 = Word6 {
        low: Tryte3::ZERO,
        high: Tryte3::ZERO,
    };

    /// Maximum value: ++++++ = 364
    pub const MAX_VALUE: i16 = 364;

    /// Minimum value: ------ = -364
    pub const MIN_VALUE: i16 = -364;

    /// Number of distinct states
    pub const NUM_STATES: u16 = 729;

    /// Create from integer value
    pub fn from_i16(value: i16) -> Option<Self> {
        if !(Self::MIN_VALUE..=Self::MAX_VALUE).contains(&value) {
            return None;
        }

        // Split into low and high trytes
        // low = value mod 27 (in balanced ternary sense)
        // high = value / 27

        let mut v = value;
        let negative = v < 0;
        if negative {
            v = -v;
        }

        // Convert to base-27 with balanced representation
        let low_val = balanced_mod(v, 27);
        let high_val = balanced_div(v, 27);

        let low = Tryte3::from_i8(if negative { -low_val } else { low_val })?;
        let high = Tryte3::from_i8(if negative { -high_val } else { high_val })?;

        Some(Word6 { low, high })
    }

    /// Convert to integer value
    #[inline]
    pub fn to_i16(self) -> i16 {
        self.low.to_i8() as i16 + 27 * self.high.to_i8() as i16
    }

    /// Trit-wise multiplication (bind)
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Word6) -> Word6 {
        Word6 {
            low: self.low.mul(other.low),
            high: self.high.mul(other.high),
        }
    }

    /// Trit-wise majority (bundle)
    pub fn bundle(self, other: Word6) -> Word6 {
        Word6 {
            low: self.low.bundle(other.low),
            high: self.high.bundle(other.high),
        }
    }

    /// Pack into 10 bits (stored in u16)
    pub fn pack(self) -> u16 {
        self.low.pack() as u16 + 27 * self.high.pack() as u16
    }

    /// Unpack from 10 bits
    pub fn unpack(bits: u16) -> Option<Self> {
        if bits >= 729 {
            return None;
        }
        let low = Tryte3::unpack((bits % 27) as u8)?;
        let high = Tryte3::unpack((bits / 27) as u8)?;
        Some(Word6 { low, high })
    }
}

/// Balanced modulo: result in range [-(n-1)/2, (n-1)/2]
const fn balanced_mod(value: i16, n: i16) -> i8 {
    let r = value % n;
    if r > n / 2 {
        (r - n) as i8
    } else if r < -(n / 2) {
        (r + n) as i8
    } else {
        r as i8
    }
}

/// Balanced division companion to balanced_mod
const fn balanced_div(value: i16, n: i16) -> i8 {
    let r = balanced_mod(value, n);
    ((value - r as i16) / n) as i8
}

/// Reconstruction correction entry
///
/// This is the key to 100% reconstruction guarantee.
/// When VSA operations produce approximation errors,
/// we store exact corrections here.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrectionEntry {
    /// Position in the data
    pub position: u64,
    /// The exact value that should be there
    pub exact_value: Vec<u8>,
    /// Hash of the original for verification
    pub verification_hash: [u8; 8],
}

/// Parity trit for error detection
///
/// Computed as: sum of all trits mod 3, balanced
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParityTrit(pub Trit);

impl ParityTrit {
    /// Compute parity for a slice of trits
    pub fn compute(trits: &[Trit]) -> Self {
        let sum: i32 = trits.iter().map(|t| t.to_i8() as i32).sum();
        let parity = sum.rem_euclid(3); // Ensure positive
        ParityTrit(match parity {
            0 => Trit::Z,
            1 => Trit::P,
            2 => Trit::N,
            _ => unreachable!(),
        })
    }

    /// Verify parity matches
    pub fn verify(&self, trits: &[Trit]) -> bool {
        Self::compute(trits) == *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TRIT TESTS ====================

    #[test]
    fn test_trit_multiplication_truth_table() {
        // Complete truth table verification
        assert_eq!(Trit::N * Trit::N, Trit::P, "N × N = P");
        assert_eq!(Trit::N * Trit::Z, Trit::Z, "N × Z = Z");
        assert_eq!(Trit::N * Trit::P, Trit::N, "N × P = N");
        assert_eq!(Trit::Z * Trit::N, Trit::Z, "Z × N = Z");
        assert_eq!(Trit::Z * Trit::Z, Trit::Z, "Z × Z = Z");
        assert_eq!(Trit::Z * Trit::P, Trit::Z, "Z × P = Z");
        assert_eq!(Trit::P * Trit::N, Trit::N, "P × N = N");
        assert_eq!(Trit::P * Trit::Z, Trit::Z, "P × Z = Z");
        assert_eq!(Trit::P * Trit::P, Trit::P, "P × P = P");
    }

    #[test]
    fn test_trit_self_inverse() {
        // Key VSA property: a × a = P for non-zero a
        assert_eq!(Trit::P * Trit::P, Trit::P, "P is self-inverse");
        assert_eq!(Trit::N * Trit::N, Trit::P, "N is self-inverse");
    }

    #[test]
    fn test_trit_multiplication_commutative() {
        for &a in &Trit::ALL {
            for &b in &Trit::ALL {
                assert_eq!(a * b, b * a, "Commutativity: {:?} × {:?}", a, b);
            }
        }
    }

    #[test]
    fn test_trit_multiplication_associative() {
        for &a in &Trit::ALL {
            for &b in &Trit::ALL {
                for &c in &Trit::ALL {
                    assert_eq!(
                        (a * b) * c,
                        a * (b * c),
                        "Associativity: ({:?} × {:?}) × {:?}",
                        a,
                        b,
                        c
                    );
                }
            }
        }
    }

    #[test]
    fn test_trit_negation() {
        assert_eq!(-Trit::N, Trit::P);
        assert_eq!(-Trit::Z, Trit::Z);
        assert_eq!(-Trit::P, Trit::N);

        // Double negation is identity
        for &t in &Trit::ALL {
            assert_eq!(-(-t), t, "Double negation of {:?}", t);
        }
    }

    #[test]
    fn test_trit_add_with_carry_complete() {
        // Verify all 27 combinations
        #[allow(clippy::type_complexity)]
        let expected: [((Trit, Trit, Trit), (Trit, Trit)); 27] = [
            // carry_in = N
            ((Trit::N, Trit::N, Trit::N), (Trit::Z, Trit::N)), // -3
            ((Trit::N, Trit::Z, Trit::N), (Trit::P, Trit::N)), // -2
            ((Trit::N, Trit::P, Trit::N), (Trit::N, Trit::Z)), // -1
            ((Trit::Z, Trit::N, Trit::N), (Trit::P, Trit::N)), // -2
            ((Trit::Z, Trit::Z, Trit::N), (Trit::N, Trit::Z)), // -1
            ((Trit::Z, Trit::P, Trit::N), (Trit::Z, Trit::Z)), // 0
            ((Trit::P, Trit::N, Trit::N), (Trit::N, Trit::Z)), // -1
            ((Trit::P, Trit::Z, Trit::N), (Trit::Z, Trit::Z)), // 0
            ((Trit::P, Trit::P, Trit::N), (Trit::P, Trit::Z)), // 1
            // carry_in = Z
            ((Trit::N, Trit::N, Trit::Z), (Trit::P, Trit::N)), // -2
            ((Trit::N, Trit::Z, Trit::Z), (Trit::N, Trit::Z)), // -1
            ((Trit::N, Trit::P, Trit::Z), (Trit::Z, Trit::Z)), // 0
            ((Trit::Z, Trit::N, Trit::Z), (Trit::N, Trit::Z)), // -1
            ((Trit::Z, Trit::Z, Trit::Z), (Trit::Z, Trit::Z)), // 0
            ((Trit::Z, Trit::P, Trit::Z), (Trit::P, Trit::Z)), // 1
            ((Trit::P, Trit::N, Trit::Z), (Trit::Z, Trit::Z)), // 0
            ((Trit::P, Trit::Z, Trit::Z), (Trit::P, Trit::Z)), // 1
            ((Trit::P, Trit::P, Trit::Z), (Trit::N, Trit::P)), // 2
            // carry_in = P
            ((Trit::N, Trit::N, Trit::P), (Trit::N, Trit::Z)), // -1
            ((Trit::N, Trit::Z, Trit::P), (Trit::Z, Trit::Z)), // 0
            ((Trit::N, Trit::P, Trit::P), (Trit::P, Trit::Z)), // 1
            ((Trit::Z, Trit::N, Trit::P), (Trit::Z, Trit::Z)), // 0
            ((Trit::Z, Trit::Z, Trit::P), (Trit::P, Trit::Z)), // 1
            ((Trit::Z, Trit::P, Trit::P), (Trit::N, Trit::P)), // 2
            ((Trit::P, Trit::N, Trit::P), (Trit::P, Trit::Z)), // 1
            ((Trit::P, Trit::Z, Trit::P), (Trit::N, Trit::P)), // 2
            ((Trit::P, Trit::P, Trit::P), (Trit::Z, Trit::P)), // 3
        ];

        for ((a, b, c), (expected_sum, expected_carry)) in expected {
            let (sum, carry) = a.add_with_carry(b, c);
            assert_eq!(
                (sum, carry),
                (expected_sum, expected_carry),
                "add_with_carry({:?}, {:?}, {:?})",
                a,
                b,
                c
            );
        }
    }

    // ==================== TRYTE3 TESTS ====================

    #[test]
    fn test_tryte3_roundtrip() {
        for v in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
            let tryte =
                Tryte3::from_i8(v).unwrap_or_else(|| panic!("Should create tryte for {}", v));
            let decoded = tryte.to_i8();
            assert_eq!(v, decoded, "Roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_tryte3_pack_unpack() {
        for packed in 0..27u8 {
            let tryte = Tryte3::unpack(packed).expect("Should unpack");
            let repacked = tryte.pack();
            assert_eq!(packed, repacked, "Pack/unpack failed for {}", packed);
        }
    }

    #[test]
    fn test_tryte3_bind_self_inverse() {
        for v in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
            let tryte = Tryte3::from_i8(v).unwrap();
            let bound = tryte * tryte;

            // Self-bind should produce all P (or Z for zero trits)
            for i in 0..3 {
                if tryte.trits[i].is_nonzero() {
                    assert_eq!(
                        bound.trits[i],
                        Trit::P,
                        "Self-bind trit {} should be P for value {}",
                        i,
                        v
                    );
                }
            }
        }
    }

    // ==================== WORD6 TESTS ====================

    #[test]
    fn test_word6_roundtrip() {
        let test_values = [0, 1, -1, 13, -13, 100, -100, 364, -364];
        for &v in &test_values {
            let word = Word6::from_i16(v).unwrap_or_else(|| panic!("Should create word for {}", v));
            let decoded = word.to_i16();
            assert_eq!(v, decoded, "Roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_word6_pack_unpack() {
        for packed in (0..729u16).step_by(7) {
            // Sample every 7th value
            let word = Word6::unpack(packed).expect("Should unpack");
            let repacked = word.pack();
            assert_eq!(packed, repacked, "Pack/unpack failed for {}", packed);
        }
    }

    // ==================== PARITY TESTS ====================

    #[test]
    fn test_parity_detection() {
        let trits = vec![Trit::P, Trit::N, Trit::P, Trit::Z, Trit::N];
        let parity = ParityTrit::compute(&trits);
        assert!(parity.verify(&trits), "Parity should verify");

        // Flip one trit and verify parity fails
        let mut corrupted = trits.clone();
        corrupted[0] = Trit::N;
        assert!(
            !parity.verify(&corrupted),
            "Parity should fail on corrupted data"
        );
    }
}
