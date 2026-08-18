//! Versioned chunk store with optimistic locking
//!
//! The chunk store is a HashMap that stores chunk ID → VersionedChunk mappings.
//! This is NOT the VSA codebook (base vectors) - that's in embeddenator-vsa.
//! This stores the encoded chunks that make up files in the engram.
//!
//! It supports concurrent reads and writes with optimistic locking for conflict detection.

use super::chunk::VersionedChunk;
use super::types::{ChunkId, VersionMismatch, VersionedResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// A versioned chunk store with optimistic locking
///
/// The chunk store maintains mappings from chunk IDs to versioned chunks. Multiple readers
/// can access the store concurrently without blocking. Writers check the global
/// version before committing changes to detect conflicts.
///
/// Note: This is distinct from the VSA codebook (base vectors used for encoding).
/// The VSA codebook is in embeddenator-vsa and is typically static.
/// This chunk store maps file chunk IDs to their VSA-encoded representations.
///
/// ## Concurrency Model
///
/// ```text
/// Reader 1 ─┐
///           ├─→ RwLock::read() ──→ Success (shared access)
/// Reader 2 ─┘
///
/// Writer 1 ──→ RwLock::write() ──→ Check version ──→ Update ──→ Increment version
///                                        ↓
///                                   If changed → VersionMismatch
/// ```
pub struct VersionedChunkStore {
    /// Map of chunk ID to versioned chunk (protected by RwLock)
    chunks: Arc<RwLock<HashMap<ChunkId, Arc<VersionedChunk>>>>,

    /// Global version number for the entire chunk store
    /// Incremented on every write operation
    global_version: Arc<AtomicU64>,

    /// Content hash index for deduplication
    /// Maps content hash → chunk ID
    hash_index: Arc<RwLock<HashMap<[u8; 8], ChunkId>>>,
}

impl VersionedChunkStore {
    /// Create a new empty versioned codebook
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            global_version: Arc::new(AtomicU64::new(0)),
            hash_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the current global version
    pub fn version(&self) -> u64 {
        self.global_version.load(Ordering::Acquire)
    }

    /// Get a chunk by ID (non-blocking read)
    ///
    /// Returns the chunk and the global version at the time of read.
    /// The version can be used later for optimistic locking on writes.
    pub fn get(&self, chunk_id: ChunkId) -> Option<(Arc<VersionedChunk>, u64)> {
        let chunks = self.chunks.read().unwrap();
        let version = self.version();
        chunks
            .get(&chunk_id)
            .map(|chunk| (Arc::clone(chunk), version))
    }

    /// Check if a chunk with the given content hash already exists (deduplication)
    pub fn find_by_hash(&self, content_hash: &[u8; 8]) -> Option<(ChunkId, Arc<VersionedChunk>)> {
        let hash_index = self.hash_index.read().unwrap();
        let chunks = self.chunks.read().unwrap();

        hash_index.get(content_hash).and_then(|&chunk_id| {
            chunks
                .get(&chunk_id)
                .map(|chunk| (chunk_id, Arc::clone(chunk)))
        })
    }

    /// Insert or update a single chunk with optimistic locking
    ///
    /// Returns the new global version on success, or VersionMismatch if
    /// the expected version doesn't match the current version.
    pub fn insert(
        &self,
        chunk_id: ChunkId,
        chunk: VersionedChunk,
        expected_version: u64,
    ) -> VersionedResult<u64> {
        let mut chunks = self.chunks.write().unwrap();
        let mut hash_index = self.hash_index.write().unwrap();

        // Check version
        let current_version = self.version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Update hash index
        hash_index.insert(chunk.content_hash, chunk_id);

        // Insert chunk
        chunks.insert(chunk_id, Arc::new(chunk));

        // Increment version
        let new_version = self.global_version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Batch insert multiple chunks atomically
    ///
    /// Either all chunks are inserted or none are (on version mismatch).
    /// This is more efficient than multiple individual inserts.
    pub fn batch_insert(
        &self,
        updates: Vec<(ChunkId, VersionedChunk)>,
        expected_version: u64,
    ) -> VersionedResult<u64> {
        let mut chunks = self.chunks.write().unwrap();
        let mut hash_index = self.hash_index.write().unwrap();

        // Check version
        let current_version = self.version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Insert all chunks
        for (chunk_id, chunk) in updates {
            hash_index.insert(chunk.content_hash, chunk_id);
            chunks.insert(chunk_id, Arc::new(chunk));
        }

        // Increment version once for the entire batch
        let new_version = self.global_version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Batch insert NEW chunks without version checking
    ///
    /// This is used for inserting brand new chunks (e.g., when creating a new file)
    /// where chunk IDs are guaranteed unique and monotonically increasing, so
    /// concurrent inserts cannot conflict.
    ///
    /// This enables lock-free concurrent file creation.
    pub fn batch_insert_new(
        &self,
        updates: Vec<(ChunkId, VersionedChunk)>,
    ) -> VersionedResult<u64> {
        let mut chunks = self.chunks.write().unwrap();
        let mut hash_index = self.hash_index.write().unwrap();

        // No version check - chunk IDs are unique
        for (chunk_id, chunk) in updates {
            hash_index.insert(chunk.content_hash, chunk_id);
            chunks.insert(chunk_id, Arc::new(chunk));
        }

        // Increment version once for the entire batch
        let new_version = self.global_version.fetch_add(1, Ordering::AcqRel) + 1;
        Ok(new_version)
    }

    /// Remove a chunk by ID
    ///
    /// Returns the removed chunk on success, or VersionMismatch if the version changed.
    pub fn remove(
        &self,
        chunk_id: ChunkId,
        expected_version: u64,
    ) -> VersionedResult<Option<Arc<VersionedChunk>>> {
        let mut chunks = self.chunks.write().unwrap();
        let mut hash_index = self.hash_index.write().unwrap();

        // Check version
        let current_version = self.version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Remove chunk and update hash index
        let removed = chunks.remove(&chunk_id);
        if let Some(ref chunk) = removed {
            hash_index.remove(&chunk.content_hash);
        }

        // Increment version
        self.global_version.fetch_add(1, Ordering::AcqRel);
        Ok(removed)
    }

    /// Get the number of chunks in the codebook
    pub fn len(&self) -> usize {
        self.chunks.read().unwrap().len()
    }

    /// Check if the codebook is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.read().unwrap().is_empty()
    }

    /// Get all chunk IDs (snapshot)
    pub fn chunk_ids(&self) -> Vec<ChunkId> {
        self.chunks.read().unwrap().keys().copied().collect()
    }

    /// Iterate over all chunks (snapshot)
    ///
    /// Returns a snapshot of (ChunkId, Arc<VersionedChunk>) pairs.
    /// Safe to iterate without holding locks.
    pub fn iter(&self) -> Vec<(ChunkId, Arc<VersionedChunk>)> {
        self.chunks
            .read()
            .unwrap()
            .iter()
            .map(|(&id, chunk)| (id, Arc::clone(chunk)))
            .collect()
    }

    /// Garbage collect unreferenced chunks
    ///
    /// Removes chunks with ref_count == 0. Returns the number of chunks removed.
    pub fn gc(&self, expected_version: u64) -> VersionedResult<usize> {
        let mut chunks = self.chunks.write().unwrap();
        let mut hash_index = self.hash_index.write().unwrap();

        // Check version
        let current_version = self.version();
        if current_version != expected_version {
            return Err(VersionMismatch {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Find unreferenced chunks
        let to_remove: Vec<ChunkId> = chunks
            .iter()
            .filter(|(_, chunk)| chunk.is_unreferenced())
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();

        // Remove them
        for chunk_id in to_remove {
            if let Some(chunk) = chunks.remove(&chunk_id) {
                hash_index.remove(&chunk.content_hash);
            }
        }

        // Increment version if we removed anything
        if count > 0 {
            self.global_version.fetch_add(1, Ordering::AcqRel);
        }

        Ok(count)
    }

    /// Get statistics about the codebook
    pub fn stats(&self) -> CodebookStats {
        let chunks = self.chunks.read().unwrap();

        let total_chunks = chunks.len();
        let mut total_refs = 0u64;
        let mut unreferenced = 0;
        let mut total_size = 0usize;

        for chunk in chunks.values() {
            let refs = chunk.ref_count();
            total_refs += refs as u64;
            if refs == 0 {
                unreferenced += 1;
            }
            total_size += chunk.original_size;
        }

        CodebookStats {
            total_chunks,
            unreferenced_chunks: unreferenced,
            total_references: total_refs,
            avg_references: if total_chunks > 0 {
                total_refs as f64 / total_chunks as f64
            } else {
                0.0
            },
            total_size_bytes: total_size,
            version: self.version(),
        }
    }
}

impl Default for VersionedChunkStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for VersionedChunkStore {
    fn clone(&self) -> Self {
        Self {
            chunks: Arc::clone(&self.chunks),
            global_version: Arc::clone(&self.global_version),
            hash_index: Arc::clone(&self.hash_index),
        }
    }
}

/// Statistics about a codebook
#[derive(Debug, Clone)]
pub struct CodebookStats {
    pub total_chunks: usize,
    pub unreferenced_chunks: usize,
    pub total_references: u64,
    pub avg_references: f64,
    pub total_size_bytes: usize,
    pub version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SparseVec;

    fn make_test_chunk(id: usize) -> VersionedChunk {
        let vec = SparseVec::new();
        let hash = [(id & 0xFF) as u8; 8];
        VersionedChunk::new(vec, 4096, hash)
    }

    #[test]
    fn test_codebook_creation() {
        let codebook = VersionedChunkStore::new();
        assert_eq!(codebook.version(), 0);
        assert!(codebook.is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let codebook = VersionedChunkStore::new();
        let chunk = make_test_chunk(1);

        let version = codebook.insert(1, chunk, 0).unwrap();
        assert_eq!(version, 1);

        let (retrieved, ver) = codebook.get(1).unwrap();
        assert_eq!(retrieved.version, 0);
        assert_eq!(ver, 1);
    }

    #[test]
    fn test_version_mismatch() {
        let codebook = VersionedChunkStore::new();
        let chunk1 = make_test_chunk(1);
        let chunk2 = make_test_chunk(2);

        // Insert first chunk
        codebook.insert(1, chunk1, 0).unwrap();

        // Try to insert with old version
        let result = codebook.insert(2, chunk2, 0);
        assert!(result.is_err());

        match result {
            Err(VersionMismatch { expected, actual }) => {
                assert_eq!(expected, 0);
                assert_eq!(actual, 1);
            }
            _ => panic!("Expected VersionMismatch"),
        }
    }

    #[test]
    fn test_batch_insert() {
        let codebook = VersionedChunkStore::new();

        let updates = vec![
            (1, make_test_chunk(1)),
            (2, make_test_chunk(2)),
            (3, make_test_chunk(3)),
        ];

        let version = codebook.batch_insert(updates, 0).unwrap();
        assert_eq!(version, 1);
        assert_eq!(codebook.len(), 3);
    }

    #[test]
    fn test_deduplication() {
        let codebook = VersionedChunkStore::new();

        let chunk1 = make_test_chunk(1);
        let hash = chunk1.content_hash;

        codebook.insert(1, chunk1, 0).unwrap();

        // Try to find by hash
        let found = codebook.find_by_hash(&hash);
        assert!(found.is_some());

        let (id, _) = found.unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn test_garbage_collection() {
        let codebook = VersionedChunkStore::new();

        let chunk = make_test_chunk(1);
        codebook.insert(1, chunk.clone(), 0).unwrap();

        // Decrement ref_count to 0
        chunk.dec_ref();

        assert_eq!(codebook.len(), 1);

        // Run GC
        let removed = codebook.gc(1).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(codebook.len(), 0);
    }

    #[test]
    fn test_stats() {
        let codebook = VersionedChunkStore::new();

        for i in 0..10 {
            codebook.insert(i, make_test_chunk(i), i as u64).unwrap();
        }

        let stats = codebook.stats();
        assert_eq!(stats.total_chunks, 10);
        assert_eq!(stats.total_size_bytes, 10 * 4096);
        assert_eq!(stats.version, 10);
    }
}
