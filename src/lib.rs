//! Embeddenator - Holographic Computing Substrate
//! 
//! Copyright (c) 2025 Embeddenator Contributors
//! Licensed under MIT License
//! 
//! Production Rust implementation of sparse ternary VSA (Vector Symbolic
//! Architecture) holographic filesystem and computing substrate.

pub mod vsa;
pub mod embrfs;
pub mod cli;

// Re-export main types for convenience
pub use vsa::{SparseVec, DIM};
pub use embrfs::{EmbrFS, Engram, Manifest, FileEntry, DEFAULT_CHUNK_SIZE};
