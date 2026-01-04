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
//! # Architecture (v0.20.0+)
//!
//! As of v0.20.0, Embeddenator has been decomposed into component libraries:
//! - **embeddenator-vsa**: Core VSA operations, sparse ternary vectors
//! - **embeddenator-retrieval**: Signature-based search, resonator, correction
//! - **embeddenator-fs**: EmbrFS FUSE filesystem
//! - **embeddenator-interop**: Kernel interop, system integration
//! - **embeddenator-io**: Envelope format, serialization
//! - **embeddenator-obs**: Metrics, logging, tracing
//!
//! This crate serves as an orchestrator, re-exporting component functionality.
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
//! - Component libraries (see dependencies in Cargo.toml)
//! - [`cli`]: Command-line interface
//! - [`codebook`]: Local codebook implementation

// Import component libraries
pub use embeddenator_vsa as vsa;
pub use embeddenator_retrieval as retrieval;
pub use embeddenator_fs as fs;
pub use embeddenator_interop as interop;
pub use embeddenator_io as io;
pub use embeddenator_obs as obs;

// Local modules
#[path = "cli/mod.rs"]
pub mod cli;

#[path = "core/codebook.rs"]
pub mod codebook;


/// Testing utilities: metrics, integrity validation, chaos injection.
#[cfg(test)]
pub mod testing;

// Re-export main types for convenience from component libraries
pub use codebook::{Codebook, BalancedTernaryWord, ProjectionResult, SemanticOutlier, WordMetadata};

// From embeddenator-retrieval
pub use retrieval::correction::{CorrectionStore, CorrectionStats, ChunkCorrection, CorrectionType, ReconstructionVerifier};
pub use retrieval::core::resonator::Resonator;
pub use retrieval::{RerankedResult, SearchResult, TernaryInvertedIndex};

// From embeddenator-vsa
pub use vsa::dimensional::{
    Trit as DimTrit, Tryte, DimensionalConfig, TritDepthConfig,
    HyperVec, DifferentialEncoder, DifferentialEncoding,
};
pub use vsa::ternary::{Trit, Tryte3, Word6, ParityTrit, CorrectionEntry};
pub use vsa::ternary_vec::PackedTritVec;
pub use vsa::bitsliced::{BitslicedTritVec, CarrySaveBundle, has_avx512, has_avx2, simd_features_string};
pub use vsa::block_sparse::{Block, BlockSparseTritVec, BlockError};
pub use vsa::hybrid::{HybridTritVec, DENSITY_THRESHOLD, MIN_BITSLICED_DIM};
pub use vsa::soft_ternary::SoftTernaryVec;
pub use vsa::vsa::{SparseVec, ReversibleVSAConfig, DIM};

// From embeddenator-io
pub use io::envelope::{BinaryWriteOptions, CompressionCodec, PayloadKind};

// From embeddenator-fs
pub use fs::fs::embrfs::{EmbrFS, Engram, FileEntry, Manifest, DEFAULT_CHUNK_SIZE};
pub use fs::fs::embrfs::{
    DirectorySubEngramStore, HierarchicalChunkHit, HierarchicalManifest, HierarchicalQueryBounds,
    SubEngram, SubEngramStore, UnifiedManifest, load_hierarchical_manifest,
    query_hierarchical_codebook, query_hierarchical_codebook_with_store, save_hierarchical_manifest,
    save_sub_engrams_dir,
};
pub use fs::fs::fuse_shim::{EngramFS, EngramFSBuilder, FileAttr, FileKind};

// From embeddenator-interop
pub use interop::kernel_interop::{
    CandidateGenerator, KernelInteropError, SparseVecBackend, VectorStore, VsaBackend,
    rerank_top_k_by_cosine,
};
