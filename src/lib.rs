//! Embeddenator - Holographic Computing Substrate
//!
//! Copyright (c) 2025 Embeddenator Contributors
//! Licensed under MIT License
//!
//! # ⚠️ DEPRECATED LIBRARY - DO NOT USE IN NEW PROJECTS
//!
//! **This monolithic crate is DEPRECATED as of v0.20.0-alpha.1.**
//!
//! All functionality has been migrated to component repositories. Use the
//! component crates instead:
//!
//! - **`embeddenator-vsa`** - Vector Symbolic Architecture primitives
//! - **`embeddenator-fs`** - Filesystem operations (EmbrFS)
//! - **`embeddenator-retrieval`** - Information retrieval
//! - **`embeddenator-io`** - I/O operations
//! - **`embeddenator-obs`** - Observability
//! - **`embeddenator-interop`** - FFI/interoperability
//! - **`embeddenator-cli`** - Command-line interface
//!
//! **Migration Guide:** See [`DEPRECATED.md`](../DEPRECATED.md) for detailed
//! migration instructions.
//!
//! **Timeline:**
//! - v0.20.0-alpha.1 (Jan 2026): DEPRECATED
//! - v0.21.0 (Q2 2026): ARCHIVED (read-only)
//! - v1.0.0 (Q3 2026): REMOVED
//!
//! # Overview (Legacy Documentation)
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
//! - [`vsa`]: Vector Symbolic Architecture implementation (DEPRECATED - use embeddenator-vsa)
//! - [`embrfs`]: Holographic filesystem layer (DEPRECATED - use embeddenator-fs)
//! - [`cli`]: Command-line interface (DEPRECATED - use embeddenator-cli)

#![deprecated(
    since = "0.20.0-alpha.1",
    note = "This monolithic crate is deprecated. Use component crates instead: embeddenator-vsa, embeddenator-fs, embeddenator-retrieval, embeddenator-io, embeddenator-obs. See DEPRECATED.md for migration guide."
)]

pub mod cli;
pub mod perf;

// Re-export embeddenator-vsa as a public module for backward compatibility
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-vsa crate directly instead of embeddenator::vsa"
)]
pub use embeddenator_vsa as vsa;
#[deprecated(since = "0.20.0-alpha.1", note = "Use embeddenator-vsa crate directly")]
pub use embeddenator_vsa::ternary;
#[deprecated(since = "0.20.0-alpha.1", note = "Use embeddenator-vsa crate directly")]
pub use embeddenator_vsa::ternary_vec;
// Re-export embeddenator-retrieval types
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-retrieval crate directly instead of embeddenator::retrieval"
)]
pub use embeddenator_retrieval as retrieval;
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-retrieval crate directly"
)]
pub use embeddenator_retrieval::core::resonator;
// Re-export embeddenator-fs types
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-fs crate directly instead of embeddenator::fs"
)]
pub use embeddenator_fs as fs;
#[deprecated(since = "0.20.0-alpha.1", note = "Use embeddenator-fs crate directly")]
pub use embeddenator_fs::correction;
#[deprecated(since = "0.20.0-alpha.1", note = "Use embeddenator-fs crate directly")]
pub use embeddenator_fs::embrfs;
#[deprecated(since = "0.20.0-alpha.1", note = "Use embeddenator-fs crate directly")]
pub use embeddenator_fs::fuse_shim;
// Re-export embeddenator-interop types
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-interop crate directly instead of embeddenator::interop"
)]
pub use embeddenator_interop as interop;
// Re-export embeddenator-io types
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-io crate directly instead of embeddenator::io"
)]
pub use embeddenator_io as io;
// Re-export embeddenator-obs types
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Use embeddenator-obs crate directly instead of embeddenator::obs"
)]
pub use embeddenator_obs as obs;
// VSA types from embeddenator-vsa component
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Import these types from embeddenator-vsa crate directly"
)]
pub use embeddenator_vsa::{
    BalancedTernaryWord, Codebook, CorrectionEntry, DifferentialEncoder, DifferentialEncoding,
    DimensionalConfig, HyperVec, PackedTritVec, ParityTrit, ProjectionResult, ReversibleVSAConfig,
    SemanticOutlier, SparseVec, Trit, Trit as DimTrit, TritDepthConfig, Tryte, Tryte3, Word6,
    WordMetadata, DIM,
};
// Retrieval types from embeddenator-retrieval component
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Import Resonator from embeddenator-retrieval crate directly"
)]
pub use embeddenator_retrieval::resonator::Resonator;
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Import these types from embeddenator-retrieval crate directly"
)]
pub use embeddenator_retrieval::{RerankedResult, SearchResult, TernaryInvertedIndex};
// Filesystem types from embeddenator-fs component
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Import these types from embeddenator-fs crate directly"
)]
pub use embeddenator_fs::{
    load_hierarchical_manifest, query_hierarchical_codebook,
    query_hierarchical_codebook_with_store, save_hierarchical_manifest, save_sub_engrams_dir,
    ChunkCorrection, CorrectionStats, CorrectionStore, CorrectionType, DirectorySubEngramStore,
    EmbrFS, Engram, EngramFS, EngramFSBuilder, FileAttr, FileEntry, FileKind, HierarchicalChunkHit,
    HierarchicalManifest, HierarchicalQueryBounds, Manifest, ReconstructionVerifier, SubEngram,
    SubEngramStore, UnifiedManifest, DEFAULT_CHUNK_SIZE,
};
// Interop types from embeddenator-interop component
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Import these types from embeddenator-interop crate directly"
)]
pub use embeddenator_interop::{
    rerank_top_k_by_cosine, CandidateGenerator, KernelInteropError, SparseVecBackend, VectorStore,
    VsaBackend,
};
// I/O types from embeddenator-io component
#[deprecated(
    since = "0.20.0-alpha.1",
    note = "Import these types from embeddenator-io crate directly"
)]
pub use embeddenator_io::{
    unwrap_auto, wrap_or_legacy, BinaryWriteOptions, CompressionCodec, PayloadKind,
};
