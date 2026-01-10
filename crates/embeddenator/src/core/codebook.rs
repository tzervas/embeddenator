//! Codebook - Differential Encoding Base Model
//!
//! The codebook serves as a basis set for differential encoding of data chunks.
//! Data is projected onto this basis, and the coefficients plus any residuals
//! are stored for reconstruction.
//!
//! # Architecture
//!
//! ```text
//! Codebook = { basis_vectors: [B₀, B₁, ..., Bₙ], metadata: [...] }
//!
//! Encoding:  data → coefficients × basis + residual
//! Decoding:  coefficients × basis + residual → data
//! ```
//!
//! # Data Encoding
//!
//! The codebook uses vector symbolic architecture (VSA) to encode data:
//! - Each data chunk is represented as a sparse ternary vector
//! - The codebook stores basis vectors for reconstruction
//! - Decoding requires the codebook (acts as a key)
//!
//! **⚠️ Security Note:** The cryptographic properties of this encoding are
//! under research. Do not use for security-critical applications.

use crate::vsa::vsa::{SparseVec, DIM};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 64-bit balanced ternary encoding unit
/// - 61 bits: data payload (39 trits worth of information)
/// - 3 bits: parity/metadata (2 trits)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BalancedTernaryWord {
    /// Raw 64-bit representation
    /// Bits 0-60: 39 trits of data (each trit = log₂(3) ≈ 1.585 bits)
    /// Bits 61-63: parity trit + metadata trit
    packed: u64,
}

/// Metadata flags stored in the upper 3 bits
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordMetadata {
    /// Standard data word
    Data = 0b000,
    /// Semantic outlier marker
    SemanticOutlier = 0b001,
    /// Residual correction word
    Residual = 0b010,
    /// Continuation of previous word
    Continuation = 0b011,
    /// End of sequence marker
    EndOfSequence = 0b100,
    /// Parity check word
    Parity = 0b101,
}

impl BalancedTernaryWord {
    /// Maximum value representable in 38 trits (signed balanced)
    /// 3^38 = 1,350,851,717,672,992,089 (fits in 61 bits)
    /// Range: -(3^38-1)/2 to +(3^38-1)/2
    pub const MAX_VALUE: i64 = 675_425_858_836_496_044;
    pub const MIN_VALUE: i64 = -675_425_858_836_496_044;
    
    /// Number of trits in the data portion (38 trits = 61 bits)
    pub const DATA_TRITS: usize = 38;
    
    /// Number of trits for metadata/parity (stored in upper 3 bits)
    pub const META_TRITS: usize = 2;

    /// Create a new word from a signed integer value and metadata
    pub fn new(value: i64, metadata: WordMetadata) -> Option<Self> {
        if value < Self::MIN_VALUE || value > Self::MAX_VALUE {
            return None;
        }
        
        // Convert signed value to balanced ternary representation
        let encoded = Self::encode_balanced_ternary(value);
        
        // Pack metadata into upper 3 bits
        let meta_bits = (metadata as u64) << 61;
        
        Some(BalancedTernaryWord {
            packed: encoded | meta_bits,
        })
    }

    /// Create from raw packed representation
    pub fn from_raw(packed: u64) -> Self {
        BalancedTernaryWord { packed }
    }

    /// Get the raw packed value
    pub fn raw(&self) -> u64 {
        self.packed
    }

    /// Extract the data portion (lower 61 bits)
    pub fn data_bits(&self) -> u64 {
        self.packed & 0x1FFF_FFFF_FFFF_FFFF
    }

    /// Extract metadata
    pub fn metadata(&self) -> WordMetadata {
        match (self.packed >> 61) & 0b111 {
            0b000 => WordMetadata::Data,
            0b001 => WordMetadata::SemanticOutlier,
            0b010 => WordMetadata::Residual,
            0b011 => WordMetadata::Continuation,
            0b100 => WordMetadata::EndOfSequence,
            0b101 => WordMetadata::Parity,
            _ => WordMetadata::Data, // Default fallback
        }
    }

    /// Decode to signed integer value
    pub fn decode(&self) -> i64 {
        Self::decode_balanced_ternary(self.data_bits())
    }

    /// Encode a signed integer to balanced ternary packed representation
    /// 
    /// We store the value directly as a base-3 representation where:
    /// - Digit 0 = trit 0
    /// - Digit 1 = trit +1  
    /// - Digit 2 = trit -1
    fn encode_balanced_ternary(value: i64) -> u64 {
        // For balanced ternary, we convert by repeatedly dividing
        // and adjusting for the balanced representation
        let mut v = value;
        let mut result: u64 = 0;
        let mut power: u64 = 1;
        
        for _ in 0..Self::DATA_TRITS {
            // Get remainder in range [-1, 0, 1]
            let mut rem = v % 3;
            v /= 3;
            
            if rem == 2 {
                rem = -1;
                v += 1;
            } else if rem == -2 {
                rem = 1;
                v -= 1;
            }
            
            // Encode: -1 -> 2, 0 -> 0, +1 -> 1
            let encoded = match rem {
                -1 => 2u64,
                0 => 0u64,
                1 => 1u64,
                _ => 0u64, // Safety fallback
            };
            
            result += encoded * power;
            power *= 3;
        }
        
        result
    }

    /// Decode balanced ternary packed representation to signed integer
    fn decode_balanced_ternary(packed: u64) -> i64 {
        let mut result: i64 = 0;
        let mut power: i64 = 1;
        let mut remaining = packed;
        
        for _ in 0..Self::DATA_TRITS {
            let trit = remaining % 3;
            remaining /= 3;
            
            match trit {
                0 => {}, // Add 0
                1 => result += power,
                2 => result -= power, // -1 in balanced ternary
                _ => unreachable!(),
            }
            power *= 3;
        }
        
        result
    }

    /// Negate all trits in a packed representation
    #[allow(dead_code)]
    fn negate_trits(packed: u64) -> u64 {
        let mut result: u64 = 0;
        let mut remaining = packed;
        let mut power: u64 = 1;
        
        for _ in 0..Self::DATA_TRITS {
            let trit = remaining % 3;
            remaining /= 3;
            
            // Negate: 0->0, 1->2, 2->1
            let negated = match trit {
                0 => 0,
                1 => 2,
                2 => 1,
                _ => unreachable!(),
            };
            result += negated * power;
            power *= 3;
        }
        
        result
    }

    /// Compute parity trit for error detection
    pub fn compute_parity(&self) -> i8 {
        let mut sum: i64 = 0;
        let mut remaining = self.data_bits();
        
        for _ in 0..Self::DATA_TRITS {
            let trit = (remaining % 3) as i64;
            remaining /= 3;
            
            // Convert to balanced: 0->0, 1->1, 2->-1
            sum += match trit {
                0 => 0,
                1 => 1,
                2 => -1,
                _ => 0,
            };
        }
        
        // Parity trit: makes sum divisible by 3
        ((3 - (sum.rem_euclid(3))) % 3) as i8
    }
}

/// Semantic outlier detected during analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticOutlier {
    /// Position in the original data
    pub position: usize,
    /// Length of the outlier pattern
    pub length: usize,
    /// Entropy score (higher = more unusual)
    pub entropy_score: f64,
    /// The outlier pattern encoded as balanced ternary words
    pub encoded_pattern: Vec<BalancedTernaryWord>,
    /// Semantic vector for similarity matching
    pub semantic_vec: SparseVec,
}

/// Basis vector in the codebook
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisVector {
    /// Unique identifier for this basis
    pub id: u32,
    /// The sparse ternary representation
    pub vector: SparseVec,
    /// Human-readable label (optional)
    pub label: Option<String>,
    /// Frequency weight (how often this pattern appears)
    pub weight: f64,
}

/// The Codebook - acts as the private key for reconstruction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Codebook {
    /// Version for compatibility
    pub version: u32,
    
    /// Dimensionality of basis vectors
    pub dimensionality: usize,
    
    /// The basis vectors forming the encoding dictionary
    /// Data is projected onto these bases
    pub basis_vectors: Vec<BasisVector>,
    
    /// Semantic marker vectors for outlier detection
    pub semantic_markers: Vec<SparseVec>,
    
    /// Statistics for adaptive encoding
    pub statistics: CodebookStatistics,
    
    /// Cryptographic salt for key derivation (optional)
    pub salt: Option<[u8; 32]>,
}

/// Statistics tracked by the codebook
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CodebookStatistics {
    /// Total bytes encoded using this codebook
    pub total_bytes_encoded: u64,
    /// Average compression ratio achieved
    pub avg_compression_ratio: f64,
    /// Number of semantic outliers detected
    pub outlier_count: u64,
    /// Distribution of coefficient magnitudes
    pub coefficient_histogram: [u64; 16],
}

/// Result of projecting data onto the codebook
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionResult {
    /// Coefficients for each basis vector (sparse - only non-zero)
    pub coefficients: HashMap<u32, BalancedTernaryWord>,
    /// Residual that couldn't be expressed by the basis
    pub residual: Vec<BalancedTernaryWord>,
    /// Detected semantic outliers
    pub outliers: Vec<SemanticOutlier>,
    /// Reconstruction quality score (1.0 = perfect)
    pub quality_score: f64,
}

impl Default for Codebook {
    fn default() -> Self {
        Self::new(DIM)
    }
}

impl Codebook {
    /// Create a new empty codebook
    pub fn new(dimensionality: usize) -> Self {
        Codebook {
            version: 1,
            dimensionality,
            basis_vectors: Vec::new(),
            semantic_markers: Vec::new(),
            statistics: CodebookStatistics::default(),
            salt: None,
        }
    }

    /// Create a codebook with cryptographic salt for key derivation
    pub fn with_salt(dimensionality: usize, salt: [u8; 32]) -> Self {
        let mut codebook = Self::new(dimensionality);
        codebook.salt = Some(salt);
        codebook
    }

    /// Initialize with common basis vectors for text/binary data
    pub fn initialize_standard_basis(&mut self) {
        // Add basis vectors for common byte patterns
        // These act as a "vocabulary" for differential encoding
        
        // Zero runs (common in binary)
        self.add_basis_for_pattern(0, b"\x00\x00\x00\x00", "zero_run");
        
        // ASCII space/newline (common in text)
        self.add_basis_for_pattern(1, b"    ", "space_run");
        self.add_basis_for_pattern(2, b"\n\n", "newline_pair");
        
        // Common text patterns
        self.add_basis_for_pattern(3, b"the ", "the_space");
        self.add_basis_for_pattern(4, b"ing ", "ing_space");
        self.add_basis_for_pattern(5, b"tion", "tion");
        
        // Binary markers
        self.add_basis_for_pattern(6, b"\x89PNG", "png_header");
        self.add_basis_for_pattern(7, b"\xFF\xD8\xFF", "jpeg_header");
        self.add_basis_for_pattern(8, b"PK\x03\x04", "zip_header");
        
        // Add semantic markers for entropy detection
        self.initialize_semantic_markers();
    }

    /// Add a basis vector for a specific pattern
    fn add_basis_for_pattern(&mut self, id: u32, pattern: &[u8], label: &str) {
        use sha2::{Sha256, Digest};
        
        // Generate deterministic sparse vector from pattern
        let mut hasher = Sha256::new();
        hasher.update(pattern);
        if let Some(salt) = &self.salt {
            hasher.update(salt);
        }
        let hash = hasher.finalize();
        
        // Use hash to seed sparse vector generation
        let seed: [u8; 32] = hash.into();
        let vector = SparseVec::from_seed(&seed, self.dimensionality);
        
        self.basis_vectors.push(BasisVector {
            id,
            vector,
            label: Some(label.to_string()),
            weight: 1.0,
        });
    }

    /// Initialize semantic markers for outlier detection
    fn initialize_semantic_markers(&mut self) {
        use sha2::{Digest, Sha256};

        let seed_for = |label: &str| -> [u8; 32] {
            let mut hasher = Sha256::new();
            hasher.update(b"embeddenator:semantic_marker:v1:");
            hasher.update(label.as_bytes());
            hasher.update(&(self.dimensionality as u64).to_le_bytes());
            if let Some(salt) = &self.salt {
                hasher.update(salt);
            }
            hasher.finalize().into()
        };

        // High entropy marker
        let seed = seed_for("high_entropy");
        self.semantic_markers
            .push(SparseVec::from_seed(&seed, self.dimensionality));

        // Repetition marker
        let seed = seed_for("repetition");
        self.semantic_markers
            .push(SparseVec::from_seed(&seed, self.dimensionality));

        // Boundary marker (transitions)
        let seed = seed_for("boundary");
        self.semantic_markers
            .push(SparseVec::from_seed(&seed, self.dimensionality));
    }

    /// Project data onto the codebook basis
    /// Returns coefficients, residual, and detected outliers
    pub fn project(&self, data: &[u8]) -> ProjectionResult {
        let mut coefficients = HashMap::new();
        let mut residual = Vec::new();
        let mut outliers = Vec::new();
        
        // 1. Analyze data for semantic outliers (entropy spikes)
        let detected_outliers = self.detect_semantic_outliers(data);
        outliers.extend(detected_outliers);
        
        // 2. Project data chunks onto basis vectors
        let chunk_size = 64; // Process in 64-byte chunks
        for (chunk_idx, chunk) in data.chunks(chunk_size).enumerate() {
            let chunk_vec = SparseVec::from_bytes(chunk);
            
            // Find best matching basis vectors
            let mut best_matches: Vec<(u32, f64)> = self.basis_vectors
                .iter()
                .map(|basis| (basis.id, chunk_vec.cosine(&basis.vector)))
                .filter(|(_, sim)| *sim > 0.3) // Threshold for relevance
                .collect();
            
            best_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            
            // Take top N matches
            for (basis_id, similarity) in best_matches.iter().take(4) {
                // Encode coefficient as balanced ternary
                let coef_value = (*similarity * 1000.0) as i64;
                if let Some(word) = BalancedTernaryWord::new(coef_value, WordMetadata::Data) {
                    coefficients.insert(
                        *basis_id * 1000 + chunk_idx as u32,
                        word,
                    );
                }
            }
            
            // 3. Compute residual (what basis couldn't capture)
            let reconstructed = self.reconstruct_chunk(&coefficients, chunk_idx, chunk.len());
            let chunk_residual = self.compute_residual(chunk, &reconstructed);
            
            for residual_byte in chunk_residual {
                if let Some(word) = BalancedTernaryWord::new(residual_byte as i64, WordMetadata::Residual) {
                    residual.push(word);
                }
            }
        }
        
        // Calculate quality score
        let quality_score = self.calculate_quality_score(data, &coefficients, &residual);
        
        ProjectionResult {
            coefficients,
            residual,
            outliers,
            quality_score,
        }
    }

    /// Detect semantic outliers (high entropy, rare patterns)
    fn detect_semantic_outliers(&self, data: &[u8]) -> Vec<SemanticOutlier> {
        let mut outliers = Vec::new();
        let window_size = 32;
        
        if data.len() < window_size {
            return outliers;
        }
        
        for i in 0..data.len() - window_size {
            let window = &data[i..i + window_size];
            let entropy = self.calculate_entropy(window);
            
            // High entropy windows are outliers (compressed/encrypted data)
            if entropy > 7.5 {
                let pattern_vec = SparseVec::from_bytes(window);
                
                // Encode the outlier pattern
                let mut encoded_pattern = Vec::new();
                for chunk in window.chunks(8) {
                    let value = chunk.iter()
                        .enumerate()
                        .fold(0i64, |acc, (j, &b)| acc + ((b as i64) << (j * 8)));
                    if let Some(word) = BalancedTernaryWord::new(value, WordMetadata::SemanticOutlier) {
                        encoded_pattern.push(word);
                    }
                }
                
                outliers.push(SemanticOutlier {
                    position: i,
                    length: window_size,
                    entropy_score: entropy,
                    encoded_pattern,
                    semantic_vec: pattern_vec,
                });
                
                // Skip ahead to avoid overlapping outliers
                // i += window_size / 2; // Can't mutate loop variable, handled by dedup later
            }
        }
        
        // Deduplicate overlapping outliers
        outliers.dedup_by(|a, b| a.position.abs_diff(b.position) < window_size / 2);
        
        outliers
    }

    /// Calculate Shannon entropy of a byte slice
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        let mut counts = [0u32; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }
        
        let len = data.len() as f64;
        counts.iter()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f64 / len;
                -p * p.log2()
            })
            .sum()
    }

    /// Reconstruct a chunk from coefficients
    fn reconstruct_chunk(
        &self,
        _coefficients: &HashMap<u32, BalancedTernaryWord>,
        _chunk_idx: usize,
        chunk_len: usize,
    ) -> Vec<u8> {
        // Placeholder - full implementation would combine basis vectors
        // weighted by coefficients
        vec![0u8; chunk_len]
    }

    /// Compute residual between original and reconstructed
    fn compute_residual(&self, original: &[u8], reconstructed: &[u8]) -> Vec<u8> {
        original.iter()
            .zip(reconstructed.iter())
            .map(|(&o, &r)| o.wrapping_sub(r))
            .collect()
    }

    /// Calculate reconstruction quality score
    fn calculate_quality_score(
        &self,
        _original: &[u8],
        _coefficients: &HashMap<u32, BalancedTernaryWord>,
        _residual: &[BalancedTernaryWord],
    ) -> f64 {
        // Placeholder - would compare reconstruction to original
        1.0
    }

    /// Reconstruct original data from projection result
    pub fn reconstruct(&self, projection: &ProjectionResult, expected_size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(expected_size);
        
        // 1. Reconstruct from basis coefficients
        let chunk_size = 64;
        let num_chunks = (expected_size + chunk_size - 1) / chunk_size;
        
        for chunk_idx in 0..num_chunks {
            let chunk = self.reconstruct_chunk(&projection.coefficients, chunk_idx, chunk_size);
            result.extend(chunk);
        }
        
        // 2. Apply residual corrections
        for (i, residual_word) in projection.residual.iter().enumerate() {
            if i < result.len() {
                let correction = residual_word.decode() as u8;
                result[i] = result[i].wrapping_add(correction);
            }
        }
        
        // 3. Apply semantic outlier corrections
        for outlier in &projection.outliers {
            if outlier.position + outlier.length <= result.len() {
                // Decode outlier pattern and overwrite
                let mut decoded = Vec::new();
                for word in &outlier.encoded_pattern {
                    let value = word.decode();
                    for j in 0..8 {
                        decoded.push(((value >> (j * 8)) & 0xFF) as u8);
                    }
                }
                
                for (j, &byte) in decoded.iter().enumerate().take(outlier.length) {
                    if outlier.position + j < result.len() {
                        result[outlier.position + j] = byte;
                    }
                }
            }
        }
        
        result.truncate(expected_size);
        result
    }
}

// TECH-DEBT: SparseVec::from_seed() and from_bytes() moved to embeddenator-vsa
// Tests moved to tests/codebook/ module for better organization
