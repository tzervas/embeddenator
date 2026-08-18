//! Distributed Search Infrastructure
//!
//! This module provides primitives for distributed semantic search across
//! multiple nodes/shards. It handles:
//!
//! - **Sharding**: Partitioning data across nodes
//! - **Query routing**: Fan-out queries to relevant shards
//! - **Result aggregation**: Merge results from multiple shards
//! - **Topology management**: Track available nodes
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Distributed Search                        │
//! │  ┌─────────────┐                                            │
//! │  │   Query     │                                            │
//! │  └──────┬──────┘                                            │
//! │         │                                                    │
//! │         ▼                                                    │
//! │  ┌─────────────┐    ┌──────────────────────────────────┐   │
//! │  │   Router    │───▶│         Shard Cluster            │   │
//! │  └──────┬──────┘    │  ┌─────┐ ┌─────┐ ┌─────┐        │   │
//! │         │           │  │ S0  │ │ S1  │ │ S2  │ ...    │   │
//! │         ▼           │  └──┬──┘ └──┬──┘ └──┬──┘        │   │
//! │  ┌─────────────┐    └────│───────│───────│───────────┘   │
//! │  │ Aggregator  │◀────────┴───────┴───────┘                │
//! │  └──────┬──────┘                                            │
//! │         │                                                    │
//! │         ▼                                                    │
//! │  ┌─────────────┐                                            │
//! │  │   Results   │                                            │
//! │  └─────────────┘                                            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_retrieval::distributed::{
//!     DistributedSearch, Shard, ShardId, DistributedConfig,
//! };
//! use embeddenator_vsa::SparseVec;
//!
//! // Create shards (each could be on a different node)
//! let mut shard0 = Shard::new(ShardId(0));
//! shard0.add(1, SparseVec::from_data(b"document one"));
//! shard0.finalize();
//!
//! let mut shard1 = Shard::new(ShardId(1));
//! shard1.add(2, SparseVec::from_data(b"document two"));
//! shard1.finalize();
//!
//! // Create distributed search coordinator
//! let mut search = DistributedSearch::new(DistributedConfig::default());
//! search.add_shard(shard0);
//! search.add_shard(shard1);
//!
//! // Execute distributed query
//! let query = SparseVec::from_data(b"document");
//! let (results, stats) = search.query(&query, 10)?;
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::retrieval::TernaryInvertedIndex;
use crate::search::{two_stage_search, SearchConfig};
use embeddenator_vsa::SparseVec;

/// Unique identifier for a shard
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardId(pub u32);

impl ShardId {
    /// Create from integer
    pub fn from_u32(id: u32) -> Self {
        Self(id)
    }
}

/// Shard status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ShardStatus {
    /// Shard is healthy and accepting queries
    #[default]
    Healthy,
    /// Shard is degraded (slow but functional)
    Degraded,
    /// Shard is offline
    Offline,
    /// Shard is rebuilding index
    Rebuilding,
}

/// A single shard containing a partition of the search corpus
#[derive(Debug)]
pub struct Shard {
    /// Unique shard identifier
    pub id: ShardId,
    /// Status of this shard
    pub status: ShardStatus,
    /// Inverted index for this shard's data
    index: TernaryInvertedIndex,
    /// Vectors in this shard
    vectors: HashMap<usize, SparseVec>,
    /// Document count
    doc_count: usize,
    /// Query counter
    query_count: AtomicU64,
}

impl Shard {
    /// Create a new empty shard
    pub fn new(id: ShardId) -> Self {
        Self {
            id,
            status: ShardStatus::Healthy,
            index: TernaryInvertedIndex::new(),
            vectors: HashMap::new(),
            doc_count: 0,
            query_count: AtomicU64::new(0),
        }
    }

    /// Add a document to this shard
    pub fn add(&mut self, doc_id: usize, vec: SparseVec) {
        self.index.add(doc_id, &vec);
        self.vectors.insert(doc_id, vec);
        self.doc_count += 1;
    }

    /// Finalize the shard's index for querying
    pub fn finalize(&mut self) {
        self.index.finalize();
    }

    /// Query this shard locally
    pub fn query(&self, query: &SparseVec, config: &SearchConfig, k: usize) -> Vec<ShardResult> {
        self.query_count.fetch_add(1, Ordering::Relaxed);

        let results = two_stage_search(query, &self.index, &self.vectors, config, k);

        results
            .into_iter()
            .map(|r| ShardResult {
                shard_id: self.id,
                doc_id: r.id,
                score: r.score,
                approx_score: r.approx_score,
            })
            .collect()
    }

    /// Get document count
    pub fn doc_count(&self) -> usize {
        self.doc_count
    }

    /// Get query count
    pub fn query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }

    /// Check if shard is available for queries
    pub fn is_available(&self) -> bool {
        matches!(self.status, ShardStatus::Healthy | ShardStatus::Degraded)
    }

    /// Update shard status
    pub fn set_status(&mut self, status: ShardStatus) {
        self.status = status;
    }
}

/// Result from a single shard query
#[derive(Clone, Debug, PartialEq)]
pub struct ShardResult {
    /// Which shard this result came from
    pub shard_id: ShardId,
    /// Document ID within the corpus
    pub doc_id: usize,
    /// Similarity score
    pub score: f64,
    /// Approximate score from index
    pub approx_score: i32,
}

/// Aggregated result from distributed query
#[derive(Clone, Debug, PartialEq)]
pub struct DistributedResult {
    /// Document ID
    pub doc_id: usize,
    /// Source shard
    pub shard_id: ShardId,
    /// Final score
    pub score: f64,
    /// Global rank (1-indexed)
    pub rank: usize,
}

/// Configuration for distributed search
#[derive(Clone, Debug)]
pub struct DistributedConfig {
    /// Search configuration for each shard
    pub search_config: SearchConfig,
    /// Multiplier for k when querying shards (to ensure enough candidates)
    pub shard_k_multiplier: f64,
    /// Timeout per shard query (milliseconds)
    pub shard_timeout_ms: u64,
    /// Minimum shards required for valid result
    pub min_shards: usize,
    /// Enable parallel shard queries
    pub parallel_shards: bool,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            search_config: SearchConfig::default(),
            shard_k_multiplier: 2.0,
            shard_timeout_ms: 5000,
            min_shards: 1,
            parallel_shards: true,
        }
    }
}

/// Statistics from a distributed query
#[derive(Clone, Debug, Default)]
pub struct QueryStats {
    /// Total shards queried
    pub shards_queried: usize,
    /// Shards that responded successfully
    pub shards_responded: usize,
    /// Total results before aggregation
    pub total_candidates: usize,
    /// Results after deduplication
    pub unique_results: usize,
    /// Query time in milliseconds
    pub query_time_ms: u64,
}

/// Error type for distributed operations
#[derive(Debug, Clone)]
pub enum DistributedError {
    /// Not enough shards available
    InsufficientShards { available: usize, required: usize },
    /// All shard queries failed
    AllShardsFailed,
    /// Query timeout (reserved for future use; timeout handling is not yet implemented)
    Timeout,
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for DistributedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistributedError::InsufficientShards {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient shards: {} available, {} required",
                    available, required
                )
            }
            DistributedError::AllShardsFailed => write!(f, "All shard queries failed"),
            DistributedError::Timeout => write!(f, "Query timeout"),
            DistributedError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

impl std::error::Error for DistributedError {}

/// Distributed search coordinator
///
/// Manages multiple shards and coordinates distributed queries.
#[derive(Default)]
pub struct DistributedSearch {
    /// Configuration
    config: DistributedConfig,
    /// Registered shards
    shards: Vec<Arc<RwLock<Shard>>>,
    /// Total queries executed
    total_queries: AtomicU64,
}

impl DistributedSearch {
    /// Create a new distributed search coordinator
    pub fn new(config: DistributedConfig) -> Self {
        Self {
            config,
            shards: Vec::new(),
            total_queries: AtomicU64::new(0),
        }
    }

    /// Add a shard to the cluster.
    ///
    /// Callers must ensure that each shard registered with this coordinator
    /// has a unique [`ShardId`] and that the same shard is not added more
    /// than once. Adding multiple shards with the same `ShardId`, or
    /// registering the same shard repeatedly, may lead to incorrect query
    /// results and statistics.
    pub fn add_shard(&mut self, shard: Shard) {
        self.shards.push(Arc::new(RwLock::new(shard)));
    }

    /// Get number of registered shards
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Get number of available shards
    pub fn available_shard_count(&self) -> usize {
        self.shards
            .iter()
            .filter(|s| s.read().map(|s| s.is_available()).unwrap_or(false))
            .count()
    }

    /// Execute a distributed query
    pub fn query(
        &self,
        query: &SparseVec,
        k: usize,
    ) -> Result<(Vec<DistributedResult>, QueryStats), DistributedError> {
        let start = std::time::Instant::now();
        self.total_queries.fetch_add(1, Ordering::Relaxed);

        // Short-circuit when k=0 to avoid unnecessary work
        if k == 0 {
            return Ok((
                Vec::new(),
                QueryStats {
                    shards_queried: 0,
                    shards_responded: 0,
                    total_candidates: 0,
                    unique_results: 0,
                    query_time_ms: start.elapsed().as_millis() as u64,
                },
            ));
        }

        // Check shard availability
        let available_shards: Vec<_> = self
            .shards
            .iter()
            .filter(|s| s.read().map(|s| s.is_available()).unwrap_or(false))
            .collect();

        if available_shards.len() < self.config.min_shards {
            return Err(DistributedError::InsufficientShards {
                available: available_shards.len(),
                required: self.config.min_shards,
            });
        }

        // Calculate k for each shard with overflow protection
        let shard_k =
            ((k as f64 * self.config.shard_k_multiplier).min(usize::MAX as f64) as usize).max(k);

        // Query shards (parallel or sequential), tracking actual responses
        let shard_results: Vec<Vec<ShardResult>> = if self.config.parallel_shards {
            available_shards
                .par_iter()
                .filter_map(|shard| {
                    shard
                        .read()
                        .ok()
                        .map(|s| s.query(query, &self.config.search_config, shard_k))
                })
                .collect()
        } else {
            available_shards
                .iter()
                .filter_map(|shard| {
                    shard
                        .read()
                        .ok()
                        .map(|s| s.query(query, &self.config.search_config, shard_k))
                })
                .collect()
        };

        // Track actual responses vs queried
        let shards_responded = shard_results.len();

        if shard_results.is_empty() {
            return Err(DistributedError::AllShardsFailed);
        }

        // Aggregate results
        let total_candidates: usize = shard_results.iter().map(|r| r.len()).sum();
        let mut all_results: Vec<ShardResult> = shard_results.into_iter().flatten().collect();

        // Sort by score descending, then by doc_id for deterministic ordering
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.doc_id.cmp(&b.doc_id))
        });

        // Deduplicate by doc_id (keep highest score)
        let mut seen = std::collections::HashSet::new();
        let unique_results: Vec<DistributedResult> = all_results
            .into_iter()
            .filter(|r| seen.insert(r.doc_id))
            .take(k)
            .enumerate()
            .map(|(idx, r)| DistributedResult {
                doc_id: r.doc_id,
                shard_id: r.shard_id,
                score: r.score,
                rank: idx + 1,
            })
            .collect();

        let stats = QueryStats {
            shards_queried: available_shards.len(),
            shards_responded,
            total_candidates,
            unique_results: unique_results.len(),
            query_time_ms: start.elapsed().as_millis() as u64,
        };

        Ok((unique_results, stats))
    }

    /// Get total queries executed
    pub fn total_queries(&self) -> u64 {
        self.total_queries.load(Ordering::Relaxed)
    }

    /// Get configuration
    pub fn config(&self) -> &DistributedConfig {
        &self.config
    }
}

/// Sharding strategy for partitioning data
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShardingStrategy {
    /// Round-robin assignment
    #[default]
    RoundRobin,
    /// Hash-based assignment (consistent hashing)
    HashBased,
    /// Range-based assignment (by document ID)
    RangeBased,
}

/// Shard assignment helper
pub struct ShardAssigner {
    strategy: ShardingStrategy,
    num_shards: u32,
    counter: AtomicU64,
}

impl ShardAssigner {
    /// Create a new shard assigner
    pub fn new(strategy: ShardingStrategy, num_shards: u32) -> Self {
        Self {
            strategy,
            num_shards,
            counter: AtomicU64::new(0),
        }
    }

    /// Assign a document to a shard
    pub fn assign(&self, doc_id: usize) -> ShardId {
        match self.strategy {
            ShardingStrategy::RoundRobin => {
                let idx = self.counter.fetch_add(1, Ordering::Relaxed);
                ShardId((idx as u32) % self.num_shards)
            }
            ShardingStrategy::HashBased => {
                // Simple hash function
                let hash = doc_id.wrapping_mul(0x9e3779b9) >> 16;
                ShardId((hash as u32) % self.num_shards)
            }
            ShardingStrategy::RangeBased => {
                // Assume doc_ids are sequential
                let range_size = usize::MAX / self.num_shards as usize;
                ShardId((doc_id / range_size).min(self.num_shards as usize - 1) as u32)
            }
        }
    }
}

/// Builder for creating a distributed search cluster
pub struct DistributedSearchBuilder {
    config: DistributedConfig,
    num_shards: u32,
    sharding_strategy: ShardingStrategy,
    shards: Vec<Shard>,
    assigner: ShardAssigner,
}

impl DistributedSearchBuilder {
    /// Create a new builder
    ///
    /// # Panics
    ///
    /// Panics if `num_shards` is 0.
    pub fn new(num_shards: u32) -> Self {
        assert!(num_shards > 0, "num_shards must be greater than 0");
        let shards = (0..num_shards).map(|i| Shard::new(ShardId(i))).collect();
        let assigner = ShardAssigner::new(ShardingStrategy::default(), num_shards);
        Self {
            config: DistributedConfig::default(),
            num_shards,
            sharding_strategy: ShardingStrategy::default(),
            shards,
            assigner,
        }
    }

    /// Set the search configuration
    pub fn with_config(mut self, config: DistributedConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the sharding strategy
    pub fn with_strategy(mut self, strategy: ShardingStrategy) -> Self {
        self.sharding_strategy = strategy;
        // Recreate assigner with new strategy, preserving counter state
        self.assigner = ShardAssigner::new(strategy, self.num_shards);
        self
    }

    /// Add a document to the cluster (assigns to appropriate shard)
    pub fn add_document(&mut self, doc_id: usize, vec: SparseVec) {
        // Use the stored assigner to maintain RoundRobin counter state
        let shard_id = self.assigner.assign(doc_id);
        if let Some(shard) = self.shards.get_mut(shard_id.0 as usize) {
            shard.add(doc_id, vec);
        }
    }

    /// Build the distributed search cluster
    pub fn build(mut self) -> DistributedSearch {
        // Finalize all shards
        for shard in &mut self.shards {
            shard.finalize();
        }

        let mut search = DistributedSearch::new(self.config);
        for shard in self.shards {
            search.add_shard(shard);
        }
        search
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddenator_vsa::ReversibleVSAConfig;

    fn create_test_vec(data: &[u8]) -> SparseVec {
        let config = ReversibleVSAConfig::default();
        SparseVec::encode_data(data, &config, None)
    }

    #[test]
    fn test_shard_id() {
        let id = ShardId(42);
        assert_eq!(id.0, 42);
        assert_eq!(ShardId::from_u32(42), id);
    }

    #[test]
    fn test_shard_basic() {
        let mut shard = Shard::new(ShardId(0));
        assert_eq!(shard.doc_count(), 0);
        assert!(shard.is_available());

        shard.add(1, create_test_vec(b"document one"));
        shard.add(2, create_test_vec(b"document two"));
        shard.finalize();

        assert_eq!(shard.doc_count(), 2);
    }

    #[test]
    fn test_shard_query() {
        let mut shard = Shard::new(ShardId(0));
        shard.add(1, create_test_vec(b"hello world"));
        shard.add(2, create_test_vec(b"goodbye world"));
        shard.finalize();

        let query = create_test_vec(b"hello");
        let config = SearchConfig::default();
        let results = shard.query(&query, &config, 2);

        assert!(!results.is_empty());
        assert_eq!(results[0].shard_id, ShardId(0));
        assert_eq!(shard.query_count(), 1);
    }

    #[test]
    fn test_shard_status() {
        let mut shard = Shard::new(ShardId(0));
        assert!(shard.is_available());

        shard.status = ShardStatus::Degraded;
        assert!(shard.is_available());

        shard.status = ShardStatus::Offline;
        assert!(!shard.is_available());
    }

    #[test]
    fn test_distributed_search_basic() {
        let mut shard0 = Shard::new(ShardId(0));
        let mut shard1 = Shard::new(ShardId(1));

        shard0.add(1, create_test_vec(b"document one"));
        shard0.add(2, create_test_vec(b"document two"));
        shard0.finalize();

        shard1.add(3, create_test_vec(b"document three"));
        shard1.add(4, create_test_vec(b"document four"));
        shard1.finalize();

        let mut search = DistributedSearch::new(DistributedConfig::default());
        search.add_shard(shard0);
        search.add_shard(shard1);

        assert_eq!(search.shard_count(), 2);
        assert_eq!(search.available_shard_count(), 2);

        let query = create_test_vec(b"document");
        let (results, stats) = search.query(&query, 5).unwrap();

        assert!(!results.is_empty());
        assert!(results.len() <= 5);
        assert_eq!(stats.shards_queried, 2);
        assert_eq!(results[0].rank, 1);
    }

    #[test]
    fn test_distributed_search_deduplication() {
        // Create two shards with overlapping content
        let mut shard0 = Shard::new(ShardId(0));
        let mut shard1 = Shard::new(ShardId(1));

        let vec = create_test_vec(b"shared document");
        shard0.add(1, vec.clone());
        shard0.finalize();

        shard1.add(1, vec); // Same doc_id
        shard1.finalize();

        let mut search = DistributedSearch::new(DistributedConfig::default());
        search.add_shard(shard0);
        search.add_shard(shard1);

        let query = create_test_vec(b"shared");
        let (results, _) = search.query(&query, 10).unwrap();

        // Should have only one result (deduplicated)
        let count_doc1 = results.iter().filter(|r| r.doc_id == 1).count();
        assert_eq!(count_doc1, 1);
    }

    #[test]
    fn test_distributed_search_insufficient_shards() {
        let search = DistributedSearch::new(DistributedConfig {
            min_shards: 3,
            ..Default::default()
        });

        let query = create_test_vec(b"test");
        let result = search.query(&query, 10);

        assert!(matches!(
            result,
            Err(DistributedError::InsufficientShards { .. })
        ));
    }

    #[test]
    fn test_shard_assigner_round_robin() {
        let assigner = ShardAssigner::new(ShardingStrategy::RoundRobin, 3);

        assert_eq!(assigner.assign(0), ShardId(0));
        assert_eq!(assigner.assign(1), ShardId(1));
        assert_eq!(assigner.assign(2), ShardId(2));
        assert_eq!(assigner.assign(3), ShardId(0)); // Wraps around
    }

    #[test]
    fn test_shard_assigner_hash_based() {
        let assigner = ShardAssigner::new(ShardingStrategy::HashBased, 4);

        // Same doc_id should always get same shard
        let shard1 = assigner.assign(100);
        let shard2 = assigner.assign(100);
        assert_eq!(shard1, shard2);

        // Different doc_ids likely get different shards (but not guaranteed)
        let _shard_a = assigner.assign(1);
        let _shard_b = assigner.assign(1000);
    }

    #[test]
    fn test_distributed_builder() {
        let mut builder = DistributedSearchBuilder::new(3)
            .with_strategy(ShardingStrategy::RoundRobin)
            .with_config(DistributedConfig::default());

        builder.add_document(1, create_test_vec(b"doc1"));
        builder.add_document(2, create_test_vec(b"doc2"));
        builder.add_document(3, create_test_vec(b"doc3"));
        builder.add_document(4, create_test_vec(b"doc4"));

        let search = builder.build();
        assert_eq!(search.shard_count(), 3);

        let query = create_test_vec(b"doc");
        let (results, _) = search.query(&query, 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_query_stats() {
        let mut builder = DistributedSearchBuilder::new(2);

        for i in 0..10 {
            let data = format!("document {}", i);
            builder.add_document(i, create_test_vec(data.as_bytes()));
        }

        let search = builder.build();
        let query = create_test_vec(b"document");
        let (_, stats) = search.query(&query, 5).unwrap();

        assert_eq!(stats.shards_queried, 2);
        assert_eq!(stats.shards_responded, 2);
        assert!(stats.total_candidates > 0);
        assert!(stats.unique_results <= 5);
    }

    #[test]
    fn test_parallel_distributed_search() {
        let config = DistributedConfig {
            parallel_shards: true,
            ..Default::default()
        };

        let mut builder = DistributedSearchBuilder::new(4).with_config(config);

        for i in 0..100 {
            let data = format!("document {} content for testing", i);
            builder.add_document(i, create_test_vec(data.as_bytes()));
        }

        let search = builder.build();
        let query = create_test_vec(b"document content");
        let (results, stats) = search.query(&query, 20).unwrap();

        assert!(!results.is_empty());
        assert_eq!(stats.shards_queried, 4);
    }

    #[test]
    fn test_all_shards_failed() {
        // Create search with shards that all become unavailable
        let mut shard0 = Shard::new(ShardId(0));
        shard0.add(1, create_test_vec(b"document one"));
        shard0.finalize();
        shard0.set_status(ShardStatus::Offline);

        let mut shard1 = Shard::new(ShardId(1));
        shard1.add(2, create_test_vec(b"document two"));
        shard1.finalize();
        shard1.set_status(ShardStatus::Offline);

        let mut search = DistributedSearch::new(DistributedConfig {
            min_shards: 1, // Require at least 1 shard
            ..Default::default()
        });
        search.add_shard(shard0);
        search.add_shard(shard1);

        let query = create_test_vec(b"document");
        let result = search.query(&query, 10);

        // Should fail because no shards are available (all offline)
        assert!(matches!(
            result,
            Err(DistributedError::InsufficientShards { available: 0, .. })
        ));
    }

    #[test]
    fn test_shard_assigner_range_based() {
        let assigner = ShardAssigner::new(ShardingStrategy::RangeBased, 4);

        // Documents with low IDs should go to early shards
        let shard_low = assigner.assign(0);
        let shard_mid = assigner.assign(usize::MAX / 2);
        let shard_high = assigner.assign(usize::MAX - 1);

        // Low IDs should go to shard 0
        assert_eq!(shard_low, ShardId(0));
        // Very high IDs should go to the last shard
        assert_eq!(shard_high, ShardId(3));
        // Middle IDs should go to middle shards
        assert!(shard_mid.0 >= 1 && shard_mid.0 <= 2);
    }

    #[test]
    fn test_round_robin_distribution() {
        // Verify that RoundRobin actually distributes across shards
        let mut builder =
            DistributedSearchBuilder::new(3).with_strategy(ShardingStrategy::RoundRobin);

        // Add 9 documents (should be 3 per shard with RoundRobin)
        for i in 0..9 {
            builder.add_document(i, create_test_vec(format!("doc{}", i).as_bytes()));
        }

        // Check that documents are distributed across shards
        let shard0_count = builder.shards[0].doc_count();
        let shard1_count = builder.shards[1].doc_count();
        let shard2_count = builder.shards[2].doc_count();

        // Each shard should have exactly 3 documents with perfect round-robin
        assert_eq!(shard0_count, 3, "Shard 0 should have 3 documents");
        assert_eq!(shard1_count, 3, "Shard 1 should have 3 documents");
        assert_eq!(shard2_count, 3, "Shard 2 should have 3 documents");
    }

    #[test]
    fn test_query_k_zero() {
        let mut builder = DistributedSearchBuilder::new(2);
        builder.add_document(1, create_test_vec(b"test document"));
        let search = builder.build();

        let query = create_test_vec(b"test");
        let (results, stats) = search.query(&query, 0).unwrap();

        // Should return empty results without querying any shards
        assert!(results.is_empty());
        assert_eq!(stats.shards_queried, 0);
    }

    #[test]
    #[should_panic(expected = "num_shards must be greater than 0")]
    fn test_builder_zero_shards_panics() {
        let _ = DistributedSearchBuilder::new(0);
    }

    #[test]
    fn test_shard_set_status() {
        let mut shard = Shard::new(ShardId(0));
        assert_eq!(shard.status, ShardStatus::Healthy);

        shard.set_status(ShardStatus::Degraded);
        assert_eq!(shard.status, ShardStatus::Degraded);
        assert!(shard.is_available());

        shard.set_status(ShardStatus::Rebuilding);
        assert_eq!(shard.status, ShardStatus::Rebuilding);
        assert!(!shard.is_available());
    }
}
