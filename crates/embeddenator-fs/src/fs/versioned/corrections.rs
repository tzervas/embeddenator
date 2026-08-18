//! Versioned correction store with optimistic locking
//!
//! The correction store maintains the bit-perfect reconstruction corrections
//! for VSA-encoded chunks with concurrent access support.

use super::types::{VersionMismatch, VersionedResult};
use crate::fs::correction::ChunkCorrection;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Statistics for the correction store
#[derive(Debug, Clone)]
pub struct CorrectionStats {
    pub total_chunks: u64,
    pub perfect_chunks: u64,
    pub corrected_chunks: u64,
    pub total_correction_bytes: u64,
    pub total_original_bytes: u64,
}

impl CorrectionStats {
    /// Calculate the correction ratio (overhead percentage)
    pub fn correction_ratio(&self) -> f64 {
        if self.total_original_bytes == 0 {
            return 0.0;
        }
        self.total_correction_bytes as f64 / self.total_original_bytes as f64
    }

    /// Calculate the perfect ratio (percentage of perfect chunks)
    pub fn perfect_ratio(&self) -> f64 {
        if self.total_chunks == 0 {
            return 0.0;
        }
        self.perfect_chunks as f64 / self.total_chunks as f64
    }
}

/// A versioned correction store with optimistic locking
///
/// Stores corrections for VSA-encoded chunks to enable bit-perfect reconstruction.
/// Each correction is wrapped in Arc for zero-copy sharing.
pub struct VersionedCorrectionStore {
    /// Map of chunk ID to correction (protected by RwLock)
    corrections: Arc<RwLock<HashMap<u64, Arc<ChunkCorrection>>>>,

    /// Statistics (protected by RwLock)
    stats: Arc<RwLock<CorrectionStats>>,

    /// Global version number
    version: Arc<AtomicU64>,
}

impl VersionedCorrectionStore {
    /// Create a new empty correction store
    pub fn new() -> Self {
        Self {
            corrections: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CorrectionStats {
                total_chunks: 0,
                perfect_chunks: 0,
                corrected_chunks: 0,
                total_correction_bytes: 0,
                total_original_bytes: 0,
            })),
            version: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the current version
    pub fn current_version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Get a correction by chunk ID
    pub fn get(&self, chunk_id: u64) -> Option<(Arc<ChunkCorrection>, u64)> {
        let corrections = self.corrections.read().unwrap();
        let version = self.current_version();
        corrections
            .get(&chunk_id)
            .map(|corr| (Arc::clone(corr), version))
    }

    /// Add or update a correction
    pub fn update(
        &self,
        chunk_id: u64,
        correction: ChunkCorrection,
        expected_version: u64,
    ) -> VersionedResult<u64> {
        let mut corrections = self.corrections.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        // Check version
        let current_version = self.current_version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Update statistics
        let is_new = !corrections.contains_key(&chunk_id);
        if is_new {
            stats.total_chunks += 1;
            // Note: We'd need to access correction internals to update perfect/corrected counts
            // This is a simplified version
        }

        // Insert correction
        corrections.insert(chunk_id, Arc::new(correction));

        // Increment version
        let new_version = self.version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Batch update multiple corrections
    pub fn batch_update(
        &self,
        updates: Vec<(u64, ChunkCorrection)>,
        expected_version: u64,
    ) -> VersionedResult<u64> {
        let mut corrections = self.corrections.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        // Check version
        let current_version = self.current_version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Insert all corrections
        for (chunk_id, correction) in updates {
            let is_new = !corrections.contains_key(&chunk_id);
            if is_new {
                stats.total_chunks += 1;
            }
            corrections.insert(chunk_id, Arc::new(correction));
        }

        // Increment version
        let new_version = self.version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Batch insert NEW corrections without version checking
    ///
    /// This is used for inserting brand new corrections (e.g., when creating a new file)
    /// where chunk IDs are guaranteed unique and monotonically increasing, so
    /// concurrent inserts cannot conflict.
    pub fn batch_insert_new(&self, updates: Vec<(u64, ChunkCorrection)>) -> VersionedResult<u64> {
        let mut corrections = self.corrections.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        // No version check - chunk IDs are unique
        for (chunk_id, correction) in updates {
            corrections.insert(chunk_id, Arc::new(correction));
            stats.total_chunks += 1;
        }

        // Increment version
        let new_version = self.version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Remove a correction
    pub fn remove(
        &self,
        chunk_id: u64,
        expected_version: u64,
    ) -> VersionedResult<Option<Arc<ChunkCorrection>>> {
        let mut corrections = self.corrections.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        // Check version
        let current_version = self.current_version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Remove correction
        let removed = corrections.remove(&chunk_id);
        if removed.is_some() {
            stats.total_chunks = stats.total_chunks.saturating_sub(1);
        }

        // Increment version
        self.version.fetch_add(1, Ordering::AcqRel);
        Ok(removed)
    }

    /// Get the number of corrections
    pub fn len(&self) -> usize {
        self.corrections.read().unwrap().len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.corrections.read().unwrap().is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> CorrectionStats {
        self.stats.read().unwrap().clone()
    }

    /// Get all chunk IDs (snapshot)
    pub fn chunk_ids(&self) -> Vec<u64> {
        self.corrections.read().unwrap().keys().copied().collect()
    }
}

impl Default for VersionedCorrectionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for VersionedCorrectionStore {
    fn clone(&self) -> Self {
        Self {
            corrections: Arc::clone(&self.corrections),
            stats: Arc::clone(&self.stats),
            version: Arc::clone(&self.version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correction_store_creation() {
        let store = VersionedCorrectionStore::new();
        assert_eq!(store.current_version(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_stats() {
        let store = VersionedCorrectionStore::new();
        let stats = store.stats();
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.correction_ratio(), 0.0);
        assert_eq!(stats.perfect_ratio(), 0.0);
    }
}
