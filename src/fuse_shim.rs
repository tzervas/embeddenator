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
//! └────────────────────────────────┬───────────────────────────────┘
//!                                  │ VFS syscalls
//!                                  ▼
//! ┌────────────────────────────────────────────────────────────────┐
//! │                     Linux Kernel VFS                           │
//! └────────────────────────────────┬───────────────────────────────┘
//!                                  │ FUSE protocol
//!                                  ▼
//! ┌────────────────────────────────────────────────────────────────┐
//! │                   /dev/fuse character device                   │
//! └────────────────────────────────┬───────────────────────────────┘
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
//! # Mount an engram
//! embeddenator mount --engram root.engram --manifest manifest.json --codebook private.cb /mnt/engram
//!
//! # Access files normally
//! ls /mnt/engram
//! cat /mnt/engram/some/file.txt
//!
//! # Unmount
//! fusermount -u /mnt/engram
//! ```

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

/// Inode number type
pub type Ino = u64;

/// File attributes for FUSE
#[derive(Clone, Debug)]
pub struct FileAttr {
    pub ino: Ino,
    pub size: u64,
    pub blocks: u64,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
    pub crtime: SystemTime,
    pub kind: FileKind,
    pub perm: u16,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
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
            uid: 1000,  // Default UID
            gid: 1000,  // Default GID
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}

/// File type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Directory,
    RegularFile,
    Symlink,
}

/// Directory entry
#[derive(Clone, Debug)]
pub struct DirEntry {
    pub ino: Ino,
    pub name: String,
    pub kind: FileKind,
}

/// Cached file data for read operations
#[derive(Clone)]
pub struct CachedFile {
    pub data: Vec<u8>,
    pub attr: FileAttr,
}

/// The EngramFS FUSE filesystem implementation
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
    /// Root inode number (standard FUSE convention)
    pub const ROOT_INO: Ino = 1;

    /// Create a new EngramFS instance
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
            ino: Self::ROOT_INO,
            size: 0,
            blocks: 0,
            kind: FileKind::Directory,
            perm: 0o755,
            nlink: 2,
            ..Default::default()
        };

        self.inodes.write().unwrap().insert(Self::ROOT_INO, root_attr);
        self.inode_paths.write().unwrap().insert(Self::ROOT_INO, "/".to_string());
        self.path_inodes.write().unwrap().insert("/".to_string(), Self::ROOT_INO);
        self.directories.write().unwrap().insert(Self::ROOT_INO, Vec::new());
    }

    /// Allocate a new inode number
    fn alloc_ino(&self) -> Ino {
        let mut next = self.next_ino.write().unwrap();
        let ino = *next;
        *next += 1;
        ino
    }

    /// Add a file to the filesystem
    pub fn add_file(&self, path: &str, data: Vec<u8>) -> Result<Ino, &'static str> {
        let path = normalize_path(path);
        
        // Check if already exists
        if self.path_inodes.read().unwrap().contains_key(&path) {
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
            blocks: (size + 511) / 512,
            kind: FileKind::RegularFile,
            perm: 0o644,
            nlink: 1,
            ..Default::default()
        };

        // Store file
        self.inodes.write().unwrap().insert(ino, attr.clone());
        self.inode_paths.write().unwrap().insert(ino, path.clone());
        self.path_inodes.write().unwrap().insert(path.clone(), ino);
        self.file_cache.write().unwrap().insert(ino, CachedFile { data, attr });

        // Add to parent directory
        let filename = filename(&path).ok_or("Invalid filename")?;
        self.directories.write().unwrap()
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
            return Ok(Self::ROOT_INO);
        }

        // Check if already exists
        if let Some(&ino) = self.path_inodes.read().unwrap().get(&path) {
            return Ok(ino);
        }

        // Create parent first
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

        self.inodes.write().unwrap().insert(ino, attr);
        self.inode_paths.write().unwrap().insert(ino, path.clone());
        self.path_inodes.write().unwrap().insert(path.clone(), ino);
        self.directories.write().unwrap().insert(ino, Vec::new());

        // Add to parent
        let dirname = filename(&path).ok_or("Invalid dirname")?;
        self.directories.write().unwrap()
            .get_mut(&parent_ino)
            .ok_or("Parent not found")?
            .push(DirEntry {
                ino,
                name: dirname.to_string(),
                kind: FileKind::Directory,
            });

        // Update parent nlink
        if let Some(parent_attr) = self.inodes.write().unwrap().get_mut(&parent_ino) {
            parent_attr.nlink += 1;
        }

        Ok(ino)
    }

    /// Lookup a path and return its inode
    pub fn lookup_path(&self, path: &str) -> Option<Ino> {
        let path = normalize_path(path);
        self.path_inodes.read().unwrap().get(&path).copied()
    }

    /// Get file attributes by inode
    pub fn getattr(&self, ino: Ino) -> Option<FileAttr> {
        self.inodes.read().unwrap().get(&ino).cloned()
    }

    /// Read file data
    pub fn read(&self, ino: Ino, offset: u64, size: u32) -> Option<Vec<u8>> {
        let cache = self.file_cache.read().unwrap();
        let cached = cache.get(&ino)?;
        
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, cached.data.len());
        
        if start >= cached.data.len() {
            return Some(Vec::new());
        }
        
        Some(cached.data[start..end].to_vec())
    }

    /// Read directory contents
    pub fn readdir(&self, ino: Ino) -> Option<Vec<DirEntry>> {
        self.directories.read().unwrap().get(&ino).cloned()
    }

    /// Lookup entry in directory by name
    pub fn lookup(&self, parent_ino: Ino, name: &str) -> Option<Ino> {
        let dirs = self.directories.read().unwrap();
        let entries = dirs.get(&parent_ino)?;
        entries.iter().find(|e| e.name == name).map(|e| e.ino)
    }

    /// Get total number of files
    pub fn file_count(&self) -> usize {
        self.file_cache.read().unwrap().len()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        self.file_cache.read().unwrap()
            .values()
            .map(|f| f.attr.size)
            .sum()
    }
}

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

/// FUSE operation handlers (to be implemented with fuser crate)
pub mod fuse_ops {
    use super::*;

    /// Initialize filesystem
    pub fn init(_fs: &EngramFS) -> i32 {
        0 // Success
    }

    /// Lookup entry
    pub fn lookup(fs: &EngramFS, parent: Ino, name: &OsStr) -> Option<FileAttr> {
        let name = name.to_str()?;
        let ino = fs.lookup(parent, name)?;
        fs.getattr(ino)
    }

    /// Get attributes
    pub fn getattr(fs: &EngramFS, ino: Ino) -> Option<FileAttr> {
        fs.getattr(ino)
    }

    /// Read data
    pub fn read(fs: &EngramFS, ino: Ino, offset: i64, size: u32) -> Option<Vec<u8>> {
        fs.read(ino, offset as u64, size)
    }

    /// Read directory
    pub fn readdir(fs: &EngramFS, ino: Ino, offset: i64) -> Vec<(Ino, i64, FileKind, String)> {
        let mut result = Vec::new();
        
        // Add . and ..
        if offset == 0 {
            result.push((ino, 1, FileKind::Directory, ".".to_string()));
        }
        if offset <= 1 {
            // For root, .. points to itself
            let parent = if ino == EngramFS::ROOT_INO { ino } else {
                // Would need parent tracking for proper implementation
                EngramFS::ROOT_INO
            };
            result.push((parent, 2, FileKind::Directory, "..".to_string()));
        }
        
        // Add directory entries
        if let Some(entries) = fs.readdir(ino) {
            for (i, entry) in entries.iter().enumerate() {
                let entry_offset = (i + 3) as i64;
                if entry_offset > offset {
                    result.push((entry.ino, entry_offset, entry.kind, entry.name.clone()));
                }
            }
        }
        
        result
    }
}

/// Builder for creating an EngramFS from engram data
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
    pub fn add_file(mut self, path: &str, data: Vec<u8>) -> Self {
        let _ = self.fs.add_file(path, data);
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

/// Statistics for the mounted filesystem
#[derive(Clone, Debug, Default)]
pub struct MountStats {
    pub reads: u64,
    pub read_bytes: u64,
    pub lookups: u64,
    pub readdirs: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub decode_time_us: u64,
}

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
        assert!(ino > EngramFS::ROOT_INO);
        
        let data = fs.read(ino, 0, 100).unwrap();
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
        
        let root_entries = fs.readdir(EngramFS::ROOT_INO).unwrap();
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
        let partial = fs.read(ino, 3, 4).unwrap();
        assert_eq!(partial, b"3456");
        
        // Read past end
        let past_end = fs.read(ino, 20, 10).unwrap();
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
}
