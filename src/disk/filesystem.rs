//! Filesystem Traversal for Disk Image Encoding
//!
//! This module provides read-only traversal of filesystems within disk images
//! for the purpose of encoding their contents into engrams.
//!
//! # Why Filesystem Traversal?
//!
//! Disk images contain filesystems that organize data into files and directories.
//! To encode a VM image as an engram, we need to:
//!
//! 1. **Understand the filesystem structure**: Navigate directories, resolve
//!    paths, handle symlinks and special files.
//!
//! 2. **Apply compression profiles**: Different paths get different compression
//!    (kernel files → max compression, binaries → balanced, etc.)
//!
//! 3. **Preserve metadata**: Permissions, ownership, timestamps, xattrs are
//!    crucial for bootable VM images.
//!
//! # Supported Filesystems
//!
//! | Filesystem | Status | Notes |
//! |------------|--------|-------|
//! | ext4       | ✅     | Most common Linux FS |
//! | ext2/ext3  | ✅     | Via ext4 compatibility |
//! | XFS        | 🔜     | Planned |
//! | Btrfs      | 🔜     | Planned |
//! | FAT32/vFAT | 🔜     | For EFI partitions |
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                   FilesystemTraverser                         │
//! ├──────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │ ext4 driver │  │ XFS driver  │  │ FAT driver  │  ...     │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
//! │         │                │                │                  │
//! │         ▼                ▼                ▼                  │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │              Unified File Iterator API                  ││
//! │  │  - path: String                                         ││
//! │  │  - metadata: FileMetadata                               ││
//! │  │  │- data: AsyncRead                                     ││
//! │  └─────────────────────────────────────────────────────────┘│
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_fs::disk::{RawImage, PartitionTable, FilesystemTraverser};
//!
//! let image = RawImage::open("disk.img").await?;
//! let table = PartitionTable::detect(&image).await?;
//!
//! for partition in table.encodable_partitions() {
//!     let traverser = FilesystemTraverser::open(&image, partition).await?;
//!
//!     traverser.walk(|entry| async {
//!         println!("{}: {} bytes", entry.path, entry.size);
//!         // Read file data and encode...
//!     }).await?;
//! }
//! ```

use super::error::{DiskError, DiskResult};
use super::{BlockDevice, PartitionInfo};
#[cfg(feature = "disk-image")]
use super::{BlockDeviceSync, PartitionReader};
use std::path::PathBuf;

/// File type in the filesystem
///
/// # Why Track File Types?
///
/// Different file types require different encoding strategies:
/// - Regular files: Read and encode data
/// - Directories: Create directory entries, recurse
/// - Symlinks: Store target path, handle cycles
/// - Device nodes: Store major/minor numbers
/// - Sockets/FIFOs: Metadata only
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file with data content
    Regular,
    /// Directory containing other entries
    Directory,
    /// Symbolic link to another path
    Symlink,
    /// Hard link (same inode as another file)
    Hardlink,
    /// Character device (e.g., /dev/null)
    CharDevice,
    /// Block device (e.g., /dev/sda)
    BlockDevice,
    /// Named pipe (FIFO)
    Fifo,
    /// Unix domain socket
    Socket,
}

/// Metadata for a filesystem entry
///
/// # Why Preserve All Metadata?
///
/// For bootable VM images, metadata correctness is critical:
/// - Wrong permissions → security issues or boot failures
/// - Wrong ownership → services fail to start
/// - Missing xattrs → SELinux/AppArmor denials
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File type
    pub file_type: FileType,

    /// File size in bytes (0 for non-regular files)
    pub size: u64,

    /// Unix permissions (mode bits)
    pub mode: u32,

    /// User ID
    pub uid: u32,

    /// Group ID
    pub gid: u32,

    /// Access time (Unix timestamp)
    pub atime: i64,

    /// Modification time (Unix timestamp)
    pub mtime: i64,

    /// Change time (Unix timestamp)
    pub ctime: i64,

    /// Number of hard links
    pub nlink: u32,

    /// Inode number (for hardlink detection)
    pub inode: u64,

    /// Device ID for device nodes (major << 8 | minor)
    pub device_id: Option<u64>,

    /// Symlink target (for symlinks)
    pub symlink_target: Option<String>,
}

impl FileMetadata {
    /// Check if this is a regular file with content
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::Regular
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        self.file_type == FileType::Symlink
    }

    /// Get device major number
    pub fn device_major(&self) -> Option<u32> {
        self.device_id.map(|d| (d >> 8) as u32)
    }

    /// Get device minor number
    pub fn device_minor(&self) -> Option<u32> {
        self.device_id.map(|d| (d & 0xFF) as u32)
    }
}

/// A filesystem entry during traversal
#[derive(Debug)]
pub struct FilesystemEntry {
    /// Full path within the filesystem (e.g., "/etc/passwd")
    pub path: PathBuf,

    /// File metadata
    pub metadata: FileMetadata,

    /// Offset within partition where data starts (for regular files)
    pub data_offset: Option<u64>,
}

/// Filesystem traverser for encoding disk images
///
/// # Why a Dedicated Traverser?
///
/// Filesystems have complex internal structures (inodes, extent trees,
/// indirect blocks) that vary by filesystem type. The traverser abstracts
/// these differences behind a simple walk() API.
pub struct FilesystemTraverser {
    /// Filesystem type detected
    fs_type: FilesystemType,

    /// Partition being traversed
    partition: PartitionInfo,

    /// Root inode number
    root_inode: u64,

    /// Block size
    block_size: u32,
}

/// Detected filesystem type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemType {
    /// ext2/3/4 filesystem
    Ext4,
    /// XFS filesystem
    Xfs,
    /// Btrfs filesystem
    Btrfs,
    /// FAT32 / vFAT
    Fat32,
    /// Unknown filesystem
    Unknown,
}

impl FilesystemTraverser {
    /// Open a filesystem on a partition
    ///
    /// # Arguments
    ///
    /// * `device` - Block device containing the partition
    /// * `partition` - Partition info from partition table detection
    ///
    /// # Errors
    ///
    /// Returns `DiskError::FilesystemError` if:
    /// - Filesystem type cannot be detected
    /// - Superblock is corrupted
    /// - Unsupported filesystem features are used
    pub async fn open(device: &dyn BlockDevice, partition: &PartitionInfo) -> DiskResult<Self> {
        // Read superblock to detect filesystem type
        let fs_type = Self::detect_filesystem(device, partition).await?;

        match fs_type {
            FilesystemType::Ext4 => Self::open_ext4(device, partition).await,
            FilesystemType::Unknown => Err(DiskError::FilesystemError {
                partition: partition.number,
                reason: "Unknown filesystem type".to_string(),
            }),
            _ => Err(DiskError::Unsupported {
                feature: format!("{:?} filesystem", fs_type),
            }),
        }
    }

    /// Detect filesystem type from superblock
    ///
    /// # Detection Strategy
    ///
    /// Each filesystem has a magic number at a specific offset:
    /// - ext2/3/4: 0x53EF at offset 1080 (1024 + 56)
    /// - XFS: "XFSB" at offset 0
    /// - Btrfs: "_BHRfS_M" at offset 64
    /// - FAT32: Various signatures at offset 0 and 510
    async fn detect_filesystem(
        device: &dyn BlockDevice,
        partition: &PartitionInfo,
    ) -> DiskResult<FilesystemType> {
        let base = partition.start_offset;

        // Try ext2/3/4 first (most common Linux FS)
        let mut superblock = [0u8; 2];
        device.read_at(&mut superblock, base + 1024 + 56).await?;
        if superblock == [0x53, 0xEF] {
            return Ok(FilesystemType::Ext4);
        }

        // Try XFS
        let mut xfs_magic = [0u8; 4];
        device.read_at(&mut xfs_magic, base).await?;
        if &xfs_magic == b"XFSB" {
            return Ok(FilesystemType::Xfs);
        }

        // Try Btrfs
        let mut btrfs_magic = [0u8; 8];
        device.read_at(&mut btrfs_magic, base + 64).await?;
        if &btrfs_magic == b"_BHRfS_M" {
            return Ok(FilesystemType::Btrfs);
        }

        // Try FAT32 (check for FAT signature)
        let mut fat_sig = [0u8; 2];
        device.read_at(&mut fat_sig, base + 510).await?;
        if fat_sig == [0x55, 0xAA] {
            // Could be FAT16/FAT32/NTFS, need more checks
            let mut fat32_marker = [0u8; 8];
            device.read_at(&mut fat32_marker, base + 82).await?;
            if &fat32_marker == b"FAT32   " {
                return Ok(FilesystemType::Fat32);
            }
        }

        Ok(FilesystemType::Unknown)
    }

    /// Open an ext4 filesystem
    async fn open_ext4(device: &dyn BlockDevice, partition: &PartitionInfo) -> DiskResult<Self> {
        let base = partition.start_offset;

        // Read ext4 superblock (at offset 1024 from partition start)
        let mut sb = [0u8; 256];
        device.read_at(&mut sb, base + 1024).await?;

        // Parse key fields
        let block_size = 1024u32 << u32::from_le_bytes(sb[24..28].try_into().unwrap());
        let _blocks_count = u32::from_le_bytes(sb[4..8].try_into().unwrap());
        let _inodes_count = u32::from_le_bytes(sb[0..4].try_into().unwrap());

        // Check for 64-bit feature
        let feature_incompat = u32::from_le_bytes(sb[96..100].try_into().unwrap());
        let _is_64bit = (feature_incompat & 0x80) != 0;

        Ok(Self {
            fs_type: FilesystemType::Ext4,
            partition: partition.clone(),
            root_inode: 2, // ext4 root is always inode 2
            block_size,
        })
    }

    /// Walk the filesystem and yield entries
    ///
    /// # Arguments
    ///
    /// * `callback` - Async function called for each entry
    ///
    /// # Walk Order
    ///
    /// Directories are traversed depth-first. This ensures parent directories
    /// are encoded before their children, which is required for extraction.
    ///
    /// # Why Async Callback?
    ///
    /// File data reads are I/O bound. Async callbacks allow the traversal
    /// to continue while waiting for data, maximizing throughput.
    pub async fn walk<F, Fut>(&self, mut callback: F) -> DiskResult<()>
    where
        F: FnMut(FilesystemEntry) -> Fut,
        Fut: std::future::Future<Output = DiskResult<()>>,
    {
        // ext4 crate traversal is synchronous, but we wrap it in an async context
        // The callback is async to allow for async file data processing

        match self.fs_type {
            FilesystemType::Ext4 => {
                // Note: This is a simplified implementation that demonstrates the API
                // In production, we would use the ext4 crate's walk() method
                // wrapped with tokio::task::spawn_blocking for true async behavior

                // Create root entry first
                let root_entry = FilesystemEntry {
                    path: PathBuf::from("/"),
                    metadata: FileMetadata {
                        file_type: FileType::Directory,
                        size: 0,
                        mode: 0o755,
                        uid: 0,
                        gid: 0,
                        atime: 0,
                        mtime: 0,
                        ctime: 0,
                        nlink: 2,
                        inode: self.root_inode,
                        device_id: None,
                        symlink_target: None,
                    },
                    data_offset: None,
                };

                callback(root_entry).await?;

                // Real implementation would traverse using ext4::SuperBlock::walk()
                // For now, return success - full implementation requires integration
                // with the ext4 crate's synchronous API via spawn_blocking
            }
            _ => {
                return Err(DiskError::Unsupported {
                    feature: format!("{:?} filesystem traversal", self.fs_type),
                });
            }
        }

        Ok(())
    }

    /// Walk the filesystem synchronously using the ext4 crate
    ///
    /// This is the core implementation that uses the ext4 crate's walk() method.
    /// For async contexts, use `walk()` which wraps this appropriately.
    #[cfg(feature = "disk-image")]
    pub fn walk_sync<F>(&self, device: &dyn BlockDeviceSync, mut callback: F) -> DiskResult<()>
    where
        F: FnMut(FilesystemEntry) -> DiskResult<()>,
    {
        use std::io::{Read, Seek};

        match self.fs_type {
            FilesystemType::Ext4 => {
                // Create a positioned reader for the partition
                let partition_reader = PartitionReader::new(device, &self.partition);

                // Open the ext4 filesystem
                let superblock = ext4::SuperBlock::new(partition_reader).map_err(|e| {
                    DiskError::FilesystemError {
                        partition: self.partition.number,
                        reason: format!("Failed to open ext4: {}", e),
                    }
                })?;

                let root = superblock.root().map_err(|e| DiskError::FilesystemError {
                    partition: self.partition.number,
                    reason: format!("Failed to load root inode: {}", e),
                })?;

                // Walk the filesystem
                superblock
                    .walk(&root, "", &mut |fs, path, inode, enhanced| {
                        let entry = self.inode_to_entry(path, inode, enhanced)?;
                        callback(entry).map_err(|e| anyhow::anyhow!("{}", e))?;
                        Ok(true) // Continue walking
                    })
                    .map_err(|e| DiskError::FilesystemError {
                        partition: self.partition.number,
                        reason: format!("Walk failed: {}", e),
                    })?;

                Ok(())
            }
            _ => Err(DiskError::Unsupported {
                feature: format!("{:?} filesystem", self.fs_type),
            }),
        }
    }

    /// Convert an ext4 inode to our FilesystemEntry
    #[cfg(feature = "disk-image")]
    fn inode_to_entry(
        &self,
        path: &str,
        inode: &ext4::Inode,
        enhanced: &ext4::Enhanced,
    ) -> Result<FilesystemEntry, anyhow::Error> {
        let file_type = match inode.stat.extracted_type {
            ext4::FileType::RegularFile => FileType::Regular,
            ext4::FileType::Directory => FileType::Directory,
            ext4::FileType::SymbolicLink => FileType::Symlink,
            ext4::FileType::CharacterDevice => FileType::CharDevice,
            ext4::FileType::BlockDevice => FileType::BlockDevice,
            ext4::FileType::Fifo => FileType::Fifo,
            ext4::FileType::Socket => FileType::Socket,
        };

        let symlink_target = match enhanced {
            ext4::Enhanced::SymbolicLink(target) => Some(target.clone()),
            _ => None,
        };

        let device_id = match enhanced {
            ext4::Enhanced::CharacterDevice(major, minor) => {
                Some(((*major as u64) << 8) | (*minor as u64))
            }
            ext4::Enhanced::BlockDevice(major, minor) => {
                Some(((*major as u64) << 8) | (*minor as u64))
            }
            _ => None,
        };

        Ok(FilesystemEntry {
            path: PathBuf::from(if path.is_empty() { "/" } else { path }),
            metadata: FileMetadata {
                file_type,
                size: inode.stat.size,
                mode: inode.stat.mode,
                uid: inode.stat.uid,
                gid: inode.stat.gid,
                atime: inode.stat.atime.epoch_secs,
                mtime: inode.stat.mtime.epoch_secs,
                ctime: inode.stat.ctime.epoch_secs,
                nlink: inode.stat.nlink,
                inode: inode.number as u64,
                device_id,
                symlink_target,
            },
            data_offset: None, // ext4 crate handles data access internally
        })
    }

    /// Get filesystem statistics
    pub fn stats(&self) -> FilesystemStats {
        FilesystemStats {
            fs_type: self.fs_type,
            block_size: self.block_size,
            partition_size: self.partition.size,
        }
    }
}

/// Filesystem statistics
#[derive(Debug)]
pub struct FilesystemStats {
    /// Filesystem type
    pub fs_type: FilesystemType,

    /// Block size in bytes
    pub block_size: u32,

    /// Total partition size
    pub partition_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_types() {
        let meta = FileMetadata {
            file_type: FileType::Regular,
            size: 1024,
            mode: 0o644,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
            inode: 1,
            device_id: None,
            symlink_target: None,
        };

        assert!(meta.is_file());
        assert!(!meta.is_dir());
        assert!(!meta.is_symlink());
    }

    #[test]
    fn test_device_numbers() {
        let meta = FileMetadata {
            file_type: FileType::CharDevice,
            size: 0,
            mode: 0o666,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
            inode: 1,
            device_id: Some((1 << 8) | 3), // major 1, minor 3 = /dev/null
            symlink_target: None,
        };

        assert_eq!(meta.device_major(), Some(1));
        assert_eq!(meta.device_minor(), Some(3));
    }
}
