//! Retrieval primitives for semantic search over sparse ternary vectors.
//!
//! This module provides an inverted index over `SparseVec` entries to enable
//! sub-linear candidate generation for similarity search. The intended usage is:
//! 1) Build an index over a collection (e.g., an Engram codebook).
//! 2) Query to generate candidates with approximate dot scores.
//! 3) Optionally rerank candidates using exact cosine similarity.

use crate::vsa::{SparseVec, DIM};
use std::collections::HashMap;

#[cfg(feature = "metrics")]
use crate::metrics::metrics;

#[cfg(feature = "metrics")]
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub id: usize,
    pub score: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RerankedResult {
    pub id: usize,
    /// Approximate score from inverted-index accumulation (sparse dot proxy).
    pub approx_score: i32,
    /// Exact cosine similarity computed against the stored vector.
    pub cosine: f64,
}

/// Inverted index for sparse ternary vectors.
///
/// For each dimension `d`, store the IDs that contain `d` in `pos` or `neg`.
///
/// Querying accumulates dot-product contributions from the postings lists.
#[derive(Clone, Debug)]
pub struct TernaryInvertedIndex {
    pos_postings: Vec<Vec<usize>>,
    neg_postings: Vec<Vec<usize>>,
    max_id: usize,
}

impl TernaryInvertedIndex {
    pub fn new() -> Self {
        Self {
            pos_postings: vec![Vec::new(); DIM],
            neg_postings: vec![Vec::new(); DIM],
            max_id: 0,
        }
    }

    /// Build an index from `(id, vector)` pairs.
    ///
    /// IDs do not need to be contiguous.
    pub fn build_from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (usize, SparseVec)>,
    {
        let mut index = Self::new();
        for (id, vec) in pairs {
            index.add(id, &vec);
        }
        index.finalize();
        index
    }

    /// Build an index from a codebook-style map.
    pub fn build_from_map(map: &HashMap<usize, SparseVec>) -> Self {
        let mut index = Self::new();
        for (&id, vec) in map {
            index.add(id, vec);
        }
        index.finalize();
        index
    }

    /// Add a vector under `id`.
    ///
    /// Call `finalize()` before querying for best performance.
    pub fn add(&mut self, id: usize, vec: &SparseVec) {
        self.max_id = self.max_id.max(id);
        for &d in &vec.pos {
            if d < DIM {
                self.pos_postings[d].push(id);
            }
        }
        for &d in &vec.neg {
            if d < DIM {
                self.neg_postings[d].push(id);
            }
        }
    }

    /// Sort and deduplicate postings lists.
    pub fn finalize(&mut self) {
        for posting in &mut self.pos_postings {
            posting.sort_unstable();
            posting.dedup();
        }
        for posting in &mut self.neg_postings {
            posting.sort_unstable();
            posting.dedup();
        }
    }

    /// Query for top-k candidates by approximate dot score.
    ///
    /// Score is the sparse ternary dot product derived from index hits.
    pub fn query_top_k(&self, query: &SparseVec, k: usize) -> Vec<SearchResult> {
        if k == 0 {
            return Vec::new();
        }

        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let mut scores = vec![0i32; self.max_id + 1];
        let mut touched = Vec::new();
        let mut touched_flag = vec![false; self.max_id + 1];

        // Query +1 dimensions
        for &d in &query.pos {
            if d >= DIM {
                continue;
            }
            for &id in &self.pos_postings[d] {
                if !touched_flag[id] {
                    touched_flag[id] = true;
                    touched.push(id);
                }
                scores[id] += 1;
            }
            for &id in &self.neg_postings[d] {
                if !touched_flag[id] {
                    touched_flag[id] = true;
                    touched.push(id);
                }
                scores[id] -= 1;
            }
        }

        // Query -1 dimensions
        for &d in &query.neg {
            if d >= DIM {
                continue;
            }
            for &id in &self.pos_postings[d] {
                if !touched_flag[id] {
                    touched_flag[id] = true;
                    touched.push(id);
                }
                scores[id] -= 1;
            }
            for &id in &self.neg_postings[d] {
                if !touched_flag[id] {
                    touched_flag[id] = true;
                    touched.push(id);
                }
                scores[id] += 1;
            }
        }

        // Collect and select top-k.
        let mut results: Vec<SearchResult> = touched
            .into_iter()
            .map(|id| SearchResult { id, score: scores[id] })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        results.truncate(k);

        #[cfg(feature = "metrics")]
        metrics().record_retrieval_query(start.elapsed());

        results
    }

    /// Query for top-k candidates, then rerank them by exact cosine similarity.
    ///
    /// `candidate_k` controls how many approximate candidates are generated
    /// before reranking.
    pub fn query_top_k_reranked(
        &self,
        query: &SparseVec,
        vectors: &HashMap<usize, SparseVec>,
        candidate_k: usize,
        k: usize,
    ) -> Vec<RerankedResult> {
        let candidates = self.query_top_k(query, candidate_k);
        rerank_candidates_by_cosine(query, &candidates, vectors, k)
    }
}

impl Default for TernaryInvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Rerank inverted-index candidates using exact cosine similarity.
pub fn rerank_candidates_by_cosine(
    query: &SparseVec,
    candidates: &[SearchResult],
    vectors: &HashMap<usize, SparseVec>,
    k: usize,
) -> Vec<RerankedResult> {
    if k == 0 || candidates.is_empty() {
        return Vec::new();
    }

    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let mut out = Vec::with_capacity(candidates.len().min(k));
    for cand in candidates {
        let Some(vec) = vectors.get(&cand.id) else {
            continue;
        };
        out.push(RerankedResult {
            id: cand.id,
            approx_score: cand.score,
            cosine: query.cosine(vec),
        });
    }

    out.sort_by(|a, b| {
        b.cosine
            .partial_cmp(&a.cosine)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.approx_score.cmp(&a.approx_score))
            .then_with(|| a.id.cmp(&b.id))
    });
    out.truncate(k);

    #[cfg(feature = "metrics")]
    metrics().record_rerank(start.elapsed());

    out
}
