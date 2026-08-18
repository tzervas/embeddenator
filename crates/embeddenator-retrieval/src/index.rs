//! Index structures for efficient retrieval
//!
//! This module provides various index structures optimized for different
//! retrieval scenarios:
//! - In-memory indexes for fast queries
//! - Disk-backed indexes for large datasets
//! - Hierarchical indexes for multi-scale search

use crate::retrieval::{RerankedResult, SearchResult};
use crate::similarity::{compute_similarity, SimilarityMetric};
use embeddenator_vsa::SparseVec;
use std::collections::HashMap;

/// Index configuration
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Similarity metric to use for reranking
    pub metric: SimilarityMetric,
    /// Whether to enable hierarchical indexing
    pub hierarchical: bool,
    /// Maximum entries per leaf node (for hierarchical)
    pub leaf_size: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            metric: SimilarityMetric::Cosine,
            hierarchical: false,
            leaf_size: 1000,
        }
    }
}

/// Abstract index trait for different retrieval strategies
pub trait RetrievalIndex {
    /// Add a vector to the index
    fn add(&mut self, id: usize, vec: &SparseVec);

    /// Finalize the index (sort, optimize, etc.)
    fn finalize(&mut self);

    /// Query for top-k candidates
    fn query_top_k(&self, query: &SparseVec, k: usize) -> Vec<SearchResult>;

    /// Query and rerank with exact similarity
    fn query_top_k_reranked(
        &self,
        query: &SparseVec,
        vectors: &HashMap<usize, SparseVec>,
        candidate_k: usize,
        k: usize,
    ) -> Vec<RerankedResult>;
}

/// Brute force index - linear scan for ground truth
///
/// Useful for:
/// - Small datasets (< 10k vectors)
/// - Ground truth for accuracy testing
/// - Baseline performance comparison
#[derive(Clone, Debug)]
pub struct BruteForceIndex {
    vectors: HashMap<usize, SparseVec>,
    config: IndexConfig,
}

impl BruteForceIndex {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            vectors: HashMap::new(),
            config,
        }
    }

    /// Build index from existing vectors
    pub fn build_from_map(vectors: HashMap<usize, SparseVec>, config: IndexConfig) -> Self {
        Self { vectors, config }
    }
}

impl RetrievalIndex for BruteForceIndex {
    fn add(&mut self, id: usize, vec: &SparseVec) {
        self.vectors.insert(id, vec.clone());
    }

    fn finalize(&mut self) {
        // Nothing to do for brute force
    }

    fn query_top_k(&self, query: &SparseVec, k: usize) -> Vec<SearchResult> {
        if k == 0 || self.vectors.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let score = (compute_similarity(query, vec, self.config.metric) * 1000.0) as i32;
                SearchResult { id: *id, score }
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        results.truncate(k);
        results
    }

    fn query_top_k_reranked(
        &self,
        query: &SparseVec,
        _vectors: &HashMap<usize, SparseVec>,
        _candidate_k: usize,
        k: usize,
    ) -> Vec<RerankedResult> {
        if k == 0 || self.vectors.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<RerankedResult> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let cosine = query.cosine(vec);
                let approx_score = (cosine * 1000.0) as i32;
                RerankedResult {
                    id: *id,
                    approx_score,
                    cosine,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.cosine
                .partial_cmp(&a.cosine)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        results.truncate(k);
        results
    }
}

/// Hierarchical index using clustering for faster search
///
/// Divides the vector space into clusters and performs beam search
/// through the hierarchy.
#[derive(Clone, Debug)]
pub struct HierarchicalIndex {
    /// Cluster centroids at each level
    clusters: Vec<Vec<SparseVec>>,
    /// Mapping from cluster to member IDs
    cluster_members: Vec<Vec<Vec<usize>>>,
    /// All vectors (for reranking)
    vectors: HashMap<usize, SparseVec>,
    config: IndexConfig,
}

impl HierarchicalIndex {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            clusters: Vec::new(),
            cluster_members: Vec::new(),
            vectors: HashMap::new(),
            config,
        }
    }

    /// Build clusters from current vectors
    fn build_hierarchy(&mut self) {
        if self.vectors.is_empty() {
            return;
        }

        // Simple k-means style clustering
        // For production, use more sophisticated methods (HNSW, etc.)
        let num_clusters = (self.vectors.len() as f64).sqrt() as usize + 1;
        let mut cluster_assignment: HashMap<usize, usize> = HashMap::new();

        // Initialize clusters with random vectors
        let cluster_centers: Vec<SparseVec> =
            self.vectors.values().take(num_clusters).cloned().collect();

        // Assign each vector to nearest cluster
        for (id, vec) in &self.vectors {
            let mut best_cluster = 0;
            let mut best_score = f64::NEG_INFINITY;

            for (cluster_id, center) in cluster_centers.iter().enumerate() {
                let score = vec.cosine(center);
                if score > best_score {
                    best_score = score;
                    best_cluster = cluster_id;
                }
            }

            cluster_assignment.insert(*id, best_cluster);
        }

        // Build cluster members lists
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); num_clusters];
        for (id, cluster_id) in cluster_assignment {
            members[cluster_id].push(id);
        }

        self.clusters = vec![cluster_centers];
        self.cluster_members = vec![members];
    }
}

impl RetrievalIndex for HierarchicalIndex {
    fn add(&mut self, id: usize, vec: &SparseVec) {
        self.vectors.insert(id, vec.clone());
    }

    fn finalize(&mut self) {
        if self.config.hierarchical {
            self.build_hierarchy();
        }
    }

    fn query_top_k(&self, query: &SparseVec, k: usize) -> Vec<SearchResult> {
        if !self.config.hierarchical || self.clusters.is_empty() {
            // Fall back to brute force
            let mut results: Vec<SearchResult> = self
                .vectors
                .iter()
                .map(|(id, vec)| {
                    let score = (query.cosine(vec) * 1000.0) as i32;
                    SearchResult { id: *id, score }
                })
                .collect();

            results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
            results.truncate(k);
            return results;
        }

        // Hierarchical search: find best clusters first
        let beam_width = k.max(10);
        let mut candidate_ids: Vec<usize> = Vec::new();

        // Use configured metric for cluster scoring
        let metric = self.config.metric;
        if let Some(top_level_clusters) = self.clusters.first() {
            let mut cluster_scores: Vec<(usize, f64)> = top_level_clusters
                .iter()
                .enumerate()
                .map(|(idx, center)| (idx, compute_similarity(query, center, metric)))
                .collect();

            cluster_scores
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Get candidates from top clusters
            for (cluster_id, _score) in cluster_scores.iter().take(beam_width) {
                if let Some(level_members) = self.cluster_members.first() {
                    if let Some(members) = level_members.get(*cluster_id) {
                        candidate_ids.extend(members);
                    }
                }
            }
        }

        // Score all candidates using configured metric
        let metric = self.config.metric;
        let mut results: Vec<SearchResult> = candidate_ids
            .into_iter()
            .filter_map(|id| {
                self.vectors.get(&id).map(|vec| {
                    let score = (compute_similarity(query, vec, metric) * 1000.0) as i32;
                    SearchResult { id, score }
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        results.truncate(k);
        results
    }

    fn query_top_k_reranked(
        &self,
        query: &SparseVec,
        _vectors: &HashMap<usize, SparseVec>,
        candidate_k: usize,
        k: usize,
    ) -> Vec<RerankedResult> {
        let candidates = self.query_top_k(query, candidate_k);

        // Rerank using configured metric
        let metric = self.config.metric;
        let mut results: Vec<RerankedResult> = candidates
            .into_iter()
            .filter_map(|cand| {
                self.vectors.get(&cand.id).map(|vec| RerankedResult {
                    id: cand.id,
                    approx_score: cand.score,
                    cosine: compute_similarity(query, vec, metric),
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.cosine
                .partial_cmp(&a.cosine)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        results.truncate(k);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddenator_vsa::ReversibleVSAConfig;

    #[test]
    fn test_brute_force_index() {
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
        assert_eq!(results[0].id, 1); // Should match apple best
    }

    #[test]
    fn test_hierarchical_index() {
        let config = ReversibleVSAConfig::default();
        let index_config = IndexConfig {
            hierarchical: true,
            ..IndexConfig::default()
        };
        let mut index = HierarchicalIndex::new(index_config);

        // Add multiple vectors
        for i in 0..20 {
            let data = format!("doc-{}", i);
            let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
            index.add(i, &vec);
        }
        index.finalize();

        let query = SparseVec::encode_data(b"doc-5", &config, None);
        let results = index.query_top_k(&query, 5);

        assert!(!results.is_empty());
    }
}
