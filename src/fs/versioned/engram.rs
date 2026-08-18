//! Top-level versioned engram with CAS-based root updates
//!
//! This module provides the main VersionedEngram struct that coordinates
//! all versioned components and provides high-level read/write operations.

use super::chunk_store::VersionedChunkStore;
use super::corrections::VersionedCorrectionStore;
use super::manifest::VersionedManifest;
use super::transaction::{Transaction, TransactionManager, TransactionStatus};
use super::types::{VersionMismatch, VersionedResult};
use crate::SparseVec;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// A fully versioned engram with optimistic locking
///
/// This is the top-level structure that coordinates all versioned components.
/// It provides high-level read/write operations with ACID-like properties.
pub struct VersionedEngram {
    /// Root VSA vector (wrapped in Arc for immutable sharing)
    root: Arc<RwLock<Arc<SparseVec>>>,

    /// Version of the root vector
    root_version: Arc<AtomicU64>,

    /// Versioned chunk store (NOT the VSA codebook - that's in embeddenator-vsa)
    pub chunk_store: VersionedChunkStore,

    /// Versioned corrections
    pub corrections: VersionedCorrectionStore,

    /// Versioned manifest
    pub manifest: VersionedManifest,

    /// Transaction manager
    tx_manager: TransactionManager,

    /// Transaction log
    tx_log: Arc<RwLock<Vec<Transaction>>>,

    /// Global engram version (coordinates all components)
    global_version: Arc<AtomicU64>,
}

impl VersionedEngram {
    /// Create a new versioned engram with default dimensionality
    ///
    /// # Arguments
    /// * `dimensionality` - Reserved for future use. The actual dimensionality is
    ///   determined by `SparseVec::new()` which uses the system's configured default.
    ///   This parameter is preserved in the API for future VSA codebook integration
    ///   when the dimensionality becomes configurable.
    pub fn new(_dimensionality: usize) -> Self {
        // SparseVec::new() doesn't take dimensionality parameter
        Self::with_root(Arc::new(SparseVec::new()))
    }

    /// Create a versioned engram with an existing root vector
    pub fn with_root(root: Arc<SparseVec>) -> Self {
        Self {
            root: Arc::new(RwLock::new(root)),
            root_version: Arc::new(AtomicU64::new(0)),
            chunk_store: VersionedChunkStore::new(),
            corrections: VersionedCorrectionStore::new(),
            manifest: VersionedManifest::new(),
            tx_manager: TransactionManager::new(),
            tx_log: Arc::new(RwLock::new(Vec::new())),
            global_version: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the current global version
    pub fn version(&self) -> u64 {
        self.global_version.load(Ordering::Acquire)
    }

    /// Get the current root version
    pub fn root_version(&self) -> u64 {
        self.root_version.load(Ordering::Acquire)
    }

    /// Get a reference to the root vector
    pub fn root(&self) -> Arc<SparseVec> {
        let root_lock = self.root.read().unwrap();
        Arc::clone(&*root_lock)
    }

    /// Update the root vector with Compare-And-Swap (CAS)
    ///
    /// This is the core operation for optimistic locking on the root.
    /// It attempts to update the root only if the current version matches
    /// the expected version.
    pub fn update_root(
        &self,
        new_root: Arc<SparseVec>,
        expected_version: u64,
    ) -> VersionedResult<u64> {
        let mut root_lock = self.root.write().unwrap();

        // Check version
        let current_version = self.root_version.load(Ordering::Acquire);
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Update root
        *root_lock = new_root;

        // Increment version
        let new_version = self.root_version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Bundle a chunk into the root with automatic retry
    ///
    /// This operation superimposes a new chunk vector into the root using the
    /// VSA bundle operation (XOR for binary vectors). The bundling is performed
    /// with optimistic locking and automatic retry on version conflicts.
    ///
    /// # Algorithm
    /// 1. Read current root and its version
    /// 2. Create new root by bundling: `new_root = current_root ⊕ chunk_vec`
    /// 3. Attempt CAS update with optimistic locking
    /// 4. Retry with exponential backoff if version conflict detected
    ///
    /// # Arguments
    /// * `chunk_vec` - The chunk vector to bundle into the root
    ///
    /// # Returns
    /// * `Ok(new_version)` - The new root version after successful bundle
    /// * `Err(e)` - Error after max retries exceeded or other failure
    pub fn bundle_chunk(&self, chunk_vec: &SparseVec) -> Result<u64, String> {
        const MAX_RETRIES: usize = 10;

        for attempt in 0..MAX_RETRIES {
            // Read current root
            let current_root = self.root();
            let current_version = self.root_version();

            // Create new root by bundling the chunk with current root
            // Bundle operation: new_root = current_root ⊕ chunk_vec
            let new_root = Arc::new(current_root.bundle(chunk_vec));

            // Try to update with CAS
            match self.update_root(new_root, current_version) {
                Ok(new_version) => return Ok(new_version),
                Err(_) if attempt < MAX_RETRIES - 1 => {
                    // Retry with exponential backoff
                    std::thread::sleep(std::time::Duration::from_micros(1 << attempt));
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to bundle chunk after {} attempts: {}",
                        MAX_RETRIES, e
                    ))
                }
            }
        }

        Err("Max retries exceeded".to_string())
    }

    /// Begin a new transaction
    pub fn begin_transaction(&self) -> Transaction {
        self.tx_manager.begin(self.version())
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, mut tx: Transaction) -> Result<(), String> {
        // Verify engram version hasn't changed too much
        let current_version = self.version();
        if current_version > tx.engram_version + 10 {
            // Allow some version drift, but not too much
            return Err("Engram version drifted too far, transaction may conflict".to_string());
        }

        // Mark as committed
        tx.commit();

        // Add to log
        let mut log = self.tx_log.write().unwrap();
        log.push(tx);

        // Increment global version
        self.global_version.fetch_add(1, Ordering::AcqRel);

        Ok(())
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, mut tx: Transaction) {
        tx.abort();

        // Add to log
        let mut log = self.tx_log.write().unwrap();
        log.push(tx);
    }

    /// Get transaction statistics
    pub fn transaction_stats(&self) -> TransactionStats {
        let log = self.tx_log.read().unwrap();

        let total = log.len();
        let committed = log
            .iter()
            .filter(|tx| tx.status == TransactionStatus::Committed)
            .count();
        let aborted = log
            .iter()
            .filter(|tx| tx.status == TransactionStatus::Aborted)
            .count();

        TransactionStats {
            total_transactions: total,
            committed_transactions: committed,
            aborted_transactions: aborted,
            success_rate: if total > 0 {
                committed as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Get comprehensive engram statistics
    pub fn stats(&self) -> EngramStats {
        EngramStats {
            global_version: self.version(),
            root_version: self.root_version(),
            chunk_store: self.chunk_store.stats(),
            corrections: self.corrections.stats(),
            manifest: self.manifest.stats(),
            transactions: self.transaction_stats(),
        }
    }
}

impl Default for VersionedEngram {
    fn default() -> Self {
        Self::new(10000) // Default VSA dimensionality
    }
}

impl Clone for VersionedEngram {
    fn clone(&self) -> Self {
        Self {
            root: Arc::clone(&self.root),
            root_version: Arc::clone(&self.root_version),
            chunk_store: self.chunk_store.clone(),
            corrections: self.corrections.clone(),
            manifest: self.manifest.clone(),
            tx_manager: TransactionManager::new(), // New manager for clone
            tx_log: Arc::new(RwLock::new(Vec::new())), // Fresh log
            global_version: Arc::clone(&self.global_version),
        }
    }
}

/// Transaction statistics
#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub total_transactions: usize,
    pub committed_transactions: usize,
    pub aborted_transactions: usize,
    pub success_rate: f64,
}

/// Comprehensive engram statistics
#[derive(Debug, Clone)]
pub struct EngramStats {
    pub global_version: u64,
    pub root_version: u64,
    pub chunk_store: super::chunk_store::CodebookStats,
    pub corrections: super::corrections::CorrectionStats,
    pub manifest: super::manifest::ManifestStats,
    pub transactions: TransactionStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engram_creation() {
        let engram = VersionedEngram::new(10000);
        assert_eq!(engram.version(), 0);
        assert_eq!(engram.root_version(), 0);
    }

    #[test]
    fn test_root_update() {
        let engram = VersionedEngram::new(10000);
        let new_root = Arc::new(SparseVec::new());

        let version = engram.update_root(new_root, 0).unwrap();
        assert_eq!(version, 1);
        assert_eq!(engram.root_version(), 1);
    }

    #[test]
    fn test_root_update_version_mismatch() {
        let engram = VersionedEngram::new(10000);
        let new_root = Arc::new(SparseVec::new());

        // Update once
        engram.update_root(Arc::clone(&new_root), 0).unwrap();

        // Try to update with old version
        let result = engram.update_root(new_root, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_lifecycle() {
        let engram = VersionedEngram::new(10000);

        let tx = engram.begin_transaction();
        assert_eq!(tx.status, TransactionStatus::Pending);

        engram.commit_transaction(tx).unwrap();

        let stats = engram.transaction_stats();
        assert_eq!(stats.total_transactions, 1);
        assert_eq!(stats.committed_transactions, 1);
        assert_eq!(stats.success_rate, 1.0);
    }
}
