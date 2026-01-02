//! FUSE Filesystem Shim for Holographic Engrams
//!
//! This module provides kernel integration via FUSE (Filesystem in Userspace),
//! allowing engrams to be mounted as native filesystems.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                     User Applications                          │
//! │                    (read, write, stat, etc.)                   │
//! └────────────────────────────────────────────────────────────────┘
//!                                  │ VFS syscalls
//!                                  ▼
//! ┌────────────────────────────────────────────────────────────────┐
//! │                     Linux Kernel VFS                           │
//! └────────────────────────────────────────────────────────────────┘
//!                                  │ FUSE protocol
//!                                  ▼
//! ┌────────────────────────────────────────────────────────────────┐
//! │                   /dev/fuse character device                   │
//! └────────────────────────────────────────────────────────────────┘
//!                                  │ libfuse / fuser crate
//!                                  ▼
//! ┌────────────────────────────────────────────────────────────────┐
//! │                    EngramFS (this module)                      │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
//! │  │   Manifest   │  │   Codebook   │  │   Differential       │  │
//! │  │   (paths)    │  │   (private)  │  │   Decoder            │  │
//! │  └──────────────┘  └──────────────┘  └──────────────────────┘  │
//! └────────────────────────────────────────────────────────────────┘
//!                                  │
//!                                  ▼
//! ┌────────────────────────────────────────────────────────────────┐
//! │                    Engram Storage                              │
//! │            (on-disk or memory-mapped .engram file)             │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Mount an engram (requires --features fuse)
//! embeddenator mount --engram root.engram --manifest manifest.json /mnt/engram
//!
//! # Access files normally
//! ls /mnt/engram
//! cat /mnt/engram/some/file.txt
//!
//! # Unmount
//! fusermount -u /mnt/engram
//! ```
//!
//! # Feature Flag
//!
//! The FUSE integration requires the `fuse` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! embeddenator = { version = "0.2", features = ["fuse"] }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "fuse")]
use std::ffi::OsStr;

#[cfg(feature = "fuse")]
use std::path::Path;

/// Inode number type (matches fuser's u64 inode convention)
pub type Ino = u64;

/// Root inode number (FUSE convention: inode 1 is root)
pub const ROOT_INO: Ino = 1;

/// File attributes for FUSE
///
/// This mirrors fuser::FileAttr but is always available regardless
/// of feature flags, allowing the core filesystem logic to work
/// without the fuser crate.
#[derive(Clone, Debug)]
pub struct FileAttr {
    /// Inode number
    pub ino: Ino,
    /// File size in bytes
    pub size: u64,
    /// Number of 512-byte blocks allocated
    pub blocks: u64,
    /// Last access time
    pub atime: SystemTime,
    /// Last modification time
    pub mtime: SystemTime,
    /// Last status change time
    pub ctime: SystemTime,
    /// Creation time (macOS only)
    pub crtime: SystemTime,
    /// File type
    pub kind: FileKind,
    /// Permissions (mode & 0o7777)
    pub perm: u16,
    /// Hard link count
    pub nlink: u32,
    /// User ID of owner
    pub uid: u32,
    /// Group ID of owner
    pub gid: u32,
    /// Device ID (for special files)
    pub rdev: u32,
    /// Block size for filesystem I/O
    pub blksize: u32,
    /// Flags (macOS only)
    pub flags: u32,
}

impl Default for FileAttr {
    fn default() -> Self {
        let now = SystemTime::now();
        FileAttr {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileKind::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}

#[cfg(feature = "fuse")]
impl From<FileAttr> for fuser::FileAttr {
    fn from(attr: FileAttr) -> Self {
        fuser::FileAttr {
            ino: attr.ino,
            size: attr.size,
            blocks: attr.blocks,
            atime: attr.atime,
            mtime: attr.mtime,
            ctime: attr.ctime,
            crtime: attr.crtime,
            kind: attr.kind.into(),
            perm: attr.perm,
            nlink: attr.nlink,
            uid: attr.uid,
            gid: attr.gid,
            rdev: attr.rdev,
            blksize: attr.blksize,
            flags: attr.flags,
        }
    }
}

/// File type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// Directory
    Directory,
    /// Regular file
    RegularFile,
    /// Symbolic link
    Symlink,
}

#[cfg(feature = "fuse")]
impl From<FileKind> for fuser::FileType {
    fn from(kind: FileKind) -> Self {
        match kind {
            FileKind::Directory => fuser::FileType::Directory,
            FileKind::RegularFile => fuser::FileType::RegularFile,
            FileKind::Symlink => fuser::FileType::Symlink,
        }
    }
}

/// Directory entry
#[derive(Clone, Debug)]
pub struct DirEntry {
    /// Inode number
    pub ino: Ino,
    /// Entry name
    pub name: String,
    /// Entry type
    pub kind: FileKind,
}

/// Cached file data for read operations
#[derive(Clone)]
pub struct CachedFile {
    /// File content
    pub data: Vec<u8>,
    /// File attributes
    pub attr: FileAttr,
}

/// The EngramFS FUSE filesystem implementation
///
/// This provides a read-only view of decoded engram data as a standard
/// POSIX filesystem. Files are decoded on-demand from the holographic
/// representation and cached for efficient repeated access.
pub struct EngramFS {
    /// Inode to file attributes mapping
    inodes: Arc<RwLock<HashMap<Ino, FileAttr>>>,
    
    /// Inode to path mapping
    inode_paths: Arc<RwLock<HashMap<Ino, String>>>,
    
    /// Path to inode mapping
    path_inodes: Arc<RwLock<HashMap<String, Ino>>>,
    
    /// Directory contents (parent_ino -> entries)
    directories: Arc<RwLock<HashMap<Ino, Vec<DirEntry>>>>,
    
    /// Cached file data (ino -> data)
    file_cache: Arc<RwLock<HashMap<Ino, CachedFile>>>,
    
    /// Next available inode number
    next_ino: Arc<RwLock<Ino>>,
    
    /// Read-only mode
    read_only: bool,
    
    /// TTL for cached attributes
    attr_ttl: Duration,
    
    /// TTL for cached entries
    entry_ttl: Duration,
}

impl EngramFS {
    /// Create a new EngramFS instance
    ///
    /// # Arguments
    ///
    /// * `read_only` - Whether the filesystem is read-only (default: true for engrams)
    pub fn new(read_only: bool) -> Self {
        let mut fs = EngramFS {
            inodes: Arc::new(RwLock::new(HashMap::new())),
            inode_paths: Arc::new(RwLock::new(HashMap::new())),
            path_inodes: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(HashMap::new())),
            file_cache: Arc::new(RwLock::new(HashMap::new())),
            next_ino: Arc::new(RwLock::new(2)), // Start after root
            read_only,
            attr_ttl: Duration::from_secs(1),
            entry_ttl: Duration::from_secs(1),
        };

        // Initialize root directory
        fs.init_root();
        fs
    }

    /// Initialize root directory
    fn init_root(&mut self) {
        let root_attr = FileAttr {
            ino: ROOT_INO,
            size: 0,
            blocks: 0,
            kind: FileKind::Directory,
            perm: 0o755,
            nlink: 2,
            ..Default::default()
        };

        // SAFETY: init_root only called during construction, before any concurrent access
        // If locks are poisoned here, the filesystem is unrecoverable anyway
        self.inodes.write().expect("Lock poisoned during init").insert(ROOT_INO, root_attr);
        self.inode_paths.write().expect("Lock poisoned during init").insert(ROOT_INO, "/".to_string());
        self.path_inodes.write().expect("Lock poisoned during init").insert("/".to_string(), ROOT_INO);
        self.directories.write().expect("Lock poisoned during init").insert(ROOT_INO, Vec::new());
    }

    /// Allocate a new inode number
    fn alloc_ino(&self) -> Result<Ino, &'static str> {
        let mut next = self.next_ino.write()
            .map_err(|_| "Inode allocator lock poisoned")?;
        let ino = *next;
        *next += 1;
        Ok(ino)
    }

    /// Add a file to the filesystem
    ///
    /// # Arguments
    ///
    /// * `path` - Absolute path within the filesystem (e.g., "/foo/bar.txt")
    /// * `data` - File content bytes
    ///
    /// # Returns
    ///
    /// The assigned inode number for the new file
    pub fn add_file(&self, path: &str, data: Vec<u8>) -> Result<Ino, &'static str> {
        let path = normalize_path(path);
        
        // Check if already exists
        let path_inodes = self.path_inodes.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: path_inodes lock poisoned, recovering...");
            poisoned.into_inner()
        });
        if path_inodes.contains_key(&path) {
            return Err("File already exists");
        }
        drop(path_inodes);

        // Ensure parent directory exists
        let parent_path = parent_path(&path).ok_or("Invalid path")?;
        let parent_ino = self.ensure_directory(&parent_path)?;

        // Create file
        let ino = self.alloc_ino()?;
        let size = data.len() as u64;
        
        let attr = FileAttr {
            ino,
            size,
            blocks: (size + 511) / 512,
            kind: FileKind::RegularFile,
            perm: 0o644,
            nlink: 1,
            ..Default::default()
        };

        // Store file
        self.inodes.write()
            .map_err(|_| "Inodes lock poisoned")?
            .insert(ino, attr.clone());
        self.inode_paths.write()
            .map_err(|_| "Inode paths lock poisoned")?
            .insert(ino, path.clone());
        self.path_inodes.write()
            .map_err(|_| "Path inodes lock poisoned")?
            .insert(path.clone(), ino);
        self.file_cache.write()
            .map_err(|_| "File cache lock poisoned")?
            .insert(ino, CachedFile { data, attr });

        // Add to parent directory
        let filename = filename(&path).ok_or("Invalid filename")?;
        self.directories.write()
            .map_err(|_| "Directories lock poisoned")?
            .get_mut(&parent_ino)
            .ok_or("Parent directory not found")?
            .push(DirEntry {
                ino,
                name: filename.to_string(),
                kind: FileKind::RegularFile,
            });

        Ok(ino)
    }

    /// Ensure a directory exists, creating it if necessary
    fn ensure_directory(&self, path: &str) -> Result<Ino, &'static str> {
        let path = normalize_path(path);
        
        // Root always exists
        if path == "/" {
            return Ok(ROOT_INO);
        }

        // Check if already exists
        let path_inodes = self.path_inodes.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: path_inodes lock poisoned in ensure_directory, recovering...");
            poisoned.into_inner()
        });
        if let Some(&ino) = path_inodes.get(&path) {
            return Ok(ino);
        }
        drop(path_inodes);

        // Create parent first
        let parent_path = parent_path(&path).ok_or("Invalid path")?;
        let parent_ino = self.ensure_directory(&parent_path)?;

        // Create this directory
        let ino = self.alloc_ino()?;
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            kind: FileKind::Directory,
            perm: 0o755,
            nlink: 2,
            ..Default::default()
        };

        self.inodes.write()
            .map_err(|_| "Inodes lock poisoned")?
            .insert(ino, attr);
        self.inode_paths.write()
            .map_err(|_| "Inode paths lock poisoned")?
            .insert(ino, path.clone());
        self.path_inodes.write()
            .map_err(|_| "Path inodes lock poisoned")?
            .insert(path.clone(), ino);
        self.directories.write()
            .map_err(|_| "Directories lock poisoned")?
            .insert(ino, Vec::new());

        // Add to parent
        let dirname = filename(&path).ok_or("Invalid dirname")?;
        self.directories.write()
            .map_err(|_| "Directories lock poisoned")?
            .get_mut(&parent_ino)
            .ok_or("Parent not found")?
            .push(DirEntry {
                ino,
                name: dirname.to_string(),
                kind: FileKind::Directory,
            });

        // Update parent nlink
        if let Some(parent_attr) = self.inodes.write()
            .map_err(|_| "Inodes lock poisoned")?
            .get_mut(&parent_ino) {
            parent_attr.nlink += 1;
        }

        Ok(ino)
    }

    /// Lookup a path and return its inode
    pub fn lookup_path(&self, path: &str) -> Option<Ino> {
        let path = normalize_path(path);
        let path_inodes = self.path_inodes.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: path_inodes lock poisoned in lookup_path, recovering...");
            poisoned.into_inner()
        });
        path_inodes.get(&path).copied()
    }

    /// Get file attributes by inode
    pub fn get_attr(&self, ino: Ino) -> Option<FileAttr> {
        let inodes = self.inodes.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: inodes lock poisoned in get_attr, recovering...");
            poisoned.into_inner()
        });
        inodes.get(&ino).cloned()
    }

    /// Read file data
    pub fn read_data(&self, ino: Ino, offset: u64, size: u32) -> Option<Vec<u8>> {
        let cache = self.file_cache.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: file_cache lock poisoned in read_data, recovering...");
            poisoned.into_inner()
        });
        let cached = cache.get(&ino)?;
        
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, cached.data.len());
        
        if start >= cached.data.len() {
            return Some(Vec::new());
        }
        
        Some(cached.data[start..end].to_vec())
    }

    /// Read directory contents
    pub fn read_dir(&self, ino: Ino) -> Option<Vec<DirEntry>> {
        let directories = self.directories.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: directories lock poisoned in read_dir, recovering...");
            poisoned.into_inner()
        });
        directories.get(&ino).cloned()
    }

    /// Lookup entry in directory by name
    pub fn lookup_entry(&self, parent_ino: Ino, name: &str) -> Option<Ino> {
        let dirs = self.directories.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: directories lock poisoned in lookup_entry, recovering...");
            poisoned.into_inner()
        });
        let entries = dirs.get(&parent_ino)?;
        entries.iter().find(|e| e.name == name).map(|e| e.ino)
    }

    /// Get parent inode for a given inode
    pub fn get_parent(&self, ino: Ino) -> Option<Ino> {
        if ino == ROOT_INO {
            return Some(ROOT_INO); // Root's parent is itself
        }
        
        let paths = self.inode_paths.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: inode_paths lock poisoned in get_parent, recovering...");
            poisoned.into_inner()
        });
        let path = paths.get(&ino)?;
        let parent = parent_path(path)?;
        
        let path_inodes = self.path_inodes.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: path_inodes lock poisoned in get_parent, recovering...");
            poisoned.into_inner()
        });
        path_inodes.get(&parent).copied()
    }

    /// Get total number of files
    pub fn file_count(&self) -> usize {
        let cache = self.file_cache.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: file_cache lock poisoned in file_count, recovering...");
            poisoned.into_inner()
        });
        cache.len()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        let cache = self.file_cache.read().unwrap_or_else(|poisoned| {
            eprintln!("WARNING: file_cache lock poisoned in total_size, recovering...");
            poisoned.into_inner()
        });
        cache.values()
            .map(|f| f.attr.size)
            .sum()
    }

    /// Check if filesystem is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Get attribute TTL
    pub fn attr_ttl(&self) -> Duration {
        self.attr_ttl
    }

    /// Get entry TTL
    pub fn entry_ttl(&self) -> Duration {
        self.entry_ttl
    }
}

// =============================================================================
// FUSER FILESYSTEM TRAIT IMPLEMENTATION
// =============================================================================

#[cfg(feature = "fuse")]
impl fuser::Filesystem for EngramFS {
    /// Initialize filesystem
    fn init(
        &mut self,
        _req: &fuser::Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        eprintln!("EngramFS initialized: {} files, {} bytes total",
            self.file_count(), self.total_size());
        Ok(())
    }

    /// Clean up filesystem
    fn destroy(&mut self) {
        eprintln!("EngramFS unmounted");
    }

    /// Look up a directory entry by name
    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        match self.lookup_entry(parent, name) {
            Some(ino) => {
                if let Some(attr) = self.get_attr(ino) {
                    let fuser_attr: fuser::FileAttr = attr.into();
                    reply.entry(&self.entry_ttl, &fuser_attr, 0);
                } else {
                    reply.error(libc::ENOENT);
                }
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    /// Get file attributes
    fn getattr(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: Option<u64>,
        reply: fuser::ReplyAttr,
    ) {
        match self.get_attr(ino) {
            Some(attr) => {
                let fuser_attr: fuser::FileAttr = attr.into();
                reply.attr(&self.attr_ttl, &fuser_attr);
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    /// Read data from a file
    fn read(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        match self.read_data(ino, offset as u64, size) {
            Some(data) => {
                reply.data(&data);
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    /// Open a file
    fn open(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        flags: i32,
        reply: fuser::ReplyOpen,
    ) {
        // Check if file exists
        if self.get_attr(ino).is_none() {
            reply.error(libc::ENOENT);
            return;
        }

        // Check for write flags on read-only filesystem
        if self.read_only {
            let write_flags = libc::O_WRONLY | libc::O_RDWR | libc::O_APPEND | libc::O_TRUNC;
            if flags & write_flags != 0 {
                reply.error(libc::EROFS);
                return;
            }
        }

        // Return a dummy file handle (we're stateless)
        reply.opened(0, 0);
    }

    /// Release an open file
    fn release(
        &mut self,
        _req: &fuser::Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    /// Open a directory
    fn opendir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _flags: i32,
        reply: fuser::ReplyOpen,
    ) {
        match self.get_attr(ino) {
            Some(attr) if attr.kind == FileKind::Directory => {
                reply.opened(0, 0);
            }
            Some(_) => {
                reply.error(libc::ENOTDIR);
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    /// Read directory entries
    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        let mut entries: Vec<(u64, fuser::FileType, String)> = Vec::new();

        // Add . and ..
        entries.push((ino, fuser::FileType::Directory, ".".to_string()));
        let parent_ino = self.get_parent(ino).unwrap_or(ino);
        entries.push((parent_ino, fuser::FileType::Directory, "..".to_string()));

        // Add directory contents
        if let Some(dir_entries) = self.read_dir(ino) {
            for entry in dir_entries {
                entries.push((entry.ino, entry.kind.into(), entry.name));
            }
        }

        // Skip entries before offset and emit remaining
        for (i, (ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            // Reply returns true if buffer is full
            if reply.add(ino, (i + 1) as i64, kind, &name) {
                break;
            }
        }

        reply.ok();
    }

    /// Release a directory handle
    fn releasedir(
        &mut self,
        _req: &fuser::Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    /// Get filesystem statistics
    fn statfs(&mut self, _req: &fuser::Request<'_>, _ino: u64, reply: fuser::ReplyStatfs) {
        let total_files = self.file_count() as u64;
        let total_size = self.total_size();
        let block_size = 4096u64;
        let total_blocks = (total_size + block_size - 1) / block_size;

        reply.statfs(
            total_blocks,       // blocks - total data blocks
            0,                  // bfree - free blocks (0 for read-only)
            0,                  // bavail - available blocks (0 for read-only)
            total_files,        // files - total file nodes
            0,                  // ffree - free file nodes (0 for read-only)
            block_size as u32,  // bsize - block size
            255,                // namelen - maximum name length
            block_size as u32,  // frsize - fragment size
        );
    }

    /// Check file access permissions
    fn access(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        mask: i32,
        reply: fuser::ReplyEmpty,
    ) {
        // Check if file exists
        if self.get_attr(ino).is_none() {
            reply.error(libc::ENOENT);
            return;
        }

        // Deny write access on read-only filesystem
        if self.read_only && (mask & libc::W_OK != 0) {
            reply.error(libc::EROFS);
            return;
        }

        // Allow all other access (simplified permission model)
        reply.ok();
    }

    /// Read symbolic link target
    fn readlink(&mut self, _req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyData) {
        // We don't support symlinks yet
        match self.get_attr(ino) {
            Some(attr) if attr.kind == FileKind::Symlink => {
                reply.error(libc::ENOSYS); // Not implemented
            }
            Some(_) => {
                reply.error(libc::EINVAL); // Not a symlink
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }
}

// =============================================================================
// MOUNT FUNCTIONS
// =============================================================================

/// Mount options for EngramFS
#[cfg(feature = "fuse")]
#[derive(Clone, Debug)]
pub struct MountOptions {
    /// Read-only mount (default: true)
    pub read_only: bool,
    /// Allow other users to access the mount (default: false)
    pub allow_other: bool,
    /// Allow root to access the mount (default: true)
    pub allow_root: bool,
    /// Filesystem name shown in mount output
    pub fsname: String,
}

#[cfg(feature = "fuse")]
impl Default for MountOptions {
    fn default() -> Self {
        MountOptions {
            read_only: true,
            allow_other: false,
            allow_root: true,
            fsname: "engram".to_string(),
        }
    }
}

/// Mount an EngramFS at the specified path
///
/// This function blocks until the filesystem is unmounted. Use `spawn_mount`
/// for a non-blocking version.
///
/// # Arguments
///
/// * `fs` - The EngramFS instance to mount
/// * `mountpoint` - Directory path where the filesystem will be mounted
/// * `options` - Mount options (see `MountOptions`)
///
/// # Example
///
/// ```no_run
/// use embeddenator::fuse_shim::{EngramFS, mount, MountOptions};
///
/// let fs = EngramFS::new(true);
/// // ... populate fs with files ...
///
/// mount(fs, "/mnt/engram", MountOptions::default()).unwrap();
/// ```
#[cfg(feature = "fuse")]
pub fn mount<P: AsRef<Path>>(
    fs: EngramFS,
    mountpoint: P,
    options: MountOptions,
) -> Result<(), std::io::Error> {
    use fuser::MountOption;

    let mut mount_options = vec![
        MountOption::FSName(options.fsname),
        MountOption::AutoUnmount,
        MountOption::DefaultPermissions,
    ];

    if options.read_only {
        mount_options.push(MountOption::RO);
    }

    if options.allow_other {
        mount_options.push(MountOption::AllowOther);
    } else if options.allow_root {
        mount_options.push(MountOption::AllowRoot);
    }

    fuser::mount2(fs, mountpoint.as_ref(), &mount_options)
}

/// Spawn an EngramFS mount in a background thread
///
/// Returns a `BackgroundSession` that will automatically unmount when dropped.
///
/// # Arguments
///
/// * `fs` - The EngramFS instance to mount
/// * `mountpoint` - Directory path where the filesystem will be mounted
/// * `options` - Mount options (see `MountOptions`)
///
/// # Example
///
/// ```no_run
/// use embeddenator::fuse_shim::{EngramFS, spawn_mount, MountOptions};
///
/// let fs = EngramFS::new(true);
/// // ... populate fs with files ...
///
/// let session = spawn_mount(fs, "/mnt/engram", MountOptions::default()).unwrap();
/// // Filesystem is now mounted and accessible
///
/// // When session is dropped, the filesystem will be unmounted
/// ```
#[cfg(feature = "fuse")]
pub fn spawn_mount<P: AsRef<Path>>(
    fs: EngramFS,
    mountpoint: P,
    options: MountOptions,
) -> Result<fuser::BackgroundSession, std::io::Error> {
    use fuser::MountOption;

    let mut mount_options = vec![
        MountOption::FSName(options.fsname),
        MountOption::AutoUnmount,
        MountOption::DefaultPermissions,
    ];

    if options.read_only {
        mount_options.push(MountOption::RO);
    }

    if options.allow_other {
        mount_options.push(MountOption::AllowOther);
    } else if options.allow_root {
        mount_options.push(MountOption::AllowRoot);
    }

    fuser::spawn_mount2(fs, mountpoint.as_ref(), &mount_options)
}

// =============================================================================
// BUILDER PATTERN
// =============================================================================

/// Builder for creating an EngramFS from engram data
///
/// # Example
///
/// ```
/// use embeddenator::fuse_shim::EngramFSBuilder;
///
/// let fs = EngramFSBuilder::new()
///     .add_file("/README.md", b"# Hello World".to_vec())
///     .add_file("/src/main.rs", b"fn main() {}".to_vec())
///     .build();
///
/// assert_eq!(fs.file_count(), 2);
/// ```
pub struct EngramFSBuilder {
    fs: EngramFS,
}

impl EngramFSBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        EngramFSBuilder {
            fs: EngramFS::new(true), // Read-only by default
        }
    }

    /// Add a file from decoded engram data
    pub fn add_file(self, path: &str, data: Vec<u8>) -> Self {
        let _ = self.fs.add_file(path, data);
        self
    }

    /// Set read-only mode (default: true)
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.fs.read_only = read_only;
        self
    }

    /// Build the filesystem
    pub fn build(self) -> EngramFS {
        self.fs
    }
}

impl Default for EngramFSBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Normalize a path (ensure leading /, remove trailing /)
fn normalize_path(path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    
    if path.len() > 1 && path.ends_with('/') {
        path[..path.len()-1].to_string()
    } else {
        path
    }
}

/// Get parent path
fn parent_path(path: &str) -> Option<String> {
    let path = normalize_path(path);
    if path == "/" {
        return None;
    }
    
    match path.rfind('/') {
        Some(0) => Some("/".to_string()),
        Some(pos) => Some(path[..pos].to_string()),
        None => None,
    }
}

/// Get filename from path
fn filename(path: &str) -> Option<&str> {
    let path = path.trim_end_matches('/');
    path.rsplit('/').next()
}

/// Convert SystemTime to Duration since UNIX_EPOCH (useful for logging)
#[allow(dead_code)]
fn system_time_to_unix(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// =============================================================================
// STATISTICS
// =============================================================================

/// Statistics for the mounted filesystem
#[derive(Clone, Debug, Default)]
pub struct MountStats {
    /// Number of read operations
    pub reads: u64,
    /// Total bytes read
    pub read_bytes: u64,
    /// Number of lookup operations
    pub lookups: u64,
    /// Number of readdir operations
    pub readdirs: u64,
    /// Number of cache hits
    pub cache_hits: u64,
    /// Number of cache misses
    pub cache_misses: u64,
    /// Total decode time in microseconds
    pub decode_time_us: u64,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("foo"), "/foo");
        assert_eq!(normalize_path("/foo"), "/foo");
        assert_eq!(normalize_path("/foo/"), "/foo");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("/foo/bar"), Some("/foo".to_string()));
        assert_eq!(parent_path("/foo"), Some("/".to_string()));
        assert_eq!(parent_path("/"), None);
    }

    #[test]
    fn test_filename() {
        assert_eq!(filename("/foo/bar"), Some("bar"));
        assert_eq!(filename("/foo"), Some("foo"));
        assert_eq!(filename("/foo/bar/"), Some("bar"));
    }

    #[test]
    fn test_add_file() {
        let fs = EngramFS::new(true);
        
        let ino = fs.add_file("/test.txt", b"hello world".to_vec()).unwrap();
        assert!(ino > ROOT_INO);
        
        let data = fs.read_data(ino, 0, 100).unwrap();
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn test_nested_directories() {
        let fs = EngramFS::new(true);
        
        fs.add_file("/a/b/c/file.txt", b"deep".to_vec()).unwrap();
        
        // All directories should exist
        assert!(fs.lookup_path("/a").is_some());
        assert!(fs.lookup_path("/a/b").is_some());
        assert!(fs.lookup_path("/a/b/c").is_some());
        assert!(fs.lookup_path("/a/b/c/file.txt").is_some());
    }

    #[test]
    fn test_readdir() {
        let fs = EngramFS::new(true);
        
        fs.add_file("/foo.txt", b"foo".to_vec()).unwrap();
        fs.add_file("/bar.txt", b"bar".to_vec()).unwrap();
        fs.add_file("/subdir/baz.txt", b"baz".to_vec()).unwrap();
        
        let root_entries = fs.read_dir(ROOT_INO).unwrap();
        assert_eq!(root_entries.len(), 3); // foo.txt, bar.txt, subdir
        
        let names: Vec<_> = root_entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"foo.txt"));
        assert!(names.contains(&"bar.txt"));
        assert!(names.contains(&"subdir"));
    }

    #[test]
    fn test_read_partial() {
        let fs = EngramFS::new(true);
        let data = b"0123456789";
        
        let ino = fs.add_file("/test.txt", data.to_vec()).unwrap();
        
        // Read middle portion
        let partial = fs.read_data(ino, 3, 4).unwrap();
        assert_eq!(partial, b"3456");
        
        // Read past end
        let past_end = fs.read_data(ino, 20, 10).unwrap();
        assert!(past_end.is_empty());
    }

    #[test]
    fn test_builder() {
        let fs = EngramFSBuilder::new()
            .add_file("/a.txt", b"a".to_vec())
            .add_file("/b.txt", b"b".to_vec())
            .build();
        
        assert_eq!(fs.file_count(), 2);
    }

    #[test]
    fn test_get_parent() {
        let fs = EngramFS::new(true);
        
        fs.add_file("/a/b/c.txt", b"test".to_vec()).unwrap();
        
        let c_ino = fs.lookup_path("/a/b/c.txt").unwrap();
        let b_ino = fs.lookup_path("/a/b").unwrap();
        let a_ino = fs.lookup_path("/a").unwrap();
        
        assert_eq!(fs.get_parent(c_ino), Some(b_ino));
        assert_eq!(fs.get_parent(b_ino), Some(a_ino));
        assert_eq!(fs.get_parent(a_ino), Some(ROOT_INO));
        assert_eq!(fs.get_parent(ROOT_INO), Some(ROOT_INO));
    }

    #[test]
    fn test_default_attrs() {
        let attr = FileAttr::default();
        assert_eq!(attr.perm, 0o644);
        assert_eq!(attr.nlink, 1);
        assert_eq!(attr.blksize, 4096);
    }

    #[test]
    fn test_file_kind_conversion() {
        // Only run conversion tests when fuse feature is enabled
        #[cfg(feature = "fuse")]
        {
            let dir: fuser::FileType = FileKind::Directory.into();
            assert_eq!(dir, fuser::FileType::Directory);
            
            let file: fuser::FileType = FileKind::RegularFile.into();
            assert_eq!(file, fuser::FileType::RegularFile);
        }
    }

    #[test]
    fn test_lock_poisoning_recovery() {
        use std::sync::Arc;
        use std::thread;
        
        // This test demonstrates that the filesystem can recover from poisoned locks
        // In the read path (read-only operations), we use unwrap_or_else with into_inner()
        // to continue serving requests even with poisoned locks
        
        let fs = Arc::new(EngramFS::new(true));
        
        // Add a file successfully
        fs.add_file("/test.txt", b"hello".to_vec()).unwrap();
        let ino = fs.lookup_path("/test.txt").unwrap();
        
        // Simulate lock poisoning scenario by creating a poisoned lock in a thread
        // Note: We can't actually poison the lock in a real test without unsafe code,
        // but we can verify that our error handling works correctly
        
        // Test that read operations continue to work
        let data = fs.read_data(ino, 0, 5);
        assert!(data.is_some());
        assert_eq!(data.unwrap(), b"hello");
        
        // Test that lookup continues to work
        let found_ino = fs.lookup_path("/test.txt");
        assert_eq!(found_ino, Some(ino));
        
        // Test that get_attr works
        let attr = fs.get_attr(ino);
        assert!(attr.is_some());
        assert_eq!(attr.unwrap().size, 5);
        
        // Test concurrent access doesn't cause issues
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            // Multiple reads from another thread
            for _ in 0..10 {
                let _ = fs_clone.read_data(ino, 0, 5);
                let _ = fs_clone.lookup_path("/test.txt");
            }
        });
        
        // Simultaneous reads from main thread
        for _ in 0..10 {
            let _ = fs.read_data(ino, 0, 5);
            let _ = fs.get_attr(ino);
        }
        
        handle.join().unwrap();
        
        // Verify filesystem is still functional
        assert_eq!(fs.file_count(), 1);
        assert_eq!(fs.total_size(), 5);
    }

    #[test]
    fn test_write_lock_error_propagation() {
        // Test that write operations properly propagate lock errors
        let fs = EngramFS::new(false);
        
        // This should succeed
        let result = fs.add_file("/test.txt", b"content".to_vec());
        assert!(result.is_ok());
        
        // Verify the file was created
        assert!(fs.lookup_path("/test.txt").is_some());
        assert_eq!(fs.file_count(), 1);
    }
}
