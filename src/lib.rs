//! Embeddenator - Holographic Computing Substrate
//!
//! Copyright (c) 2025 Embeddenator Contributors
//! Licensed under MIT License
//!
//! Production Rust implementation of sparse ternary VSA (Vector Symbolic
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

pub mod cli;
pub mod correction;
pub mod embrfs;
pub mod fuse_shim;
pub mod kernel_interop;

// Re-export embeddenator-vsa as a public module for backward compatibility
pub use embeddenator_vsa as vsa;
pub use embeddenator_vsa::ternary;
pub use embeddenator_vsa::ternary_vec;
// Re-export embeddenator-retrieval types  
pub use embeddenator_retrieval as retrieval;
pub use embeddenator_retrieval::core::resonator;
// VSA types from embeddenator-vsa component
pub use embeddenator_vsa::{
    BalancedTernaryWord, Codebook, CorrectionEntry, DifferentialEncoder, DifferentialEncoding,
    DimensionalConfig, HyperVec, PackedTritVec, ParityTrit, ProjectionResult, ReversibleVSAConfig,
    SemanticOutlier, SparseVec, Trit, TritDepthConfig, Tryte, Tryte3, Word6, WordMetadata, DIM,
    Trit as DimTrit,
};
// Retrieval types from embeddenator-retrieval component
pub use embeddenator_retrieval::{RerankedResult, SearchResult, TernaryInvertedIndex};
pub use embeddenator_retrieval::resonator::Resonator;
pub use correction::{CorrectionStore, CorrectionStats, ChunkCorrection, CorrectionType, ReconstructionVerifier};
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
// Re-exported from embeddenator-vsa component
// pub use ternary::{Trit, Tryte3, Word6, ParityTrit, CorrectionEntry};
// pub use ternary_vec::PackedTritVec;
// pub use vsa::{SparseVec, ReversibleVSAConfig, DIM};
