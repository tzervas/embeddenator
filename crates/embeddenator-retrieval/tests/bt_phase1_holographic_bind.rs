//! BT-phase-1 holographic bind guard.
//!
//! Retrieval must keep using `SparseVec::bind` as trit multiplication
//! (`P*P=P`, self-inverse). A future mixup with trit-vsa bind
//! (`P.bind(P)=Z`) would make `a.bind(&a)` the zero vector and
//! `cosine(a, a.bind(&a))` collapse to ~0.

use embeddenator_vsa::SparseVec;

#[test]
fn holographic_self_bind_is_not_zero() {
    // All-positive support: mul maps P*P → P, so self-bind is identity.
    // trit-vsa bind maps P.bind(P) → Z, so self-bind would be empty.
    let a = SparseVec {
        pos: vec![1, 5, 9, 20, 42],
        neg: vec![],
    };
    let bound = a.bind(&a);

    assert!(
        !bound.pos.is_empty() || !bound.neg.is_empty(),
        "SparseVec::bind must stay holographic (P*P=P); trit-vsa bind would yield the zero vector"
    );
    assert_eq!(
        bound.pos, a.pos,
        "all-positive self-bind is identity under mul"
    );
    assert!(bound.neg.is_empty());

    let sim = a.cosine(&bound);
    assert!(
        sim > 0.9,
        "cosine(a, a.bind(&a)) must stay high under holographic mul, got {sim}"
    );
}

#[test]
fn holographic_self_bind_mixed_support_not_zero() {
    // P*P=P and N*N=P: self-bind is all-positive on the union of support, never empty.
    let a = SparseVec {
        pos: vec![2, 8, 16],
        neg: vec![4, 10],
    };
    let bound = a.bind(&a);
    assert_eq!(bound.pos, vec![2, 4, 8, 10, 16]);
    assert!(bound.neg.is_empty());
    assert!(
        a.cosine(&bound) > 0.0,
        "holographic self-bind must not be orthogonal/zero vs a"
    );
}
