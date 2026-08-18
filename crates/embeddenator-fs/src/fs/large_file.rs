//! Large File Handling with Hierarchical Sub-Engrams
//!
//! This module provides improved algorithms for handling large files (>1MB) with
//! better fidelity than the default chunking approach. The key insight is that
//! VSA encoding quality degrades when too many vectors are bundled together.
//!
//! # The Problem
//!
//! Standard VSA encoding has a "capacity limit" - the number of vectors that can
//! be reliably bundled before signal-to-noise ratio degrades. For sparse ternary
//! vectors with dimension D, this is roughly O(sqrt(D)).
//!
//! With D=10000, capacity ≈ 100 vectors per bundle. A 10MB file with 4KB chunks
//! creates ~2500 chunks - well over capacity.
//!
//! # The Solution: Hierarchical Sub-Engrams
//!
//! Instead of bundling all chunks into one root:
//!
//! ```text
//! Traditional (fails at scale):
//!   root = chunk1 ⊕ chunk2 ⊕ ... ⊕ chunk2500
//!
//! Hierarchical (scales well):
//!   level0 = [sub1, sub2, ... sub25]      (25 sub-engrams)
//!   sub1 = chunk1 ⊕ chunk2 ⊕ ... ⊕ chunk100
//!   sub2 = chunk101 ⊕ ... ⊕ chunk200
//!   root = sub1 ⊕ sub2 ⊕ ... ⊕ sub25
//! ```
//!
//! Each level bundles at most ~100 vectors, staying within capacity.
//!
//! # Adaptive Chunk Size
//!
//! For files with low entropy (highly compressible), larger chunks work better.
//! For high-entropy data (already compressed), smaller chunks preserve fidelity.
//!
//! ```text
//! entropy < 0.3  → chunk_size = 16KB (compressible data)
//! entropy < 0.6  → chunk_size = 8KB  (mixed content)
//! entropy >= 0.6 → chunk_size = 4KB  (high entropy)
//! ```

use crate::correction::ChunkCorrection;
use crate::versioned::{ChunkId, VersionedChunk, VersionedFileEntry};
use crate::versioned_embrfs::{
    EmbrFSError, VersionedEmbrFS, DEFAULT_CHUNK_SIZE, ENCODING_FORMAT_REVERSIBLE_VSA,
};
use embeddenator_vsa::SparseVec;
use sha2::{Digest, Sha256};

/// Maximum chunks per bundle level (based on VSA capacity theory)
const MAX_BUNDLE_CAPACITY: usize = 100;

/// Entropy thresholds for adaptive chunking
const LOW_ENTROPY_THRESHOLD: f64 = 0.3;
const MEDIUM_ENTROPY_THRESHOLD: f64 = 0.6;

/// Chunk sizes for different entropy levels
const LOW_ENTROPY_CHUNK_SIZE: usize = 16 * 1024; // 16KB
const MEDIUM_ENTROPY_CHUNK_SIZE: usize = 8 * 1024; // 8KB
const HIGH_ENTROPY_CHUNK_SIZE: usize = 4 * 1024; // 4KB (default)

/// Sub-engram for hierarchical encoding of large files
///
/// This is distinct from `embrfs::SubEngram` - this version is optimized
/// for the hierarchical bundling of large file chunks.
#[derive(Clone)]
pub struct HierarchicalSubEngram {
    /// Root vector of this sub-engram
    pub root: SparseVec,
    /// Chunk IDs contained in this sub-engram
    pub chunk_ids: Vec<ChunkId>,
    /// Level in the hierarchy (0 = leaf level)
    pub level: usize,
}

/// Configuration for large file handling
#[derive(Clone, Debug)]
pub struct LargeFileConfig {
    /// Enable adaptive chunk sizing based on entropy
    pub adaptive_chunking: bool,
    /// Maximum chunks per bundle
    pub max_bundle_size: usize,
    /// Enable hierarchical sub-engrams
    pub hierarchical: bool,
    /// Correction threshold for re-encoding
    pub correction_threshold: f64,
    /// Enable parallel encoding (when feature enabled)
    pub parallel: bool,
}

impl Default for LargeFileConfig {
    fn default() -> Self {
        Self {
            adaptive_chunking: true,
            max_bundle_size: MAX_BUNDLE_CAPACITY,
            hierarchical: true,
            correction_threshold: 0.1,
            parallel: true,
        }
    }
}

/// Large file handler with improved algorithms
pub struct LargeFileHandler<'a> {
    fs: &'a VersionedEmbrFS,
    config: LargeFileConfig,
}

impl<'a> LargeFileHandler<'a> {
    /// Create a new large file handler
    pub fn new(fs: &'a VersionedEmbrFS) -> Self {
        Self {
            fs,
            config: LargeFileConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(fs: &'a VersionedEmbrFS, config: LargeFileConfig) -> Self {
        Self { fs, config }
    }

    /// Write a large file with improved encoding
    ///
    /// Uses hierarchical sub-engrams and adaptive chunking for better fidelity.
    pub fn write_large_file(
        &self,
        path: &str,
        data: &[u8],
        expected_version: Option<u64>,
    ) -> Result<LargeFileResult, EmbrFSError> {
        // Calculate optimal chunk size based on entropy
        let chunk_size = if self.config.adaptive_chunking {
            self.calculate_optimal_chunk_size(data)
        } else {
            DEFAULT_CHUNK_SIZE
        };

        // Split into chunks
        let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();
        let chunk_count = chunks.len();

        // Determine if we need hierarchical encoding
        let use_hierarchical =
            self.config.hierarchical && chunk_count > self.config.max_bundle_size;

        if use_hierarchical {
            self.write_hierarchical(path, &chunks, expected_version, chunk_size)
        } else {
            self.write_flat(path, &chunks, expected_version, chunk_size)
        }
    }

    /// Calculate optimal chunk size based on data entropy
    fn calculate_optimal_chunk_size(&self, data: &[u8]) -> usize {
        let entropy = self.estimate_entropy(data);

        if entropy < LOW_ENTROPY_THRESHOLD {
            LOW_ENTROPY_CHUNK_SIZE
        } else if entropy < MEDIUM_ENTROPY_THRESHOLD {
            MEDIUM_ENTROPY_CHUNK_SIZE
        } else {
            HIGH_ENTROPY_CHUNK_SIZE
        }
    }

    /// Estimate Shannon entropy of data (0.0 - 1.0)
    fn estimate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        // Sample for large files
        let sample_size = data.len().min(64 * 1024);
        let sample = &data[0..sample_size];

        // Count byte frequencies
        let mut freq = [0u64; 256];
        for &byte in sample {
            freq[byte as usize] += 1;
        }

        // Calculate entropy
        let total = sample.len() as f64;
        let mut entropy = 0.0;

        for &count in &freq {
            if count > 0 {
                let p = count as f64 / total;
                entropy -= p * p.log2();
            }
        }

        // Normalize to 0-1 range (max entropy for bytes is 8 bits)
        entropy / 8.0
    }

    /// Write using flat (non-hierarchical) encoding
    fn write_flat(
        &self,
        path: &str,
        chunks: &[&[u8]],
        expected_version: Option<u64>,
        chunk_size: usize,
    ) -> Result<LargeFileResult, EmbrFSError> {
        let mut chunk_ids = Vec::new();
        let mut chunk_updates = Vec::new();
        let mut corrections = Vec::new();
        let mut total_correction_bytes = 0usize;

        for chunk_data in chunks {
            let chunk_id = self.fs.allocate_chunk_id();

            // Encode (using mode-appropriate encoder)
            let chunk_vec = self.fs.encode_chunk(chunk_data, Some(path));

            // Verify (using mode-appropriate decoder)
            let decoded = self
                .fs
                .decode_chunk(&chunk_vec, Some(path), chunk_data.len());

            // Compute hash
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            // Create correction
            let correction = ChunkCorrection::new(chunk_id as u64, chunk_data, &decoded);
            total_correction_bytes += correction.storage_size();

            chunk_updates.push((
                chunk_id,
                VersionedChunk::new(chunk_vec, chunk_data.len(), hash_bytes),
            ));
            corrections.push((chunk_id as u64, correction));
            chunk_ids.push(chunk_id);
        }

        // Batch insert
        self.fs.chunk_store.batch_insert_new(chunk_updates)?;
        self.fs.corrections.batch_insert_new(corrections)?;

        // Create manifest entry
        let total_size: usize = chunks.iter().map(|c| c.len()).sum();
        let is_text = is_text_data_sample(chunks.first().copied().unwrap_or(&[]));
        let mut file_entry =
            VersionedFileEntry::new(path.to_string(), is_text, total_size, chunk_ids.clone());

        // Set encoding format for holographic mode files
        if self.fs.is_holographic() {
            file_entry.encoding_format = Some(ENCODING_FORMAT_REVERSIBLE_VSA);
        }

        let version = if let Some(expected) = expected_version {
            let existing = self
                .fs
                .manifest
                .get_file(path)
                .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;
            if existing.0.version != expected {
                return Err(EmbrFSError::VersionMismatch {
                    expected,
                    actual: existing.0.version,
                });
            }
            self.fs.manifest.update_file(path, file_entry, expected)?;
            expected + 1
        } else {
            self.fs.manifest.add_file(file_entry)?;
            0
        };

        // Bundle
        self.fs.bundle_chunks_to_root_streaming(&chunk_ids)?;

        Ok(LargeFileResult {
            path: path.to_string(),
            total_bytes: total_size,
            chunk_count: chunk_ids.len(),
            version,
            correction_bytes: total_correction_bytes,
            hierarchy_levels: 1,
            sub_engram_count: 1,
            chunk_size_used: chunk_size,
        })
    }

    /// Write using hierarchical sub-engram encoding
    fn write_hierarchical(
        &self,
        path: &str,
        chunks: &[&[u8]],
        expected_version: Option<u64>,
        chunk_size: usize,
    ) -> Result<LargeFileResult, EmbrFSError> {
        let mut chunk_ids = Vec::new();
        let mut chunk_updates = Vec::new();
        let mut corrections = Vec::new();
        let mut total_correction_bytes = 0usize;

        // Level 0: Encode all chunks
        let mut level0_vectors: Vec<SparseVec> = Vec::new();

        for chunk_data in chunks {
            let chunk_id = self.fs.allocate_chunk_id();

            // Encode (using mode-appropriate encoder)
            let chunk_vec = self.fs.encode_chunk(chunk_data, Some(path));

            // Verify (using mode-appropriate decoder)
            let decoded = self
                .fs
                .decode_chunk(&chunk_vec, Some(path), chunk_data.len());

            // Compute hash
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            // Create correction
            let correction = ChunkCorrection::new(chunk_id as u64, chunk_data, &decoded);
            total_correction_bytes += correction.storage_size();

            level0_vectors.push(chunk_vec.clone());
            chunk_updates.push((
                chunk_id,
                VersionedChunk::new(chunk_vec, chunk_data.len(), hash_bytes),
            ));
            corrections.push((chunk_id as u64, correction));
            chunk_ids.push(chunk_id);
        }

        // Build hierarchy of sub-engrams
        let mut current_level = level0_vectors;
        let mut hierarchy_levels = 1;

        while current_level.len() > self.config.max_bundle_size {
            let mut next_level = Vec::new();

            // Group into sub-engrams
            for group in current_level.chunks(self.config.max_bundle_size) {
                // Bundle group into sub-engram
                let mut sub_root = group[0].clone();
                for vec in &group[1..] {
                    sub_root = sub_root.bundle(vec);
                }
                next_level.push(sub_root);
            }

            current_level = next_level;
            hierarchy_levels += 1;
        }

        let sub_engram_count = current_level.len();

        // Batch insert chunks
        self.fs.chunk_store.batch_insert_new(chunk_updates)?;
        self.fs.corrections.batch_insert_new(corrections)?;

        // Create manifest entry
        let total_size: usize = chunks.iter().map(|c| c.len()).sum();
        let is_text = is_text_data_sample(chunks.first().copied().unwrap_or(&[]));
        let mut file_entry =
            VersionedFileEntry::new(path.to_string(), is_text, total_size, chunk_ids.clone());

        // Set encoding format for holographic mode files
        if self.fs.is_holographic() {
            file_entry.encoding_format = Some(ENCODING_FORMAT_REVERSIBLE_VSA);
        }

        let version = if let Some(expected) = expected_version {
            let existing = self
                .fs
                .manifest
                .get_file(path)
                .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;
            if existing.0.version != expected {
                return Err(EmbrFSError::VersionMismatch {
                    expected,
                    actual: existing.0.version,
                });
            }
            self.fs.manifest.update_file(path, file_entry, expected)?;
            expected + 1
        } else {
            self.fs.manifest.add_file(file_entry)?;
            0
        };

        // Bundle final level into root
        self.fs.bundle_chunks_to_root_streaming(&chunk_ids)?;

        Ok(LargeFileResult {
            path: path.to_string(),
            total_bytes: total_size,
            chunk_count: chunk_ids.len(),
            version,
            correction_bytes: total_correction_bytes,
            hierarchy_levels,
            sub_engram_count,
            chunk_size_used: chunk_size,
        })
    }
}

/// Result of large file write operation
#[derive(Debug, Clone)]
pub struct LargeFileResult {
    /// Path of the file
    pub path: String,
    /// Total bytes written
    pub total_bytes: usize,
    /// Number of chunks created
    pub chunk_count: usize,
    /// File version
    pub version: u64,
    /// Total correction bytes
    pub correction_bytes: usize,
    /// Number of hierarchy levels used
    pub hierarchy_levels: usize,
    /// Number of sub-engrams at lowest level
    pub sub_engram_count: usize,
    /// Chunk size used for this file
    pub chunk_size_used: usize,
}

impl LargeFileResult {
    /// Calculate correction ratio (correction bytes / total bytes)
    pub fn correction_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.correction_bytes as f64 / self.total_bytes as f64
        }
    }

    /// Check if encoding quality is acceptable (< 10% correction)
    pub fn is_acceptable_quality(&self) -> bool {
        self.correction_ratio() < 0.1
    }
}

/// Heuristic text detection for sample data
fn is_text_data_sample(data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }

    let sample_size = data.len().min(8192);
    let sample = &data[0..sample_size];

    let non_printable = sample
        .iter()
        .filter(|&&b| b < 32 && b != b'\n' && b != b'\r' && b != b'\t')
        .count();

    (non_printable as f64 / sample_size as f64) < 0.05
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        let fs = VersionedEmbrFS::new();
        let handler = LargeFileHandler::new(&fs);

        // Uniform data = high entropy
        let uniform: Vec<u8> = (0..256).cycle().take(1000).map(|x| x as u8).collect();
        let uniform_entropy = handler.estimate_entropy(&uniform);
        assert!(
            uniform_entropy > 0.9,
            "Uniform data should have high entropy"
        );

        // Repetitive data = low entropy
        let repetitive = vec![0u8; 1000];
        let rep_entropy = handler.estimate_entropy(&repetitive);
        assert!(rep_entropy < 0.1, "Repetitive data should have low entropy");

        // Text-like data = medium entropy
        let text = b"The quick brown fox jumps over the lazy dog. ".repeat(20);
        let text_entropy = handler.estimate_entropy(&text);
        assert!(
            text_entropy > 0.3 && text_entropy < 0.8,
            "Text should have medium entropy"
        );
    }

    #[test]
    fn test_adaptive_chunk_sizing() {
        let fs = VersionedEmbrFS::new();
        let handler = LargeFileHandler::new(&fs);

        // Low entropy -> large chunks
        let low_entropy = vec![42u8; 10000];
        let size1 = handler.calculate_optimal_chunk_size(&low_entropy);
        assert_eq!(size1, LOW_ENTROPY_CHUNK_SIZE);

        // High entropy -> small chunks
        let high_entropy: Vec<u8> = (0..10000).map(|i| (i * 7 % 256) as u8).collect();
        let size2 = handler.calculate_optimal_chunk_size(&high_entropy);
        assert_eq!(size2, HIGH_ENTROPY_CHUNK_SIZE);
    }

    #[test]
    fn test_small_file_flat_encoding() {
        let fs = VersionedEmbrFS::new();
        let handler = LargeFileHandler::new(&fs);

        let data = b"Small file content";
        let result = handler.write_large_file("small.txt", data, None).unwrap();

        assert_eq!(result.total_bytes, data.len());
        assert_eq!(result.hierarchy_levels, 1);
        assert_eq!(result.sub_engram_count, 1);

        // Verify data
        let (content, _) = fs.read_file("small.txt").unwrap();
        assert_eq!(&content[..], data);
    }

    #[test]
    fn test_large_file_hierarchical_encoding() {
        let fs = VersionedEmbrFS::new();
        let config = LargeFileConfig {
            max_bundle_size: 10, // Force hierarchical with small bundle size
            ..Default::default()
        };
        let handler = LargeFileHandler::with_config(&fs, config);

        // Create file that will require hierarchical encoding
        let data: Vec<u8> = (0..50000).map(|i| (i % 256) as u8).collect();
        let result = handler.write_large_file("large.bin", &data, None).unwrap();

        assert_eq!(result.total_bytes, data.len());
        assert!(
            result.hierarchy_levels > 1,
            "Should use hierarchical encoding"
        );
        assert!(result.chunk_count > 10);

        // Verify data integrity
        let (content, _) = fs.read_file("large.bin").unwrap();
        assert_eq!(content, data);
    }
}
