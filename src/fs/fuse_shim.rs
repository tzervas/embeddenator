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

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_swap::ArcSwap;
use rustc_hash::FxHashMap;

use crate::embrfs::Engram;
use crate::vsa::ReversibleVSAConfig;

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

#[derive(Clone, Debug)]
struct BackedFile {
    path: String,
    chunks: Vec<usize>,
    size: usize,
}

#[derive(Clone, Debug)]
enum FileStorage {
    /// File bytes are already present in-memory (used by the builder/tests).
    Preloaded(Vec<u8>),
    /// File is backed by an engram and should be decoded on-demand.
    Backed(BackedFile),
}

#[derive(Clone, Debug)]
struct FileRecord {
    storage: FileStorage,
    attr: FileAttr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ChunkKey {
    ino: Ino,
    chunk_id: u64,
}

struct ChunkCache {
    map: FxHashMap<ChunkKey, Vec<u8>>,
    order: VecDeque<ChunkKey>,
    total_bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}

impl ChunkCache {
    fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            map: FxHashMap::default(),
            order: VecDeque::new(),
            total_bytes: 0,
            max_entries,
            max_bytes,
        }
    }

    fn get(&mut self, key: ChunkKey) -> Option<&[u8]> {
        if self.map.contains_key(&key) {
            // touch
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
        }
        self.map.get(&key).map(|v: &Vec<u8>| v.as_slice())
    }

    fn insert(&mut self, key: ChunkKey, value: Vec<u8>) {
        if self.max_entries == 0 || self.max_bytes == 0 {
            return;
        }

        let value_len = value.len();
        if value_len > self.max_bytes {
            // Don't cache single entries bigger than the entire cache.
            return;
        }

        if let Some(existing) = self.map.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(existing.len());
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
        }

        self.total_bytes += value_len;
        self.map.insert(key, value);
        self.order.push_back(key);

        while self.map.len() > self.max_entries || self.total_bytes > self.max_bytes {
            let Some(evict) = self.order.pop_front() else { break };
            if let Some(v) = self.map.remove(&evict) {
                self.total_bytes = self.total_bytes.saturating_sub(v.len());
            }
        }
    }
}

/// The EngramFS FUSE filesystem implementation
///
/// This provides a read-only view of decoded engram data as a standard
/// POSIX filesystem. Files are decoded on-demand from the holographic
/// representation and cached for efficient repeated access.
///
/// # Concurrency Model
///
/// Uses lock-free reads via `ArcSwap` for metadata (read-heavy, write-rare):
/// - `inodes`, `inode_paths`, `path_inodes`, `directories`, `files`: Lock-free reads via atomic swap
/// - `next_ino`: Lock-free increment via `AtomicU64`
/// - `chunk_cache`: `RwLock` with read-then-write pattern for cache access
///
/// This eliminates read-lock contention in the hot path (FUSE operations).
/// 
/// # Performance Notes
///
/// Uses `FxHashMap` (rustc-hash) for faster hashing on string keys.
/// Hot-path reads use `ArcSwap::load()` which is ~20ns on modern CPUs.
pub struct EngramFS {
    /// Inode to file attributes mapping (lock-free reads)
    inodes: ArcSwap<FxHashMap<Ino, FileAttr>>,
    
    /// Inode to path mapping (lock-free reads)
    inode_paths: ArcSwap<FxHashMap<Ino, String>>,
    
    /// Path to inode mapping (lock-free reads)
    path_inodes: ArcSwap<FxHashMap<String, Ino>>,
    
    /// Directory contents (parent_ino -> entries) (lock-free reads)
    directories: ArcSwap<FxHashMap<Ino, Vec<DirEntry>>>,
    
    /// File records (ino -> backing/preloaded bytes + attrs) (lock-free reads)
    files: ArcSwap<FxHashMap<Ino, FileRecord>>,

    /// Optional engram backing for on-demand decode.
    engram: Option<Arc<Engram>>,

    /// Decode config used for on-demand reads.
    decode_config: Option<ReversibleVSAConfig>,

    /// Chunk size used for decode.
    chunk_size: usize,

    /// Small LRU chunk cache to avoid repeated decode on hot reads.
    /// Uses RwLock because LRU cache mutates on read (access order).
    chunk_cache: Arc<RwLock<ChunkCache>>,
    
    /// Next available inode number (lock-free increment)
    next_ino: AtomicU64,
    
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
            inodes: ArcSwap::from_pointee(FxHashMap::default()),
            inode_paths: ArcSwap::from_pointee(FxHashMap::default()),
            path_inodes: ArcSwap::from_pointee(FxHashMap::default()),
            directories: ArcSwap::from_pointee(FxHashMap::default()),
            files: ArcSwap::from_pointee(FxHashMap::default()),
            next_ino: AtomicU64::new(2), // Start after root
            read_only,
            attr_ttl: Duration::from_secs(1),
            entry_ttl: Duration::from_secs(1),

            engram: None,
            decode_config: None,
            chunk_size: 4096,
            // Default: keep this small and bounded for production safety.
            chunk_cache: Arc::new(RwLock::new(ChunkCache::new(16_384, 64 * 1024 * 1024))),
        };

        // Initialize root directory
        fs.init_root();
        fs
    }

    /// Construct an EngramFS backed by an engram+manifest.
    ///
    /// This is the production mount path: we populate directory structure and
    /// file metadata only, and decode chunk data on-demand during reads.
    pub fn from_engram(
        engram: Engram,
        manifest: crate::embrfs::Manifest,
        decode_config: ReversibleVSAConfig,
        chunk_size: usize,
        read_only: bool,
    ) -> Self {
        let mut fs = Self::new(read_only);
        fs.engram = Some(Arc::new(engram));
        fs.decode_config = Some(decode_config);
        fs.chunk_size = chunk_size;

        for file_entry in &manifest.files {
            let _ = fs.add_backed_file(&file_entry.path, file_entry.chunks.clone(), file_entry.size);
        }

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

        // Use rcu_modify for atomic copy-on-write updates
        self.inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ROOT_INO, root_attr.clone());
            new_map
        });
        self.inode_paths.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ROOT_INO, "/".to_string());
            new_map
        });
        self.path_inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert("/".to_string(), ROOT_INO);
            new_map
        });
        self.directories.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ROOT_INO, Vec::new());
            new_map
        });
    }

    /// Allocate a new inode number (lock-free)
    fn alloc_ino(&self) -> Ino {
        self.next_ino.fetch_add(1, Ordering::SeqCst)
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
        
        // Check if already exists - lock-free read
        if self.path_inodes.load().contains_key(&path) {
            return Err("File already exists");
        }

        // Ensure parent directory exists
        let parent_path = parent_path(&path).ok_or("Invalid path")?;
        let parent_ino = self.ensure_directory(&parent_path)?;

        // Create file
        let ino = self.alloc_ino();
        let size = data.len() as u64;
        
        let attr = FileAttr {
            ino,
            size,
            blocks: size.div_ceil(512),
            kind: FileKind::RegularFile,
            perm: 0o644,
            nlink: 1,
            ..Default::default()
        };

        // Store file metadata using copy-on-write
        self.inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, attr.clone());
            new_map
        });
        self.inode_paths.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, path.clone());
            new_map
        });
        self.path_inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(path.clone(), ino);
            new_map
        });
        self.files.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(
                ino,
                FileRecord {
                    storage: FileStorage::Preloaded(data.clone()),
                    attr: attr.clone(),
                },
            );
            new_map
        });

        // Add to parent directory
        let filename = filename(&path).ok_or("Invalid filename")?;
        self.directories.rcu(|map| {
            let mut new_map = (**map).clone();
            if let Some(entries) = new_map.get_mut(&parent_ino) {
                entries.push(DirEntry {
                    ino,
                    name: filename.to_string(),
                    kind: FileKind::RegularFile,
                });
            }
            new_map
        });

        Ok(ino)
    }

    /// Add a file whose bytes are backed by an engram and decoded on-demand.
    pub fn add_backed_file(&self, path: &str, chunks: Vec<usize>, size: usize) -> Result<Ino, &'static str> {
        let path = normalize_path(path);

        // Lock-free existence check
        if self.path_inodes.load().contains_key(&path) {
            return Err("File already exists");
        }

        let parent_path = parent_path(&path).ok_or("Invalid path")?;
        let parent_ino = self.ensure_directory(&parent_path)?;

        let ino = self.alloc_ino();
        let size_u64 = size as u64;

        let attr = FileAttr {
            ino,
            size: size_u64,
            blocks: size_u64.div_ceil(512),
            kind: FileKind::RegularFile,
            perm: 0o644,
            nlink: 1,
            ..Default::default()
        };

        // Copy-on-write updates
        self.inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, attr.clone());
            new_map
        });
        self.inode_paths.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, path.clone());
            new_map
        });
        self.path_inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(path.clone(), ino);
            new_map
        });
        self.files.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(
                ino,
                FileRecord {
                    storage: FileStorage::Backed(BackedFile { path: path.clone(), chunks: chunks.clone(), size }),
                    attr: attr.clone(),
                },
            );
            new_map
        });

        let filename = filename(&path).ok_or("Invalid filename")?;
        self.directories.rcu(|map| {
            let mut new_map = (**map).clone();
            if let Some(entries) = new_map.get_mut(&parent_ino) {
                entries.push(DirEntry {
                    ino,
                    name: filename.to_string(),
                    kind: FileKind::RegularFile,
                });
            }
            new_map
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

        // Lock-free existence check
        if let Some(&ino) = self.path_inodes.load().get(&path) {
            return Ok(ino);
        }

        // Create parent first (recursive)
        let parent_path = parent_path(&path).ok_or("Invalid path")?;
        let parent_ino = self.ensure_directory(&parent_path)?;

        // Create this directory
        let ino = self.alloc_ino();
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            kind: FileKind::Directory,
            perm: 0o755,
            nlink: 2,
            ..Default::default()
        };

        // Copy-on-write updates
        self.inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, attr.clone());
            new_map
        });
        self.inode_paths.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, path.clone());
            new_map
        });
        self.path_inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(path.clone(), ino);
            new_map
        });
        self.directories.rcu(|map| {
            let mut new_map = (**map).clone();
            new_map.insert(ino, Vec::new());
            new_map
        });

        // Add to parent
        let dirname = filename(&path).ok_or("Invalid dirname")?;
        self.directories.rcu(|map| {
            let mut new_map = (**map).clone();
            if let Some(entries) = new_map.get_mut(&parent_ino) {
                entries.push(DirEntry {
                    ino,
                    name: dirname.to_string(),
                    kind: FileKind::Directory,
                });
            }
            new_map
        });

        // Update parent nlink
        self.inodes.rcu(|map| {
            let mut new_map = (**map).clone();
            if let Some(parent_attr) = new_map.get_mut(&parent_ino) {
                parent_attr.nlink += 1;
            }
            new_map
        });

        Ok(ino)
    }

    /// Lookup a path and return its inode (lock-free)
    #[inline]
    pub fn lookup_path(&self, path: &str) -> Option<Ino> {
        let path = normalize_path(path);
        self.path_inodes.load().get(&path).copied()
    }

    /// Get file attributes by inode (lock-free)
    #[inline]
    pub fn get_attr(&self, ino: Ino) -> Option<FileAttr> {
        self.inodes.load().get(&ino).cloned()
    }

    /// Read file data (lock-free for metadata lookup)
    #[inline]
    pub fn read_data(&self, ino: Ino, offset: u64, size: u32) -> Option<Vec<u8>> {
        if size == 0 {
            return Some(Vec::new());
        }

        let offset_usize = match usize::try_from(offset) {
            Ok(v) => v,
            Err(_) => return Some(Vec::new()),
        };

        // Lock-free load of file records
        let files = self.files.load();
        let rec = files.get(&ino)?;

        match &rec.storage {
            FileStorage::Preloaded(data) => {
                if offset_usize >= data.len() {
                    return Some(Vec::new());
                }
                let end = std::cmp::min(offset_usize.saturating_add(size as usize), data.len());
                Some(data[offset_usize..end].to_vec())
            }
            FileStorage::Backed(backed) => {
                let max_len = backed.size;
                if offset_usize >= max_len {
                    return Some(Vec::new());
                }
                let end = std::cmp::min(offset_usize.saturating_add(size as usize), max_len);
                Some(self.read_backed_range(ino, backed, offset_usize, end))
            }
        }
    }

    fn read_backed_range(&self, ino: Ino, backed: &BackedFile, start: usize, end: usize) -> Vec<u8> {
        if start >= end {
            return Vec::new();
        }

        let Some(engram) = self.engram.as_ref() else {
            return Vec::new();
        };
        let Some(cfg) = self.decode_config.as_ref() else {
            return Vec::new();
        };

        let chunk_size = self.chunk_size;
        if chunk_size == 0 {
            return Vec::new();
        }

        let start_chunk = start / chunk_size;
        let end_chunk = (end - 1) / chunk_size;
        if start_chunk >= backed.chunks.len() {
            return Vec::new();
        }

        let mut out = Vec::with_capacity(end - start);
        let last_chunk = end_chunk.min(backed.chunks.len().saturating_sub(1));

        for chunk_index in start_chunk..=last_chunk {
            let chunk_id = backed.chunks[chunk_index] as u64;
            let key = ChunkKey { ino, chunk_id };

            // Try cache first.
            if let Ok(mut cache) = self.chunk_cache.write() {
                if let Some(bytes) = cache.get(key) {
                    let (a, b) = slice_chunk_bounds(start, end, chunk_index, chunk_size);
                    if a < b && b <= bytes.len() {
                        out.extend_from_slice(&bytes[a..b]);
                        continue;
                    }
                }
            }

            // Decode chunk.
            let Some(chunk_vec) = engram.codebook.get(&(chunk_id as usize)) else {
                continue;
            };
            let decoded = chunk_vec.decode_data(cfg, Some(&backed.path), chunk_size);
            let chunk_bytes = if let Some(corrected) = engram.corrections.apply(chunk_id, &decoded) {
                corrected
            } else {
                decoded
            };

            // Cache decoded chunk (best-effort).
            if let Ok(mut cache) = self.chunk_cache.write() {
                cache.insert(key, chunk_bytes.clone());
            }

            let (a, b) = slice_chunk_bounds(start, end, chunk_index, chunk_size);
            if a < b && b <= chunk_bytes.len() {
                out.extend_from_slice(&chunk_bytes[a..b]);
            }
        }

        out
    }

    /// Read directory contents (lock-free)
    pub fn read_dir(&self, ino: Ino) -> Option<Vec<DirEntry>> {
        self.directories.load().get(&ino).cloned()
    }

    /// Lookup entry in directory by name (lock-free)
    pub fn lookup_entry(&self, parent_ino: Ino, name: &str) -> Option<Ino> {
        let dirs = self.directories.load();
        let entries = dirs.get(&parent_ino)?;
        entries.iter().find(|e| e.name == name).map(|e| e.ino)
    }

    /// Get parent inode for a given inode (lock-free)
    pub fn get_parent(&self, ino: Ino) -> Option<Ino> {
        if ino == ROOT_INO {
            return Some(ROOT_INO); // Root's parent is itself
        }
        
        let paths = self.inode_paths.load();
        let path = paths.get(&ino)?;
        let parent = parent_path(path)?;
        
        self.path_inodes.load().get(&parent).copied()
    }

    /// Get total number of files (lock-free)
    pub fn file_count(&self) -> usize {
        self.files.load().len()
    }

    /// Get total size of all files (lock-free)
    pub fn total_size(&self) -> u64 {
        self.files.load().values().map(|f| f.attr.size).sum()
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
        match self.get_attr(parent) {
            Some(attr) if attr.kind == FileKind::Directory => {}
            Some(_) => {
                reply.error(libc::ENOTDIR);
                return;
            }
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        }

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
        if offset < 0 {
            reply.error(libc::EINVAL);
            return;
        }

        match self.get_attr(ino) {
            Some(attr) if attr.kind == FileKind::Directory => {
                reply.error(libc::EISDIR);
                return;
            }
            Some(_) => {}
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        }

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
        // Check if file exists and is a file.
        match self.get_attr(ino) {
            Some(attr) if attr.kind == FileKind::Directory => {
                reply.error(libc::EISDIR);
                return;
            }
            Some(_) => {}
            None => {
                reply.error(libc::ENOENT);
                return;
            }
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
        if offset < 0 {
            reply.error(libc::EINVAL);
            return;
        }

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
        for (i, (ino, kind, name)) in entries
            .into_iter()
            .enumerate()
            .skip(offset as usize)
        {
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

fn slice_chunk_bounds(start: usize, end: usize, chunk_index: usize, chunk_size: usize) -> (usize, usize) {
    let chunk_start = chunk_index * chunk_size;
    let chunk_end = chunk_start + chunk_size;
    let a = start.saturating_sub(chunk_start);
    let b = end.saturating_sub(chunk_start).min(chunk_end - chunk_start);
    (a, b)
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
/// 
/// Performance: This is on the hot path - uses minimal allocations.
#[inline]
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
#[inline]
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
#[inline]
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
}
