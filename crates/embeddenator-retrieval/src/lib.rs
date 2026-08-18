//! # embeddenator-retrieval
//!
//! Semantic retrieval and search operations for VSA-based vector representations.
//!
//! This crate provides:
//! - **Inverted indexing** for fast approximate search
//! - **Multiple similarity metrics** (cosine, Hamming, Jaccard)
//! - **Search strategies** (exact, approximate, two-stage, hierarchical)
//! - **Index structures** (brute force, hierarchical)
//! - **Resonator networks** for pattern completion and factorization
//! - **Algebraic correction** for guaranteed reconstruction
//!
//! Extracted from embeddenator core as part of Phase 2A component decomposition.
//! See [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md).
//!
//! # Examples
//!
//! ## Basic Retrieval
//!
//! ```
//! use embeddenator_retrieval::{TernaryInvertedIndex, search::two_stage_search, search::SearchConfig};
//! use embeddenator_vsa::SparseVec;
//! use std::collections::HashMap;
//!
//! // Build index
//! let mut index = TernaryInvertedIndex::new();
//! let mut vectors = HashMap::new();
//!
//! let vec1 = SparseVec::from_data(b"document one");
//! let vec2 = SparseVec::from_data(b"document two");
//!
//! index.add(1, &vec1);
//! index.add(2, &vec2);
//! index.finalize();
//!
//! vectors.insert(1, vec1);
//! vectors.insert(2, vec2);
//!
//! // Search
//! let query = SparseVec::from_data(b"document");
//! let config = SearchConfig::default();
//! let results = two_stage_search(&query, &index, &vectors, &config, 5);
//!
//! assert!(!results.is_empty());
//! ```
//!
//! ## Similarity Metrics
//!
//! ```
//! use embeddenator_retrieval::similarity::{compute_similarity, SimilarityMetric};
//! use embeddenator_vsa::SparseVec;
//!
//! let a = SparseVec::from_data(b"hello");
//! let b = SparseVec::from_data(b"hello");
//!
//! let cosine = compute_similarity(&a, &b, SimilarityMetric::Cosine);
//! let hamming = compute_similarity(&a, &b, SimilarityMetric::Hamming);
//! let jaccard = compute_similarity(&a, &b, SimilarityMetric::Jaccard);
//!
//! assert!(cosine > 0.9);
//! assert!(hamming < 10.0);
//! ```

pub mod core;
pub mod distributed;
pub mod hnsw;
pub mod index;
pub mod retrieval;
pub mod search;
pub mod similarity;

// Re-export key types for convenience
pub use core::{correction, resonator};
pub use hnsw::{HNSWConfig, HNSWIndex, HNSWStats};
pub use index::{BruteForceIndex, HierarchicalIndex, IndexConfig, RetrievalIndex};
pub use retrieval::*;
pub use search::{
    approximate_search, batch_search, exact_search, exact_search_parallel, two_stage_search,
    RankedResult, SearchConfig,
};
pub use similarity::{compute_similarity, SimilarityMetric};

// Distributed search (#51)
pub use distributed::{
    DistributedConfig, DistributedError, DistributedResult, DistributedSearch,
    DistributedSearchBuilder, QueryStats, Shard, ShardAssigner, ShardId, ShardResult, ShardStatus,
    ShardingStrategy,
};

// Convenience wrappers for integration tests
use embeddenator_vsa::SparseVec;
use std::collections::HashMap;

/// Builder for creating a search index
pub struct IndexBuilder {
    vectors: HashMap<String, SparseVec>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
        }
    }

    pub fn add_vector(&mut self, id: String, vec: SparseVec) {
        self.vectors.insert(id, vec);
    }

    pub fn build(self) -> SearchIndex {
        SearchIndex {
            vectors: self.vectors,
        }
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Search index for querying
#[derive(Clone)]
pub struct SearchIndex {
    vectors: HashMap<String, SparseVec>,
}

/// Query result
pub struct QueryResult {
    pub id: String,
    pub score: f64,
}

/// Query engine for search operations
pub struct QueryEngine {
    index: SearchIndex,
}

impl QueryEngine {
    pub fn new(index: SearchIndex) -> Self {
        Self { index }
    }

    pub fn top_k(&self, query: &SparseVec, k: usize) -> Vec<QueryResult> {
        let mut results: Vec<(String, f64)> = self
            .index
            .vectors
            .iter()
            .map(|(id, vec)| {
                let score = compute_similarity(query, vec, SimilarityMetric::Cosine);
                (id.clone(), score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        results
            .into_iter()
            .map(|(id, score)| QueryResult { id, score })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn component_loads() {
        // Simply verify the module compiles
    }
}
