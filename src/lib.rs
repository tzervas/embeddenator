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
pub mod codebook;
pub mod correction;
pub mod dimensional;
pub mod embrfs;
pub mod fuse_shim;
pub mod resonator;
pub mod ternary;
pub mod vsa;

// Re-export main types for convenience
pub use codebook::{Codebook, BalancedTernaryWord, ProjectionResult, SemanticOutlier, WordMetadata};
pub use correction::{CorrectionStore, CorrectionStats, ChunkCorrection, CorrectionType, ReconstructionVerifier};
pub use dimensional::{
    Trit as DimTrit, Tryte, DimensionalConfig, TritDepthConfig,
    HyperVec, DifferentialEncoder, DifferentialEncoding,
};
pub use embrfs::{EmbrFS, Engram, FileEntry, Manifest, DEFAULT_CHUNK_SIZE};
pub use fuse_shim::{EngramFS, EngramFSBuilder, FileAttr, FileKind};
pub use resonator::Resonator;
pub use ternary::{Trit, Tryte3, Word6, ParityTrit, CorrectionEntry};
pub use vsa::{SparseVec, ReversibleVSAConfig, DIM};
