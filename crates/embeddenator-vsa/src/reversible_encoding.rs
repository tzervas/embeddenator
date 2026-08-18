//! Reversible Position-Aware VSA Encoding
//!
//! This module implements true holographic storage where data can be
//! reconstructed from the VSA vector. With chunked encoding, typical
//! accuracy is 90-95% before correction layer application.
//!
//! Chunk size is configurable: smaller chunks (8-64 bytes) provide higher
//! accuracy per chunk but more overhead; embeddenator-fs uses 64 bytes
//! as a balance between accuracy and efficiency.
//!
//! # Architecture
//!
//! ```text
//! For each byte at position i:
//!   encoded[i] = bind(position_vector[i], byte_vector[data[i]])
//!
//! Memory = bundle(encoded[0], encoded[1], ..., encoded[n])
//!
//! To retrieve byte at position i:
//!   query = bind(position_vector[i], Memory)
//!   byte = argmax_b(cosine(query, byte_vector[b]))
//! ```
//!
//! This uses the fundamental VSA operations:
//! - **Bind**: Creates a composite that's dissimilar to both inputs but can be unbound
//! - **Bundle**: Superimposes vectors while preserving retrievability
//! - **Unbind**: Reverses bind to retrieve the bound content
//!
//! # Why This Works
//!
//! - Each (position, byte) pair has a unique representation
//! - Bundle creates holographic superposition of all pairs
//! - Unbind + similarity search retrieves the original byte
//! - No information loss from collisions

use crate::vsa::{SparseVec, DIM};
use rayon::prelude::*;
use sha2::{Digest, Sha256};

/// Maximum file size for position-aware encoding (4MB in 64-byte chunks)
pub const MAX_POSITIONS: usize = 65536;

/// Reversible encoder using position-aware VSA binding
pub struct ReversibleVSAEncoder {
    /// Basis vectors for each byte value (0-255)
    byte_vectors: Vec<SparseVec>,
    /// Basis vectors for each position (0 to MAX_POSITIONS-1)
    position_vectors: Vec<SparseVec>,
    /// Dimensionality
    dim: usize,
}

impl ReversibleVSAEncoder {
    /// Create a new reversible encoder
    pub fn new() -> Self {
        Self::with_dim(DIM)
    }

    /// Create a new reversible encoder with custom dimensionality
    pub fn with_dim(dim: usize) -> Self {
        let mut encoder = Self {
            byte_vectors: Vec::with_capacity(256),
            position_vectors: Vec::with_capacity(MAX_POSITIONS),
            dim,
        };
        encoder.initialize_basis_vectors();
        encoder
    }

    /// Initialize basis vectors for bytes and positions
    fn initialize_basis_vectors(&mut self) {
        // Create deterministic byte vectors
        for byte_val in 0u8..=255 {
            let seed = Self::hash_to_seed(b"byte", &[byte_val]);
            self.byte_vectors
                .push(SparseVec::from_seed(&seed, self.dim));
        }

        // Create deterministic position vectors (lazy - create on demand up to max)
        // For now, pre-create a reasonable number
        for pos in 0..4096 {
            let seed = Self::hash_to_seed(b"position", &(pos as u64).to_le_bytes());
            self.position_vectors
                .push(SparseVec::from_seed(&seed, self.dim));
        }
    }

    /// Hash a prefix and value to a 32-byte seed
    fn hash_to_seed(prefix: &[u8], value: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"embeddenator:reversible:v1:");
        hasher.update(prefix);
        hasher.update(b":");
        hasher.update(value);
        hasher.finalize().into()
    }

    /// Get or create position vector for a given position
    ///
    /// # Panics
    ///
    /// Panics if `pos >= MAX_POSITIONS` (65536). Use `ensure_positions` with
    /// proper bounds checking before calling this method with untrusted input.
    fn get_position_vector(&mut self, pos: usize) -> &SparseVec {
        assert!(
            pos < MAX_POSITIONS,
            "Position {} exceeds MAX_POSITIONS ({})",
            pos,
            MAX_POSITIONS
        );

        while pos >= self.position_vectors.len() {
            let new_pos = self.position_vectors.len();
            let seed = Self::hash_to_seed(b"position", &(new_pos as u64).to_le_bytes());
            self.position_vectors
                .push(SparseVec::from_seed(&seed, self.dim));
        }
        &self.position_vectors[pos]
    }

    /// Encode a byte at a specific position
    ///
    /// Returns bind(position_vector, byte_vector)
    fn encode_byte_at_position(&self, byte: u8, position: usize) -> SparseVec {
        let byte_vec = &self.byte_vectors[byte as usize];
        let pos_vec = &self.position_vectors[position % self.position_vectors.len()];
        byte_vec.bind(pos_vec)
    }

    /// Encode data into a holographic representation
    ///
    /// Returns a single SparseVec that contains all the data holographically.
    pub fn encode(&mut self, data: &[u8]) -> SparseVec {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Ensure we have enough position vectors
        let _ = self.get_position_vector(data.len().saturating_sub(1));

        // Encode each byte and bundle them together
        let mut result = self.encode_byte_at_position(data[0], 0);

        for (pos, &byte) in data.iter().enumerate().skip(1) {
            let encoded_byte = self.encode_byte_at_position(byte, pos);
            result = result.bundle(&encoded_byte);
        }

        result
    }

    /// Encode data in chunks for better accuracy with large files
    ///
    /// Returns a vector of SparseVecs, one per chunk.
    ///
    /// # Panics
    /// Panics if `chunk_size` is 0.
    pub fn encode_chunked(&mut self, data: &[u8], chunk_size: usize) -> Vec<SparseVec> {
        assert!(chunk_size > 0, "chunk_size must be > 0");
        data.chunks(chunk_size)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let offset = chunk_idx * chunk_size;
                self.encode_with_offset(chunk, offset)
            })
            .collect()
    }

    /// Encode data with a position offset
    fn encode_with_offset(&mut self, data: &[u8], offset: usize) -> SparseVec {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Ensure we have enough position vectors
        let _ = self.get_position_vector(offset + data.len().saturating_sub(1));

        // Encode each byte and bundle
        let mut result = self.encode_byte_at_position(data[0], offset);

        for (i, &byte) in data.iter().enumerate().skip(1) {
            let encoded_byte = self.encode_byte_at_position(byte, offset + i);
            result = result.bundle(&encoded_byte);
        }

        result
    }

    /// Decode data from a holographic representation
    ///
    /// Uses bind + similarity search to retrieve each byte.
    pub fn decode(&self, encoded: &SparseVec, length: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(length);

        for pos in 0..length {
            let pos_vec = &self.position_vectors[pos % self.position_vectors.len()];

            // Unbind position to get query for byte
            let query = encoded.bind(pos_vec);

            // Find best matching byte vector
            let byte = self.find_best_byte_match(&query);
            result.push(byte);
        }

        result
    }

    /// Decode chunked data
    ///
    /// # Panics
    /// Panics if `chunk_size` is 0.
    pub fn decode_chunked(
        &self,
        chunks: &[SparseVec],
        chunk_size: usize,
        total_length: usize,
    ) -> Vec<u8> {
        assert!(chunk_size > 0, "chunk_size must be > 0");
        let mut result = Vec::with_capacity(total_length);

        for (chunk_idx, chunk_vec) in chunks.iter().enumerate() {
            let offset = chunk_idx * chunk_size;
            let remaining = total_length.saturating_sub(offset);
            let this_chunk_size = remaining.min(chunk_size);

            for i in 0..this_chunk_size {
                let pos = offset + i;
                let pos_vec = &self.position_vectors[pos % self.position_vectors.len()];
                let query = chunk_vec.bind(pos_vec);
                let byte = self.find_best_byte_match(&query);
                result.push(byte);
            }
        }

        result
    }

    /// Find the byte value with highest similarity to query
    fn find_best_byte_match(&self, query: &SparseVec) -> u8 {
        let mut best_byte = 0u8;
        let mut best_sim = f64::NEG_INFINITY;

        for (byte_val, byte_vec) in self.byte_vectors.iter().enumerate() {
            let sim = query.cosine(byte_vec);
            if sim > best_sim {
                best_sim = sim;
                best_byte = byte_val as u8;
            }
        }

        best_byte
    }

    /// Get reference to byte vectors (for GPU acceleration)
    ///
    /// Returns slice of 256 basis vectors, one per byte value.
    pub fn get_byte_vectors(&self) -> &[SparseVec] {
        &self.byte_vectors
    }

    /// Get position vector for a given position (for GPU acceleration)
    ///
    /// Returns reference to position basis vector. Uses modulo if position
    /// exceeds pre-allocated vectors.
    ///
    /// Note: Call `ensure_positions(max_pos)` first if you need exact position
    /// vectors without modulo wrapping.
    pub fn get_position_vector_ref(&self, pos: usize) -> &SparseVec {
        &self.position_vectors[pos % self.position_vectors.len()]
    }

    /// Ensure position vectors exist up to (and including) the given position
    ///
    /// # Panics
    ///
    /// Panics if `max_pos >= MAX_POSITIONS` (65536).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut encoder = ReversibleVSAEncoder::new();
    /// encoder.ensure_positions(1000); // OK
    /// // encoder.ensure_positions(100000); // Would panic
    /// ```
    pub fn ensure_positions(&mut self, max_pos: usize) {
        assert!(
            max_pos < MAX_POSITIONS,
            "max_pos {} exceeds MAX_POSITIONS ({})",
            max_pos,
            MAX_POSITIONS
        );
        let _ = self.get_position_vector(max_pos);
    }

    /// Ensure position vectors exist up to the given position, returning error on invalid input
    ///
    /// This is a non-panicking alternative to `ensure_positions`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if positions are allocated successfully, or `Err` if
    /// `max_pos >= MAX_POSITIONS`.
    pub fn try_ensure_positions(&mut self, max_pos: usize) -> Result<(), String> {
        if max_pos >= MAX_POSITIONS {
            return Err(format!(
                "max_pos {} exceeds MAX_POSITIONS ({})",
                max_pos, MAX_POSITIONS
            ));
        }
        let _ = self.get_position_vector(max_pos);
        Ok(())
    }

    /// Compute reconstruction accuracy (for testing)
    pub fn test_accuracy(&mut self, data: &[u8]) -> f64 {
        let encoded = self.encode(data);
        let decoded = self.decode(&encoded, data.len());

        let matches = data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count();

        matches as f64 / data.len() as f64
    }

    /// Compute chunked reconstruction accuracy
    pub fn test_accuracy_chunked(&mut self, data: &[u8], chunk_size: usize) -> f64 {
        let chunks = self.encode_chunked(data, chunk_size);
        let decoded = self.decode_chunked(&chunks, chunk_size, data.len());

        let matches = data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count();

        matches as f64 / data.len() as f64
    }

    /// Batch encode data in parallel using rayon
    ///
    /// This method achieves higher throughput by processing chunks in parallel.
    /// Position vectors are pre-allocated before parallel processing.
    ///
    /// # Arguments
    /// * `data` - Raw bytes to encode
    /// * `chunk_size` - Size of each chunk (64 bytes recommended)
    ///
    /// # Returns
    /// Vector of encoded chunks, one `SparseVec` per chunk
    ///
    /// # Panics
    /// Panics if `chunk_size` is 0.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut encoder = ReversibleVSAEncoder::new();
    /// let chunks = encoder.batch_encode(&large_data, 64);
    /// ```
    pub fn batch_encode(&mut self, data: &[u8], chunk_size: usize) -> Vec<SparseVec> {
        assert!(chunk_size > 0, "chunk_size must be > 0");

        if data.is_empty() {
            return Vec::new();
        }

        // Ensure at least one position vector exists to avoid modulo by zero
        let _ = self.get_position_vector(0);

        // Pre-ensure all position vectors exist (sequential, but fast)
        let max_pos = data.len().saturating_sub(1);
        if max_pos < MAX_POSITIONS {
            let _ = self.get_position_vector(max_pos);
        }

        // Use shared references for parallel access (no cloning/allocation)
        // SparseVec is Sync+Send for read access, so shared refs work with Rayon
        let byte_vectors = &self.byte_vectors;
        let position_vectors = &self.position_vectors;

        // Process chunks in parallel
        let chunks: Vec<(usize, &[u8])> = data.chunks(chunk_size).enumerate().collect();

        chunks
            .par_iter()
            .map(|(chunk_idx, chunk)| {
                let offset = chunk_idx * chunk_size;
                Self::encode_chunk_parallel(chunk, offset, byte_vectors, position_vectors)
            })
            .collect()
    }

    /// Encode a single chunk (used by parallel encoding)
    fn encode_chunk_parallel(
        data: &[u8],
        offset: usize,
        byte_vectors: &[SparseVec],
        position_vectors: &[SparseVec],
    ) -> SparseVec {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Encode first byte
        let byte_vec = &byte_vectors[data[0] as usize];
        let pos_vec = &position_vectors[offset % position_vectors.len()];
        let mut result = byte_vec.bind(pos_vec);

        // Encode remaining bytes and bundle
        for (i, &byte) in data.iter().enumerate().skip(1) {
            let byte_vec = &byte_vectors[byte as usize];
            let pos_vec = &position_vectors[(offset + i) % position_vectors.len()];
            let encoded_byte = byte_vec.bind(pos_vec);
            result = result.bundle(&encoded_byte);
        }

        result
    }

    /// Batch decode chunks in parallel using rayon
    ///
    /// This method achieves higher throughput by decoding chunks in parallel.
    ///
    /// # Arguments
    /// * `chunks` - Encoded chunks to decode
    /// * `chunk_size` - Size of each chunk (must match encoding chunk_size)
    /// * `total_length` - Total expected output length
    ///
    /// # Returns
    /// Reconstructed bytes
    ///
    /// # Panics
    /// Panics if `chunk_size` is 0.
    pub fn batch_decode(
        &self,
        chunks: &[SparseVec],
        chunk_size: usize,
        total_length: usize,
    ) -> Vec<u8> {
        assert!(chunk_size > 0, "chunk_size must be > 0");

        if chunks.is_empty() || total_length == 0 {
            return Vec::new();
        }

        // Use shared references for parallel access (no cloning/allocation)
        // SparseVec is Sync+Send for read access, so shared refs work with Rayon
        let byte_vectors = &self.byte_vectors;
        let position_vectors = &self.position_vectors;

        // Decode chunks in parallel
        let decoded_chunks: Vec<Vec<u8>> = chunks
            .par_iter()
            .enumerate()
            .map(|(chunk_idx, chunk_vec)| {
                let offset = chunk_idx * chunk_size;
                let remaining = total_length.saturating_sub(offset);
                let this_chunk_size = remaining.min(chunk_size);

                Self::decode_chunk_parallel(
                    chunk_vec,
                    offset,
                    this_chunk_size,
                    byte_vectors,
                    position_vectors,
                )
            })
            .collect();

        // Flatten results
        decoded_chunks.into_iter().flatten().collect()
    }

    /// Decode a single chunk (used by parallel decoding)
    fn decode_chunk_parallel(
        chunk_vec: &SparseVec,
        offset: usize,
        chunk_size: usize,
        byte_vectors: &[SparseVec],
        position_vectors: &[SparseVec],
    ) -> Vec<u8> {
        let mut result = Vec::with_capacity(chunk_size);

        for i in 0..chunk_size {
            let pos = offset + i;
            let pos_vec = &position_vectors[pos % position_vectors.len()];
            let query = chunk_vec.bind(pos_vec);

            // Find best matching byte
            let mut best_byte = 0u8;
            let mut best_sim = f64::NEG_INFINITY;

            for (byte_val, byte_vec) in byte_vectors.iter().enumerate() {
                let sim = query.cosine(byte_vec);
                if sim > best_sim {
                    best_sim = sim;
                    best_byte = byte_val as u8;
                }
            }

            result.push(best_byte);
        }

        result
    }

    /// Get throughput statistics for batch encoding
    ///
    /// Returns approximate throughput in MB/s based on data size and elapsed time.
    /// Useful for benchmarking and optimization.
    ///
    /// Returns `f64::INFINITY` if elapsed_secs is zero or negative.
    pub fn estimate_throughput(data_size: usize, elapsed_secs: f64) -> f64 {
        if elapsed_secs <= 0.0 {
            return f64::INFINITY;
        }
        let mb = data_size as f64 / (1024.0 * 1024.0);
        mb / elapsed_secs
    }
}

impl Default for ReversibleVSAEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_byte_roundtrip() {
        let mut encoder = ReversibleVSAEncoder::new();

        for byte in [0u8, 42, 127, 255] {
            let encoded = encoder.encode(&[byte]);
            let decoded = encoder.decode(&encoded, 1);
            assert_eq!(decoded[0], byte, "Failed roundtrip for byte {}", byte);
        }
    }

    #[test]
    fn test_short_string_roundtrip() {
        let mut encoder = ReversibleVSAEncoder::new();
        let data = b"Hello";

        let encoded = encoder.encode(data);
        let decoded = encoder.decode(&encoded, data.len());

        // Check accuracy
        let accuracy = data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count() as f64
            / data.len() as f64;

        assert!(accuracy >= 0.5, "Accuracy too low: {}", accuracy);
    }

    #[test]
    fn test_chunked_encoding() {
        let mut encoder = ReversibleVSAEncoder::new();
        let data = b"This is a test of chunked encoding for longer data.";

        let accuracy = encoder.test_accuracy_chunked(data, 8);
        println!("Chunked accuracy: {:.2}%", accuracy * 100.0);

        // Chunked encoding typically achieves 90-95% accuracy, but there's natural
        // variance depending on data content (some byte patterns are more distinct
        // in the VSA representation). Use 80% threshold to accommodate variance
        // while still catching regressions. The correction layer handles any
        // remaining errors in production use.
        assert!(
            accuracy >= 0.80,
            "Chunked accuracy {:.1}% is below expected threshold 80%",
            accuracy * 100.0
        );
    }

    #[test]
    fn test_try_ensure_positions_within_limit() {
        let mut encoder = ReversibleVSAEncoder::new();
        // Should succeed within limit
        assert!(encoder.try_ensure_positions(1000).is_ok());
        assert!(encoder.try_ensure_positions(MAX_POSITIONS - 1).is_ok());
    }

    #[test]
    fn test_try_ensure_positions_exceeds_limit() {
        let mut encoder = ReversibleVSAEncoder::new();
        // Should fail when exceeding limit
        assert!(encoder.try_ensure_positions(MAX_POSITIONS).is_err());
        assert!(encoder.try_ensure_positions(MAX_POSITIONS + 1).is_err());
    }

    #[test]
    #[should_panic(expected = "MAX_POSITIONS")]
    fn test_ensure_positions_panics_on_overflow() {
        let mut encoder = ReversibleVSAEncoder::new();
        encoder.ensure_positions(MAX_POSITIONS);
    }

    #[test]
    fn test_batch_encode_matches_sequential() {
        let mut encoder = ReversibleVSAEncoder::new();
        let data = b"This is a test of parallel batch encoding for higher throughput.";
        let chunk_size = 16;

        // Sequential encoding
        let sequential_chunks = encoder.encode_chunked(data, chunk_size);

        // Parallel batch encoding
        let parallel_chunks = encoder.batch_encode(data, chunk_size);

        // Should produce same number of chunks
        assert_eq!(sequential_chunks.len(), parallel_chunks.len());

        // Each chunk should produce same decoded output
        for (i, (seq_chunk, par_chunk)) in sequential_chunks
            .iter()
            .zip(parallel_chunks.iter())
            .enumerate()
        {
            let seq_decoded =
                encoder.decode(seq_chunk, chunk_size.min(data.len() - i * chunk_size));
            let par_decoded =
                encoder.decode(par_chunk, chunk_size.min(data.len() - i * chunk_size));
            assert_eq!(seq_decoded, par_decoded, "Chunk {} decoded differently", i);
        }
    }

    #[test]
    fn test_batch_decode_matches_sequential() {
        let mut encoder = ReversibleVSAEncoder::new();
        let data = b"Testing parallel batch decode for higher throughput on multi-core systems.";
        let chunk_size = 16;

        // Encode
        let chunks = encoder.batch_encode(data, chunk_size);

        // Sequential decode
        let sequential_decoded = encoder.decode_chunked(&chunks, chunk_size, data.len());

        // Parallel batch decode
        let parallel_decoded = encoder.batch_decode(&chunks, chunk_size, data.len());

        // Should match exactly
        assert_eq!(sequential_decoded, parallel_decoded);
    }

    #[test]
    fn test_batch_encode_empty() {
        let mut encoder = ReversibleVSAEncoder::new();
        let chunks = encoder.batch_encode(&[], 64);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_batch_decode_empty() {
        let encoder = ReversibleVSAEncoder::new();
        let decoded = encoder.batch_decode(&[], 64, 0);
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_batch_encode_accuracy() {
        let mut encoder = ReversibleVSAEncoder::new();
        let data = b"The quick brown fox jumps over the lazy dog. 0123456789!";
        let chunk_size = 16;

        let chunks = encoder.batch_encode(data, chunk_size);
        let decoded = encoder.batch_decode(&chunks, chunk_size, data.len());

        let matches = data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count();
        let accuracy = matches as f64 / data.len() as f64;

        println!("Batch encode/decode accuracy: {:.2}%", accuracy * 100.0);
        assert!(
            accuracy >= 0.80,
            "Batch accuracy {:.1}% below expected 80%",
            accuracy * 100.0
        );
    }
}
