//! Tests for bitsliced ↔ packed ↔ sparse conversions and operation equivalence.
//!
//! These tests ensure that BitslicedTritVec produces identical results to
//! PackedTritVec for all VSA operations, validating the refactoring.

use embeddenator::{BitslicedTritVec, CarrySaveBundle, PackedTritVec, SparseVec, Trit};

fn random_sparse(nnz: usize, dim: usize) -> SparseVec {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    let mut indices: Vec<usize> = (0..dim).collect();
    indices.shuffle(&mut rng);

    let mut pos: Vec<_> = indices[..nnz].to_vec();
    let mut neg: Vec<_> = indices[nnz..nnz * 2].to_vec();

    pos.sort_unstable();
    neg.sort_unstable();

    SparseVec { pos, neg }
}

#[test]
fn test_sparse_roundtrip_through_bitsliced() {
    for dim in [64, 100, 1000, 10000] {
        let sparse = random_sparse(dim / 100 + 1, dim);
        let bitsliced = BitslicedTritVec::from_sparse(&sparse, dim);
        let back = bitsliced.to_sparse();

        assert_eq!(back.pos, sparse.pos, "dim={dim}: pos mismatch");
        assert_eq!(back.neg, sparse.neg, "dim={dim}: neg mismatch");
    }
}

#[test]
fn test_packed_bitsliced_conversion_roundtrip() {
    for dim in [32, 64, 100, 1000] {
        let sparse = random_sparse(dim / 50 + 1, dim);
        let packed = PackedTritVec::from_sparsevec(&sparse, dim);

        // Packed → Bitsliced → Packed
        let bitsliced = BitslicedTritVec::from_packed(&packed);
        let back_packed = bitsliced.to_packed();

        // Verify element-by-element
        for i in 0..dim {
            assert_eq!(
                packed.get(i),
                back_packed.get(i),
                "dim={dim}, i={i}: trit mismatch"
            );
        }
    }
}

#[test]
fn test_bind_equivalence() {
    for dim in [64, 100, 1000] {
        let sparse_a = random_sparse(dim / 100 + 1, dim);
        let sparse_b = random_sparse(dim / 100 + 1, dim);

        let packed_a = PackedTritVec::from_sparsevec(&sparse_a, dim);
        let packed_b = PackedTritVec::from_sparsevec(&sparse_b, dim);

        let bitsliced_a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let bitsliced_b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        // Compute bind with both representations
        let packed_result = packed_a.bind(&packed_b);
        let bitsliced_result = bitsliced_a.bind(&bitsliced_b);

        // Compare results
        for i in 0..dim {
            assert_eq!(
                packed_result.get(i),
                bitsliced_result.get(i),
                "dim={dim}, i={i}: bind result mismatch"
            );
        }
    }
}

#[test]
fn test_bundle_equivalence() {
    for dim in [64, 100, 1000] {
        let sparse_a = random_sparse(dim / 100 + 1, dim);
        let sparse_b = random_sparse(dim / 100 + 1, dim);

        let packed_a = PackedTritVec::from_sparsevec(&sparse_a, dim);
        let packed_b = PackedTritVec::from_sparsevec(&sparse_b, dim);

        let bitsliced_a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let bitsliced_b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        // Compute bundle with both representations
        let packed_result = packed_a.bundle(&packed_b);
        let bitsliced_result = bitsliced_a.bundle(&bitsliced_b);

        // Compare results
        for i in 0..dim {
            assert_eq!(
                packed_result.get(i),
                bitsliced_result.get(i),
                "dim={dim}, i={i}: bundle result mismatch"
            );
        }
    }
}

#[test]
fn test_dot_equivalence() {
    for dim in [64, 100, 1000, 10000] {
        let sparse_a = random_sparse(dim / 100 + 1, dim);
        let sparse_b = random_sparse(dim / 100 + 1, dim);

        let packed_a = PackedTritVec::from_sparsevec(&sparse_a, dim);
        let packed_b = PackedTritVec::from_sparsevec(&sparse_b, dim);

        let bitsliced_a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let bitsliced_b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let packed_dot = packed_a.dot(&packed_b);
        let bitsliced_dot = bitsliced_a.dot(&bitsliced_b);

        assert_eq!(packed_dot, bitsliced_dot, "dim={dim}: dot mismatch");
    }
}

#[test]
fn test_word_boundary_correctness() {
    // Test dimensions that stress word boundaries
    for dim in [63, 64, 65, 127, 128, 129, 255, 256, 257] {
        let mut bitsliced = BitslicedTritVec::new_zero(dim);

        // Set trits at boundaries
        if dim > 0 {
            bitsliced.set(0, Trit::P);
        }
        if dim > 63 {
            bitsliced.set(63, Trit::N);
            bitsliced.set(64, Trit::P);
        }
        if dim > 127 {
            bitsliced.set(127, Trit::N);
            bitsliced.set(128, Trit::P);
        }
        bitsliced.set(dim - 1, Trit::N);

        // Roundtrip through sparse
        let sparse = bitsliced.to_sparse();
        let back = BitslicedTritVec::from_sparse(&sparse, dim);

        for i in 0..dim {
            assert_eq!(
                bitsliced.get(i),
                back.get(i),
                "dim={dim}, i={i}: boundary roundtrip mismatch"
            );
        }
    }
}

#[test]
fn test_carry_save_vs_sequential_bundle() {
    let dim = 1000;
    let n_vectors = 7;

    let vectors: Vec<_> = (0..n_vectors)
        .map(|_| {
            let sparse = random_sparse(dim / 100 + 1, dim);
            BitslicedTritVec::from_sparse(&sparse, dim)
        })
        .collect();

    // Sequential bundling
    let mut sequential = vectors[0].clone();
    for v in vectors.iter().skip(1) {
        sequential = sequential.bundle(v);
    }

    // Carry-save bundling
    let mut acc = CarrySaveBundle::new(dim);
    for v in &vectors {
        acc.accumulate(v);
    }
    let carry_save = acc.finalize();

    // Results may differ slightly due to different accumulation semantics
    // (carry-save preserves more info before majority vote)
    // But both should be valid ternary vectors
    assert_eq!(carry_save.len(), dim);

    // Count how many positions match
    let mut matches = 0;
    for i in 0..dim {
        if sequential.get(i) == carry_save.get(i) {
            matches += 1;
        }
    }

    // Most positions should match for sparse inputs
    let match_ratio = matches as f64 / dim as f64;
    assert!(
        match_ratio > 0.9,
        "Expected >90% match, got {:.1}%",
        match_ratio * 100.0
    );
}

#[test]
fn test_negate() {
    let dim = 100;
    let sparse = random_sparse(10, dim);
    let bitsliced = BitslicedTritVec::from_sparse(&sparse, dim);

    let negated = bitsliced.negate();

    for i in 0..dim {
        let original = bitsliced.get(i);
        let neg = negated.get(i);

        match original {
            Trit::P => assert_eq!(neg, Trit::N, "P should become N at {i}"),
            Trit::N => assert_eq!(neg, Trit::P, "N should become P at {i}"),
            Trit::Z => assert_eq!(neg, Trit::Z, "Z should stay Z at {i}"),
        }
    }

    // Double negation should be identity
    let double_neg = negated.negate();
    for i in 0..dim {
        assert_eq!(
            bitsliced.get(i),
            double_neg.get(i),
            "Double negation should be identity at {i}"
        );
    }
}

#[test]
fn test_bind_self_inverse() {
    // For non-zero trits: a ⊙ a = P (self-inverse property)
    let dim = 100;
    let sparse = random_sparse(20, dim);
    let bitsliced = BitslicedTritVec::from_sparse(&sparse, dim);

    let self_bind = bitsliced.bind(&bitsliced);

    for i in 0..dim {
        let original = bitsliced.get(i);
        let bound = self_bind.get(i);

        match original {
            Trit::P | Trit::N => assert_eq!(bound, Trit::P, "Non-zero ⊙ self should be P at {i}"),
            Trit::Z => assert_eq!(bound, Trit::Z, "Zero ⊙ self should be Z at {i}"),
        }
    }
}

#[test]
fn test_large_dimension() {
    // Test at D=100K (close to proposed D=10M but faster)
    let dim = 100_000;
    let sparse = random_sparse(1000, dim);

    let bitsliced = BitslicedTritVec::from_sparse(&sparse, dim);
    assert_eq!(bitsliced.len(), dim);
    assert_eq!(bitsliced.nnz(), 2000); // 1000 pos + 1000 neg

    // Verify roundtrip
    let back = bitsliced.to_sparse();
    assert_eq!(back.pos.len(), sparse.pos.len());
    assert_eq!(back.neg.len(), sparse.neg.len());
}
