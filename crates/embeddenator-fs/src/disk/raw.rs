//! Raw Disk Image Support
//!
//! Raw images are the simplest disk image format: a direct byte-for-byte
//! copy of a block device with no metadata or structure.
//!
//! # Why Support Raw Images?
//!
//! 1. **Simplicity**: No format parsing needed - just read bytes at offsets.
//!    This makes raw images the most reliable format for testing.
//!
//! 2. **Performance**: No L1/L2 table lookups, no decompression. Reads go
//!    directly to the data with optimal I/O patterns.
//!
//! 3. **Compatibility**: Every tool understands raw images. `dd`, `losetup`,
//!    `qemu-img`, and all OS disk utilities work with raw images.
//!
//! 4. **Conversion Target**: Raw images are the common denominator for
//!    converting between formats with `qemu-img convert`.
//!
//! # Sparse File Support
//!
//! Modern filesystems (ext4, XFS, NTFS, APFS) support sparse files where
//! unwritten regions don't consume disk space. A 100GB raw image might
//! only use 10GB if most of it is zeros.
//!
//! # Why Async?
//!
//! While raw reads are simpler than QCOW2, large disk images still benefit
//! from async I/O:
//!
//! - **io_uring batching**: Multiple read requests can be submitted together
//! - **Non-blocking**: Other tasks continue while waiting for disk I/O
//! - **Consistent API**: Same interface as QCOW2 for unified handling
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_fs::disk::RawImage;
//!
//! let image = RawImage::open("disk.img").await?;
//! println!("Size: {} bytes", image.size());
//!
//! // Read the MBR (first 512 bytes)
//! let mut mbr = [0u8; 512];
//! image.read_at(&mut mbr, 0).await?;
//!
//! // Check for MBR signature
//! assert_eq!(&mbr[510..512], &[0x55, 0xAA]);
//! ```

use super::error::{DiskError, DiskResult};
use super::BlockDevice;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;

/// Raw disk image wrapper
///
/// # Why a Wrapper for Raw Images?
///
/// Even though raw images are simple, wrapping them provides:
///
/// 1. **Consistent Interface**: Same `BlockDevice` trait as QCOW2
/// 2. **Size Caching**: Avoid repeated `fstat` calls for size queries
/// 3. **Error Context**: Add path info to I/O errors
/// 4. **Future Optimization**: Room for read-ahead, caching, io_uring
pub struct RawImage {
    /// Path to the image file (for error messages)
    path: PathBuf,

    /// Cached file size
    size: u64,

    /// File handle wrapped for async access
    /// Mutex because tokio::fs::File is not Sync
    file: Arc<Mutex<File>>,
}

impl RawImage {
    /// Open a raw disk image file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the raw image file
    ///
    /// # Errors
    ///
    /// Returns `DiskError::Io` for file access errors.
    ///
    /// # Why No Format Validation?
    ///
    /// Raw images have no magic bytes or header - they're just bytes.
    /// We trust the caller to provide a valid disk image. Invalid data
    /// will be caught later during partition table or filesystem parsing.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let image = RawImage::open("/dev/sda").await?;  // Real device
    /// let image = RawImage::open("disk.img").await?;  // Image file
    /// ```
    pub async fn open(path: impl AsRef<Path>) -> DiskResult<Self> {
        let path = path.as_ref().to_path_buf();

        let file = File::open(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DiskError::InvalidFormat {
                    path: path.clone(),
                    reason: "File not found".to_string(),
                }
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                DiskError::InvalidFormat {
                    path: path.clone(),
                    reason: "Permission denied (try running as root for block devices)".to_string(),
                }
            } else {
                DiskError::Io(e)
            }
        })?;

        let metadata = file.metadata().await?;
        let size = metadata.len();

        // Sanity check: disk images should have some content
        if size == 0 {
            return Err(DiskError::InvalidFormat {
                path,
                reason: "File is empty".to_string(),
            });
        }

        Ok(Self {
            path,
            size,
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Create a raw image from an existing file handle
    ///
    /// # Why Allow External File Handles?
    ///
    /// Sometimes the caller already has a file open (e.g., from a pipe or
    /// network stream). This allows wrapping any async file-like object
    /// as a RawImage.
    pub fn from_file(path: PathBuf, file: File, size: u64) -> Self {
        Self {
            path,
            size,
            file: Arc::new(Mutex::new(file)),
        }
    }

    /// Get the file path
    ///
    /// # Why Expose Path?
    ///
    /// Useful for error messages and logging. The path tells users which
    /// image had a problem when encoding multiple images.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if this is a sparse file
    ///
    /// # Why Check Sparseness?
    ///
    /// Sparse files can appear much larger than their actual disk usage.
    /// This information helps estimate encoding time and resource needs.
    ///
    /// # Returns
    ///
    /// `true` if the file uses less disk space than its logical size
    #[cfg(target_os = "linux")]
    pub async fn is_sparse(&self) -> DiskResult<bool> {
        use std::os::unix::fs::MetadataExt;

        let file = self.file.lock().await;
        let metadata = file.metadata().await?;

        // blocks * 512 is actual disk usage
        // metadata.len() is logical size
        let disk_usage = metadata.blocks() * 512;
        Ok(disk_usage < metadata.len())
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn is_sparse(&self) -> DiskResult<bool> {
        // Can't easily detect on non-Linux
        Ok(false)
    }
}

impl BlockDevice for RawImage {
    async fn read_at(&self, buf: &mut [u8], offset: u64) -> DiskResult<usize> {
        if offset >= self.size {
            return Err(DiskError::OutOfBounds {
                requested: offset,
                size: self.size,
            });
        }

        let mut file = self.file.lock().await;

        // Seek to offset
        file.seek(std::io::SeekFrom::Start(offset)).await?;

        // Clamp read to file size
        let available = (self.size - offset) as usize;
        let to_read = buf.len().min(available);

        // Read data
        let bytes_read = file.read(&mut buf[..to_read]).await?;

        Ok(bytes_read)
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn block_size(&self) -> u32 {
        // Could detect actual block device sector size on Linux via ioctl
        // For now, assume standard 512-byte sectors
        512
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_open_raw_image() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.img");

        // Create a small raw image with MBR signature
        let mut data = vec![0u8; 1024];
        data[510] = 0x55;
        data[511] = 0xAA;
        tokio::fs::write(&path, &data).await.unwrap();

        let image = RawImage::open(&path).await.unwrap();
        assert_eq!(image.size(), 1024);
    }

    #[tokio::test]
    async fn test_read_at() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.img");

        // Create test data
        let data: Vec<u8> = (0..256).collect();
        tokio::fs::write(&path, &data).await.unwrap();

        let image = RawImage::open(&path).await.unwrap();

        // Read from middle
        let mut buf = [0u8; 10];
        let read = image.read_at(&mut buf, 100).await.unwrap();
        assert_eq!(read, 10);
        assert_eq!(&buf, &data[100..110]);
    }

    #[tokio::test]
    async fn test_empty_file_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.img");
        tokio::fs::write(&path, b"").await.unwrap();

        let result = RawImage::open(&path).await;
        assert!(matches!(result, Err(DiskError::InvalidFormat { .. })));
    }

    #[tokio::test]
    async fn test_out_of_bounds_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("small.img");
        tokio::fs::write(&path, b"small").await.unwrap();

        let image = RawImage::open(&path).await.unwrap();

        let mut buf = [0u8; 10];
        let result = image.read_at(&mut buf, 1000).await;
        assert!(matches!(result, Err(DiskError::OutOfBounds { .. })));
    }
}
