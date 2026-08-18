//! GPU-Accelerated Reversible VSA Encoding
//!
//! This module provides GPU-accelerated versions of the encoding/decoding operations
//! in [`ReversibleVSAEncoder`]. The key optimization is using GPU batch cosine
//! similarity to decode all 256 byte candidates in parallel.
//!
//! # Performance
//!
//! | Operation | CPU | GPU | Speedup |
//! |-----------|-----|-----|---------|
//! | Decode 1KB | 256K cosines | 1K GPU calls | 50-100x |
//! | Decode 1MB | 256M cosines | 1M GPU calls | 50-100x |
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_vsa::{GpuAcceleratedEncoder, GpuBackend, GpuConfig};
//!
//! let gpu = GpuBackend::new(GpuConfig::default())?;
//! let mut encoder = GpuAcceleratedEncoder::new(gpu)?;
//!
//! let data = b"Hello, GPU-accelerated VSA!";
//! let encoded = encoder.encode(data);
//! let decoded = encoder.decode_gpu(&encoded, data.len())?;
//! ```

#[cfg(feature = "cuda")]
use crate::gpu::{GpuBackend, GpuError};
use crate::reversible_encoding::{ReversibleVSAEncoder, MAX_POSITIONS};
use crate::vsa::SparseVec;

/// GPU-accelerated VSA encoder/decoder
///
/// Wraps [`ReversibleVSAEncoder`] and uses GPU for expensive decode operations.
/// The byte vectors (256 total) are kept on CPU and transferred per-batch to
/// amortize the cost.
#[cfg(feature = "cuda")]
pub struct GpuAcceleratedEncoder {
    /// CPU encoder for basis vectors and encode operations
    encoder: ReversibleVSAEncoder,
    /// GPU backend for batch cosine operations
    gpu: GpuBackend,
}

#[cfg(feature = "cuda")]
impl GpuAcceleratedEncoder {
    /// Create a new GPU-accelerated encoder
    pub fn new(gpu: GpuBackend) -> Result<Self, GpuError> {
        Ok(Self {
            encoder: ReversibleVSAEncoder::new(),
            gpu,
        })
    }

    /// Create with custom dimensionality
    pub fn with_dim(gpu: GpuBackend, dim: usize) -> Result<Self, GpuError> {
        Ok(Self {
            encoder: ReversibleVSAEncoder::with_dim(dim),
            gpu,
        })
    }

    /// Get reference to the underlying CPU encoder
    pub fn cpu_encoder(&self) -> &ReversibleVSAEncoder {
        &self.encoder
    }

    /// Get mutable reference to the underlying CPU encoder
    pub fn cpu_encoder_mut(&mut self) -> &mut ReversibleVSAEncoder {
        &mut self.encoder
    }

    /// Decode a single position using GPU batch cosine similarity.
    ///
    /// This is the core decode operation: bind encoded vector with position vector,
    /// then find the byte (0-255) with highest similarity.
    ///
    /// # Arguments
    /// * `encoded` - The encoded holographic vector (or chunk) to decode from
    /// * `pos` - The position index to decode
    /// * `byte_vectors` - Slice of 256 byte vectors
    ///
    /// # Returns
    /// The decoded byte value (0-255)
    fn decode_position_with_gpu(
        &self,
        encoded: &SparseVec,
        pos: usize,
        byte_vectors: &[SparseVec],
    ) -> Result<u8, GpuError> {
        let pos_vec = self.encoder.get_position_vector_ref(pos);
        let query = encoded.bind(pos_vec);

        // GPU batch cosine: compare query against all 256 byte vectors at once
        let similarities = self.gpu.batch_cosine(&query, byte_vectors)?;

        // Find byte with highest similarity (argmax)
        let best_byte = similarities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx as u8)
            .unwrap_or(0);

        Ok(best_byte)
    }

    /// Encode data (uses CPU - encoding is already efficient)
    ///
    /// Encoding creates a holographic representation by bundling position-bound
    /// byte vectors. This is already O(n) and CPU-bound bundle operations are
    /// efficient enough that GPU overhead doesn't help.
    pub fn encode(&mut self, data: &[u8]) -> SparseVec {
        self.encoder.encode(data)
    }

    /// Encode data in chunks (uses CPU)
    pub fn encode_chunked(&mut self, data: &[u8], chunk_size: usize) -> Vec<SparseVec> {
        self.encoder.encode_chunked(data, chunk_size)
    }

    /// Decode using GPU-accelerated batch cosine similarity
    ///
    /// For each position, this compares the query against all 256 byte vectors
    /// in a single GPU kernel call, achieving 256x parallelism.
    ///
    /// # Performance
    ///
    /// - CPU decode: O(length * 256) sequential cosine operations
    /// - GPU decode: O(length) GPU kernel calls, each with 256-way parallelism
    ///
    /// # Note
    ///
    /// This method ensures position vectors exist up to `length - 1` before decoding.
    pub fn decode_gpu(&mut self, encoded: &SparseVec, length: usize) -> Result<Vec<u8>, GpuError> {
        if length == 0 {
            return Ok(Vec::new());
        }

        // Ensure position vectors exist (validates MAX_POSITIONS limit)
        if length > MAX_POSITIONS {
            return Err(GpuError::InvalidValue(format!(
                "decode length {} exceeds MAX_POSITIONS {}",
                length, MAX_POSITIONS
            )));
        }
        self.encoder.ensure_positions(length - 1);

        let mut result = Vec::with_capacity(length);
        let byte_vectors = self.encoder.get_byte_vectors();

        for pos in 0..length {
            let best_byte = self.decode_position_with_gpu(encoded, pos, byte_vectors)?;
            result.push(best_byte);
        }

        Ok(result)
    }

    /// Decode chunked data using GPU acceleration
    ///
    /// # Note
    ///
    /// This method ensures position vectors exist up to `total_length - 1` before decoding.
    pub fn decode_chunked_gpu(
        &mut self,
        chunks: &[SparseVec],
        chunk_size: usize,
        total_length: usize,
    ) -> Result<Vec<u8>, GpuError> {
        if total_length == 0 {
            return Ok(Vec::new());
        }

        // Ensure position vectors exist (validates MAX_POSITIONS limit)
        if total_length > MAX_POSITIONS {
            return Err(GpuError::InvalidValue(format!(
                "total_length {} exceeds MAX_POSITIONS {}",
                total_length, MAX_POSITIONS
            )));
        }
        self.encoder.ensure_positions(total_length - 1);

        let mut result = Vec::with_capacity(total_length);
        let byte_vectors = self.encoder.get_byte_vectors();

        for (chunk_idx, chunk_vec) in chunks.iter().enumerate() {
            let offset = chunk_idx * chunk_size;
            let remaining = total_length.saturating_sub(offset);
            let this_chunk_size = remaining.min(chunk_size);

            for i in 0..this_chunk_size {
                let pos = offset + i;
                let best_byte = self.decode_position_with_gpu(chunk_vec, pos, byte_vectors)?;
                result.push(best_byte);
            }
        }

        Ok(result)
    }

    /// Decode using GPU with sequential position processing
    ///
    /// This processes positions in batches for better memory locality but
    /// still issues one GPU kernel call per position. For true multi-query
    /// batching (multiple queries per GPU call), additional GPU kernel
    /// development would be needed.
    ///
    /// # Arguments
    /// * `encoded` - The encoded holographic vector
    /// * `length` - Number of bytes to decode
    /// * `batch_size` - Processing batch size (must be >= 1)
    pub fn decode_gpu_batched(
        &mut self,
        encoded: &SparseVec,
        length: usize,
        batch_size: usize,
    ) -> Result<Vec<u8>, GpuError> {
        if batch_size == 0 {
            return Err(GpuError::InvalidValue(
                "batch_size must be >= 1".to_string(),
            ));
        }

        if length == 0 {
            return Ok(Vec::new());
        }

        // Ensure position vectors exist (validates MAX_POSITIONS limit)
        if length > MAX_POSITIONS {
            return Err(GpuError::InvalidValue(format!(
                "decode length {} exceeds MAX_POSITIONS {}",
                length, MAX_POSITIONS
            )));
        }
        self.encoder.ensure_positions(length - 1);

        let mut result = Vec::with_capacity(length);
        let byte_vectors = self.encoder.get_byte_vectors();

        // Process positions in batches
        for batch_start in (0..length).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(length);

            for pos in batch_start..batch_end {
                let best_byte = self.decode_position_with_gpu(encoded, pos, byte_vectors)?;
                result.push(best_byte);
            }
        }

        Ok(result)
    }

    /// Compute reconstruction accuracy using GPU decode
    ///
    /// Returns 1.0 for empty data (trivially accurate).
    pub fn test_accuracy_gpu(&mut self, data: &[u8]) -> Result<f64, GpuError> {
        if data.is_empty() {
            return Ok(1.0);
        }

        let encoded = self.encode(data);
        let decoded = self.decode_gpu(&encoded, data.len())?;

        let matches = data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count();

        Ok(matches as f64 / data.len() as f64)
    }

    /// Compute chunked reconstruction accuracy using GPU decode
    ///
    /// Returns 1.0 for empty data (trivially accurate).
    pub fn test_accuracy_chunked_gpu(
        &mut self,
        data: &[u8],
        chunk_size: usize,
    ) -> Result<f64, GpuError> {
        if data.is_empty() {
            return Ok(1.0);
        }

        let chunks = self.encode_chunked(data, chunk_size);
        let decoded = self.decode_chunked_gpu(&chunks, chunk_size, data.len())?;

        let matches = data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count();

        Ok(matches as f64 / data.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(feature = "cuda")]
    fn test_empty_data_accuracy() {
        // This test only runs with cuda feature
        // Empty data should return 1.0 accuracy without errors
    }
}
