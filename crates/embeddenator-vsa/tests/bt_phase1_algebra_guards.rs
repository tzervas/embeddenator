//! BT-phase-1 algebra guards.
//!
//! Holographic bind = trit multiplication (self-inverse, `P*P=P`).
//! trit-vsa bind = subtraction mod 3 (`P.bind(P)=Z`).
//! These algebras must stay distinct.

use embeddenator_vsa::{PackedTritVec, SparseVec, Trit, DIM};

fn holographic_mul_table() -> [((Trit, Trit), Trit); 9] {
    [
        ((Trit::N, Trit::N), Trit::P),
        ((Trit::N, Trit::Z), Trit::Z),
        ((Trit::N, Trit::P), Trit::N),
        ((Trit::Z, Trit::N), Trit::Z),
        ((Trit::Z, Trit::Z), Trit::Z),
        ((Trit::Z, Trit::P), Trit::Z),
        ((Trit::P, Trit::N), Trit::N),
        ((Trit::P, Trit::Z), Trit::Z),
        ((Trit::P, Trit::P), Trit::P),
    ]
}

fn trit_vsa_bind_table() -> [((trit_vsa::Trit, trit_vsa::Trit), trit_vsa::Trit); 9] {
    [
        ((trit_vsa::Trit::N, trit_vsa::Trit::N), trit_vsa::Trit::Z),
        ((trit_vsa::Trit::N, trit_vsa::Trit::Z), trit_vsa::Trit::N),
        ((trit_vsa::Trit::N, trit_vsa::Trit::P), trit_vsa::Trit::P),
        ((trit_vsa::Trit::Z, trit_vsa::Trit::N), trit_vsa::Trit::P),
        ((trit_vsa::Trit::Z, trit_vsa::Trit::Z), trit_vsa::Trit::Z),
        ((trit_vsa::Trit::Z, trit_vsa::Trit::P), trit_vsa::Trit::N),
        ((trit_vsa::Trit::P, trit_vsa::Trit::N), trit_vsa::Trit::N),
        ((trit_vsa::Trit::P, trit_vsa::Trit::Z), trit_vsa::Trit::P),
        ((trit_vsa::Trit::P, trit_vsa::Trit::P), trit_vsa::Trit::Z),
    ]
}

#[test]
fn holographic_mul_truth_table() {
    for ((a, b), expected) in holographic_mul_table() {
        assert_eq!(a.mul(b), expected, "mul({a:?}, {b:?})");
        assert_eq!(a * b, expected, "Mul({a:?}, {b:?})");
    }
}

#[test]
fn trit_vsa_bind_truth_table_sub_mod_3() {
    // Call trit_vsa::Trit::bind only — never prelude, never holographic bind.
    for ((a, b), expected) in trit_vsa_bind_table() {
        assert_eq!(a.bind(b), expected, "trit_vsa::Trit::bind({a:?}, {b:?})");
    }
}

#[test]
fn algebras_diverge_on_p_times_p_vs_p_bind_p() {
    assert_eq!(Trit::P * Trit::P, Trit::P, "holographic P*P = P");
    assert_eq!(
        trit_vsa::Trit::P.bind(trit_vsa::Trit::P),
        trit_vsa::Trit::Z,
        "trit-vsa P.bind(P) = Z"
    );
    assert_ne!(
        (Trit::P * Trit::P).to_i8(),
        trit_vsa::Trit::P.bind(trit_vsa::Trit::P).value(),
        "P*P and P.bind(P) must disagree"
    );
}

#[test]
fn sparsevec_bind_is_mul_not_trit_vsa_bind() {
    // Self-bind under mul: every non-zero becomes P (self-inverse).
    // Under trit-vsa bind, P.bind(P)=Z and N.bind(N)=Z so the result is empty.
    let v = SparseVec {
        pos: vec![1, 5, 9],
        neg: vec![3, 7],
    };
    let bound = v.bind(&v);
    assert_eq!(
        bound.pos,
        vec![1, 3, 5, 7, 9],
        "SparseVec::bind is mul: self-bind yields P on support"
    );
    assert!(
        bound.neg.is_empty(),
        "SparseVec::bind is mul: self-bind has no negatives"
    );

    // Cross-bind on a shared index: P * N = N (mul), not P.bind(N)=N wait —
    // they happen to agree on P*N, so use P*P vs P.bind(P) via self-overlap.
    let a = SparseVec {
        pos: vec![0, 2],
        neg: vec![1],
    };
    let b = SparseVec {
        pos: vec![0, 1],
        neg: vec![2],
    };
    let ab = a.bind(&b);
    // idx 0: P*P = P
    // idx 1: N*P = N
    // idx 2: P*N = N
    assert_eq!(ab.pos, vec![0]);
    assert_eq!(ab.neg, vec![1, 2]);
}

#[test]
fn packed_wrap_bind_matches_sparse_bind() {
    let a = SparseVec {
        pos: vec![0, 4, 31, 32, 63],
        neg: vec![1, 5, 30, 64],
    };
    let b = SparseVec {
        pos: vec![0, 5, 31, 64, 100],
        neg: vec![1, 4, 32, 63],
    };
    let len = 128;
    let pa = PackedTritVec::from_sparsevec(&a, len);
    let pb = PackedTritVec::from_sparsevec(&b, len);
    let packed_bound = pa.bind(&pb).to_sparsevec();
    let sparse_bound = a.bind(&b);
    assert_eq!(packed_bound.pos, sparse_bound.pos);
    assert_eq!(packed_bound.neg, sparse_bound.neg);
}

#[test]
fn packed_wrap_roundtrip_sparse() {
    let v = SparseVec {
        pos: vec![0, 7, 31, 32, 99],
        neg: vec![2, 8, 30, 64],
    };
    let packed = PackedTritVec::from_sparsevec(&v, 128);
    let back = packed.to_sparsevec();
    assert_eq!(back.pos, v.pos);
    assert_eq!(back.neg, v.neg);
    assert_eq!(packed.len(), 128);
    assert_eq!(packed.get(0), Trit::P);
    assert_eq!(packed.get(2), Trit::N);
    assert_eq!(packed.get(3), Trit::Z);
}

#[test]
fn resonator_unbind_still_self_inverse() {
    // Resonator unbind = bind again (mul is self-inverse on shared support).
    // a.bind(a).bind(a) == a
    let a = SparseVec {
        pos: vec![10, 20, 30],
        neg: vec![11, 21, 40],
    };
    let recovered = a.bind(&a).bind(&a);
    assert_eq!(recovered.pos, a.pos);
    assert_eq!(recovered.neg, a.neg);

    // On identical support, bind is an involution: a.bind(b).bind(b) == a
    let b = SparseVec {
        pos: vec![10, 21, 40],
        neg: vec![11, 20, 30],
    };
    let recovered = a.bind(&b).bind(&b);
    assert_eq!(recovered.pos, a.pos);
    assert_eq!(recovered.neg, a.neg);
}

#[test]
fn permute_unchanged() {
    let v = SparseVec {
        pos: vec![1, 2, DIM - 1],
        neg: vec![3, 50],
    };
    let p = v.permute(1);
    assert_eq!(p.pos, vec![0, 2, 3]); // (DIM-1+1)%DIM = 0, then 2, 3
    assert_eq!(p.neg, vec![4, 51]);

    let back = p.inverse_permute(1);
    assert_eq!(back.pos, v.pos);
    assert_eq!(back.neg, v.neg);
}
