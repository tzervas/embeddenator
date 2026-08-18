//! Search strategies for semantic retrieval
//!
//! This module implements various search algorithms:
//! - Exact search (brute force)
//! - Approximate search (inverted index)
//! - Beam search (hierarchical)
//! - Two-stage search (candidate generation + reranking)
//!
//! All search functions support parallel execution via `SearchConfig::parallel`.

use std::collections::HashMap;

use rayon::prelude::*;

use crate::retrieval::{SearchResult, TernaryInvertedIndex};
use crate::similarity::{compute_similarity, SimilarityMetric};
use embeddenator_vsa::SparseVec;

/// Search strategy configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Similarity metric for final ranking
    pub metric: SimilarityMetric,
    /// Number of candidates to generate before reranking
    pub candidate_k: usize,
    /// Beam width for hierarchical search.
    ///
    /// **Note:** This field is reserved for future hierarchical/beam search
    /// implementations. It is not currently used by the search functions in
    /// this module. Setting this value has no effect on search behavior.
    pub beam_width: usize,
    /// Enable parallel search
    pub parallel: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            metric: SimilarityMetric::Cosine,
            candidate_k: 200,
            beam_width: 10,
            parallel: false,
        }
    }
}

/// Search result with additional metadata
#[derive(Debug, Clone, PartialEq)]
pub struct RankedResult {
    /// Document ID
    pub id: usize,
    /// Final similarity score
    pub score: f64,
    /// Approximate score from first stage
    pub approx_score: i32,
    /// Rank in results (1-indexed)
    pub rank: usize,
}

/// Two-stage search: fast candidate generation + accurate reranking
///
/// This is the recommended strategy for most use cases. It combines the
/// speed of inverted index search with the accuracy of exact similarity.
///
/// # Arguments
/// * `query` - Query vector
/// * `index` - Inverted index for candidate generation
/// * `vectors` - Full vector collection for reranking
/// * `config` - Search configuration
/// * `k` - Number of final results to return
///
/// # Returns
/// Top-k results ranked by exact similarity
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::search::{two_stage_search, SearchConfig};
/// use embeddenator_retrieval::TernaryInvertedIndex;
/// use embeddenator_vsa::SparseVec;
/// use std::collections::HashMap;
///
/// let mut index = TernaryInvertedIndex::new();
/// let mut vectors = HashMap::new();
///
/// let vec1 = SparseVec::from_data(b"document one");
/// let vec2 = SparseVec::from_data(b"document two");
///
/// index.add(1, &vec1);
/// index.add(2, &vec2);
/// index.finalize();
///
/// vectors.insert(1, vec1);
/// vectors.insert(2, vec2);
///
/// let query = SparseVec::from_data(b"document");
/// let config = SearchConfig::default();
/// let results = two_stage_search(&query, &index, &vectors, &config, 5);
///
/// assert!(!results.is_empty());
/// ```
pub fn two_stage_search(
    query: &SparseVec,
    index: &TernaryInvertedIndex,
    vectors: &HashMap<usize, SparseVec>,
    config: &SearchConfig,
    k: usize,
) -> Vec<RankedResult> {
    if k == 0 {
        return Vec::new();
    }

    // Stage 1: Generate candidates using inverted index
    let candidate_k = config.candidate_k.max(k);
    let candidates = index.query_top_k(query, candidate_k);

    // Stage 2: Rerank candidates with exact similarity
    // Use parallel iteration when enabled for compute-intensive similarity calculations
    let mut reranked: Vec<RankedResult> = if config.parallel {
        // Collect candidates with their vectors first to enable parallel processing
        let candidates_with_vecs: Vec<_> = candidates
            .iter()
            .filter_map(|cand| vectors.get(&cand.id).map(|vec| (cand, vec)))
            .collect();

        candidates_with_vecs
            .par_iter()
            .map(|(cand, vec)| {
                let score = compute_similarity(query, vec, config.metric);
                RankedResult {
                    id: cand.id,
                    score,
                    approx_score: cand.score,
                    rank: 0, // Will be set after sorting
                }
            })
            .collect()
    } else {
        candidates
            .iter()
            .filter_map(|cand| {
                vectors.get(&cand.id).map(|vec| {
                    let score = compute_similarity(query, vec, config.metric);
                    RankedResult {
                        id: cand.id,
                        score,
                        approx_score: cand.score,
                        rank: 0, // Will be set after sorting
                    }
                })
            })
            .collect()
    };

    // Sort by similarity score
    reranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    // Assign ranks and truncate
    reranked.truncate(k);
    for (idx, result) in reranked.iter_mut().enumerate() {
        result.rank = idx + 1;
    }

    reranked
}

/// Exact search using brute force comparison
///
/// Computes similarity against all vectors in the collection.
/// Use for small collections or ground truth evaluation.
///
/// # Arguments
/// * `query` - Query vector
/// * `vectors` - Vector collection
/// * `metric` - Similarity metric to use
/// * `k` - Number of results to return
///
/// # Returns
/// Top-k results ranked by similarity
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::search::{exact_search};
/// use embeddenator_retrieval::similarity::SimilarityMetric;
/// use embeddenator_vsa::SparseVec;
/// use std::collections::HashMap;
///
/// let mut vectors = HashMap::new();
/// vectors.insert(1, SparseVec::from_data(b"document one"));
/// vectors.insert(2, SparseVec::from_data(b"document two"));
///
/// let query = SparseVec::from_data(b"document");
/// let results = exact_search(&query, &vectors, SimilarityMetric::Cosine, 5);
///
/// assert!(!results.is_empty());
/// ```
pub fn exact_search(
    query: &SparseVec,
    vectors: &HashMap<usize, SparseVec>,
    metric: SimilarityMetric,
    k: usize,
) -> Vec<RankedResult> {
    exact_search_impl(query, vectors, metric, k, false)
}

/// Exact search with parallel option
///
/// Same as `exact_search` but allows enabling parallel processing
/// for large vector collections.
pub fn exact_search_parallel(
    query: &SparseVec,
    vectors: &HashMap<usize, SparseVec>,
    metric: SimilarityMetric,
    k: usize,
    parallel: bool,
) -> Vec<RankedResult> {
    exact_search_impl(query, vectors, metric, k, parallel)
}

fn exact_search_impl(
    query: &SparseVec,
    vectors: &HashMap<usize, SparseVec>,
    metric: SimilarityMetric,
    k: usize,
    parallel: bool,
) -> Vec<RankedResult> {
    if k == 0 || vectors.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<RankedResult> = if parallel {
        // Collect to vec first for parallel iteration
        let vec_entries: Vec<_> = vectors.iter().collect();
        vec_entries
            .par_iter()
            .map(|(id, vec)| {
                let score = compute_similarity(query, vec, metric);
                RankedResult {
                    id: **id,
                    score,
                    approx_score: (score * 1000.0) as i32,
                    rank: 0,
                }
            })
            .collect()
    } else {
        vectors
            .iter()
            .map(|(id, vec)| {
                let score = compute_similarity(query, vec, metric);
                RankedResult {
                    id: *id,
                    score,
                    approx_score: (score * 1000.0) as i32,
                    rank: 0,
                }
            })
            .collect()
    };

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    results.truncate(k);
    for (idx, result) in results.iter_mut().enumerate() {
        result.rank = idx + 1;
    }

    results
}

/// Approximate search using only the inverted index
///
/// Fast but less accurate. Good for initial filtering or when
/// speed is more important than perfect ranking.
///
/// # Arguments
/// * `query` - Query vector
/// * `index` - Inverted index
/// * `k` - Number of results to return
///
/// # Returns
/// Top-k results ranked by approximate score
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::search::approximate_search;
/// use embeddenator_retrieval::TernaryInvertedIndex;
/// use embeddenator_vsa::SparseVec;
///
/// let mut index = TernaryInvertedIndex::new();
/// let vec1 = SparseVec::from_data(b"document one");
/// index.add(1, &vec1);
/// index.finalize();
///
/// let query = SparseVec::from_data(b"document");
/// let results = approximate_search(&query, &index, 5);
///
/// assert!(!results.is_empty());
/// ```
pub fn approximate_search(
    query: &SparseVec,
    index: &TernaryInvertedIndex,
    k: usize,
) -> Vec<SearchResult> {
    index.query_top_k(query, k)
}

/// Batch search - process multiple queries efficiently
///
/// # Arguments
/// * `queries` - Multiple query vectors
/// * `index` - Inverted index
/// * `vectors` - Vector collection
/// * `config` - Search configuration
/// * `k` - Number of results per query
///
/// # Returns
/// Results for each query
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::search::{batch_search, SearchConfig};
/// use embeddenator_retrieval::TernaryInvertedIndex;
/// use embeddenator_vsa::SparseVec;
/// use std::collections::HashMap;
///
/// let mut index = TernaryInvertedIndex::new();
/// let mut vectors = HashMap::new();
/// let vec1 = SparseVec::from_data(b"doc one");
/// index.add(1, &vec1);
/// index.finalize();
/// vectors.insert(1, vec1);
///
/// let queries = vec![
///     SparseVec::from_data(b"query1"),
///     SparseVec::from_data(b"query2"),
/// ];
/// let config = SearchConfig::default();
/// let results = batch_search(&queries, &index, &vectors, &config, 5);
///
/// assert_eq!(results.len(), 2);
/// ```
pub fn batch_search(
    queries: &[SparseVec],
    index: &TernaryInvertedIndex,
    vectors: &HashMap<usize, SparseVec>,
    config: &SearchConfig,
    k: usize,
) -> Vec<Vec<RankedResult>> {
    if config.parallel {
        // Process multiple queries concurrently
        queries
            .par_iter()
            .map(|query| two_stage_search(query, index, vectors, config, k))
            .collect()
    } else {
        queries
            .iter()
            .map(|query| two_stage_search(query, index, vectors, config, k))
            .collect()
    }
}

/// Compute recall@k metric for search quality evaluation
///
/// Compares approximate search results against ground truth.
///
/// # Arguments
/// * `approx_results` - Results from approximate search
/// * `exact_results` - Ground truth from exact search
/// * `k` - Number of top results to consider
///
/// # Returns
/// Recall score in [0, 1]
pub fn compute_recall_at_k(
    approx_results: &[SearchResult],
    exact_results: &[RankedResult],
    k: usize,
) -> f64 {
    if k == 0 || exact_results.is_empty() {
        return 0.0;
    }

    let exact_ids: std::collections::HashSet<usize> =
        exact_results.iter().take(k).map(|r| r.id).collect();

    let matches = approx_results
        .iter()
        .take(k)
        .filter(|r| exact_ids.contains(&r.id))
        .count();

    matches as f64 / k.min(exact_results.len()) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddenator_vsa::ReversibleVSAConfig;

    #[test]
    fn test_two_stage_search() {
        let config = ReversibleVSAConfig::default();
        let mut index = TernaryInvertedIndex::new();
        let mut vectors = HashMap::new();

        let vec1 = SparseVec::encode_data(b"hello world", &config, None);
        let vec2 = SparseVec::encode_data(b"goodbye world", &config, None);

        index.add(1, &vec1);
        index.add(2, &vec2);
        index.finalize();

        vectors.insert(1, vec1);
        vectors.insert(2, vec2);

        let query = SparseVec::encode_data(b"hello", &config, None);
        let search_config = SearchConfig::default();
        let results = two_stage_search(&query, &index, &vectors, &search_config, 2);

        assert!(!results.is_empty());
        assert_eq!(results[0].rank, 1);
    }

    #[test]
    fn test_exact_search() {
        let config = ReversibleVSAConfig::default();
        let mut vectors = HashMap::new();

        vectors.insert(1, SparseVec::encode_data(b"apple", &config, None));
        vectors.insert(2, SparseVec::encode_data(b"banana", &config, None));
        vectors.insert(3, SparseVec::encode_data(b"cherry", &config, None));

        let query = SparseVec::encode_data(b"apple", &config, None);
        let results = exact_search(&query, &vectors, SimilarityMetric::Cosine, 3);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, 1); // Should match apple best
    }

    #[test]
    fn test_batch_search() {
        let config = ReversibleVSAConfig::default();
        let mut index = TernaryInvertedIndex::new();
        let mut vectors = HashMap::new();

        let vec1 = SparseVec::encode_data(b"doc1", &config, None);
        let vec2 = SparseVec::encode_data(b"doc2", &config, None);

        index.add(1, &vec1);
        index.add(2, &vec2);
        index.finalize();

        vectors.insert(1, vec1);
        vectors.insert(2, vec2);

        let queries = vec![
            SparseVec::encode_data(b"query1", &config, None),
            SparseVec::encode_data(b"query2", &config, None),
        ];

        let search_config = SearchConfig::default();
        let results = batch_search(&queries, &index, &vectors, &search_config, 2);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_recall_computation() {
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
        assert!((recall - 0.666).abs() < 0.01); // 2/3 match
    }

    #[test]
    fn test_parallel_two_stage_search_matches_sequential() {
        let config = ReversibleVSAConfig::default();
        let mut index = TernaryInvertedIndex::new();
        let mut vectors = HashMap::new();

        // Build a corpus of 50 vectors for meaningful parallel work
        for i in 0..50 {
            let data = format!("document number {} with some content", i);
            let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
            index.add(i, &vec);
            vectors.insert(i, vec);
        }
        index.finalize();

        let query = SparseVec::encode_data(b"document number 25", &config, None);

        let seq_config = SearchConfig {
            parallel: false,
            ..SearchConfig::default()
        };
        let par_config = SearchConfig {
            parallel: true,
            ..SearchConfig::default()
        };

        let seq_results = two_stage_search(&query, &index, &vectors, &seq_config, 10);
        let par_results = two_stage_search(&query, &index, &vectors, &par_config, 10);

        assert_eq!(seq_results.len(), par_results.len());
        for (seq, par) in seq_results.iter().zip(par_results.iter()) {
            assert_eq!(seq.id, par.id);
            assert!((seq.score - par.score).abs() < 1e-10);
            assert_eq!(seq.rank, par.rank);
        }
    }

    #[test]
    fn test_parallel_exact_search_matches_sequential() {
        let config = ReversibleVSAConfig::default();
        let mut vectors = HashMap::new();

        for i in 0..100 {
            let data = format!("item {} for testing parallel exact search", i);
            vectors.insert(i, SparseVec::encode_data(data.as_bytes(), &config, None));
        }

        let query = SparseVec::encode_data(b"item 50 for testing", &config, None);

        let seq_results =
            exact_search_parallel(&query, &vectors, SimilarityMetric::Cosine, 20, false);
        let par_results =
            exact_search_parallel(&query, &vectors, SimilarityMetric::Cosine, 20, true);

        assert_eq!(seq_results.len(), par_results.len());
        for (seq, par) in seq_results.iter().zip(par_results.iter()) {
            assert_eq!(seq.id, par.id);
            assert!((seq.score - par.score).abs() < 1e-10);
        }
    }

    #[test]
    fn test_parallel_batch_search_matches_sequential() {
        let config = ReversibleVSAConfig::default();
        let mut index = TernaryInvertedIndex::new();
        let mut vectors = HashMap::new();

        for i in 0..30 {
            let data = format!("batch doc {}", i);
            let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
            index.add(i, &vec);
            vectors.insert(i, vec);
        }
        index.finalize();

        let queries: Vec<SparseVec> = (0..10)
            .map(|i| {
                let data = format!("query {}", i);
                SparseVec::encode_data(data.as_bytes(), &config, None)
            })
            .collect();

        let seq_config = SearchConfig {
            parallel: false,
            ..SearchConfig::default()
        };
        let par_config = SearchConfig {
            parallel: true,
            ..SearchConfig::default()
        };

        let seq_results = batch_search(&queries, &index, &vectors, &seq_config, 5);
        let par_results = batch_search(&queries, &index, &vectors, &par_config, 5);

        assert_eq!(seq_results.len(), par_results.len());
        for (seq_batch, par_batch) in seq_results.iter().zip(par_results.iter()) {
            assert_eq!(seq_batch.len(), par_batch.len());
            for (seq, par) in seq_batch.iter().zip(par_batch.iter()) {
                assert_eq!(seq.id, par.id);
                assert!((seq.score - par.score).abs() < 1e-10);
            }
        }
    }
}
