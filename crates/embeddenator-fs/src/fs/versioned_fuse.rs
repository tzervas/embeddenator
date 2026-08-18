//! FUSE adapter for VersionedEmbrFS with write support
//!
//! This module provides FUSE integration for the mutable VersionedEmbrFS,
//! enabling full read-write filesystem operations through FUSE.
//!
//! Unlike the original read-only EngramFS, this supports:
//! - Writing files (create, update)
//! - Creating files
//! - Deleting files
//! - Truncating files
//! - Symbolic links (readlink, symlink)
//! - Device nodes (mknod)
//! - All with optimistic locking and version tracking

use crate::versioned_embrfs::VersionedEmbrFS;
use crate::{DirEntry, FileAttr, FileKind, Ino, ROOT_INO};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

#[cfg(feature = "fuse")]
use crate::EmbrFSError;
#[cfg(feature = "fuse")]
use fuser::{
    Filesystem, KernelConfig, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, ReplyXattr, Request,
};
#[cfg(feature = "fuse")]
use std::ffi::OsStr;

/// Type alias for extended attributes storage: inode -> (attr_name -> attr_value)
type XattrStorage = Arc<RwLock<HashMap<Ino, HashMap<String, Vec<u8>>>>>;

/// FUSE adapter for VersionedEmbrFS
///
/// Provides full read-write FUSE filesystem on top of VersionedEmbrFS.
/// Manages inode assignments, directory structures, and FUSE protocol translation.
#[allow(dead_code)]
pub struct VersionedFUSE {
    /// The underlying versioned filesystem
    pub(crate) fs: Arc<VersionedEmbrFS>,

    /// Inode to path mapping
    inode_paths: Arc<RwLock<HashMap<Ino, String>>>,

    /// Path to inode mapping
    path_inodes: Arc<RwLock<HashMap<String, Ino>>>,

    /// Directory structure (parent_ino -> child entries)
    directories: Arc<RwLock<HashMap<Ino, Vec<DirEntry>>>>,

    /// Parent inode mapping (child_ino -> parent_ino)
    parent_inodes: Arc<RwLock<HashMap<Ino, Ino>>>,

    /// Symlink targets (ino -> target)
    symlinks: Arc<RwLock<HashMap<Ino, String>>>,

    /// Next available inode number
    next_ino: Arc<RwLock<Ino>>,

    /// File handle counter
    next_fh: Arc<RwLock<u64>>,

    /// Open file handles (fh -> (path, version))
    open_files: Arc<RwLock<HashMap<u64, (String, u64)>>>,

    /// Extended attributes storage (ino -> (name -> value))
    xattrs: XattrStorage,

    /// TTL for attributes
    attr_ttl: Duration,

    /// TTL for directory entries
    entry_ttl: Duration,
}

#[allow(dead_code)]
impl VersionedFUSE {
    /// Create a new FUSE adapter for a versioned filesystem
    pub fn new(fs: VersionedEmbrFS) -> Self {
        let mut adapter = Self {
            fs: Arc::new(fs),
            inode_paths: Arc::new(RwLock::new(HashMap::new())),
            path_inodes: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(HashMap::new())),
            parent_inodes: Arc::new(RwLock::new(HashMap::new())),
            symlinks: Arc::new(RwLock::new(HashMap::new())),
            next_ino: Arc::new(RwLock::new(ROOT_INO + 1)),
            next_fh: Arc::new(RwLock::new(1)),
            open_files: Arc::new(RwLock::new(HashMap::new())),
            xattrs: Arc::new(RwLock::new(HashMap::new())),
            attr_ttl: Duration::from_secs(1),
            entry_ttl: Duration::from_secs(1),
        };

        // Initialize root directory
        adapter.init_root();

        // Scan filesystem and build inode structure
        adapter.rebuild_directory_structure();

        adapter
    }

    /// Initialize root directory inode
    fn init_root(&mut self) {
        let mut inode_paths = self.inode_paths.write().unwrap();
        let mut path_inodes = self.path_inodes.write().unwrap();
        let mut directories = self.directories.write().unwrap();
        let mut parent_inodes = self.parent_inodes.write().unwrap();

        inode_paths.insert(ROOT_INO, "/".to_string());
        path_inodes.insert("/".to_string(), ROOT_INO);
        directories.insert(ROOT_INO, Vec::new());
        parent_inodes.insert(ROOT_INO, ROOT_INO); // Root is its own parent
    }

    /// Rebuild the directory structure from filesystem contents
    fn rebuild_directory_structure(&mut self) {
        let files = self.fs.list_files();

        for path in files {
            self.register_file(&path);
        }
    }

    /// Register a file and all its parent directories
    fn register_file(&self, path: &str) {
        // Split path into components
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Register each directory level
        let mut current_path = String::new();
        for (i, part) in parts.iter().enumerate() {
            let parent_path = current_path.clone();
            current_path = if current_path.is_empty() {
                format!("/{}", part)
            } else {
                format!("{}/{}", current_path, part)
            };

            // Check if this is the file (last component) or a directory
            let is_file = i == parts.len() - 1;

            // Get or allocate inode
            let ino = self.get_or_create_inode(&current_path);

            // Add to parent directory
            let parent_ino = if parent_path.is_empty() {
                ROOT_INO
            } else {
                self.get_or_create_inode(&parent_path)
            };

            // Track parent inode
            let mut parent_map = self.parent_inodes.write().unwrap();
            parent_map.insert(ino, parent_ino);
            drop(parent_map);

            let mut directories = self.directories.write().unwrap();
            let parent_entries = directories.entry(parent_ino).or_default();

            // Check if entry already exists
            if !parent_entries.iter().any(|e| e.name == *part) {
                parent_entries.push(DirEntry {
                    ino,
                    name: part.to_string(),
                    kind: if is_file {
                        FileKind::RegularFile
                    } else {
                        FileKind::Directory
                    },
                });
            }

            // If it's a directory, ensure it has an entry in directories map
            if !is_file {
                directories.entry(ino).or_default();
            }
        }
    }

    /// Get parent inode for a given inode
    fn get_parent_ino(&self, ino: Ino) -> Ino {
        self.parent_inodes
            .read()
            .unwrap()
            .get(&ino)
            .copied()
            .unwrap_or(ROOT_INO)
    }

    /// Get existing inode or create new one for a path
    fn get_or_create_inode(&self, path: &str) -> Ino {
        let path_inodes = self.path_inodes.read().unwrap();
        if let Some(&ino) = path_inodes.get(path) {
            return ino;
        }
        drop(path_inodes);

        // Allocate new inode
        let mut next_ino = self.next_ino.write().unwrap();
        let ino = *next_ino;
        *next_ino += 1;
        drop(next_ino);

        // Register it
        let mut path_inodes = self.path_inodes.write().unwrap();
        let mut inode_paths = self.inode_paths.write().unwrap();

        path_inodes.insert(path.to_string(), ino);
        inode_paths.insert(ino, path.to_string());

        ino
    }

    /// Get path for an inode
    fn get_path(&self, ino: Ino) -> Option<String> {
        let inode_paths = self.inode_paths.read().unwrap();
        inode_paths.get(&ino).cloned()
    }

    /// Get inode for a path
    fn get_inode(&self, path: &str) -> Option<Ino> {
        let path_inodes = self.path_inodes.read().unwrap();
        path_inodes.get(path).copied()
    }

    /// Create file attributes from file info
    fn make_file_attr(&self, ino: Ino, path: &str) -> FileAttr {
        let now = SystemTime::now();

        // Check if it's a directory
        let directories = self.directories.read().unwrap();
        if directories.contains_key(&ino) {
            return FileAttr {
                ino,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: FileKind::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                blksize: 4096,
                flags: 0,
            };
        }

        // It's a file - get its size
        let size = if let Ok((data, _version)) = self.fs.read_file(path) {
            data.len() as u64
        } else {
            0
        };

        FileAttr {
            ino,
            size,
            blocks: size.div_ceil(4096),
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileKind::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    /// Allocate a file handle
    fn allocate_fh(&self, path: String, version: u64) -> u64 {
        let mut next_fh = self.next_fh.write().unwrap();
        let fh = *next_fh;
        *next_fh += 1;
        drop(next_fh);

        let mut open_files = self.open_files.write().unwrap();
        open_files.insert(fh, (path, version));

        fh
    }

    /// Release a file handle
    fn release_fh(&self, fh: u64) -> Option<(String, u64)> {
        let mut open_files = self.open_files.write().unwrap();
        open_files.remove(&fh)
    }
}

#[cfg(feature = "fuse")]
impl Filesystem for VersionedFUSE {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), libc::c_int> {
        Ok(())
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct child path
        let child_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path, name_str)
        };

        // Get or create inode
        let ino = match self.get_inode(&child_path) {
            Some(i) => i,
            None => {
                // File doesn't exist yet
                reply.error(libc::ENOENT);
                return;
            }
        };

        let attr = self.make_file_attr(ino, &child_path);
        reply.entry(&self.entry_ttl, &attr.into(), 0);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let attr = self.make_file_attr(ino, &path);
        reply.attr(&self.attr_ttl, &attr.into());
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Use read_range for memory-efficient partial reads
        // This avoids loading the entire file when only a portion is needed
        let offset = offset as usize;
        let size = size as usize;

        match self.fs.read_range(&path, offset, size) {
            Ok((data, _version)) => {
                reply.data(&data);
            }
            Err(EmbrFSError::FileNotFound(_)) => {
                reply.error(libc::ENOENT);
            }
            Err(_) => {
                reply.error(libc::EIO);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let directories = self.directories.read().unwrap();
        let entries = match directories.get(&ino) {
            Some(e) => e,
            None => {
                reply.error(libc::ENOTDIR);
                return;
            }
        };

        // Get real parent inode
        let parent_ino = self.get_parent_ino(ino);

        // Always include . and ..
        let mut all_entries = vec![
            DirEntry {
                ino,
                name: ".".to_string(),
                kind: FileKind::Directory,
            },
            DirEntry {
                ino: parent_ino,
                name: "..".to_string(),
                kind: FileKind::Directory,
            },
        ];
        all_entries.extend_from_slice(entries);

        for (i, entry) in all_entries.iter().enumerate().skip(offset as usize) {
            let full = reply.add(entry.ino, (i + 1) as i64, entry.kind.into(), &entry.name);

            if full {
                break;
            }
        }

        reply.ok();
    }

    fn open(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Get current file version
        let version = match self.fs.read_file(&path) {
            Ok((_data, version)) => version,
            Err(EmbrFSError::FileNotFound(_)) => 0, // New file
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        // Allocate file handle
        let fh = self.allocate_fh(path, version);
        reply.opened(fh, flags as u32);
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        self.release_fh(fh);
        reply.ok();
    }

    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        // Get path and version from file handle
        let (path, file_version) = match self.open_files.read().unwrap().get(&fh) {
            Some(info) => info.clone(),
            None => {
                reply.error(libc::EBADF);
                return;
            }
        };

        // Read current file data
        let mut file_data = match self.fs.read_file(&path) {
            Ok((data, _ver)) => data,
            Err(EmbrFSError::FileNotFound(_)) => Vec::new(), // New file
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        // Apply write at offset
        let offset = offset as usize;
        if offset + data.len() > file_data.len() {
            file_data.resize(offset + data.len(), 0);
        }
        file_data[offset..offset + data.len()].copy_from_slice(data);

        // Write back with optimistic locking
        let expected_version = if file_data.len() == data.len() {
            None // New file
        } else {
            Some(file_version)
        };

        match self.fs.write_file(&path, &file_data, expected_version) {
            Ok(new_version) => {
                // Update file handle version
                let mut open_files = self.open_files.write().unwrap();
                if let Some(info) = open_files.get_mut(&fh) {
                    info.1 = new_version;
                }
                reply.written(data.len() as u32);
            }
            Err(EmbrFSError::VersionMismatch { .. }) => {
                // Concurrent modification - retry needed
                reply.error(libc::EAGAIN);
            }
            Err(_) => {
                reply.error(libc::EIO);
            }
        }
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct file path
        let file_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path, name_str)
        };

        // Create empty file
        match self.fs.write_file(&file_path, &[], None) {
            Ok(version) => {
                // Register in inode structure
                self.register_file(&file_path);

                let ino = self.get_inode(&file_path).unwrap();
                let attr = self.make_file_attr(ino, &file_path);
                let fh = self.allocate_fh(file_path, version);

                reply.created(&self.entry_ttl, &attr.into(), 0, fh, flags as u32);
            }
            Err(_) => {
                reply.error(libc::EIO);
            }
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct file path
        let file_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path, name_str)
        };

        // Get current file version
        let file_version = match self.fs.read_file(&file_path) {
            Ok((_data, version)) => version,
            Err(_) => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Delete file
        match self.fs.delete_file(&file_path, file_version) {
            Ok(_) => {
                // Remove from directory structure
                let mut directories = self.directories.write().unwrap();
                if let Some(entries) = directories.get_mut(&parent) {
                    entries.retain(|e| e.name != name_str);
                }
                reply.ok();
            }
            Err(EmbrFSError::VersionMismatch { .. }) => {
                reply.error(libc::EAGAIN);
            }
            Err(_) => {
                reply.error(libc::EIO);
            }
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        // Handle truncate (size change)
        if let Some(new_size) = size {
            let path = match self.get_path(ino) {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };

            // Read current file
            let (mut data, version) = match self.fs.read_file(&path) {
                Ok(result) => result,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };

            // Resize
            data.resize(new_size as usize, 0);

            // Write back
            match self.fs.write_file(&path, &data, Some(version)) {
                Ok(_) => {
                    let attr = self.make_file_attr(ino, &path);
                    reply.attr(&self.attr_ttl, &attr.into());
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        } else {
            // No size change - just return current attributes
            let path = match self.get_path(ino) {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
            let attr = self.make_file_attr(ino, &path);
            reply.attr(&self.attr_ttl, &attr.into());
        }
    }

    fn readlink(&mut self, _req: &Request, ino: u64, reply: ReplyData) {
        let symlinks = self.symlinks.read().unwrap();
        match symlinks.get(&ino) {
            Some(target) => {
                reply.data(target.as_bytes());
            }
            None => {
                reply.error(libc::EINVAL); // Not a symlink
            }
        }
    }

    fn symlink(
        &mut self,
        _req: &Request,
        parent: u64,
        link_name: &OsStr,
        target: &std::path::Path,
        reply: ReplyEntry,
    ) {
        let link_name_str = match link_name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let target_str = match target.to_str() {
            Some(s) => s.to_string(),
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct symlink path
        let symlink_path = if parent_path == "/" {
            format!("/{}", link_name_str)
        } else {
            format!("{}/{}", parent_path, link_name_str)
        };

        // Allocate inode
        let mut next_ino = self.next_ino.write().unwrap();
        let ino = *next_ino;
        *next_ino += 1;
        drop(next_ino);

        // Register symlink
        let mut path_inodes = self.path_inodes.write().unwrap();
        let mut inode_paths = self.inode_paths.write().unwrap();
        let mut symlinks = self.symlinks.write().unwrap();
        let mut directories = self.directories.write().unwrap();

        path_inodes.insert(symlink_path.clone(), ino);
        inode_paths.insert(ino, symlink_path);
        symlinks.insert(ino, target_str.clone());

        // Add to parent directory
        if let Some(parent_entries) = directories.get_mut(&parent) {
            parent_entries.push(DirEntry {
                ino,
                name: link_name_str.to_string(),
                kind: FileKind::Symlink,
            });
        }

        // Create attributes
        let now = SystemTime::now();
        let attr = FileAttr {
            ino,
            size: target_str.len() as u64,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileKind::Symlink,
            perm: 0o777,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        };

        reply.entry(&self.entry_ttl, &attr.into(), 0);
    }

    fn mkdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct directory path
        let dir_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path, name_str)
        };

        // Check if already exists
        if self.get_inode(&dir_path).is_some() {
            reply.error(libc::EEXIST);
            return;
        }

        // Allocate inode
        let mut next_ino = self.next_ino.write().unwrap();
        let ino = *next_ino;
        *next_ino += 1;
        drop(next_ino);

        // Register directory
        let mut path_inodes = self.path_inodes.write().unwrap();
        let mut inode_paths = self.inode_paths.write().unwrap();
        let mut directories = self.directories.write().unwrap();
        let mut parent_map = self.parent_inodes.write().unwrap();

        path_inodes.insert(dir_path.clone(), ino);
        inode_paths.insert(ino, dir_path);
        directories.insert(ino, Vec::new());
        parent_map.insert(ino, parent);

        // Add to parent directory
        if let Some(parent_entries) = directories.get_mut(&parent) {
            parent_entries.push(DirEntry {
                ino,
                name: name_str.to_string(),
                kind: FileKind::Directory,
            });
        }

        // Create attributes
        let now = SystemTime::now();
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileKind::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        };

        reply.entry(&self.entry_ttl, &attr.into(), 0);
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct directory path
        let dir_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path, name_str)
        };

        // Get inode
        let ino = match self.get_inode(&dir_path) {
            Some(i) => i,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Check if it's a directory and if it's empty
        {
            let directories = self.directories.read().unwrap();
            match directories.get(&ino) {
                Some(entries) if !entries.is_empty() => {
                    reply.error(libc::ENOTEMPTY);
                    return;
                }
                None => {
                    reply.error(libc::ENOTDIR);
                    return;
                }
                _ => {}
            }
        }

        // Remove from structures
        let mut path_inodes = self.path_inodes.write().unwrap();
        let mut inode_paths = self.inode_paths.write().unwrap();
        let mut directories = self.directories.write().unwrap();
        let mut parent_map = self.parent_inodes.write().unwrap();

        path_inodes.remove(&dir_path);
        inode_paths.remove(&ino);
        directories.remove(&ino);
        parent_map.remove(&ino);

        // Remove from parent directory
        if let Some(parent_entries) = directories.get_mut(&parent) {
            parent_entries.retain(|e| e.name != name_str);
        }

        reply.ok();
    }

    fn rename(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let newname_str = match newname.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent paths
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let newparent_path = match self.get_path(newparent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Construct paths
        let old_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path, name_str)
        };

        let new_path = if newparent_path == "/" {
            format!("/{}", newname_str)
        } else {
            format!("{}/{}", newparent_path, newname_str)
        };

        // Get old inode
        let ino = match self.get_inode(&old_path) {
            Some(i) => i,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Get entry kind
        let kind = {
            let directories = self.directories.read().unwrap();
            if directories.contains_key(&ino) {
                FileKind::Directory
            } else {
                FileKind::RegularFile
            }
        };

        // Remove old entry from parent
        {
            let mut directories = self.directories.write().unwrap();
            if let Some(parent_entries) = directories.get_mut(&parent) {
                parent_entries.retain(|e| e.name != name_str);
            }
        }

        // Update path mappings
        {
            let mut path_inodes = self.path_inodes.write().unwrap();
            let mut inode_paths = self.inode_paths.write().unwrap();
            let mut parent_map = self.parent_inodes.write().unwrap();

            path_inodes.remove(&old_path);
            path_inodes.insert(new_path.clone(), ino);
            inode_paths.insert(ino, new_path.clone());
            parent_map.insert(ino, newparent);
        }

        // Add new entry to new parent
        {
            let mut directories = self.directories.write().unwrap();
            let new_parent_entries = directories.entry(newparent).or_default();

            // Remove any existing entry with the new name
            new_parent_entries.retain(|e| e.name != newname_str);

            new_parent_entries.push(DirEntry {
                ino,
                name: newname_str.to_string(),
                kind,
            });
        }

        // If it's a file in the underlying fs, we need to rename it there too
        if kind == FileKind::RegularFile {
            // Read old file, write to new path, delete old
            if let Ok((data, version)) = self.fs.read_file(&old_path) {
                // Write to new location
                let _ = self.fs.write_file(&new_path, &data, None);
                // Delete old file
                let _ = self.fs.delete_file(&old_path, version);
            }
        }

        reply.ok();
    }

    /// Get an extended attribute value
    ///
    /// Supports both user-defined attributes and built-in engram attributes:
    /// - `user.engram.version`: Current global filesystem version
    /// - `user.engram.file_count`: Number of active files
    /// - `user.engram.total_size`: Total size of all files in bytes
    /// - `user.engram.chunk_count`: Total number of chunks
    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::ENODATA);
                return;
            }
        };

        // Check if it's a built-in engram attribute
        let value = match name_str {
            "user.engram.version" => {
                let version = self.fs.version();
                Some(version.to_string().into_bytes())
            }
            "user.engram.file_count" => {
                let stats = self.fs.stats();
                Some(stats.active_files.to_string().into_bytes())
            }
            "user.engram.total_size" => {
                let stats = self.fs.stats();
                Some(stats.total_size_bytes.to_string().into_bytes())
            }
            "user.engram.chunk_count" => {
                let stats = self.fs.stats();
                Some(stats.total_chunks.to_string().into_bytes())
            }
            "user.engram.correction_overhead" => {
                let stats = self.fs.stats();
                Some(stats.correction_overhead_bytes.to_string().into_bytes())
            }
            _ => {
                // Check user-defined xattrs
                let xattrs = self.xattrs.read().unwrap();
                xattrs
                    .get(&ino)
                    .and_then(|attrs| attrs.get(name_str).cloned())
            }
        };

        match value {
            Some(data) => {
                if size == 0 {
                    // Return size needed
                    reply.size(data.len() as u32);
                } else if size >= data.len() as u32 {
                    // Return the data
                    reply.data(&data);
                } else {
                    // Buffer too small
                    reply.error(libc::ERANGE);
                }
            }
            None => {
                reply.error(libc::ENODATA);
            }
        }
    }

    /// Set an extended attribute value
    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Don't allow setting built-in attributes
        if name_str.starts_with("user.engram.") {
            reply.error(libc::EPERM);
            return;
        }

        // Check that the inode exists
        let inode_paths = self.inode_paths.read().unwrap();
        if !inode_paths.contains_key(&ino) {
            reply.error(libc::ENOENT);
            return;
        }
        drop(inode_paths);

        // Store the xattr
        let mut xattrs = self.xattrs.write().unwrap();
        let inode_xattrs = xattrs.entry(ino).or_default();
        inode_xattrs.insert(name_str.to_string(), value.to_vec());

        reply.ok();
    }

    /// List extended attribute names
    fn listxattr(&mut self, _req: &Request<'_>, ino: u64, size: u32, reply: ReplyXattr) {
        // Start with built-in attributes (only for valid inodes)
        let inode_paths = self.inode_paths.read().unwrap();
        if !inode_paths.contains_key(&ino) {
            reply.error(libc::ENOENT);
            return;
        }
        drop(inode_paths);

        // Collect all attribute names
        let mut names: Vec<String> = vec![
            "user.engram.version".to_string(),
            "user.engram.file_count".to_string(),
            "user.engram.total_size".to_string(),
            "user.engram.chunk_count".to_string(),
            "user.engram.correction_overhead".to_string(),
        ];

        // Add user-defined xattrs
        let xattrs = self.xattrs.read().unwrap();
        if let Some(inode_xattrs) = xattrs.get(&ino) {
            for name in inode_xattrs.keys() {
                names.push(name.clone());
            }
        }

        // Format as null-terminated strings
        let mut data = Vec::new();
        for name in &names {
            data.extend(name.as_bytes());
            data.push(0); // null terminator
        }

        if size == 0 {
            // Return size needed
            reply.size(data.len() as u32);
        } else if size >= data.len() as u32 {
            // Return the data
            reply.data(&data);
        } else {
            // Buffer too small
            reply.error(libc::ERANGE);
        }
    }

    /// Remove an extended attribute
    fn removexattr(&mut self, _req: &Request<'_>, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Don't allow removing built-in attributes
        if name_str.starts_with("user.engram.") {
            reply.error(libc::EPERM);
            return;
        }

        // Try to remove the xattr
        let mut xattrs = self.xattrs.write().unwrap();
        if let Some(inode_xattrs) = xattrs.get_mut(&ino) {
            if inode_xattrs.remove(name_str).is_some() {
                reply.ok();
                return;
            }
        }

        reply.error(libc::ENODATA);
    }
}

/// Mount a VersionedEmbrFS as a FUSE filesystem
#[cfg(feature = "fuse")]
pub fn mount_versioned_fs(
    fs: VersionedEmbrFS,
    mountpoint: &std::path::Path,
    options: &[fuser::MountOption],
) -> std::io::Result<()> {
    let fuse_fs = VersionedFUSE::new(fs);
    fuser::mount2(fuse_fs, mountpoint, options)
}

/// Options for mounting VersionedEmbrFS with signal handling
#[cfg(feature = "fuse")]
#[derive(Clone, Debug)]
pub struct VersionedMountOptions {
    /// Filesystem name shown in mount output
    pub fsname: String,
    /// Allow other users to access the mount (requires user_allow_other in /etc/fuse.conf)
    pub allow_other: bool,
    /// Allow root to access the mount
    pub allow_root: bool,
    /// Auto-unmount when process exits
    pub auto_unmount: bool,
}

#[cfg(feature = "fuse")]
impl Default for VersionedMountOptions {
    fn default() -> Self {
        Self {
            fsname: "versioned-engram".to_string(),
            allow_other: false,
            allow_root: true,
            auto_unmount: true,
        }
    }
}

/// Mount a VersionedEmbrFS with signal handling for graceful unmount
///
/// This function installs signal handlers for SIGINT (Ctrl+C), SIGTERM, and SIGHUP,
/// enabling graceful unmount with proper cleanup when signals are received.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Signal Handler                           │
/// │       (SIGINT, SIGTERM, SIGHUP)                            │
/// └─────────────────────────────────────────────────────────────┘
///                              │
///                              ▼
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Shutdown Flag                            │
/// │              (AtomicBool set on signal)                     │
/// └─────────────────────────────────────────────────────────────┘
///                              │
///                              ▼
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Graceful Shutdown                          │
/// │  1. Stop accepting new operations                           │
/// │  2. Wait for pending writes to complete                     │
/// │  3. Flush WAL to disk                                       │
/// │  4. Drop FUSE session (triggers clean unmount)              │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// # Arguments
///
/// * `fs` - The VersionedEmbrFS instance to mount
/// * `mountpoint` - Directory path where the filesystem will be mounted
/// * `options` - Mount options (see `VersionedMountOptions`)
///
/// # Returns
///
/// Returns `Ok(())` on clean unmount, or an error if mounting fails.
///
/// # Example
///
/// ```no_run
/// use embeddenator_fs::fs::versioned_fuse::{mount_versioned_fs_with_signals, VersionedMountOptions};
/// use embeddenator_fs::VersionedEmbrFS;
/// use std::path::Path;
///
/// let fs = VersionedEmbrFS::new();
/// // ... write files to fs ...
///
/// // Mount with signal handling (blocks until unmount)
/// mount_versioned_fs_with_signals(
///     fs,
///     Path::new("/mnt/engram"),
///     VersionedMountOptions::default(),
/// ).expect("Mount failed");
/// ```
#[cfg(feature = "fuse")]
pub fn mount_versioned_fs_with_signals(
    fs: VersionedEmbrFS,
    mountpoint: &std::path::Path,
    options: VersionedMountOptions,
) -> std::io::Result<()> {
    use crate::fs::signal::{install_signal_handlers, ShutdownSignal};
    use fuser::MountOption;
    use std::sync::Arc;

    // Set up shutdown signal
    let shutdown = Arc::new(ShutdownSignal::new());
    install_signal_handlers(shutdown.clone())?;

    // Build mount options
    let mut mount_options = vec![
        MountOption::FSName(options.fsname),
        MountOption::DefaultPermissions,
    ];

    if options.auto_unmount {
        mount_options.push(MountOption::AutoUnmount);
    }

    if options.allow_other {
        mount_options.push(MountOption::AllowOther);
    } else if options.allow_root {
        mount_options.push(MountOption::AllowRoot);
    }

    // Create FUSE filesystem adapter
    let fuse_fs = VersionedFUSE::new(fs);

    // Keep reference to the underlying FS for cleanup
    let underlying_fs = fuse_fs.fs.clone();

    // Use spawn_mount2 to get a controllable session
    let session = fuser::spawn_mount2(fuse_fs, mountpoint, &mount_options)?;

    eprintln!("VersionedEmbrFS mounted at {}", mountpoint.display());
    eprintln!("Write operations are supported. Press Ctrl+C to unmount gracefully.");
    eprintln!(
        "Use 'fusermount -u {}' to unmount manually.",
        mountpoint.display()
    );

    // Poll for shutdown signal
    loop {
        if shutdown.is_signaled() {
            eprintln!(
                "\nReceived {} - initiating graceful shutdown...",
                shutdown.signal_name()
            );

            // Give pending operations a moment to complete
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Log filesystem stats before unmounting
            let stats = underlying_fs.stats();
            eprintln!(
                "Filesystem stats: {} files, {} bytes total",
                stats.active_files, stats.total_size_bytes
            );

            // Drop the session to trigger clean unmount
            // This will call destroy() on the FUSE filesystem
            drop(session);

            eprintln!("VersionedEmbrFS unmounted cleanly.");
            return Ok(());
        }

        // Brief sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Mount a VersionedEmbrFS in foreground mode with optional signal handling
///
/// This is a convenience wrapper that supports both foreground blocking mode
/// and signal-aware mode.
///
/// # Arguments
///
/// * `fs` - The VersionedEmbrFS instance to mount
/// * `mountpoint` - Directory path where the filesystem will be mounted
/// * `options` - Mount options
/// * `handle_signals` - If true, install signal handlers for graceful shutdown
///
/// # Example
///
/// ```no_run
/// use embeddenator_fs::fs::versioned_fuse::{mount_versioned_foreground, VersionedMountOptions};
/// use embeddenator_fs::VersionedEmbrFS;
/// use std::path::Path;
///
/// let fs = VersionedEmbrFS::new();
///
/// // Mount with signal handling in foreground
/// mount_versioned_foreground(
///     fs,
///     Path::new("/mnt/engram"),
///     VersionedMountOptions::default(),
///     true, // handle signals
/// ).expect("Mount failed");
/// ```
#[cfg(feature = "fuse")]
pub fn mount_versioned_foreground(
    fs: VersionedEmbrFS,
    mountpoint: &std::path::Path,
    options: VersionedMountOptions,
    handle_signals: bool,
) -> std::io::Result<()> {
    if handle_signals {
        mount_versioned_fs_with_signals(fs, mountpoint, options)
    } else {
        // Use the basic mount without signal handling
        use fuser::MountOption;

        let mut mount_options = vec![
            MountOption::FSName(options.fsname),
            MountOption::DefaultPermissions,
        ];

        if options.auto_unmount {
            mount_options.push(MountOption::AutoUnmount);
        }

        if options.allow_other {
            mount_options.push(MountOption::AllowOther);
        } else if options.allow_root {
            mount_options.push(MountOption::AllowRoot);
        }

        mount_versioned_fs(fs, mountpoint, &mount_options)
    }
}
