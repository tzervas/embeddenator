//! GPU VRAM Memory Pool for Persistent Engrams
//!
//! This module provides arena-style GPU memory management for persistent engrams,
//! enabling unified memory access across CPU RAM, GPU VRAM, and storage.
//!
//! # Design Goals
//!
//! 1. **Arena-style allocation**: Pre-allocate VRAM chunks to reduce allocation overhead
//! 2. **Eviction policy**: Automatically evict least-recently-used engrams under VRAM pressure
//! 3. **Safe limits**: Respect GPU memory constraints from `GpuMemoryConfig`
//! 4. **Integration**: Wire into existing `GpuBackend` infrastructure
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_vsa::{VramPool, VramPoolConfig, GpuBackend, GpuConfig};
//!
//! let gpu = GpuBackend::new(GpuConfig::default())?;
//! let pool = VramPool::new(&gpu, VramPoolConfig::default())?;
//!
//! // Allocate space for an engram
//! let handle = pool.allocate(1024 * 1024)?; // 1MB
//!
//! // Upload data
//! pool.upload(&handle, &my_data)?;
//!
//! // Download data
//! let data = pool.download(&handle)?;
//!
//! // Free when done
//! pool.free(handle)?;
//! ```

#[cfg(feature = "cuda")]
use std::collections::HashMap;
#[cfg(feature = "cuda")]
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
#[cfg(feature = "cuda")]
use std::sync::{Arc, RwLock};

#[cfg(feature = "cuda")]
use cudarc::driver::{CudaSlice, CudaStream};

#[cfg(feature = "cuda")]
use crate::gpu::{GpuError, GpuMemoryConfig};

// Stub types for non-CUDA builds
#[cfg(not(feature = "cuda"))]
#[derive(Debug, Clone)]
pub enum GpuError {
    NotAvailable,
}

#[cfg(not(feature = "cuda"))]
#[derive(Clone, Debug, Default)]
pub struct GpuMemoryConfig {
    pub safe_limit: usize,
}

/// Configuration for VRAM pool
#[derive(Clone, Debug)]
pub struct VramPoolConfig {
    /// Maximum percentage of safe VRAM to use (0.0 - 1.0)
    pub max_usage_ratio: f64,
    /// Enable LRU eviction when pool is full
    pub enable_eviction: bool,
    /// Minimum free space to maintain (bytes)
    pub min_free_bytes: usize,
    /// Enable async transfers
    pub enable_async: bool,
}

impl Default for VramPoolConfig {
    fn default() -> Self {
        Self {
            max_usage_ratio: 0.80, // Use up to 80% of safe VRAM
            enable_eviction: true,
            min_free_bytes: 256 * 1024 * 1024, // 256MB minimum free
            enable_async: true,
        }
    }
}

/// Handle to a VRAM allocation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VramHandle {
    /// Unique allocation ID
    pub id: u64,
    /// Size in bytes
    pub size: usize,
}

#[cfg(feature = "cuda")]
impl VramHandle {
    /// Create a new handle
    fn new(id: u64, size: usize) -> Self {
        Self { id, size }
    }
}

/// Metadata for a VRAM allocation
#[cfg(feature = "cuda")]
#[derive(Debug)]
struct VramAllocation {
    /// Handle to this allocation
    handle: VramHandle,
    /// Last access time for LRU eviction
    last_access: std::time::Instant,
    /// Whether the data is dirty (modified on device)
    dirty: bool,
    /// Whether the allocation is pinned (cannot be evicted)
    pinned: bool,
}

#[cfg(feature = "cuda")]
impl VramAllocation {
    fn new(handle: VramHandle) -> Self {
        Self {
            handle,
            last_access: std::time::Instant::now(),
            dirty: false,
            pinned: false,
        }
    }

    fn touch(&mut self) {
        self.last_access = std::time::Instant::now();
    }
}

/// VRAM memory pool for persistent GPU allocations
///
/// Provides arena-style memory management with LRU eviction
/// for GPU VRAM, enabling persistent engram storage.
#[cfg(feature = "cuda")]
pub struct VramPool {
    /// CUDA stream for transfers
    stream: Arc<CudaStream>,
    /// Pool configuration
    config: VramPoolConfig,
    /// Memory limits from GPU
    memory_config: GpuMemoryConfig,
    /// Active allocations (handle ID -> device buffer)
    allocations: RwLock<HashMap<u64, CudaSlice<u8>>>,
    /// Allocation metadata for LRU tracking
    metadata: RwLock<HashMap<u64, VramAllocation>>,
    /// Next allocation ID
    next_id: AtomicU64,
    /// Current total allocated bytes
    allocated_bytes: AtomicUsize,
}

#[cfg(feature = "cuda")]
impl VramPool {
    /// Create a new VRAM pool with the given GPU stream
    pub fn new(
        stream: Arc<CudaStream>,
        memory_config: GpuMemoryConfig,
        config: VramPoolConfig,
    ) -> Self {
        Self {
            stream,
            config,
            memory_config,
            allocations: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            allocated_bytes: AtomicUsize::new(0),
        }
    }

    /// Get the maximum usable VRAM based on config
    pub fn max_usable_bytes(&self) -> usize {
        let safe_limit = self.memory_config.safe_limit;
        let from_ratio = (safe_limit as f64 * self.config.max_usage_ratio) as usize;
        from_ratio.saturating_sub(self.config.min_free_bytes)
    }

    /// Get current allocated bytes
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes.load(Ordering::Relaxed)
    }

    /// Get available bytes for allocation
    pub fn available_bytes(&self) -> usize {
        self.max_usable_bytes()
            .saturating_sub(self.allocated_bytes())
    }

    /// Check if an allocation of the given size would fit
    pub fn can_allocate(&self, size: usize) -> bool {
        size <= self.available_bytes()
    }

    /// Allocate VRAM for the given size
    ///
    /// Returns a handle to the allocation. If eviction is enabled and
    /// there isn't enough space, LRU allocations will be evicted first.
    pub fn allocate(&self, size: usize) -> Result<VramHandle, GpuError> {
        // Check if we have space (potentially after eviction)
        if !self.can_allocate(size) {
            if self.config.enable_eviction {
                self.evict_until_available(size)?;
            }

            if !self.can_allocate(size) {
                return Err(GpuError::MemoryAlloc(format!(
                    "VRAM pool exhausted: need {} bytes, available {} bytes",
                    size,
                    self.available_bytes()
                )));
            }
        }

        // Allocate on device
        let device_buffer: CudaSlice<u8> = self
            .stream
            .alloc_zeros(size)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;

        // Create handle and track allocation
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let handle = VramHandle::new(id, size);
        let allocation = VramAllocation::new(handle);

        // Store allocation
        {
            let mut allocs = self.allocations.write().unwrap();
            let mut meta = self.metadata.write().unwrap();
            allocs.insert(id, device_buffer);
            meta.insert(id, allocation);
        }

        self.allocated_bytes.fetch_add(size, Ordering::SeqCst);

        Ok(handle)
    }

    /// Free a VRAM allocation
    pub fn free(&self, handle: VramHandle) -> Result<(), GpuError> {
        let mut allocs = self.allocations.write().unwrap();
        let mut meta = self.metadata.write().unwrap();

        if allocs.remove(&handle.id).is_some() {
            meta.remove(&handle.id);
            self.allocated_bytes
                .fetch_sub(handle.size, Ordering::SeqCst);
            Ok(())
        } else {
            Err(GpuError::InvalidValue(format!(
                "VRAM handle {} not found",
                handle.id
            )))
        }
    }

    /// Upload data from host to a VRAM allocation
    pub fn upload(&self, handle: &VramHandle, data: &[u8]) -> Result<(), GpuError> {
        if data.len() != handle.size {
            return Err(GpuError::InvalidValue(format!(
                "Data size {} doesn't match allocation size {}",
                data.len(),
                handle.size
            )));
        }

        // Touch metadata for LRU
        {
            let mut meta = self.metadata.write().unwrap();
            if let Some(alloc) = meta.get_mut(&handle.id) {
                alloc.touch();
            }
        }

        // Get the device buffer and copy data
        let mut allocs = self.allocations.write().unwrap();
        let device_buf = allocs.get_mut(&handle.id).ok_or_else(|| {
            GpuError::InvalidValue(format!("VRAM handle {} not found", handle.id))
        })?;

        // Copy host data to device buffer
        self.stream
            .memcpy_htod(data, device_buf)
            .map_err(|e| GpuError::MemoryCopy(e.to_string()))?;

        Ok(())
    }

    /// Download data from a VRAM allocation to host
    pub fn download(&self, handle: &VramHandle) -> Result<Vec<u8>, GpuError> {
        // Touch metadata for LRU
        {
            let mut meta = self.metadata.write().unwrap();
            if let Some(alloc) = meta.get_mut(&handle.id) {
                alloc.touch();
            }
        }

        let allocs = self.allocations.read().unwrap();
        let device_buf = allocs.get(&handle.id).ok_or_else(|| {
            GpuError::InvalidValue(format!("VRAM handle {} not found", handle.id))
        })?;

        let data = self
            .stream
            .clone_dtoh(device_buf)
            .map_err(|e| GpuError::MemoryCopy(e.to_string()))?;

        Ok(data)
    }

    /// Pin an allocation (prevent eviction)
    pub fn pin(&self, handle: &VramHandle) -> Result<(), GpuError> {
        let mut meta = self.metadata.write().unwrap();
        let alloc = meta.get_mut(&handle.id).ok_or_else(|| {
            GpuError::InvalidValue(format!("VRAM handle {} not found", handle.id))
        })?;
        alloc.pinned = true;
        Ok(())
    }

    /// Unpin an allocation (allow eviction)
    pub fn unpin(&self, handle: &VramHandle) -> Result<(), GpuError> {
        let mut meta = self.metadata.write().unwrap();
        let alloc = meta.get_mut(&handle.id).ok_or_else(|| {
            GpuError::InvalidValue(format!("VRAM handle {} not found", handle.id))
        })?;
        alloc.pinned = false;
        Ok(())
    }

    /// Mark an allocation as dirty (modified on device)
    pub fn mark_dirty(&self, handle: &VramHandle) -> Result<(), GpuError> {
        let mut meta = self.metadata.write().unwrap();
        let alloc = meta.get_mut(&handle.id).ok_or_else(|| {
            GpuError::InvalidValue(format!("VRAM handle {} not found", handle.id))
        })?;
        alloc.dirty = true;
        Ok(())
    }

    /// Check if an allocation is dirty
    pub fn is_dirty(&self, handle: &VramHandle) -> Result<bool, GpuError> {
        let meta = self.metadata.read().unwrap();
        let alloc = meta.get(&handle.id).ok_or_else(|| {
            GpuError::InvalidValue(format!("VRAM handle {} not found", handle.id))
        })?;
        Ok(alloc.dirty)
    }

    /// Evict allocations until we have at least `needed` bytes available
    fn evict_until_available(&self, needed: usize) -> Result<(), GpuError> {
        while self.available_bytes() < needed {
            // Find LRU non-pinned allocation
            let to_evict = {
                let meta = self.metadata.read().unwrap();
                meta.values()
                    .filter(|a| !a.pinned)
                    .min_by_key(|a| a.last_access)
                    .map(|a| a.handle)
            };

            match to_evict {
                Some(handle) => {
                    self.free(handle)?;
                }
                None => {
                    // No evictable allocations
                    return Err(GpuError::MemoryAlloc(
                        "Cannot evict: all allocations are pinned".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get statistics about the pool
    pub fn stats(&self) -> VramPoolStats {
        let meta = self.metadata.read().unwrap();
        let num_allocations = meta.len();
        let num_pinned = meta.values().filter(|a| a.pinned).count();
        let num_dirty = meta.values().filter(|a| a.dirty).count();

        VramPoolStats {
            total_capacity: self.max_usable_bytes(),
            allocated_bytes: self.allocated_bytes(),
            available_bytes: self.available_bytes(),
            num_allocations,
            num_pinned,
            num_dirty,
        }
    }
}

/// Statistics about VRAM pool usage
#[derive(Clone, Debug)]
pub struct VramPoolStats {
    /// Total usable capacity in bytes
    pub total_capacity: usize,
    /// Currently allocated bytes
    pub allocated_bytes: usize,
    /// Available bytes for new allocations
    pub available_bytes: usize,
    /// Number of active allocations
    pub num_allocations: usize,
    /// Number of pinned allocations
    pub num_pinned: usize,
    /// Number of dirty allocations
    pub num_dirty: usize,
}

// Stub for non-CUDA builds
#[cfg(not(feature = "cuda"))]
pub struct VramPool {
    _private: (),
}

#[cfg(not(feature = "cuda"))]
impl VramPool {
    pub fn new(_stream: (), _memory_config: GpuMemoryConfig, _config: VramPoolConfig) -> Self {
        Self { _private: () }
    }

    pub fn allocate(&self, _size: usize) -> Result<VramHandle, GpuError> {
        Err(GpuError::NotAvailable)
    }

    pub fn free(&self, _handle: VramHandle) -> Result<(), GpuError> {
        Err(GpuError::NotAvailable)
    }

    pub fn upload(&self, _handle: &VramHandle, _data: &[u8]) -> Result<(), GpuError> {
        Err(GpuError::NotAvailable)
    }

    pub fn download(&self, _handle: &VramHandle) -> Result<Vec<u8>, GpuError> {
        Err(GpuError::NotAvailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_handle() {
        let h1 = VramHandle { id: 1, size: 1024 };
        let h2 = VramHandle { id: 2, size: 2048 };

        assert_eq!(h1.id, 1);
        assert_eq!(h1.size, 1024);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_vram_pool_config_default() {
        let config = VramPoolConfig::default();
        assert!((config.max_usage_ratio - 0.80).abs() < 0.001);
        assert!(config.enable_eviction);
        assert_eq!(config.min_free_bytes, 256 * 1024 * 1024);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn test_vram_allocation_lru() {
        let handle = VramHandle { id: 1, size: 100 };
        let mut alloc = VramAllocation::new(handle);

        let t1 = alloc.last_access;
        std::thread::sleep(std::time::Duration::from_millis(10));
        alloc.touch();
        let t2 = alloc.last_access;

        assert!(t2 > t1);
    }

    #[test]
    fn test_vram_pool_stats() {
        let stats = VramPoolStats {
            total_capacity: 1024 * 1024 * 1024,
            allocated_bytes: 512 * 1024 * 1024,
            available_bytes: 512 * 1024 * 1024,
            num_allocations: 10,
            num_pinned: 2,
            num_dirty: 1,
        };

        assert_eq!(stats.num_allocations, 10);
        assert_eq!(stats.num_pinned, 2);
    }
}
