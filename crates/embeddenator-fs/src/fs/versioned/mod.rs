//! # Versioned Data Structures for Mutable Engrams
//!
//! This module implements versioned, concurrency-safe data structures that enable
//! mutable engrams with optimistic locking. The key design principle is to layer
//! versioning and concurrency control over the mathematically pure VSA operations.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              VersionedEngram (Top Level)                    │
//! │  - Global version tracking                                  │
//! │  - Transaction coordination                                 │
//! │  - CAS-based root updates                                   │
//! └─────────────────────────────────────────────────────────────┘
//!                          ↓
//!          ┌───────────────┴───────────────┬─────────────────┐
//!          ↓                               ↓                 ↓
//! ┌───────────────────┐       ┌──────────────────┐  ┌──────────────────┐
//! │VersionedChunk     │       │VersionedManifest │  │VersionedCorr...  │
//! │       Store       │       │ - File-level     │  │ - Chunk-level    │
//! │ - Chunk-level     │       │   versioning     │  │   versioning     │
//! │   versioning      │       │ - RwLock         │  │ - RwLock         │
//! │ - RwLock          │       │ - Per-file ver.  │  │ - Arc<Corr>      │
//! │ - Arc<SparseVec>  │       │                  │  │                  │
//! │ (NOT VSA codebook)│       │                  │  │                  │
//! └───────────────────┘       └──────────────────┘  └──────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! ### Optimistic Locking
//!
//! Instead of preventing concurrent access, we allow it and detect conflicts:
//!
//! 1. **Read Phase**: Reader captures data + version number
//! 2. **Computation**: Reader processes data (no locks held)
//! 3. **Write Phase**: Writer checks if version unchanged, updates if valid
//! 4. **Retry**: If version mismatch, retry from step 1
//!
//! ### VSA Codebook vs Chunk Store
//!
//! **Important distinction:**
//! - **VSA Codebook** (in embeddenator-vsa): Static base vectors used for encoding/decoding.
//!   This is the "dictionary" or "basis" that the VSA uses. Typically not versioned.
//! - **Chunk Store** (this module): Maps file chunk IDs to their VSA-encoded representations.
//!   This is what gets versioned for mutable engrams.
//!
//! The engram's transparent compression comes from the VSA encoding itself, not from
//! explicit compression as a separate layer. Future layers (signatures, encryption) build on top.
//!
//! ### Multi-Level Versioning
//!
//! - **Component-Level**: Chunk Store, Manifest, Corrections each have global version
//! - **Entry-Level**: Individual chunks/files have local versions
//! - **Engram-Level**: Overall engram version coordinates components
//!
//! ### Structural Sharing with Arc
//!
//! Immutable data wrapped in `Arc` allows zero-copy sharing:
//!
//! ```rust,no_run
//! # use std::sync::Arc;
//! # struct SparseVec;
//! // Old version
//! let old_chunk = Arc::new(SparseVec { /* ... */ });
//!
//! // New version - only new data is allocated
//! let new_chunk = Arc::new(SparseVec { /* ... */ });
//!
//! // Old readers still have valid Arc::clone(old_chunk)
//! // New readers get Arc::clone(new_chunk)
//! // No copying of underlying data!
//! ```
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! # use embeddenator_fs::versioned::*;
//! # use embeddenator_fs::VersionedEmbrFS;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create versioned filesystem
//! let fs = VersionedEmbrFS::new();
//!
//! // Write a file
//! let version = fs.write_file("path/to/file", b"hello world", None)?;
//!
//! // Read with snapshot isolation (non-blocking)
//! let (data, version) = fs.read_file("path/to/file")?;
//!
//! // Process data...
//! let modified_data = process(&data);
//!
//! // Write with optimistic locking
//! match fs.write_file("path/to/file", &modified_data, Some(version)) {
//!     Ok(new_version) => println!("Write succeeded: v{}", new_version),
//!     Err(e) => {
//!         println!("Write failed: {:?}", e);
//!         // Retry logic here
//!     },
//! }
//! # Ok(())
//! # }
//! # fn process(data: &[u8]) -> Vec<u8> { data.to_vec() }
//! ```

pub mod chunk;
pub mod chunk_store;
pub mod corrections;
pub mod engram;
pub mod journal;
pub mod manifest;
pub mod transaction;
pub mod types;

pub use chunk::VersionedChunk;
pub use chunk_store::VersionedChunkStore;
pub use corrections::VersionedCorrectionStore;
pub use engram::VersionedEngram;
pub use journal::{DurabilityMode, Journal, JournalOp, RecordFlags};
pub use manifest::{VersionedFileEntry, VersionedManifest};
pub use transaction::{Operation, Transaction, TransactionStatus};
pub use types::{ChunkId, VersionMismatch};
