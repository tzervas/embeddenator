//! Disk image error types
//!
//! # Why Custom Error Types?
//!
//! Disk operations can fail in many ways: I/O errors, format errors, partition
//! errors, filesystem errors. A unified error type with specific variants helps
//! users understand exactly what went wrong and how to fix it.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type for disk operations
pub type DiskResult<T> = Result<T, DiskError>;

/// Errors that can occur during disk image operations
#[derive(Debug)]
pub enum DiskError {
    /// I/O error reading or writing the image file
    ///
    /// # Common Causes
    /// - File not found
    /// - Permission denied
    /// - Disk full (for writes)
    /// - Hardware failure
    Io(io::Error),

    /// Invalid or unsupported image format
    ///
    /// # Common Causes
    /// - Corrupted QCOW2 header
    /// - Unknown magic bytes
    /// - Unsupported QCOW2 version
    InvalidFormat { path: PathBuf, reason: String },

    /// Partition table error
    ///
    /// # Common Causes
    /// - No partition table found
    /// - Corrupted GPT/MBR
    /// - Invalid partition boundaries
    PartitionError { reason: String },

    /// Filesystem error during traversal
    ///
    /// # Common Causes
    /// - Corrupted superblock
    /// - Invalid inode
    /// - Unsupported filesystem type
    FilesystemError { partition: u32, reason: String },

    /// QCOW2-specific error
    ///
    /// # Common Causes
    /// - Backing file not found
    /// - Encryption not supported
    /// - Compressed cluster error
    Qcow2Error { reason: String },

    /// Requested offset is out of bounds
    ///
    /// # Why This Happens
    /// Reading past the end of the image or partition. Usually indicates
    /// a bug in the calling code or corrupted metadata pointing to invalid
    /// locations.
    OutOfBounds { requested: u64, size: u64 },

    /// Feature not supported
    ///
    /// # Why This Exists
    /// Some features (like QCOW2 encryption) are intentionally not implemented
    /// because they're rare and complex. This provides a clear error rather
    /// than silent failure.
    Unsupported { feature: String },
}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiskError::Io(e) => write!(f, "I/O error: {}", e),
            DiskError::InvalidFormat { path, reason } => {
                write!(f, "Invalid image format for {:?}: {}", path, reason)
            }
            DiskError::PartitionError { reason } => {
                write!(f, "Partition table error: {}", reason)
            }
            DiskError::FilesystemError { partition, reason } => {
                write!(f, "Filesystem error on partition {}: {}", partition, reason)
            }
            DiskError::Qcow2Error { reason } => {
                write!(f, "QCOW2 error: {}", reason)
            }
            DiskError::OutOfBounds { requested, size } => {
                write!(
                    f,
                    "Offset {} is out of bounds (image size: {})",
                    requested, size
                )
            }
            DiskError::Unsupported { feature } => {
                write!(f, "Unsupported feature: {}", feature)
            }
        }
    }
}

impl std::error::Error for DiskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DiskError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for DiskError {
    fn from(err: io::Error) -> Self {
        DiskError::Io(err)
    }
}
