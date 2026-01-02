use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MetricsSnapshot {
    pub poison_recoveries_total: u64,

    pub poison_path_inodes: u64,
    pub poison_inodes: u64,
    pub poison_inode_paths: u64,
    pub poison_directories: u64,
    pub poison_file_cache: u64,

    pub sub_cache_hits: u64,
    pub sub_cache_misses: u64,
    pub sub_cache_evictions: u64,

    pub index_cache_hits: u64,
    pub index_cache_misses: u64,
    pub index_cache_evictions: u64,

    pub retrieval_query_calls: u64,
    pub retrieval_query_ns_total: u64,
    pub retrieval_query_ns_max: u64,

    pub rerank_calls: u64,
    pub rerank_ns_total: u64,
    pub rerank_ns_max: u64,

    pub hier_query_calls: u64,
    pub hier_query_ns_total: u64,
    pub hier_query_ns_max: u64,
}

pub struct Metrics {
    poison_recoveries_total: AtomicU64,

    poison_path_inodes: AtomicU64,
    poison_inodes: AtomicU64,
    poison_inode_paths: AtomicU64,
    poison_directories: AtomicU64,
    poison_file_cache: AtomicU64,

    sub_cache_hits: AtomicU64,
    sub_cache_misses: AtomicU64,
    sub_cache_evictions: AtomicU64,

    index_cache_hits: AtomicU64,
    index_cache_misses: AtomicU64,
    index_cache_evictions: AtomicU64,

    retrieval_query_calls: AtomicU64,
    retrieval_query_ns_total: AtomicU64,
    retrieval_query_ns_max: AtomicU64,

    rerank_calls: AtomicU64,
    rerank_ns_total: AtomicU64,
    rerank_ns_max: AtomicU64,

    hier_query_calls: AtomicU64,
    hier_query_ns_total: AtomicU64,
    hier_query_ns_max: AtomicU64,
}

impl Metrics {
    pub const fn new() -> Self {
        Self {
            poison_recoveries_total: AtomicU64::new(0),

            poison_path_inodes: AtomicU64::new(0),
            poison_inodes: AtomicU64::new(0),
            poison_inode_paths: AtomicU64::new(0),
            poison_directories: AtomicU64::new(0),
            poison_file_cache: AtomicU64::new(0),

            sub_cache_hits: AtomicU64::new(0),
            sub_cache_misses: AtomicU64::new(0),
            sub_cache_evictions: AtomicU64::new(0),

            index_cache_hits: AtomicU64::new(0),
            index_cache_misses: AtomicU64::new(0),
            index_cache_evictions: AtomicU64::new(0),

            retrieval_query_calls: AtomicU64::new(0),
            retrieval_query_ns_total: AtomicU64::new(0),
            retrieval_query_ns_max: AtomicU64::new(0),

            rerank_calls: AtomicU64::new(0),
            rerank_ns_total: AtomicU64::new(0),
            rerank_ns_max: AtomicU64::new(0),

            hier_query_calls: AtomicU64::new(0),
            hier_query_ns_total: AtomicU64::new(0),
            hier_query_ns_max: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            poison_recoveries_total: self.poison_recoveries_total.load(Ordering::Relaxed),

            poison_path_inodes: self.poison_path_inodes.load(Ordering::Relaxed),
            poison_inodes: self.poison_inodes.load(Ordering::Relaxed),
            poison_inode_paths: self.poison_inode_paths.load(Ordering::Relaxed),
            poison_directories: self.poison_directories.load(Ordering::Relaxed),
            poison_file_cache: self.poison_file_cache.load(Ordering::Relaxed),

            sub_cache_hits: self.sub_cache_hits.load(Ordering::Relaxed),
            sub_cache_misses: self.sub_cache_misses.load(Ordering::Relaxed),
            sub_cache_evictions: self.sub_cache_evictions.load(Ordering::Relaxed),

            index_cache_hits: self.index_cache_hits.load(Ordering::Relaxed),
            index_cache_misses: self.index_cache_misses.load(Ordering::Relaxed),
            index_cache_evictions: self.index_cache_evictions.load(Ordering::Relaxed),

            retrieval_query_calls: self.retrieval_query_calls.load(Ordering::Relaxed),
            retrieval_query_ns_total: self.retrieval_query_ns_total.load(Ordering::Relaxed),
            retrieval_query_ns_max: self.retrieval_query_ns_max.load(Ordering::Relaxed),

            rerank_calls: self.rerank_calls.load(Ordering::Relaxed),
            rerank_ns_total: self.rerank_ns_total.load(Ordering::Relaxed),
            rerank_ns_max: self.rerank_ns_max.load(Ordering::Relaxed),

            hier_query_calls: self.hier_query_calls.load(Ordering::Relaxed),
            hier_query_ns_total: self.hier_query_ns_total.load(Ordering::Relaxed),
            hier_query_ns_max: self.hier_query_ns_max.load(Ordering::Relaxed),
        }
    }

    pub fn inc_poison_path_inodes(&self) {
        #[cfg(feature = "metrics")]
        {
            self.poison_recoveries_total.fetch_add(1, Ordering::Relaxed);
            self.poison_path_inodes.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_poison_inodes(&self) {
        #[cfg(feature = "metrics")]
        {
            self.poison_recoveries_total.fetch_add(1, Ordering::Relaxed);
            self.poison_inodes.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_poison_inode_paths(&self) {
        #[cfg(feature = "metrics")]
        {
            self.poison_recoveries_total.fetch_add(1, Ordering::Relaxed);
            self.poison_inode_paths.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_poison_directories(&self) {
        #[cfg(feature = "metrics")]
        {
            self.poison_recoveries_total.fetch_add(1, Ordering::Relaxed);
            self.poison_directories.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_poison_file_cache(&self) {
        #[cfg(feature = "metrics")]
        {
            self.poison_recoveries_total.fetch_add(1, Ordering::Relaxed);
            self.poison_file_cache.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_sub_cache_hit(&self) {
        #[cfg(feature = "metrics")]
        {
            self.sub_cache_hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_sub_cache_miss(&self) {
        #[cfg(feature = "metrics")]
        {
            self.sub_cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_sub_cache_eviction(&self) {
        #[cfg(feature = "metrics")]
        {
            self.sub_cache_evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_index_cache_hit(&self) {
        #[cfg(feature = "metrics")]
        {
            self.index_cache_hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_index_cache_miss(&self) {
        #[cfg(feature = "metrics")]
        {
            self.index_cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_index_cache_eviction(&self) {
        #[cfg(feature = "metrics")]
        {
            self.index_cache_evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_retrieval_query(&self, _dur: Duration) {
        #[cfg(feature = "metrics")]
        {
            record_duration(
                &self.retrieval_query_calls,
                &self.retrieval_query_ns_total,
                &self.retrieval_query_ns_max,
                _dur,
            );
        }
    }

    pub fn record_rerank(&self, _dur: Duration) {
        #[cfg(feature = "metrics")]
        {
            record_duration(
                &self.rerank_calls,
                &self.rerank_ns_total,
                &self.rerank_ns_max,
                _dur,
            );
        }
    }

    pub fn record_hier_query(&self, _dur: Duration) {
        #[cfg(feature = "metrics")]
        {
            record_duration(
                &self.hier_query_calls,
                &self.hier_query_ns_total,
                &self.hier_query_ns_max,
                _dur,
            );
        }
    }
}

#[cfg(feature = "metrics")]
fn record_duration(calls: &AtomicU64, total_ns: &AtomicU64, max_ns: &AtomicU64, dur: Duration) {
    let ns = dur.as_nanos().min(u128::from(u64::MAX)) as u64;
    calls.fetch_add(1, Ordering::Relaxed);
    total_ns.fetch_add(ns, Ordering::Relaxed);

    let mut cur = max_ns.load(Ordering::Relaxed);
    while ns > cur {
        match max_ns.compare_exchange_weak(cur, ns, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(next) => cur = next,
        }
    }
}

static METRICS: Metrics = Metrics::new();

pub fn metrics() -> &'static Metrics {
    &METRICS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_snapshot_delta_behaves_under_feature_gate() {
        let before = metrics().snapshot();

        metrics().inc_poison_inodes();
        metrics().inc_sub_cache_hit();
        metrics().record_retrieval_query(Duration::from_millis(2));

        let after = metrics().snapshot();

        #[cfg(feature = "metrics")]
        {
            assert!(after.poison_inodes >= before.poison_inodes + 1);
            assert!(after.poison_recoveries_total >= before.poison_recoveries_total + 1);
            assert!(after.sub_cache_hits >= before.sub_cache_hits + 1);
            assert!(after.retrieval_query_calls >= before.retrieval_query_calls + 1);
            assert!(after.retrieval_query_ns_total >= before.retrieval_query_ns_total);
            assert!(after.retrieval_query_ns_max >= before.retrieval_query_ns_max);
        }

        #[cfg(not(feature = "metrics"))]
        {
            assert_eq!(after, before);
        }
    }
}
