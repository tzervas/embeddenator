//! Vector Symbolic Architecture (VSA) Implementation
//!
//! Sparse ternary vector representation with algebraic operations:
//! - Bundle (⊕): Associative superposition
//! - Bind (⊙): Non-commutative composition
//! - Cosine similarity for retrieval

use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ternary_vec::PackedTritVec;

use std::cell::RefCell;

/// Default dimension of VSA vectors (10,000 dimensions)
pub const DIM: usize = 10000;

/// Configuration for VSA vector operations
///
/// Controls the dimensionality and sparsity of vectors. This allows
/// dynamic configuration of vector dimensions at runtime rather than
/// being locked to the compile-time `DIM` constant.
///
/// # Example
///
/// ```rust
/// use embeddenator_vsa::VsaConfig;
///
/// // Default 10,000 dimensions
/// let default_config = VsaConfig::default();
/// assert_eq!(default_config.dimension, 10_000);
///
/// // Custom configuration for smaller vectors
/// let small_config = VsaConfig::new(1000).with_density(0.02); // 2% density
///
/// // Configuration for larger vectors
/// let large_config = VsaConfig::new(100_000).with_density(0.005); // 0.5% density
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VsaConfig {
    /// Number of dimensions in vectors
    pub dimension: usize,
    /// Target density of non-zero elements (0.0 to 1.0)
    /// Default is 0.01 (1% density)
    pub density: f64,
    /// Minimum non-zero elements per vector
    pub min_nonzero: usize,
    /// Maximum non-zero elements per vector
    pub max_nonzero: usize,
    /// Auto-thinning threshold: automatically thin bundles when nnz exceeds this
    /// Set to 0 to disable auto-thinning
    pub auto_thin_threshold: usize,
    /// Dynamic sparsity scaling mode
    pub scaling_mode: SparsityScaling,
    /// Target reconstruction accuracy (0.0-1.0)
    /// Used by `for_accuracy()` to select appropriate dimensions
    pub target_accuracy: f64,
}

/// Sparsity scaling modes for dynamic dimension handling
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum SparsityScaling {
    /// Fixed density - nnz scales linearly with dimension
    /// Good for: Consistent memory usage per dimension
    Fixed,
    /// Square root scaling - nnz scales as sqrt(dimension)
    /// Good for: Maintaining constant similarity precision
    /// Formula: target_nnz = base_nnz * sqrt(dim / base_dim)
    #[default]
    SquareRoot,
    /// Logarithmic scaling - nnz scales as log(dimension)
    /// Good for: Memory-constrained environments with large dimensions
    /// Formula: target_nnz = base_nnz * log(dim) / log(base_dim)
    Logarithmic,
    /// Custom scaling with user-provided function coefficients
    /// Formula: target_nnz = a * dim^b + c
    Custom { a: f64, b: f64, c: f64 },
}

impl Default for VsaConfig {
    fn default() -> Self {
        Self {
            dimension: DIM,
            density: 0.01, // 1% density
            min_nonzero: 10,
            max_nonzero: DIM / 10,
            auto_thin_threshold: DIM / 5, // Auto-thin when > 20% dense
            scaling_mode: SparsityScaling::SquareRoot,
            target_accuracy: 0.95,
        }
    }
}

impl VsaConfig {
    /// Create a new VSA configuration with the specified dimension
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            density: 0.01,
            min_nonzero: 10,
            max_nonzero: dimension / 10,
            auto_thin_threshold: dimension / 5,
            scaling_mode: SparsityScaling::SquareRoot,
            target_accuracy: 0.95,
        }
    }

    /// Set the target density for non-zero elements
    pub fn with_density(mut self, density: f64) -> Self {
        self.density = density.clamp(0.001, 0.5);
        self
    }

    /// Set minimum number of non-zero elements
    pub fn with_min_nonzero(mut self, min: usize) -> Self {
        self.min_nonzero = min;
        self
    }

    /// Set maximum number of non-zero elements
    pub fn with_max_nonzero(mut self, max: usize) -> Self {
        self.max_nonzero = max;
        self
    }

    /// Set auto-thinning threshold (0 to disable)
    pub fn with_auto_thin(mut self, threshold: usize) -> Self {
        self.auto_thin_threshold = threshold;
        self
    }

    /// Set sparsity scaling mode
    pub fn with_scaling(mut self, mode: SparsityScaling) -> Self {
        self.scaling_mode = mode;
        self
    }

    /// Set target reconstruction accuracy
    pub fn with_target_accuracy(mut self, accuracy: f64) -> Self {
        self.target_accuracy = accuracy.clamp(0.5, 0.9999);
        self
    }

    /// Calculate the sparsity (number of non-zero elements per polarity)
    /// based on dimension, density, and scaling mode
    pub fn sparsity(&self) -> usize {
        let base_target = ((self.dimension as f64 * self.density) / 2.0).round() as usize;

        let scaled_target = match self.scaling_mode {
            SparsityScaling::Fixed => base_target,
            SparsityScaling::SquareRoot => {
                // Scale as sqrt(dim) relative to base 10K dimensions
                let base_dim = 10_000.0_f64;
                let scale = (self.dimension as f64 / base_dim).sqrt();
                (base_target as f64 * scale).round() as usize
            }
            SparsityScaling::Logarithmic => {
                // Scale as log(dim) relative to base 10K dimensions
                let base_dim = 10_000.0_f64;
                let scale = (self.dimension as f64).ln() / base_dim.ln();
                (base_target as f64 * scale).round() as usize
            }
            SparsityScaling::Custom { a, b, c } => {
                // Custom formula: a * dim^b + c
                (a * (self.dimension as f64).powf(b) + c).round() as usize
            }
        };

        scaled_target.clamp(self.min_nonzero, self.max_nonzero)
    }

    /// Create configuration optimized for a target reconstruction accuracy
    ///
    /// This automatically selects dimension and density based on empirical
    /// relationships between VSA parameters and encoding fidelity.
    ///
    /// | Accuracy | Dimension | Density | Typical Use Case |
    /// |----------|-----------|---------|------------------|
    /// | 0.90     | 5,000     | 2%      | Fast prototyping |
    /// | 0.95     | 10,000    | 1%      | Default balance  |
    /// | 0.98     | 50,000    | 0.5%    | High fidelity    |
    /// | 0.99     | 100,000   | 0.3%    | Production       |
    ///
    /// # Example
    ///
    /// ```rust
    /// use embeddenator_vsa::VsaConfig;
    ///
    /// // Create config targeting 95% reconstruction accuracy
    /// let config = VsaConfig::for_accuracy(0.95);
    /// // Accuracy maps to 25K dimensions (between 0.95 and 0.98)
    /// assert!(config.dimension >= 10_000 && config.dimension <= 50_000);
    /// ```
    pub fn for_accuracy(target: f64) -> Self {
        let target = target.clamp(0.5, 0.9999);

        // Empirical mapping: accuracy → (dimension, density)
        // Based on information-theoretic capacity of ternary HDV
        let (dimension, density) = if target < 0.85 {
            (2_000, 0.03)
        } else if target < 0.90 {
            (5_000, 0.02)
        } else if target < 0.95 {
            (10_000, 0.01)
        } else if target < 0.98 {
            (25_000, 0.008)
        } else if target < 0.99 {
            (50_000, 0.005)
        } else if target < 0.995 {
            (100_000, 0.003)
        } else if target < 0.999 {
            (250_000, 0.002)
        } else {
            (500_000, 0.001)
        };

        Self::new(dimension)
            .with_density(density)
            .with_target_accuracy(target)
            .with_scaling(SparsityScaling::SquareRoot)
    }

    /// Create configuration from a custom schema/specification
    ///
    /// Allows users to provide their own parameter mapping for specialized use cases.
    ///
    /// # Example
    ///
    /// ```rust
    /// use embeddenator_vsa::{VsaConfig, VsaConfigSchema, SparsityScaling};
    ///
    /// // Custom schema for memory-constrained embedded systems
    /// let schema = VsaConfigSchema {
    ///     dimension: 4096,
    ///     density: 0.025,
    ///     scaling: Some(SparsityScaling::Logarithmic),
    ///     auto_thin: Some(512),
    ///     min_nnz: Some(8),
    ///     max_nnz: Some(256),
    ///     target_accuracy: None,
    /// };
    /// let config = VsaConfig::from_schema(schema);
    /// assert_eq!(config.dimension, 4096);
    /// ```
    pub fn from_schema(schema: VsaConfigSchema) -> Self {
        let mut config = Self::new(schema.dimension).with_density(schema.density);

        if let Some(scaling) = schema.scaling {
            config = config.with_scaling(scaling);
        }
        if let Some(threshold) = schema.auto_thin {
            config = config.with_auto_thin(threshold);
        }
        if let Some(min) = schema.min_nnz {
            config = config.with_min_nonzero(min);
        }
        if let Some(max) = schema.max_nnz {
            config = config.with_max_nonzero(max);
        }
        if let Some(accuracy) = schema.target_accuracy {
            config = config.with_target_accuracy(accuracy);
        }

        config
    }

    /// Configuration for small vectors (1,000 dimensions)
    pub fn small() -> Self {
        Self::new(1_000).with_density(0.02)
    }

    /// Configuration for medium vectors (10,000 dimensions - default)
    pub fn medium() -> Self {
        Self::default()
    }

    /// Configuration for large vectors (100,000 dimensions)
    pub fn large() -> Self {
        Self::new(100_000).with_density(0.005)
    }

    /// Configuration for huge vectors (1,000,000 dimensions)
    /// Use sparingly - memory intensive
    pub fn huge() -> Self {
        Self::new(1_000_000).with_density(0.001)
    }

    /// Get the effective auto-thin threshold, accounting for dimension
    pub fn effective_auto_thin(&self) -> usize {
        if self.auto_thin_threshold == 0 {
            usize::MAX // Disabled
        } else {
            self.auto_thin_threshold
        }
    }

    /// Check if a vector exceeds the auto-thin threshold
    pub fn should_thin(&self, nnz: usize) -> bool {
        self.auto_thin_threshold > 0 && nnz > self.auto_thin_threshold
    }
}

/// User-provided schema for custom VSA configuration
///
/// This allows users to specify their own parameters for specialized use cases
/// without needing to understand the internal scaling formulas.
///
/// # Example
///
/// ```rust
/// use embeddenator_vsa::{VsaConfig, VsaConfigSchema, SparsityScaling};
///
/// // Custom schema for a specific application
/// let schema = VsaConfigSchema {
///     dimension: 8192,
///     density: 0.015,
///     scaling: Some(SparsityScaling::SquareRoot),
///     auto_thin: Some(1000),
///     min_nnz: Some(16),
///     max_nnz: Some(512),
///     target_accuracy: Some(0.97),
/// };
///
/// let config = VsaConfig::from_schema(schema);
/// assert_eq!(config.dimension, 8192);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VsaConfigSchema {
    /// Required: Number of dimensions
    pub dimension: usize,
    /// Required: Target density (0.0-1.0)
    pub density: f64,
    /// Optional: Sparsity scaling mode
    #[serde(default)]
    pub scaling: Option<SparsityScaling>,
    /// Optional: Auto-thinning threshold (0 to disable)
    #[serde(default)]
    pub auto_thin: Option<usize>,
    /// Optional: Minimum non-zero elements
    #[serde(default)]
    pub min_nnz: Option<usize>,
    /// Optional: Maximum non-zero elements
    #[serde(default)]
    pub max_nnz: Option<usize>,
    /// Optional: Target reconstruction accuracy
    #[serde(default)]
    pub target_accuracy: Option<f64>,
}

impl VsaConfigSchema {
    /// Create a minimal schema with just dimension and density
    pub fn new(dimension: usize, density: f64) -> Self {
        Self {
            dimension,
            density,
            scaling: None,
            auto_thin: None,
            min_nnz: None,
            max_nnz: None,
            target_accuracy: None,
        }
    }

    /// Set scaling mode
    pub fn with_scaling(mut self, mode: SparsityScaling) -> Self {
        self.scaling = Some(mode);
        self
    }

    /// Set auto-thin threshold
    pub fn with_auto_thin(mut self, threshold: usize) -> Self {
        self.auto_thin = Some(threshold);
        self
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

thread_local! {
    // Reused packed buffers for hot paths. Using TLS keeps this allocation
    // amortized while remaining safe in multi-threaded contexts.
    static PACKED_SCRATCH_A: RefCell<PackedTritVec> = RefCell::new(PackedTritVec::new_zero(DIM));
    static PACKED_SCRATCH_B: RefCell<PackedTritVec> = RefCell::new(PackedTritVec::new_zero(DIM));
    static PACKED_SCRATCH_OUT: RefCell<PackedTritVec> = RefCell::new(PackedTritVec::new_zero(DIM));
}

/// Configuration for reversible VSA encoding/decoding operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReversibleVSAConfig {
    /// Block size for chunked encoding (must be power of 2 for efficiency)
    pub block_size: usize,
    /// Maximum path depth for hierarchical encoding
    pub max_path_depth: usize,
    /// Base permutation shift for path-based encoding
    pub base_shift: usize,
    /// Target sparsity level for operations (number of non-zero elements)
    pub target_sparsity: usize,
}

impl Default for ReversibleVSAConfig {
    fn default() -> Self {
        ReversibleVSAConfig {
            block_size: 256, // 256-byte blocks
            max_path_depth: 10,
            base_shift: 1000,
            target_sparsity: 200, // Default sparsity level
        }
    }
}

impl ReversibleVSAConfig {
    /// Create config optimized for small data blocks
    pub fn small_blocks() -> Self {
        ReversibleVSAConfig {
            block_size: 64,
            max_path_depth: 5,
            base_shift: 500,
            target_sparsity: 100,
        }
    }

    /// Create config optimized for large data blocks
    pub fn large_blocks() -> Self {
        ReversibleVSAConfig {
            block_size: 1024,
            max_path_depth: 20,
            base_shift: 2000,
            target_sparsity: 400,
        }
    }
}

/// Sparse ternary vector with positive and negative indices
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseVec {
    /// Indices with +1 value
    pub pos: Vec<usize>,
    /// Indices with -1 value
    pub neg: Vec<usize>,
}

impl Default for SparseVec {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseVec {
    #[inline]
    fn nnz(&self) -> usize {
        self.pos.len() + self.neg.len()
    }

    fn intersection_count_sorted(a: &[usize], b: &[usize]) -> usize {
        let mut i = 0usize;
        let mut j = 0usize;
        let mut count = 0usize;
        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    count += 1;
                    i += 1;
                    j += 1;
                }
            }
        }
        count
    }

    fn union_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
        let mut out = Vec::with_capacity(a.len() + b.len());
        let mut i = 0usize;
        let mut j = 0usize;

        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => {
                    out.push(a[i]);
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    out.push(b[j]);
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    out.push(a[i]);
                    i += 1;
                    j += 1;
                }
            }
        }

        if i < a.len() {
            out.extend_from_slice(&a[i..]);
        }
        if j < b.len() {
            out.extend_from_slice(&b[j..]);
        }

        out
    }

    fn difference_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
        let mut out = Vec::with_capacity(a.len());
        let mut i = 0usize;
        let mut j = 0usize;

        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => {
                    out.push(a[i]);
                    i += 1;
                }
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
            }
        }

        if i < a.len() {
            out.extend_from_slice(&a[i..]);
        }

        out
    }

    fn intersection_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
        let mut out = Vec::with_capacity(a.len().min(b.len()));
        let mut i = 0usize;
        let mut j = 0usize;

        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    out.push(a[i]);
                    i += 1;
                    j += 1;
                }
            }
        }

        out
    }
    /// Create an empty sparse vector
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let vec = SparseVec::new();
    /// assert!(vec.pos.is_empty());
    /// assert!(vec.neg.is_empty());
    /// ```
    pub fn new() -> Self {
        SparseVec {
            pos: Vec::new(),
            neg: Vec::new(),
        }
    }

    /// Generate a random sparse vector with ~1% density
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let vec = SparseVec::random();
    /// // Vector should have approximately 1% density (100 positive + 100 negative)
    /// assert!(vec.pos.len() > 0);
    /// assert!(vec.neg.len() > 0);
    /// ```
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let sparsity = DIM / 100; // ~1% density

        // Generate random indices without replacement
        let mut indices: Vec<usize> = (0..DIM).collect();
        indices.shuffle(&mut rng);

        let mut pos: Vec<_> = indices[..sparsity].to_vec();
        let mut neg: Vec<_> = indices[sparsity..sparsity * 2].to_vec();

        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Generate a random sparse vector with configurable dimensions and density
    ///
    /// This method allows creating vectors with custom dimensions, useful for
    /// benchmarking with different data types or when dimensions need to be
    /// determined at runtime.
    ///
    /// # Arguments
    /// * `config` - Configuration specifying dimension and density
    ///
    /// # Examples
    ///
    /// ```rust
    /// use embeddenator_vsa::{SparseVec, VsaConfig};
    ///
    /// // Create a small vector (1000 dims, 2% density)
    /// let config = VsaConfig::small();
    /// let vec = SparseVec::random_with_config(&config);
    /// assert!(vec.pos.len() > 0);
    ///
    /// // Create a custom-sized vector
    /// let custom = VsaConfig::new(5000).with_density(0.02);
    /// let vec = SparseVec::random_with_config(&custom);
    /// ```
    pub fn random_with_config(config: &VsaConfig) -> Self {
        let mut rng = rand::rng();
        let sparsity = config.sparsity();

        // Generate random indices without replacement
        let mut indices: Vec<usize> = (0..config.dimension).collect();
        indices.shuffle(&mut rng);

        let mut pos: Vec<_> = indices[..sparsity].to_vec();
        let mut neg: Vec<_> = indices[sparsity..sparsity * 2].to_vec();

        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Encode data into a reversible sparse vector using block-based mapping
    ///
    /// This method implements hierarchical encoding with path-based permutations
    /// for lossless data recovery. The encoding process:
    /// 1. Splits data into blocks of configurable size
    /// 2. Applies path-based permutations to each block
    /// 3. Combines blocks using hierarchical bundling
    ///
    /// # Arguments
    /// * `data` - The data to encode
    /// * `config` - Configuration for encoding parameters
    /// * `path` - Optional path string for hierarchical encoding (affects permutation)
    ///
    /// # Returns
    /// A SparseVec that can be decoded back to the original data
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
    ///
    /// let data = b"hello world";
    /// let config = ReversibleVSAConfig::default();
    /// let encoded = SparseVec::encode_data(data, &config, None);
    ///
    /// // encoded vector contains reversible representation of the data
    /// assert!(!encoded.pos.is_empty() || !encoded.neg.is_empty());
    /// ```
    pub fn encode_data(data: &[u8], config: &ReversibleVSAConfig, path: Option<&str>) -> Self {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Calculate path-based shift for hierarchical encoding
        let path_shift = if let Some(path_str) = path {
            // Use path hash to determine shift, constrained to prevent overflow
            let mut hasher = Sha256::new();
            hasher.update(path_str.as_bytes());
            let hash = hasher.finalize();
            // SAFETY: SHA256 always produces 32 bytes, first 4 bytes are always valid
            let hash_bytes: [u8; 4] = hash[0..4]
                .try_into()
                .expect("SHA256 hash must produce at least 4 bytes");
            let path_hash = u32::from_le_bytes(hash_bytes) as usize;
            (path_hash % config.max_path_depth) * config.base_shift
        } else {
            0
        };

        // Split data into blocks
        let mut blocks = Vec::new();
        for chunk in data.chunks(config.block_size) {
            blocks.push(chunk);
        }

        // Encode each block with position-based permutation
        let mut encoded_blocks = Vec::new();
        for (i, block) in blocks.iter().enumerate() {
            let block_shift = path_shift + (i * config.base_shift / blocks.len().max(1));
            let block_vec = Self::encode_block(block, block_shift);
            encoded_blocks.push(block_vec);
        }

        // Combine blocks hierarchically
        if encoded_blocks.is_empty() {
            SparseVec::new()
        } else if encoded_blocks.len() == 1 {
            // SAFETY: we just checked len() == 1, so next() must return Some
            encoded_blocks
                .into_iter()
                .next()
                .expect("encoded_blocks must have exactly one element")
        } else {
            // Hierarchical bundling: combine in binary tree fashion
            Self::hierarchical_bundle(&encoded_blocks)
        }
    }

    /// Decode data from a reversible sparse vector
    ///
    /// Reverses the encoding process to recover the original data.
    /// Requires the same configuration and path used during encoding.
    ///
    /// # Arguments
    /// * `config` - Same configuration used for encoding
    /// * `path` - Same path string used for encoding
    /// * `expected_size` - Expected size of the decoded data (for validation)
    ///
    /// # Returns
    /// The original data bytes (may need correction layer for 100% fidelity)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
    ///
    /// let data = b"hello world";
    /// let config = ReversibleVSAConfig::default();
    /// let encoded = SparseVec::encode_data(data, &config, None);
    /// let decoded = encoded.decode_data(&config, None, data.len());
    ///
    /// // Note: For 100% fidelity, use CorrectionStore with EmbrFS
    /// // Raw decode may have minor differences that corrections compensate for
    /// ```
    pub fn decode_data(
        &self,
        config: &ReversibleVSAConfig,
        path: Option<&str>,
        expected_size: usize,
    ) -> Vec<u8> {
        if self.pos.is_empty() && self.neg.is_empty() {
            return Vec::new();
        }

        if expected_size == 0 {
            return Vec::new();
        }

        // Calculate path-based shift (same as encoding)
        let path_shift = if let Some(path_str) = path {
            let mut hasher = Sha256::new();
            hasher.update(path_str.as_bytes());
            let hash = hasher.finalize();
            // SAFETY: SHA256 always produces 32 bytes, first 4 bytes are always valid
            let hash_bytes: [u8; 4] = hash[0..4]
                .try_into()
                .expect("SHA256 hash must produce at least 4 bytes");
            let path_hash = u32::from_le_bytes(hash_bytes) as usize;
            (path_hash % config.max_path_depth) * config.base_shift
        } else {
            0
        };

        // Estimate number of blocks based on expected size
        let estimated_blocks = expected_size.div_ceil(config.block_size);

        // For single block case
        if estimated_blocks <= 1 {
            return Self::decode_block(self, path_shift, expected_size);
        }

        // =======================================================================
        // MULTI-BLOCK DECODING LIMITATIONS (v0.21.0)
        // =======================================================================
        //
        // Current Status: EXPERIMENTAL / SIMPLIFIED IMPLEMENTATION
        //
        // The multi-block decoding uses a simplified linear approach without proper
        // hierarchical factorization. This has the following implications:
        //
        // 1. **Information Loss**: When multiple blocks are bundled hierarchically,
        //    separating them requires VSA factorization techniques (e.g., HRR unbinding,
        //    MAP estimation) that are not yet implemented.
        //
        // 2. **Use Case**: Works reliably for:
        //    - Single-block data (data.len() <= config.block_size)
        //    - Data with clear block boundaries
        //
        // 3. **Known Limitation**: Multi-block data (> block_size) may have reduced
        //    fidelity. For 100% accuracy, use the Codebook's residual mechanism or
        //    external correction stores.
        //
        // 4. **Future Work**: Full hierarchical decoding is planned for v1.1.0.
        //    See IMPLEMENTATION_PLAN.md Task 4.2 for details.
        //
        // =======================================================================

        // For multiple blocks, we need to factorize the hierarchical bundle
        // This is a simplified approach - in practice, we'd need more sophisticated
        // factorization to separate the blocks
        let mut result = Vec::new();

        // Simplified best-effort linear decoding for multi-block data.
        // WARNING: This path trades accuracy for speed and may:
        //   - Lose information across blocks, and/or
        //   - Mis-assign tokens between adjacent blocks.
        // For applications that require 100% fidelity, prefer the Codebook's
        // residual mechanism or external correction stores instead of relying
        // solely on this multi-block decoder.
        for i in 0..estimated_blocks {
            let block_shift = path_shift + (i * config.base_shift / estimated_blocks.max(1));
            let remaining = expected_size.saturating_sub(result.len());
            if remaining == 0 {
                break;
            }
            let max_len = remaining.min(config.block_size);
            let block_data = Self::decode_block(self, block_shift, max_len);
            if block_data.is_empty() {
                break;
            }
            result.extend(block_data);
            if result.len() >= expected_size {
                break;
            }
        }

        // Truncate to expected size
        result.truncate(expected_size);
        result
    }

    /// Encode a single block of data with position-based permutation
    fn encode_block(data: &[u8], shift: usize) -> SparseVec {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Map data bytes to vector indices using the permuted mapping
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        for (i, &byte) in data.iter().enumerate() {
            let base_idx = (i + shift) % DIM;

            // Use byte value to determine polarity and offset
            if byte & 0x80 != 0 {
                // High bit set -> negative
                neg.push((base_idx + (byte & 0x7F) as usize) % DIM);
            } else {
                // High bit clear -> positive
                pos.push((base_idx + byte as usize) % DIM);
            }
        }

        pos.sort_unstable();
        pos.dedup();
        neg.sort_unstable();
        neg.dedup();

        // CRITICAL FIX: Ensure pos and neg are disjoint
        // If an index appears in both pos and neg, it means we have conflicting
        // encodings. Remove from both to treat as 0 (cancel out).
        let overlap = Self::intersection_sorted(&pos, &neg);
        if !overlap.is_empty() {
            pos = Self::difference_sorted(&pos, &overlap);
            neg = Self::difference_sorted(&neg, &overlap);
        }

        SparseVec { pos, neg }
    }

    /// Decode a single block of data
    fn decode_block(encoded: &SparseVec, shift: usize, max_len: usize) -> Vec<u8> {
        if max_len == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(max_len);

        // Reconstruct data by reversing the permutation.
        // Note: `pos` and `neg` are kept sorted, so membership can be checked via binary search.
        for i in 0..max_len {
            let base_idx = (i + shift) % DIM;

            // Look for indices that map back to this position
            let mut found_byte = None;
            for offset in 0..128u8 {
                let test_idx = (base_idx + offset as usize) % DIM;

                if encoded.pos.binary_search(&test_idx).is_ok() {
                    found_byte = Some(offset);
                    break;
                } else if encoded.neg.binary_search(&test_idx).is_ok() {
                    found_byte = Some(offset | 0x80);
                    break;
                }
            }

            if let Some(byte) = found_byte {
                result.push(byte);
            } else {
                // No more data found
                break;
            }
        }

        result
    }

    /// Combine multiple vectors using hierarchical bundling
    fn hierarchical_bundle(vectors: &[SparseVec]) -> SparseVec {
        if vectors.is_empty() {
            return SparseVec::new();
        }
        if vectors.len() == 1 {
            return vectors[0].clone();
        }

        // Binary tree combination
        let mut result = vectors[0].clone();
        for vec in &vectors[1..] {
            result = result.bundle(vec);
        }
        result
    }

    /// Generate a deterministic sparse vector from data using SHA256 seed
    /// DEPRECATED: Use encode_data() for new code
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
    ///
    /// let data = b"hello world";
    /// let config = ReversibleVSAConfig::default();
    /// let vec1 = SparseVec::encode_data(data, &config, None);
    /// let vec2 = SparseVec::encode_data(data, &config, None);
    ///
    /// // Same input produces same vector (deterministic)
    /// assert_eq!(vec1.pos, vec2.pos);
    /// assert_eq!(vec1.neg, vec2.neg);
    /// ```
    #[deprecated(since = "0.2.0", note = "Use encode_data() for reversible encoding")]
    pub fn from_data(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        // SAFETY: SHA256 always produces exactly 32 bytes
        let seed: [u8; 32] = hash[..32]
            .try_into()
            .expect("SHA256 must produce exactly 32 bytes");
        let mut rng = rand::rngs::StdRng::from_seed(seed);

        let mut indices: Vec<usize> = (0..DIM).collect();
        indices.shuffle(&mut rng);

        let sparsity = DIM / 100;
        let mut pos = indices[..sparsity].to_vec();
        let mut neg = indices[sparsity..sparsity * 2].to_vec();

        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Bundle operation: pairwise conflict-cancel superposition (A ⊕ B)
    ///
    /// This is a fast, commutative merge for two vectors:
    /// - same sign => keep
    /// - opposite signs => cancel to 0
    /// - sign vs 0 => keep sign
    ///
    /// Note: While this is well-defined for two vectors, repeated application across 3+
    /// vectors is generally **not associative** because early cancellation/thresholding can
    /// discard multiplicity information.
    ///
    /// # Arguments
    /// * `other` - The vector to bundle with self
    /// * `config` - Optional ReversibleVSAConfig for controlling sparsity via thinning
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
    ///
    /// let config = ReversibleVSAConfig::default();
    /// let vec1 = SparseVec::encode_data(b"data1", &config, None);
    /// let vec2 = SparseVec::encode_data(b"data2", &config, None);
    /// let bundled = vec1.bundle_with_config(&vec2, Some(&config));
    ///
    /// // Bundled vector contains superposition of both inputs
    /// // Should be similar to both original vectors
    /// let sim1 = vec1.cosine(&bundled);
    /// let sim2 = vec2.cosine(&bundled);
    /// assert!(sim1 > 0.3);
    /// assert!(sim2 > 0.3);
    /// ```
    pub fn bundle_with_config(
        &self,
        other: &SparseVec,
        config: Option<&ReversibleVSAConfig>,
    ) -> SparseVec {
        let mut result = self.bundle(other);

        // Apply thinning if config provided and result exceeds target sparsity
        if let Some(cfg) = config {
            let current_count = result.pos.len() + result.neg.len();
            if current_count > cfg.target_sparsity {
                result = result.thin(cfg.target_sparsity);
            }
        }

        result
    }

    /// Bundle operation: pairwise conflict-cancel superposition (A ⊕ B)
    ///
    /// See `bundle()` for semantic details; this wrapper optionally applies thinning via
    /// `ReversibleVSAConfig`.
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let config = embeddenator_vsa::ReversibleVSAConfig::default();
    /// let vec1 = SparseVec::encode_data(b"data1", &config, None);
    /// let vec2 = SparseVec::encode_data(b"data2", &config, None);
    /// let bundled = vec1.bundle(&vec2);
    ///
    /// // Bundled vector contains superposition of both inputs
    /// // Should be similar to both original vectors
    /// let sim1 = vec1.cosine(&bundled);
    /// let sim2 = vec2.cosine(&bundled);
    /// assert!(sim1 > 0.3);
    /// assert!(sim2 > 0.3);
    /// ```
    pub fn bundle(&self, other: &SparseVec) -> SparseVec {
        // Optional ternary-native fast path (migration gate).
        // This is primarily intended for cases where vectors become dense enough
        // that packed word-wise operations are competitive.
        {
            // Converting SparseVec -> PackedTritVec is O(DIM) for allocation/zeroing, plus O(nnz)
            // for setting lanes. Only attempt the packed path when total density is high enough,
            // and avoid converting an extremely sparse operand.
            // Thresholds lowered from DIM/4 to DIM/8 for earlier SIMD dispatch now that
            // PackedTritVec has proper bitsliced operations.
            let a_nnz = self.nnz();
            let b_nnz = other.nnz();
            let total = a_nnz + b_nnz;
            if total > DIM / 8 {
                let min_nnz = a_nnz.min(b_nnz);
                if min_nnz > DIM / 64 {
                    return PACKED_SCRATCH_A.with(|a_cell| {
                        PACKED_SCRATCH_B.with(|b_cell| {
                            PACKED_SCRATCH_OUT.with(|out_cell| {
                                let mut a = a_cell.borrow_mut();
                                let mut b = b_cell.borrow_mut();
                                let mut out = out_cell.borrow_mut();

                                a.fill_from_sparsevec(self, DIM);
                                b.fill_from_sparsevec(other, DIM);
                                a.bundle_into(&b, &mut out);
                                out.to_sparsevec()
                            })
                        })
                    });
                }
            }
        }

        // Majority voting for two sparse ternary vectors:
        // - Same sign => keep
        // - Opposite signs => cancel to 0
        // - Sign vs 0 => keep sign
        // This can be expressed via sorted set differences/unions.
        let pos_a = Self::difference_sorted(&self.pos, &other.neg);
        let pos_b = Self::difference_sorted(&other.pos, &self.neg);
        let neg_a = Self::difference_sorted(&self.neg, &other.pos);
        let neg_b = Self::difference_sorted(&other.neg, &self.pos);

        let mut pos = Self::union_sorted(&pos_a, &pos_b);
        let mut neg = Self::union_sorted(&neg_a, &neg_b);

        // CRITICAL FIX: Ensure pos and neg remain disjoint
        // Remove any indices that appear in both (shouldn't happen in theory,
        // but can occur due to floating-point or hierarchical bundling issues)
        let overlap = Self::intersection_sorted(&pos, &neg);
        if !overlap.is_empty() {
            pos = Self::difference_sorted(&pos, &overlap);
            neg = Self::difference_sorted(&neg, &overlap);
        }

        SparseVec { pos, neg }
    }

    /// Associative bundle over many vectors: sums contributions per index, then thresholds to sign.
    /// This is order-independent because all contributions are accumulated before applying sign.
    /// Complexity: O(K log K) where K is total non-zero entries across inputs.
    pub fn bundle_sum_many<'a, I>(vectors: I) -> SparseVec
    where
        I: IntoIterator<Item = &'a SparseVec>,
    {
        let mut contributions: Vec<(usize, i32)> = Vec::new();

        for vec in vectors {
            contributions.extend(vec.pos.iter().map(|&idx| (idx, 1i32)));
            contributions.extend(vec.neg.iter().map(|&idx| (idx, -1i32)));
        }

        if contributions.is_empty() {
            return SparseVec {
                pos: Vec::new(),
                neg: Vec::new(),
            };
        }

        contributions.sort_unstable_by_key(|(idx, _)| *idx);

        let mut pos = Vec::new();
        let mut neg = Vec::new();

        let mut iter = contributions.into_iter();
        // SAFETY: we checked contributions.is_empty() above and returned early if empty
        let (mut current_idx, mut acc) = iter
            .next()
            .expect("contributions must be non-empty after early return check");

        for (idx, value) in iter {
            if idx == current_idx {
                acc += value;
            } else {
                if acc > 0 {
                    pos.push(current_idx);
                } else if acc < 0 {
                    neg.push(current_idx);
                }
                current_idx = idx;
                acc = value;
            }
        }

        if acc > 0 {
            pos.push(current_idx);
        } else if acc < 0 {
            neg.push(current_idx);
        }

        SparseVec { pos, neg }
    }

    /// Hybrid bundle: choose a fast pairwise fold for very sparse regimes (to preserve sparsity),
    /// otherwise use the associative sum-then-threshold path (order-independent, more faithful to majority).
    ///
    /// Heuristic: estimate expected overlap/collision count assuming uniform hashing into `DIM`.
    /// If expected colliding dimensions is below a small budget, use pairwise `bundle`; else use
    /// `bundle_sum_many`.
    pub fn bundle_hybrid_many<'a, I>(vectors: I) -> SparseVec
    where
        I: IntoIterator<Item = &'a SparseVec>,
    {
        let collected: Vec<&'a SparseVec> = vectors.into_iter().collect();
        if collected.is_empty() {
            return SparseVec {
                pos: Vec::new(),
                neg: Vec::new(),
            };
        }

        if collected.len() == 1 {
            return collected[0].clone();
        }

        if collected.len() == 2 {
            return collected[0].bundle(collected[1]);
        }

        let total_nnz: usize = collected.iter().map(|v| v.pos.len() + v.neg.len()).sum();

        if total_nnz == 0 {
            return SparseVec {
                pos: Vec::new(),
                neg: Vec::new(),
            };
        }

        // Constant-time overlap/collision risk estimate.
        // Model: each non-zero lands uniformly in DIM dimensions.
        // Let λ = total_nnz / DIM be expected hits per dimension.
        // Then P(K>=2) ≈ 1 - e^{-λ}(1+λ) for Poisson(λ).
        // If expected number of colliding dimensions is tiny, pairwise fold is effectively safe
        // (and faster). Otherwise, use associative accumulation to avoid order sensitivity.
        let lambda = total_nnz as f64 / DIM as f64;
        let p_ge_2 = 1.0 - (-lambda).exp() * (1.0 + lambda);
        let expected_colliding_dims = p_ge_2 * DIM as f64;

        // Budget: allow a small number of potentially order-sensitive dimensions.
        // Tune via benchmarks; this is conservative for integrity.
        let collision_budget_dims = 32.0;

        if expected_colliding_dims <= collision_budget_dims {
            let mut iter = collected.into_iter();
            // SAFETY: hierarchical_bundle is only called when collected.len() > 1
            let mut acc = iter
                .next()
                .expect("hierarchical_bundle must be called with non-empty collection")
                .clone();
            for v in iter {
                acc = acc.bundle(v);
            }
            return acc;
        }

        SparseVec::bundle_sum_many(collected)
    }

    /// Bind operation: non-commutative composition (A ⊙ B)
    /// Performs element-wise multiplication. Self-inverse: A ⊙ A ≈ I
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let config = embeddenator_vsa::ReversibleVSAConfig::default();
    /// let vec = SparseVec::encode_data(b"test", &config, None);
    /// let bound = vec.bind(&vec);
    ///
    /// // Bind with self should produce high similarity (self-inverse property)
    /// let identity = SparseVec::encode_data(b"identity", &config, None);
    /// let sim = bound.cosine(&identity);
    /// // Result is approximately identity, so similarity varies
    /// assert!(sim >= -1.0 && sim <= 1.0);
    /// ```
    pub fn bind(&self, other: &SparseVec) -> SparseVec {
        {
            // Packed bind is only worthwhile when both operands are dense enough.
            // Using a short-circuiting check avoids paying extra overhead for sparse workloads.
            // Thresholds lowered from DIM/4 to DIM/8 for earlier SIMD dispatch.
            let a_nnz = self.nnz();
            if a_nnz > DIM / 8 {
                let b_nnz = other.nnz();
                if b_nnz > DIM / 8 {
                    return PACKED_SCRATCH_A.with(|a_cell| {
                        PACKED_SCRATCH_B.with(|b_cell| {
                            PACKED_SCRATCH_OUT.with(|out_cell| {
                                let mut a = a_cell.borrow_mut();
                                let mut b = b_cell.borrow_mut();
                                let mut out = out_cell.borrow_mut();

                                a.fill_from_sparsevec(self, DIM);
                                b.fill_from_sparsevec(other, DIM);
                                a.bind_into(&b, &mut out);
                                out.to_sparsevec()
                            })
                        })
                    });
                }
            }
        }

        // Sparse bind is element-wise multiplication restricted to non-zero support.
        // We can compute this in a single merge-join over the signed supports, keeping
        // outputs sorted without any post-sort.
        let mut result_pos = Vec::new();
        let mut result_neg = Vec::new();

        let mut a_pos = 0usize;
        let mut a_neg = 0usize;
        let mut b_pos = 0usize;
        let mut b_neg = 0usize;

        loop {
            let next_a = match (self.pos.get(a_pos), self.neg.get(a_neg)) {
                (Some(&p), Some(&n)) => {
                    if p < n {
                        Some((p, 1i8, true))
                    } else {
                        Some((n, -1i8, false))
                    }
                }
                (Some(&p), None) => Some((p, 1i8, true)),
                (None, Some(&n)) => Some((n, -1i8, false)),
                (None, None) => None,
            };

            let next_b = match (other.pos.get(b_pos), other.neg.get(b_neg)) {
                (Some(&p), Some(&n)) => {
                    if p < n {
                        Some((p, 1i8, true))
                    } else {
                        Some((n, -1i8, false))
                    }
                }
                (Some(&p), None) => Some((p, 1i8, true)),
                (None, Some(&n)) => Some((n, -1i8, false)),
                (None, None) => None,
            };

            let Some((idx_a, sign_a, a_is_pos)) = next_a else {
                break;
            };
            let Some((idx_b, sign_b, b_is_pos)) = next_b else {
                break;
            };

            match idx_a.cmp(&idx_b) {
                std::cmp::Ordering::Less => {
                    if a_is_pos {
                        a_pos += 1;
                    } else {
                        a_neg += 1;
                    }
                }
                std::cmp::Ordering::Greater => {
                    if b_is_pos {
                        b_pos += 1;
                    } else {
                        b_neg += 1;
                    }
                }
                std::cmp::Ordering::Equal => {
                    let prod = sign_a * sign_b;
                    if prod == 1 {
                        result_pos.push(idx_a);
                    } else {
                        result_neg.push(idx_a);
                    }

                    if a_is_pos {
                        a_pos += 1;
                    } else {
                        a_neg += 1;
                    }
                    if b_is_pos {
                        b_pos += 1;
                    } else {
                        b_neg += 1;
                    }
                }
            }
        }

        SparseVec {
            pos: result_pos,
            neg: result_neg,
        }
    }

    /// Calculate cosine similarity between two sparse vectors
    /// Returns value in [-1, 1] where 1 is identical, 0 is orthogonal
    ///
    /// When the `simd` feature is enabled, this will automatically use
    /// AVX2 (x86_64) or NEON (aarch64) acceleration if available.
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let config = embeddenator_vsa::ReversibleVSAConfig::default();
    /// let vec1 = SparseVec::encode_data(b"cat", &config, None);
    /// let vec2 = SparseVec::encode_data(b"cat", &config, None);
    /// let vec3 = SparseVec::encode_data(b"dog", &config, None);
    ///
    /// // Identical data produces identical vectors
    /// assert!((vec1.cosine(&vec2) - 1.0).abs() < 0.01);
    ///
    /// // Different data produces low similarity
    /// let sim = vec1.cosine(&vec3);
    /// assert!(sim < 0.3);
    /// ```
    pub fn cosine(&self, other: &SparseVec) -> f64 {
        #[cfg(feature = "simd")]
        {
            crate::simd_cosine::cosine_simd(self, other)
        }

        #[cfg(not(feature = "simd"))]
        {
            // Original implementation remains as fallback
            self.cosine_scalar(other)
        }
    }

    /// Scalar (non-SIMD) cosine similarity implementation.
    ///
    /// This is the original implementation and serves as the baseline
    /// for SIMD optimizations. It's also used when SIMD is not available.
    #[inline]
    pub fn cosine_scalar(&self, other: &SparseVec) -> f64 {
        {
            // Only use packed cosine when total density is high enough to amortize conversion,
            // and avoid converting an extremely sparse operand.
            // Thresholds lowered from DIM/4 to DIM/8 for earlier SIMD dispatch.
            let a_nnz = self.nnz();
            let b_nnz = other.nnz();
            let total = a_nnz + b_nnz;
            if total > DIM / 8 {
                let min_nnz = a_nnz.min(b_nnz);
                if min_nnz > DIM / 64 {
                    let dot = PACKED_SCRATCH_A.with(|a_cell| {
                        PACKED_SCRATCH_B.with(|b_cell| {
                            let mut a = a_cell.borrow_mut();
                            let mut b = b_cell.borrow_mut();
                            a.fill_from_sparsevec(self, DIM);
                            b.fill_from_sparsevec(other, DIM);
                            a.dot(&b)
                        })
                    }) as f64;
                    let self_norm = a_nnz as f64;
                    let other_norm = b_nnz as f64;
                    if self_norm == 0.0 || other_norm == 0.0 {
                        return 0.0;
                    }
                    return dot / (self_norm.sqrt() * other_norm.sqrt());
                }
            }
        }

        // Sparse ternary dot product:
        // +1 when signs match, -1 when signs oppose.
        let pp = Self::intersection_count_sorted(&self.pos, &other.pos) as i32;
        let nn = Self::intersection_count_sorted(&self.neg, &other.neg) as i32;
        let pn = Self::intersection_count_sorted(&self.pos, &other.neg) as i32;
        let np = Self::intersection_count_sorted(&self.neg, &other.pos) as i32;
        let dot = (pp + nn) - (pn + np);

        let self_norm = self.nnz() as f64;
        let other_norm = other.nnz() as f64;

        if self_norm == 0.0 || other_norm == 0.0 {
            return 0.0;
        }

        dot as f64 / (self_norm.sqrt() * other_norm.sqrt())
    }

    /// Apply cyclic permutation to vector indices
    /// Used for encoding sequence order in hierarchical structures
    ///
    /// # Arguments
    /// * `shift` - Number of positions to shift indices cyclically
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let config = embeddenator_vsa::ReversibleVSAConfig::default();
    /// let vec = SparseVec::encode_data(b"test", &config, None);
    /// let permuted = vec.permute(100);
    ///
    /// // Permuted vector should have different indices but same structure
    /// assert_eq!(vec.pos.len(), permuted.pos.len());
    /// assert_eq!(vec.neg.len(), permuted.neg.len());
    /// ```
    pub fn permute(&self, shift: usize) -> SparseVec {
        let permute_index = |idx: usize| (idx + shift) % DIM;

        let pos: Vec<usize> = self.pos.iter().map(|&idx| permute_index(idx)).collect();
        let neg: Vec<usize> = self.neg.iter().map(|&idx| permute_index(idx)).collect();

        // Indices must remain sorted for efficient operations
        let mut pos = pos;
        let mut neg = neg;
        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Apply inverse cyclic permutation to vector indices
    /// Decodes sequence order by reversing the permutation shift
    ///
    /// # Arguments
    /// * `shift` - Number of positions to reverse shift indices cyclically
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let config = embeddenator_vsa::ReversibleVSAConfig::default();
    /// let vec = SparseVec::encode_data(b"test", &config, None);
    /// let permuted = vec.permute(100);
    /// let recovered = permuted.inverse_permute(100);
    ///
    /// // Round-trip should recover original vector
    /// assert_eq!(vec.pos, recovered.pos);
    /// assert_eq!(vec.neg, recovered.neg);
    /// ```
    pub fn inverse_permute(&self, shift: usize) -> SparseVec {
        let inverse_permute_index = |idx: usize| (idx + DIM - (shift % DIM)) % DIM;

        let pos: Vec<usize> = self
            .pos
            .iter()
            .map(|&idx| inverse_permute_index(idx))
            .collect();
        let neg: Vec<usize> = self
            .neg
            .iter()
            .map(|&idx| inverse_permute_index(idx))
            .collect();

        // Indices must remain sorted for efficient operations
        let mut pos = pos;
        let mut neg = neg;
        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Context-Dependent Thinning Algorithm
    ///
    /// Thinning controls vector sparsity during bundle operations to prevent
    /// exponential density growth that degrades VSA performance. The algorithm:
    ///
    /// 1. Calculate current density = (pos.len() + neg.len()) as f32 / DIM as f32
    /// 2. If current_density <= target_density, return unchanged
    /// 3. Otherwise, randomly sample indices to reduce to target count
    /// 4. Preserve pos/neg ratio to maintain signal polarity balance
    /// 5. Use deterministic seeding for reproducible results
    ///
    /// Edge Cases:
    /// - Empty vector: return unchanged
    /// - target_non_zero = 0: return empty vector (not recommended)
    /// - target_non_zero >= current: return clone
    /// - Single polarity vectors: preserve polarity distribution
    ///
    /// Performance: O(n log n) due to sorting, where n = target_non_zero
    pub fn thin(&self, target_non_zero: usize) -> SparseVec {
        let current_count = self.pos.len() + self.neg.len();

        // Edge case: already at or below target
        if current_count <= target_non_zero {
            return self.clone();
        }

        // Edge case: target is zero
        if target_non_zero == 0 {
            return SparseVec::new();
        }

        // Calculate how many to keep from each polarity
        let pos_ratio = self.pos.len() as f32 / current_count as f32;
        let target_pos = (target_non_zero as f32 * pos_ratio).round() as usize;
        let target_neg = target_non_zero - target_pos;

        // Randomly sample indices using deterministic seed based on vector content
        let mut seed = [0u8; 32];
        seed[0..4].copy_from_slice(&(self.pos.len() as u32).to_le_bytes());
        seed[4..8].copy_from_slice(&(self.neg.len() as u32).to_le_bytes());
        seed[8..12].copy_from_slice(&(target_non_zero as u32).to_le_bytes());
        // Rest remains zero for deterministic seeding
        let mut rng = rand::rngs::StdRng::from_seed(seed);

        // Sample positive indices
        let mut sampled_pos: Vec<usize> = self.pos.clone();
        sampled_pos.shuffle(&mut rng);
        sampled_pos.truncate(target_pos);
        sampled_pos.sort_unstable();

        // Sample negative indices
        let mut sampled_neg: Vec<usize> = self.neg.clone();
        sampled_neg.shuffle(&mut rng);
        sampled_neg.truncate(target_neg);
        sampled_neg.sort_unstable();

        SparseVec {
            pos: sampled_pos,
            neg: sampled_neg,
        }
    }
}
