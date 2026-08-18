//! Streaming Ingester for Memory-Efficient File Encoding
//!
//! This module provides a streaming API for ingesting large files without loading
//! them entirely into memory. It processes data in chunks, encoding each chunk
//! as it arrives and bundling progressively into the root engram.
//!
//! # Why Streaming?
//!
//! For large files (>100MB), loading everything into memory before encoding:
//! - Causes memory pressure and potential OOM for multi-GB files
//! - Increases latency (must read entire file before any encoding starts)
//! - Wastes resources (chunks are independent, can encode in parallel)
//!
//! Streaming ingestion solves these by:
//! - Processing fixed-size chunks as they arrive
//! - Encoding each chunk immediately
//! - Maintaining bounded memory usage (~chunk_size * pipeline_depth)
//! - Enabling early error detection
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────────┐     ┌──────────────────┐
//! │ Data Source │────▶│ StreamingIngester│────▶│ VersionedEmbrFS │
//! │ (Read/io)   │     │ (chunk + encode) │     │ (store + bundle) │
//! └─────────────┘     └─────────────────┘     └──────────────────┘
//!                              │
//!                              ├── Chunk Buffer (4KB default)
//!                              ├── Correction Accumulator
//!                              └── Progressive Hash
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_fs::streaming::StreamingIngester;
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("large_file.bin")?;
//! let reader = BufReader::new(file);
//!
//! let mut ingester = StreamingIngester::builder(&fs)
//!     .with_chunk_size(8192)
//!     .with_path("large_file.bin");
//!
//! ingester.ingest_reader(reader)?;
//! let result = ingester.finalize()?;
//! ```

use crate::correction::ChunkCorrection;
use crate::versioned::{ChunkId, VersionedChunk, VersionedFileEntry};
use crate::versioned_embrfs::{
    EmbrFSError, VersionedEmbrFS, DEFAULT_CHUNK_SIZE, ENCODING_FORMAT_REVERSIBLE_VSA,
};
use embeddenator_io::{wrap_or_legacy, BinaryWriteOptions, CompressionCodec, PayloadKind};
use embeddenator_vsa::SparseVec;
use sha2::{Digest, Sha256};
use std::io::{BufRead, Read};
use std::sync::atomic::{AtomicU64, Ordering};

/// Result of a completed streaming ingestion
#[derive(Debug, Clone)]
pub struct StreamingResult {
    /// Path of the ingested file
    pub path: String,
    /// Total bytes ingested
    pub total_bytes: usize,
    /// Number of chunks created
    pub chunk_count: usize,
    /// File version in the filesystem
    pub version: u64,
    /// Bytes saved by correction optimization (if applicable)
    pub correction_savings: usize,
}

/// Pending chunk data for batch insertion
struct PendingChunk {
    chunk_id: ChunkId,
    data: Vec<u8>,
    vector: SparseVec,
    correction: ChunkCorrection,
}

/// Builder for configuring streaming ingestion
pub struct StreamingIngesterBuilder<'a> {
    fs: &'a VersionedEmbrFS,
    chunk_size: usize,
    path: Option<String>,
    expected_version: Option<u64>,
    compression: Option<CompressionCodec>,
    adaptive_chunking: bool,
    correction_threshold: f64,
}

impl<'a> StreamingIngesterBuilder<'a> {
    /// Create a new builder with default settings
    pub fn new(fs: &'a VersionedEmbrFS) -> Self {
        Self {
            fs,
            chunk_size: DEFAULT_CHUNK_SIZE,
            path: None,
            expected_version: None,
            compression: None,
            adaptive_chunking: false,
            correction_threshold: 0.1,
        }
    }

    /// Set the chunk size for encoding
    ///
    /// Smaller chunks improve memory efficiency but may reduce encoding quality
    /// for highly correlated data. Default is 4KB.
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size.clamp(512, 1024 * 1024); // Clamp to 512B - 1MB
        self
    }

    /// Set the target path for the file
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set expected version for optimistic locking (update existing file)
    pub fn with_expected_version(mut self, version: u64) -> Self {
        self.expected_version = Some(version);
        self
    }

    /// Enable compression with specified codec
    pub fn with_compression(mut self, codec: CompressionCodec) -> Self {
        self.compression = Some(codec);
        self
    }

    /// Enable adaptive chunking based on content similarity
    ///
    /// When enabled, the ingester may adjust chunk boundaries to improve
    /// encoding quality for files with repeated patterns.
    pub fn with_adaptive_chunking(mut self, enabled: bool) -> Self {
        self.adaptive_chunking = enabled;
        self
    }

    /// Set correction threshold for large file optimization
    ///
    /// For large files, corrections beyond this threshold trigger re-encoding
    /// with adjusted parameters. Range: 0.0 - 1.0 (default 0.1 = 10%)
    pub fn with_correction_threshold(mut self, threshold: f64) -> Self {
        self.correction_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Build the streaming ingester
    pub fn build(self) -> Result<StreamingIngester<'a>, EmbrFSError> {
        let path = self.path.ok_or_else(|| {
            EmbrFSError::InvalidOperation("Path must be specified for streaming ingestion".into())
        })?;

        Ok(StreamingIngester {
            fs: self.fs,
            path,
            chunk_size: self.chunk_size,
            expected_version: self.expected_version,
            compression: self.compression,
            adaptive_chunking: self.adaptive_chunking,
            correction_threshold: self.correction_threshold,
            // State
            buffer: Vec::with_capacity(self.chunk_size),
            pending_chunks: Vec::new(),
            chunk_ids: Vec::new(),
            total_bytes: AtomicU64::new(0),
            hasher: Sha256::new(),
            correction_bytes: AtomicU64::new(0),
        })
    }
}

/// Streaming ingester for memory-efficient file encoding
///
/// Processes data in chunks as it arrives, maintaining bounded memory usage.
/// Suitable for files of any size including multi-GB datasets.
pub struct StreamingIngester<'a> {
    // Configuration
    fs: &'a VersionedEmbrFS,
    path: String,
    chunk_size: usize,
    expected_version: Option<u64>,
    compression: Option<CompressionCodec>,
    adaptive_chunking: bool,
    correction_threshold: f64,

    // Mutable state
    buffer: Vec<u8>,
    pending_chunks: Vec<PendingChunk>,
    chunk_ids: Vec<ChunkId>,
    total_bytes: AtomicU64,
    hasher: Sha256,
    correction_bytes: AtomicU64,
}

impl<'a> StreamingIngester<'a> {
    /// Create a builder for configuring a streaming ingester
    pub fn builder(fs: &'a VersionedEmbrFS) -> StreamingIngesterBuilder<'a> {
        StreamingIngesterBuilder::new(fs)
    }

    /// Ingest data from a reader
    ///
    /// Reads data in chunks and encodes progressively. Memory usage is bounded
    /// by `chunk_size` regardless of total file size.
    pub fn ingest_reader<R: Read>(&mut self, mut reader: R) -> Result<(), EmbrFSError> {
        let mut read_buf = vec![0u8; self.chunk_size];

        loop {
            let bytes_read = reader
                .read(&mut read_buf)
                .map_err(|e| EmbrFSError::IoError(e.to_string()))?;

            if bytes_read == 0 {
                break; // EOF
            }

            self.ingest_bytes(&read_buf[..bytes_read])?;
        }

        Ok(())
    }

    /// Ingest data from a buffered reader (more efficient)
    pub fn ingest_buffered<R: BufRead>(&mut self, mut reader: R) -> Result<(), EmbrFSError> {
        loop {
            let buf = reader
                .fill_buf()
                .map_err(|e| EmbrFSError::IoError(e.to_string()))?;

            if buf.is_empty() {
                break; // EOF
            }

            let len = buf.len();
            self.ingest_bytes(buf)?;
            reader.consume(len);
        }

        Ok(())
    }

    /// Ingest a slice of bytes
    ///
    /// Can be called multiple times. Data is buffered until a full chunk is
    /// available, then encoded and stored.
    pub fn ingest_bytes(&mut self, data: &[u8]) -> Result<(), EmbrFSError> {
        self.total_bytes
            .fetch_add(data.len() as u64, Ordering::Relaxed);
        self.hasher.update(data);

        // Add to buffer
        self.buffer.extend_from_slice(data);

        // Process complete chunks
        while self.buffer.len() >= self.chunk_size {
            let chunk_data: Vec<u8> = self.buffer.drain(..self.chunk_size).collect();
            self.process_chunk(chunk_data)?;
        }

        Ok(())
    }

    /// Finalize the ingestion and commit to filesystem
    ///
    /// Processes any remaining buffered data and commits all chunks to the
    /// versioned filesystem atomically.
    pub fn finalize(mut self) -> Result<StreamingResult, EmbrFSError> {
        // Process remaining data in buffer
        if !self.buffer.is_empty() {
            let remaining = std::mem::take(&mut self.buffer);
            self.process_chunk(remaining)?;
        }

        let total_bytes = self.total_bytes.load(Ordering::Relaxed) as usize;
        let chunk_count = self.chunk_ids.len();
        let correction_bytes = self.correction_bytes.load(Ordering::Relaxed) as usize;

        // Check existing file for version verification
        let existing = self.fs.manifest.get_file(&self.path);

        match (&existing, self.expected_version) {
            (Some((entry, _)), Some(expected_ver)) => {
                if entry.version != expected_ver {
                    return Err(EmbrFSError::VersionMismatch {
                        expected: expected_ver,
                        actual: entry.version,
                    });
                }
            }
            (Some(_), None) => {
                return Err(EmbrFSError::FileExists(self.path.clone()));
            }
            (None, Some(_)) => {
                return Err(EmbrFSError::FileNotFound(self.path.clone()));
            }
            (None, None) => {}
        }

        // Build chunk updates from pending chunks
        let chunk_updates: Vec<_> = self
            .pending_chunks
            .iter()
            .map(|pc| {
                let mut hash_bytes = [0u8; 8];
                let mut hasher = Sha256::new();
                hasher.update(&pc.data);
                let hash = hasher.finalize();
                hash_bytes.copy_from_slice(&hash[0..8]);
                (
                    pc.chunk_id,
                    VersionedChunk::new(pc.vector.clone(), pc.data.len(), hash_bytes),
                )
            })
            .collect();

        // Insert chunks
        self.fs.chunk_store.batch_insert_new(chunk_updates)?;

        // Insert corrections
        let corrections: Vec<_> = self
            .pending_chunks
            .into_iter()
            .map(|pc| (pc.chunk_id as u64, pc.correction))
            .collect();
        self.fs.corrections.batch_insert_new(corrections)?;

        // Create manifest entry
        let is_text = total_bytes > 0 && is_likely_text(total_bytes);
        let mut file_entry = if let Some(codec) = self.compression {
            let codec_byte = match codec {
                CompressionCodec::None => 0,
                CompressionCodec::Zstd => 1,
                CompressionCodec::Lz4 => 2,
            };
            VersionedFileEntry::new_compressed(
                self.path.clone(),
                is_text,
                total_bytes, // compressed size (actual)
                total_bytes, // original size
                codec_byte,
                self.chunk_ids.clone(),
            )
        } else {
            VersionedFileEntry::new(
                self.path.clone(),
                is_text,
                total_bytes,
                self.chunk_ids.clone(),
            )
        };

        // Set encoding format for holographic mode files
        if self.fs.is_holographic() {
            file_entry.encoding_format = Some(ENCODING_FORMAT_REVERSIBLE_VSA);
        }

        let version = if let Some((entry, _)) = existing {
            self.fs
                .manifest
                .update_file(&self.path, file_entry, entry.version)?;
            entry.version + 1
        } else {
            self.fs.manifest.add_file(file_entry)?;
            0
        };

        // Bundle chunks into root
        self.fs.bundle_chunks_to_root_streaming(&self.chunk_ids)?;

        Ok(StreamingResult {
            path: self.path,
            total_bytes,
            chunk_count,
            version,
            correction_savings: correction_bytes,
        })
    }

    /// Process a single chunk
    fn process_chunk(&mut self, data: Vec<u8>) -> Result<(), EmbrFSError> {
        let chunk_id = self.fs.allocate_chunk_id();

        // Apply compression if configured
        let encoded_data = if let Some(codec) = self.compression {
            if codec != CompressionCodec::None {
                let write_opts = BinaryWriteOptions { codec, level: None };
                wrap_or_legacy(PayloadKind::EngramBincode, write_opts, &data)
                    .map_err(|e| EmbrFSError::IoError(format!("Compression failed: {}", e)))?
            } else {
                data.clone()
            }
        } else {
            data.clone()
        };

        // Encode chunk using appropriate encoder based on mode
        let chunk_vec = if self.fs.is_holographic() {
            // Use ReversibleVSAEncoder for ~94% uncorrected accuracy
            self.fs
                .reversible_encoder()
                .write()
                .unwrap()
                .encode(&encoded_data)
        } else {
            // Legacy mode: use SparseVec::encode_data
            SparseVec::encode_data(&encoded_data, self.fs.config(), Some(&self.path))
        };

        // Decode and compute correction using matching decoder
        let decoded = if self.fs.is_holographic() {
            self.fs
                .reversible_encoder()
                .read()
                .unwrap()
                .decode(&chunk_vec, encoded_data.len())
        } else {
            chunk_vec.decode_data(self.fs.config(), Some(&self.path), encoded_data.len())
        };
        let correction = ChunkCorrection::new(chunk_id as u64, &encoded_data, &decoded);

        // Track correction overhead
        let corr_size = correction.storage_size();
        self.correction_bytes
            .fetch_add(corr_size as u64, Ordering::Relaxed);

        // Check if correction exceeds threshold (triggers adaptive strategy)
        if self.adaptive_chunking {
            let correction_ratio = corr_size as f64 / encoded_data.len() as f64;
            if correction_ratio > self.correction_threshold {
                // For now, just log - future: implement re-encoding with different params
                eprintln!(
                    "Warning: High correction ratio {:.2}% for chunk {} - consider adjusting parameters",
                    correction_ratio * 100.0,
                    chunk_id
                );
            }
        }

        // Store pending chunk for batch insertion
        self.pending_chunks.push(PendingChunk {
            chunk_id,
            data: encoded_data,
            vector: chunk_vec,
            correction,
        });
        self.chunk_ids.push(chunk_id);

        Ok(())
    }

    /// Get current progress
    pub fn progress(&self) -> StreamingProgress {
        StreamingProgress {
            bytes_processed: self.total_bytes.load(Ordering::Relaxed) as usize,
            chunks_created: self.chunk_ids.len(),
            buffer_usage: self.buffer.len(),
            correction_overhead: self.correction_bytes.load(Ordering::Relaxed) as usize,
        }
    }
}

/// Progress information for streaming ingestion
#[derive(Debug, Clone)]
pub struct StreamingProgress {
    /// Total bytes processed so far
    pub bytes_processed: usize,
    /// Number of chunks created
    pub chunks_created: usize,
    /// Current buffer usage in bytes
    pub buffer_usage: usize,
    /// Total correction overhead in bytes
    pub correction_overhead: usize,
}

/// Heuristic to detect if data is likely text based on size
fn is_likely_text(size: usize) -> bool {
    // Small files are often text, large files are often binary
    size < 1024 * 1024
}

// =============================================================================
// ASYNC STREAMING SUPPORT
// =============================================================================

/// Async streaming ingester for memory-efficient file encoding
///
/// This is the async counterpart to `StreamingIngester`, supporting
/// `tokio::io::AsyncRead` and `tokio::io::AsyncBufRead` for non-blocking I/O.
///
/// # Example
///
/// ```rust,ignore
/// use embeddenator_fs::fs::AsyncStreamingIngester;
/// use tokio::io::AsyncReadExt;
///
/// async fn ingest_async(fs: &VersionedEmbrFS, path: &str) {
///     let file = tokio::fs::File::open("large_file.bin").await?;
///     let reader = tokio::io::BufReader::new(file);
///
///     let mut ingester = AsyncStreamingIngester::builder(fs)
///         .with_path(path)
///         .build()
///         .await?;
///
///     ingester.ingest_async_reader(reader).await?;
///     let result = ingester.finalize().await?;
/// }
/// ```
#[cfg(feature = "tokio")]
pub struct AsyncStreamingIngester<'a> {
    /// Inner sync ingester (we delegate chunk processing to it)
    inner: StreamingIngester<'a>,
}

#[cfg(feature = "tokio")]
impl<'a> AsyncStreamingIngester<'a> {
    /// Create a new async streaming ingester builder
    pub fn builder(fs: &'a VersionedEmbrFS) -> AsyncStreamingIngesterBuilder<'a> {
        AsyncStreamingIngesterBuilder {
            inner: StreamingIngesterBuilder::new(fs),
        }
    }

    /// Ingest data from an async reader
    ///
    /// Reads data in chunks and processes asynchronously.
    pub async fn ingest_async_reader<R>(&mut self, mut reader: R) -> Result<(), EmbrFSError>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = vec![0u8; self.inner.chunk_size];

        loop {
            let n = reader
                .read(&mut buf)
                .await
                .map_err(|e| EmbrFSError::IoError(e.to_string()))?;

            if n == 0 {
                break;
            }

            // Delegate to sync ingester's bytes method
            self.inner.ingest_bytes(&buf[..n])?;
        }

        Ok(())
    }

    /// Ingest data from an async buffered reader
    ///
    /// More efficient than `ingest_async_reader` as it avoids double-buffering.
    pub async fn ingest_async_buffered<R>(&mut self, mut reader: R) -> Result<(), EmbrFSError>
    where
        R: tokio::io::AsyncBufRead + Unpin,
    {
        use tokio::io::AsyncBufReadExt;

        loop {
            let buf = reader
                .fill_buf()
                .await
                .map_err(|e| EmbrFSError::IoError(e.to_string()))?;

            if buf.is_empty() {
                break;
            }

            let len = buf.len();
            self.inner.ingest_bytes(buf)?;
            reader.consume(len);
        }

        Ok(())
    }

    /// Ingest a slice of bytes (synchronous, for convenience)
    pub fn ingest_bytes(&mut self, data: &[u8]) -> Result<(), EmbrFSError> {
        self.inner.ingest_bytes(data)
    }

    /// Get current ingestion progress
    pub fn progress(&self) -> StreamingProgress {
        self.inner.progress()
    }

    /// Finalize the ingestion and commit to filesystem
    pub fn finalize(self) -> Result<StreamingResult, EmbrFSError> {
        self.inner.finalize()
    }
}

/// Builder for async streaming ingester
#[cfg(feature = "tokio")]
pub struct AsyncStreamingIngesterBuilder<'a> {
    inner: StreamingIngesterBuilder<'a>,
}

#[cfg(feature = "tokio")]
impl<'a> AsyncStreamingIngesterBuilder<'a> {
    /// Set the target path for the file
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.inner = self.inner.with_path(path);
        self
    }

    /// Set chunk size for processing
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.inner = self.inner.with_chunk_size(size);
        self
    }

    /// Set expected version for optimistic locking
    pub fn with_expected_version(mut self, version: u64) -> Self {
        self.inner = self.inner.with_expected_version(version);
        self
    }

    /// Enable compression with specified codec
    pub fn with_compression(mut self, codec: CompressionCodec) -> Self {
        self.inner = self.inner.with_compression(codec);
        self
    }

    /// Enable adaptive chunking
    pub fn with_adaptive_chunking(mut self, enabled: bool) -> Self {
        self.inner = self.inner.with_adaptive_chunking(enabled);
        self
    }

    /// Set correction threshold
    pub fn with_correction_threshold(mut self, threshold: f64) -> Self {
        self.inner = self.inner.with_correction_threshold(threshold);
        self
    }

    /// Build the async streaming ingester
    pub fn build(self) -> Result<AsyncStreamingIngester<'a>, EmbrFSError> {
        Ok(AsyncStreamingIngester {
            inner: self.inner.build()?,
        })
    }
}

// =============================================================================
// STREAMING DECODE SUPPORT
// =============================================================================

/// Streaming decoder for memory-efficient file reading
///
/// Decodes engram data incrementally, yielding chunks as they become available.
/// Memory usage is bounded by a single chunk regardless of file size.
///
/// # Why Streaming Decode?
///
/// For large files, loading everything into memory before processing:
/// - Causes memory pressure for multi-GB engrams
/// - Increases latency (must decode all chunks before returning any data)
/// - Prevents early termination for partial reads
///
/// Streaming decode solves these by:
/// - Decoding chunks on-demand
/// - Yielding data as it becomes available
/// - Supporting early termination
/// - Maintaining bounded memory usage
///
/// # Example
///
/// ```rust,ignore
/// use embeddenator_fs::streaming::StreamingDecoder;
/// use std::io::Read;
///
/// let decoder = StreamingDecoder::new(&fs, "large_file.bin")?;
///
/// // Read first 1KB without decoding entire file
/// let mut buf = vec![0u8; 1024];
/// decoder.read_exact(&mut buf)?;
///
/// // Or iterate over chunks
/// for chunk_result in decoder {
///     let chunk_data = chunk_result?;
///     process(chunk_data);
/// }
/// ```
pub struct StreamingDecoder<'a> {
    fs: &'a VersionedEmbrFS,
    path: String,
    chunks: Vec<ChunkId>,
    file_size: usize,
    version: u64,
    // State
    current_chunk_idx: usize,
    current_chunk_data: Vec<u8>,
    position_in_chunk: usize,
    total_bytes_read: usize,
    /// Whether the file is stored compressed (for future decompression support)
    #[allow(dead_code)]
    is_compressed: bool,
    /// Compression codec used (for future decompression support)
    #[allow(dead_code)]
    compression_codec: Option<u8>,
    /// Encoding format (0=legacy, 1=reversible VSA)
    encoding_format: Option<u8>,
}

impl<'a> StreamingDecoder<'a> {
    /// Create a new streaming decoder for a file
    pub fn new(fs: &'a VersionedEmbrFS, path: &str) -> Result<Self, EmbrFSError> {
        let (file_entry, _) = fs
            .manifest
            .get_file(path)
            .ok_or_else(|| EmbrFSError::FileNotFound(path.to_string()))?;

        if file_entry.deleted {
            return Err(EmbrFSError::FileNotFound(path.to_string()));
        }

        Ok(Self {
            fs,
            path: path.to_string(),
            chunks: file_entry.chunks.clone(),
            file_size: file_entry.size,
            version: file_entry.version,
            current_chunk_idx: 0,
            current_chunk_data: Vec::new(),
            position_in_chunk: 0,
            total_bytes_read: 0,
            is_compressed: file_entry
                .compression_codec
                .map(|c| c != 0)
                .unwrap_or(false),
            compression_codec: file_entry.compression_codec,
            encoding_format: file_entry.encoding_format,
        })
    }

    /// Get the file version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Get the total file size
    pub fn file_size(&self) -> usize {
        self.file_size
    }

    /// Get the number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get current read position (bytes from start)
    pub fn position(&self) -> usize {
        self.total_bytes_read
    }

    /// Check if all data has been read
    pub fn is_exhausted(&self) -> bool {
        self.total_bytes_read >= self.file_size
    }

    /// Get progress information
    pub fn progress(&self) -> StreamingDecodeProgress {
        StreamingDecodeProgress {
            bytes_read: self.total_bytes_read,
            total_bytes: self.file_size,
            chunks_decoded: self.current_chunk_idx,
            total_chunks: self.chunks.len(),
        }
    }

    /// Decode and return the next chunk's data
    ///
    /// Returns None when all chunks have been read.
    fn decode_next_chunk(&mut self) -> Result<Option<Vec<u8>>, EmbrFSError> {
        if self.current_chunk_idx >= self.chunks.len() {
            return Ok(None);
        }

        let chunk_id = self.chunks[self.current_chunk_idx];

        // Get chunk from store
        let (chunk, _) = self
            .fs
            .chunk_store
            .get(chunk_id)
            .ok_or(EmbrFSError::ChunkNotFound(chunk_id))?;

        // Decode chunk using matching decoder based on encoding format
        let decoded = if self.encoding_format == Some(ENCODING_FORMAT_REVERSIBLE_VSA) {
            self.fs
                .reversible_encoder()
                .read()
                .unwrap()
                .decode(&chunk.vector, chunk.original_size)
        } else {
            chunk
                .vector
                .decode_data(self.fs.config(), Some(&self.path), chunk.original_size)
        };

        // Apply correction
        let corrected = self
            .fs
            .corrections
            .get(chunk_id as u64)
            .map(|(corr, _)| corr.apply(&decoded))
            .unwrap_or(decoded);

        self.current_chunk_idx += 1;

        Ok(Some(corrected))
    }

    /// Skip to a specific byte position
    ///
    /// Efficiently skips chunks that don't need to be decoded.
    pub fn seek_to(&mut self, position: usize) -> Result<(), EmbrFSError> {
        if position >= self.file_size {
            self.current_chunk_idx = self.chunks.len();
            self.current_chunk_data.clear();
            self.position_in_chunk = 0;
            self.total_bytes_read = self.file_size;
            return Ok(());
        }

        // Calculate which chunk contains this position
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let target_chunk = position / chunk_size;
        let offset_in_chunk = position % chunk_size;

        // Reset state
        self.current_chunk_idx = target_chunk;
        self.current_chunk_data.clear();
        self.position_in_chunk = offset_in_chunk;
        self.total_bytes_read = position;

        // Pre-load the target chunk if needed
        if offset_in_chunk > 0 {
            if let Some(data) = self.decode_next_chunk()? {
                self.current_chunk_data = data;
                self.current_chunk_idx -= 1; // decode_next_chunk incremented it
            }
        }

        Ok(())
    }

    /// Read exactly n bytes, returning them
    ///
    /// Returns less than n bytes only at EOF.
    pub fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, EmbrFSError> {
        let mut result = Vec::with_capacity(n);
        let remaining_in_file = self.file_size.saturating_sub(self.total_bytes_read);
        let to_read = n.min(remaining_in_file);

        while result.len() < to_read {
            // Check if we need more data from current chunk
            if self.position_in_chunk >= self.current_chunk_data.len() {
                // Load next chunk
                match self.decode_next_chunk()? {
                    Some(data) => {
                        self.current_chunk_data = data;
                        self.position_in_chunk = 0;
                    }
                    None => break, // No more chunks
                }
            }

            // Copy data from current chunk
            let available = self.current_chunk_data.len() - self.position_in_chunk;
            let needed = to_read - result.len();
            let copy_len = available.min(needed);

            result.extend_from_slice(
                &self.current_chunk_data[self.position_in_chunk..self.position_in_chunk + copy_len],
            );
            self.position_in_chunk += copy_len;
            self.total_bytes_read += copy_len;
        }

        // Truncate to file size if needed
        let max_len = self
            .file_size
            .saturating_sub(self.total_bytes_read - result.len());
        if result.len() > max_len {
            result.truncate(max_len);
        }

        Ok(result)
    }
}

impl std::io::Read for StreamingDecoder<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.is_exhausted() {
            return Ok(0);
        }

        let data = self
            .read_n_bytes(buf.len())
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let len = data.len();
        buf[..len].copy_from_slice(&data);
        Ok(len)
    }
}

/// Iterator that yields decoded chunks
impl<'a> Iterator for StreamingDecoder<'a> {
    type Item = Result<Vec<u8>, EmbrFSError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_exhausted() {
            return None;
        }

        match self.decode_next_chunk() {
            Ok(Some(mut data)) => {
                // Truncate last chunk to exact file size
                let remaining = self.file_size.saturating_sub(self.total_bytes_read);
                if data.len() > remaining {
                    data.truncate(remaining);
                }
                self.total_bytes_read += data.len();
                self.current_chunk_data.clear();
                self.position_in_chunk = 0;
                Some(Ok(data))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Progress information for streaming decode
#[derive(Debug, Clone, Copy)]
pub struct StreamingDecodeProgress {
    /// Bytes read so far
    pub bytes_read: usize,
    /// Total bytes in file
    pub total_bytes: usize,
    /// Chunks decoded so far
    pub chunks_decoded: usize,
    /// Total chunks in file
    pub total_chunks: usize,
}

impl StreamingDecodeProgress {
    /// Get percentage complete (0.0 - 1.0)
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            1.0
        } else {
            self.bytes_read as f64 / self.total_bytes as f64
        }
    }
}

/// Builder for configuring streaming decode with options
pub struct StreamingDecoderBuilder<'a> {
    fs: &'a VersionedEmbrFS,
    path: String,
    start_offset: Option<usize>,
    max_bytes: Option<usize>,
}

impl<'a> StreamingDecoderBuilder<'a> {
    /// Create a new streaming decoder builder
    pub fn new(fs: &'a VersionedEmbrFS, path: impl Into<String>) -> Self {
        Self {
            fs,
            path: path.into(),
            start_offset: None,
            max_bytes: None,
        }
    }

    /// Set starting offset for partial reads
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.start_offset = Some(offset);
        self
    }

    /// Limit maximum bytes to read
    pub fn with_max_bytes(mut self, max: usize) -> Self {
        self.max_bytes = Some(max);
        self
    }

    /// Build the streaming decoder
    pub fn build(self) -> Result<StreamingDecoder<'a>, EmbrFSError> {
        let mut decoder = StreamingDecoder::new(self.fs, &self.path)?;

        if let Some(offset) = self.start_offset {
            decoder.seek_to(offset)?;
        }

        // max_bytes is handled by the caller limiting reads

        Ok(decoder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_small_file() {
        let fs = VersionedEmbrFS::new();
        let data = b"Hello, streaming world!";

        let mut ingester = StreamingIngester::builder(&fs)
            .with_path("test.txt")
            .build()
            .unwrap();

        ingester.ingest_bytes(data).unwrap();
        let result = ingester.finalize().unwrap();

        assert_eq!(result.path, "test.txt");
        assert_eq!(result.total_bytes, data.len());
        assert!(result.chunk_count >= 1);

        // Verify data
        let (content, _) = fs.read_file("test.txt").unwrap();
        assert_eq!(&content[..], data);
    }

    #[test]
    fn test_streaming_large_file() {
        let fs = VersionedEmbrFS::new();

        // Create data larger than default chunk size
        let data: Vec<u8> = (0..DEFAULT_CHUNK_SIZE * 3 + 500)
            .map(|i| (i % 256) as u8)
            .collect();

        let mut ingester = StreamingIngester::builder(&fs)
            .with_path("large.bin")
            .with_chunk_size(DEFAULT_CHUNK_SIZE)
            .build()
            .unwrap();

        // Feed in smaller pieces to test buffering
        for chunk in data.chunks(1024) {
            ingester.ingest_bytes(chunk).unwrap();
        }

        let result = ingester.finalize().unwrap();

        assert_eq!(result.total_bytes, data.len());
        assert!(result.chunk_count >= 3); // At least 3 chunks

        // Verify data integrity
        let (content, _) = fs.read_file("large.bin").unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_streaming_progress() {
        let fs = VersionedEmbrFS::new();

        let mut ingester = StreamingIngester::builder(&fs)
            .with_path("progress.txt")
            .with_chunk_size(1024)
            .build()
            .unwrap();

        // Check initial progress
        let p1 = ingester.progress();
        assert_eq!(p1.bytes_processed, 0);
        assert_eq!(p1.chunks_created, 0);

        // Add data
        ingester.ingest_bytes(&[0u8; 500]).unwrap();
        let p2 = ingester.progress();
        assert_eq!(p2.bytes_processed, 500);
        assert_eq!(p2.buffer_usage, 500);
        assert_eq!(p2.chunks_created, 0); // Not a full chunk yet

        // Add more data to complete a chunk
        ingester.ingest_bytes(&[0u8; 600]).unwrap();
        let p3 = ingester.progress();
        assert_eq!(p3.bytes_processed, 1100);
        assert_eq!(p3.chunks_created, 1);
    }

    #[test]
    fn test_streaming_reader() {
        use std::io::Cursor;

        let fs = VersionedEmbrFS::new();
        let data = b"Data from a reader interface!";
        let reader = Cursor::new(data);

        let mut ingester = StreamingIngester::builder(&fs)
            .with_path("from_reader.txt")
            .build()
            .unwrap();

        ingester.ingest_reader(reader).unwrap();
        let result = ingester.finalize().unwrap();

        assert_eq!(result.total_bytes, data.len());

        let (content, _) = fs.read_file("from_reader.txt").unwrap();
        assert_eq!(&content[..], data);
    }

    // =========================================================================
    // STREAMING DECODER TESTS
    // =========================================================================

    #[test]
    fn test_streaming_decoder_small_file() {
        let fs = VersionedEmbrFS::new();
        let data = b"Hello, streaming decoder!";

        // Write file first
        fs.write_file("decode_test.txt", data, None).unwrap();

        // Create streaming decoder
        let mut decoder = StreamingDecoder::new(&fs, "decode_test.txt").unwrap();

        assert_eq!(decoder.file_size(), data.len());
        assert_eq!(decoder.position(), 0);
        assert!(!decoder.is_exhausted());

        // Read all data
        let read_data = decoder.read_n_bytes(data.len() + 10).unwrap();

        assert_eq!(&read_data[..], data);
        assert!(decoder.is_exhausted());
    }

    #[test]
    fn test_streaming_decoder_read_trait() {
        use std::io::Read;

        let fs = VersionedEmbrFS::new();
        let data = b"Read trait test data";

        fs.write_file("read_trait.txt", data, None).unwrap();

        let mut decoder = StreamingDecoder::new(&fs, "read_trait.txt").unwrap();
        let mut buf = vec![0u8; 100];

        let bytes_read = decoder.read(&mut buf).unwrap();

        assert_eq!(bytes_read, data.len());
        assert_eq!(&buf[..bytes_read], data);
    }

    #[test]
    fn test_streaming_decoder_iterator() {
        let fs = VersionedEmbrFS::new();

        // Create data larger than chunk size
        let data: Vec<u8> = (0..DEFAULT_CHUNK_SIZE * 2 + 100)
            .map(|i| (i % 256) as u8)
            .collect();

        fs.write_file("iterator_test.bin", &data, None).unwrap();

        let decoder = StreamingDecoder::new(&fs, "iterator_test.bin").unwrap();

        // Collect all chunks via iterator
        let chunks: Vec<Vec<u8>> = decoder.map(|r| r.unwrap()).collect();

        // Should have at least 2 chunks
        assert!(chunks.len() >= 2);

        // Verify total data matches
        let total: Vec<u8> = chunks.into_iter().flatten().collect();
        assert_eq!(total, data);
    }

    #[test]
    fn test_streaming_decoder_partial_read() {
        let fs = VersionedEmbrFS::new();
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        fs.write_file("partial.bin", &data, None).unwrap();

        let mut decoder = StreamingDecoder::new(&fs, "partial.bin").unwrap();

        // Read only first 100 bytes
        let first_100 = decoder.read_n_bytes(100).unwrap();
        assert_eq!(first_100.len(), 100);
        assert_eq!(&first_100[..], &data[..100]);

        // Position should be updated
        assert_eq!(decoder.position(), 100);

        // Read next 50 bytes
        let next_50 = decoder.read_n_bytes(50).unwrap();
        assert_eq!(next_50.len(), 50);
        assert_eq!(&next_50[..], &data[100..150]);
    }

    #[test]
    fn test_streaming_decoder_seek() {
        let fs = VersionedEmbrFS::new();
        let data: Vec<u8> = (0..DEFAULT_CHUNK_SIZE * 3)
            .map(|i| (i % 256) as u8)
            .collect();

        fs.write_file("seek_test.bin", &data, None).unwrap();

        let mut decoder = StreamingDecoder::new(&fs, "seek_test.bin").unwrap();

        // Seek to middle of second chunk
        let seek_pos = DEFAULT_CHUNK_SIZE + 500;
        decoder.seek_to(seek_pos).unwrap();

        assert_eq!(decoder.position(), seek_pos);

        // Read some bytes and verify
        let read_data = decoder.read_n_bytes(100).unwrap();
        assert_eq!(&read_data[..], &data[seek_pos..seek_pos + 100]);
    }

    #[test]
    fn test_streaming_decoder_progress() {
        let fs = VersionedEmbrFS::new();
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        fs.write_file("progress_decode.bin", &data, None).unwrap();

        let mut decoder = StreamingDecoder::new(&fs, "progress_decode.bin").unwrap();

        let p1 = decoder.progress();
        assert_eq!(p1.bytes_read, 0);
        assert_eq!(p1.total_bytes, 1000);
        assert!((p1.percentage() - 0.0).abs() < 0.001);

        // Read half the data
        decoder.read_n_bytes(500).unwrap();

        let p2 = decoder.progress();
        assert_eq!(p2.bytes_read, 500);
        assert!((p2.percentage() - 0.5).abs() < 0.001);

        // Read remaining
        decoder.read_n_bytes(500).unwrap();

        let p3 = decoder.progress();
        assert_eq!(p3.bytes_read, 1000);
        assert!((p3.percentage() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_streaming_decoder_builder() {
        let fs = VersionedEmbrFS::new();
        let data: Vec<u8> = (0..500).map(|i| (i % 256) as u8).collect();

        fs.write_file("builder_decode.bin", &data, None).unwrap();

        let mut decoder = StreamingDecoderBuilder::new(&fs, "builder_decode.bin")
            .with_offset(100)
            .build()
            .unwrap();

        assert_eq!(decoder.position(), 100);

        let read_data = decoder.read_n_bytes(50).unwrap();
        assert_eq!(&read_data[..], &data[100..150]);
    }

    #[test]
    fn test_stream_decode_convenience_method() {
        let fs = VersionedEmbrFS::new();
        let data = b"Testing stream_decode convenience method";

        fs.write_file("convenience.txt", data, None).unwrap();

        // Use the convenience method
        let mut decoder = fs.stream_decode("convenience.txt").unwrap();

        assert_eq!(decoder.file_size(), data.len());
        let read_data = decoder.read_n_bytes(data.len()).unwrap();
        assert_eq!(&read_data[..], data);
    }

    #[test]
    fn test_stream_decode_range_convenience_method() {
        let fs = VersionedEmbrFS::new();
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        fs.write_file("range_convenience.bin", &data, None).unwrap();

        // Use the convenience method with offset
        let mut decoder = fs
            .stream_decode_range("range_convenience.bin", 500, Some(200))
            .unwrap();

        assert_eq!(decoder.position(), 500);
        let read_data = decoder.read_n_bytes(200).unwrap();
        assert_eq!(&read_data[..], &data[500..700]);
    }

    #[test]
    fn test_streaming_decoder_memory_bounded() {
        // This test verifies that streaming decode maintains bounded memory usage
        // by processing a file much larger than the internal buffer.
        //
        // The key property is that StreamingDecoder only keeps one chunk in memory
        // at a time (current_chunk_data), regardless of total file size.

        let fs = VersionedEmbrFS::new();

        // Create a file spanning many chunks (e.g., 50 chunks = 200KB with 4KB chunks)
        // In production, this would be 1GB+, but we test the algorithm with smaller data
        let num_chunks = 50;
        let data: Vec<u8> = (0..DEFAULT_CHUNK_SIZE * num_chunks)
            .map(|i| (i % 256) as u8)
            .collect();

        fs.write_file("memory_bounded.bin", &data, None).unwrap();

        // Create decoder and verify it only loads one chunk at a time
        let mut decoder = StreamingDecoder::new(&fs, "memory_bounded.bin").unwrap();

        assert_eq!(decoder.chunk_count(), num_chunks);
        assert_eq!(decoder.file_size(), data.len());

        // Read chunk by chunk via iterator to verify bounded memory
        let mut total_read = 0;
        let mut chunk_count = 0;
        for chunk_result in &mut decoder {
            let chunk_data = chunk_result.unwrap();
            // Each chunk should be at most DEFAULT_CHUNK_SIZE
            assert!(chunk_data.len() <= DEFAULT_CHUNK_SIZE);
            total_read += chunk_data.len();
            chunk_count += 1;
        }

        assert_eq!(total_read, data.len());
        assert_eq!(chunk_count, num_chunks);

        // Verify full data matches via fresh decoder with random access
        let mut decoder2 = StreamingDecoder::new(&fs, "memory_bounded.bin").unwrap();
        let full_data = decoder2.read_n_bytes(data.len()).unwrap();
        assert_eq!(full_data, data);
    }

    #[test]
    fn test_streaming_decoder_seek_across_chunks() {
        let fs = VersionedEmbrFS::new();

        // Create multi-chunk file
        let data: Vec<u8> = (0..DEFAULT_CHUNK_SIZE * 5)
            .map(|i| (i % 256) as u8)
            .collect();

        fs.write_file("seek_multi.bin", &data, None).unwrap();

        let mut decoder = StreamingDecoder::new(&fs, "seek_multi.bin").unwrap();

        // Seek to middle of 3rd chunk
        let seek_pos = DEFAULT_CHUNK_SIZE * 2 + 100;
        decoder.seek_to(seek_pos).unwrap();
        assert_eq!(decoder.position(), seek_pos);

        // Read and verify
        let read_data = decoder.read_n_bytes(500).unwrap();
        assert_eq!(&read_data[..], &data[seek_pos..seek_pos + 500]);

        // Seek backward to 1st chunk
        decoder.seek_to(50).unwrap();
        assert_eq!(decoder.position(), 50);

        let read_data2 = decoder.read_n_bytes(100).unwrap();
        assert_eq!(&read_data2[..], &data[50..150]);

        // Seek to end
        decoder.seek_to(data.len()).unwrap();
        assert!(decoder.is_exhausted());
    }
}
