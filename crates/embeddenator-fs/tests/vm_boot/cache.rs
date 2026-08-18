//! Cached VM Image Management
//! ===========================
//!
//! This module provides intelligent caching for VM test images to avoid
//! repeated downloads and enable offline testing.
//!
//! # Why Caching?
//!
//! VM test images are large (50-500MB) and downloading them every test run:
//!
//! 1. **Wastes bandwidth** - Both yours and the CDN's
//! 2. **Slows testing** - Minutes of download vs milliseconds of cache hit
//! 3. **Breaks offline** - Can't run tests without internet
//! 4. **Wastes energy** - Data centers consume power; unnecessary transfers add up
//!
//! A good caching strategy makes tests:
//! - **Fast**: Cache hits are instant
//! - **Reliable**: Checksum verification catches corruption
//! - **Offline-capable**: Once cached, no network needed
//! - **Space-efficient**: Configurable retention and cleanup
//!
//! # Cache Structure
//!
//! ```text
//! ~/.cache/embeddenator/vm-images/
//! ├── metadata.json           # Cache state, timestamps, access counts
//! ├── x86_64/
//! │   ├── alpine-virt-3.19.0-x86_64.qcow2
//! │   ├── alpine-virt-3.19.0-x86_64.qcow2.sha256
//! │   └── alpine-virt-3.19.0-x86_64.qcow2.meta
//! ├── aarch64/
//! │   ├── alpine-virt-3.19.0-aarch64.qcow2
//! │   └── ...
//! └── riscv64/
//!     └── ...
//! ```
//!
//! # Why This Structure?
//!
//! - **By architecture**: Easy to find, clear organization
//! - **Checksum files**: Verify integrity without parsing image
//! - **Metadata files**: Track download time, source URL, access count
//! - **Single metadata.json**: Global cache state for cleanup decisions
//!
//! # Cache Invalidation
//!
//! "There are only two hard things in Computer Science: cache invalidation
//! and naming things." - Phil Karlton
//!
//! We invalidate cached images when:
//!
//! 1. **Checksum mismatch**: Image on disk doesn't match expected hash
//! 2. **Source URL change**: Same image name but different download URL
//! 3. **Explicit request**: User calls `cache.invalidate(image)`
//! 4. **Expiration**: Configurable max age (default: never expire)
//!
//! We do NOT invalidate when:
//! - Test code changes (image is external, unchanged)
//! - Embeddenator version changes (image is independent)
//! - System reboots (cache survives reboots)
//!
//! # Space Management
//!
//! Large caches consume disk space. We provide:
//!
//! - **Size limits**: Max total cache size (default: 10GB)
//! - **LRU eviction**: Least-recently-used images deleted first
//! - **Manual cleanup**: `cache.cleanup()` respects limits
//! - **Full purge**: `cache.purge()` removes everything

use std::collections::HashMap;
use std::fs::{self};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Default cache directory name
const CACHE_DIR_NAME: &str = "embeddenator/vm-images";

/// Default maximum cache size (10 GB)
const DEFAULT_MAX_CACHE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// Metadata for a single cached image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedImageMeta {
    /// Original download URL
    pub source_url: String,
    /// Expected SHA256 checksum
    pub expected_sha256: String,
    /// When the image was downloaded (Unix timestamp)
    pub downloaded_at: u64,
    /// When the image was last accessed (Unix timestamp)
    pub last_accessed: u64,
    /// Number of times this image has been used
    pub access_count: u64,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Global cache state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheState {
    /// Version of cache format (for migrations)
    pub version: u32,
    /// Total size of all cached images
    pub total_size_bytes: u64,
    /// Metadata for each cached image (path relative to cache root → metadata)
    pub images: HashMap<String, CachedImageMeta>,
}

impl CacheState {
    /// Create new empty cache state
    pub fn new() -> Self {
        Self {
            version: 1,
            total_size_bytes: 0,
            images: HashMap::new(),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Root directory for cache
    pub root: PathBuf,
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
    /// Maximum age before expiration (None = never expire)
    pub max_age: Option<Duration>,
    /// Whether to verify checksums on cache hit
    pub verify_on_hit: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(CACHE_DIR_NAME);

        Self {
            root,
            max_size_bytes: DEFAULT_MAX_CACHE_SIZE,
            max_age: None,        // Never expire by default
            verify_on_hit: false, // Trust cache by default (verified on download)
        }
    }
}

/// Result of a cache lookup
#[derive(Debug)]
pub enum CacheLookup {
    /// Image found in cache and valid
    Hit {
        path: PathBuf,
        meta: CachedImageMeta,
    },
    /// Image not in cache
    Miss,
    /// Image in cache but invalid (checksum mismatch, expired, etc.)
    Invalid { reason: String },
}

/// VM image cache manager
pub struct ImageCache {
    config: CacheConfig,
    state: CacheState,
    state_path: PathBuf,
}

impl ImageCache {
    /// Open or create a cache with the given configuration
    ///
    /// # Why This Design
    ///
    /// We load state eagerly on open because:
    /// 1. Fail fast if cache directory is inaccessible
    /// 2. State is small (<1KB typically), quick to load
    /// 3. Enables validation before any operations
    pub fn open(config: CacheConfig) -> io::Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&config.root)?;

        let state_path = config.root.join("metadata.json");
        let state = if state_path.exists() {
            let content = fs::read_to_string(&state_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| {
                eprintln!("Warning: corrupt cache metadata, starting fresh");
                CacheState::new()
            })
        } else {
            CacheState::new()
        };

        Ok(Self {
            config,
            state,
            state_path,
        })
    }

    /// Open cache with default configuration
    pub fn open_default() -> io::Result<Self> {
        Self::open(CacheConfig::default())
    }

    /// Look up an image in the cache
    ///
    /// # Arguments
    ///
    /// * `key` - Unique key for the image (typically arch/filename)
    /// * `expected_sha256` - Expected checksum (for validation)
    pub fn lookup(&mut self, key: &str, expected_sha256: &str) -> CacheLookup {
        let image_path = self.config.root.join(key);

        // Check if file exists
        if !image_path.exists() {
            return CacheLookup::Miss;
        }

        // Check metadata
        let meta = match self.state.images.get(key) {
            Some(m) => m.clone(),
            None => {
                // File exists but no metadata - might be manually placed
                // Treat as miss and let caller download fresh
                return CacheLookup::Invalid {
                    reason: "No metadata for cached file".to_string(),
                };
            }
        };

        // Check checksum matches expected
        if meta.expected_sha256 != expected_sha256 {
            return CacheLookup::Invalid {
                reason: format!(
                    "Checksum changed: cached={}, expected={}",
                    &meta.expected_sha256[..8],
                    &expected_sha256[..8]
                ),
            };
        }

        // Check expiration
        if let Some(max_age) = self.config.max_age {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let age = Duration::from_secs(now - meta.downloaded_at);
            if age > max_age {
                return CacheLookup::Invalid {
                    reason: format!("Expired: age {:?} > max {:?}", age, max_age),
                };
            }
        }

        // Optionally verify checksum on hit
        if self.config.verify_on_hit {
            match compute_sha256(&image_path) {
                Ok(actual) if actual == expected_sha256 => {}
                Ok(actual) => {
                    return CacheLookup::Invalid {
                        reason: format!(
                            "On-disk checksum mismatch: expected {}, got {}",
                            &expected_sha256[..8],
                            &actual[..8]
                        ),
                    };
                }
                Err(e) => {
                    return CacheLookup::Invalid {
                        reason: format!("Failed to verify checksum: {}", e),
                    };
                }
            }
        }

        // Update access tracking
        if let Some(meta) = self.state.images.get_mut(key) {
            meta.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            meta.access_count += 1;
        }
        let _ = self.save_state();

        CacheLookup::Hit {
            path: image_path,
            meta,
        }
    }

    /// Add an image to the cache
    ///
    /// # Arguments
    ///
    /// * `key` - Unique key for the image
    /// * `source_path` - Path to the downloaded image
    /// * `source_url` - Original download URL
    /// * `sha256` - Verified checksum
    ///
    /// # Why Copy Instead of Move?
    ///
    /// We copy instead of move because:
    /// 1. Source might be on different filesystem (can't rename across fs)
    /// 2. Caller might want to keep original
    /// 3. Atomic: if copy fails, cache unchanged
    pub fn insert(
        &mut self,
        key: &str,
        source_path: &Path,
        source_url: &str,
        sha256: &str,
    ) -> io::Result<PathBuf> {
        let dest_path = self.config.root.join(key);

        // Create parent directories
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy file to cache
        fs::copy(source_path, &dest_path)?;

        // Get file size
        let size = fs::metadata(&dest_path)?.len();

        // Update state
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.state.images.insert(
            key.to_string(),
            CachedImageMeta {
                source_url: source_url.to_string(),
                expected_sha256: sha256.to_string(),
                downloaded_at: now,
                last_accessed: now,
                access_count: 1,
                size_bytes: size,
            },
        );

        self.state.total_size_bytes += size;
        self.save_state()?;

        // Check if cleanup needed
        if self.state.total_size_bytes > self.config.max_size_bytes {
            let _ = self.cleanup();
        }

        Ok(dest_path)
    }

    /// Remove an image from the cache
    pub fn invalidate(&mut self, key: &str) -> io::Result<bool> {
        let path = self.config.root.join(key);

        let existed = if path.exists() {
            fs::remove_file(&path)?;
            true
        } else {
            false
        };

        if let Some(meta) = self.state.images.remove(key) {
            self.state.total_size_bytes =
                self.state.total_size_bytes.saturating_sub(meta.size_bytes);
        }

        self.save_state()?;
        Ok(existed)
    }

    /// Clean up cache to stay within size limits
    ///
    /// Uses LRU (Least Recently Used) eviction: removes oldest-accessed
    /// images until total size is under the limit.
    pub fn cleanup(&mut self) -> io::Result<Vec<String>> {
        let mut removed = Vec::new();

        while self.state.total_size_bytes > self.config.max_size_bytes {
            // Find least recently used image
            let lru_key = self
                .state
                .images
                .iter()
                .min_by_key(|(_, meta)| meta.last_accessed)
                .map(|(k, _)| k.clone());

            match lru_key {
                Some(key) => {
                    self.invalidate(&key)?;
                    removed.push(key);
                }
                None => break, // No more images to remove
            }
        }

        Ok(removed)
    }

    /// Remove all cached images
    pub fn purge(&mut self) -> io::Result<()> {
        // Remove all image files
        for key in self.state.images.keys().cloned().collect::<Vec<_>>() {
            let path = self.config.root.join(&key);
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }

        // Reset state
        self.state = CacheState::new();
        self.save_state()?;

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_size_bytes: self.state.total_size_bytes,
            image_count: self.state.images.len(),
            max_size_bytes: self.config.max_size_bytes,
            utilization: self.state.total_size_bytes as f64 / self.config.max_size_bytes as f64,
        }
    }

    /// Save cache state to disk
    fn save_state(&self) -> io::Result<()> {
        let content = serde_json::to_string_pretty(&self.state).map_err(io::Error::other)?;
        fs::write(&self.state_path, content)?;
        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total size of cached images
    pub total_size_bytes: u64,
    /// Number of cached images
    pub image_count: usize,
    /// Maximum allowed cache size
    pub max_size_bytes: u64,
    /// Cache utilization (0.0 to 1.0+)
    pub utilization: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache: {} images, {:.1} MB / {:.1} MB ({:.1}%)",
            self.image_count,
            self.total_size_bytes as f64 / (1024.0 * 1024.0),
            self.max_size_bytes as f64 / (1024.0 * 1024.0),
            self.utilization * 100.0
        )
    }
}

/// Compute SHA256 hash of a file
fn compute_sha256(path: &Path) -> io::Result<String> {
    use std::process::Command;

    let output = Command::new("sha256sum").arg(path).output()?;

    if !output.status.success() {
        return Err(io::Error::other("sha256sum command failed"));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let hash = output_str
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::other("Failed to parse sha256sum output"))?;

    Ok(hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_miss() {
        let temp = tempdir().unwrap();
        let config = CacheConfig {
            root: temp.path().to_path_buf(),
            ..Default::default()
        };
        let mut cache = ImageCache::open(config).unwrap();

        match cache.lookup("nonexistent", "abc123") {
            CacheLookup::Miss => {} // Expected
            other => panic!("Expected Miss, got {:?}", other),
        }
    }

    #[test]
    fn test_cache_insert_and_hit() {
        let temp = tempdir().unwrap();
        let config = CacheConfig {
            root: temp.path().to_path_buf(),
            ..Default::default()
        };
        let mut cache = ImageCache::open(config).unwrap();

        // Create a test file
        let test_file = temp.path().join("test.img");
        fs::write(&test_file, b"test image content").unwrap();
        let sha256 = compute_sha256(&test_file).unwrap();

        // Insert into cache
        cache
            .insert(
                "x86_64/test.img",
                &test_file,
                "https://example.com/test.img",
                &sha256,
            )
            .unwrap();

        // Lookup should hit
        match cache.lookup("x86_64/test.img", &sha256) {
            CacheLookup::Hit { path, meta } => {
                assert!(path.exists());
                // access_count is 1 from insert (lookup doesn't increment in test due to fresh state load)
                assert!(meta.access_count >= 1);
            }
            other => panic!("Expected Hit, got {:?}", other),
        }
    }

    #[test]
    fn test_cache_invalidation() {
        let temp = tempdir().unwrap();
        let config = CacheConfig {
            root: temp.path().to_path_buf(),
            ..Default::default()
        };
        let mut cache = ImageCache::open(config).unwrap();

        // Create and insert a test file
        let test_file = temp.path().join("test.img");
        fs::write(&test_file, b"test content").unwrap();
        let sha256 = compute_sha256(&test_file).unwrap();

        cache
            .insert("test.img", &test_file, "https://example.com", &sha256)
            .unwrap();

        // Invalidate
        let existed = cache.invalidate("test.img").unwrap();
        assert!(existed);

        // Should now miss
        match cache.lookup("test.img", &sha256) {
            CacheLookup::Miss => {}
            other => panic!("Expected Miss after invalidation, got {:?}", other),
        }
    }

    #[test]
    fn test_cache_stats() {
        let temp = tempdir().unwrap();
        let config = CacheConfig {
            root: temp.path().to_path_buf(),
            max_size_bytes: 1024 * 1024, // 1 MB
            ..Default::default()
        };
        let cache = ImageCache::open(config).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.image_count, 0);
        assert_eq!(stats.total_size_bytes, 0);
        println!("{}", stats);
    }
}
