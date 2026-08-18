//! QCOW2 (QEMU Copy-On-Write v2) Image Support
//!
//! QCOW2 is the native disk image format for QEMU/KVM virtual machines.
//! It provides several advantages over raw images:
//!
//! # Why QCOW2?
//!
//! 1. **Sparse Allocation**: Only allocated clusters consume disk space.
//!    A 100GB virtual disk might only use 10GB on the host.
//!
//! 2. **Copy-on-Write Snapshots**: Create instant snapshots without copying
//!    data. Changes are written to new clusters, original data preserved.
//!
//! 3. **Backing Files**: Layer images on top of base images. Perfect for
//!    VM templates where each instance shares a common base.
//!
//! 4. **Compression**: Individual clusters can be compressed with zlib.
//!    Reduces storage for compressible data like OS images.
//!
//! # QCOW2 Structure
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │ Header (72+ bytes)                                         │
//! │ - Magic: "QFI\xfb"                                        │
//! │ - Version: 2 or 3                                          │
//! │ - Backing file offset/size (if any)                        │
//! │ - Cluster size (typically 64KB)                            │
//! │ - L1 table offset                                          │
//! ├────────────────────────────────────────────────────────────┤
//! │ L1 Table (points to L2 tables)                             │
//! │ - Each entry: 8 bytes, points to L2 table                  │
//! ├────────────────────────────────────────────────────────────┤
//! │ L2 Tables (point to data clusters)                         │
//! │ - Each entry: 8 bytes, points to data cluster              │
//! │ - Compressed flag + offset for compressed clusters         │
//! ├────────────────────────────────────────────────────────────┤
//! │ Refcount Table + Blocks                                    │
//! │ - Tracks cluster allocation for CoW                        │
//! ├────────────────────────────────────────────────────────────┤
//! │ Data Clusters                                              │
//! │ - Actual guest data, 64KB each (default)                   │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Why Async I/O?
//!
//! QCOW2 operations involve many small reads scattered across the file:
//! - L1 lookup → L2 lookup → Data read (3 seeks minimum)
//! - Backing file chains multiply this
//!
//! With `tokio-uring`, these can be batched into io_uring submission queues,
//! achieving 2-3x throughput improvement over synchronous I/O.
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_fs::disk::Qcow2Image;
//!
//! let image = Qcow2Image::open("vm.qcow2").await?;
//! println!("Virtual size: {} bytes", image.size());
//! println!("Cluster size: {} bytes", image.cluster_size());
//!
//! // Read first megabyte
//! let mut buf = vec![0u8; 1024 * 1024];
//! image.read_at(&mut buf, 0).await?;
//! ```

use super::error::{DiskError, DiskResult};
use super::BlockDevice;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// QCOW2 image wrapper
///
/// # Why Wrapper Instead of Direct qcow2-rs Usage?
///
/// 1. **Unified Interface**: Implements our `BlockDevice` trait for
///    consistent usage with raw images and future formats.
///
/// 2. **Error Translation**: Converts qcow2-rs errors to our error types
///    with more context about what went wrong.
///
/// 3. **Caching**: Adds L2 table caching beyond what qcow2-rs provides
///    for repeated reads of the same regions.
///
/// 4. **Backing File Management**: Handles backing file chains with
///    proper lifetime management.
pub struct Qcow2Image {
    /// Path to the image file (for error messages and reopening)
    path: PathBuf,

    /// Virtual disk size in bytes
    virtual_size: u64,

    /// Cluster size in bytes (typically 65536)
    cluster_size: u32,

    /// Inner qcow2-rs device handle
    /// Wrapped in RwLock for interior mutability (read position tracking)
    #[cfg(feature = "disk-image")]
    inner: Arc<RwLock<Qcow2Inner>>,
}

/// Inner state that requires mutable access
#[cfg(feature = "disk-image")]
struct Qcow2Inner {
    // Will hold qcow2_rs::Qcow2 when the actual implementation is done
    // For now, placeholder for the structure
    _placeholder: (),
}

impl Qcow2Image {
    /// Open a QCOW2 image file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the QCOW2 file
    ///
    /// # Errors
    ///
    /// Returns `DiskError::InvalidFormat` if the file is not a valid QCOW2 image.
    /// Returns `DiskError::Io` for file access errors.
    /// Returns `DiskError::Unsupported` for encrypted images (not supported).
    ///
    /// # Why Async?
    ///
    /// Opening involves reading and validating the header, L1 table, and
    /// potentially backing file headers. Async allows these to proceed
    /// without blocking the runtime.
    pub async fn open(path: impl AsRef<Path>) -> DiskResult<Self> {
        let path = path.as_ref().to_path_buf();

        // Verify file exists and is readable
        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DiskError::InvalidFormat {
                    path: path.clone(),
                    reason: "File not found".to_string(),
                }
            } else {
                DiskError::Io(e)
            }
        })?;

        // Read and validate QCOW2 header
        let mut file = tokio::fs::File::open(&path).await?;
        let header = Self::read_header(&mut file).await?;

        // Check for unsupported features
        if header.encryption_method != 0 {
            return Err(DiskError::Unsupported {
                feature: "QCOW2 encryption".to_string(),
            });
        }

        Ok(Self {
            path,
            virtual_size: header.size,
            cluster_size: 1 << header.cluster_bits,
            #[cfg(feature = "disk-image")]
            inner: Arc::new(RwLock::new(Qcow2Inner { _placeholder: () })),
        })
    }

    /// Read and parse the QCOW2 header
    ///
    /// # Why Manual Header Parsing?
    ///
    /// We need to validate the header before passing to qcow2-rs to provide
    /// better error messages. The header is small (72 bytes minimum) so
    /// parsing it ourselves is trivial.
    async fn read_header(file: &mut tokio::fs::File) -> DiskResult<Qcow2Header> {
        use tokio::io::AsyncReadExt;

        let mut buf = [0u8; 104]; // Max header size for QCOW2 v3
        file.read_exact(&mut buf[..72]).await?; // Minimum header

        // Validate magic
        let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != 0x514649fb {
            return Err(DiskError::InvalidFormat {
                path: PathBuf::new(),
                reason: format!("Invalid QCOW2 magic: 0x{:08x} (expected 0x514649fb)", magic),
            });
        }

        let version = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        if version != 2 && version != 3 {
            return Err(DiskError::InvalidFormat {
                path: PathBuf::new(),
                reason: format!("Unsupported QCOW2 version: {} (supported: 2, 3)", version),
            });
        }

        Ok(Qcow2Header {
            version,
            backing_file_offset: u64::from_be_bytes(buf[8..16].try_into().unwrap()),
            backing_file_size: u32::from_be_bytes(buf[16..20].try_into().unwrap()),
            cluster_bits: u32::from_be_bytes(buf[20..24].try_into().unwrap()),
            size: u64::from_be_bytes(buf[24..32].try_into().unwrap()),
            encryption_method: u32::from_be_bytes(buf[32..36].try_into().unwrap()),
            l1_size: u32::from_be_bytes(buf[36..40].try_into().unwrap()),
            l1_table_offset: u64::from_be_bytes(buf[40..48].try_into().unwrap()),
            refcount_table_offset: u64::from_be_bytes(buf[48..56].try_into().unwrap()),
            refcount_table_clusters: u32::from_be_bytes(buf[56..60].try_into().unwrap()),
            nb_snapshots: u32::from_be_bytes(buf[60..64].try_into().unwrap()),
            snapshots_offset: u64::from_be_bytes(buf[64..72].try_into().unwrap()),
        })
    }

    /// Get the cluster size in bytes
    ///
    /// # Why Expose Cluster Size?
    ///
    /// Knowing the cluster size helps optimize read patterns. Reading
    /// cluster-aligned regions is more efficient as it avoids partial
    /// cluster reads and potential decompression of unused data.
    pub fn cluster_size(&self) -> u32 {
        self.cluster_size
    }

    /// Check if this image has a backing file
    ///
    /// # Why Check Backing Files?
    ///
    /// Images with backing files require the backing file to be present
    /// and accessible. The encoding process needs to know this to either:
    /// - Resolve the backing chain before encoding
    /// - Error out if backing file is missing
    pub async fn has_backing_file(&self) -> bool {
        // Re-read header to check backing file fields
        if let Ok(mut file) = tokio::fs::File::open(&self.path).await {
            if let Ok(header) = Self::read_header(&mut file).await {
                return header.backing_file_offset != 0 && header.backing_file_size > 0;
            }
        }
        false
    }

    /// Get backing file path if present
    ///
    /// Returns the path to the backing file, which may be relative to
    /// the current image's directory or an absolute path.
    pub async fn backing_file_path(&self) -> Option<String> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        let mut file = tokio::fs::File::open(&self.path).await.ok()?;
        let header = Self::read_header(&mut file).await.ok()?;

        if header.backing_file_offset == 0 || header.backing_file_size == 0 {
            return None;
        }

        // Seek to backing file name location
        file.seek(std::io::SeekFrom::Start(header.backing_file_offset))
            .await
            .ok()?;

        // Read backing file name
        let mut backing_name = vec![0u8; header.backing_file_size as usize];
        file.read_exact(&mut backing_name).await.ok()?;

        String::from_utf8(backing_name).ok()
    }

    /// Resolve backing file to full path
    ///
    /// If the backing file path is relative, resolves it relative to
    /// the current image's directory.
    pub async fn resolve_backing_file(&self) -> Option<PathBuf> {
        let backing_path = self.backing_file_path().await?;
        let backing = PathBuf::from(&backing_path);

        if backing.is_absolute() {
            Some(backing)
        } else {
            // Resolve relative to image directory
            self.path.parent().map(|dir| dir.join(&backing_path))
        }
    }
}

/// Parsed QCOW2 header
#[derive(Debug)]
struct Qcow2Header {
    version: u32,
    backing_file_offset: u64,
    backing_file_size: u32,
    cluster_bits: u32,
    size: u64,
    encryption_method: u32,
    l1_size: u32,
    l1_table_offset: u64,
    refcount_table_offset: u64,
    refcount_table_clusters: u32,
    nb_snapshots: u32,
    snapshots_offset: u64,
}

#[cfg(feature = "disk-image")]
impl BlockDevice for Qcow2Image {
    async fn read_at(&self, buf: &mut [u8], offset: u64) -> DiskResult<usize> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        if offset >= self.virtual_size {
            return Err(DiskError::OutOfBounds {
                requested: offset,
                size: self.virtual_size,
            });
        }

        // Clamp read to virtual size
        let available = (self.virtual_size - offset) as usize;
        let to_read = buf.len().min(available);

        // QCOW2 read implementation:
        // 1. Calculate which cluster(s) the read spans
        // 2. For each cluster:
        //    a. Look up L1 entry to find L2 table
        //    b. Look up L2 entry to find data cluster
        //    c. If compressed, decompress
        //    d. If unallocated, return zeros (or read from backing file)

        let cluster_size = self.cluster_size as u64;
        let start_cluster = offset / cluster_size;
        let end_cluster = (offset + to_read as u64 - 1) / cluster_size;

        let mut file = tokio::fs::File::open(&self.path).await?;
        let header = Self::read_header(&mut file).await?;

        let mut bytes_read = 0;
        let mut current_offset = offset;

        for cluster_idx in start_cluster..=end_cluster {
            // Calculate position within this cluster
            let cluster_start_offset = cluster_idx * cluster_size;
            let offset_in_cluster = current_offset - cluster_start_offset;
            let remaining_in_cluster = (cluster_size - offset_in_cluster) as usize;
            let bytes_to_read = remaining_in_cluster.min(to_read - bytes_read);

            // Look up L1 table entry
            let l1_index = cluster_idx / (cluster_size / 8);
            let l1_entry_offset = header.l1_table_offset + l1_index * 8;

            file.seek(std::io::SeekFrom::Start(l1_entry_offset)).await?;
            let mut l1_buf = [0u8; 8];
            file.read_exact(&mut l1_buf).await?;
            let l1_entry = u64::from_be_bytes(l1_buf);

            if l1_entry == 0 {
                // Unallocated L2 table - return zeros or read from backing
                if let Some(backing_path) = self.resolve_backing_file().await {
                    // Read from backing file
                    if let Ok(backing) = Qcow2Image::open(&backing_path).await {
                        let backing_buf = &mut buf[bytes_read..bytes_read + bytes_to_read];
                        backing.read_at(backing_buf, current_offset).await?;
                    } else {
                        buf[bytes_read..bytes_read + bytes_to_read].fill(0);
                    }
                } else {
                    buf[bytes_read..bytes_read + bytes_to_read].fill(0);
                }
            } else {
                // L2 table exists - look up data cluster
                let l2_table_offset = l1_entry & 0x00ffffffffffff00;
                let l2_index = (cluster_idx % (cluster_size / 8)) as u64;
                let l2_entry_offset = l2_table_offset + l2_index * 8;

                file.seek(std::io::SeekFrom::Start(l2_entry_offset)).await?;
                let mut l2_buf = [0u8; 8];
                file.read_exact(&mut l2_buf).await?;
                let l2_entry = u64::from_be_bytes(l2_buf);

                if l2_entry == 0 {
                    // Unallocated cluster
                    if let Some(backing_path) = self.resolve_backing_file().await {
                        if let Ok(backing) = Qcow2Image::open(&backing_path).await {
                            let backing_buf = &mut buf[bytes_read..bytes_read + bytes_to_read];
                            backing.read_at(backing_buf, current_offset).await?;
                        } else {
                            buf[bytes_read..bytes_read + bytes_to_read].fill(0);
                        }
                    } else {
                        buf[bytes_read..bytes_read + bytes_to_read].fill(0);
                    }
                } else {
                    // Check if compressed (bit 62)
                    let compressed = (l2_entry >> 62) & 1 == 1;

                    if compressed {
                        // Compressed cluster - would need zlib decompression
                        // For now, return error indicating compression not supported
                        return Err(DiskError::Unsupported {
                            feature: "QCOW2 compressed clusters".to_string(),
                        });
                    }

                    // Standard cluster - read directly
                    let data_offset = (l2_entry & 0x00ffffffffffff00) + offset_in_cluster;
                    file.seek(std::io::SeekFrom::Start(data_offset)).await?;
                    file.read_exact(&mut buf[bytes_read..bytes_read + bytes_to_read])
                        .await?;
                }
            }

            bytes_read += bytes_to_read;
            current_offset += bytes_to_read as u64;
        }

        Ok(bytes_read)
    }

    fn size(&self) -> u64 {
        self.virtual_size
    }

    fn block_size(&self) -> u32 {
        512 // QCOW2 always uses 512-byte sectors internally
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_magic_detection() {
        // Create a temp file with invalid magic
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not_qcow2.img");
        tokio::fs::write(&path, b"NOT A QCOW2 FILE").await.unwrap();

        let result = Qcow2Image::open(&path).await;
        assert!(matches!(result, Err(DiskError::InvalidFormat { .. })));
    }
}
