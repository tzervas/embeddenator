use embeddenator::{ReversibleVSAConfig, SparseVec, TernaryInvertedIndex};

#[test]
fn test_inverted_index_returns_self_top_hit() {
    // Use deterministic vectors; query should match itself best.
    let config = ReversibleVSAConfig::default();
    let a = SparseVec::encode_data(b"alpha", &config, None);
    let b = SparseVec::encode_data(b"beta", &config, None);

    let mut index = TernaryInvertedIndex::new();
    index.add(0, &a);
    index.add(1, &b);
    index.finalize();

    let hits = index.query_top_k(&a, 2);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].id, 0);
    if hits.len() > 1 {
        assert!(hits[0].score >= hits[1].score);
    }
}
