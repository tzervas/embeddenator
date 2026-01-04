#![cfg(feature = "bt-phase-1")]

// Use an explicit path to avoid ambiguity between helper modules.
#[path = "../support/bt_migration.rs"]
mod bt_migration;

use bt_migration::{mk_random_sparsevec, sparse_dot};
use embeddenator::{PackedTritVec, SparseVec, DIM};
use rand::SeedableRng;

#[test]
fn phase1_packed_dot_matches_sparse_dot_randomized() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0x0515_51D0_7u64);

    for _case in 0..400 {
        let a = mk_random_sparsevec(&mut rng, DIM, 200);
        let b = mk_random_sparsevec(&mut rng, DIM, 200);

        let packed_a = PackedTritVec::from_sparsevec(&a, DIM);
        let packed_b = PackedTritVec::from_sparsevec(&b, DIM);

        let got = packed_a.dot(&packed_b);
        let expect = sparse_dot(&a, &b);
        assert_eq!(got, expect);
    }
}

#[test]
fn phase1_packed_bind_matches_sparse_bind_randomized() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xB11D_B11Du64);

    for _case in 0..250 {
        let a = mk_random_sparsevec(&mut rng, DIM, 200);
        let b = mk_random_sparsevec(&mut rng, DIM, 200);

        let packed_a = PackedTritVec::from_sparsevec(&a, DIM);
        let packed_b = PackedTritVec::from_sparsevec(&b, DIM);

        let got = packed_a.bind(&packed_b).to_sparsevec();
        let expect = a.bind(&b);

        assert_eq!(got.pos, expect.pos);
        assert_eq!(got.neg, expect.neg);
    }
}

#[test]
fn phase1_packed_bundle_matches_sparse_bundle_randomized() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xB00D_01EEu64);

    for _case in 0..250 {
        let a = mk_random_sparsevec(&mut rng, DIM, 200);
        let b = mk_random_sparsevec(&mut rng, DIM, 200);

        let packed_a = PackedTritVec::from_sparsevec(&a, DIM);
        let packed_b = PackedTritVec::from_sparsevec(&b, DIM);

        let got = packed_a.bundle(&packed_b).to_sparsevec();
        let expect = a.bundle(&b);

        assert_eq!(got.pos, expect.pos);
        assert_eq!(got.neg, expect.neg);
    }
}

#[test]
fn phase1_zero_vectors_behave() {
    let a = SparseVec::new();
    let b = SparseVec::new();

    let packed_a = PackedTritVec::from_sparsevec(&a, DIM);
    let packed_b = PackedTritVec::from_sparsevec(&b, DIM);

    assert_eq!(packed_a.dot(&packed_b), 0);

    let bind = packed_a.bind(&packed_b).to_sparsevec();
    assert!(bind.pos.is_empty() && bind.neg.is_empty());

    let bundle = packed_a.bundle(&packed_b).to_sparsevec();
    assert!(bundle.pos.is_empty() && bundle.neg.is_empty());
}
