use embeddenator_retrieval::{
    search::{
        approximate_search, batch_search, compute_recall_at_k, exact_search, two_stage_search,
        RankedResult, SearchConfig,
    },
    similarity::SimilarityMetric,
    SearchResult, TernaryInvertedIndex,
};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::collections::HashMap;

fn build_test_corpus(size: usize) -> (TernaryInvertedIndex, HashMap<usize, SparseVec>) {
    let config = ReversibleVSAConfig::default();
    let mut index = TernaryInvertedIndex::new();
    let mut vectors = HashMap::new();

    for i in 0..size {
        let data = format!("document-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
        vectors.insert(i, vec);
    }

    index.finalize();
    (index, vectors)
}

#[test]
fn test_two_stage_search_basic() {
    let (index, vectors) = build_test_corpus(10);
    let config = ReversibleVSAConfig::default();

    let query = SparseVec::encode_data(b"document-5", &config, None);
    let search_config = SearchConfig::default();
    let results = two_stage_search(&query, &index, &vectors, &search_config, 5);

    assert!(!results.is_empty(), "Should return results");
    assert!(results.len() <= 5, "Should return at most k results");

    // Results should be ranked
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.rank, i + 1, "Rank should be sequential");
    }

    // Should match itself best (or very close)
    assert_eq!(results[0].id, 5, "Should match query document best");
}

#[test]
fn test_exact_search() {
    let config = ReversibleVSAConfig::default();
    let mut vectors = HashMap::new();

    for i in 0..20 {
        let data = format!("item-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        vectors.insert(i, vec);
    }

    let query = SparseVec::encode_data(b"item-10", &config, None);
    let results = exact_search(&query, &vectors, SimilarityMetric::Cosine, 5);

    assert_eq!(results.len(), 5);
    // Due to VSA encoding, the exact item should be in top results
    let top_ids: Vec<usize> = results.iter().map(|r| r.id).collect();
    assert!(
        top_ids.contains(&10),
        "Query item should be in top 5 results"
    );

    // Scores should be descending
    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "Results should be sorted by score"
        );
    }
}

#[test]
fn test_approximate_search() {
    let (index, _vectors) = build_test_corpus(50);
    let config = ReversibleVSAConfig::default();

    let query = SparseVec::encode_data(b"document-25", &config, None);
    let results = approximate_search(&query, &index, 10);

    assert!(!results.is_empty());
    assert!(results.len() <= 10);

    // Approximate scores should be descending
    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "Results should be sorted by approximate score"
        );
    }
}

#[test]
fn test_batch_search() {
    let (index, vectors) = build_test_corpus(30);
    let config = ReversibleVSAConfig::default();

    let queries = vec![
        SparseVec::encode_data(b"document-5", &config, None),
        SparseVec::encode_data(b"document-15", &config, None),
        SparseVec::encode_data(b"document-25", &config, None),
    ];

    let search_config = SearchConfig::default();
    let results = batch_search(&queries, &index, &vectors, &search_config, 5);

    assert_eq!(results.len(), 3, "Should return results for each query");

    for query_results in &results {
        assert!(!query_results.is_empty(), "Each query should have results");
        assert!(
            query_results.len() <= 5,
            "Each query should have at most k results"
        );
    }
}

#[test]
fn test_search_with_empty_query() {
    let (index, vectors) = build_test_corpus(10);

    let empty_query = SparseVec {
        pos: vec![],
        neg: vec![],
    };
    let search_config = SearchConfig::default();
    let results = two_stage_search(&empty_query, &index, &vectors, &search_config, 5);

    // Should still return results (may not be meaningful, but shouldn't crash)
    assert!(results.len() <= 5);
}

#[test]
fn test_search_k_zero() {
    let (index, vectors) = build_test_corpus(10);
    let config = ReversibleVSAConfig::default();

    let query = SparseVec::encode_data(b"test", &config, None);
    let search_config = SearchConfig::default();
    let results = two_stage_search(&query, &index, &vectors, &search_config, 0);

    assert!(results.is_empty(), "k=0 should return no results");
}

#[test]
fn test_search_k_larger_than_corpus() {
    let (index, vectors) = build_test_corpus(5);
    let config = ReversibleVSAConfig::default();

    let query = SparseVec::encode_data(b"document-2", &config, None);
    let search_config = SearchConfig::default();
    let results = two_stage_search(&query, &index, &vectors, &search_config, 100);

    assert!(results.len() <= 5, "Should return at most corpus size");
}

#[test]
fn test_recall_at_k_perfect() {
    // Perfect recall: all approximate results match exact results
    let approx = vec![
        SearchResult { id: 1, score: 100 },
        SearchResult { id: 2, score: 90 },
        SearchResult { id: 3, score: 80 },
    ];

    let exact = vec![
        RankedResult {
            id: 1,
            score: 0.95,
            approx_score: 100,
            rank: 1,
        },
        RankedResult {
            id: 2,
            score: 0.90,
            approx_score: 90,
            rank: 2,
        },
        RankedResult {
            id: 3,
            score: 0.85,
            approx_score: 80,
            rank: 3,
        },
    ];

    let recall = compute_recall_at_k(&approx, &exact, 3);
    assert_eq!(recall, 1.0, "Perfect match should have recall of 1.0");
}

#[test]
fn test_recall_at_k_partial() {
    let approx = vec![
        SearchResult { id: 1, score: 100 },
        SearchResult { id: 2, score: 90 },
        SearchResult { id: 5, score: 80 },
    ];

    let exact = vec![
        RankedResult {
            id: 1,
            score: 0.95,
            approx_score: 100,
            rank: 1,
        },
        RankedResult {
            id: 3,
            score: 0.90,
            approx_score: 95,
            rank: 2,
        },
        RankedResult {
            id: 2,
            score: 0.85,
            approx_score: 90,
            rank: 3,
        },
    ];

    let recall = compute_recall_at_k(&approx, &exact, 3);
    assert!((recall - 0.666).abs() < 0.01, "Should have 2/3 recall");
}

#[test]
fn test_recall_at_k_zero() {
    let approx = vec![
        SearchResult { id: 10, score: 100 },
        SearchResult { id: 20, score: 90 },
    ];

    let exact = vec![
        RankedResult {
            id: 1,
            score: 0.95,
            approx_score: 100,
            rank: 1,
        },
        RankedResult {
            id: 2,
            score: 0.90,
            approx_score: 90,
            rank: 2,
        },
    ];

    let recall = compute_recall_at_k(&approx, &exact, 2);
    assert_eq!(recall, 0.0, "No overlap should have recall of 0.0");
}

#[test]
fn test_search_consistency() {
    // Two-stage should be more accurate than approximate
    let (index, vectors) = build_test_corpus(100);
    let config = ReversibleVSAConfig::default();

    let query = SparseVec::encode_data(b"document-50", &config, None);

    // Get ground truth from exact search
    let exact_results = exact_search(&query, &vectors, SimilarityMetric::Cosine, 10);

    // Get approximate results
    let approx_results = approximate_search(&query, &index, 10);

    // Get two-stage results
    let search_config = SearchConfig::default();
    let two_stage_results = two_stage_search(&query, &index, &vectors, &search_config, 10);

    // Convert for recall computation
    let exact_ranked: Vec<RankedResult> = exact_results;

    let approx_recall = compute_recall_at_k(&approx_results, &exact_ranked, 10);

    // Two-stage uses exact scoring in stage 2, so should match exact search
    assert_eq!(
        two_stage_results[0].id, exact_ranked[0].id,
        "Two-stage should match exact search for top result"
    );

    // Approximate recall should be reasonably high but may not be perfect
    assert!(
        approx_recall > 0.5,
        "Approximate search should have decent recall"
    );
}

#[test]
fn test_different_similarity_metrics() {
    let config = ReversibleVSAConfig::default();
    let mut vectors = HashMap::new();

    // Create more distinct documents
    for i in 0..10 {
        let data = format!("document_number_{:03}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        vectors.insert(i, vec);
    }

    let query = SparseVec::encode_data(b"document_number_005", &config, None);

    // Test different metrics - all should return results
    let cosine_results = exact_search(&query, &vectors, SimilarityMetric::Cosine, 5);
    let jaccard_results = exact_search(&query, &vectors, SimilarityMetric::Jaccard, 5);

    assert_eq!(cosine_results.len(), 5);
    assert_eq!(jaccard_results.len(), 5);

    // Cosine is most reliable for VSA - should find target
    let cosine_ids: Vec<usize> = cosine_results.iter().map(|r| r.id).collect();
    assert!(
        cosine_ids.contains(&5),
        "Cosine should find doc5 in top 5, got {:?}",
        cosine_ids
    );

    // Jaccard should also work reasonably well
    let jaccard_ids: Vec<usize> = jaccard_results.iter().map(|r| r.id).collect();
    assert!(
        jaccard_ids.contains(&5),
        "Jaccard should find doc5 in top 5, got {:?}",
        jaccard_ids
    );
}

#[test]
fn test_search_config_customization() {
    let (index, vectors) = build_test_corpus(50);
    let config = ReversibleVSAConfig::default();

    let query = SparseVec::encode_data(b"document-10", &config, None);

    // Test with different candidate_k values
    let config1 = SearchConfig {
        candidate_k: 5,
        ..SearchConfig::default()
    };
    let results1 = two_stage_search(&query, &index, &vectors, &config1, 3);

    let config2 = SearchConfig {
        candidate_k: 50,
        ..SearchConfig::default()
    };
    let results2 = two_stage_search(&query, &index, &vectors, &config2, 3);

    assert_eq!(results1.len(), 3);
    assert_eq!(results2.len(), 3);

    // Both should return valid results
    assert!(
        results1[0].score > 0.0,
        "Should have positive similarity scores"
    );
    assert!(
        results2[0].score > 0.0,
        "Should have positive similarity scores"
    );

    // Larger candidate_k should generally give better or equal results
    // (but we don't enforce exact match due to VSA randomness)
}
