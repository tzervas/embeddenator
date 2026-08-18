# FUSE Implementation Guide

**Document Version:** 1.0  
**Last Updated:** January 10, 2026  
**Target Audience:** Developers, System Administrators

## Overview

EmbrFS provides optional FUSE (Filesystem in Userspace) support for mounting holographic engrams as read-only filesystems. This allows standard Unix tools (`ls`, `cat`, `grep`, etc.) to access engram contents transparently.

## Quick Start

### Prerequisites

**Linux Requirements:**
- Linux kernel 2.6.26 or later
- FUSE library installed:
  ```bash
  # Debian/Ubuntu
  sudo apt-get install libfuse3-3 libfuse3-dev
  
  # Fedora/RHEL
  sudo dnf install fuse3 fuse3-devel
  
  # Arch Linux
  sudo pacman -S fuse3
  ```

**Permissions:**
- Root access (for mounting), OR
- User must be in `fuse` group with `user_allow_other` in `/etc/fuse.conf`

**Enable Feature Flag:**
```toml
[dependencies]
embeddenator-fs = { version = "0.20.0-alpha.3", features = ["fuse"] }
```

### Basic Mounting

```rust
use embeddenator_fs::{EmbrFS, fuse::mount_embrfs};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load engram
    let fs = EmbrFS::load("filesystem.engram")?;
    
    // Mount as read-only filesystem
    let mountpoint = Path::new("/mnt/embrfs");
    mount_embrfs(fs, mountpoint, &[])?;
    
    Ok(())
}
```

**Command-line usage** (if CLI tool available):
```bash
# Mount engram
embrfs mount filesystem.engram /mnt/embrfs

# Unmount
fusermount -u /mnt/embrfs
# or
umount /mnt/embrfs
```

### Accessing Mounted Filesystem

Once mounted, access files normally:

```bash
# List files
ls -lah /mnt/embrfs

# Read file
cat /mnt/embrfs/file.txt

# Search
grep -r "pattern" /mnt/embrfs/

# Copy out
cp -r /mnt/embrfs/data /tmp/extracted

# Tar archive
tar czf backup.tar.gz -C /mnt/embrfs .
```

## FUSE Operations Reference

### Implemented Operations

| Operation   | Description                          | Status | Notes                          |
|-------------|--------------------------------------|--------|--------------------------------|
| `init`      | Initialize filesystem                |  Full | Called on mount               |
| `destroy`   | Cleanup on unmount                   |  Full | Called on unmount             |
| `lookup`    | Resolve filename to inode            |  Full | Path normalization applied    |
| `getattr`   | Get file/directory attributes        |  Full | Returns size, mode, timestamps|
| `read`      | Read file data                       |  Full | Supports offset and size      |
| `open`      | Open file for reading                |  Full | Read-only enforcement (O_RDONLY)|
| `release`   | Close file handle                    |  Full | Cleanup file state            |
| `opendir`   | Open directory                       |  Full | Validates directory exists    |
| `readdir`   | Read directory entries               |  Full | Includes `.` and `..`         |
| `releasedir`| Close directory handle               |  Full | Cleanup directory state       |
| `statfs`    | Get filesystem statistics            |  Full | Reports blocks, inodes        |
| `access`    | Check access permissions             |  Simplified | Basic mode checking      |
| `readlink`  | Read symbolic link target            |  ENOSYS | Symlinks not supported       |

### Not Implemented (Write Operations)

All write operations return **EROFS** (Read-Only Filesystem):

| Operation    | Why Not Implemented                                    |
|--------------|--------------------------------------------------------|
| `write`      | Engrams are immutable (by design)                      |
| `create`     | Cannot create files in immutable engram                |
| `mknod`      | Cannot create special files                            |
| `mkdir`      | Cannot create directories                              |
| `unlink`     | Cannot delete files (use API-level `remove_files`)     |
| `rmdir`      | Cannot delete directories                              |
| `rename`     | Cannot rename files                                    |
| `link`       | Hard links not supported                               |
| `symlink`    | Symbolic links not supported                           |
| `chmod`      | Permissions are immutable                              |
| `chown`      | Ownership is immutable                                 |
| `truncate`   | Cannot modify file sizes                               |
| `utimens`    | Timestamps are immutable                               |
| `flush`      | No write buffer to flush                               |
| `fsync`      | No dirty data to sync                                  |
| `setxattr`   | Extended attributes not supported                      |
| `getxattr`   | Extended attributes not supported                      |
| `listxattr`  | Extended attributes not supported                      |
| `removexattr`| Extended attributes not supported                      |

**Design Rationale:** Holographic engrams are immutable snapshots. Modifications require API-level operations (`add_files`, `modify_files`, etc.) that re-encode the engram. FUSE is for browsing, not editing.

## Implementation Details

### Inode Management

**Inode Number Assignment:**
- Root directory: inode 1
- Files/directories: Sequentially assigned from 2
- Stable across mounts (same path → same inode)

**Inode Table:**
```rust
pub struct InodeTable {
    path_to_inode: HashMap<PathBuf, u64>,
    inode_to_path: HashMap<u64, PathBuf>,
    next_inode: AtomicU64,
}
```

**Thread Safety:**
- Wrapped in `Arc<RwLock<...>>`
- Read lock for lookups (concurrent)
- Write lock for new inode assignments (rare)

### File Attributes

**Reported Attributes:**
- `st_ino` - Inode number
- `st_mode` - File type and permissions
  - Files: `0o100444` (r--r--r--)
  - Directories: `0o040555` (r-xr-xr-x)
- `st_nlink` - Hard link count (always 1)
- `st_uid` - User ID (current user)
- `st_gid` - Group ID (current group)
- `st_size` - File size in bytes
- `st_blocks` - Blocks allocated (calculated from size)
- `st_atime`, `st_mtime`, `st_ctime` - Timestamps (engram creation time)

**Limitations:**
- All files owned by mounting user (no per-file ownership)
- All files have read-only permissions (no per-file modes)
- Timestamps are engram creation time (no per-file modification times)

### Read Operation

**Implementation:**
```rust
fn read(
    &mut self,
    _req: &Request<'_>,
    ino: u64,
    fh: u64,
    offset: i64,
    size: u32,
    _flags: i32,
    _lock_owner: Option<u64>,
    reply: ReplyData,
) {
    // 1. Lookup path from inode
    // 2. Read file from engram (with offset and size)
    // 3. Return data or error
}
```

**Optimizations:**
- Partial reads supported (offset + size)
- Chunk-level caching (LRU for decoded chunks)
- Zero-copy where possible (direct buffer pass-through)

**Error Handling:**
- Invalid inode → ENOENT
- Offset beyond EOF → Empty read (0 bytes)
- Engram read error → EIO

### Directory Operations

**readdir Implementation:**
```rust
fn readdir(
    &mut self,
    _req: &Request<'_>,
    ino: u64,
    fh: u64,
    offset: i64,
    mut reply: ReplyDirectory,
) {
    // 1. Lookup directory path
    // 2. Add "." and ".." entries
    // 3. List children from manifest
    // 4. Return entries with inodes and types
}
```

**Entry Order:**
1. `.` (current directory)
2. `..` (parent directory)
3. Children (sorted alphabetically)

**Performance:**
- O(1) lookup via inverted index
- O(N) iteration over children (N = # children)
- No hierarchical traversal (flat manifest)

### Statfs (Filesystem Statistics)

**Reported Statistics:**
```rust
pub struct StatfsInfo {
    blocks: u64,      // Total blocks (engram size / block_size)
    bfree: u64,       // Free blocks (0 - read-only)
    bavail: u64,      // Available blocks (0 - read-only)
    files: u64,       // Total files (from manifest)
    ffree: u64,       // Free inodes (0 - read-only)
    bsize: u32,       // Block size (4096)
    namelen: u32,     // Max filename length (255)
    frsize: u32,      // Fragment size (4096)
}
```

**Notes:**
- `blocks` calculated from engram file size
- `bfree` and `bavail` are 0 (read-only filesystem)
- `files` counts all files in manifest
- `ffree` is 0 (no more files can be added via FUSE)

## Performance Considerations

### Benchmarks

**Read Performance:**
- Sequential read: ~100-200 MB/s (single-threaded)
- Random read: ~10-50 MB/s (depends on chunk caching)
- Small file read (<4KB): ~1-5 ms per file
- Large file read (>1MB): ~5-20 ms + data transfer time

**Directory Listing:**
- Small directory (<100 files): ~1-5 ms
- Large directory (>1000 files): ~10-50 ms
- Nested directory traversal: O(depth) × directory_list_time

**Factors:**
- Disk I/O (engram file read)
- Chunk decoding (VSA unbundling)
- Correction application (bit-perfect reconstruction)
- LRU cache hit rate

### Optimization Tips

**1. Enable LRU Caching:**
```rust
let options = MountOptions {
    cache_size: 1000,  // Cache 1000 sub-engrams
    ..Default::default()
};
```

**2. Use Hierarchical Engrams:**
- Reduces memory footprint
- Improves cache hit rate
- Faster directory listings

**3. Avoid Repeated Mounts:**
- Keep engram mounted for duration of work
- Unmounting/remounting is expensive (full reload)

**4. Batch Operations:**
- Use `rsync` or `tar` for bulk extraction
- Reduces FUSE call overhead

## Troubleshooting

### Common Issues

**1. Permission Denied on Mount**

```
Error: FUSE mount failed: Permission denied
```

**Solutions:**
- Run with `sudo`
- Add user to `fuse` group: `sudo usermod -a -G fuse $USER`
- Enable `user_allow_other` in `/etc/fuse.conf`:
  ```
  # /etc/fuse.conf
  user_allow_other
  ```
- Logout and login for group changes to take effect

**2. Device or Resource Busy**

```
Error: umount: /mnt/embrfs: target is busy
```

**Solutions:**
- Close all programs accessing mountpoint
- Check with `lsof /mnt/embrfs` or `fuser -m /mnt/embrfs`
- Force unmount: `sudo umount -l /mnt/embrfs` (lazy unmount)

**3. File Not Found (ENOENT)**

```
Error: cat: /mnt/embrfs/file.txt: No such file or directory
```

**Causes:**
- File not in engram (check with `ls -R /mnt/embrfs`)
- Path case mismatch (Linux is case-sensitive)
- File marked as deleted (soft delete in manifest)

**Debug:**
- Check manifest: `embrfs info filesystem.engram`
- Verify path: `embrfs list filesystem.engram | grep file.txt`

**4. Input/Output Error (EIO)**

```
Error: cat: /mnt/embrfs/file.txt: Input/output error
```

**Causes:**
- Corrupted engram file
- Hash verification failure (data mismatch)
- Disk read error

**Debug:**
- Verify engram integrity: `embrfs verify filesystem.engram`
- Check disk errors: `dmesg | tail`
- Re-encode engram from original source

**5. Operation Not Supported (ENOSYS)**

```
Error: readlink: /mnt/embrfs/link: Function not implemented
```

**Cause:** Symbolic links not supported in EmbrFS.

**Workaround:** Copy regular files only, skip symlinks.

### Debug Mode

**Enable FUSE debugging:**
```rust
let options = MountOptions {
    debug: true,  // Print all FUSE operations to stderr
    ..Default::default()
};
mount_embrfs(fs, mountpoint, &options)?;
```

**Output:**
```
FUSE: lookup(parent=1, name="file.txt")
FUSE: getattr(ino=42)
FUSE: open(ino=42, flags=O_RDONLY)
FUSE: read(ino=42, offset=0, size=4096)
FUSE: release(ino=42)
```

### Logging

**Enable library logging:**
```rust
env_logger::Builder::from_default_env()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

**Environment variable:**
```bash
RUST_LOG=embeddenator_fs=debug embrfs mount filesystem.engram /mnt/embrfs
```

## Advanced Usage

### Mount Options

```rust
pub struct MountOptions {
    pub debug: bool,           // Enable FUSE debug output
    pub auto_unmount: bool,    // Unmount on process exit
    pub allow_root: bool,      // Allow root to access filesystem
    pub allow_other: bool,     // Allow all users to access filesystem
    pub cache_size: usize,     // LRU cache size for sub-engrams
    pub read_only: bool,       // Force read-only (redundant, always true)
}
```

**Example:**
```rust
let options = MountOptions {
    debug: false,
    auto_unmount: true,
    allow_root: false,
    allow_other: true,         // Requires user_allow_other in fuse.conf
    cache_size: 500,
    read_only: true,
};

mount_embrfs(fs, mountpoint, &options)?;
```

### Programmatic Unmount

```rust
use embeddenator_fs::fuse::unmount_embrfs;

// Unmount filesystem
unmount_embrfs(Path::new("/mnt/embrfs"))?;
```

**Note:** Unmounting may fail if filesystem is busy (files open, processes in directory).

### Background Mounting

```rust
use std::thread;

let fs = EmbrFS::load("filesystem.engram")?;
let mountpoint = Path::new("/mnt/embrfs");

// Mount in background thread
let handle = thread::spawn(move || {
    mount_embrfs(fs, mountpoint, &[]).expect("Mount failed");
});

// Do work while mounted
// ...

// Wait for unmount
handle.join().unwrap();
```

### Integration with systemd

**Create systemd service:**
```ini
# /etc/systemd/system/embrfs@.service
[Unit]
Description=EmbrFS Mount for %I
After=network.target

[Service]
Type=forking
ExecStart=/usr/local/bin/embrfs mount /data/%i.engram /mnt/%i
ExecStop=/bin/fusermount -u /mnt/%i
Restart=on-failure
User=embrfs

[Install]
WantedBy=multi-user.target
```

**Usage:**
```bash
# Enable and start
sudo systemctl enable embrfs@filesystem
sudo systemctl start embrfs@filesystem

# Check status
sudo systemctl status embrfs@filesystem

# Stop and disable
sudo systemctl stop embrfs@filesystem
sudo systemctl disable embrfs@filesystem
```

## Security Considerations

### Attack Surface

**Read-Only Nature:**
-  No write-based attacks (EROFS for all write operations)
-  No file creation/deletion
-  No permission changes

**Path Traversal:**
-  Mitigated by path normalization
-  All paths resolved relative to engram root
-  No `../..` escape possible

**Resource Exhaustion:**
-  Large engrams can consume memory (use hierarchical mode)
-  Repeated reads without caching can be slow (enable LRU cache)
-  No amplification attacks (output bounded by engram size)

### Recommendations

**Production Deployments:**
1. Mount with `allow_other=false` (default) to restrict access to mounting user
2. Use dedicated user for EmbrFS mounts (e.g., `embrfs` user)
3. Set restrictive permissions on engram files (0600)
4. Monitor for excessive FUSE operations (rate limiting)
5. Use hierarchical engrams to bound memory usage

**Untrusted Engrams:**
1. Verify hash before mounting: `embrfs verify untrusted.engram`
2. Mount in isolated namespace (container, VM)
3. Limit resources (ulimit, cgroups)
4. Scan extracted files with antivirus before use

**Network Exposure:**
-  Do NOT expose FUSE mountpoints over NFS or CIFS (performance issues)
-  Use API-level access for network sharing
-  Consider read-only HTTP server for web access

## Platform Support

| Platform        | FUSE Support | Status | Notes                          |
|-----------------|--------------|--------|--------------------------------|
| Linux           |  libfuse3  | Full   | Primary target, well-tested    |
| macOS           |  OSXFUSE   | Untested | Should work, not verified     |
| Windows         |  WinFsp    | No     | Future work, requires porting  |
| FreeBSD         |  fusefs    | Untested | Should work, not verified     |

**Recommendations:**
- **Linux:** Use libfuse3 (recommended)
- **macOS:** Install OSXFUSE, test thoroughly before production use
- **Windows:** Not supported, use API-level extraction instead

## Alternatives to FUSE

If FUSE is not available or suitable:

**1. API-Level Extraction:**
```rust
let fs = EmbrFS::load("filesystem.engram")?;
fs.extract_all("/output/dir")?;
```

**2. HTTP Server:**
```rust
// Pseudo-code for future HTTP server
let fs = EmbrFS::load("filesystem.engram")?;
let server = HttpServer::new(fs);
server.listen("0.0.0.0:8080")?;
```

**3. Archive Conversion:**
```bash
# Extract to tar (via FUSE)
embrfs mount filesystem.engram /mnt/embrfs
tar czf filesystem.tar.gz -C /mnt/embrfs .
fusermount -u /mnt/embrfs

# Or direct API (future feature)
embrfs export filesystem.engram --format tar.gz --output filesystem.tar.gz
```

## Future Enhancements

**Planned Features:**
- Copy-on-write support (immutable history)
- Overlay mode (mount engram + writable overlay)
- Snapshot browsing (mount specific engram version)
- Network filesystem support (custom protocol)

**Non-Goals:**
- Writable FUSE operations (conflicts with immutability)
- POSIX full compliance (simplified model by design)
- High-performance databases (not target use case)

## References

- [FUSE Documentation](https://www.kernel.org/doc/html/latest/filesystems/fuse.html)
- [libfuse GitHub](https://github.com/libfuse/libfuse)
- [fuser crate](https://docs.rs/fuser/) - Rust FUSE bindings used by EmbrFS
- [Writing a FUSE Filesystem](https://www.cs.hmc.edu/~geoff/classes/hmc.cs135.201001/homework/fuse/fuse_doc.html)

---

**Document Maintenance:**
- Update after FUSE API changes
- Add benchmark results from real-world usage
- Document known bugs and workarounds
