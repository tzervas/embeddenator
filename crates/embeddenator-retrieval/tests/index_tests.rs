use embeddenator_retrieval::{
    index::{BruteForceIndex, HierarchicalIndex, IndexConfig, RetrievalIndex},
    similarity::SimilarityMetric,
};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::collections::HashMap;

#[test]
fn test_brute_force_index_basic() {
    let config = ReversibleVSAConfig::default();
    let mut index = BruteForceIndex::new(IndexConfig::default());

    let vec1 = SparseVec::encode_data(b"apple", &config, None);
    let vec2 = SparseVec::encode_data(b"banana", &config, None);
    let vec3 = SparseVec::encode_data(b"cherry", &config, None);

    index.add(1, &vec1);
    index.add(2, &vec2);
    index.add(3, &vec3);
    index.finalize();

    let query = SparseVec::encode_data(b"apple", &config, None);
    let results = index.query_top_k(&query, 2);

    assert!(!results.is_empty());
    assert_eq!(results[0].id, 1, "Should match apple best");
}

#[test]
fn test_brute_force_index_reranked() {
    let config = ReversibleVSAConfig::default();
    let mut index = BruteForceIndex::new(IndexConfig::default());
    let mut vectors = HashMap::new();

    let vec1 = SparseVec::encode_data(b"hello world", &config, None);
    let vec2 = SparseVec::encode_data(b"goodbye world", &config, None);

    index.add(1, &vec1);
    index.add(2, &vec2);
    index.finalize();

    vectors.insert(1, vec1);
    vectors.insert(2, vec2);

    let query = SparseVec::encode_data(b"hello", &config, None);
    let results = index.query_top_k_reranked(&query, &vectors, 10, 2);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 1, "Should match hello world best");

    // Check that cosine scores are present
    assert!(results[0].cosine > 0.0);
}

#[test]
fn test_brute_force_build_from_map() {
    let config = ReversibleVSAConfig::default();
    let mut vectors = HashMap::new();

    for i in 0..10 {
        let data = format!("doc-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        vectors.insert(i, vec);
    }

    let index = BruteForceIndex::build_from_map(vectors.clone(), IndexConfig::default());

    let query = SparseVec::encode_data(b"doc-5", &config, None);
    let results = index.query_top_k(&query, 5);

    assert!(!results.is_empty());
}

#[test]
fn test_hierarchical_index_basic() {
    let config = ReversibleVSAConfig::default();
    let index_config = IndexConfig {
        hierarchical: true,
        ..IndexConfig::default()
    };
    let mut index = HierarchicalIndex::new(index_config);

    // Add multiple vectors
    for i in 0..20 {
        let data = format!("item-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
    }
    index.finalize();

    let query = SparseVec::encode_data(b"item-10", &config, None);
    let results = index.query_top_k(&query, 5);

    assert!(!results.is_empty());
    assert!(results.len() <= 5);
}

#[test]
fn test_hierarchical_index_non_hierarchical_mode() {
    let config = ReversibleVSAConfig::default();
    let index_config = IndexConfig {
        hierarchical: false,
        ..IndexConfig::default()
    };
    let mut index = HierarchicalIndex::new(index_config);

    for i in 0..10 {
        let data = format!("test-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
    }
    index.finalize();

    let query = SparseVec::encode_data(b"test-3", &config, None);
    let results = index.query_top_k(&query, 3);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, 3, "Should match query item");
}

#[test]
fn test_hierarchical_index_reranked() {
    let config = ReversibleVSAConfig::default();
    let index_config = IndexConfig {
        hierarchical: true,
        ..IndexConfig::default()
    };
    let mut index = HierarchicalIndex::new(index_config);

    let mut vectors = HashMap::new();
    for i in 0..30 {
        let data = format!("doc-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
        vectors.insert(i, vec);
    }
    index.finalize();

    let query = SparseVec::encode_data(b"doc-15", &config, None);
    let results = index.query_top_k_reranked(&query, &vectors, 20, 5);

    assert_eq!(results.len(), 5);

    // Results should be sorted by cosine similarity
    for i in 1..results.len() {
        assert!(results[i - 1].cosine >= results[i].cosine);
    }
}

#[test]
fn test_index_config_different_metrics() {
    let config = ReversibleVSAConfig::default();

    // Test with Jaccard metric
    let jaccard_config = IndexConfig {
        metric: SimilarityMetric::Jaccard,
        ..IndexConfig::default()
    };
    let mut jaccard_index = BruteForceIndex::new(jaccard_config);

    let vec1 = SparseVec::encode_data(b"test", &config, None);
    let vec2 = SparseVec::encode_data(b"data", &config, None);

    jaccard_index.add(1, &vec1);
    jaccard_index.add(2, &vec2);
    jaccard_index.finalize();

    let query = SparseVec::encode_data(b"test", &config, None);
    let results = jaccard_index.query_top_k(&query, 2);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 1);
}

#[test]
fn test_index_with_empty_vectors() {
    let mut index = BruteForceIndex::new(IndexConfig::default());

    let empty1 = SparseVec {
        pos: vec![],
        neg: vec![],
    };
    let empty2 = SparseVec {
        pos: vec![],
        neg: vec![],
    };

    index.add(1, &empty1);
    index.add(2, &empty2);
    index.finalize();

    let query = SparseVec {
        pos: vec![],
        neg: vec![],
    };
    let results = index.query_top_k(&query, 2);

    assert_eq!(results.len(), 2);
}

#[test]
fn test_index_k_zero() {
    let config = ReversibleVSAConfig::default();
    let mut index = BruteForceIndex::new(IndexConfig::default());

    let vec = SparseVec::encode_data(b"test", &config, None);
    index.add(1, &vec);
    index.finalize();

    let query = SparseVec::encode_data(b"query", &config, None);
    let results = index.query_top_k(&query, 0);

    assert!(results.is_empty());
}

#[test]
fn test_index_large_corpus() {
    let config = ReversibleVSAConfig::default();
    let index_config = IndexConfig {
        hierarchical: true,
        ..IndexConfig::default()
    };
    let mut index = HierarchicalIndex::new(index_config);

    // Add 100 vectors
    for i in 0..100 {
        let data = format!("document-{:04}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
    }
    index.finalize();

    let query = SparseVec::encode_data(b"document-0050", &config, None);
    let results = index.query_top_k(&query, 10);

    assert_eq!(results.len(), 10);

    // Should find the exact match in top results
    let top_ids: Vec<usize> = results.iter().map(|r| r.id).collect();
    assert!(top_ids.contains(&50), "Should find exact match in top 10");
}

#[test]
fn test_index_consistency_across_implementations() {
    // Both BruteForce and Hierarchical should find the same top result
    let config = ReversibleVSAConfig::default();

    let mut bf_index = BruteForceIndex::new(IndexConfig::default());
    let h_config = IndexConfig {
        hierarchical: true,
        ..IndexConfig::default()
    };
    let mut h_index = HierarchicalIndex::new(h_config);

    // Add same data to both
    for i in 0..20 {
        let data = format!("entry-{}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        bf_index.add(i, &vec);
        h_index.add(i, &vec.clone());
    }

    bf_index.finalize();
    h_index.finalize();

    let query = SparseVec::encode_data(b"entry-7", &config, None);

    let bf_results = bf_index.query_top_k(&query, 5);
    let h_results = h_index.query_top_k(&query, 5);

    // Both should identify the same top match
    assert_eq!(bf_results[0].id, 7);
    assert_eq!(h_results[0].id, 7);
}

#[test]
fn test_index_add_duplicate_ids() {
    let config = ReversibleVSAConfig::default();
    let mut index = BruteForceIndex::new(IndexConfig::default());

    let vec1 = SparseVec::encode_data(b"first", &config, None);
    let vec2 = SparseVec::encode_data(b"second", &config, None);

    index.add(1, &vec1);
    index.add(1, &vec2); // Duplicate ID - should overwrite
    index.finalize();

    let query = SparseVec::encode_data(b"second", &config, None);
    let results = index.query_top_k(&query, 1);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
}

#[test]
fn test_hierarchical_clustering_quality() {
    let config = ReversibleVSAConfig::default();
    let index_config = IndexConfig {
        hierarchical: true,
        leaf_size: 10,
        ..IndexConfig::default()
    };
    let mut index = HierarchicalIndex::new(index_config);

    // Add vectors with some structure
    for i in 0..50 {
        let prefix = if i < 25 { "group-a" } else { "group-b" };
        let data = format!("{}-{}", prefix, i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
    }
    index.finalize();

    // Query for group-a items
    let query = SparseVec::encode_data(b"group-a-10", &config, None);
    let results = index.query_top_k(&query, 10);

    assert!(!results.is_empty());

    // Top result should be from the same group
    assert!(results[0].id < 25, "Should prefer items from same group");
}
