use std::collections::HashMap;

use embeddenator::{ReversibleVSAConfig, SparseVec, TernaryInvertedIndex};

fn dv(label: &str) -> SparseVec {
    // Deterministic pseudo-random sparse vector.
    #[allow(deprecated)]
    {
        SparseVec::from_data(label.as_bytes())
    }
}

#[test]
fn lens_contract_cosine_self_is_one() {
    let cfg = ReversibleVSAConfig::default();
    let v = SparseVec::encode_data(b"lens-self", &cfg, None);
    let sim = v.cosine(&v);
    assert!((sim - 1.0).abs() < 1e-12, "cos(v,v) should be 1, got {sim}");
}

#[test]
fn lens_contract_unrelated_vectors_are_near_orthogonal_statistically() {
    // This is a *probabilistic* check but uses deterministic vectors to avoid flaky RNG.
    // With D=10k sparse ternary vectors, unrelated cosine should be tightly clustered near 0.
    let pairs = 2_000usize;

    let mut max_abs = 0.0f64;
    let mut count_over_0_10 = 0usize;
    for i in 0..pairs {
        let a = dv(&format!("a/{i}"));
        let b = dv(&format!("b/{i}"));
        let sim = a.cosine(&b).abs();
        if sim > max_abs {
            max_abs = sim;
        }
        if sim > 0.10 {
            count_over_0_10 += 1;
        }
    }

    // Conservative thresholds: allow a few tail events but not many.
    assert!(
        max_abs < 0.20,
        "max |cos| for unrelated vectors too high: {max_abs}"
    );
    assert!(
        count_over_0_10 <= 2,
        "too many unrelated pairs had |cos| > 0.10: {count_over_0_10}/{pairs}"
    );
}

#[test]
fn lens_contract_permutation_preserves_cosine() {
    let a = dv("perm/a");
    let b = dv("perm/b");
    let base = a.cosine(&b);

    for shift in [0usize, 1, 7, 101, 999, 10_000 + 17] {
        let pa = a.permute(shift);
        let pb = b.permute(shift);
        let got = pa.cosine(&pb);
        let diff = (got - base).abs();
        assert!(diff < 1e-12, "permute should preserve cosine, diff={diff}");
    }
}

#[test]
fn lens_contract_retrieval_recall_at_10_on_noisy_self_queries() {
    // Synthetic retrieval benchmark:
    // - Build a set of deterministic vectors
    // - Query with "self bundled with small deterministic noise"
    // - Expect the original ID to appear in the top-10 after reranking

    let n = 2_000usize;
    let queries = 200usize;

    let mut vectors: HashMap<usize, SparseVec> = HashMap::with_capacity(n);
    for id in 0..n {
        vectors.insert(id, dv(&format!("item/{id}")));
    }

    let index = TernaryInvertedIndex::build_from_map(&vectors);

    let mut hits = 0usize;
    for q in 0..queries {
        let id = (q * 7) % n; // deterministic sampling without RNG
        let v = vectors.get(&id).unwrap();
        let noise = dv(&format!("noise/{q}"));
        let query = v.bundle(&noise);

        let reranked = index.query_top_k_reranked(&query, &vectors, 1_000, 10);
        let found = reranked.iter().any(|r| r.id == id);
        if found {
            hits += 1;
        }
    }

    let recall_at_10 = hits as f64 / queries as f64;
    assert!(
        recall_at_10 >= 0.95,
        "recall@10 too low on synthetic noisy-self queries: {recall_at_10}"
    );
}
