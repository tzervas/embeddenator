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

#[path = "vsa/soft_ternary.rs"]
pub mod soft_ternary;

#[path = "vsa/vsa.rs"]
pub mod vsa;

// Re-export main types for convenience
pub use codebook::{Codebook, BalancedTernaryWord, ProjectionResult, SemanticOutlier, WordMetadata};
pub use correction::{CorrectionStore, CorrectionStats, ChunkCorrection, CorrectionType, ReconstructionVerifier};
pub use dimensional::{
    Trit as DimTrit, Tryte, DimensionalConfig, TritDepthConfig,
    HyperVec, DifferentialEncoder, DifferentialEncoding,
};
pub use envelope::{BinaryWriteOptions, CompressionCodec, PayloadKind};
pub use embrfs::{EmbrFS, Engram, FileEntry, Manifest, DEFAULT_CHUNK_SIZE};
pub use embrfs::{
    DirectorySubEngramStore, HierarchicalChunkHit, HierarchicalManifest, HierarchicalQueryBounds,
    SubEngram, SubEngramStore, UnifiedManifest, load_hierarchical_manifest,
    query_hierarchical_codebook, query_hierarchical_codebook_with_store, save_hierarchical_manifest,
    save_sub_engrams_dir,
};
pub use fuse_shim::{EngramFS, EngramFSBuilder, FileAttr, FileKind};
pub use kernel_interop::{
    CandidateGenerator, KernelInteropError, SparseVecBackend, VectorStore, VsaBackend,
    rerank_top_k_by_cosine,
};
pub use resonator::Resonator;
pub use retrieval::{RerankedResult, SearchResult, TernaryInvertedIndex};
pub use ternary::{Trit, Tryte3, Word6, ParityTrit, CorrectionEntry};
pub use ternary_vec::PackedTritVec;
pub use bitsliced::{BitslicedTritVec, CarrySaveBundle};
pub use soft_ternary::SoftTernaryVec;
pub use vsa::{SparseVec, ReversibleVSAConfig, DIM};
