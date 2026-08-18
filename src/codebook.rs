//! Codebook - Differential Encoding Base Model
//!
//! The codebook serves as a learned/constructed basis set for differential encoding.
//! Data is projected onto this basis, and only the residuals (what can't be expressed
//! by the codebook) plus semantic markers are stored in the engram.
//!
//! # Architecture
//!
//! ```text
//! Codebook = { basis_vectors: [B₀, B₁, ..., Bₙ], semantic_markers: [...] }
//!
//! Encoding:  data → coefficients × basis + residual + semantic_outliers
//! Decoding:  coefficients × basis + residual + semantic_outliers → data
//! ```
//!
//! # Note on Reconstruction
//!
//! The codebook is required for data reconstruction:
//! - Without the codebook, reconstruction is not possible
//! - Different codebooks produce different representations
//!
//! Note: This is not a security feature. The codebook is not encryption.
//! Security features will be implemented separately in the future.

use crate::vsa::{SparseVec, DIM};
use crate::VsaError;
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
    pub fn new(value: i64, metadata: WordMetadata) -> Result<Self, VsaError> {
        if !(Self::MIN_VALUE..=Self::MAX_VALUE).contains(&value) {
            return Err(VsaError::ValueOutOfRange {
                value,
                min: Self::MIN_VALUE,
                max: Self::MAX_VALUE,
            });
        }

        // Convert signed value to balanced ternary representation
        let encoded = Self::encode_balanced_ternary(value);

        // Pack metadata into upper 3 bits
        let meta_bits = (metadata as u64) << 61;

        Ok(BalancedTernaryWord {
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
                0 => {} // Add 0
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

/// Configuration for codebook projection operations
#[derive(Clone, Debug)]
pub struct ProjectionConfig {
    /// Size of data chunks to process (bytes)
    pub chunk_size: usize,
    /// Similarity threshold for basis vector relevance
    pub similarity_threshold: f64,
    /// Maximum number of top basis matches to use
    pub max_basis_matches: usize,
    /// Scaling factor for coefficient encoding
    pub coefficient_scale: f64,
    /// Key spacing factor for coefficient indexing
    pub coefficient_key_spacing: u32,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64,
            similarity_threshold: 0.3,
            max_basis_matches: 4,
            coefficient_scale: 1000.0,
            coefficient_key_spacing: 1000,
        }
    }
}

/// Configuration for codebook training
#[derive(Clone, Debug)]
pub struct CodebookTrainingConfig {
    /// Maximum number of learned basis vectors
    pub max_basis_vectors: usize,
    /// Minimum frequency for a pattern to become a basis
    pub min_frequency: u64,
    /// Whether to include byte-level basis vectors (0-255)
    pub include_byte_basis: bool,
    /// Whether to include position-aware basis vectors
    pub include_position_basis: bool,
}

impl Default for CodebookTrainingConfig {
    fn default() -> Self {
        Self {
            max_basis_vectors: 512,
            min_frequency: 5,
            include_byte_basis: true,
            include_position_basis: true,
        }
    }
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

    /// Initialize with byte-level basis vectors (256 basis vectors for each byte value)
    ///
    /// This creates a complete basis that can represent any byte data.
    /// Each byte value 0-255 gets its own basis vector.
    ///
    /// Position basis vectors (64 vectors for positions 0-63) are also added
    /// by default. Use `initialize_byte_basis_with_config` to control this.
    pub fn initialize_byte_basis(&mut self) {
        self.initialize_byte_basis_with_config(true);
    }

    /// Initialize with byte-level basis vectors with optional position basis
    ///
    /// # Arguments
    /// * `include_position_basis` - Whether to add position-aware basis vectors (64 vectors)
    pub fn initialize_byte_basis_with_config(&mut self, include_position_basis: bool) {
        use sha2::{Digest, Sha256};

        // Create basis vectors for all 256 byte values
        for byte_val in 0u8..=255 {
            let mut hasher = Sha256::new();
            hasher.update(b"embeddenator:byte_basis:v1:");
            hasher.update([byte_val]);
            hasher.update((self.dimensionality as u64).to_le_bytes());
            if let Some(salt) = &self.salt {
                hasher.update(salt);
            }
            let hash = hasher.finalize();
            let seed: [u8; 32] = hash.into();

            let vector = SparseVec::from_seed(&seed, self.dimensionality);
            self.basis_vectors.push(BasisVector {
                id: byte_val as u32,
                vector,
                label: Some(format!("byte_{:02x}", byte_val)),
                weight: 1.0,
            });
        }

        // Optionally add position basis vectors (for position-aware encoding)
        // This helps distinguish the same byte at different positions
        if include_position_basis {
            for pos in 0..64 {
                let mut hasher = Sha256::new();
                hasher.update(b"embeddenator:position_basis:v1:");
                hasher.update((pos as u64).to_le_bytes());
                hasher.update((self.dimensionality as u64).to_le_bytes());
                if let Some(salt) = &self.salt {
                    hasher.update(salt);
                }
                let hash = hasher.finalize();
                let seed: [u8; 32] = hash.into();

                let vector = SparseVec::from_seed(&seed, self.dimensionality);
                self.basis_vectors.push(BasisVector {
                    id: 256 + pos as u32,
                    vector,
                    label: Some(format!("pos_{}", pos)),
                    weight: 1.0,
                });
            }
        }
    }

    /// Train the codebook on representative data
    ///
    /// This learns basis vectors by analyzing patterns in the training data.
    /// The algorithm:
    /// 1. Chunk the data into blocks
    /// 2. Find frequently occurring patterns (n-grams)
    /// 3. Create basis vectors for the most common patterns
    /// 4. Optionally add byte-level basis as fallback
    ///
    /// # Arguments
    /// * `training_data` - Slice of training samples
    /// * `config` - Training configuration
    ///
    /// # Returns
    /// Number of basis vectors learned
    ///
    /// # ID Allocation
    ///
    /// Basis vector IDs are allocated in non-overlapping ranges:
    /// - Byte basis: 0-255
    /// - Position basis: 256-319
    /// - Learned patterns: 1000+
    pub fn train(&mut self, training_data: &[&[u8]], config: &CodebookTrainingConfig) -> usize {
        use std::collections::HashMap;

        // 1. If enabled, add byte-level basis first (IDs 0-319)
        // This ensures byte basis gets the reserved ID range
        let mut added = 0;
        if config.include_byte_basis {
            self.initialize_byte_basis_with_config(config.include_position_basis);
            added += 256; // 256 byte basis
            if config.include_position_basis {
                added += 64; // 64 position basis
            }
        }

        // 2. Count n-gram frequencies across all training data
        let mut ngram_counts: HashMap<Vec<u8>, u64> = HashMap::new();

        for data in training_data {
            for window_size in &[2usize, 3, 4, 6, 8] {
                if data.len() >= *window_size {
                    for window in data.windows(*window_size) {
                        *ngram_counts.entry(window.to_vec()).or_insert(0) += 1;
                    }
                }
            }
        }

        // 3. Sort by frequency and take top patterns
        let mut patterns: Vec<(Vec<u8>, u64)> = ngram_counts.into_iter().collect();
        patterns.sort_by_key(|a| std::cmp::Reverse(a.1));

        // 4. Add top patterns as basis vectors with IDs starting at 1000
        // This avoids collision with byte basis (0-319)
        const PATTERN_ID_START: u32 = 1000;
        let mut pattern_id = PATTERN_ID_START;
        for (pattern, count) in patterns.iter().take(config.max_basis_vectors) {
            if *count >= config.min_frequency && pattern.len() >= 2 {
                let label = format!("pattern_{:02x}_{}_freq{}", pattern[0], pattern.len(), count);
                self.add_basis_for_pattern(pattern_id, pattern, &label);
                pattern_id += 1;
                added += 1;
            }
        }

        self.statistics.total_bytes_encoded = training_data.iter().map(|d| d.len() as u64).sum();

        added
    }

    /// Train codebook from files on disk
    ///
    /// Convenience method that reads files and trains on their content.
    pub fn train_from_files(
        &mut self,
        paths: &[&std::path::Path],
        config: &CodebookTrainingConfig,
    ) -> std::io::Result<usize> {
        let mut training_data: Vec<Vec<u8>> = Vec::new();

        for path in paths {
            if let Ok(data) = std::fs::read(path) {
                training_data.push(data);
            }
        }

        let refs: Vec<&[u8]> = training_data.iter().map(|v| v.as_slice()).collect();
        Ok(self.train(&refs, config))
    }

    /// Add a basis vector for a specific pattern
    fn add_basis_for_pattern(&mut self, id: u32, pattern: &[u8], label: &str) {
        use sha2::{Digest, Sha256};

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
            hasher.update((self.dimensionality as u64).to_le_bytes());
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
        self.project_with_config(data, &ProjectionConfig::default())
    }

    /// Project data onto the codebook using custom configuration
    pub fn project_with_config(&self, data: &[u8], config: &ProjectionConfig) -> ProjectionResult {
        let mut coefficients = HashMap::new();
        let mut residual = Vec::new();
        let mut outliers = Vec::new();

        // 1. Analyze data for semantic outliers (entropy spikes)
        let detected_outliers = self.detect_semantic_outliers(data);
        outliers.extend(detected_outliers);

        // 2. Project data chunks onto basis vectors
        let chunk_size = config.chunk_size;
        for (chunk_idx, chunk) in data.chunks(chunk_size).enumerate() {
            let chunk_vec = SparseVec::from_bytes(chunk);

            // Find best matching basis vectors
            let mut best_matches: Vec<(u32, f64)> = self
                .basis_vectors
                .iter()
                .map(|basis| (basis.id, chunk_vec.cosine(&basis.vector)))
                .filter(|(_, sim)| *sim > config.similarity_threshold)
                .collect();

            // Sort by similarity (descending), treating NaN as less than any value
            best_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Less));

            // Take top N matches
            for (basis_id, similarity) in best_matches.iter().take(config.max_basis_matches) {
                // Encode coefficient as balanced ternary
                let coef_value = (*similarity * config.coefficient_scale) as i64;
                if let Ok(word) = BalancedTernaryWord::new(coef_value, WordMetadata::Data) {
                    coefficients.insert(
                        *basis_id * config.coefficient_key_spacing + chunk_idx as u32,
                        word,
                    );
                }
            }

            // 3. Compute residual (what basis couldn't capture)
            let reconstructed = self.reconstruct_chunk(&coefficients, chunk_idx, chunk.len());
            let chunk_residual = self.compute_residual(chunk, &reconstructed);

            for residual_byte in chunk_residual {
                if let Ok(word) =
                    BalancedTernaryWord::new(residual_byte as i64, WordMetadata::Residual)
                {
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
                    let value = chunk
                        .iter()
                        .enumerate()
                        .fold(0i64, |acc, (j, &b)| acc + ((b as i64) << (j * 8)));
                    if let Ok(word) = BalancedTernaryWord::new(value, WordMetadata::SemanticOutlier)
                    {
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
        counts
            .iter()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f64 / len;
                -p * p.log2()
            })
            .sum()
    }

    /// Reconstruct a chunk from coefficients
    ///
    /// This implementation combines basis vectors weighted by their coefficients.
    /// The coefficient keys are structured as: `basis_id * coefficient_key_spacing + chunk_idx`
    ///
    /// Note: Since basis vectors are sparse ternary representations (SparseVec),
    /// reconstruction is an approximation. The actual byte-level reconstruction
    /// relies on the residual corrections applied after this step.
    fn reconstruct_chunk(
        &self,
        coefficients: &HashMap<u32, BalancedTernaryWord>,
        chunk_idx: usize,
        chunk_len: usize,
    ) -> Vec<u8> {
        // Early return for empty cases
        if chunk_len == 0 || coefficients.is_empty() || self.basis_vectors.is_empty() {
            return vec![0u8; chunk_len];
        }

        let config = ProjectionConfig::default();
        let key_spacing = config.coefficient_key_spacing;
        let coef_scale = config.coefficient_scale;

        // Accumulator for weighted reconstruction (using i32 for intermediate values)
        let mut reconstruction: Vec<i32> = vec![0i32; chunk_len];

        // Find coefficients for this chunk
        for basis in &self.basis_vectors {
            let key = basis.id * key_spacing + chunk_idx as u32;
            if let Some(coef_word) = coefficients.get(&key) {
                let coef_value = coef_word.decode();
                // Convert back from scaled similarity to weight factor
                let weight = coef_value as f64 / coef_scale;

                // Apply weighted contribution from this basis vector
                // Since basis vectors are sparse ternary, we use their structure
                // to influence reconstruction. The magnitude is based on coefficient.
                //
                // For reconstruction: treat pos indices as contributing +weight
                // and neg indices as contributing -weight to nearby byte positions.
                let chunk_weight = (weight * 128.0) as i32;

                // Map basis vector indices to chunk positions using modulo.
                // Note: This is an approximation since the basis covers the full DIM space.
                // The modulo mapping may cause non-uniform distribution, but this is
                // acceptable because the residual mechanism compensates for any
                // reconstruction errors in the final output.
                for &idx in &basis.vector.pos {
                    let pos = idx % chunk_len;
                    reconstruction[pos] = reconstruction[pos].saturating_add(chunk_weight);
                }
                for &idx in &basis.vector.neg {
                    let pos = idx % chunk_len;
                    reconstruction[pos] = reconstruction[pos].saturating_sub(chunk_weight);
                }
            }
        }

        // Normalize and clamp to u8 range
        reconstruction
            .iter()
            .map(|&val| val.clamp(0, 255) as u8)
            .collect()
    }

    /// Compute residual between original and reconstructed
    fn compute_residual(&self, original: &[u8], reconstructed: &[u8]) -> Vec<u8> {
        original
            .iter()
            .zip(reconstructed.iter())
            .map(|(&o, &r)| o.wrapping_sub(r))
            .collect()
    }

    /// Calculate reconstruction quality score
    ///
    /// Quality is measured based on how well the basis vectors capture the data.
    /// Score of 1.0 means excellent basis coverage, near 0.0 means poor coverage.
    ///
    /// The score reflects:
    /// 1. Number of basis vectors that matched the data (coefficients stored)
    /// 2. The magnitude of similarity scores (higher = better match)
    fn calculate_quality_score(
        &self,
        original: &[u8],
        coefficients: &HashMap<u32, BalancedTernaryWord>,
        _residual: &[BalancedTernaryWord],
    ) -> f64 {
        if original.is_empty() {
            return 1.0; // Empty data is trivially reconstructed
        }

        if coefficients.is_empty() {
            // No basis vectors matched - low quality
            // But not zero since residual can still reconstruct
            return 0.1;
        }

        let config = ProjectionConfig::default();

        // Calculate average coefficient magnitude (similarity scores)
        let total_coef_magnitude: f64 = coefficients
            .values()
            .map(|word| {
                let val = word.decode() as f64;
                // Convert back from scaled similarity
                (val / config.coefficient_scale).abs()
            })
            .sum();

        let avg_similarity = total_coef_magnitude / coefficients.len() as f64;

        // Calculate coverage: how many chunks have at least one coefficient
        let chunk_count = original.len().div_ceil(config.chunk_size);
        let key_spacing = config.coefficient_key_spacing;

        let chunks_with_coefs: std::collections::HashSet<u32> =
            coefficients.keys().map(|&key| key % key_spacing).collect();

        let coverage_ratio = chunks_with_coefs.len() as f64 / chunk_count.max(1) as f64;

        // Quality combines average similarity with coverage
        // Both factors are in [0, 1] range
        let quality = (avg_similarity * 0.5 + coverage_ratio * 0.5).min(1.0);

        // Ensure non-zero quality when coefficients exist
        quality.max(0.1)
    }

    /// Reconstruct original data from projection result
    pub fn reconstruct(&self, projection: &ProjectionResult, expected_size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(expected_size);

        // 1. Reconstruct from basis coefficients
        let chunk_size = 64;
        let num_chunks = expected_size.div_ceil(chunk_size);

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

impl SparseVec {
    /// Create a sparse vector from a seed (deterministic)
    pub fn from_seed(seed: &[u8; 32], dim: usize) -> Self {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::from_seed(*seed);
        let sparsity = dim / 100; // 1% density

        let mut indices: Vec<usize> = (0..dim).collect();
        indices.shuffle(&mut rng);

        let mut pos: Vec<_> = indices[..sparsity].to_vec();
        let mut neg: Vec<_> = indices[sparsity..sparsity * 2].to_vec();

        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Create a sparse vector directly from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let seed: [u8; 32] = hash.into();

        Self::from_seed(&seed, DIM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balanced_ternary_roundtrip() {
        let test_values = [
            0i64,
            1,
            -1,
            100,
            -100,
            12345,
            -12345,
            BalancedTernaryWord::MAX_VALUE / 2,
            BalancedTernaryWord::MIN_VALUE / 2,
        ];

        for &value in &test_values {
            let word = BalancedTernaryWord::new(value, WordMetadata::Data)
                .expect("Test value should be encodable");
            let decoded = word.decode();
            assert_eq!(value, decoded, "Failed roundtrip for {}", value);
        }
    }

    #[test]
    fn test_balanced_ternary_metadata() {
        let word = BalancedTernaryWord::new(42, WordMetadata::SemanticOutlier)
            .expect("42 should be encodable");
        assert_eq!(word.metadata(), WordMetadata::SemanticOutlier);
        assert_eq!(word.decode(), 42);
    }

    #[test]
    fn test_balanced_ternary_range() {
        // Should succeed at boundaries
        assert!(
            BalancedTernaryWord::new(BalancedTernaryWord::MAX_VALUE, WordMetadata::Data).is_ok()
        );
        assert!(
            BalancedTernaryWord::new(BalancedTernaryWord::MIN_VALUE, WordMetadata::Data).is_ok()
        );

        // Should fail outside boundaries
        assert!(
            BalancedTernaryWord::new(BalancedTernaryWord::MAX_VALUE + 1, WordMetadata::Data)
                .is_err()
        );
        assert!(
            BalancedTernaryWord::new(BalancedTernaryWord::MIN_VALUE - 1, WordMetadata::Data)
                .is_err()
        );
    }

    #[test]
    fn test_codebook_projection() {
        let mut codebook = Codebook::new(10000);
        codebook.initialize_standard_basis();

        let data = b"the quick brown fox jumps over the lazy dog";
        let projection = codebook.project(data);

        assert!(projection.quality_score > 0.0);
        assert!(!projection.coefficients.is_empty() || !projection.residual.is_empty());
    }

    #[test]
    fn test_parity_computation() {
        let word =
            BalancedTernaryWord::new(12345, WordMetadata::Data).expect("12345 should be encodable");
        let parity = word.compute_parity();
        assert!((-1..=1).contains(&parity));
    }
}
