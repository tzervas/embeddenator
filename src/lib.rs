//! Embeddenator - Holographic Computing Substrate
//!
//! Copyright (c) 2025 Embeddenator Contributors
//! Licensed under MIT License
//!
//! Pre-1.0 Rust implementation of sparse ternary VSA (Vector Symbolic
//! Architecture) holographic filesystem and computing substrate.
//!
//! # Overview
//!
//! Embeddenator encodes entire filesystems into holographic "engrams" using
//! sparse ternary vectors, enabling:
//! - 100% bit-perfect reconstruction of all files
//! - Holographic superposition of multiple data sources
//! - Algebraic operations (bundle, bind) on encoded data
//! - Hierarchical chunked encoding for TB-scale datasets
//!
//! # Quick Start
//!
//! ```no_run
//! use embeddenator::{EmbrFS, SparseVec};
//! use std::path::Path;
//!
//! // Create a new holographic filesystem
//! let mut fs = EmbrFS::new();
//!
//! // Ingest a directory (would require actual directory)
//! // fs.ingest_directory("./input", false)?;
//!
//! // Save the engram and manifest
//! // fs.save_engram("root.engram")?;
//! // fs.save_manifest("manifest.json")?;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! # Core Concepts
//!
//! ## Vector Symbolic Architecture (VSA)
//!
//! The foundation of Embeddenator is VSA with three key operations:
//!
//! - **Bundle (⊕)**: Associative superposition - combine multiple vectors
//! - **Bind (⊙)**: Non-commutative composition - encode associations
//! - **Cosine Similarity**: Retrieve similar patterns (>0.75 strong match)
//!
//! ## Engrams
//!
//! An engram is a holographic encoding containing:
//! - Root vector: superposition of all data chunks
//! - Codebook: mapping of chunk IDs to original data
//! - Manifest: file structure and metadata
//!
//! # Modules
//!
//! - [`vsa`]: Vector Symbolic Architecture implementation
//! - [`embrfs`]: Holographic filesystem layer
//! - [`cli`]: Command-line interface

#[path = "cli/mod.rs"]
pub mod cli;

#[path = "core/codebook.rs"]
pub mod codebook;

#[path = "core/correction.rs"]
pub mod correction;

#[path = "vsa/dimensional.rs"]
pub mod dimensional;

#[path = "io/envelope.rs"]
pub mod envelope;

#[path = "fs/embrfs.rs"]
pub mod embrfs;

#[path = "fs/fuse_shim.rs"]
pub mod fuse_shim;

#[path = "interop/kernel_interop.rs"]
pub mod kernel_interop;

#[path = "obs/logging.rs"]
pub mod logging;

#[path = "obs/metrics.rs"]
pub mod metrics;

#[path = "obs/hires_timing.rs"]
pub mod hires_timing;

#[path = "core/resonator.rs"]
pub mod resonator;

#[path = "retrieval/retrieval.rs"]
pub mod retrieval;

#[path = "retrieval/signature.rs"]
pub mod signature;

#[path = "vsa/simd_cosine.rs"]
pub mod simd_cosine;

#[path = "vsa/ternary.rs"]
pub mod ternary;

#[path = "vsa/ternary_vec.rs"]
pub mod ternary_vec;

#[path = "vsa/bitsliced.rs"]
pub mod bitsliced;

#[path = "vsa/block_sparse.rs"]
pub mod block_sparse;

#[path = "vsa/hybrid.rs"]
pub mod hybrid;

#[path = "vsa/soft_ternary.rs"]
pub mod soft_ternary;

#[path = "vsa/vsa.rs"]
pub mod vsa;

/// Testing utilities: metrics, integrity validation, chaos injection.
#[cfg(test)]
pub mod testing;

// Re-export main types for convenience
pub use bitsliced::{
    has_avx2, has_avx512, simd_features_string, BitslicedTritVec, CarrySaveBundle,
};
pub use block_sparse::{Block, BlockError, BlockSparseTritVec};
pub use codebook::{
    BalancedTernaryWord, Codebook, ProjectionResult, SemanticOutlier, WordMetadata,
};
pub use correction::{
    ChunkCorrection, CorrectionStats, CorrectionStore, CorrectionType, ReconstructionVerifier,
};
pub use dimensional::{
    DifferentialEncoder, DifferentialEncoding, DimensionalConfig, HyperVec, Trit as DimTrit,
    TritDepthConfig, Tryte,
};
pub use embrfs::{
    load_hierarchical_manifest, query_hierarchical_codebook,
    query_hierarchical_codebook_with_store, save_hierarchical_manifest, save_sub_engrams_dir,
    DirectorySubEngramStore, HierarchicalChunkHit, HierarchicalManifest, HierarchicalQueryBounds,
    SubEngram, SubEngramStore, UnifiedManifest,
};
pub use embrfs::{EmbrFS, Engram, FileEntry, Manifest, DEFAULT_CHUNK_SIZE};
pub use envelope::{BinaryWriteOptions, CompressionCodec, PayloadKind};
pub use fuse_shim::{EngramFS, EngramFSBuilder, FileAttr, FileKind};
pub use hybrid::{HybridTritVec, DENSITY_THRESHOLD, MIN_BITSLICED_DIM};
pub use kernel_interop::{
    rerank_top_k_by_cosine, CandidateGenerator, KernelInteropError, SparseVecBackend, VectorStore,
    VsaBackend,
};
pub use resonator::Resonator;
pub use retrieval::{RerankedResult, SearchResult, TernaryInvertedIndex};
pub use soft_ternary::SoftTernaryVec;
pub use ternary::{CorrectionEntry, ParityTrit, Trit, Tryte3, Word6};
pub use ternary_vec::PackedTritVec;
pub use vsa::{ReversibleVSAConfig, SparseVec, DIM};
