//! Lens contract invariant tests across all three ternary substrates:
//! - PackedTritVec (scalar packed arithmetic)
//! - BitslicedTritVec (SIMD-friendly bitsliced)
//! - Hybrid (CarrySaveBundle accumulator -> bitsliced finalize)
//!
//! These tests verify that fundamental VSA lens properties hold on each representation:
//! 1. Cosine self-similarity = 1.0
//! 2. Unrelated vectors near-orthogonal
//! 3. Bundle/bind preserve algebraic properties
//! 4. Cross-representation consistency (results should match across all three)

use embeddenator::{BitslicedTritVec, CarrySaveBundle, PackedTritVec, SparseVec, DIM};

/// Creates a deterministic pseudo-random sparse vector from a label.
/// Uses from_data (deprecated but stable) for true orthogonality testing.
fn make_sparse(label: &str) -> SparseVec {
    #[allow(deprecated)]
    SparseVec::from_data(label.as_bytes())
}

// ============================================================================
// PackedTritVec lens invariants
// ============================================================================

#[test]
fn packed_cosine_self_is_one() {
    let s = make_sparse("packed/self");
    let p = PackedTritVec::from_sparsevec(&s, DIM);
    let sim = p.cosine(&p);
    assert!((sim - 1.0).abs() < 1e-9, "packed cos(v,v) != 1: {sim}");
}

#[test]
fn packed_unrelated_near_orthogonal() {
    let mut max_abs = 0.0f64;
    for i in 0..200 {
        let a = PackedTritVec::from_sparsevec(&make_sparse(&format!("packed/a/{i}")), DIM);
        let b = PackedTritVec::from_sparsevec(&make_sparse(&format!("packed/b/{i}")), DIM);
        let sim = a.cosine(&b).abs();
        if sim > max_abs {
            max_abs = sim;
        }
    }
    assert!(max_abs < 0.20, "packed max |cos| too high: {max_abs}");
}

#[test]
fn packed_bundle_commutativity() {
    let a = PackedTritVec::from_sparsevec(&make_sparse("packed/comm/a"), DIM);
    let b = PackedTritVec::from_sparsevec(&make_sparse("packed/comm/b"), DIM);
    let ab = a.bundle(&b);
    let ba = b.bundle(&a);
    // Verify element-wise equality via dot (identical vectors have max dot).
    let d = ab.dot(&ba);
    let self_d = ab.dot(&ab);
    assert_eq!(d, self_d, "packed bundle not commutative");
}

#[test]
fn packed_bind_produces_orthogonal_result() {
    // In sparse ternary VSA, bind(a,b) creates a unique composite that is
    // approximately orthogonal to both a and b individually.
    let a = PackedTritVec::from_sparsevec(&make_sparse("packed/bind/a"), DIM);
    let b = PackedTritVec::from_sparsevec(&make_sparse("packed/bind/b"), DIM);
    let bound = a.bind(&b);

    // The bound result should be near-orthogonal to the inputs.
    let sim_a = bound.cosine(&a).abs();
    let sim_b = bound.cosine(&b).abs();
    assert!(sim_a < 0.30, "packed bind result too similar to a: {sim_a}");
    assert!(sim_b < 0.30, "packed bind result too similar to b: {sim_b}");
}

// ============================================================================
// BitslicedTritVec lens invariants
// ============================================================================

#[test]
fn bitsliced_cosine_self_is_one() {
    let s = make_sparse("bitsliced/self");
    let bs = BitslicedTritVec::from_sparse(&s, DIM);
    let sim = bs.cosine(&bs);
    assert!((sim - 1.0).abs() < 1e-9, "bitsliced cos(v,v) != 1: {sim}");
}

#[test]
fn bitsliced_unrelated_near_orthogonal() {
    let mut max_abs = 0.0f64;
    for i in 0..200 {
        let a = BitslicedTritVec::from_sparse(&make_sparse(&format!("bitsliced/a/{i}")), DIM);
        let b = BitslicedTritVec::from_sparse(&make_sparse(&format!("bitsliced/b/{i}")), DIM);
        let sim = a.cosine(&b).abs();
        if sim > max_abs {
            max_abs = sim;
        }
    }
    assert!(max_abs < 0.20, "bitsliced max |cos| too high: {max_abs}");
}

#[test]
fn bitsliced_bundle_commutativity() {
    let a = BitslicedTritVec::from_sparse(&make_sparse("bitsliced/comm/a"), DIM);
    let b = BitslicedTritVec::from_sparse(&make_sparse("bitsliced/comm/b"), DIM);
    let ab = a.bundle_dispatch(&b);
    let ba = b.bundle_dispatch(&a);
    let d = ab.dot(&ba);
    let self_d = ab.dot(&ab);
    assert_eq!(d, self_d, "bitsliced bundle not commutative");
}

#[test]
fn bitsliced_bind_produces_orthogonal_result() {
    // In sparse ternary VSA, bind(a,b) creates a unique composite that is
    // approximately orthogonal to both a and b individually.
    let a = BitslicedTritVec::from_sparse(&make_sparse("bitsliced/bind/a"), DIM);
    let b = BitslicedTritVec::from_sparse(&make_sparse("bitsliced/bind/b"), DIM);
    let bound = a.bind_dispatch(&b);

    let sim_a = bound.cosine(&a).abs();
    let sim_b = bound.cosine(&b).abs();
    assert!(sim_a < 0.30, "bitsliced bind result too similar to a: {sim_a}");
    assert!(sim_b < 0.30, "bitsliced bind result too similar to b: {sim_b}");
}

// ============================================================================
// Hybrid (CarrySaveBundle) lens invariants
// ============================================================================

#[test]
fn hybrid_bundle_matches_sequential() {
    let sa = make_sparse("hybrid/seq/a");
    let sb = make_sparse("hybrid/seq/b");
    let sc = make_sparse("hybrid/seq/c");

    let a = BitslicedTritVec::from_sparse(&sa, DIM);
    let b = BitslicedTritVec::from_sparse(&sb, DIM);
    let c = BitslicedTritVec::from_sparse(&sc, DIM);

    // Sequential bundle: ((a ⊕ b) ⊕ c)
    let seq = a.bundle_dispatch(&b).bundle_dispatch(&c);

    // Hybrid carry-save bundle
    let mut acc = CarrySaveBundle::new(DIM);
    acc.accumulate(&a);
    acc.accumulate(&b);
    acc.accumulate(&c);
    let hybrid = acc.finalize();

    // Should produce the same result (or very close via tie-breaking).
    let sim = seq.cosine(&hybrid);
    assert!(sim > 0.99, "hybrid bundle != sequential bundle: {sim}");
}

#[test]
fn hybrid_cosine_self_is_one() {
    let s = make_sparse("hybrid/self");
    let bs = BitslicedTritVec::from_sparse(&s, DIM);
    let mut acc = CarrySaveBundle::new(DIM);
    acc.accumulate(&bs);
    let h = acc.finalize();
    let sim = h.cosine(&h);
    assert!((sim - 1.0).abs() < 1e-9, "hybrid cos(v,v) != 1: {sim}");
}

#[test]
fn hybrid_unrelated_near_orthogonal() {
    let mut max_abs = 0.0f64;
    for i in 0..200 {
        let a = BitslicedTritVec::from_sparse(&make_sparse(&format!("hybrid/a/{i}")), DIM);
        let b = BitslicedTritVec::from_sparse(&make_sparse(&format!("hybrid/b/{i}")), DIM);

        let mut acc_a = CarrySaveBundle::new(DIM);
        acc_a.accumulate(&a);
        let ha = acc_a.finalize();

        let mut acc_b = CarrySaveBundle::new(DIM);
        acc_b.accumulate(&b);
        let hb = acc_b.finalize();

        let sim = ha.cosine(&hb).abs();
        if sim > max_abs {
            max_abs = sim;
        }
    }
    assert!(max_abs < 0.20, "hybrid max |cos| too high: {max_abs}");
}

// ============================================================================
// Cross-representation consistency
// ============================================================================

#[test]
fn cross_representation_cosine_consistency() {
    // Verify that the same logical vector has the same cosine regardless of representation.
    let sa = make_sparse("cross/a");
    let sb = make_sparse("cross/b");

    // SparseVec
    let sparse_sim = sa.cosine(&sb);

    // PackedTritVec
    let pa = PackedTritVec::from_sparsevec(&sa, DIM);
    let pb = PackedTritVec::from_sparsevec(&sb, DIM);
    let packed_sim = pa.cosine(&pb);

    // BitslicedTritVec
    let ba = BitslicedTritVec::from_sparse(&sa, DIM);
    let bb = BitslicedTritVec::from_sparse(&sb, DIM);
    let bitsliced_sim = ba.cosine(&bb);

    // All should match within FP tolerance.
    assert!(
        (sparse_sim - packed_sim).abs() < 1e-9,
        "sparse vs packed cosine mismatch: {sparse_sim} vs {packed_sim}"
    );
    assert!(
        (sparse_sim - bitsliced_sim).abs() < 1e-9,
        "sparse vs bitsliced cosine mismatch: {sparse_sim} vs {bitsliced_sim}"
    );
}

#[test]
fn cross_representation_bundle_consistency() {
    let sa = make_sparse("cross/bundle/a");
    let sb = make_sparse("cross/bundle/b");

    // SparseVec bundle
    let sparse_bundle = sa.bundle(&sb);

    // PackedTritVec bundle -> convert back to sparse for comparison
    let pa = PackedTritVec::from_sparsevec(&sa, DIM);
    let pb = PackedTritVec::from_sparsevec(&sb, DIM);
    let packed_bundle = pa.bundle(&pb);
    let packed_as_sparse = packed_bundle.to_sparsevec();

    // BitslicedTritVec bundle -> convert back to sparse
    let ba = BitslicedTritVec::from_sparse(&sa, DIM);
    let bb = BitslicedTritVec::from_sparse(&sb, DIM);
    let bitsliced_bundle = ba.bundle_dispatch(&bb);
    let bitsliced_as_sparse = bitsliced_bundle.to_sparse();

    // Compare via cosine (exact match not guaranteed due to tie-breaking, but should be very close).
    let sp_vs_pk = sparse_bundle.cosine(&packed_as_sparse);
    let sp_vs_bs = sparse_bundle.cosine(&bitsliced_as_sparse);

    assert!(
        sp_vs_pk > 0.99,
        "sparse vs packed bundle diverged: {sp_vs_pk}"
    );
    assert!(
        sp_vs_bs > 0.99,
        "sparse vs bitsliced bundle diverged: {sp_vs_bs}"
    );
}
