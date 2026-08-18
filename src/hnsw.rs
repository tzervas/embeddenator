//! Hierarchical Navigable Small World (HNSW) Index
//!
//! HNSW is a graph-based approximate nearest neighbor search algorithm that
//! provides logarithmic search complexity with high recall (>95%).
//!
//! # Algorithm Overview
//!
//! HNSW builds a multi-layer graph where:
//! - Each layer is a navigable small-world graph
//! - Higher layers have exponentially fewer nodes
//! - Search starts from the top layer and descends
//! - Each layer refines the search with denser connections
//!
//! # Performance Characteristics
//!
//! | Metric | Complexity |
//! |--------|------------|
//! | Build  | O(N log N) |
//! | Search | O(log N)   |
//! | Insert | O(log N)   |
//! | Memory | O(N × M)   |
//!
//! # Example
//!
//! ```
//! use embeddenator_retrieval::hnsw::{HNSWIndex, HNSWConfig};
//! use embeddenator_retrieval::index::{IndexConfig, RetrievalIndex};
//! use embeddenator_vsa::SparseVec;
//!
//! let config = HNSWConfig::default();
//! let index_config = IndexConfig::default();
//! let mut index = HNSWIndex::new(config, index_config);
//!
//! // Add vectors
//! #[allow(deprecated)]
//! let vec = SparseVec::from_data(b"example");
//! index.add(1, &vec);
//! index.finalize();
//!
//! // Search
//! let results = index.query_top_k(&vec, 5);
//! ```

use crate::index::IndexConfig;
use crate::retrieval::{RerankedResult, SearchResult};
use crate::similarity::compute_similarity;
use embeddenator_vsa::SparseVec;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Configuration for HNSW index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HNSWConfig {
    /// Maximum number of connections per element per layer (default: 16)
    pub m: usize,
    /// Maximum number of connections for layer 0 (default: 2*M = 32)
    pub m_max0: usize,
    /// Size of the dynamic candidate list during construction (default: 200)
    pub ef_construction: usize,
    /// Size of the dynamic candidate list during search (default: 50)
    pub ef_search: usize,
    /// Level generation factor (default: 1/ln(M))
    pub ml: f64,
}

impl Default for HNSWConfig {
    fn default() -> Self {
        let m = 16;
        Self {
            m,
            m_max0: m * 2,
            ef_construction: 200,
            ef_search: 50,
            // ml = 1/ln(M) for optimal layer distribution
            ml: 1.0 / (m as f64).ln(),
        }
    }
}

impl HNSWConfig {
    /// Create a config optimized for speed (lower recall)
    pub fn fast() -> Self {
        Self {
            m: 12,
            m_max0: 24,
            ef_construction: 100,
            ef_search: 20,
            ml: 1.0 / 12_f64.ln(),
        }
    }

    /// Create a config optimized for accuracy (higher recall)
    pub fn accurate() -> Self {
        Self {
            m: 32,
            m_max0: 64,
            ef_construction: 400,
            ef_search: 200,
            ml: 1.0 / 32_f64.ln(),
        }
    }

    /// Builder method to set M
    pub fn with_m(mut self, m: usize) -> Self {
        self.m = m;
        self.m_max0 = m * 2;
        self.ml = 1.0 / (m as f64).ln();
        self
    }

    /// Builder method to set ef_search
    pub fn with_ef_search(mut self, ef: usize) -> Self {
        self.ef_search = ef;
        self
    }

    /// Builder method to set ef_construction
    pub fn with_ef_construction(mut self, ef: usize) -> Self {
        self.ef_construction = ef;
        self
    }
}

/// A node in the HNSW graph
#[derive(Clone, Debug)]
struct HNSWNode {
    /// Unique identifier (used for debugging and stats)
    #[allow(dead_code)]
    id: usize,
    /// The vector data
    vec: SparseVec,
    /// Maximum layer this node appears in (0-indexed, used for stats)
    #[allow(dead_code)]
    level: usize,
    /// Neighbors at each layer: neighbors[layer] = list of neighbor IDs
    neighbors: Vec<Vec<usize>>,
}

impl HNSWNode {
    fn new(id: usize, vec: SparseVec, level: usize) -> Self {
        Self {
            id,
            vec,
            level,
            neighbors: vec![Vec::new(); level + 1],
        }
    }
}

/// Candidate for nearest neighbor search with distance ordering
#[derive(Clone, Copy, Debug)]
struct Candidate {
    id: usize,
    distance: f64,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap (smaller distance = higher priority)
        other
            .distance
            .partial_cmp(&self.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// HNSW index for approximate nearest neighbor search
///
/// Implements the Hierarchical Navigable Small World algorithm for
/// efficient similarity search in high-dimensional spaces.
#[derive(Clone, Debug)]
pub struct HNSWIndex {
    /// HNSW-specific configuration
    hnsw_config: HNSWConfig,
    /// General index configuration (similarity metric, etc.)
    index_config: IndexConfig,
    /// All nodes in the graph
    nodes: HashMap<usize, HNSWNode>,
    /// Entry point (highest-level node ID)
    entry_point: Option<usize>,
    /// Current maximum level in the graph
    max_level: usize,
    /// Random seed for deterministic level generation
    rng_state: u64,
}

impl HNSWIndex {
    /// Create a new HNSW index with the given configuration
    pub fn new(hnsw_config: HNSWConfig, index_config: IndexConfig) -> Self {
        Self {
            hnsw_config,
            index_config,
            nodes: HashMap::new(),
            entry_point: None,
            max_level: 0,
            rng_state: 0x5DEECE66D, // Same as Java Random
        }
    }

    /// Create with default HNSW config
    pub fn with_index_config(index_config: IndexConfig) -> Self {
        Self::new(HNSWConfig::default(), index_config)
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the current maximum level
    pub fn max_level(&self) -> usize {
        self.max_level
    }

    /// Set ef_search parameter for runtime tuning
    pub fn set_ef_search(&mut self, ef: usize) {
        self.hnsw_config.ef_search = ef;
    }

    /// Generate a random level for a new node using the configured ml factor
    fn generate_level(&mut self) -> usize {
        // LCG random number generator
        self.rng_state = self.rng_state.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
        let r = (self.rng_state >> 17) as f64 / (1u64 << 47) as f64;

        // Level = floor(-ln(uniform) * ml)
        (-(r.max(f64::MIN_POSITIVE).ln()) * self.hnsw_config.ml) as usize
    }

    /// Compute distance between two vectors (1 - cosine similarity)
    fn distance(&self, a: &SparseVec, b: &SparseVec) -> f64 {
        1.0 - compute_similarity(a, b, self.index_config.metric)
    }

    /// Search for the ef nearest neighbors of query starting from entry_point at given layer
    ///
    /// Returns a list of (id, distance) pairs sorted by distance (ascending)
    fn search_layer(
        &self,
        query: &SparseVec,
        entry_points: &[usize],
        ef: usize,
        layer: usize,
    ) -> Vec<(usize, f64)> {
        let mut visited: HashSet<usize> = entry_points.iter().copied().collect();

        // Min-heap for candidates to explore (closest first)
        let mut candidates: BinaryHeap<Candidate> = BinaryHeap::new();
        // Max-heap for results (furthest first for easy pruning)
        let mut results: BinaryHeap<Reverse<Candidate>> = BinaryHeap::new();

        // Initialize with entry points
        for &ep in entry_points {
            if let Some(node) = self.nodes.get(&ep) {
                let dist = self.distance(query, &node.vec);
                candidates.push(Candidate {
                    id: ep,
                    distance: dist,
                });
                results.push(Reverse(Candidate {
                    id: ep,
                    distance: dist,
                }));
            }
        }

        while let Some(Candidate {
            id: current_id,
            distance: current_dist,
        }) = candidates.pop()
        {
            // Get the furthest result distance for comparison
            let furthest_dist = results
                .peek()
                .map(|Reverse(c)| c.distance)
                .unwrap_or(f64::INFINITY);

            // If current candidate is further than all results, we're done
            if current_dist > furthest_dist && results.len() >= ef {
                break;
            }

            // Explore neighbors at this layer
            if let Some(node) = self.nodes.get(&current_id) {
                if layer < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[layer] {
                        if visited.insert(neighbor_id) {
                            if let Some(neighbor) = self.nodes.get(&neighbor_id) {
                                let dist = self.distance(query, &neighbor.vec);

                                // Add to candidates if better than worst result or results not full
                                let should_add = results.len() < ef || {
                                    let worst = results
                                        .peek()
                                        .map(|Reverse(c)| c.distance)
                                        .unwrap_or(f64::INFINITY);
                                    dist < worst
                                };

                                if should_add {
                                    candidates.push(Candidate {
                                        id: neighbor_id,
                                        distance: dist,
                                    });
                                    results.push(Reverse(Candidate {
                                        id: neighbor_id,
                                        distance: dist,
                                    }));

                                    // Prune results if exceeding ef
                                    if results.len() > ef {
                                        results.pop();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Extract and sort results
        let mut result_vec: Vec<(usize, f64)> = results
            .into_iter()
            .map(|Reverse(c)| (c.id, c.distance))
            .collect();
        result_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        result_vec
    }

    /// Select neighbors using the heuristic algorithm
    ///
    /// Prefers neighbors that provide diverse directions, not just closest
    fn select_neighbors_heuristic(
        &self,
        _query: &SparseVec,
        candidates: &[(usize, f64)],
        m: usize,
    ) -> Vec<usize> {
        if candidates.len() <= m {
            return candidates.iter().map(|(id, _)| *id).collect();
        }

        let mut selected: Vec<usize> = Vec::with_capacity(m);
        let mut working: Vec<(usize, f64)> = candidates.to_vec();

        // Greedy selection: pick closest, then pick next that's not too close to already selected
        while selected.len() < m && !working.is_empty() {
            // Sort by distance (in case order changed)
            working.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            // Take the closest
            let (best_id, best_dist) = working.remove(0);

            // Check if this candidate is closer to query than to any already selected neighbor
            // This ensures diversity in the neighbor set
            let is_diverse = selected.iter().all(|&sel_id| {
                if let (Some(best_node), Some(sel_node)) =
                    (self.nodes.get(&best_id), self.nodes.get(&sel_id))
                {
                    let inter_dist = self.distance(&best_node.vec, &sel_node.vec);
                    best_dist <= inter_dist
                } else {
                    true
                }
            });

            if is_diverse || selected.len() < m / 2 {
                selected.push(best_id);
            }

            // If we still need more and have exhausted diverse candidates, just take closest
            if selected.len() < m && working.is_empty() && !is_diverse {
                // Re-add to working list and try again without diversity check
                working.push((best_id, best_dist));
                for (id, dist) in candidates {
                    if !selected.contains(id) && !working.iter().any(|(wid, _)| wid == id) {
                        working.push((*id, *dist));
                    }
                }
            }
        }

        // If still not enough, just take closest remaining
        if selected.len() < m {
            let remaining: Vec<usize> = candidates
                .iter()
                .filter(|(id, _)| !selected.contains(id))
                .map(|(id, _)| *id)
                .take(m - selected.len())
                .collect();
            selected.extend(remaining);
        }

        selected
    }

    /// Add bidirectional edge between two nodes at a given layer
    fn connect_nodes(&mut self, id1: usize, id2: usize, layer: usize) {
        let m_max = if layer == 0 {
            self.hnsw_config.m_max0
        } else {
            self.hnsw_config.m
        };

        // Add id2 to id1's neighbors
        if let Some(node1) = self.nodes.get_mut(&id1) {
            if layer < node1.neighbors.len() && !node1.neighbors[layer].contains(&id2) {
                node1.neighbors[layer].push(id2);
            }
        }

        // Add id1 to id2's neighbors
        if let Some(node2) = self.nodes.get_mut(&id2) {
            if layer < node2.neighbors.len() && !node2.neighbors[layer].contains(&id1) {
                node2.neighbors[layer].push(id1);
            }
        }

        // Prune if over capacity (for both nodes)
        for id in [id1, id2] {
            if let Some(node) = self.nodes.get(&id) {
                if layer < node.neighbors.len() && node.neighbors[layer].len() > m_max {
                    // Need to prune - get the node's vector and neighbors
                    let query = node.vec.clone();
                    let neighbors: Vec<usize> = node.neighbors[layer].clone();

                    // Compute distances to all neighbors
                    let candidates: Vec<(usize, f64)> = neighbors
                        .iter()
                        .filter_map(|&nid| {
                            self.nodes
                                .get(&nid)
                                .map(|n| (nid, self.distance(&query, &n.vec)))
                        })
                        .collect();

                    // Select best neighbors
                    let selected = self.select_neighbors_heuristic(&query, &candidates, m_max);

                    // Update the neighbors list
                    if let Some(node) = self.nodes.get_mut(&id) {
                        if layer < node.neighbors.len() {
                            node.neighbors[layer] = selected;
                        }
                    }
                }
            }
        }
    }

    /// Insert a new vector into the index
    ///
    /// This is the main HNSW insertion algorithm
    pub fn insert(&mut self, id: usize, vec: &SparseVec) {
        let level = self.generate_level();
        let node = HNSWNode::new(id, vec.clone(), level);

        // If this is the first node, just add it
        if self.nodes.is_empty() {
            self.nodes.insert(id, node);
            self.entry_point = Some(id);
            self.max_level = level;
            return;
        }

        let entry_point = self.entry_point.expect("Entry point should exist");
        let mut curr_ep = vec![entry_point];

        // Search from top to level+1, finding single nearest neighbor
        for lc in (level + 1..=self.max_level).rev() {
            let nearest = self.search_layer(vec, &curr_ep, 1, lc);
            if let Some((nearest_id, _)) = nearest.first() {
                curr_ep = vec![*nearest_id];
            }
        }

        // Add the node to the graph
        self.nodes.insert(id, node);

        // Search and connect at each layer from min(level, max_level) down to 0
        let top_layer = level.min(self.max_level);
        for lc in (0..=top_layer).rev() {
            let candidates = self.search_layer(vec, &curr_ep, self.hnsw_config.ef_construction, lc);

            let m = if lc == 0 {
                self.hnsw_config.m_max0
            } else {
                self.hnsw_config.m
            };

            // Select neighbors
            let neighbors = self.select_neighbors_heuristic(vec, &candidates, m);

            // Connect to selected neighbors
            for &neighbor_id in &neighbors {
                self.connect_nodes(id, neighbor_id, lc);
            }

            // Update entry points for next layer
            curr_ep = candidates.iter().map(|(cid, _)| *cid).collect();
        }

        // Update entry point if new node has higher level
        if level > self.max_level {
            self.entry_point = Some(id);
            self.max_level = level;
        }
    }

    /// Search for k nearest neighbors
    fn search(&self, query: &SparseVec, k: usize) -> Vec<(usize, f64)> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        let entry_point = match self.entry_point {
            Some(ep) => ep,
            None => return Vec::new(),
        };

        let mut curr_ep = vec![entry_point];

        // Search from top layer down to layer 1 with ef=1
        for lc in (1..=self.max_level).rev() {
            let nearest = self.search_layer(query, &curr_ep, 1, lc);
            if let Some((nearest_id, _)) = nearest.first() {
                curr_ep = vec![*nearest_id];
            }
        }

        // Search layer 0 with ef_search
        let candidates = self.search_layer(query, &curr_ep, self.hnsw_config.ef_search.max(k), 0);

        // Return top k
        candidates.into_iter().take(k).collect()
    }

    /// Get index statistics
    pub fn stats(&self) -> HNSWStats {
        let mut nodes_per_level = vec![0usize; self.max_level + 1];
        let mut total_edges = 0usize;

        for node in self.nodes.values() {
            for (layer, neighbors) in node.neighbors.iter().enumerate() {
                if layer <= self.max_level {
                    nodes_per_level[layer] += 1;
                    total_edges += neighbors.len();
                }
            }
        }

        HNSWStats {
            num_vectors: self.nodes.len(),
            max_level: self.max_level,
            nodes_per_level,
            total_edges: total_edges / 2, // Each edge counted twice
            avg_degree: if self.nodes.is_empty() {
                0.0
            } else {
                total_edges as f64 / self.nodes.len() as f64
            },
        }
    }
}

/// Statistics about the HNSW index
#[derive(Debug, Clone)]
pub struct HNSWStats {
    /// Total number of vectors
    pub num_vectors: usize,
    /// Maximum level in the hierarchy
    pub max_level: usize,
    /// Number of nodes at each level
    pub nodes_per_level: Vec<usize>,
    /// Total number of edges in the graph
    pub total_edges: usize,
    /// Average degree (edges per node)
    pub avg_degree: f64,
}

impl crate::index::RetrievalIndex for HNSWIndex {
    fn add(&mut self, id: usize, vec: &SparseVec) {
        self.insert(id, vec);
    }

    fn finalize(&mut self) {
        // HNSW is already built incrementally, nothing to do
        // Could add compaction/optimization here in the future
    }

    fn query_top_k(&self, query: &SparseVec, k: usize) -> Vec<SearchResult> {
        if k == 0 {
            return Vec::new();
        }

        self.search(query, k)
            .into_iter()
            .map(|(id, distance)| {
                // Convert distance back to similarity score (scaled)
                let similarity = 1.0 - distance;
                let score = (similarity * 1000.0) as i32;
                SearchResult { id, score }
            })
            .collect()
    }

    fn query_top_k_reranked(
        &self,
        query: &SparseVec,
        vectors: &HashMap<usize, SparseVec>,
        candidate_k: usize,
        k: usize,
    ) -> Vec<RerankedResult> {
        if k == 0 {
            return Vec::new();
        }

        // Get candidates from HNSW search
        let candidates = self.search(query, candidate_k);

        // Rerank with exact cosine similarity
        let mut results: Vec<RerankedResult> = candidates
            .into_iter()
            .filter_map(|(id, _dist)| {
                // Use stored vector if available, otherwise use HNSW's copy
                let vec = vectors
                    .get(&id)
                    .or_else(|| self.nodes.get(&id).map(|n| &n.vec))?;

                let cosine = query.cosine(vec);
                let approx_score = (cosine * 1000.0) as i32;

                Some(RerankedResult {
                    id,
                    approx_score,
                    cosine,
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
    use crate::index::RetrievalIndex;

    #[allow(deprecated)]
    fn create_test_vector(data: &[u8]) -> SparseVec {
        // Use from_data for faster test vector creation
        SparseVec::from_data(data)
    }

    #[test]
    fn test_hnsw_basic() {
        let config = HNSWConfig::default();
        let index_config = IndexConfig::default();
        let mut index = HNSWIndex::new(config, index_config);

        // Add some vectors
        let vec1 = create_test_vector(b"apple");
        let vec2 = create_test_vector(b"banana");
        let vec3 = create_test_vector(b"cherry");

        index.add(1, &vec1);
        index.add(2, &vec2);
        index.add(3, &vec3);

        assert_eq!(index.len(), 3);

        // Query for apple - should find itself
        let results = index.query_top_k(&vec1, 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_hnsw_empty() {
        let config = HNSWConfig::default();
        let index_config = IndexConfig::default();
        let index = HNSWIndex::new(config, index_config);

        let query = create_test_vector(b"test");
        let results = index.query_top_k(&query, 5);

        assert!(results.is_empty());
    }

    #[test]
    fn test_hnsw_single_element() {
        let config = HNSWConfig::default();
        let index_config = IndexConfig::default();
        let mut index = HNSWIndex::new(config, index_config);

        let vec = create_test_vector(b"single");
        index.add(42, &vec);

        let results = index.query_top_k(&vec, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 42);
    }

    #[test]
    fn test_hnsw_many_vectors() {
        let config = HNSWConfig::fast(); // Use fast config for tests
        let index_config = IndexConfig::default();
        let mut index = HNSWIndex::new(config, index_config);

        // Add 10 vectors (reduced for faster tests in debug mode)
        for i in 0..10 {
            let data = format!("document-{}", i);
            let vec = create_test_vector(data.as_bytes());
            index.add(i, &vec);
        }

        assert_eq!(index.len(), 10);

        // Stats should show structure
        let stats = index.stats();
        assert_eq!(stats.num_vectors, 10);

        // Query should return results
        let query = create_test_vector(b"document-5");
        let results = index.query_top_k(&query, 5);
        assert!(!results.is_empty());
        assert!(results.len() <= 5);

        // The query vector (5) should be in top results
        let top_ids: Vec<usize> = results.iter().map(|r| r.id).collect();
        assert!(
            top_ids.contains(&5),
            "Query vector should be in top results"
        );
    }

    #[test]
    fn test_hnsw_reranking() {
        let config = HNSWConfig::default();
        let index_config = IndexConfig::default();
        let mut index = HNSWIndex::new(config, index_config);
        let mut vectors = HashMap::new();

        // Add vectors
        for i in 0..20 {
            let data = format!("item-{}", i);
            let vec = create_test_vector(data.as_bytes());
            index.add(i, &vec);
            vectors.insert(i, vec);
        }

        let query = create_test_vector(b"item-5");
        let results = index.query_top_k_reranked(&query, &vectors, 10, 5);

        assert!(!results.is_empty());
        assert!(results.len() <= 5);

        // Results should be sorted by cosine
        for i in 1..results.len() {
            assert!(results[i - 1].cosine >= results[i].cosine);
        }
    }

    #[test]
    fn test_hnsw_config_builders() {
        let fast = HNSWConfig::fast();
        assert!(fast.m < HNSWConfig::default().m);
        assert!(fast.ef_search < HNSWConfig::default().ef_search);

        let accurate = HNSWConfig::accurate();
        assert!(accurate.m > HNSWConfig::default().m);
        assert!(accurate.ef_search > HNSWConfig::default().ef_search);

        let custom = HNSWConfig::default()
            .with_m(24)
            .with_ef_search(100)
            .with_ef_construction(300);
        assert_eq!(custom.m, 24);
        assert_eq!(custom.ef_search, 100);
        assert_eq!(custom.ef_construction, 300);
    }

    #[test]
    fn test_hnsw_stats() {
        let config = HNSWConfig::fast(); // Use fast config for tests
        let index_config = IndexConfig::default();
        let mut index = HNSWIndex::new(config, index_config);

        for i in 0..20 {
            let data = format!("vec-{}", i);
            let vec = create_test_vector(data.as_bytes());
            index.add(i, &vec);
        }

        let stats = index.stats();
        assert_eq!(stats.num_vectors, 20);
        assert!(stats.total_edges > 0);
        assert!(stats.avg_degree > 0.0);
    }
}
