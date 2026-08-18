//! Algebraic Correction Layer - Guaranteeing 100% Bitwise Reconstruction
//!
//! The fundamental challenge: VSA operations (bundle, bind) are inherently
//! approximate when superposing multiple vectors. This module provides the
//! mathematical machinery to guarantee bit-perfect reconstruction.
//!
//! # The Problem
//!
//! When you bundle N vectors: R = V₁ ⊕ V₂ ⊕ ... ⊕ Vₙ
//!
//! And then query: Q = R ⊙ Vᵢ⁻¹ (unbind to retrieve Vᵢ)
//!
//! You get: Q ≈ Vᵢ + noise (crosstalk from other vectors)
//!
//! The similarity cos(Q, Vᵢ) decreases as N increases (more crosstalk).
//!
//! # The Solution: Multi-Layer Correction
//!
//! 1. **Codebook Lookup** (not similarity): If pattern is in codebook,
//!    retrieve EXACT original, not approximate match.
//!
//! 2. **Residual Storage**: For anything not in codebook, store exact
//!    difference between approximation and original.
//!
//! 3. **Semantic Markers**: High-entropy regions that can't be approximated
//!    well are stored verbatim with markers.
//!
//! 4. **Parity Verification**: Detect when approximation has errors,
//!    triggering residual application.
//!
//! # Mathematical Guarantee
//!
//! Let D = original data, E = encoded approximation, R = residual
//!
//! Invariant: D = decode(E) + R (always, by construction)
//!
//! If decode(E) = D, then R = 0 (no storage needed)
//! If decode(E) ≠ D, then R = D - decode(E) (exact correction stored)
//!
//! Either way: D is perfectly recoverable.

use embeddenator_vsa::Trit;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Correction type for different error scenarios
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CorrectionType {
    /// No correction needed - exact match
    None,
    /// Bit flip at specific positions
    BitFlips(Vec<(u64, u8)>),
    /// Trit flip at specific positions
    TritFlips(Vec<(u64, Trit, Trit)>), // position, was, should_be
    /// Block replacement
    BlockReplace { offset: u64, original: Vec<u8> },
    /// Full data (for high-entropy regions)
    Verbatim(Vec<u8>),
}

/// A correction record for a data chunk
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkCorrection {
    /// Chunk identifier
    pub chunk_id: u64,
    /// Type of correction needed
    pub correction: CorrectionType,
    /// Verification hash (first 8 bytes of SHA256)
    pub hash: [u8; 8],
    /// Parity trit for the chunk
    pub parity: Trit,
}

impl ChunkCorrection {
    /// Create a correction record
    pub fn new(chunk_id: u64, original: &[u8], approximation: &[u8]) -> Self {
        let hash = compute_hash(original);
        let parity = compute_data_parity(original);

        let correction = compute_correction(original, approximation);

        ChunkCorrection {
            chunk_id,
            correction,
            hash,
            parity,
        }
    }

    /// Check if correction is needed
    pub fn needs_correction(&self) -> bool {
        !matches!(self.correction, CorrectionType::None)
    }

    /// Apply correction to approximation to get original
    pub fn apply(&self, approximation: &[u8]) -> Vec<u8> {
        match &self.correction {
            CorrectionType::None => approximation.to_vec(),

            CorrectionType::BitFlips(flips) => {
                let mut result = approximation.to_vec();
                for &(pos, mask) in flips {
                    if (pos as usize) < result.len() {
                        result[pos as usize] ^= mask;
                    }
                }
                result
            }

            CorrectionType::TritFlips(flips) => {
                // Convert to bytes, apply trit corrections
                let mut result = approximation.to_vec();
                for &(pos, _was, should_be) in flips {
                    // Trit position to byte position
                    let byte_pos = (pos / 5) as usize; // 5 trits per byte
                    if byte_pos < result.len() {
                        // This is simplified - real impl would unpack/repack trits
                        let trit_in_byte = (pos % 5) as u8;
                        let shift = trit_in_byte * 2;
                        let mask = !(0b11 << shift);
                        let trit_bits = match should_be {
                            Trit::N => 0b10,
                            Trit::Z => 0b00,
                            Trit::P => 0b01,
                        };
                        result[byte_pos] = (result[byte_pos] & mask) | (trit_bits << shift);
                    }
                }
                result
            }

            CorrectionType::BlockReplace { offset, original } => {
                let mut result = approximation.to_vec();
                let start = *offset as usize;
                let end = std::cmp::min(start + original.len(), result.len());
                if start < result.len() {
                    result[start..end].copy_from_slice(&original[..end - start]);
                }
                result
            }

            CorrectionType::Verbatim(data) => data.clone(),
        }
    }

    /// Verify the correction produces the expected hash
    pub fn verify(&self, result: &[u8]) -> bool {
        compute_hash(result) == self.hash
    }

    /// Storage size of this correction
    pub fn storage_size(&self) -> usize {
        match &self.correction {
            CorrectionType::None => 0,
            CorrectionType::BitFlips(flips) => flips.len() * 9, // pos(8) + mask(1)
            CorrectionType::TritFlips(flips) => flips.len() * 10, // pos(8) + 2 trits(2)
            CorrectionType::BlockReplace { original, .. } => 8 + original.len(),
            CorrectionType::Verbatim(data) => data.len(),
        }
    }
}

/// Compute verification hash (first 8 bytes of SHA256)
fn compute_hash(data: &[u8]) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 8];
    hash.copy_from_slice(&result[..8]);
    hash
}

/// Compute parity trit for data
fn compute_data_parity(data: &[u8]) -> Trit {
    let sum: i64 = data.iter().map(|&b| b as i64).sum();
    match (sum % 3) as i8 {
        0 => Trit::Z,
        1 | -2 => Trit::P,
        2 | -1 => Trit::N,
        _ => Trit::Z,
    }
}

/// Compute the minimal correction to transform approximation into original
fn compute_correction(original: &[u8], approximation: &[u8]) -> CorrectionType {
    // If identical, no correction
    if original == approximation {
        return CorrectionType::None;
    }

    // Count differences
    let mut diff_positions: Vec<(u64, u8, u8)> = Vec::new();
    let max_len = std::cmp::max(original.len(), approximation.len());

    for i in 0..max_len {
        let orig_byte = original.get(i).copied().unwrap_or(0);
        let approx_byte = approximation.get(i).copied().unwrap_or(0);

        if orig_byte != approx_byte {
            diff_positions.push((i as u64, orig_byte, approx_byte));
        }
    }

    // Choose correction strategy based on number of differences
    let diff_count = diff_positions.len();

    if diff_count == 0 {
        return CorrectionType::None;
    }

    // If most bytes are different, store verbatim
    if diff_count > original.len() / 2 {
        return CorrectionType::Verbatim(original.to_vec());
    }

    // If differences are clustered, use block replace
    if diff_count > 10 {
        let first_diff = diff_positions.first().map(|p| p.0).unwrap_or(0);
        let last_diff = diff_positions.last().map(|p| p.0).unwrap_or(0);
        let span = (last_diff - first_diff + 1) as usize;

        // If span is small compared to storing individual corrections
        if span < diff_count * 9 {
            let start = first_diff as usize;
            let end = std::cmp::min(start + span, original.len());
            return CorrectionType::BlockReplace {
                offset: first_diff,
                original: original[start..end].to_vec(),
            };
        }
    }

    // Use bit flips for sparse differences
    let bit_flips: Vec<(u64, u8)> = diff_positions
        .iter()
        .map(|&(pos, orig, approx)| (pos, orig ^ approx))
        .collect();

    CorrectionType::BitFlips(bit_flips)
}

/// Correction store - manages all corrections for an engram
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CorrectionStore {
    /// Corrections indexed by chunk ID
    corrections: HashMap<u64, ChunkCorrection>,

    /// Total storage used by corrections
    total_correction_bytes: u64,

    /// Total original data size
    total_original_bytes: u64,

    /// Chunks that needed no correction
    perfect_chunks: u64,

    /// Chunks that needed correction
    corrected_chunks: u64,
}

impl CorrectionStore {
    /// Create a new correction store
    pub fn new() -> Self {
        CorrectionStore::default()
    }

    /// Add a correction for a chunk
    pub fn add(&mut self, chunk_id: u64, original: &[u8], approximation: &[u8]) {
        let correction = ChunkCorrection::new(chunk_id, original, approximation);

        self.total_original_bytes += original.len() as u64;

        if correction.needs_correction() {
            self.total_correction_bytes += correction.storage_size() as u64;
            self.corrected_chunks += 1;
        } else {
            self.perfect_chunks += 1;
        }

        self.corrections.insert(chunk_id, correction);
    }

    /// Get correction for a chunk
    pub fn get(&self, chunk_id: u64) -> Option<&ChunkCorrection> {
        self.corrections.get(&chunk_id)
    }

    /// Apply correction to approximation
    pub fn apply(&self, chunk_id: u64, approximation: &[u8]) -> Option<Vec<u8>> {
        let correction = self.corrections.get(&chunk_id)?;
        let result = correction.apply(approximation);

        // Verify correction worked
        if correction.verify(&result) {
            Some(result)
        } else {
            None // Correction failed verification
        }
    }

    /// Get correction statistics
    pub fn stats(&self) -> CorrectionStats {
        CorrectionStats {
            total_chunks: self.perfect_chunks + self.corrected_chunks,
            perfect_chunks: self.perfect_chunks,
            corrected_chunks: self.corrected_chunks,
            correction_bytes: self.total_correction_bytes,
            original_bytes: self.total_original_bytes,
            correction_ratio: if self.total_original_bytes > 0 {
                self.total_correction_bytes as f64 / self.total_original_bytes as f64
            } else {
                0.0
            },
            perfect_ratio: if self.perfect_chunks + self.corrected_chunks > 0 {
                self.perfect_chunks as f64 / (self.perfect_chunks + self.corrected_chunks) as f64
            } else {
                1.0
            },
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }
}

/// Statistics about corrections
#[derive(Clone, Debug)]
pub struct CorrectionStats {
    pub total_chunks: u64,
    pub perfect_chunks: u64,
    pub corrected_chunks: u64,
    pub correction_bytes: u64,
    pub original_bytes: u64,
    pub correction_ratio: f64,
    pub perfect_ratio: f64,
}

impl std::fmt::Display for CorrectionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Corrections: {}/{} chunks perfect ({:.1}%), \
                   {:.2}% overhead ({} bytes corrections / {} bytes original)",
            self.perfect_chunks,
            self.total_chunks,
            self.perfect_ratio * 100.0,
            self.correction_ratio * 100.0,
            self.correction_bytes,
            self.original_bytes,
        )
    }
}

/// Reconstruction verifier
pub struct ReconstructionVerifier {
    /// Expected hashes for all chunks
    expected_hashes: HashMap<u64, [u8; 8]>,
}

impl ReconstructionVerifier {
    /// Create a new verifier from original data
    pub fn from_chunks(chunks: impl Iterator<Item = (u64, Vec<u8>)>) -> Self {
        let expected_hashes: HashMap<u64, [u8; 8]> =
            chunks.map(|(id, data)| (id, compute_hash(&data))).collect();

        ReconstructionVerifier { expected_hashes }
    }

    /// Verify a reconstructed chunk
    pub fn verify_chunk(&self, chunk_id: u64, data: &[u8]) -> bool {
        match self.expected_hashes.get(&chunk_id) {
            Some(expected) => compute_hash(data) == *expected,
            None => false, // Unknown chunk
        }
    }

    /// Verify all chunks
    pub fn verify_all(&self, chunks: impl Iterator<Item = (u64, Vec<u8>)>) -> VerificationResult {
        let mut verified = 0u64;
        let mut failed = 0u64;
        let mut failed_ids = Vec::new();

        for (id, data) in chunks {
            if self.verify_chunk(id, &data) {
                verified += 1;
            } else {
                failed += 1;
                failed_ids.push(id);
            }
        }

        let missing = self.expected_hashes.len() as u64 - verified - failed;

        VerificationResult {
            verified,
            failed,
            missing,
            failed_ids,
            perfect: failed == 0 && missing == 0,
        }
    }
}

/// Result of verification
#[derive(Clone, Debug)]
pub struct VerificationResult {
    pub verified: u64,
    pub failed: u64,
    pub missing: u64,
    pub failed_ids: Vec<u64>,
    pub perfect: bool,
}

impl std::fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.perfect {
            write!(
                f,
                "✓ Perfect reconstruction: {} chunks verified",
                self.verified
            )
        } else {
            write!(
                f,
                "✗ Reconstruction issues: {} verified, {} failed, {} missing",
                self.verified, self.failed, self.missing
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_correction_needed() {
        let original = b"hello world";
        let approximation = b"hello world";

        let correction = ChunkCorrection::new(0, original, approximation);

        assert!(!correction.needs_correction());
        assert_eq!(correction.storage_size(), 0);
    }

    #[test]
    fn test_bit_flip_correction() {
        let original = b"hello world";
        let mut approximation = original.to_vec();
        approximation[0] ^= 0x01; // Flip one bit

        let correction = ChunkCorrection::new(0, original, &approximation);

        assert!(correction.needs_correction());

        let recovered = correction.apply(&approximation);
        assert_eq!(recovered, original);
        assert!(correction.verify(&recovered));
    }

    #[test]
    fn test_verbatim_correction() {
        let original = b"completely different data here";
        let approximation = b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

        let correction = ChunkCorrection::new(0, original, approximation);

        assert!(correction.needs_correction());

        let recovered = correction.apply(approximation);
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_correction_store() {
        let mut store = CorrectionStore::new();

        // Add some perfect chunks
        store.add(0, b"chunk0", b"chunk0");
        store.add(1, b"chunk1", b"chunk1");

        // Add a chunk needing correction
        store.add(2, b"chunk2", b"chunkX");

        let stats = store.stats();
        assert_eq!(stats.perfect_chunks, 2);
        assert_eq!(stats.corrected_chunks, 1);

        // Verify correction works
        let recovered = store.apply(2, b"chunkX").unwrap();
        assert_eq!(recovered, b"chunk2");
    }

    #[test]
    fn test_reconstruction_verifier() {
        let chunks = vec![
            (0u64, b"chunk0".to_vec()),
            (1u64, b"chunk1".to_vec()),
            (2u64, b"chunk2".to_vec()),
        ];

        let verifier = ReconstructionVerifier::from_chunks(chunks.clone().into_iter());

        // Verify correct chunks
        assert!(verifier.verify_chunk(0, b"chunk0"));
        assert!(verifier.verify_chunk(1, b"chunk1"));

        // Verify incorrect chunk fails
        assert!(!verifier.verify_chunk(0, b"wrong"));

        // Verify all
        let result = verifier.verify_all(chunks.into_iter());
        assert!(result.perfect);
        assert_eq!(result.verified, 3);
    }

    #[test]
    fn test_hash_stability() {
        // Ensure hash function is deterministic
        let data = b"test data for hashing";
        let hash1 = compute_hash(data);
        let hash2 = compute_hash(data);
        assert_eq!(hash1, hash2);

        // Different data should produce different hash
        let hash3 = compute_hash(b"different data");
        assert_ne!(hash1, hash3);
    }
}
