//! # embeddenator-fs
//!
//! EmbrFS: FUSE-based holographic filesystem.
//!
//! Extracted from embeddenator core as part of Phase 2A component decomposition.
//!
//! # Feature Flags
//!
//! - **`fuse`**: Enable FUSE filesystem support (requires `fuser` crate)
//! - **`disk-image`**: Enable QCOW2/raw disk image encoding (Linux with io_uring)
//! - **`disk-image-portable`**: Disk image support without io_uring (cross-platform)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                          embeddenator-fs                                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌───────────────────────┐  ┌───────────────────────┐                  │
//! │  │       EmbrFS          │  │      disk module      │                  │
//! │  │  (Directory/File      │  │  (QCOW2/Raw image     │                  │
//! │  │   encoding)           │  │   encoding)           │                  │
//! │  └───────────┬───────────┘  └───────────┬───────────┘                  │
//! │              │                          │                               │
//! │              ▼                          ▼                               │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                     embeddenator-vsa                            │   │
//! │  │  (Sparse ternary vectors, VSA encoding/decoding)                │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │              │                                                          │
//! │              ▼                                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                     embeddenator-io                             │   │
//! │  │  (Compression profiles: zstd, lz4)                              │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

pub mod fs;
pub use fs::*;

/// Disk image support (QCOW2, raw images, partition tables, filesystem traversal)
///
/// # Why a Separate Module?
///
/// Disk image handling is optional and adds significant dependencies:
/// - `qcow2-rs` for QCOW2 format
/// - `tokio` + `tokio-uring` for async I/O
/// - `gpt`, `mbrman` for partition tables
/// - `ext4` for filesystem traversal
///
/// Users who only need directory encoding can avoid these dependencies.
#[cfg(any(feature = "disk-image", feature = "disk-image-portable"))]
pub mod disk;

// Re-export common types from vsa and retrieval for convenience
pub use embeddenator_retrieval::resonator::Resonator;
pub use embeddenator_vsa::{ReversibleVSAConfig, ReversibleVSAEncoder, SparseVec};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_loads() {
        // Verify core types are accessible
        let fs = EmbrFS::new_holographic();
        assert!(fs.engram.codebook.is_empty());
    }
}
