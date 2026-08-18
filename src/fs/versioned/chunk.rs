//! Versioned chunk data structure
//!
//! A chunk is a fixed-size block of file data encoded as a SparseVec. This module
//! implements versioning for chunks to enable concurrent access with optimistic locking.

use crate::SparseVec;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// A versioned chunk with metadata
///
/// Chunks are immutable once created - the SparseVec is wrapped in Arc for
/// zero-copy sharing across threads. Each update creates a new VersionedChunk
/// with an incremented version number.
#[derive(Clone)]
pub struct VersionedChunk {
    /// The actual VSA-encoded chunk data (immutable, shared)
    pub vector: Arc<SparseVec>,

    /// Version number of this chunk (local to the chunk)
    pub version: u64,

    /// When this chunk was first created
    pub created_at: Instant,

    /// When this chunk was last modified
    pub modified_at: Instant,

    /// Reference count - how many files reference this chunk
    /// Used for garbage collection and deduplication tracking
    pub ref_count: Arc<AtomicU32>,

    /// Size of the original data in bytes (before VSA encoding)
    pub original_size: usize,

    /// Content hash for deduplication (first 8 bytes of SHA256)
    pub content_hash: [u8; 8],
}

impl VersionedChunk {
    /// Create a new versioned chunk
    pub fn new(vector: SparseVec, original_size: usize, content_hash: [u8; 8]) -> Self {
        let now = Instant::now();
        Self {
            vector: Arc::new(vector),
            version: 0,
            created_at: now,
            modified_at: now,
            ref_count: Arc::new(AtomicU32::new(1)),
            original_size,
            content_hash,
        }
    }

    /// Create a new version of this chunk with updated data
    pub fn update(&self, new_vector: SparseVec, new_hash: [u8; 8]) -> Self {
        Self {
            vector: Arc::new(new_vector),
            version: self.version + 1,
            created_at: self.created_at,
            modified_at: Instant::now(),
            ref_count: Arc::new(AtomicU32::new(1)),
            original_size: self.original_size,
            content_hash: new_hash,
        }
    }

    /// Increment the reference count
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Decrement the reference count and return the new value
    pub fn dec_ref(&self) -> u32 {
        self.ref_count
            .fetch_sub(1, Ordering::AcqRel)
            .saturating_sub(1)
    }

    /// Get the current reference count
    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Acquire)
    }

    /// Check if this chunk can be garbage collected
    pub fn is_unreferenced(&self) -> bool {
        self.ref_count() == 0
    }

    /// Get the age of this chunk
    pub fn age(&self) -> std::time::Duration {
        Instant::now().duration_since(self.created_at)
    }

    /// Get time since last modification
    pub fn time_since_modification(&self) -> std::time::Duration {
        Instant::now().duration_since(self.modified_at)
    }
}

impl fmt::Debug for VersionedChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VersionedChunk")
            .field("version", &self.version)
            .field("original_size", &self.original_size)
            .field("ref_count", &self.ref_count())
            .field("content_hash", &format!("{:02x?}", self.content_hash))
            .field("age_ms", &self.age().as_millis())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let vec = SparseVec::new();
        let chunk = VersionedChunk::new(vec, 4096, [1, 2, 3, 4, 5, 6, 7, 8]);

        assert_eq!(chunk.version, 0);
        assert_eq!(chunk.original_size, 4096);
        assert_eq!(chunk.ref_count(), 1);
        assert!(!chunk.is_unreferenced());
    }

    #[test]
    fn test_chunk_update() {
        let vec1 = SparseVec::new();
        let chunk1 = VersionedChunk::new(vec1, 4096, [1, 2, 3, 4, 5, 6, 7, 8]);

        let vec2 = SparseVec::new();
        let chunk2 = chunk1.update(vec2, [9, 10, 11, 12, 13, 14, 15, 16]);

        assert_eq!(chunk2.version, 1);
        assert_eq!(chunk2.created_at, chunk1.created_at);
        assert!(chunk2.modified_at >= chunk1.modified_at);
    }

    #[test]
    fn test_reference_counting() {
        let vec = SparseVec::new();
        let chunk = VersionedChunk::new(vec, 4096, [0; 8]);

        assert_eq!(chunk.ref_count(), 1);

        chunk.inc_ref();
        assert_eq!(chunk.ref_count(), 2);

        chunk.inc_ref();
        assert_eq!(chunk.ref_count(), 3);

        let count = chunk.dec_ref();
        assert_eq!(count, 2);

        let count = chunk.dec_ref();
        assert_eq!(count, 1);

        let count = chunk.dec_ref();
        assert_eq!(count, 0);
        assert!(chunk.is_unreferenced());
    }
}
