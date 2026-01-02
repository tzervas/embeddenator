#![cfg(feature = "proptest")]

use embeddenator::SparseVec;
use proptest::prelude::*;
use std::collections::BTreeMap;

fn sparse_vec_strategy(max_nonzeros: usize) -> impl Strategy<Value = SparseVec> {
    // Generate (idx, sign) pairs and canonicalize to unique indices.
    // Keeping this small ensures the property suite stays fast.
    prop::collection::vec(
        (0usize..embeddenator::DIM, prop_oneof![Just(1i8), Just(-1i8)]),
        0..max_nonzeros,
    )
    .prop_map(|pairs| {
        let mut by_idx: BTreeMap<usize, i8> = BTreeMap::new();
        for (idx, sign) in pairs {
            by_idx.insert(idx, sign);
        }

        let mut v = SparseVec::new();
        v.pos = Vec::new();
        v.neg = Vec::new();

        for (idx, sign) in by_idx {
            match sign {
                1 => v.pos.push(idx),
                -1 => v.neg.push(idx),
                _ => {}
            }
        }

        v
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        .. ProptestConfig::default()
    })]

    #[test]
    fn bundle_is_commutative(a in sparse_vec_strategy(256), b in sparse_vec_strategy(256)) {
        let ab = a.bundle(&b);
        let ba = b.bundle(&a);
        prop_assert_eq!(ab.pos, ba.pos);
        prop_assert_eq!(ab.neg, ba.neg);
    }

    #[test]
    fn bundle_is_idempotent(a in sparse_vec_strategy(256)) {
        let aa = a.bundle(&a);
        prop_assert_eq!(aa.pos, a.pos);
        prop_assert_eq!(aa.neg, a.neg);
    }

    #[test]
    fn bundle_similarity_with_left_is_nonnegative(a in sparse_vec_strategy(256), b in sparse_vec_strategy(256)) {
        let ab = a.bundle(&b);
        let sim = a.cosine(&ab);
        prop_assert!(sim >= -1e-12 && sim <= 1.0 + 1e-12);
    }

    #[test]
    fn bundle_nnz_is_bounded(a in sparse_vec_strategy(256), b in sparse_vec_strategy(256)) {
        let ab = a.bundle(&b);
        let nnz_a = a.pos.len() + a.neg.len();
        let nnz_b = b.pos.len() + b.neg.len();
        let nnz_ab = ab.pos.len() + ab.neg.len();
        prop_assert!(nnz_ab <= nnz_a + nnz_b);
    }

    #[test]
    fn bind_support_is_subset_of_key(a in sparse_vec_strategy(256), b in sparse_vec_strategy(256)) {
        let r = a.bind(&b);

        let mut b_support = b.pos.clone();
        b_support.extend_from_slice(&b.neg);
        b_support.sort_unstable();
        b_support.dedup();

        for idx in r.pos.iter().chain(r.neg.iter()) {
            prop_assert!(b_support.binary_search(idx).is_ok());
        }
    }

    #[test]
    fn bind_double_application_matches_abs_key(a in sparse_vec_strategy(256), b in sparse_vec_strategy(256)) {
        // For the current SparseVec::bind semantics (support-restricted elementwise multiply),
        // binding twice by the same key is equivalent to binding once by key⊙key
        // (which effectively removes sign flips in the key).
        let bb = b.bind(&b);
        let left = a.bind(&b).bind(&b);
        let right = a.bind(&bb);
        prop_assert_eq!(left.pos, right.pos);
        prop_assert_eq!(left.neg, right.neg);
    }
}
