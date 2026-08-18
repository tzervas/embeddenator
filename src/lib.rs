//! Embeddenator VSA - Vector Symbolic Architecture Core
//!
//! This crate provides the foundational Vector Symbolic Architecture (VSA)
//! operations for sparse ternary representations used in Embeddenator.
//!
//! # Overview
//!
//! VSA is a computational framework for representing and manipulating
//! high-dimensional vectors through algebraic operations:
//!
//! - **Bundle (⊕)**: Superposition of multiple vectors
//! - **Bind (⊙)**: Compositional binding of associations
//! - **Similarity**: Cosine similarity for pattern retrieval
//!
//! # Core Types
//!
//! - [`SparseVec`]: Sparse ternary vector representation
//! - [`PackedTritVec`]: Memory-efficient packed trit storage
//! - [`Codebook`]: Mapping between symbols and vectors
//!
//! # Example
//!
//! ```rust
//! use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
//!
//! // Create random sparse vectors
//! let a = SparseVec::random();
//! let b = SparseVec::random();
//!
//! // Bundle operation (superposition)
//! let bundled = a.bundle(&b);
//!
//! // Compute similarity
//! let similarity = a.cosine(&b);
//! assert!(similarity >= -1.0 && similarity <= 1.0);
//! ```

use std::fmt;

/// Unified error type for VSA operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VsaError {
    /// Value is outside the valid range for the operation
    ValueOutOfRange { value: i64, min: i64, max: i64 },
    /// Hash operation produced unexpected length
    InvalidHashLength { expected: usize, actual: usize },
    /// Operation expected a non-empty collection
    EmptyCollection,
    /// Packed data could not be unpacked
    InvalidPackedData,
    /// Value cannot be converted to the target type
    InvalidValue(String),
}

impl fmt::Display for VsaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VsaError::ValueOutOfRange { value, min, max } => {
                write!(
                    f,
                    "Value {} is outside valid range [{}, {}]",
                    value, min, max
                )
            }
            VsaError::InvalidHashLength { expected, actual } => {
                write!(
                    f,
                    "Invalid hash length: expected {}, got {}",
                    expected, actual
                )
            }
            VsaError::EmptyCollection => {
                write!(f, "Operation requires non-empty collection")
            }
            VsaError::InvalidPackedData => {
                write!(f, "Packed data could not be unpacked")
            }
            VsaError::InvalidValue(msg) => {
                write!(f, "Invalid value: {}", msg)
            }
        }
    }
}

impl std::error::Error for VsaError {}

#[cfg(feature = "block-sparse")]
pub mod block_sparse;
pub mod codebook;
pub mod coherency;
pub mod dimensional;
#[cfg(feature = "cuda")]
pub mod gpu;
#[cfg(feature = "cuda")]
pub mod gpu_encoding;
#[cfg(feature = "cuda")]
pub mod gpu_kernels;
pub mod phase_training;
pub mod resonator;
pub mod reversible_encoding;
pub mod simd_cosine;
pub mod ternary;
pub mod ternary_vec;
pub mod virtual_memory;
pub mod vram_pool;
pub mod vsa;

// Re-export main types
pub use codebook::{
    BalancedTernaryWord, Codebook, CodebookTrainingConfig, ProjectionConfig, ProjectionResult,
    SemanticOutlier, WordMetadata,
};
pub use dimensional::{
    DifferentialEncoder, DifferentialEncoding, DimensionalConfig, HyperVec, Trit as DimTrit,
    TritDepthConfig, Tryte,
};
#[cfg(feature = "cuda")]
pub use gpu::{
    GpuArch, GpuBackend, GpuConfig, GpuError, GpuMemoryConfig, DEFAULT_MEMORY_HEADROOM_PERCENT,
    MAX_MEMORY_USAGE_RATIO, MIN_SYSTEM_RESERVE_BYTES,
};
#[cfg(feature = "cuda")]
pub use gpu_encoding::GpuAcceleratedEncoder;
#[cfg(feature = "cuda")]
pub use gpu_kernels::GpuKernelRunner;
pub use phase_training::{
    train_codebook_with_phases, PhaseTrainer, PhaseTrainingConfig, PhaseTrainingStats,
    TrainingPhase,
};
pub use reversible_encoding::{ReversibleVSAEncoder, MAX_POSITIONS};
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
pub use simd_cosine::cosine_avx2;
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
pub use simd_cosine::cosine_avx_vnni;
#[cfg(target_arch = "aarch64")]
pub use simd_cosine::cosine_neon;
pub use simd_cosine::cosine_scalar;
// SIMD dispatch function that automatically selects best available implementation
pub use simd_cosine::cosine_simd;
pub use ternary::{CorrectionEntry, ParityTrit, Trit, Tryte3, Word6};
pub use ternary_vec::PackedTritVec;
pub use vsa::{ReversibleVSAConfig, SparseVec, SparsityScaling, VsaConfig, VsaConfigSchema, DIM};

#[cfg(feature = "block-sparse")]
pub use block_sparse::{Block, BlockSparseError, BlockSparseTritVec, BLOCK_SIZE};

// Coherency types for host/device memory management
pub use coherency::{CoherencyManager, CoherencyState, CoherencyStats, CoherentEngram};

// Multi-tier coherency protocol (#48)
pub use coherency::{
    SyncProtocol, Tier, TierMask, TieredBlock, TieredCoherencyStats, TieredState, WritePolicy,
};

// VRAM pool types for GPU memory management
// Note: Non-cuda builds have stub implementations that return errors
pub use vram_pool::{VramHandle, VramPool, VramPoolConfig, VramPoolStats};

// Virtual memory abstraction for tiered storage (VRAM → Host RAM → Disk)
pub use virtual_memory::{
    MemoryTier, VMemHandle, VirtualMemory, VirtualMemoryConfig, VirtualMemoryError,
    VirtualMemoryStats,
};

// Resonator network types for learned codebooks and semantic inference
pub use resonator::{
    FactorizationResult, RecoveredFactor, Resonator, ResonatorConfig, ResonatorStats,
    SemanticInference, TrainingExample, TrainingResult,
};
