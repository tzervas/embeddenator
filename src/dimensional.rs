//! Dimensional Configuration - Variable Precision Hyperdimensional Substrate
//!
//! This module implements configurable dimensional encoding where:
//! - Number of dimensions is tunable (sparse vs dense tradeoff)
//! - Trit depth per dimension is variable (precision vs storage tradeoff)
//! - Differential encoding reduces actual storage to deltas from codebook
//!
//! # Mathematical Foundation
//!
//! Each dimension `d` has a configurable trit depth `T_d`:
//! - 1 trit  = 3 states {-1, 0, +1} = 1.585 bits
//! - 2 trits = 9 states = 3.17 bits
//! - 3 trits = 27 states = 4.75 bits
//! - N trits = 3^N states = N × 1.585 bits
//!
//! Total information capacity: Σ(T_d × 1.585) bits across all dimensions
//!
//! # Algebraic Primitives
//!
//! These operations form the low-level compute substrate:
//! - **Bundle (⊕)**: Superposition - trit-wise majority voting
//! - **Bind (⊙)**: Composition - trit-wise multiplication  
//! - **Permute (ρ)**: Sequence encoding - cyclic index shift
//! - **Cosine**: Similarity - normalized dot product
//! - **Factorize**: Decomposition - resonator-based unbinding
//!
//! All operations preserve balanced ternary properties and are algebraically closed.

use serde::{Deserialize, Serialize};
use std::ops::{Mul, Neg};

/// Single trit: balanced ternary digit {-1, 0, +1}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i8)]
pub enum Trit {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl Trit {
    /// Multiply two trits (binding operation at trit level)
    #[inline]
    pub fn mul(self, other: Trit) -> Trit {
        match (self, other) {
            (Trit::Zero, _) | (_, Trit::Zero) => Trit::Zero,
            (Trit::Pos, Trit::Pos) | (Trit::Neg, Trit::Neg) => Trit::Pos,
            (Trit::Pos, Trit::Neg) | (Trit::Neg, Trit::Pos) => Trit::Neg,
        }
    }

    /// Add two trits with carry (returns sum, carry)
    #[inline]
    pub fn add_with_carry(self, other: Trit, carry_in: Trit) -> (Trit, Trit) {
        let sum = (self as i8) + (other as i8) + (carry_in as i8);
        match sum {
            -3 => (Trit::Zero, Trit::Neg),
            -2 => (Trit::Pos, Trit::Neg),
            -1 => (Trit::Neg, Trit::Zero),
            0 => (Trit::Zero, Trit::Zero),
            1 => (Trit::Pos, Trit::Zero),
            2 => (Trit::Neg, Trit::Pos),
            3 => (Trit::Zero, Trit::Pos),
            _ => unreachable!(),
        }
    }

    /// Negate a trit
    #[inline]
    pub fn neg(self) -> Trit {
        match self {
            Trit::Neg => Trit::Pos,
            Trit::Zero => Trit::Zero,
            Trit::Pos => Trit::Neg,
        }
    }

    /// Convert from i8
    #[inline]
    pub fn from_i8(v: i8) -> Self {
        match v.signum() {
            -1 => Trit::Neg,
            0 => Trit::Zero,
            1 => Trit::Pos,
            _ => unreachable!(),
        }
    }

    /// Convert to i8
    #[inline]
    pub fn to_i8(self) -> i8 {
        self as i8
    }
}

impl Neg for Trit {
    type Output = Trit;
    fn neg(self) -> Trit {
        Trit::neg(self)
    }
}

impl Mul for Trit {
    type Output = Trit;
    fn mul(self, rhs: Trit) -> Trit {
        Trit::mul(self, rhs)
    }
}

/// Tryte: A group of trits (configurable size)
/// Default is 6 trits = 729 states ≈ 9.51 bits
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tryte {
    /// The trits composing this tryte (LST first)
    pub trits: Vec<Trit>,
}

impl Tryte {
    /// Create a zero tryte of given size
    pub fn zero(num_trits: usize) -> Self {
        Tryte {
            trits: vec![Trit::Zero; num_trits],
        }
    }

    /// Create from integer value
    pub fn from_i64(mut value: i64, num_trits: usize) -> Self {
        let mut trits = Vec::with_capacity(num_trits);
        let negative = value < 0;
        if negative {
            value = -value;
        }

        for _ in 0..num_trits {
            let remainder = (value % 3) as i8;
            value /= 3;

            // Balanced ternary conversion
            let trit = match remainder {
                0 => Trit::Zero,
                1 => Trit::Pos,
                2 => {
                    value += 1; // Carry
                    Trit::Neg
                }
                _ => unreachable!(),
            };
            trits.push(if negative { -trit } else { trit });
        }

        Tryte { trits }
    }

    /// Convert to integer value
    pub fn to_i64(&self) -> i64 {
        let mut result: i64 = 0;
        let mut power: i64 = 1;

        for trit in &self.trits {
            result += trit.to_i8() as i64 * power;
            power *= 3;
        }

        result
    }

    /// Get the number of trits
    pub fn len(&self) -> usize {
        self.trits.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.trits.is_empty()
    }

    /// Maximum representable value for this tryte size
    pub fn max_value(num_trits: usize) -> i64 {
        (3i64.pow(num_trits as u32) - 1) / 2
    }

    /// Minimum representable value for this tryte size  
    pub fn min_value(num_trits: usize) -> i64 {
        -Self::max_value(num_trits)
    }

    /// Trit-wise multiplication (bind at tryte level)
    pub fn bind(&self, other: &Tryte) -> Tryte {
        assert_eq!(self.len(), other.len(), "Tryte sizes must match for bind");
        let trits = self.trits.iter()
            .zip(other.trits.iter())
            .map(|(&a, &b)| a * b)
            .collect();
        Tryte { trits }
    }

    /// Trit-wise majority voting (bundle at tryte level)
    pub fn bundle(&self, other: &Tryte) -> Tryte {
        assert_eq!(self.len(), other.len(), "Tryte sizes must match for bundle");
        let trits = self.trits.iter()
            .zip(other.trits.iter())
            .map(|(&a, &b)| {
                let sum = a.to_i8() + b.to_i8();
                Trit::from_i8(sum.signum())
            })
            .collect();
        Tryte { trits }
    }

    /// Dot product contribution (for cosine similarity)
    pub fn dot(&self, other: &Tryte) -> i64 {
        self.trits.iter()
            .zip(other.trits.iter())
            .map(|(&a, &b)| (a.to_i8() * b.to_i8()) as i64)
            .sum()
    }

    /// L2 norm squared (count of non-zero trits)
    pub fn norm_squared(&self) -> i64 {
        self.trits.iter()
            .map(|&t| if t == Trit::Zero { 0 } else { 1 })
            .sum()
    }
}

/// Configuration for dimensional encoding
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DimensionalConfig {
    /// Number of dimensions in the vector space
    pub num_dimensions: usize,

    /// Trit depth per dimension (uniform or variable)
    pub trit_depth: TritDepthConfig,

    /// Sparsity target (fraction of non-zero dimensions)
    pub target_sparsity: f64,

    /// Whether to use adaptive precision
    pub adaptive_precision: bool,
}

/// Configuration for trit depth across dimensions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TritDepthConfig {
    /// All dimensions have the same trit depth
    Uniform(u8),

    /// Variable trit depth per dimension
    Variable(Vec<u8>),

    /// Adaptive: start with base, expand where needed
    Adaptive {
        base_depth: u8,
        max_depth: u8,
    },
}

impl Default for DimensionalConfig {
    fn default() -> Self {
        DimensionalConfig {
            num_dimensions: 10_000,
            trit_depth: TritDepthConfig::Uniform(6), // 6 trits = 729 states
            target_sparsity: 0.02, // 2% non-zero
            adaptive_precision: false,
        }
    }
}

impl DimensionalConfig {
    /// Create a high-precision configuration
    pub fn high_precision() -> Self {
        DimensionalConfig {
            num_dimensions: 100_000,
            trit_depth: TritDepthConfig::Uniform(8), // 8 trits = 6561 states
            target_sparsity: 0.002,
            adaptive_precision: true,
        }
    }

    /// Create a compact configuration for constrained environments
    pub fn compact() -> Self {
        DimensionalConfig {
            num_dimensions: 4_096,
            trit_depth: TritDepthConfig::Uniform(4), // 4 trits = 81 states
            target_sparsity: 0.05,
            adaptive_precision: false,
        }
    }

    /// Create adaptive configuration
    pub fn adaptive(num_dims: usize, base_depth: u8, max_depth: u8) -> Self {
        DimensionalConfig {
            num_dimensions: num_dims,
            trit_depth: TritDepthConfig::Adaptive { base_depth, max_depth },
            target_sparsity: 200.0 / num_dims as f64,
            adaptive_precision: true,
        }
    }

    /// Get trit depth for a specific dimension
    pub fn depth_for_dimension(&self, dim: usize) -> u8 {
        match &self.trit_depth {
            TritDepthConfig::Uniform(d) => *d,
            TritDepthConfig::Variable(depths) => depths.get(dim).copied().unwrap_or(6),
            TritDepthConfig::Adaptive { base_depth, .. } => *base_depth,
        }
    }

    /// Calculate total information capacity in bits
    pub fn total_capacity_bits(&self) -> f64 {
        let log2_3: f64 = 3.0f64.log2(); // ≈ 1.585
        
        match &self.trit_depth {
            TritDepthConfig::Uniform(d) => {
                self.num_dimensions as f64 * (*d as f64) * log2_3
            }
            TritDepthConfig::Variable(depths) => {
                depths.iter().map(|&d| d as f64 * log2_3).sum()
            }
            TritDepthConfig::Adaptive { base_depth, .. } => {
                self.num_dimensions as f64 * (*base_depth as f64) * log2_3
            }
        }
    }

    /// Calculate expected storage size in bytes (sparse representation)
    pub fn expected_storage_bytes(&self) -> usize {
        let non_zero_dims = (self.num_dimensions as f64 * self.target_sparsity) as usize;
        let avg_depth = match &self.trit_depth {
            TritDepthConfig::Uniform(d) => *d as usize,
            TritDepthConfig::Variable(depths) => {
                depths.iter().map(|&d| d as usize).sum::<usize>() / depths.len().max(1)
            }
            TritDepthConfig::Adaptive { base_depth, .. } => *base_depth as usize,
        };

        // Index (4 bytes) + trits (ceil(avg_depth * 1.585 / 8)) per non-zero
        non_zero_dims * (4 + (avg_depth * 2 + 7) / 8)
    }
}

/// A hyperdimensional vector with configurable dimensional depth
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyperVec {
    /// Configuration for this vector
    pub config: DimensionalConfig,

    /// Sparse storage: dimension index -> tryte value
    /// Only non-zero dimensions are stored
    pub dimensions: std::collections::BTreeMap<usize, Tryte>,
}

impl HyperVec {
    /// Create a new zero vector
    pub fn new(config: DimensionalConfig) -> Self {
        HyperVec {
            config,
            dimensions: std::collections::BTreeMap::new(),
        }
    }

    /// Create from dense representation
    pub fn from_dense(config: DimensionalConfig, values: &[i64]) -> Self {
        let mut vec = HyperVec::new(config.clone());
        
        for (dim, &value) in values.iter().enumerate() {
            if dim >= config.num_dimensions {
                break;
            }
            if value != 0 {
                let depth = config.depth_for_dimension(dim);
                let tryte = Tryte::from_i64(value, depth as usize);
                vec.dimensions.insert(dim, tryte);
            }
        }
        
        vec
    }

    /// Get value at dimension (returns 0 for unset dimensions)
    pub fn get(&self, dim: usize) -> i64 {
        self.dimensions
            .get(&dim)
            .map(|t| t.to_i64())
            .unwrap_or(0)
    }

    /// Set value at dimension
    pub fn set(&mut self, dim: usize, value: i64) {
        if value == 0 {
            self.dimensions.remove(&dim);
        } else {
            let depth = self.config.depth_for_dimension(dim);
            let tryte = Tryte::from_i64(value, depth as usize);
            self.dimensions.insert(dim, tryte);
        }
    }

    /// Number of non-zero dimensions (sparsity count)
    pub fn nnz(&self) -> usize {
        self.dimensions.len()
    }

    /// Current sparsity ratio
    pub fn sparsity(&self) -> f64 {
        self.dimensions.len() as f64 / self.config.num_dimensions as f64
    }

    /// Bundle operation (⊕): Superposition via majority voting
    /// 
    /// Algebraic properties:
    /// - Associative: (A ⊕ B) ⊕ C = A ⊕ (B ⊕ C)
    /// - Commutative: A ⊕ B = B ⊕ A
    /// - Preserves similarity to inputs
    pub fn bundle(&self, other: &HyperVec) -> HyperVec {
        assert_eq!(self.config.num_dimensions, other.config.num_dimensions,
                   "Dimension count must match for bundle");

        let mut result = HyperVec::new(self.config.clone());
        
        // Union of all dimension indices
        let all_dims: std::collections::BTreeSet<_> = self.dimensions.keys()
            .chain(other.dimensions.keys())
            .copied()
            .collect();

        for dim in all_dims {
            let depth = self.config.depth_for_dimension(dim);
            let zero = Tryte::zero(depth as usize);
            
            let a = self.dimensions.get(&dim).unwrap_or(&zero);
            let b = other.dimensions.get(&dim).unwrap_or(&zero);
            
            let bundled = a.bundle(b);
            
            // Only store if non-zero
            if bundled.to_i64() != 0 {
                result.dimensions.insert(dim, bundled);
            }
        }

        result
    }

    /// Bind operation (⊙): Composition via trit-wise multiplication
    ///
    /// Algebraic properties:
    /// - Self-inverse: A ⊙ A ≈ I (identity, up to sparsity)
    /// - Distributes over bundle: A ⊙ (B ⊕ C) = (A ⊙ B) ⊕ (A ⊙ C)
    /// - Creates quasi-orthogonal result
    pub fn bind(&self, other: &HyperVec) -> HyperVec {
        assert_eq!(self.config.num_dimensions, other.config.num_dimensions,
                   "Dimension count must match for bind");

        let mut result = HyperVec::new(self.config.clone());

        // Only dimensions present in BOTH vectors contribute
        for (dim, tryte_a) in &self.dimensions {
            if let Some(tryte_b) = other.dimensions.get(dim) {
                let bound = tryte_a.bind(tryte_b);
                if bound.to_i64() != 0 {
                    result.dimensions.insert(*dim, bound);
                }
            }
        }

        result
    }

    /// Permute operation (ρ): Cyclic index shift for sequence encoding
    ///
    /// Algebraic properties:
    /// - ρ^n(A) cycles through orthogonal representations
    /// - Preserves sparsity and magnitude
    /// - Enables positional encoding
    pub fn permute(&self, shift: usize) -> HyperVec {
        let mut result = HyperVec::new(self.config.clone());
        let n = self.config.num_dimensions;

        for (&dim, tryte) in &self.dimensions {
            let new_dim = (dim + shift) % n;
            result.dimensions.insert(new_dim, tryte.clone());
        }

        result
    }

    /// Inverse permute (ρ⁻¹): Reverse cyclic shift
    pub fn inverse_permute(&self, shift: usize) -> HyperVec {
        let n = self.config.num_dimensions;
        self.permute(n - (shift % n))
    }

    /// Cosine similarity: Normalized dot product
    /// 
    /// Returns value in [-1, 1]:
    /// - 1.0: Identical vectors
    /// - 0.0: Orthogonal vectors
    /// - -1.0: Opposite vectors
    pub fn cosine(&self, other: &HyperVec) -> f64 {
        let mut dot: i64 = 0;
        let mut norm_a: i64 = 0;
        let mut norm_b: i64 = 0;

        // Calculate norms
        for (_, tryte) in &self.dimensions {
            norm_a += tryte.norm_squared();
        }
        for (_, tryte) in &other.dimensions {
            norm_b += tryte.norm_squared();
        }

        // Calculate dot product (only overlapping dimensions contribute)
        for (dim, tryte_a) in &self.dimensions {
            if let Some(tryte_b) = other.dimensions.get(dim) {
                dot += tryte_a.dot(tryte_b);
            }
        }

        if norm_a == 0 || norm_b == 0 {
            return 0.0;
        }

        dot as f64 / ((norm_a as f64).sqrt() * (norm_b as f64).sqrt())
    }

    /// Thin to target sparsity (Context-Dependent Thinning)
    /// 
    /// Reduces vector density while preserving signal structure
    pub fn thin(&self, target_nnz: usize) -> HyperVec {
        if self.nnz() <= target_nnz {
            return self.clone();
        }

        let mut result = HyperVec::new(self.config.clone());

        // Sort by magnitude, keep highest
        let mut indexed: Vec<_> = self.dimensions.iter()
            .map(|(&d, t)| (d, t.clone(), t.to_i64().abs()))
            .collect();
        
        indexed.sort_by(|a, b| b.2.cmp(&a.2));

        for (dim, tryte, _) in indexed.into_iter().take(target_nnz) {
            result.dimensions.insert(dim, tryte);
        }

        result
    }

    /// Expand precision at specific dimension (adaptive depth)
    pub fn expand_precision(&mut self, dim: usize, new_depth: u8) {
        if let TritDepthConfig::Adaptive { max_depth, .. } = &self.config.trit_depth {
            if new_depth > *max_depth {
                return; // Can't exceed max
            }
        }

        if let Some(tryte) = self.dimensions.get(&dim) {
            let value = tryte.to_i64();
            let new_tryte = Tryte::from_i64(value, new_depth as usize);
            self.dimensions.insert(dim, new_tryte);
        }
    }

    /// Pack to bytes for storage/transmission
    pub fn pack(&self) -> Vec<u8> {
        // Format: [num_entries: u32][entries...]
        // Entry: [dim: u32][num_trits: u8][trit_data...]
        let mut bytes = Vec::new();
        
        // Number of entries
        bytes.extend((self.dimensions.len() as u32).to_le_bytes());
        
        for (&dim, tryte) in &self.dimensions {
            // Dimension index
            bytes.extend((dim as u32).to_le_bytes());
            // Number of trits
            bytes.push(tryte.len() as u8);
            // Pack trits: 5 trits per byte (3^5 = 243 < 256)
            let packed_trits = pack_trits(&tryte.trits);
            bytes.extend(packed_trits);
        }
        
        bytes
    }

    /// Unpack from bytes
    pub fn unpack(config: DimensionalConfig, bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let num_entries = u32::from_le_bytes(bytes[0..4].try_into().ok()?) as usize;
        let mut vec = HyperVec::new(config);
        let mut offset = 4;

        for _ in 0..num_entries {
            if offset + 5 > bytes.len() {
                return None;
            }

            let dim = u32::from_le_bytes(bytes[offset..offset+4].try_into().ok()?) as usize;
            offset += 4;

            let num_trits = bytes[offset] as usize;
            offset += 1;

            let packed_bytes = (num_trits + 4) / 5; // 5 trits per byte
            if offset + packed_bytes > bytes.len() {
                return None;
            }

            let trits = unpack_trits(&bytes[offset..offset+packed_bytes], num_trits);
            offset += packed_bytes;

            vec.dimensions.insert(dim, Tryte { trits });
        }

        Some(vec)
    }
}

/// Pack trits into bytes (5 trits per byte)
fn pack_trits(trits: &[Trit]) -> Vec<u8> {
    let mut bytes = Vec::new();
    
    for chunk in trits.chunks(5) {
        let mut byte: u8 = 0;
        let mut power: u8 = 1;
        
        for &trit in chunk {
            // Encode: -1 -> 2, 0 -> 0, +1 -> 1
            let encoded = match trit {
                Trit::Neg => 2,
                Trit::Zero => 0,
                Trit::Pos => 1,
            };
            byte += encoded * power;
            power *= 3;
        }
        
        bytes.push(byte);
    }
    
    bytes
}

/// Unpack bytes to trits
fn unpack_trits(bytes: &[u8], num_trits: usize) -> Vec<Trit> {
    let mut trits = Vec::with_capacity(num_trits);
    
    for &byte in bytes {
        let mut remaining = byte;
        for _ in 0..5 {
            if trits.len() >= num_trits {
                break;
            }
            
            let encoded = remaining % 3;
            remaining /= 3;
            
            // Decode: 2 -> -1, 0 -> 0, 1 -> +1
            let trit = match encoded {
                2 => Trit::Neg,
                0 => Trit::Zero,
                1 => Trit::Pos,
                _ => unreachable!(),
            };
            trits.push(trit);
        }
    }
    
    trits
}

/// Differential encoding result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DifferentialEncoding {
    /// Coefficients against codebook basis (sparse)
    pub coefficients: HyperVec,
    
    /// Residual that couldn't be captured by basis
    pub residual: HyperVec,
    
    /// Dimensions that needed precision expansion
    pub expanded_dims: Vec<(usize, u8)>,
    
    /// Reconstruction quality (1.0 = perfect)
    pub quality: f64,
}

/// Differential encoder using a codebook basis
pub struct DifferentialEncoder {
    /// Configuration
    pub config: DimensionalConfig,
    
    /// Basis vectors (the codebook)
    pub basis: Vec<HyperVec>,
    
    /// Threshold for considering a match
    pub match_threshold: f64,
    
    /// Threshold for expanding precision
    pub precision_threshold: f64,
}

impl DifferentialEncoder {
    /// Create a new encoder
    pub fn new(config: DimensionalConfig) -> Self {
        DifferentialEncoder {
            config,
            basis: Vec::new(),
            match_threshold: 0.3,
            precision_threshold: 0.001,
        }
    }

    /// Add a basis vector to the codebook
    pub fn add_basis(&mut self, vector: HyperVec) {
        self.basis.push(vector);
    }

    /// Encode data differentially against the codebook
    pub fn encode(&self, data: &HyperVec) -> DifferentialEncoding {
        let mut coefficients = HyperVec::new(self.config.clone());
        let mut expanded_dims = Vec::new();
        
        // Project data onto each basis vector
        for (basis_idx, basis_vec) in self.basis.iter().enumerate() {
            let similarity = data.cosine(basis_vec);
            
            if similarity.abs() > self.match_threshold {
                // Quantize coefficient to balanced ternary
                let coef_value = (similarity * 100.0) as i64;
                coefficients.set(basis_idx, coef_value);
            }
        }

        // Compute residual (what basis couldn't capture)
        let reconstructed = self.reconstruct_from_coefficients(&coefficients);
        let mut residual = HyperVec::new(self.config.clone());
        
        for dim in 0..self.config.num_dimensions {
            let original_val = data.get(dim);
            let reconstructed_val = reconstructed.get(dim);
            let diff = original_val - reconstructed_val;
            
            if diff.abs() > 0 {
                residual.set(dim, diff);
                
                // Check if we need more precision
                let relative_error = if original_val != 0 {
                    (diff as f64).abs() / (original_val as f64).abs()
                } else {
                    0.0
                };
                
                if relative_error > self.precision_threshold {
                    if let TritDepthConfig::Adaptive { base_depth, max_depth } = &self.config.trit_depth {
                        let new_depth = (*base_depth + 2).min(*max_depth);
                        expanded_dims.push((dim, new_depth));
                    }
                }
            }
        }

        // Calculate quality
        let quality = self.calculate_quality(data, &coefficients, &residual);

        DifferentialEncoding {
            coefficients,
            residual,
            expanded_dims,
            quality,
        }
    }

    /// Reconstruct from coefficients only
    fn reconstruct_from_coefficients(&self, coefficients: &HyperVec) -> HyperVec {
        let mut result = HyperVec::new(self.config.clone());
        
        for (&basis_idx, coef_tryte) in &coefficients.dimensions {
            if let Some(basis_vec) = self.basis.get(basis_idx) {
                let coef = coef_tryte.to_i64() as f64 / 100.0;
                
                // Scale basis vector by coefficient and bundle
                for (&dim, tryte) in &basis_vec.dimensions {
                    let scaled = (tryte.to_i64() as f64 * coef) as i64;
                    let current = result.get(dim);
                    result.set(dim, current + scaled);
                }
            }
        }
        
        result
    }

    /// Full reconstruction from differential encoding
    pub fn decode(&self, encoding: &DifferentialEncoding) -> HyperVec {
        let mut result = self.reconstruct_from_coefficients(&encoding.coefficients);
        
        // Add residual corrections
        for (&dim, tryte) in &encoding.residual.dimensions {
            let current = result.get(dim);
            result.set(dim, current + tryte.to_i64());
        }
        
        result
    }

    /// Calculate reconstruction quality
    fn calculate_quality(&self, original: &HyperVec, coefficients: &HyperVec, residual: &HyperVec) -> f64 {
        let reconstructed = self.reconstruct_from_coefficients(coefficients);
        
        let mut total_error: f64 = 0.0;
        let mut total_energy: f64 = 0.0;
        
        for dim in 0..self.config.num_dimensions {
            let orig = original.get(dim) as f64;
            let recon = reconstructed.get(dim) as f64;
            let res = residual.get(dim) as f64;
            
            total_energy += orig * orig;
            total_error += (orig - recon - res).powi(2);
        }
        
        if total_energy == 0.0 {
            return 1.0;
        }
        
        1.0 - (total_error / total_energy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_multiplication() {
        assert_eq!(Trit::Pos * Trit::Pos, Trit::Pos);
        assert_eq!(Trit::Pos * Trit::Neg, Trit::Neg);
        assert_eq!(Trit::Neg * Trit::Neg, Trit::Pos);
        assert_eq!(Trit::Zero * Trit::Pos, Trit::Zero);
    }

    #[test]
    fn test_tryte_roundtrip() {
        let test_values = [0i64, 1, -1, 42, -42, 100, -100, 364, -364];
        
        for &value in &test_values {
            let tryte = Tryte::from_i64(value, 6);
            let decoded = tryte.to_i64();
            assert_eq!(value, decoded, "Roundtrip failed for {}", value);
        }
    }

    #[test]
    fn test_tryte_max_values() {
        // 6 trits = 729 states = range [-364, 364]
        assert_eq!(Tryte::max_value(6), 364);
        assert_eq!(Tryte::min_value(6), -364);
        
        // 8 trits = 6561 states = range [-3280, 3280]
        assert_eq!(Tryte::max_value(8), 3280);
    }

    #[test]
    fn test_hypervec_bundle_commutative() {
        let config = DimensionalConfig::default();
        let a = HyperVec::from_dense(config.clone(), &[1, 0, -1, 1, 0]);
        let b = HyperVec::from_dense(config.clone(), &[0, 1, -1, 0, 1]);
        
        let ab = a.bundle(&b);
        let ba = b.bundle(&a);
        
        assert_eq!(ab.dimensions.len(), ba.dimensions.len());
        for (dim, tryte) in &ab.dimensions {
            assert_eq!(tryte.to_i64(), ba.get(*dim));
        }
    }

    #[test]
    fn test_hypervec_bind_self_inverse() {
        let config = DimensionalConfig::default();
        let mut a = HyperVec::new(config.clone());
        a.set(0, 1);
        a.set(1, -1);
        a.set(2, 1);
        
        // A ⊙ A should produce all 1s (positive) for non-zero dimensions
        let aa = a.bind(&a);
        
        for (_, tryte) in &aa.dimensions {
            assert_eq!(tryte.to_i64(), 1, "Self-bind should produce +1");
        }
    }

    #[test]
    fn test_hypervec_permute_inverse() {
        let config = DimensionalConfig::compact();
        let mut a = HyperVec::new(config.clone());
        a.set(0, 1);
        a.set(100, -1);
        a.set(500, 1);
        
        let permuted = a.permute(1000);
        let recovered = permuted.inverse_permute(1000);
        
        assert_eq!(a.get(0), recovered.get(0));
        assert_eq!(a.get(100), recovered.get(100));
        assert_eq!(a.get(500), recovered.get(500));
    }

    #[test]
    fn test_hypervec_cosine_identical() {
        let config = DimensionalConfig::default();
        let mut a = HyperVec::new(config.clone());
        a.set(0, 1);
        a.set(1, -1);
        a.set(2, 1);
        
        let similarity = a.cosine(&a);
        assert!((similarity - 1.0).abs() < 0.001, "Self-similarity should be 1.0");
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let config = DimensionalConfig::compact();
        let mut vec = HyperVec::new(config.clone());
        vec.set(0, 42);
        vec.set(100, -17);
        vec.set(1000, 100);
        
        let packed = vec.pack();
        let unpacked = HyperVec::unpack(config, &packed).expect("Unpack failed");
        
        assert_eq!(vec.get(0), unpacked.get(0));
        assert_eq!(vec.get(100), unpacked.get(100));
        assert_eq!(vec.get(1000), unpacked.get(1000));
    }

    #[test]
    fn test_differential_encoding() {
        let config = DimensionalConfig::compact();
        let mut encoder = DifferentialEncoder::new(config.clone());
        
        // Add some basis vectors
        let mut basis1 = HyperVec::new(config.clone());
        basis1.set(0, 1);
        basis1.set(1, 1);
        encoder.add_basis(basis1);
        
        // Create data similar to basis
        let mut data = HyperVec::new(config.clone());
        data.set(0, 1);
        data.set(1, 1);
        data.set(2, 1); // Extra dimension
        
        let encoding = encoder.encode(&data);
        let decoded = encoder.decode(&encoding);
        
        // Should reconstruct original
        assert_eq!(data.get(0), decoded.get(0));
        assert_eq!(data.get(1), decoded.get(1));
        assert_eq!(data.get(2), decoded.get(2));
    }
}
