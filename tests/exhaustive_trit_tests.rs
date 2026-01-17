//! Exhaustive Trit Layer Tests
//!
//! This test module provides complete coverage of the balanced ternary
//! foundation layer. Every algebraic property must be verified.

use embeddenator::ternary::{ParityTrit, Trit, Tryte3, Word6};

/// All 27 possible (a, b, carry_in) combinations for add_with_carry
const ALL_ADD_CASES: [((Trit, Trit, Trit), (Trit, Trit)); 27] = [
    // carry_in = N (-1)
    ((Trit::N, Trit::N, Trit::N), (Trit::Z, Trit::N)), // -1 + -1 + -1 = -3 = 0 + 3×(-1)
    ((Trit::N, Trit::Z, Trit::N), (Trit::P, Trit::N)), // -1 + 0 + -1 = -2 = 1 + 3×(-1)
    ((Trit::N, Trit::P, Trit::N), (Trit::N, Trit::Z)), // -1 + 1 + -1 = -1 = -1 + 3×0
    ((Trit::Z, Trit::N, Trit::N), (Trit::P, Trit::N)), // 0 + -1 + -1 = -2 = 1 + 3×(-1)
    ((Trit::Z, Trit::Z, Trit::N), (Trit::N, Trit::Z)), // 0 + 0 + -1 = -1 = -1 + 3×0
    ((Trit::Z, Trit::P, Trit::N), (Trit::Z, Trit::Z)), // 0 + 1 + -1 = 0 = 0 + 3×0
    ((Trit::P, Trit::N, Trit::N), (Trit::N, Trit::Z)), // 1 + -1 + -1 = -1 = -1 + 3×0
    ((Trit::P, Trit::Z, Trit::N), (Trit::Z, Trit::Z)), // 1 + 0 + -1 = 0 = 0 + 3×0
    ((Trit::P, Trit::P, Trit::N), (Trit::P, Trit::Z)), // 1 + 1 + -1 = 1 = 1 + 3×0
    // carry_in = Z (0)
    ((Trit::N, Trit::N, Trit::Z), (Trit::P, Trit::N)), // -1 + -1 + 0 = -2 = 1 + 3×(-1)
    ((Trit::N, Trit::Z, Trit::Z), (Trit::N, Trit::Z)), // -1 + 0 + 0 = -1 = -1 + 3×0
    ((Trit::N, Trit::P, Trit::Z), (Trit::Z, Trit::Z)), // -1 + 1 + 0 = 0 = 0 + 3×0
    ((Trit::Z, Trit::N, Trit::Z), (Trit::N, Trit::Z)), // 0 + -1 + 0 = -1 = -1 + 3×0
    ((Trit::Z, Trit::Z, Trit::Z), (Trit::Z, Trit::Z)), // 0 + 0 + 0 = 0 = 0 + 3×0
    ((Trit::Z, Trit::P, Trit::Z), (Trit::P, Trit::Z)), // 0 + 1 + 0 = 1 = 1 + 3×0
    ((Trit::P, Trit::N, Trit::Z), (Trit::Z, Trit::Z)), // 1 + -1 + 0 = 0 = 0 + 3×0
    ((Trit::P, Trit::Z, Trit::Z), (Trit::P, Trit::Z)), // 1 + 0 + 0 = 1 = 1 + 3×0
    ((Trit::P, Trit::P, Trit::Z), (Trit::N, Trit::P)), // 1 + 1 + 0 = 2 = -1 + 3×1
    // carry_in = P (+1)
    ((Trit::N, Trit::N, Trit::P), (Trit::N, Trit::Z)), // -1 + -1 + 1 = -1 = -1 + 3×0
    ((Trit::N, Trit::Z, Trit::P), (Trit::Z, Trit::Z)), // -1 + 0 + 1 = 0 = 0 + 3×0
    ((Trit::N, Trit::P, Trit::P), (Trit::P, Trit::Z)), // -1 + 1 + 1 = 1 = 1 + 3×0
    ((Trit::Z, Trit::N, Trit::P), (Trit::Z, Trit::Z)), // 0 + -1 + 1 = 0 = 0 + 3×0
    ((Trit::Z, Trit::Z, Trit::P), (Trit::P, Trit::Z)), // 0 + 0 + 1 = 1 = 1 + 3×0
    ((Trit::Z, Trit::P, Trit::P), (Trit::N, Trit::P)), // 0 + 1 + 1 = 2 = -1 + 3×1
    ((Trit::P, Trit::N, Trit::P), (Trit::P, Trit::Z)), // 1 + -1 + 1 = 1 = 1 + 3×0
    ((Trit::P, Trit::Z, Trit::P), (Trit::N, Trit::P)), // 1 + 0 + 1 = 2 = -1 + 3×1
    ((Trit::P, Trit::P, Trit::P), (Trit::Z, Trit::P)), // 1 + 1 + 1 = 3 = 0 + 3×1
];

// ==================== TRIT TESTS ====================

#[test]
fn test_all_27_add_with_carry_cases() {
    for ((a, b, c), (expected_sum, expected_carry)) in ALL_ADD_CASES {
        let (sum, carry) = a.add_with_carry(b, c);

        // Verify the output
        assert_eq!(
            (sum, carry),
            (expected_sum, expected_carry),
            "add_with_carry({:?}, {:?}, {:?}) = ({:?}, {:?}), expected ({:?}, {:?})",
            a,
            b,
            c,
            sum,
            carry,
            expected_sum,
            expected_carry
        );

        // Verify mathematical correctness: a + b + c = sum + 3*carry
        let input_sum = a.to_i8() + b.to_i8() + c.to_i8();
        let output_val = sum.to_i8() + 3 * carry.to_i8();
        assert_eq!(
            input_sum as i16,
            output_val as i16,
            "Mathematical verification failed: {} + {} + {} = {} ≠ {} + 3×{} = {}",
            a.to_i8(),
            b.to_i8(),
            c.to_i8(),
            input_sum,
            sum.to_i8(),
            carry.to_i8(),
            output_val
        );
    }
}

#[test]
fn test_multiplication_complete_truth_table() {
    // Complete 3×3 truth table
    let expected: [[Trit; 3]; 3] = [
        // N, Z, P (first operand)
        [Trit::P, Trit::Z, Trit::N], // N × {N, Z, P}
        [Trit::Z, Trit::Z, Trit::Z], // Z × {N, Z, P}
        [Trit::N, Trit::Z, Trit::P], // P × {N, Z, P}
    ];

    for (i, &a) in [Trit::N, Trit::Z, Trit::P].iter().enumerate() {
        for (j, &b) in [Trit::N, Trit::Z, Trit::P].iter().enumerate() {
            let result = a * b;
            assert_eq!(
                result, expected[i][j],
                "{:?} × {:?} = {:?}, expected {:?}",
                a, b, result, expected[i][j]
            );
        }
    }
}

#[test]
fn test_multiplication_commutativity_exhaustive() {
    for &a in &[Trit::N, Trit::Z, Trit::P] {
        for &b in &[Trit::N, Trit::Z, Trit::P] {
            assert_eq!(
                a * b,
                b * a,
                "Commutativity: {:?} × {:?} ≠ {:?} × {:?}",
                a,
                b,
                b,
                a
            );
        }
    }
}

#[test]
fn test_multiplication_associativity_exhaustive() {
    for &a in &[Trit::N, Trit::Z, Trit::P] {
        for &b in &[Trit::N, Trit::Z, Trit::P] {
            for &c in &[Trit::N, Trit::Z, Trit::P] {
                assert_eq!(
                    (a * b) * c,
                    a * (b * c),
                    "Associativity: ({:?} × {:?}) × {:?} ≠ {:?} × ({:?} × {:?})",
                    a,
                    b,
                    c,
                    a,
                    b,
                    c
                );
            }
        }
    }
}

#[test]
fn test_multiplication_identity() {
    // P (positive/+1) is the multiplicative identity
    for &a in &[Trit::N, Trit::Z, Trit::P] {
        assert_eq!(a * Trit::P, a, "Identity: {:?} × P = {:?}", a, a * Trit::P);
        assert_eq!(Trit::P * a, a, "Identity: P × {:?} = {:?}", a, Trit::P * a);
    }
}

#[test]
fn test_multiplication_zero_annihilator() {
    // Z (zero) annihilates everything
    for &a in &[Trit::N, Trit::Z, Trit::P] {
        assert_eq!(
            a * Trit::Z,
            Trit::Z,
            "Zero: {:?} × Z = {:?}",
            a,
            a * Trit::Z
        );
        assert_eq!(
            Trit::Z * a,
            Trit::Z,
            "Zero: Z × {:?} = {:?}",
            a,
            Trit::Z * a
        );
    }
}

#[test]
fn test_multiplication_self_inverse() {
    // a × a = P for non-zero a (self-inverse property crucial for VSA bind)
    assert_eq!(Trit::P * Trit::P, Trit::P, "P × P = P");
    assert_eq!(Trit::N * Trit::N, Trit::P, "N × N = P");
    // Z × Z = Z (not P, but Z is not invertible)
    assert_eq!(Trit::Z * Trit::Z, Trit::Z, "Z × Z = Z");
}

#[test]
fn test_negation_involutive() {
    // Double negation is identity
    for &a in &[Trit::N, Trit::Z, Trit::P] {
        assert_eq!(-(-a), a, "Double negation: -(-{:?}) = {:?}", a, -(-a));
    }
}

#[test]
fn test_negation_values() {
    assert_eq!(-Trit::N, Trit::P);
    assert_eq!(-Trit::Z, Trit::Z);
    assert_eq!(-Trit::P, Trit::N);
}

#[test]
fn test_trit_from_i8_exact() {
    assert_eq!(Trit::from_i8_exact(-1), Some(Trit::N));
    assert_eq!(Trit::from_i8_exact(0), Some(Trit::Z));
    assert_eq!(Trit::from_i8_exact(1), Some(Trit::P));
    assert_eq!(Trit::from_i8_exact(-2), None);
    assert_eq!(Trit::from_i8_exact(2), None);
}

#[test]
fn test_trit_from_i8_clamped() {
    assert_eq!(Trit::from_i8_clamped(-100), Trit::N);
    assert_eq!(Trit::from_i8_clamped(-1), Trit::N);
    assert_eq!(Trit::from_i8_clamped(0), Trit::Z);
    assert_eq!(Trit::from_i8_clamped(1), Trit::P);
    assert_eq!(Trit::from_i8_clamped(100), Trit::P);
}

#[test]
fn test_majority3() {
    // All same
    assert_eq!(Trit::majority3(Trit::P, Trit::P, Trit::P), Trit::P);
    assert_eq!(Trit::majority3(Trit::N, Trit::N, Trit::N), Trit::N);
    assert_eq!(Trit::majority3(Trit::Z, Trit::Z, Trit::Z), Trit::Z);

    // Two vs one
    assert_eq!(Trit::majority3(Trit::P, Trit::P, Trit::N), Trit::P);
    assert_eq!(Trit::majority3(Trit::N, Trit::N, Trit::P), Trit::N);

    // Balanced (cancels out)
    assert_eq!(Trit::majority3(Trit::P, Trit::Z, Trit::N), Trit::Z);
}

// ==================== TRYTE3 TESTS (3 trits = 27 states) ====================

#[test]
fn test_tryte3_all_27_values() {
    // Verify all 27 values from -13 to +13 roundtrip correctly
    for v in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
        let tryte =
            Tryte3::from_i8(v).unwrap_or_else(|| panic!("Failed to create Tryte3 for {}", v));
        let decoded = tryte.to_i8();
        assert_eq!(v, decoded, "Tryte3 roundtrip failed for {}", v);
    }
}

#[test]
fn test_tryte3_pack_all_27() {
    // Verify all 27 pack/unpack values
    for packed in 0..27u8 {
        let tryte = Tryte3::unpack(packed).unwrap_or_else(|| panic!("Failed to unpack {}", packed));
        let repacked = tryte.pack();
        assert_eq!(packed, repacked, "Tryte3 pack/unpack failed for {}", packed);
    }
}

#[test]
fn test_tryte3_pack_value_correspondence() {
    // Verify that packed value corresponds to expected tryte value
    for v in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
        let tryte = Tryte3::from_i8(v).unwrap();
        let packed = tryte.pack();
        let unpacked = Tryte3::unpack(packed).unwrap();
        assert_eq!(
            tryte, unpacked,
            "Pack/value correspondence failed for {}",
            v
        );
    }
}

#[test]
fn test_tryte3_bind_self_produces_all_positive() {
    for v in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
        let tryte = Tryte3::from_i8(v).unwrap();
        let bound = tryte * tryte;

        // Self-bind should produce P for each non-zero trit
        for i in 0..3 {
            if tryte.trits[i].is_nonzero() {
                assert_eq!(
                    bound.trits[i],
                    Trit::P,
                    "Self-bind trit {} should be P for value {}",
                    i,
                    v
                );
            } else {
                assert_eq!(
                    bound.trits[i],
                    Trit::Z,
                    "Self-bind trit {} should be Z for value {}",
                    i,
                    v
                );
            }
        }
    }
}

#[test]
fn test_tryte3_bind_commutativity() {
    for v1 in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
        for v2 in Tryte3::MIN_VALUE..=Tryte3::MAX_VALUE {
            let a = Tryte3::from_i8(v1).unwrap();
            let b = Tryte3::from_i8(v2).unwrap();
            assert_eq!(a * b, b * a, "Tryte3 bind not commutative: {} × {}", v1, v2);
        }
    }
}

#[test]
fn test_tryte3_arithmetic_add() {
    // Test arithmetic addition with carry
    let a = Tryte3::from_i8(10).unwrap();
    let b = Tryte3::from_i8(3).unwrap();
    let (sum, carry) = a.add_with_carry(b, Trit::Z);

    // 10 + 3 = 13, which is within range, so carry should be Z
    assert_eq!(sum.to_i8(), 13);
    assert_eq!(carry, Trit::Z);
}

#[test]
fn test_tryte3_arithmetic_overflow() {
    // Test overflow
    let a = Tryte3::from_i8(13).unwrap(); // MAX
    let b = Tryte3::from_i8(1).unwrap();
    let (_sum, carry) = a.add_with_carry(b, Trit::Z);

    // 13 + 1 = 14 = -13 + 27 = -13 + 3×9 → sum = -13, carry needs to propagate
    // In balanced ternary: 13 = PPP, 1 = P00
    // PPP + P00 with carries...
    assert_eq!(carry, Trit::P, "Overflow should produce positive carry");
}

#[test]
fn test_tryte3_dot_product() {
    let a = Tryte3::from_i8(13).unwrap(); // PPP
    let b = Tryte3::from_i8(13).unwrap(); // PPP
                                          // Dot product = 1×1 + 1×1 + 1×1 = 3
    assert_eq!(a.dot(b), 3);

    let c = Tryte3::from_i8(-13).unwrap(); // NNN
                                           // Dot with opposite = -1×1 + -1×1 + -1×1 = -3
    assert_eq!(a.dot(c), -3);
}

#[test]
fn test_tryte3_nonzero_count() {
    assert_eq!(Tryte3::from_i8(0).unwrap().nnz(), 0);
    assert_eq!(Tryte3::from_i8(1).unwrap().nnz(), 1); // P00
    assert_eq!(Tryte3::from_i8(3).unwrap().nnz(), 1); // 0P0
    assert_eq!(Tryte3::from_i8(13).unwrap().nnz(), 3); // PPP
}

// ==================== WORD6 TESTS (6 trits = 729 states) ====================

#[test]
fn test_word6_boundary_values() {
    let test_vals = [0i16, 1, -1, 13, -13, 27, -27, 100, -100, 364, -364];
    for &v in &test_vals {
        let word = Word6::from_i16(v).unwrap_or_else(|| panic!("Failed to create Word6 for {}", v));
        let decoded = word.to_i16();
        assert_eq!(v, decoded, "Word6 roundtrip failed for {}", v);
    }
}

#[test]
fn test_word6_pack_unpack_sample() {
    // Test a sample of the 729 values
    for packed in (0..729u16).step_by(7) {
        let word = Word6::unpack(packed).unwrap_or_else(|| panic!("Failed to unpack {}", packed));
        let repacked = word.pack();
        assert_eq!(packed, repacked, "Word6 pack/unpack failed for {}", packed);
    }
}

#[test]
fn test_word6_bind_commutativity() {
    let test_vals = [0i16, 1, -1, 50, -50, 100, -100, 200, -200];
    for &v1 in &test_vals {
        for &v2 in &test_vals {
            let a = Word6::from_i16(v1).unwrap();
            let b = Word6::from_i16(v2).unwrap();
            assert_eq!(
                a.mul(b),
                b.mul(a),
                "Word6 bind not commutative: {} × {}",
                v1,
                v2
            );
        }
    }
}

// ==================== PARITY TESTS ====================

#[test]
fn test_parity_detection_all_single_flips() {
    let trits = vec![Trit::P, Trit::N, Trit::P, Trit::Z, Trit::N, Trit::P];
    let parity = ParityTrit::compute(&trits);

    // Verify parity detects single trit changes
    for i in 0..trits.len() {
        for &new_val in &[Trit::N, Trit::Z, Trit::P] {
            if new_val != trits[i] {
                let mut corrupted = trits.clone();
                corrupted[i] = new_val;
                assert!(
                    !parity.verify(&corrupted),
                    "Parity should detect flip at position {} from {:?} to {:?}",
                    i,
                    trits[i],
                    new_val
                );
            }
        }
    }
}

#[test]
fn test_parity_all_zero() {
    let trits = vec![Trit::Z; 10];
    let parity = ParityTrit::compute(&trits);
    assert!(parity.verify(&trits));
    assert_eq!(parity.0, Trit::Z);
}

#[test]
fn test_parity_balanced() {
    // Equal positive and negative should sum to zero
    let trits = vec![Trit::P, Trit::N, Trit::P, Trit::N];
    let parity = ParityTrit::compute(&trits);
    assert!(parity.verify(&trits));
    assert_eq!(parity.0, Trit::Z);
}

// ==================== ALGEBRAIC CLOSURE TESTS ====================

#[test]
fn test_trit_operations_closure() {
    // Verify all operations on trits produce trits
    for &a in &[Trit::N, Trit::Z, Trit::P] {
        for &b in &[Trit::N, Trit::Z, Trit::P] {
            // Multiplication is closed
            let _mul: Trit = a * b;

            // Addition with carry produces trits
            for &c in &[Trit::N, Trit::Z, Trit::P] {
                let (sum, carry): (Trit, Trit) = a.add_with_carry(b, c);
                let _ = sum;
                let _ = carry;
            }
        }

        // Negation is closed
        let _neg: Trit = -a;
        let _abs: Trit = a.abs();
    }
}

#[test]
fn test_tryte3_operations_closure() {
    let vals = [Tryte3::MIN_VALUE, 0, Tryte3::MAX_VALUE];
    for &v1 in &vals {
        for &v2 in &vals {
            let a = Tryte3::from_i8(v1).unwrap();
            let b = Tryte3::from_i8(v2).unwrap();

            // Bind is closed
            let _bound: Tryte3 = a * b;

            // Bundle is closed
            let _bundled: Tryte3 = a.bundle(b);

            // Negation is closed
            let _neg: Tryte3 = -a;
        }
    }
}
