//! Common types for versioned data structures

use std::fmt;

/// Unique identifier for a chunk
pub type ChunkId = usize;

/// Chunk location within a file with byte offset tracking for range queries
///
/// This structure enables O(log n) byte-offset lookups by storing the start
/// position of each chunk within the original file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkOffset {
    /// The chunk identifier
    pub chunk_id: ChunkId,
    /// Start byte offset of this chunk within the file
    pub byte_offset: usize,
    /// Size of the original (decoded) data in this chunk
    pub byte_length: usize,
}

impl ChunkOffset {
    /// Create a new chunk offset entry
    pub fn new(chunk_id: ChunkId, byte_offset: usize, byte_length: usize) -> Self {
        Self {
            chunk_id,
            byte_offset,
            byte_length,
        }
    }

    /// Get the end byte offset (exclusive) of this chunk
    #[inline]
    pub fn end_offset(&self) -> usize {
        self.byte_offset + self.byte_length
    }

    /// Check if this chunk contains the given byte offset
    #[inline]
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.byte_offset && offset < self.end_offset()
    }

    /// Calculate the offset within this chunk for a given file offset
    ///
    /// Returns None if the offset is outside this chunk's range
    pub fn offset_within(&self, file_offset: usize) -> Option<usize> {
        if self.contains(file_offset) {
            Some(file_offset - self.byte_offset)
        } else {
            None
        }
    }
}

/// A range of data within a chunk for partial reads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkRange {
    /// The chunk identifier
    pub chunk_id: ChunkId,
    /// Start offset within the chunk (not the file)
    pub start_within_chunk: usize,
    /// Number of bytes to read from this chunk
    pub length: usize,
}

impl ChunkRange {
    /// Create a new chunk range
    pub fn new(chunk_id: ChunkId, start_within_chunk: usize, length: usize) -> Self {
        Self {
            chunk_id,
            start_within_chunk,
            length,
        }
    }

    /// Check if this range covers the entire chunk
    pub fn is_full_chunk(&self, chunk_size: usize) -> bool {
        self.start_within_chunk == 0 && self.length == chunk_size
    }
}

/// Error returned when optimistic locking detects a version mismatch
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMismatch {
    pub expected: u64,
    pub actual: u64,
}

impl fmt::Display for VersionMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Version mismatch: expected {}, found {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for VersionMismatch {}

/// Result type for operations that may encounter version mismatches
pub type VersionedResult<T> = Result<T, VersionMismatch>;

/// Trait for versioned data structures
pub trait Versioned {
    /// Get the current version
    fn version(&self) -> u64;

    /// Increment the version atomically
    fn increment_version(&self) -> u64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_mismatch_display() {
        let err = VersionMismatch {
            expected: 5,
            actual: 7,
        };
        assert_eq!(err.to_string(), "Version mismatch: expected 5, found 7");
    }
}
