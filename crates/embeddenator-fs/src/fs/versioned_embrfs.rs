//! Mutable EmbrFS implementation using versioned structures
//!
//! This module provides a mutable, concurrent-safe version of EmbrFS that supports
//! read-write operations with optimistic locking. Unlike the original EmbrFS which is
//! immutable by design, VersionedEmbrFS allows in-place updates while maintaining:
//!
//! - Bit-perfect reconstruction guarantees
//! - Concurrent read access without blocking
//! - Optimistic locking for writes with conflict detection
//! - VSA-native operations on compressed state
//!
//! ## Architecture
//!
//! ```text
//! VersionedEmbrFS
//!     ↓
//! VersionedEngram (coordinatesthree versioned components)
//!     ├── VersionedChunkStore (chunk_id → SparseVec)
//!     ├── VersionedManifest (file metadata)
//!     └── VersionedCorrectionStore (bit-perfect adjustments)
//! ```
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use embeddenator_fs::VersionedEmbrFS;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create new mutable filesystem
//! let fs = VersionedEmbrFS::new();
//!
//! // Write a file
//! let data = b"Hello, EmbrFS!";
//! fs.write_file("hello.txt", data, None)?;
//!
//! // Read it back
//! let (content, version) = fs.read_file("hello.txt")?;
//! assert_eq!(&content, data);
//!
//! // Update the file
//! let updated = b"Hello, Mutable EmbrFS!";
//! fs.write_file("hello.txt", updated, Some(version))?;
//!
//! // Concurrent operations work with optimistic locking
//! # Ok(())
//! # }
//! ```

use crate::versioned::{
    VersionedChunk, VersionedChunkStore, VersionedCorrectionStore, VersionedFileEntry,
    VersionedManifest,
};
use crate::ReversibleVSAConfig;
use embeddenator_io::{
    unwrap_auto, wrap_or_legacy, CompressionCodec, CompressionProfiler, PayloadKind,
};
use embeddenator_vsa::{Codebook, ProjectionResult, ReversibleVSAEncoder, SparseVec, DIM};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

pub use crate::versioned::types::{ChunkId, VersionMismatch, VersionedResult};
pub use crate::versioned::Operation;

/// Default chunk size for file encoding (4KB)
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

/// Holographic encoding format versions
/// Format 0: Legacy Codebook.project() - ~0% uncorrected accuracy
pub const ENCODING_FORMAT_LEGACY: u8 = 0;
/// Format 1: ReversibleVSAEncoder - ~94% uncorrected accuracy
pub const ENCODING_FORMAT_REVERSIBLE_VSA: u8 = 1;

/// Internal symbol chunk size for `ReversibleVSAEncoder`.
///
/// This 64-byte size was chosen empirically: benchmarks showed that it
/// provides a good trade-off between reconstruction accuracy and encoding
/// overhead, with larger sizes giving negligible accuracy improvements for
/// typical workloads while increasing memory and CPU cost. This is distinct
/// from `DEFAULT_CHUNK_SIZE` (4KB), which controls how file data is split
/// before being passed to the encoder.
pub const REVERSIBLE_CHUNK_SIZE: usize = 64;

/// Error types for VersionedEmbrFS operations
#[derive(Debug, Clone)]
pub enum EmbrFSError {
    /// File not found
    FileNotFound(String),
    /// Chunk not found
    ChunkNotFound(ChunkId),
    /// Version mismatch during optimistic locking
    VersionMismatch { expected: u64, actual: u64 },
    /// File already exists
    FileExists(String),
    /// Invalid operation
    InvalidOperation(String),
    /// IO error
    IoError(String),
}

impl std::fmt::Display for EmbrFSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbrFSError::FileNotFound(path) => write!(f, "File not found: {}", path),
            EmbrFSError::ChunkNotFound(id) => write!(f, "Chunk not found: {}", id),
            EmbrFSError::VersionMismatch { expected, actual } => {
                write!(f, "Version mismatch: expected {}, got {}", expected, actual)
            }
            EmbrFSError::FileExists(path) => write!(f, "File already exists: {}", path),
            EmbrFSError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            EmbrFSError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for EmbrFSError {}

impl From<VersionMismatch> for EmbrFSError {
    fn from(e: VersionMismatch) -> Self {
        EmbrFSError::VersionMismatch {
            expected: e.expected,
            actual: e.actual,
        }
    }
}

/// A mutable, versioned filesystem backed by holographic engrams
///
/// VersionedEmbrFS provides read-write operations with optimistic locking,
/// enabling concurrent access while maintaining consistency and bit-perfect
/// reconstruction guarantees.
pub struct VersionedEmbrFS {
    /// Root VSA vector (bundled superposition of all chunks)
    root: Arc<RwLock<Arc<SparseVec>>>,
    root_version: Arc<AtomicU64>,

    /// Versioned chunk store (chunk_id → encoded SparseVec)
    pub chunk_store: VersionedChunkStore,

    /// Versioned corrections for bit-perfect reconstruction
    pub corrections: VersionedCorrectionStore,

    /// Versioned manifest (file metadata)
    pub manifest: VersionedManifest,

    /// VSA configuration
    config: ReversibleVSAConfig,

    /// Compression profiler for path-based compression selection
    profiler: CompressionProfiler,

    /// Global filesystem version
    global_version: Arc<AtomicU64>,

    /// Next chunk ID to allocate
    next_chunk_id: Arc<AtomicU64>,

    /// Codebook for differential encoding (basis vectors)
    /// When holographic mode is enabled, data is projected onto these
    /// basis vectors and only residuals are stored in corrections.
    /// NOTE: Legacy mode only - new code should use reversible_encoder
    codebook: Arc<RwLock<Codebook>>,

    /// ReversibleVSAEncoder for true holographic encoding with ~94% accuracy
    /// This encoder uses position-aware binding to achieve reversible storage
    reversible_encoder: Arc<RwLock<ReversibleVSAEncoder>>,

    /// Whether holographic mode is enabled
    /// When true, uses ReversibleVSAEncoder for encoding (~94% uncorrected accuracy)
    /// When false, uses SparseVec::encode_data() (legacy mode)
    holographic_mode: bool,
}

impl VersionedEmbrFS {
    /// Create a new empty mutable filesystem (legacy mode)
    pub fn new() -> Self {
        Self::with_config(ReversibleVSAConfig::default())
    }

    /// Create a new mutable filesystem with holographic mode enabled
    ///
    /// In holographic mode, data is encoded using ReversibleVSAEncoder which
    /// achieves ~94% uncorrected accuracy through position-aware VSA binding.
    /// Only ~6% of bytes need correction, resulting in <10% correction overhead.
    pub fn new_holographic() -> Self {
        let mut fs = Self::with_config(ReversibleVSAConfig::default());
        fs.holographic_mode = true;
        // ReversibleVSAEncoder is already initialized in with_config_and_profiler
        fs
    }

    /// Create a new mutable filesystem with custom VSA configuration
    pub fn with_config(config: ReversibleVSAConfig) -> Self {
        Self::with_config_and_profiler(config, CompressionProfiler::default())
    }

    /// Create a new mutable filesystem with custom VSA configuration and compression profiler
    pub fn with_config_and_profiler(
        config: ReversibleVSAConfig,
        profiler: CompressionProfiler,
    ) -> Self {
        Self {
            root: Arc::new(RwLock::new(Arc::new(SparseVec::new()))),
            root_version: Arc::new(AtomicU64::new(0)),
            chunk_store: VersionedChunkStore::new(),
            corrections: VersionedCorrectionStore::new(),
            manifest: VersionedManifest::new(),
            config,
            profiler,
            global_version: Arc::new(AtomicU64::new(0)),
            next_chunk_id: Arc::new(AtomicU64::new(1)),
            codebook: Arc::new(RwLock::new(Codebook::new(DIM))),
            reversible_encoder: Arc::new(RwLock::new(ReversibleVSAEncoder::new())),
            holographic_mode: false,
        }
    }

    /// Enable holographic mode on an existing filesystem
    ///
    /// New writes will use ReversibleVSAEncoder for encoding with ~94% accuracy.
    pub fn enable_holographic_mode(&mut self) {
        self.holographic_mode = true;
        // ReversibleVSAEncoder is already initialized
    }

    /// Get a reference to the reversible encoder
    pub fn reversible_encoder(&self) -> &Arc<RwLock<ReversibleVSAEncoder>> {
        &self.reversible_encoder
    }

    /// Check if holographic mode is enabled
    pub fn is_holographic(&self) -> bool {
        self.holographic_mode
    }

    /// Get a reference to the codebook
    pub fn codebook(&self) -> &Arc<RwLock<Codebook>> {
        &self.codebook
    }

    /// Get the compression profiler
    pub fn profiler(&self) -> &CompressionProfiler {
        &self.profiler
    }

    /// Get the current global version
    pub fn version(&self) -> u64 {
        self.global_version.load(Ordering::Acquire)
    }

    /// Read a file's contents
    ///
    /// Returns the file data and the file entry version at read time.
    /// The version can be used for optimistic locking on subsequent writes.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::VersionedEmbrFS;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    /// let (data, version) = fs.read_file("example.txt")?;
    /// println!("Read {} bytes at version {}", data.len(), version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_file(&self, path: &str) -> Result<(Vec<u8>, u64), EmbrFSError> {
        // 1. Get file entry with version
        let (file_entry, _manifest_version) = self
            .manifest
            .get_file(path)
            .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;

        if file_entry.deleted {
            return Err(EmbrFSError::FileNotFound(path.to_string()));
        }

        // 2. Read and decode chunks
        let mut file_data = Vec::with_capacity(file_entry.size);

        // Check if file was encoded with reversible VSA
        let use_reversible = file_entry.encoding_format == Some(ENCODING_FORMAT_REVERSIBLE_VSA);

        for &chunk_id in &file_entry.chunks {
            // Get chunk from store
            let (chunk, _chunk_version) = self
                .chunk_store
                .get(chunk_id)
                .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;

            // Decode chunk using matching decoder
            let decoded = if use_reversible {
                self.reversible_encoder
                    .read()
                    .unwrap()
                    .decode(&chunk.vector, DEFAULT_CHUNK_SIZE)
            } else {
                chunk
                    .vector
                    .decode_data(&self.config, Some(&file_entry.path), DEFAULT_CHUNK_SIZE)
            };

            // Apply correction
            let corrected = self
                .corrections
                .get(chunk_id as u64)
                .map(|(corr, _)| corr.apply(&decoded))
                .unwrap_or(decoded);

            file_data.extend_from_slice(&corrected);
        }

        // Truncate to exact file size
        file_data.truncate(file_entry.size);

        // 3. Decompress if file was stored compressed
        let final_data = if let Some(codec) = file_entry.compression_codec {
            if codec != 0 {
                // Use unwrap_auto which auto-detects envelope format
                unwrap_auto(PayloadKind::EngramBincode, &file_data)
                    .map_err(|e| EmbrFSError::IoError(format!("Decompression failed: {}", e)))?
            } else {
                file_data
            }
        } else {
            file_data
        };

        Ok((final_data, file_entry.version))
    }

    /// Read a specific byte range from a file
    ///
    /// This method enables efficient partial file reads by only decoding the chunks
    /// needed to satisfy the requested byte range. When the file has a chunk offset
    /// index, chunks are located in O(log n) time.
    ///
    /// # Arguments
    /// * `path` - The file path within the engram
    /// * `offset` - The starting byte offset to read from
    /// * `length` - The number of bytes to read
    ///
    /// # Returns
    /// A tuple of (data, version) where data is the requested byte range.
    /// If the range extends beyond the file, only available bytes are returned.
    ///
    /// # Performance
    /// - With offset index: O(log n + k) where k = number of chunks in range
    /// - Without offset index: O(n) where n = total chunks (must scan all)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::VersionedEmbrFS;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    /// // Read bytes 1000-1999 from a file
    /// let (data, version) = fs.read_range("large_file.bin", 1000, 1000)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_range(
        &self,
        path: &str,
        offset: usize,
        length: usize,
    ) -> Result<(Vec<u8>, u64), EmbrFSError> {
        // 1. Get file entry
        let (file_entry, _manifest_version) = self
            .manifest
            .get_file(path)
            .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;

        if file_entry.deleted {
            return Err(EmbrFSError::FileNotFound(path.to_string()));
        }

        // Handle edge cases
        if offset >= file_entry.size || length == 0 {
            return Ok((Vec::new(), file_entry.version));
        }

        // Clamp the actual read length to file bounds
        let actual_length = length.min(file_entry.size - offset);

        // 2. Determine which chunks to read
        if file_entry.has_offset_index() {
            // Fast path: use offset index
            self.read_range_with_index(path, &file_entry, offset, actual_length)
        } else {
            // Slow path: must read sequentially and skip
            self.read_range_sequential(path, &file_entry, offset, actual_length)
        }
    }

    /// Read a byte range using the chunk offset index (fast path)
    fn read_range_with_index(
        &self,
        _path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        offset: usize,
        length: usize,
    ) -> Result<(Vec<u8>, u64), EmbrFSError> {
        let chunk_ranges = file_entry.chunks_for_range(offset, length);

        if chunk_ranges.is_empty() {
            return Ok((Vec::new(), file_entry.version));
        }

        let mut result = Vec::with_capacity(length);

        for range in chunk_ranges {
            // Get chunk from store
            let (chunk, _chunk_version) = self
                .chunk_store
                .get(range.chunk_id)
                .ok_or(EmbrFSError::ChunkNotFound(range.chunk_id))?;

            // Decode the chunk
            let decoded =
                chunk
                    .vector
                    .decode_data(&self.config, Some(&file_entry.path), chunk.original_size);

            // Apply correction
            let corrected = self
                .corrections
                .get(range.chunk_id as u64)
                .map(|(corr, _)| corr.apply(&decoded))
                .unwrap_or(decoded);

            // Extract the relevant portion
            let chunk_data = if range.start_within_chunk == 0 && range.length == corrected.len() {
                corrected
            } else {
                let end = (range.start_within_chunk + range.length).min(corrected.len());
                corrected[range.start_within_chunk..end].to_vec()
            };

            result.extend_from_slice(&chunk_data);
        }

        // Ensure we don't return more than requested
        result.truncate(length);

        Ok((result, file_entry.version))
    }

    /// Read a byte range without offset index (slow path)
    fn read_range_sequential(
        &self,
        _path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        offset: usize,
        length: usize,
    ) -> Result<(Vec<u8>, u64), EmbrFSError> {
        let mut result = Vec::with_capacity(length);
        let mut current_offset = 0usize;
        let end_offset = offset + length;

        for &chunk_id in &file_entry.chunks {
            // Get chunk from store
            let (chunk, _chunk_version) = self
                .chunk_store
                .get(chunk_id)
                .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;

            let chunk_size = chunk.original_size;
            let chunk_end = current_offset + chunk_size;

            // Skip chunks entirely before our range
            if chunk_end <= offset {
                current_offset = chunk_end;
                continue;
            }

            // Stop if we've passed our range
            if current_offset >= end_offset {
                break;
            }

            // Decode the chunk
            let decoded =
                chunk
                    .vector
                    .decode_data(&self.config, Some(&file_entry.path), chunk_size);

            // Apply correction
            let corrected = self
                .corrections
                .get(chunk_id as u64)
                .map(|(corr, _)| corr.apply(&decoded))
                .unwrap_or(decoded);

            // Calculate overlap with our range
            let start_in_chunk = offset.saturating_sub(current_offset);
            let end_in_chunk = (end_offset - current_offset).min(corrected.len());

            if start_in_chunk < end_in_chunk {
                result.extend_from_slice(&corrected[start_in_chunk..end_in_chunk]);
            }

            current_offset = chunk_end;

            // Stop if we've read enough
            if result.len() >= length {
                break;
            }
        }

        // Ensure we don't return more than requested
        result.truncate(length);

        Ok((result, file_entry.version))
    }

    /// Apply a delta operation to a file
    ///
    /// Delta encoding allows efficient modification of files without full re-encoding.
    /// For non-shifting operations (ByteReplace, RangeReplace), only affected chunks
    /// are re-encoded, providing significant speedup for large files.
    ///
    /// # Performance
    ///
    /// | Operation | Full Re-encode | Delta | Speedup |
    /// |-----------|----------------|-------|---------|
    /// | 1 byte in 1MB file | ~90ms | ~1ms | ~90x |
    /// | 10 bytes in 1MB file | ~90ms | ~10ms | ~9x |
    /// | Append 1KB to 1MB file | ~90ms | ~1ms | ~90x |
    ///
    /// # Arguments
    /// * `path` - Path to the file to modify
    /// * `delta` - The delta operation to apply
    ///
    /// # Returns
    /// The new file version after applying the delta
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::{VersionedEmbrFS, Delta, DeltaType};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    ///
    /// // Create a file
    /// let version = fs.write_file("data.txt", b"Hello World", None)?;
    ///
    /// // Replace 'W' with 'w' at position 6
    /// let delta = Delta::with_version(
    ///     DeltaType::ByteReplace {
    ///         offset: 6,
    ///         old_value: b'W',
    ///         new_value: b'w',
    ///     },
    ///     version,
    /// );
    /// let new_version = fs.apply_delta("data.txt", &delta)?;
    ///
    /// // File now contains "Hello world"
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply_delta(
        &self,
        path: &str,
        delta: &crate::fs::delta::Delta,
    ) -> Result<u64, EmbrFSError> {
        use crate::fs::delta::{analyze_delta, DeltaType};

        // 1. Get file entry and verify version
        let (file_entry, _manifest_version) = self
            .manifest
            .get_file(path)
            .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;

        if file_entry.deleted {
            return Err(EmbrFSError::FileNotFound(path.to_string()));
        }

        // 2. Check version if required
        if let Some(expected) = delta.expected_version {
            if file_entry.version != expected {
                return Err(EmbrFSError::VersionMismatch {
                    expected,
                    actual: file_entry.version,
                });
            }
        }

        // 3. Analyze the delta to determine affected chunks
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let affected = analyze_delta(
            delta,
            file_entry.size,
            chunk_size,
            file_entry.chunk_offsets.as_deref(),
        );

        if affected.is_empty() {
            // No chunks affected - return current version
            return Ok(file_entry.version);
        }

        // 4. Handle based on delta type
        match &delta.delta_type {
            DeltaType::ByteReplace {
                offset,
                old_value,
                new_value,
            } => self.apply_byte_replace(path, &file_entry, *offset, *old_value, *new_value),

            DeltaType::MultiByteReplace { changes } => {
                self.apply_multi_byte_replace(path, &file_entry, changes)
            }

            DeltaType::RangeReplace {
                offset,
                old_data,
                new_data,
            } => {
                if old_data.len() == new_data.len() {
                    // Same-length replace: efficient chunk-level update
                    self.apply_same_length_replace(path, &file_entry, *offset, old_data, new_data)
                } else {
                    // Different length: fall back to full rewrite for affected region
                    self.apply_length_changing_replace(
                        path,
                        &file_entry,
                        *offset,
                        old_data,
                        new_data,
                    )
                }
            }

            DeltaType::Append { data } => self.apply_append(path, &file_entry, data),

            DeltaType::Truncate {
                new_length,
                truncated_data: _,
            } => self.apply_truncate(path, &file_entry, *new_length),

            DeltaType::Insert { .. } | DeltaType::Delete { .. } => {
                // For shifting operations, we need to re-read and rewrite the affected region
                // This is handled by reading current content, applying delta, then writing
                Err(EmbrFSError::InvalidOperation(
                    "Insert/Delete operations require full file rewrite - use read_file + write_file".to_string(),
                ))
            }
        }
    }

    /// Encode chunk data using appropriate encoder based on mode.
    ///
    /// When holographic mode is enabled, uses ReversibleVSAEncoder for ~94% uncorrected accuracy.
    /// Otherwise uses legacy SparseVec::encode_data().
    pub fn encode_chunk(&self, data: &[u8], path: Option<&str>) -> SparseVec {
        if self.holographic_mode {
            self.reversible_encoder.write().unwrap().encode(data)
        } else {
            SparseVec::encode_data(data, &self.config, path)
        }
    }

    /// Decode chunk data using appropriate decoder based on mode.
    ///
    /// Must match the encoding method: use ReversibleVSAEncoder::decode for holographic mode,
    /// SparseVec::decode_data for legacy mode.
    pub fn decode_chunk(
        &self,
        vec: &SparseVec,
        path: Option<&str>,
        original_size: usize,
    ) -> Vec<u8> {
        if self.holographic_mode {
            self.reversible_encoder
                .read()
                .unwrap()
                .decode(vec, original_size)
        } else {
            vec.decode_data(&self.config, path, original_size)
        }
    }

    /// Encode chunk data using an explicit encoding format.
    ///
    /// Use this when encoding chunks for existing files that have a specific encoding_format
    /// stored in their manifest entry, to preserve consistency.
    pub fn encode_chunk_with_format(
        &self,
        data: &[u8],
        path: Option<&str>,
        encoding_format: Option<u8>,
    ) -> SparseVec {
        if encoding_format == Some(ENCODING_FORMAT_REVERSIBLE_VSA) {
            self.reversible_encoder.write().unwrap().encode(data)
        } else {
            SparseVec::encode_data(data, &self.config, path)
        }
    }

    /// Decode chunk data using an explicit encoding format.
    ///
    /// Use this when decoding chunks for existing files that have a specific encoding_format
    /// stored in their manifest entry, to ensure correct decoding.
    pub fn decode_chunk_with_format(
        &self,
        vec: &SparseVec,
        path: Option<&str>,
        original_size: usize,
        encoding_format: Option<u8>,
    ) -> Vec<u8> {
        if encoding_format == Some(ENCODING_FORMAT_REVERSIBLE_VSA) {
            self.reversible_encoder
                .read()
                .unwrap()
                .decode(vec, original_size)
        } else {
            vec.decode_data(&self.config, path, original_size)
        }
    }

    /// Apply a single byte replacement
    fn apply_byte_replace(
        &self,
        path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        offset: usize,
        _old_value: u8,
        new_value: u8,
    ) -> Result<u64, EmbrFSError> {
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let chunk_idx = offset / chunk_size;

        if chunk_idx >= file_entry.chunks.len() {
            return Err(EmbrFSError::InvalidOperation(
                "Offset out of bounds".to_string(),
            ));
        }

        let chunk_id = file_entry.chunks[chunk_idx];

        // 1. Get and decode the current chunk
        let (chunk, _) = self
            .chunk_store
            .get(chunk_id)
            .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;

        let decoded = chunk
            .vector
            .decode_data(&self.config, Some(path), chunk.original_size);

        let corrected = self
            .corrections
            .get(chunk_id as u64)
            .map(|(corr, _)| corr.apply(&decoded))
            .unwrap_or(decoded);

        // 2. Apply the byte change
        let offset_in_chunk = offset % chunk_size;
        let mut modified = corrected;
        if offset_in_chunk < modified.len() {
            modified[offset_in_chunk] = new_value;
        }

        // 3. Re-encode the modified chunk (using mode-appropriate encoder)
        let new_vec = self.encode_chunk(&modified, Some(path));

        // 4. Compute new correction (using mode-appropriate decoder)
        let decoded_new = self.decode_chunk(&new_vec, Some(path), modified.len());
        let correction =
            crate::correction::ChunkCorrection::new(chunk_id as u64, &modified, &decoded_new);

        // 5. Compute new content hash
        let mut hasher = Sha256::new();
        hasher.update(&modified);
        let hash = hasher.finalize();
        let mut hash_bytes = [0u8; 8];
        hash_bytes.copy_from_slice(&hash[0..8]);

        // 6. Create updated versioned chunk
        let new_chunk = crate::versioned::VersionedChunk::new(new_vec, modified.len(), hash_bytes);

        // 7. Update stores
        let store_version = self.chunk_store.version();
        self.chunk_store
            .insert(chunk_id, new_chunk, store_version)?;

        if correction.needs_correction() {
            let corrections_version = self.corrections.current_version();
            self.corrections
                .update(chunk_id as u64, correction, corrections_version)?;
        }

        // 8. Update manifest with new version
        let new_version = self.global_version.fetch_add(1, Ordering::SeqCst);
        let mut updated_entry = file_entry.clone();
        updated_entry.version = new_version;
        self.manifest
            .update_file(path, updated_entry, file_entry.version)?;

        Ok(new_version)
    }

    /// Apply multiple byte replacements
    fn apply_multi_byte_replace(
        &self,
        path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        changes: &[(usize, u8, u8)],
    ) -> Result<u64, EmbrFSError> {
        // Group changes by chunk
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let mut changes_by_chunk: std::collections::HashMap<usize, Vec<(usize, u8)>> =
            std::collections::HashMap::new();

        for &(offset, _old_val, new_val) in changes {
            let chunk_idx = offset / chunk_size;
            let offset_in_chunk = offset % chunk_size;
            changes_by_chunk
                .entry(chunk_idx)
                .or_default()
                .push((offset_in_chunk, new_val));
        }

        // Process each affected chunk
        for (chunk_idx, chunk_changes) in changes_by_chunk {
            if chunk_idx >= file_entry.chunks.len() {
                continue;
            }

            let chunk_id = file_entry.chunks[chunk_idx];

            // 1. Get and decode the current chunk
            let (chunk, _) = self
                .chunk_store
                .get(chunk_id)
                .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;

            let decoded = chunk
                .vector
                .decode_data(&self.config, Some(path), chunk.original_size);

            let corrected = self
                .corrections
                .get(chunk_id as u64)
                .map(|(corr, _)| corr.apply(&decoded))
                .unwrap_or(decoded);

            // 2. Apply all changes to this chunk
            let mut modified = corrected;
            for (offset_in_chunk, new_val) in chunk_changes {
                if offset_in_chunk < modified.len() {
                    modified[offset_in_chunk] = new_val;
                }
            }

            // 3. Re-encode and update (using mode-appropriate encoder)
            let new_vec = self.encode_chunk(&modified, Some(path));
            let decoded_new = self.decode_chunk(&new_vec, Some(path), modified.len());
            let correction =
                crate::correction::ChunkCorrection::new(chunk_id as u64, &modified, &decoded_new);

            let mut hasher = Sha256::new();
            hasher.update(&modified);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            let new_chunk =
                crate::versioned::VersionedChunk::new(new_vec, modified.len(), hash_bytes);
            let store_version = self.chunk_store.version();
            self.chunk_store
                .insert(chunk_id, new_chunk, store_version)?;

            if correction.needs_correction() {
                let corrections_version = self.corrections.current_version();
                self.corrections
                    .update(chunk_id as u64, correction, corrections_version)?;
            }
        }

        // Update manifest
        let new_version = self.global_version.fetch_add(1, Ordering::SeqCst);
        let mut updated_entry = file_entry.clone();
        updated_entry.version = new_version;
        self.manifest
            .update_file(path, updated_entry, file_entry.version)?;

        Ok(new_version)
    }

    /// Apply same-length range replacement
    fn apply_same_length_replace(
        &self,
        path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        offset: usize,
        _old_data: &[u8],
        new_data: &[u8],
    ) -> Result<u64, EmbrFSError> {
        // Convert to multi-byte replace
        let changes: Vec<(usize, u8, u8)> = new_data
            .iter()
            .enumerate()
            .map(|(i, &new_val)| (offset + i, 0, new_val)) // old_val not used in apply
            .collect();

        self.apply_multi_byte_replace(path, file_entry, &changes)
    }

    /// Apply length-changing range replacement (requires rewrite of affected chunks)
    fn apply_length_changing_replace(
        &self,
        path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        offset: usize,
        old_data: &[u8],
        new_data: &[u8],
    ) -> Result<u64, EmbrFSError> {
        // Read the full file
        let (content, _) = self.read_file(path)?;

        // Verify old data matches
        if offset + old_data.len() <= content.len()
            && &content[offset..offset + old_data.len()] != old_data
        {
            return Err(EmbrFSError::VersionMismatch {
                expected: 0,
                actual: 1,
            });
        }

        // Apply the replacement
        let before = &content[..offset];
        let after = if offset + old_data.len() < content.len() {
            &content[offset + old_data.len()..]
        } else {
            &[]
        };

        let mut new_content = Vec::with_capacity(before.len() + new_data.len() + after.len());
        new_content.extend_from_slice(before);
        new_content.extend_from_slice(new_data);
        new_content.extend_from_slice(after);

        // Write back
        self.write_file(path, &new_content, Some(file_entry.version))
    }

    /// Apply an append operation
    fn apply_append(
        &self,
        path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        data: &[u8],
    ) -> Result<u64, EmbrFSError> {
        if data.is_empty() {
            return Ok(file_entry.version);
        }

        let chunk_size = DEFAULT_CHUNK_SIZE;
        let current_size = file_entry.size;

        // Calculate how much space remains in the last chunk
        let bytes_in_last_chunk = if current_size == 0 {
            0
        } else {
            ((current_size - 1) % chunk_size) + 1
        };
        let space_in_last = chunk_size - bytes_in_last_chunk;

        let mut chunk_ids = file_entry.chunks.clone();
        let mut new_size = current_size;

        // If there's space in the last chunk, fill it first
        let mut remaining_data = data;
        if space_in_last > 0 && !file_entry.chunks.is_empty() && bytes_in_last_chunk > 0 {
            let last_chunk_id = *file_entry.chunks.last().unwrap();
            let fill_amount = space_in_last.min(remaining_data.len());

            // Get and decode last chunk
            let (chunk, _) = self
                .chunk_store
                .get(last_chunk_id)
                .ok_or(EmbrFSError::ChunkNotFound(last_chunk_id))?;

            let decoded = chunk
                .vector
                .decode_data(&self.config, Some(path), chunk.original_size);

            let mut corrected = self
                .corrections
                .get(last_chunk_id as u64)
                .map(|(corr, _)| corr.apply(&decoded))
                .unwrap_or(decoded);

            // Append data to last chunk
            corrected.extend_from_slice(&remaining_data[..fill_amount]);

            // Re-encode (using mode-appropriate encoder)
            let new_vec = self.encode_chunk(&corrected, Some(path));
            let decoded_new = self.decode_chunk(&new_vec, Some(path), corrected.len());
            let correction = crate::correction::ChunkCorrection::new(
                last_chunk_id as u64,
                &corrected,
                &decoded_new,
            );

            let mut hasher = Sha256::new();
            hasher.update(&corrected);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            let new_chunk =
                crate::versioned::VersionedChunk::new(new_vec, corrected.len(), hash_bytes);
            let store_version = self.chunk_store.version();
            self.chunk_store
                .insert(last_chunk_id, new_chunk, store_version)?;

            if correction.needs_correction() {
                let corrections_version = self.corrections.current_version();
                self.corrections
                    .update(last_chunk_id as u64, correction, corrections_version)?;
            }

            remaining_data = &remaining_data[fill_amount..];
            new_size += fill_amount;
        }

        // Create new chunks for remaining data
        for chunk_data in remaining_data.chunks(chunk_size) {
            let chunk_id = self.allocate_chunk_id();

            // Use mode-appropriate encoder
            let chunk_vec = self.encode_chunk(chunk_data, Some(path));
            let decoded = self.decode_chunk(&chunk_vec, Some(path), chunk_data.len());
            let correction =
                crate::correction::ChunkCorrection::new(chunk_id as u64, chunk_data, &decoded);

            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            let versioned_chunk =
                crate::versioned::VersionedChunk::new(chunk_vec, chunk_data.len(), hash_bytes);
            let store_version = self.chunk_store.version();
            self.chunk_store
                .insert(chunk_id, versioned_chunk, store_version)?;

            if correction.needs_correction() {
                let corrections_version = self.corrections.current_version();
                self.corrections
                    .update(chunk_id as u64, correction, corrections_version)?;
            }

            chunk_ids.push(chunk_id);
            new_size += chunk_data.len();
        }

        // Update manifest
        let new_version = self.global_version.fetch_add(1, Ordering::SeqCst);
        let mut updated_entry = file_entry.clone();
        updated_entry.chunks = chunk_ids;
        updated_entry.size = new_size;
        updated_entry.version = new_version;

        // Rebuild offset index
        let chunk_sizes: Vec<usize> = updated_entry
            .chunks
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i < updated_entry.chunks.len() - 1 {
                    chunk_size
                } else {
                    new_size - (i * chunk_size)
                }
            })
            .collect();
        updated_entry.build_offset_index(&chunk_sizes);

        self.manifest
            .update_file(path, updated_entry, file_entry.version)?;

        Ok(new_version)
    }

    /// Apply a truncate operation
    fn apply_truncate(
        &self,
        path: &str,
        file_entry: &crate::fs::versioned::manifest::VersionedFileEntry,
        new_length: usize,
    ) -> Result<u64, EmbrFSError> {
        if new_length >= file_entry.size {
            return Ok(file_entry.version);
        }

        let chunk_size = DEFAULT_CHUNK_SIZE;
        let new_chunk_count = new_length.div_ceil(chunk_size);

        // Truncate chunk list
        let mut new_chunks = file_entry.chunks.clone();
        new_chunks.truncate(new_chunk_count);

        // If the last chunk is partial, we need to re-encode it
        if new_length > 0 {
            let last_chunk_bytes = new_length - ((new_chunk_count - 1) * chunk_size);
            if last_chunk_bytes < chunk_size && !new_chunks.is_empty() {
                let last_chunk_id = *new_chunks.last().unwrap();

                // Get and decode last chunk
                let (chunk, _) = self
                    .chunk_store
                    .get(last_chunk_id)
                    .ok_or(EmbrFSError::ChunkNotFound(last_chunk_id))?;

                let decoded =
                    chunk
                        .vector
                        .decode_data(&self.config, Some(path), chunk.original_size);

                let corrected = self
                    .corrections
                    .get(last_chunk_id as u64)
                    .map(|(corr, _)| corr.apply(&decoded))
                    .unwrap_or(decoded);

                // Truncate the chunk data
                let truncated: Vec<u8> = corrected.into_iter().take(last_chunk_bytes).collect();

                // Re-encode (using mode-appropriate encoder)
                let new_vec = self.encode_chunk(&truncated, Some(path));
                let decoded_new = self.decode_chunk(&new_vec, Some(path), truncated.len());
                let correction = crate::correction::ChunkCorrection::new(
                    last_chunk_id as u64,
                    &truncated,
                    &decoded_new,
                );

                let mut hasher = Sha256::new();
                hasher.update(&truncated);
                let hash = hasher.finalize();
                let mut hash_bytes = [0u8; 8];
                hash_bytes.copy_from_slice(&hash[0..8]);

                let new_chunk =
                    crate::versioned::VersionedChunk::new(new_vec, truncated.len(), hash_bytes);
                let store_version = self.chunk_store.version();
                self.chunk_store
                    .insert(last_chunk_id, new_chunk, store_version)?;

                if correction.needs_correction() {
                    let corrections_version = self.corrections.current_version();
                    self.corrections.update(
                        last_chunk_id as u64,
                        correction,
                        corrections_version,
                    )?;
                }
            }
        }

        // Update manifest
        let new_version = self.global_version.fetch_add(1, Ordering::SeqCst);
        let mut updated_entry = file_entry.clone();
        updated_entry.chunks = new_chunks;
        updated_entry.size = new_length;
        updated_entry.version = new_version;

        // Rebuild offset index
        let chunk_sizes: Vec<usize> = updated_entry
            .chunks
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i < updated_entry.chunks.len() - 1 {
                    chunk_size
                } else {
                    new_length - (i * chunk_size)
                }
            })
            .collect();
        updated_entry.build_offset_index(&chunk_sizes);

        self.manifest
            .update_file(path, updated_entry, file_entry.version)?;

        Ok(new_version)
    }

    /// Write a file's contents
    ///
    /// If `expected_version` is provided, performs optimistic locking - the write
    /// will fail with VersionMismatch if the file has been modified since the version
    /// was read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::VersionedEmbrFS;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    ///
    /// // Create new file
    /// let version = fs.write_file("new.txt", b"content", None)?;
    ///
    /// // Update with version check
    /// let new_version = fs.write_file("new.txt", b"updated", Some(version))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_file(
        &self,
        path: &str,
        data: &[u8],
        expected_version: Option<u64>,
    ) -> Result<u64, EmbrFSError> {
        // 1. Check existing file
        let existing = self.manifest.get_file(path);

        match (&existing, expected_version) {
            (Some((entry, _)), Some(expected_ver)) => {
                // Update existing - verify version
                if entry.version != expected_ver {
                    return Err(EmbrFSError::VersionMismatch {
                        expected: expected_ver,
                        actual: entry.version,
                    });
                }
            }
            (Some(_), None) => {
                // File exists but no version check - fail
                return Err(EmbrFSError::FileExists(path.to_string()));
            }
            (None, Some(_)) => {
                // Expected file but doesn't exist
                return Err(EmbrFSError::FileNotFound(path.to_string()));
            }
            (None, None) => {
                // New file - OK
            }
        }

        // 2. Chunk the data
        let chunks = self.chunk_data(data);
        let mut chunk_ids = Vec::new();

        // 3. Get current chunk store version
        let store_version = self.chunk_store.version();

        // 4. Encode chunks and build updates
        let mut chunk_updates = Vec::new();
        let mut corrections_to_add = Vec::new();

        for chunk_data in chunks {
            let chunk_id = self.allocate_chunk_id();

            // Encode and verify using helper methods
            let chunk_vec = self.encode_chunk(chunk_data, Some(path));
            let decoded = self.decode_chunk(&chunk_vec, Some(path), chunk_data.len());

            // Compute content hash
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            // Create versioned chunk
            let versioned_chunk = VersionedChunk::new(chunk_vec, chunk_data.len(), hash_bytes);

            chunk_updates.push((chunk_id, versioned_chunk));

            // Prepare correction
            let correction =
                crate::correction::ChunkCorrection::new(chunk_id as u64, chunk_data, &decoded);
            corrections_to_add.push((chunk_id as u64, correction));

            chunk_ids.push(chunk_id);
        }

        // 5. Batch insert chunks into store
        if expected_version.is_none() {
            // New file - use lock-free insert (chunk IDs are unique)
            self.chunk_store.batch_insert_new(chunk_updates)?;
        } else {
            // Existing file - use versioned update with optimistic locking
            self.chunk_store
                .batch_insert(chunk_updates, store_version)?;
        }

        // 6. Add corrections (after chunk store update)
        if expected_version.is_none() {
            // New file - use lock-free insert (chunk IDs are unique)
            self.corrections.batch_insert_new(corrections_to_add)?;
        } else {
            // Existing file - use versioned batch update
            let corrections_version = self.corrections.current_version();
            self.corrections
                .batch_update(corrections_to_add, corrections_version)?;
        }

        // 6. Update manifest
        let is_text = is_text_data(data);
        let mut new_entry =
            VersionedFileEntry::new(path.to_string(), is_text, data.len(), chunk_ids.clone());

        // Set encoding format for holographic mode files
        if self.holographic_mode {
            new_entry.encoding_format = Some(ENCODING_FORMAT_REVERSIBLE_VSA);
        }

        let file_version = if let Some((entry, _)) = existing {
            self.manifest.update_file(path, new_entry, entry.version)?;
            entry.version + 1
        } else {
            self.manifest.add_file(new_entry)?;
            0
        };

        // 7. Bundle chunks into root
        self.bundle_chunks_to_root(&chunk_ids)?;

        // 8. Increment global version
        self.global_version.fetch_add(1, Ordering::AcqRel);

        Ok(file_version)
    }

    /// Write a file's contents with path-based automatic compression
    ///
    /// Uses the compression profiler to automatically select the appropriate
    /// compression codec based on the file path. For example, config files
    /// in /etc get fast LZ4 compression, kernel images get maximum zstd
    /// compression, and pre-compressed media files skip compression entirely.
    ///
    /// If `expected_version` is provided, performs optimistic locking - the write
    /// will fail with VersionMismatch if the file has been modified since the version
    /// was read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::VersionedEmbrFS;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    ///
    /// // Config file gets fast LZ4 compression automatically
    /// let version = fs.write_file_compressed("/etc/nginx.conf", b"worker_processes 4;", None)?;
    ///
    /// // Kernel image gets maximum zstd compression
    /// let kernel_data = std::fs::read("/boot/vmlinuz")?;
    /// fs.write_file_compressed("/boot/vmlinuz", &kernel_data, None)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_file_compressed(
        &self,
        path: &str,
        data: &[u8],
        expected_version: Option<u64>,
    ) -> Result<u64, EmbrFSError> {
        // 1. Get compression profile for this path
        let profile = self.profiler.for_path(path);
        let write_opts = profile.to_write_options();

        // 2. Compress data using the selected profile
        let (compressed_data, codec_byte) = if write_opts.codec == CompressionCodec::None {
            // No compression - store as-is
            (data.to_vec(), 0u8)
        } else {
            // Wrap with envelope format (includes compression)
            let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, write_opts, data)
                .map_err(|e| EmbrFSError::IoError(format!("Compression failed: {}", e)))?;
            let codec = match write_opts.codec {
                CompressionCodec::None => 0,
                CompressionCodec::Zstd => 1,
                CompressionCodec::Lz4 => 2,
            };
            (wrapped, codec)
        };

        // 3. Check existing file
        let existing = self.manifest.get_file(path);

        match (&existing, expected_version) {
            (Some((entry, _)), Some(expected_ver)) => {
                if entry.version != expected_ver {
                    return Err(EmbrFSError::VersionMismatch {
                        expected: expected_ver,
                        actual: entry.version,
                    });
                }
            }
            (Some(_), None) => {
                return Err(EmbrFSError::FileExists(path.to_string()));
            }
            (None, Some(_)) => {
                return Err(EmbrFSError::FileNotFound(path.to_string()));
            }
            (None, None) => {}
        }

        // 4. Chunk the compressed data
        let chunks = self.chunk_data(&compressed_data);
        let mut chunk_ids = Vec::new();

        // 5. Get current chunk store version
        let store_version = self.chunk_store.version();

        // 6. Encode chunks and build updates
        let mut chunk_updates = Vec::new();
        let mut corrections_to_add = Vec::new();

        for chunk_data in chunks {
            let chunk_id = self.allocate_chunk_id();

            // Encode and verify using helper methods
            let chunk_vec = self.encode_chunk(chunk_data, Some(path));
            let decoded = self.decode_chunk(&chunk_vec, Some(path), chunk_data.len());

            // Compute content hash
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            // Create versioned chunk
            let versioned_chunk = VersionedChunk::new(chunk_vec, chunk_data.len(), hash_bytes);
            chunk_updates.push((chunk_id, versioned_chunk));

            // Prepare correction
            let correction =
                crate::correction::ChunkCorrection::new(chunk_id as u64, chunk_data, &decoded);
            corrections_to_add.push((chunk_id as u64, correction));

            chunk_ids.push(chunk_id);
        }

        // 7. Batch insert chunks into store
        if expected_version.is_none() {
            self.chunk_store.batch_insert_new(chunk_updates)?;
        } else {
            self.chunk_store
                .batch_insert(chunk_updates, store_version)?;
        }

        // 8. Add corrections
        if expected_version.is_none() {
            self.corrections.batch_insert_new(corrections_to_add)?;
        } else {
            let corrections_version = self.corrections.current_version();
            self.corrections
                .batch_update(corrections_to_add, corrections_version)?;
        }

        // 9. Update manifest with compression metadata
        let is_text = is_text_data(data);
        let mut new_entry = if codec_byte == 0 {
            VersionedFileEntry::new(path.to_string(), is_text, data.len(), chunk_ids.clone())
        } else {
            VersionedFileEntry::new_compressed(
                path.to_string(),
                is_text,
                compressed_data.len(),
                data.len(),
                codec_byte,
                chunk_ids.clone(),
            )
        };

        // Set encoding format for holographic mode files
        if self.holographic_mode {
            new_entry.encoding_format = Some(ENCODING_FORMAT_REVERSIBLE_VSA);
        }

        let file_version = if let Some((entry, _)) = existing {
            self.manifest.update_file(path, new_entry, entry.version)?;
            entry.version + 1
        } else {
            self.manifest.add_file(new_entry)?;
            0
        };

        // 10. Bundle chunks into root
        self.bundle_chunks_to_root(&chunk_ids)?;

        // 11. Increment global version
        self.global_version.fetch_add(1, Ordering::AcqRel);

        Ok(file_version)
    }

    /// Delete a file (soft delete)
    ///
    /// The file is marked as deleted but its chunks remain in the engram until
    /// compaction. Requires the current file version for optimistic locking.
    pub fn delete_file(&self, path: &str, expected_version: u64) -> Result<(), EmbrFSError> {
        self.manifest.remove_file(path, expected_version)?;
        self.global_version.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    /// List all non-deleted files
    pub fn list_files(&self) -> Vec<String> {
        self.manifest.list_files()
    }

    /// Check if a file exists
    pub fn exists(&self, path: &str) -> bool {
        self.manifest
            .get_file(path)
            .map(|(entry, _)| !entry.deleted)
            .unwrap_or(false)
    }

    /// Get filesystem statistics
    pub fn stats(&self) -> FilesystemStats {
        let manifest_stats = self.manifest.stats();
        let chunk_stats = self.chunk_store.stats();
        let correction_stats = self.corrections.stats();

        FilesystemStats {
            total_files: manifest_stats.total_files,
            active_files: manifest_stats.active_files,
            deleted_files: manifest_stats.deleted_files,
            total_chunks: chunk_stats.total_chunks as u64,
            total_size_bytes: manifest_stats.total_size_bytes,
            correction_overhead_bytes: correction_stats.total_correction_bytes,
            version: self.version(),
        }
    }

    // === Holographic encoding methods ===

    /// Write a file using holographic encoding via ReversibleVSAEncoder
    ///
    /// This is the TRUE holographic storage method achieving ~94% uncorrected accuracy:
    /// 1. Encode data using ReversibleVSAEncoder.encode_chunked() (position-aware binding)
    /// 2. Store encoded SparseVecs in chunk_store (one per REVERSIBLE_CHUNK_SIZE bytes)
    /// 3. Decode to verify and compute corrections for bit-perfect reconstruction
    /// 4. Store only sparse corrections (~6% of bytes need correction)
    ///
    /// The position-aware binding ensures each byte at each position has a unique
    /// representation that can be retrieved via unbinding.
    pub fn write_file_holographic(
        &self,
        path: &str,
        data: &[u8],
        expected_version: Option<u64>,
    ) -> Result<u64, EmbrFSError> {
        // 1. Check existing file
        let existing = self.manifest.get_file(path);

        match (&existing, expected_version) {
            (Some((entry, _)), Some(expected_ver)) => {
                if entry.version != expected_ver {
                    return Err(EmbrFSError::VersionMismatch {
                        expected: expected_ver,
                        actual: entry.version,
                    });
                }
            }
            (Some(_), None) => {
                return Err(EmbrFSError::FileExists(path.to_string()));
            }
            (None, Some(_)) => {
                return Err(EmbrFSError::FileNotFound(path.to_string()));
            }
            (None, None) => {}
        }

        // 2. Encode data using ReversibleVSAEncoder with chunking for optimal accuracy
        let mut encoder = self.reversible_encoder.write().unwrap();
        let encoded_chunks = encoder.encode_chunked(data, REVERSIBLE_CHUNK_SIZE);

        // Decode to verify and compute corrections
        let decoded = encoder.decode_chunked(&encoded_chunks, REVERSIBLE_CHUNK_SIZE, data.len());
        drop(encoder);

        // 3. Store each encoded chunk
        let store_version = self.chunk_store.version();
        let mut chunk_ids = Vec::with_capacity(encoded_chunks.len());
        let mut chunk_updates = Vec::with_capacity(encoded_chunks.len());
        let mut corrections_to_add = Vec::new();

        for (chunk_idx, chunk_vec) in encoded_chunks.into_iter().enumerate() {
            let chunk_id = self.allocate_chunk_id();
            chunk_ids.push(chunk_id);

            // Calculate the data range for this chunk
            let start = chunk_idx * REVERSIBLE_CHUNK_SIZE;
            let end = (start + REVERSIBLE_CHUNK_SIZE).min(data.len());
            let chunk_data = &data[start..end];
            let decoded_chunk = &decoded[start..end];

            // Compute content hash for verification
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize();
            let mut hash_bytes = [0u8; 8];
            hash_bytes.copy_from_slice(&hash[0..8]);

            let versioned_chunk = VersionedChunk::new(chunk_vec, chunk_data.len(), hash_bytes);
            chunk_updates.push((chunk_id, versioned_chunk));

            // Prepare correction (only stores differences, should be ~6% of bytes)
            let correction =
                crate::correction::ChunkCorrection::new(chunk_id as u64, chunk_data, decoded_chunk);
            corrections_to_add.push((chunk_id as u64, correction));
        }

        // 4. Batch insert chunks
        if expected_version.is_none() {
            self.chunk_store.batch_insert_new(chunk_updates)?;
        } else {
            self.chunk_store
                .batch_insert(chunk_updates, store_version)?;
        }

        // 5. Batch insert corrections
        if expected_version.is_none() {
            self.corrections.batch_insert_new(corrections_to_add)?;
        } else {
            let corrections_version = self.corrections.current_version();
            self.corrections
                .batch_update(corrections_to_add, corrections_version)?;
        }

        // 6. Update manifest with encoding format version
        let is_text = is_text_data(data);
        let new_entry = VersionedFileEntry::new_holographic(
            path.to_string(),
            is_text,
            data.len(),
            chunk_ids.clone(),
            ENCODING_FORMAT_REVERSIBLE_VSA,
        );

        let file_version = if let Some((entry, _)) = existing {
            self.manifest.update_file(path, new_entry, entry.version)?;
            entry.version + 1
        } else {
            self.manifest.add_file(new_entry)?;
            0
        };

        // 7. Bundle into root
        self.bundle_chunks_to_root(&chunk_ids)?;

        // 8. Increment global version
        self.global_version.fetch_add(1, Ordering::AcqRel);

        Ok(file_version)
    }

    /// Read a file using holographic decoding via ReversibleVSAEncoder
    ///
    /// This reverses the holographic encoding:
    /// 1. Load all SparseVecs from chunk_store
    /// 2. Use ReversibleVSAEncoder.decode_chunked() to reconstruct data
    /// 3. Apply per-chunk corrections for bit-perfect result
    ///
    /// Supports both legacy (format 0) and new (format 1) encoding formats.
    pub fn read_file_holographic(&self, path: &str) -> Result<(Vec<u8>, u64), EmbrFSError> {
        // 1. Get file entry
        let (file_entry, _) = self
            .manifest
            .get_file(path)
            .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;

        if file_entry.deleted {
            return Err(EmbrFSError::FileNotFound(path.to_string()));
        }

        // 2. Handle empty files
        if file_entry.chunks.is_empty() {
            return Ok((Vec::new(), file_entry.version));
        }

        // 3. Check encoding format and dispatch to appropriate decoder
        let encoding_format = file_entry.encoding_format.unwrap_or(ENCODING_FORMAT_LEGACY);

        match encoding_format {
            ENCODING_FORMAT_REVERSIBLE_VSA => self.read_file_holographic_reversible(&file_entry),
            // Legacy format (0) and any unknown formats: use codebook-based reconstruction
            _ => self.read_file_holographic_legacy(&file_entry),
        }
    }

    /// Read a file encoded with ReversibleVSAEncoder (format 1)
    fn read_file_holographic_reversible(
        &self,
        file_entry: &VersionedFileEntry,
    ) -> Result<(Vec<u8>, u64), EmbrFSError> {
        // 1. Load all chunk vectors (clone from Arc to owned SparseVec)
        let mut chunk_vecs = Vec::with_capacity(file_entry.chunks.len());
        for &chunk_id in &file_entry.chunks {
            let (chunk, _) = self
                .chunk_store
                .get(chunk_id)
                .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;
            // Clone the SparseVec out of the Arc for decode_chunked
            chunk_vecs.push((*chunk.vector).clone());
        }

        // 2. Decode using ReversibleVSAEncoder
        let encoder = self.reversible_encoder.read().unwrap();
        let mut reconstructed =
            encoder.decode_chunked(&chunk_vecs, REVERSIBLE_CHUNK_SIZE, file_entry.size);
        drop(encoder);

        // 3. Apply per-chunk corrections for bit-perfect result
        for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
            if let Some((correction, _)) = self.corrections.get(chunk_id as u64) {
                // Calculate the data range for this chunk
                let start = chunk_idx * REVERSIBLE_CHUNK_SIZE;
                let end = (start + REVERSIBLE_CHUNK_SIZE).min(file_entry.size);

                // Apply correction to this chunk's portion of the reconstructed data
                let chunk_data = &reconstructed[start..end];
                let corrected = correction.apply(chunk_data);

                // Copy corrected data back
                reconstructed[start..end].copy_from_slice(&corrected[..end - start]);
            }
        }

        // Truncate to exact size
        reconstructed.truncate(file_entry.size);

        Ok((reconstructed, file_entry.version))
    }

    /// Read a file encoded with legacy Codebook.project() (format 0)
    fn read_file_holographic_legacy(
        &self,
        file_entry: &VersionedFileEntry,
    ) -> Result<(Vec<u8>, u64), EmbrFSError> {
        // Legacy format expects a single chunk containing the projection
        let chunk_id = file_entry.chunks[0];
        let (chunk, _) = self
            .chunk_store
            .get(chunk_id)
            .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;

        // Convert SparseVec back to projection
        let projection = self.sparsevec_to_projection(&chunk.vector);

        // Reconstruct using codebook
        let codebook = self.codebook.read().unwrap();
        let mut reconstructed = codebook.reconstruct(&projection, file_entry.size);
        drop(codebook);

        // Apply correction for bit-perfect result
        if let Some((correction, _)) = self.corrections.get(chunk_id as u64) {
            reconstructed = correction.apply(&reconstructed);
        }

        // Truncate to exact size
        reconstructed.truncate(file_entry.size);

        Ok((reconstructed, file_entry.version))
    }

    /// Convert a ProjectionResult to a SparseVec for holographic storage
    ///
    /// The coefficients (basis_id -> weight) are encoded as:
    /// - pos indices: basis_id * 256 + encoded_weight for positive weights
    /// - neg indices: basis_id * 256 + encoded_weight for negative weights
    ///
    /// NOTE: This is legacy code kept for backward compatibility with format 0.
    /// New writes use ReversibleVSAEncoder (format 1) instead.
    #[allow(dead_code)]
    fn projection_to_sparsevec(&self, projection: &ProjectionResult) -> SparseVec {
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        for (&key, word) in &projection.coefficients {
            let value = word.decode();
            let basis_id = (key / 1000) as usize; // coefficient_key_spacing = 1000
            let chunk_idx = (key % 1000) as usize;

            // Encode as: (basis_id * max_chunks + chunk_idx) * 2 + sign
            // This gives us a unique index for each coefficient
            let base_idx = (basis_id * 1000 + chunk_idx) % DIM;

            if value > 0 {
                pos.push(base_idx);
            } else if value < 0 {
                neg.push(base_idx);
            }
        }

        pos.sort_unstable();
        pos.dedup();
        neg.sort_unstable();
        neg.dedup();

        SparseVec { pos, neg }
    }

    /// Convert a SparseVec back to a ProjectionResult for reconstruction
    fn sparsevec_to_projection(&self, vec: &SparseVec) -> ProjectionResult {
        use embeddenator_vsa::{BalancedTernaryWord, WordMetadata};
        use std::collections::HashMap;

        let mut coefficients = HashMap::new();

        // Decode positive indices
        for &idx in &vec.pos {
            let chunk_idx = idx % 1000;
            let basis_id = (idx / 1000) % 100; // Assume max 100 basis vectors
            let key = (basis_id * 1000 + chunk_idx) as u32;

            // Positive weight (use default scale)
            if let Ok(word) = BalancedTernaryWord::new(500, WordMetadata::Data) {
                coefficients.insert(key, word);
            }
        }

        // Decode negative indices
        for &idx in &vec.neg {
            let chunk_idx = idx % 1000;
            let basis_id = (idx / 1000) % 100;
            let key = (basis_id * 1000 + chunk_idx) as u32;

            // Negative weight
            if let Ok(word) = BalancedTernaryWord::new(-500, WordMetadata::Data) {
                coefficients.insert(key, word);
            }
        }

        ProjectionResult {
            coefficients,
            residual: Vec::new(), // Residual is in corrections
            outliers: Vec::new(),
            quality_score: 0.5,
        }
    }

    /// Convert projection residual to a ChunkCorrection
    ///
    /// NOTE: This is legacy code kept for backward compatibility with format 0.
    /// New writes use ReversibleVSAEncoder (format 1) instead.
    #[allow(dead_code)]
    fn projection_to_correction(
        &self,
        chunk_id: u64,
        original: &[u8],
        projection: &ProjectionResult,
    ) -> crate::correction::ChunkCorrection {
        // Reconstruct from projection (without correction)
        let codebook = self.codebook.read().unwrap();
        let reconstructed = codebook.reconstruct(projection, original.len());
        drop(codebook);

        // Create correction from difference
        crate::correction::ChunkCorrection::new(chunk_id, original, &reconstructed)
    }

    // === Public helper methods for streaming API ===

    /// Get the VSA configuration
    pub fn config(&self) -> &ReversibleVSAConfig {
        &self.config
    }

    /// Create a streaming decoder for memory-efficient file reading
    ///
    /// Returns a `StreamingDecoder` that decodes chunks on-demand, keeping memory
    /// usage bounded regardless of file size. Use this for large files where loading
    /// the entire file into memory is impractical.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::VersionedEmbrFS;
    /// # use std::io::Read;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    ///
    /// // Create streaming decoder
    /// let mut decoder = fs.stream_decode("large_file.bin")?;
    ///
    /// // Read in chunks without loading entire file
    /// let mut buffer = vec![0u8; 4096];
    /// while let Ok(n) = decoder.read(&mut buffer) {
    ///     if n == 0 { break; }
    ///     // Process buffer[..n]
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn stream_decode(
        &self,
        path: &str,
    ) -> Result<crate::streaming::StreamingDecoder<'_>, EmbrFSError> {
        crate::streaming::StreamingDecoder::new(self, path)
    }

    /// Create a streaming decoder with custom options
    ///
    /// Allows setting starting offset and maximum bytes to read for partial decoding.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embeddenator_fs::VersionedEmbrFS;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let fs = VersionedEmbrFS::new();
    ///
    /// // Read bytes 1000-2000 from a file
    /// let decoder = fs.stream_decode_range("large_file.bin", 1000, Some(1000))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stream_decode_range(
        &self,
        path: &str,
        offset: usize,
        max_bytes: Option<usize>,
    ) -> Result<crate::streaming::StreamingDecoder<'_>, EmbrFSError> {
        let mut builder = crate::streaming::StreamingDecoderBuilder::new(self, path);
        if offset > 0 {
            builder = builder.with_offset(offset);
        }
        if let Some(max) = max_bytes {
            builder = builder.with_max_bytes(max);
        }
        builder.build()
    }

    /// Allocate a new unique chunk ID (public for streaming API)
    pub fn allocate_chunk_id(&self) -> ChunkId {
        self.next_chunk_id.fetch_add(1, Ordering::AcqRel) as ChunkId
    }

    /// Bundle chunks into root - streaming variant that doesn't retry on mismatch
    ///
    /// For streaming ingestion, we bundle progressively without requiring
    /// atomicity since we're building up the root from scratch.
    pub fn bundle_chunks_to_root_streaming(
        &self,
        chunk_ids: &[ChunkId],
    ) -> Result<(), EmbrFSError> {
        self.bundle_chunks_to_root(chunk_ids)
    }

    // === Private helper methods ===

    /// Chunk data into DEFAULT_CHUNK_SIZE blocks
    fn chunk_data<'a>(&self, data: &'a [u8]) -> Vec<&'a [u8]> {
        data.chunks(DEFAULT_CHUNK_SIZE).collect()
    }

    /// Bundle chunks into root with CAS retry loop
    fn bundle_chunks_to_root(&self, chunk_ids: &[ChunkId]) -> Result<(), EmbrFSError> {
        // Retry loop for CAS
        loop {
            // Read current root
            let root_lock = self.root.read().unwrap();
            let current_root = Arc::clone(&*root_lock);
            let current_version = self.root_version.load(Ordering::Acquire);
            drop(root_lock);

            // Build new root by bundling chunks
            let mut new_root = (*current_root).clone();
            for &chunk_id in chunk_ids {
                if let Some((chunk, _)) = self.chunk_store.get(chunk_id) {
                    new_root = new_root.bundle(&chunk.vector);
                }
            }

            // Try CAS update
            let mut root_lock = self.root.write().unwrap();
            let actual_version = self.root_version.load(Ordering::Acquire);

            if actual_version == current_version {
                // Success - no concurrent update
                *root_lock = Arc::new(new_root);
                self.root_version.fetch_add(1, Ordering::AcqRel);
                return Ok(());
            }

            // Retry - someone else updated root
            drop(root_lock);
            // Small backoff to reduce contention
            std::thread::yield_now();
        }
    }
}

impl Default for VersionedEmbrFS {
    fn default() -> Self {
        Self::new()
    }
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FilesystemStats {
    pub total_files: usize,
    pub active_files: usize,
    pub deleted_files: usize,
    pub total_chunks: u64,
    pub total_size_bytes: usize,
    pub correction_overhead_bytes: u64,
    pub version: u64,
}

/// Heuristic to detect if data is likely text
fn is_text_data(data: &[u8]) -> bool {
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
    fn test_new_filesystem() {
        let fs = VersionedEmbrFS::new();
        assert_eq!(fs.version(), 0);
        assert_eq!(fs.list_files().len(), 0);
    }

    #[test]
    fn test_write_and_read_file() {
        let fs = VersionedEmbrFS::new();
        let data = b"Hello, EmbrFS!";

        // Write file
        let version = fs.write_file("test.txt", data, None).unwrap();
        assert_eq!(version, 0);

        // Read it back
        let (content, read_version) = fs.read_file("test.txt").unwrap();
        assert_eq!(&content[..], data);
        assert_eq!(read_version, 0);
    }

    #[test]
    fn test_update_file_with_version_check() {
        let fs = VersionedEmbrFS::new();

        // Create file
        let v1 = fs.write_file("test.txt", b"version 1", None).unwrap();

        // Update with correct version
        let v2 = fs.write_file("test.txt", b"version 2", Some(v1)).unwrap();
        assert_eq!(v2, v1 + 1);

        // Try to update with stale version (should fail)
        let result = fs.write_file("test.txt", b"version 3", Some(v1));
        assert!(matches!(result, Err(EmbrFSError::VersionMismatch { .. })));
    }

    #[test]
    fn test_delete_file() {
        let fs = VersionedEmbrFS::new();

        // Create and delete
        let version = fs.write_file("test.txt", b"data", None).unwrap();
        fs.delete_file("test.txt", version).unwrap();

        // Should not exist
        assert!(!fs.exists("test.txt"));

        // Read should fail
        let result = fs.read_file("test.txt");
        assert!(matches!(result, Err(EmbrFSError::FileNotFound(_))));
    }

    #[test]
    fn test_list_files() {
        let fs = VersionedEmbrFS::new();

        fs.write_file("file1.txt", b"a", None).unwrap();
        fs.write_file("file2.txt", b"b", None).unwrap();
        fs.write_file("file3.txt", b"c", None).unwrap();

        let files = fs.list_files();
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"file2.txt".to_string()));
        assert!(files.contains(&"file3.txt".to_string()));
    }

    #[test]
    fn test_large_file() {
        let fs = VersionedEmbrFS::new();

        // Create file larger than one chunk
        let data = vec![42u8; DEFAULT_CHUNK_SIZE * 3 + 100];
        fs.write_file("large.bin", &data, None).unwrap();

        let (content, _) = fs.read_file("large.bin").unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_stats() {
        let fs = VersionedEmbrFS::new();

        fs.write_file("file1.txt", b"hello", None).unwrap();
        fs.write_file("file2.txt", b"world", None).unwrap();

        let stats = fs.stats();
        assert_eq!(stats.active_files, 2);
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.deleted_files, 0);
        assert_eq!(stats.total_size_bytes, 10);
    }

    #[test]
    fn test_write_and_read_compressed_file() {
        let fs = VersionedEmbrFS::new();

        // Config files get LZ4 compression
        let config_data = b"[server]\nport = 8080\nhost = localhost";
        let version = fs
            .write_file_compressed("/etc/app.conf", config_data, None)
            .unwrap();
        assert_eq!(version, 0);

        // Read it back - should auto-decompress
        let (content, read_version) = fs.read_file("/etc/app.conf").unwrap();
        assert_eq!(&content[..], config_data);
        assert_eq!(read_version, 0);
    }

    #[test]
    fn test_write_compressed_with_zstd_profile() {
        let fs = VersionedEmbrFS::new();

        // Binary files get zstd compression
        let binary_data: Vec<u8> = (0..1000).map(|i| [0xDE, 0xAD, 0xBE, 0xEF][i % 4]).collect();
        let version = fs
            .write_file_compressed("/usr/bin/myapp", &binary_data, None)
            .unwrap();
        assert_eq!(version, 0);

        // Read it back
        let (content, _) = fs.read_file("/usr/bin/myapp").unwrap();
        assert_eq!(content, binary_data);
    }

    #[test]
    fn test_write_compressed_no_compression_for_media() {
        let fs = VersionedEmbrFS::new();

        // Media files skip compression (already compressed)
        let media_data: Vec<u8> = (0..500).map(|i| [0xFF, 0xD8, 0xFF, 0xE0][i % 4]).collect();
        let version = fs
            .write_file_compressed("/photos/image.jpg", &media_data, None)
            .unwrap();
        assert_eq!(version, 0);

        // Read it back - no decompression needed
        let (content, _) = fs.read_file("/photos/image.jpg").unwrap();
        assert_eq!(content, media_data);
    }

    #[test]
    fn test_profiler_access() {
        let fs = VersionedEmbrFS::new();
        let profiler = fs.profiler();

        // Test profile selection
        let kernel_profile = profiler.for_path("/boot/vmlinuz");
        assert_eq!(kernel_profile.name, "Kernel");

        let config_profile = profiler.for_path("/etc/nginx.conf");
        assert_eq!(config_profile.name, "Config");
    }

    #[test]
    fn test_holographic_write_and_read() {
        let fs = VersionedEmbrFS::new_holographic();
        let data = b"Hello, Holographic EmbrFS!";

        // Write file using holographic encoding
        let version = fs.write_file_holographic("test.txt", data, None).unwrap();
        assert_eq!(version, 0);

        // Read it back
        let (content, read_version) = fs.read_file_holographic("test.txt").unwrap();
        assert_eq!(&content[..], data);
        assert_eq!(read_version, 0);
    }

    #[test]
    fn test_holographic_accuracy() {
        let fs = VersionedEmbrFS::new_holographic();

        // Test with various data patterns to verify >90% uncorrected accuracy
        let test_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

        // Write and read back
        fs.write_file_holographic("accuracy_test.bin", &test_data, None)
            .unwrap();
        let (content, _) = fs.read_file_holographic("accuracy_test.bin").unwrap();

        // Should be bit-perfect (with corrections applied)
        assert_eq!(content, test_data);

        // Check that correction overhead is reasonable
        let stats = fs.stats();
        let correction_ratio =
            stats.correction_overhead_bytes as f64 / stats.total_size_bytes as f64;

        // With ~94% accuracy, correction should be <10% of data
        // (allowing some margin for test data patterns)
        assert!(
            correction_ratio < 0.15,
            "Correction overhead too high: {:.1}%",
            correction_ratio * 100.0
        );
    }

    #[test]
    fn test_holographic_large_file() {
        let fs = VersionedEmbrFS::new_holographic();

        // Create a larger file that spans multiple chunks
        let data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();

        fs.write_file_holographic("large_holo.bin", &data, None)
            .unwrap();
        let (content, _) = fs.read_file_holographic("large_holo.bin").unwrap();

        // Should be bit-perfect
        assert_eq!(content, data);
    }

    #[test]
    fn test_holographic_encoding_format_in_manifest() {
        let fs = VersionedEmbrFS::new_holographic();
        let data = b"Test encoding format";

        fs.write_file_holographic("format_test.txt", data, None)
            .unwrap();

        // Check manifest has correct encoding format
        let (file_entry, _) = fs.manifest.get_file("format_test.txt").unwrap();
        assert_eq!(
            file_entry.encoding_format,
            Some(ENCODING_FORMAT_REVERSIBLE_VSA)
        );
    }

    #[test]
    fn test_holographic_update_file() {
        let fs = VersionedEmbrFS::new_holographic();

        // Create file
        let v1 = fs
            .write_file_holographic("update_test.txt", b"version 1", None)
            .unwrap();
        assert_eq!(v1, 0);

        // Update with correct version
        let v2 = fs
            .write_file_holographic("update_test.txt", b"version 2 is longer", Some(v1))
            .unwrap();
        assert_eq!(v2, 1);

        // Read back and verify
        let (content, version) = fs.read_file_holographic("update_test.txt").unwrap();
        assert_eq!(&content[..], b"version 2 is longer");
        assert_eq!(version, 1);
    }

    #[test]
    fn test_holographic_empty_file() {
        let fs = VersionedEmbrFS::new_holographic();

        fs.write_file_holographic("empty.txt", b"", None).unwrap();
        let (content, _) = fs.read_file_holographic("empty.txt").unwrap();

        assert!(content.is_empty());
    }

    #[test]
    fn test_enable_holographic_mode() {
        let mut fs = VersionedEmbrFS::new();
        assert!(!fs.is_holographic());

        fs.enable_holographic_mode();
        assert!(fs.is_holographic());
    }

    #[test]
    fn test_read_range_basic() {
        let fs = VersionedEmbrFS::new();

        // Create a file with known content
        let data = b"Hello, World! This is a test file for range queries.";
        fs.write_file("range_test.txt", data, None).unwrap();

        // Read specific ranges
        let (result, _) = fs.read_range("range_test.txt", 0, 5).unwrap();
        assert_eq!(&result[..], b"Hello");

        let (result, _) = fs.read_range("range_test.txt", 7, 6).unwrap();
        assert_eq!(&result[..], b"World!");

        // Read beyond file size (should return what's available)
        let (result, _) = fs.read_range("range_test.txt", 44, 100).unwrap();
        assert_eq!(&result[..], b"queries.");

        // Read at/beyond file end
        let (result, _) = fs.read_range("range_test.txt", 1000, 10).unwrap();
        assert!(result.is_empty());

        // Read zero length
        let (result, _) = fs.read_range("range_test.txt", 0, 0).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_range_not_found() {
        let fs = VersionedEmbrFS::new();
        let result = fs.read_range("nonexistent.txt", 0, 10);
        assert!(result.is_err());
    }
}
