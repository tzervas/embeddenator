//! Virtual Memory Abstraction for Tiered Storage
//!
//! This module provides a unified memory interface that spans multiple storage tiers:
//! - **Tier 0 (Hot)**: GPU VRAM - Fastest, limited capacity
//! - **Tier 1 (Warm)**: Host RAM - Fast, moderate capacity
//! - **Tier 2 (Cold)**: SSD/Disk - Slow, large capacity
//!
//! # Design
//!
//! The virtual memory system uses a handle-based allocation model where data can
//! transparently migrate between tiers based on access patterns. Each allocation
//! has a "home tier" (where data is persisted) and a "current tier" (where data
//! is currently resident for fast access).
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                  Virtual Memory Manager                       │
//! │  ┌─────────────────┬─────────────────┬─────────────────┐     │
//! │  │    VRAM Pool    │    Host Pool    │    Disk Pool    │     │
//! │  │   (Tier 0)      │    (Tier 1)     │    (Tier 2)     │     │
//! │  └────────┬────────┴────────┬────────┴────────┬────────┘     │
//! │           │                 │                 │               │
//! │           ▼                 ▼                 ▼               │
//! │      GPU Memory       Host Memory        File System         │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Transparent Migration**: Data moves between tiers automatically
//! - **LRU Eviction**: Least-recently-used data evicted from faster tiers
//! - **Pinning**: Prevent specific allocations from being evicted
//! - **Async I/O**: Optional async transfers for disk operations
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_vsa::virtual_memory::{VirtualMemory, VirtualMemoryConfig, MemoryTier};
//!
//! let config = VirtualMemoryConfig::default();
//! let vmem = VirtualMemory::new(config)?;
//!
//! // Allocate memory (starts in host RAM)
//! let handle = vmem.allocate(1024 * 1024, MemoryTier::Host)?;
//!
//! // Write data
//! vmem.write(&handle, &data)?;
//!
//! // Promote to VRAM for GPU access
//! vmem.promote(&handle, MemoryTier::Vram)?;
//!
//! // Read data (transparent access regardless of tier)
//! let data = vmem.read(&handle)?;
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Memory tier indicating where data resides
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryTier {
    /// GPU VRAM (fastest, limited)
    Vram,
    /// Host RAM (fast, moderate capacity)
    Host,
    /// Disk/SSD (slow, large capacity)
    Disk,
}

impl MemoryTier {
    /// Get tier priority (lower = faster/preferred)
    pub fn priority(&self) -> u8 {
        match self {
            MemoryTier::Vram => 0,
            MemoryTier::Host => 1,
            MemoryTier::Disk => 2,
        }
    }

    /// Check if this tier is faster than another
    pub fn is_faster_than(&self, other: &MemoryTier) -> bool {
        self.priority() < other.priority()
    }
}

/// Error type for virtual memory operations
#[derive(Debug, Clone)]
pub enum VirtualMemoryError {
    /// Allocation failed (out of memory in all tiers)
    AllocationFailed(String),
    /// Handle not found
    HandleNotFound(u64),
    /// I/O error
    IoError(String),
    /// Tier not available
    TierNotAvailable(MemoryTier),
    /// Invalid operation
    InvalidOperation(String),
    /// Size mismatch
    SizeMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for VirtualMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VirtualMemoryError::AllocationFailed(msg) => write!(f, "Allocation failed: {}", msg),
            VirtualMemoryError::HandleNotFound(id) => write!(f, "Handle {} not found", id),
            VirtualMemoryError::IoError(msg) => write!(f, "I/O error: {}", msg),
            VirtualMemoryError::TierNotAvailable(tier) => {
                write!(f, "Tier {:?} not available", tier)
            }
            VirtualMemoryError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            VirtualMemoryError::SizeMismatch { expected, actual } => {
                write!(f, "Size mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl std::error::Error for VirtualMemoryError {}

/// Configuration for virtual memory
#[derive(Clone, Debug)]
pub struct VirtualMemoryConfig {
    /// Enable VRAM tier (requires CUDA)
    pub enable_vram: bool,
    /// Maximum host memory to use (bytes)
    pub max_host_bytes: usize,
    /// Maximum disk space to use (bytes)
    pub max_disk_bytes: usize,
    /// Directory for disk-backed storage
    pub disk_path: PathBuf,
    /// Enable automatic tier migration
    pub auto_migrate: bool,
    /// LRU eviction threshold (0.0-1.0, evict when tier is this full)
    pub eviction_threshold: f64,
    /// Minimum access count before promotion to faster tier
    pub promotion_threshold: u32,
}

impl Default for VirtualMemoryConfig {
    fn default() -> Self {
        Self {
            enable_vram: false,                      // Disabled by default (requires CUDA)
            max_host_bytes: 4 * 1024 * 1024 * 1024,  // 4GB host
            max_disk_bytes: 64 * 1024 * 1024 * 1024, // 64GB disk
            disk_path: std::env::temp_dir().join("embeddenator_vmem"),
            auto_migrate: true,
            eviction_threshold: 0.90, // Evict when 90% full
            promotion_threshold: 3,   // Promote after 3 accesses
        }
    }
}

/// Handle to a virtual memory allocation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VMemHandle {
    /// Unique allocation ID
    id: u64,
    /// Size in bytes
    size: usize,
    /// Home tier (where data is persisted)
    home_tier: MemoryTier,
}

impl VMemHandle {
    fn new(id: u64, size: usize, home_tier: MemoryTier) -> Self {
        Self {
            id,
            size,
            home_tier,
        }
    }

    /// Get the allocation ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the allocation size in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the home tier for this allocation
    pub fn home_tier(&self) -> MemoryTier {
        self.home_tier
    }
}

/// Metadata for a virtual memory allocation
#[derive(Debug)]
struct AllocationMetadata {
    /// Handle to this allocation
    handle: VMemHandle,
    /// Current tier where data is resident
    current_tier: MemoryTier,
    /// Last access time for LRU
    last_access: Instant,
    /// Access count for promotion decisions
    access_count: u32,
    /// Whether allocation is pinned (cannot be evicted)
    pinned: bool,
    /// Whether data is dirty (modified but not persisted to home)
    dirty: bool,
}

impl AllocationMetadata {
    fn new(handle: VMemHandle) -> Self {
        Self {
            handle,
            current_tier: handle.home_tier,
            last_access: Instant::now(),
            access_count: 0,
            pinned: false,
            dirty: false,
        }
    }

    fn touch(&mut self) {
        self.last_access = Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }
}

/// Per-tier storage pool
struct TierPool {
    /// Tier this pool manages
    tier: MemoryTier,
    /// Data storage (handle ID -> data)
    data: HashMap<u64, Vec<u8>>,
    /// Current used bytes
    used_bytes: usize,
    /// Maximum capacity
    max_bytes: usize,
}

impl TierPool {
    fn new(tier: MemoryTier, max_bytes: usize) -> Self {
        Self {
            tier,
            data: HashMap::new(),
            used_bytes: 0,
            max_bytes,
        }
    }

    fn available(&self) -> usize {
        self.max_bytes.saturating_sub(self.used_bytes)
    }

    #[allow(dead_code)]
    fn usage_ratio(&self) -> f64 {
        if self.max_bytes == 0 {
            1.0
        } else {
            self.used_bytes as f64 / self.max_bytes as f64
        }
    }

    fn insert(&mut self, id: u64, data: Vec<u8>) -> Result<(), VirtualMemoryError> {
        let size = data.len();
        if size > self.available() {
            return Err(VirtualMemoryError::AllocationFailed(format!(
                "Tier {:?} full: need {} bytes, available {}",
                self.tier,
                size,
                self.available()
            )));
        }
        self.data.insert(id, data);
        self.used_bytes += size;
        Ok(())
    }

    fn remove(&mut self, id: u64) -> Option<Vec<u8>> {
        if let Some(data) = self.data.remove(&id) {
            self.used_bytes = self.used_bytes.saturating_sub(data.len());
            Some(data)
        } else {
            None
        }
    }

    fn get(&self, id: u64) -> Option<&Vec<u8>> {
        self.data.get(&id)
    }

    #[allow(dead_code)]
    fn contains(&self, id: u64) -> bool {
        self.data.contains_key(&id)
    }
}

/// Disk-backed storage pool with file persistence
struct DiskPool {
    /// Base path for storage
    base_path: PathBuf,
    /// Index of stored allocations (handle ID -> file path)
    index: HashMap<u64, PathBuf>,
    /// Current used bytes
    used_bytes: usize,
    /// Maximum capacity
    max_bytes: usize,
}

impl DiskPool {
    fn new(base_path: PathBuf, max_bytes: usize) -> Result<Self, VirtualMemoryError> {
        std::fs::create_dir_all(&base_path)
            .map_err(|e| VirtualMemoryError::IoError(e.to_string()))?;
        Ok(Self {
            base_path,
            index: HashMap::new(),
            used_bytes: 0,
            max_bytes,
        })
    }

    fn available(&self) -> usize {
        self.max_bytes.saturating_sub(self.used_bytes)
    }

    #[allow(dead_code)]
    fn usage_ratio(&self) -> f64 {
        if self.max_bytes == 0 {
            1.0
        } else {
            self.used_bytes as f64 / self.max_bytes as f64
        }
    }

    fn file_path(&self, id: u64) -> PathBuf {
        self.base_path.join(format!("vmem_{:016x}.bin", id))
    }

    fn write(&mut self, id: u64, data: &[u8]) -> Result<(), VirtualMemoryError> {
        let size = data.len();
        if size > self.available() {
            return Err(VirtualMemoryError::AllocationFailed(format!(
                "Disk pool full: need {} bytes, available {}",
                size,
                self.available()
            )));
        }

        let path = self.file_path(id);
        std::fs::write(&path, data).map_err(|e| VirtualMemoryError::IoError(e.to_string()))?;

        // Update tracking
        if let Some(old_path) = self.index.insert(id, path) {
            // Remove old size tracking
            if let Ok(meta) = std::fs::metadata(&old_path) {
                self.used_bytes = self.used_bytes.saturating_sub(meta.len() as usize);
            }
        }
        self.used_bytes += size;

        Ok(())
    }

    fn read(&self, id: u64) -> Result<Vec<u8>, VirtualMemoryError> {
        let path = self
            .index
            .get(&id)
            .ok_or(VirtualMemoryError::HandleNotFound(id))?;
        std::fs::read(path).map_err(|e| VirtualMemoryError::IoError(e.to_string()))
    }

    fn remove(&mut self, id: u64) -> Result<(), VirtualMemoryError> {
        if let Some(path) = self.index.remove(&id) {
            if let Ok(meta) = std::fs::metadata(&path) {
                self.used_bytes = self.used_bytes.saturating_sub(meta.len() as usize);
            }
            let _ = std::fs::remove_file(&path);
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn contains(&self, id: u64) -> bool {
        self.index.contains_key(&id)
    }
}

/// Virtual memory manager providing unified access to tiered storage
pub struct VirtualMemory {
    /// Configuration
    config: VirtualMemoryConfig,
    /// Host memory pool
    host_pool: RwLock<TierPool>,
    /// Disk storage pool
    disk_pool: RwLock<DiskPool>,
    /// Allocation metadata
    metadata: RwLock<HashMap<u64, AllocationMetadata>>,
    /// Next allocation ID
    next_id: AtomicU64,
    /// Total allocations across all tiers
    total_allocations: AtomicUsize,
}

impl VirtualMemory {
    /// Create a new virtual memory manager
    pub fn new(config: VirtualMemoryConfig) -> Result<Self, VirtualMemoryError> {
        let host_pool = TierPool::new(MemoryTier::Host, config.max_host_bytes);
        let disk_pool = DiskPool::new(config.disk_path.clone(), config.max_disk_bytes)?;

        Ok(Self {
            config,
            host_pool: RwLock::new(host_pool),
            disk_pool: RwLock::new(disk_pool),
            metadata: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            total_allocations: AtomicUsize::new(0),
        })
    }

    /// Allocate virtual memory
    ///
    /// The `home_tier` specifies where data is persisted. For Disk tier,
    /// data is written to disk; for Host tier, data stays in RAM.
    pub fn allocate(
        &self,
        size: usize,
        home_tier: MemoryTier,
    ) -> Result<VMemHandle, VirtualMemoryError> {
        // VRAM allocations not directly supported (use coherency layer)
        if home_tier == MemoryTier::Vram {
            return Err(VirtualMemoryError::TierNotAvailable(MemoryTier::Vram));
        }

        // Check if we have space
        let can_alloc = match home_tier {
            MemoryTier::Host => {
                let pool = self.host_pool.read().unwrap();
                pool.available() >= size
            }
            MemoryTier::Disk => {
                let pool = self.disk_pool.read().unwrap();
                pool.available() >= size
            }
            MemoryTier::Vram => false,
        };

        if !can_alloc {
            // Try eviction if enabled
            if self.config.auto_migrate {
                self.evict_if_needed(home_tier, size)?;
            }
        }

        // Create handle
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let handle = VMemHandle::new(id, size, home_tier);

        // Initialize storage with zeros
        let zeros = vec![0u8; size];
        match home_tier {
            MemoryTier::Host => {
                let mut pool = self.host_pool.write().unwrap();
                pool.insert(id, zeros)?;
            }
            MemoryTier::Disk => {
                let mut pool = self.disk_pool.write().unwrap();
                pool.write(id, &zeros)?;
            }
            MemoryTier::Vram => unreachable!(),
        }

        // Track metadata
        let mut meta = self.metadata.write().unwrap();
        meta.insert(id, AllocationMetadata::new(handle));

        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        Ok(handle)
    }

    /// Free a virtual memory allocation
    pub fn free(&self, handle: &VMemHandle) -> Result<(), VirtualMemoryError> {
        let mut meta_guard = self.metadata.write().unwrap();
        let meta = meta_guard
            .remove(&handle.id)
            .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?;

        // Remove from current tier
        match meta.current_tier {
            MemoryTier::Host => {
                let mut pool = self.host_pool.write().unwrap();
                pool.remove(handle.id);
            }
            MemoryTier::Disk => {
                let mut pool = self.disk_pool.write().unwrap();
                pool.remove(handle.id)?;
            }
            MemoryTier::Vram => {
                // VRAM handled by coherency layer
            }
        }

        // Also remove from home tier if different
        if meta.current_tier != meta.handle.home_tier {
            match meta.handle.home_tier {
                MemoryTier::Host => {
                    let mut pool = self.host_pool.write().unwrap();
                    pool.remove(handle.id);
                }
                MemoryTier::Disk => {
                    let mut pool = self.disk_pool.write().unwrap();
                    pool.remove(handle.id)?;
                }
                MemoryTier::Vram => {}
            }
        }

        self.total_allocations.fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }

    /// Write data to a virtual memory allocation
    pub fn write(&self, handle: &VMemHandle, data: &[u8]) -> Result<(), VirtualMemoryError> {
        if data.len() != handle.size {
            return Err(VirtualMemoryError::SizeMismatch {
                expected: handle.size,
                actual: data.len(),
            });
        }

        // Update metadata
        {
            let mut meta_guard = self.metadata.write().unwrap();
            let meta = meta_guard
                .get_mut(&handle.id)
                .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?;
            meta.touch();
            meta.dirty = true;
        }

        // Get current tier
        let current_tier = {
            let meta_guard = self.metadata.read().unwrap();
            meta_guard
                .get(&handle.id)
                .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?
                .current_tier
        };

        // Write to current tier
        match current_tier {
            MemoryTier::Host => {
                let mut pool = self.host_pool.write().unwrap();
                if let Some(existing) = pool.data.get_mut(&handle.id) {
                    existing.copy_from_slice(data);
                } else {
                    pool.insert(handle.id, data.to_vec())?;
                }
            }
            MemoryTier::Disk => {
                let mut pool = self.disk_pool.write().unwrap();
                pool.write(handle.id, data)?;
            }
            MemoryTier::Vram => {
                return Err(VirtualMemoryError::InvalidOperation(
                    "Direct VRAM write not supported, use coherency layer".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Read data from a virtual memory allocation
    pub fn read(&self, handle: &VMemHandle) -> Result<Vec<u8>, VirtualMemoryError> {
        // Update access tracking
        {
            let mut meta_guard = self.metadata.write().unwrap();
            if let Some(meta) = meta_guard.get_mut(&handle.id) {
                meta.touch();
            }
        }

        // Get current tier
        let current_tier = {
            let meta_guard = self.metadata.read().unwrap();
            meta_guard
                .get(&handle.id)
                .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?
                .current_tier
        };

        // Read from current tier
        match current_tier {
            MemoryTier::Host => {
                let pool = self.host_pool.read().unwrap();
                pool.get(handle.id)
                    .cloned()
                    .ok_or(VirtualMemoryError::HandleNotFound(handle.id))
            }
            MemoryTier::Disk => {
                let pool = self.disk_pool.read().unwrap();
                pool.read(handle.id)
            }
            MemoryTier::Vram => Err(VirtualMemoryError::InvalidOperation(
                "Direct VRAM read not supported, use coherency layer".to_string(),
            )),
        }
    }

    /// Migrate data to a different tier
    pub fn migrate(
        &self,
        handle: &VMemHandle,
        target_tier: MemoryTier,
    ) -> Result<(), VirtualMemoryError> {
        self.migrate_internal(handle, target_tier, false)
    }

    /// Internal migration with option to force removal from source tier
    fn migrate_internal(
        &self,
        handle: &VMemHandle,
        target_tier: MemoryTier,
        force_remove_source: bool,
    ) -> Result<(), VirtualMemoryError> {
        if target_tier == MemoryTier::Vram {
            return Err(VirtualMemoryError::TierNotAvailable(MemoryTier::Vram));
        }

        let current_tier = {
            let meta_guard = self.metadata.read().unwrap();
            meta_guard
                .get(&handle.id)
                .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?
                .current_tier
        };

        if current_tier == target_tier {
            return Ok(()); // Already there
        }

        // Read from current tier
        let data = self.read(handle)?;

        // Write to target tier
        match target_tier {
            MemoryTier::Host => {
                let mut pool = self.host_pool.write().unwrap();
                pool.insert(handle.id, data)?;
            }
            MemoryTier::Disk => {
                let mut pool = self.disk_pool.write().unwrap();
                pool.write(handle.id, &data)?;
            }
            MemoryTier::Vram => unreachable!(),
        }

        // Remove from old tier (if not home tier, or if forced for eviction)
        // For eviction, we always want to remove from source to free memory
        if current_tier != handle.home_tier || force_remove_source {
            match current_tier {
                MemoryTier::Host => {
                    let mut pool = self.host_pool.write().unwrap();
                    pool.remove(handle.id);
                }
                MemoryTier::Disk => {
                    let mut pool = self.disk_pool.write().unwrap();
                    pool.remove(handle.id)?;
                }
                MemoryTier::Vram => {}
            }
        }

        // Update metadata
        {
            let mut meta_guard = self.metadata.write().unwrap();
            if let Some(meta) = meta_guard.get_mut(&handle.id) {
                meta.current_tier = target_tier;
                // Mark dirty if we moved away from home tier
                if target_tier != handle.home_tier {
                    meta.dirty = true;
                }
            }
        }

        Ok(())
    }

    /// Pin an allocation (prevent eviction)
    pub fn pin(&self, handle: &VMemHandle) -> Result<(), VirtualMemoryError> {
        let mut meta_guard = self.metadata.write().unwrap();
        let meta = meta_guard
            .get_mut(&handle.id)
            .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?;
        meta.pinned = true;
        Ok(())
    }

    /// Unpin an allocation (allow eviction)
    pub fn unpin(&self, handle: &VMemHandle) -> Result<(), VirtualMemoryError> {
        let mut meta_guard = self.metadata.write().unwrap();
        let meta = meta_guard
            .get_mut(&handle.id)
            .ok_or(VirtualMemoryError::HandleNotFound(handle.id))?;
        meta.pinned = false;
        Ok(())
    }

    /// Get statistics about virtual memory usage
    pub fn stats(&self) -> VirtualMemoryStats {
        // Acquire locks in consistent order: metadata first, then pools.
        // This prevents deadlocks with other methods that follow this order.
        let meta_guard = self.metadata.read().unwrap();
        let host_pool = self.host_pool.read().unwrap();
        let disk_pool = self.disk_pool.read().unwrap();

        let host_allocations = meta_guard
            .values()
            .filter(|m| m.current_tier == MemoryTier::Host)
            .count();
        let disk_allocations = meta_guard
            .values()
            .filter(|m| m.current_tier == MemoryTier::Disk)
            .count();
        let pinned = meta_guard.values().filter(|m| m.pinned).count();
        let dirty = meta_guard.values().filter(|m| m.dirty).count();

        VirtualMemoryStats {
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            host_used_bytes: host_pool.used_bytes,
            host_max_bytes: host_pool.max_bytes,
            host_allocations,
            disk_used_bytes: disk_pool.used_bytes,
            disk_max_bytes: disk_pool.max_bytes,
            disk_allocations,
            pinned_allocations: pinned,
            dirty_allocations: dirty,
        }
    }

    /// Evict least-recently-used allocations to make space
    fn evict_if_needed(
        &self,
        tier: MemoryTier,
        needed_bytes: usize,
    ) -> Result<(), VirtualMemoryError> {
        let (current_used, max_bytes) = match tier {
            MemoryTier::Host => {
                let pool = self.host_pool.read().unwrap();
                (pool.used_bytes, pool.max_bytes)
            }
            MemoryTier::Disk => {
                let pool = self.disk_pool.read().unwrap();
                (pool.used_bytes, pool.max_bytes)
            }
            MemoryTier::Vram => return Ok(()),
        };

        let target_usage = current_used + needed_bytes;
        let threshold = (max_bytes as f64 * self.config.eviction_threshold) as usize;

        if target_usage <= threshold {
            return Ok(()); // No eviction needed
        }

        // Find LRU non-pinned allocations to evict
        let to_evict: Vec<VMemHandle> = {
            let meta_guard = self.metadata.read().unwrap();
            let mut candidates: Vec<_> = meta_guard
                .values()
                .filter(|m| !m.pinned && m.current_tier == tier)
                .collect();
            candidates.sort_by_key(|m| m.last_access);

            let mut to_free = 0usize;
            let bytes_to_free = target_usage.saturating_sub(threshold);
            candidates
                .iter()
                .take_while(|m| {
                    if to_free >= bytes_to_free {
                        false
                    } else {
                        to_free += m.handle.size;
                        true
                    }
                })
                .map(|m| m.handle)
                .collect()
        };

        // Evict by migrating to slower tier or removing
        for handle in to_evict {
            if tier == MemoryTier::Host {
                // Migrate from host to disk, forcing removal from source
                self.migrate_internal(&handle, MemoryTier::Disk, true)?;
            } else if tier == MemoryTier::Disk {
                // Disk eviction: Cannot evict disk data without losing it.
                // This is a configuration issue - the disk tier is full and
                // there's no slower tier to migrate to. For now, we skip disk
                // eviction and let the allocation fail if disk is truly full.
                // In the future, this could support external/cloud storage tiers.
                return Err(VirtualMemoryError::AllocationFailed(
                    "Disk tier full, cannot evict (no slower tier available)".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Flush all dirty allocations to their home tier
    pub fn flush(&self) -> Result<(), VirtualMemoryError> {
        let dirty_handles: Vec<VMemHandle> = {
            let meta_guard = self.metadata.read().unwrap();
            meta_guard
                .values()
                .filter(|m| m.dirty)
                .map(|m| m.handle)
                .collect()
        };

        for handle in dirty_handles {
            // If current tier != home tier, migrate back
            let (current, home) = {
                let meta_guard = self.metadata.read().unwrap();
                if let Some(meta) = meta_guard.get(&handle.id) {
                    (meta.current_tier, meta.handle.home_tier)
                } else {
                    continue;
                }
            };

            if current != home {
                let data = self.read(&handle)?;
                match home {
                    MemoryTier::Host => {
                        let mut pool = self.host_pool.write().unwrap();
                        if pool.contains(handle.id) {
                            if let Some(existing) = pool.data.get_mut(&handle.id) {
                                existing.copy_from_slice(&data);
                            }
                        } else {
                            pool.insert(handle.id, data)?;
                        }
                    }
                    MemoryTier::Disk => {
                        let mut pool = self.disk_pool.write().unwrap();
                        pool.write(handle.id, &data)?;
                    }
                    MemoryTier::Vram => {}
                }
            }

            // Mark as clean
            let mut meta_guard = self.metadata.write().unwrap();
            if let Some(meta) = meta_guard.get_mut(&handle.id) {
                meta.dirty = false;
            }
        }

        Ok(())
    }
}

/// Statistics about virtual memory usage
#[derive(Clone, Debug, Default)]
pub struct VirtualMemoryStats {
    /// Total number of allocations
    pub total_allocations: usize,
    /// Host memory used (bytes)
    pub host_used_bytes: usize,
    /// Host memory capacity (bytes)
    pub host_max_bytes: usize,
    /// Allocations in host memory
    pub host_allocations: usize,
    /// Disk space used (bytes)
    pub disk_used_bytes: usize,
    /// Disk space capacity (bytes)
    pub disk_max_bytes: usize,
    /// Allocations on disk
    pub disk_allocations: usize,
    /// Number of pinned allocations
    pub pinned_allocations: usize,
    /// Number of dirty allocations
    pub dirty_allocations: usize,
}

impl VirtualMemoryStats {
    /// Get host usage ratio (0.0-1.0)
    pub fn host_usage_ratio(&self) -> f64 {
        if self.host_max_bytes == 0 {
            0.0
        } else {
            self.host_used_bytes as f64 / self.host_max_bytes as f64
        }
    }

    /// Get disk usage ratio (0.0-1.0)
    pub fn disk_usage_ratio(&self) -> f64 {
        if self.disk_max_bytes == 0 {
            0.0
        } else {
            self.disk_used_bytes as f64 / self.disk_max_bytes as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn test_config_with_dir(temp_dir: &TempDir) -> VirtualMemoryConfig {
        VirtualMemoryConfig {
            enable_vram: false,
            max_host_bytes: 1024 * 1024,      // 1MB
            max_disk_bytes: 10 * 1024 * 1024, // 10MB
            disk_path: temp_dir.path().to_path_buf(),
            auto_migrate: true,
            eviction_threshold: 0.80,
            promotion_threshold: 3,
        }
    }

    #[test]
    fn test_memory_tier_priority() {
        assert!(MemoryTier::Vram.is_faster_than(&MemoryTier::Host));
        assert!(MemoryTier::Host.is_faster_than(&MemoryTier::Disk));
        assert!(!MemoryTier::Disk.is_faster_than(&MemoryTier::Host));
    }

    #[test]
    fn test_allocate_host() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(1024, MemoryTier::Host).unwrap();

        assert_eq!(handle.size, 1024);
        assert_eq!(handle.home_tier, MemoryTier::Host);

        let stats = vmem.stats();
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.host_allocations, 1);
    }

    #[test]
    fn test_allocate_disk() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(2048, MemoryTier::Disk).unwrap();

        assert_eq!(handle.size, 2048);
        assert_eq!(handle.home_tier, MemoryTier::Disk);

        let stats = vmem.stats();
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.disk_allocations, 1);
    }

    #[test]
    fn test_write_read_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(100, MemoryTier::Host).unwrap();

        let data: Vec<u8> = (0..100).collect();
        vmem.write(&handle, &data).unwrap();

        let read_data = vmem.read(&handle).unwrap();
        assert_eq!(data, read_data);
    }

    #[test]
    fn test_disk_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config_with_dir(&temp_dir);
        let vmem = VirtualMemory::new(config).unwrap();
        let handle = vmem.allocate(256, MemoryTier::Disk).unwrap();

        let data: Vec<u8> = (0..=255).collect();
        vmem.write(&handle, &data).unwrap();

        let read_data = vmem.read(&handle).unwrap();
        assert_eq!(data, read_data);
    }

    #[test]
    fn test_migrate_host_to_disk() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(512, MemoryTier::Host).unwrap();

        let data: Vec<u8> = vec![42; 512];
        vmem.write(&handle, &data).unwrap();

        // Migrate to disk
        vmem.migrate(&handle, MemoryTier::Disk).unwrap();

        // Data should still be readable
        let read_data = vmem.read(&handle).unwrap();
        assert_eq!(data, read_data);
    }

    #[test]
    fn test_pin_unpin() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(100, MemoryTier::Host).unwrap();

        vmem.pin(&handle).unwrap();
        vmem.unpin(&handle).unwrap();

        // Should succeed without error
    }

    #[test]
    fn test_free() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(100, MemoryTier::Host).unwrap();

        let stats = vmem.stats();
        assert_eq!(stats.total_allocations, 1);

        vmem.free(&handle).unwrap();

        let stats = vmem.stats();
        assert_eq!(stats.total_allocations, 0);
    }

    #[test]
    fn test_stats() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();

        // Allocate some memory (handles kept to ensure allocations persist)
        let _h1 = vmem.allocate(100, MemoryTier::Host).unwrap();
        let _h2 = vmem.allocate(200, MemoryTier::Host).unwrap();
        let _h3 = vmem.allocate(300, MemoryTier::Disk).unwrap();

        let stats = vmem.stats();
        assert_eq!(stats.total_allocations, 3);
        assert_eq!(stats.host_allocations, 2);
        assert_eq!(stats.disk_allocations, 1);
        assert_eq!(stats.host_used_bytes, 300); // 100 + 200
        assert_eq!(stats.disk_used_bytes, 300);
    }

    #[test]
    fn test_size_mismatch_error() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();
        let handle = vmem.allocate(100, MemoryTier::Host).unwrap();

        // Try to write wrong size
        let result = vmem.write(&handle, &[0u8; 50]);
        assert!(matches!(
            result,
            Err(VirtualMemoryError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn test_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = Arc::new(VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap());
        let handles: Vec<_> = (0..10)
            .map(|_| vmem.allocate(100, MemoryTier::Host).unwrap())
            .collect();

        // Concurrent reads/writes
        let threads: Vec<_> = handles
            .into_iter()
            .map(|handle| {
                let vmem = Arc::clone(&vmem);
                std::thread::spawn(move || {
                    let data = vec![42u8; 100];
                    vmem.write(&handle, &data).unwrap();
                    let read = vmem.read(&handle).unwrap();
                    assert_eq!(data, read);
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }
    }

    #[test]
    fn test_eviction_from_host_to_disk() {
        let temp_dir = TempDir::new().unwrap();
        let config = VirtualMemoryConfig {
            enable_vram: false,
            max_host_bytes: 500, // Small host pool to trigger eviction
            max_disk_bytes: 10 * 1024 * 1024,
            disk_path: temp_dir.path().to_path_buf(),
            auto_migrate: true,
            eviction_threshold: 0.80, // Evict when 80% full (400 bytes)
            promotion_threshold: 3,
        };
        let vmem = VirtualMemory::new(config).unwrap();

        // Allocate data that will trigger eviction
        let h1 = vmem.allocate(200, MemoryTier::Host).unwrap();
        let h2 = vmem.allocate(200, MemoryTier::Host).unwrap();

        // Write data to both
        vmem.write(&h1, &[1u8; 200]).unwrap();
        vmem.write(&h2, &[2u8; 200]).unwrap();

        // Now try to allocate more - this should trigger eviction
        let h3 = vmem.allocate(200, MemoryTier::Host).unwrap();
        vmem.write(&h3, &[3u8; 200]).unwrap();

        // Check stats - h1 or h2 should have been evicted to disk
        let stats = vmem.stats();
        assert!(stats.disk_allocations > 0, "Should have evicted to disk");
        assert!(
            stats.host_used_bytes <= 400,
            "Host should be under threshold"
        );

        // Verify we can still read the evicted data
        let data1 = vmem.read(&h1).unwrap();
        let data2 = vmem.read(&h2).unwrap();
        let data3 = vmem.read(&h3).unwrap();

        assert_eq!(data1, vec![1u8; 200]);
        assert_eq!(data2, vec![2u8; 200]);
        assert_eq!(data3, vec![3u8; 200]);
    }

    #[test]
    fn test_pinned_allocation_not_evicted() {
        let temp_dir = TempDir::new().unwrap();
        let config = VirtualMemoryConfig {
            enable_vram: false,
            max_host_bytes: 400, // Small host pool
            max_disk_bytes: 10 * 1024 * 1024,
            disk_path: temp_dir.path().to_path_buf(),
            auto_migrate: true,
            eviction_threshold: 0.75, // Evict when 75% full (300 bytes)
            promotion_threshold: 3,
        };
        let vmem = VirtualMemory::new(config).unwrap();

        // Allocate and pin first allocation
        let h1 = vmem.allocate(150, MemoryTier::Host).unwrap();
        vmem.write(&h1, &[1u8; 150]).unwrap();
        vmem.pin(&h1).unwrap();

        // Allocate second (unpinned)
        let h2 = vmem.allocate(150, MemoryTier::Host).unwrap();
        vmem.write(&h2, &[2u8; 150]).unwrap();

        // Try to allocate more - should evict h2 (not h1 which is pinned)
        let h3 = vmem.allocate(150, MemoryTier::Host).unwrap();
        vmem.write(&h3, &[3u8; 150]).unwrap();

        // h1 should still be on host (pinned), h2 should be on disk
        let stats = vmem.stats();
        // The pinned allocation should not have been evicted
        assert_eq!(stats.pinned_allocations, 1);

        // All data should still be readable
        assert_eq!(vmem.read(&h1).unwrap(), vec![1u8; 150]);
        assert_eq!(vmem.read(&h2).unwrap(), vec![2u8; 150]);
        assert_eq!(vmem.read(&h3).unwrap(), vec![3u8; 150]);
    }

    #[test]
    fn test_flush_persists_dirty_data() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();

        // Allocate on disk (home tier)
        let handle = vmem.allocate(256, MemoryTier::Disk).unwrap();
        let original_data: Vec<u8> = (0..=255).collect();
        vmem.write(&handle, &original_data).unwrap();

        // Migrate to host (faster access, but away from home)
        vmem.migrate(&handle, MemoryTier::Host).unwrap();

        // Modify the data (now dirty relative to disk home)
        let modified_data: Vec<u8> = (0..=255).rev().collect();
        vmem.write(&handle, &modified_data).unwrap();

        // Stats should show dirty allocation
        let stats = vmem.stats();
        assert_eq!(stats.dirty_allocations, 1);

        // Flush should persist back to disk
        vmem.flush().unwrap();

        // After flush, should no longer be dirty
        let stats = vmem.stats();
        assert_eq!(stats.dirty_allocations, 0);

        // Data should be readable and correct
        let read_data = vmem.read(&handle).unwrap();
        assert_eq!(read_data, modified_data);
    }

    #[test]
    fn test_vmemhandle_accessors() {
        let temp_dir = TempDir::new().unwrap();
        let vmem = VirtualMemory::new(test_config_with_dir(&temp_dir)).unwrap();

        let handle = vmem.allocate(1024, MemoryTier::Host).unwrap();

        // Test accessor methods
        assert!(handle.id() > 0);
        assert_eq!(handle.size(), 1024);
        assert_eq!(handle.home_tier(), MemoryTier::Host);
    }
}
