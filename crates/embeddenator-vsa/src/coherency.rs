//! Host-Device Coherency Protocol for GPU VRAM
//!
//! This module implements a coherency protocol for maintaining consistency
//! between CPU (host) and GPU (device) memory for engrams.
//!
//! # Design
//!
//! The protocol uses a state machine with dirty bits to track modifications:
//!
//! ```text
//! Host-Resident → (upload) → Device-Resident → (download) → Host-Resident
//!       ↓                           ↓
//!  (modify host)              (modify device)
//!       ↓                           ↓
//! Host-Dirty ←── (sync) ──→ Device-Dirty
//! ```
//!
//! # States
//!
//! - `HostResident`: Data is current on host, may or may not be on device
//! - `DeviceResident`: Data is current on device, host may be stale
//! - `HostDirty`: Host has been modified, device is stale
//! - `DeviceDirty`: Device has been modified, host is stale
//! - `Synced`: Both host and device have identical data
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_vsa::{CoherencyState, CoherentEngram};
//!
//! let mut engram = CoherentEngram::new(data);
//!
//! // Upload to device
//! engram.upload_to_device(&pool)?;
//! assert!(engram.state() == CoherencyState::Synced);
//!
//! // Modify on device
//! engram.mark_device_dirty();
//! assert!(engram.state() == CoherencyState::DeviceDirty);
//!
//! // Sync back to host
//! engram.sync_to_host(&pool)?;
//! assert!(engram.state() == CoherencyState::Synced);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "cuda")]
use crate::gpu::GpuError;
#[cfg(feature = "cuda")]
use crate::vram_pool::{VramHandle, VramPool};
use crate::vsa::SparseVec;

/// Error type for coherency operations (used when cuda feature is disabled)
#[cfg(not(feature = "cuda"))]
#[derive(Debug, Clone)]
pub enum GpuError {
    /// GPU not available
    NotAvailable,
}

/// Coherency state for host-device memory
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoherencyState {
    /// Data only exists on host
    HostOnly,
    /// Data only exists on device
    DeviceOnly,
    /// Host and device are synchronized (identical)
    Synced,
    /// Host has been modified, device is stale
    HostDirty,
    /// Device has been modified, host is stale
    DeviceDirty,
}

impl CoherencyState {
    /// Check if host data is current
    pub fn host_is_current(&self) -> bool {
        matches!(
            self,
            CoherencyState::HostOnly | CoherencyState::Synced | CoherencyState::HostDirty
        )
    }

    /// Check if device data is current
    pub fn device_is_current(&self) -> bool {
        matches!(
            self,
            CoherencyState::DeviceOnly | CoherencyState::Synced | CoherencyState::DeviceDirty
        )
    }

    /// Check if synchronization is needed
    pub fn needs_sync(&self) -> bool {
        matches!(
            self,
            CoherencyState::HostDirty | CoherencyState::DeviceDirty
        )
    }
}

/// A coherent engram that can live on both host and device
///
/// This type manages the synchronization state between CPU and GPU memory
/// for a SparseVec engram.
#[derive(Debug)]
pub struct CoherentEngram {
    /// Host-side data (always present as fallback)
    host_data: Vec<u8>,
    /// VRAM handle (if uploaded to device)
    #[cfg(feature = "cuda")]
    device_handle: Option<VramHandle>,
    #[cfg(not(feature = "cuda"))]
    device_handle: Option<()>,
    /// Current coherency state
    state: CoherencyState,
    /// Version counter for optimistic concurrency
    version: AtomicU64,
}

impl CoherentEngram {
    /// Create a new coherent engram from serialized data
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            host_data: data,
            device_handle: None,
            state: CoherencyState::HostOnly,
            version: AtomicU64::new(0),
        }
    }

    /// Create from a SparseVec by serializing it
    pub fn from_sparse_vec(vec: &SparseVec) -> Self {
        // Simple serialization: pos length, neg length, then pos dims, then neg dims
        let mut data = Vec::new();

        // Write lengths as u32
        let pos_len = vec.pos.len() as u32;
        let neg_len = vec.neg.len() as u32;
        data.extend_from_slice(&pos_len.to_le_bytes());
        data.extend_from_slice(&neg_len.to_le_bytes());

        // Write pos dimensions as u32
        for &dim in &vec.pos {
            data.extend_from_slice(&(dim as u32).to_le_bytes());
        }

        // Write neg dimensions as u32
        for &dim in &vec.neg {
            data.extend_from_slice(&(dim as u32).to_le_bytes());
        }

        Self::new(data)
    }

    /// Deserialize back to SparseVec
    pub fn to_sparse_vec(&self) -> Option<SparseVec> {
        if self.host_data.len() < 8 {
            return None;
        }

        let pos_len = u32::from_le_bytes([
            self.host_data[0],
            self.host_data[1],
            self.host_data[2],
            self.host_data[3],
        ]) as usize;
        let neg_len = u32::from_le_bytes([
            self.host_data[4],
            self.host_data[5],
            self.host_data[6],
            self.host_data[7],
        ]) as usize;

        let expected_size = 8 + (pos_len + neg_len) * 4;
        if self.host_data.len() < expected_size {
            return None;
        }

        let mut pos = Vec::with_capacity(pos_len);
        let mut offset = 8;
        for _ in 0..pos_len {
            let dim = u32::from_le_bytes([
                self.host_data[offset],
                self.host_data[offset + 1],
                self.host_data[offset + 2],
                self.host_data[offset + 3],
            ]) as usize;
            pos.push(dim);
            offset += 4;
        }

        let mut neg = Vec::with_capacity(neg_len);
        for _ in 0..neg_len {
            let dim = u32::from_le_bytes([
                self.host_data[offset],
                self.host_data[offset + 1],
                self.host_data[offset + 2],
                self.host_data[offset + 3],
            ]) as usize;
            neg.push(dim);
            offset += 4;
        }

        Some(SparseVec { pos, neg })
    }

    /// Get current coherency state
    pub fn state(&self) -> CoherencyState {
        self.state
    }

    /// Get version number
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Increment version
    fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::AcqRel);
    }

    /// Get host data (may be stale if DeviceDirty)
    pub fn host_data(&self) -> &[u8] {
        &self.host_data
    }

    /// Get mutable host data and mark as HostDirty
    pub fn host_data_mut(&mut self) -> &mut Vec<u8> {
        self.state = match self.state {
            CoherencyState::HostOnly => CoherencyState::HostOnly,
            CoherencyState::Synced => CoherencyState::HostDirty,
            CoherencyState::HostDirty => CoherencyState::HostDirty,
            _ => CoherencyState::HostDirty,
        };
        self.bump_version();
        &mut self.host_data
    }

    /// Check if data is on device
    pub fn is_on_device(&self) -> bool {
        self.device_handle.is_some()
    }

    /// Upload host data to device
    #[cfg(feature = "cuda")]
    pub fn upload_to_device(&mut self, pool: &VramPool) -> Result<(), GpuError> {
        // Allocate if not already on device
        let handle = if let Some(h) = self.device_handle {
            h
        } else {
            let h = pool.allocate(self.host_data.len())?;
            self.device_handle = Some(h);
            h
        };

        // Upload data
        pool.upload(&handle, &self.host_data)?;

        // Update state
        self.state = CoherencyState::Synced;
        Ok(())
    }

    /// Download device data to host
    #[cfg(feature = "cuda")]
    pub fn download_to_host(&mut self, pool: &VramPool) -> Result<(), GpuError> {
        let handle = self
            .device_handle
            .ok_or_else(|| GpuError::InvalidValue("No device handle".to_string()))?;

        self.host_data = pool.download(&handle)?;
        self.state = CoherencyState::Synced;
        self.bump_version();
        Ok(())
    }

    /// Sync: ensure host and device are consistent
    #[cfg(feature = "cuda")]
    pub fn sync(&mut self, pool: &VramPool) -> Result<(), GpuError> {
        match self.state {
            CoherencyState::HostDirty => {
                // Upload host changes to device
                if self.device_handle.is_some() {
                    self.upload_to_device(pool)?;
                }
                // If not on device, nothing to sync
                self.state = if self.device_handle.is_some() {
                    CoherencyState::Synced
                } else {
                    CoherencyState::HostOnly
                };
            }
            CoherencyState::DeviceDirty => {
                // Download device changes to host
                self.download_to_host(pool)?;
            }
            _ => {
                // Already synced or single-location
            }
        }
        Ok(())
    }

    /// Mark device data as dirty (modified on GPU)
    pub fn mark_device_dirty(&mut self) {
        if self.device_handle.is_some() {
            self.state = CoherencyState::DeviceDirty;
            self.bump_version();
        }
    }

    /// Release device memory
    #[cfg(feature = "cuda")]
    pub fn release_device(&mut self, pool: &VramPool) -> Result<(), GpuError> {
        if let Some(handle) = self.device_handle.take() {
            // Ensure we have current data on host first
            if self.state == CoherencyState::DeviceDirty {
                let data = pool.download(&handle)?;
                self.host_data = data;
            }
            pool.free(handle)?;
            self.state = CoherencyState::HostOnly;
        }
        Ok(())
    }

    /// Get device handle (if on device)
    #[cfg(feature = "cuda")]
    pub fn device_handle(&self) -> Option<VramHandle> {
        self.device_handle
    }
}

// Stubs for non-CUDA builds
#[cfg(not(feature = "cuda"))]
impl CoherentEngram {
    pub fn upload_to_device(&mut self, _pool: &()) -> Result<(), GpuError> {
        Err(GpuError::NotAvailable)
    }

    pub fn download_to_host(&mut self, _pool: &()) -> Result<(), GpuError> {
        Err(GpuError::NotAvailable)
    }

    pub fn sync(&mut self, _pool: &()) -> Result<(), GpuError> {
        Err(GpuError::NotAvailable)
    }

    pub fn release_device(&mut self, _pool: &()) -> Result<(), GpuError> {
        Ok(())
    }
}

/// Coherency manager for multiple engrams
///
/// Manages a collection of coherent engrams with batched sync operations.
#[derive(Default)]
pub struct CoherencyManager {
    /// Tracked engrams by ID
    engrams: std::collections::HashMap<u64, CoherentEngram>,
    /// Next engram ID
    next_id: AtomicU64,
}

impl CoherencyManager {
    /// Create a new coherency manager
    pub fn new() -> Self {
        Self {
            engrams: std::collections::HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Register a new engram
    pub fn register(&mut self, engram: CoherentEngram) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.engrams.insert(id, engram);
        id
    }

    /// Get an engram by ID
    pub fn get(&self, id: u64) -> Option<&CoherentEngram> {
        self.engrams.get(&id)
    }

    /// Get mutable engram by ID
    pub fn get_mut(&mut self, id: u64) -> Option<&mut CoherentEngram> {
        self.engrams.get_mut(&id)
    }

    /// Remove an engram
    pub fn remove(&mut self, id: u64) -> Option<CoherentEngram> {
        self.engrams.remove(&id)
    }

    /// Sync all dirty engrams
    #[cfg(feature = "cuda")]
    pub fn sync_all(&mut self, pool: &VramPool) -> Result<(), GpuError> {
        for engram in self.engrams.values_mut() {
            if engram.state().needs_sync() {
                engram.sync(pool)?;
            }
        }
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> CoherencyStats {
        let total = self.engrams.len();
        let host_only = self
            .engrams
            .values()
            .filter(|e| e.state() == CoherencyState::HostOnly)
            .count();
        let device_only = self
            .engrams
            .values()
            .filter(|e| e.state() == CoherencyState::DeviceOnly)
            .count();
        let synced = self
            .engrams
            .values()
            .filter(|e| e.state() == CoherencyState::Synced)
            .count();
        let dirty = self
            .engrams
            .values()
            .filter(|e| e.state().needs_sync())
            .count();

        CoherencyStats {
            total,
            host_only,
            device_only,
            synced,
            dirty,
        }
    }
}

/// Statistics about coherency state
#[derive(Clone, Debug, Default)]
pub struct CoherencyStats {
    /// Total engrams tracked
    pub total: usize,
    /// Host-only engrams
    pub host_only: usize,
    /// Device-only engrams
    pub device_only: usize,
    /// Synced engrams
    pub synced: usize,
    /// Dirty engrams needing sync
    pub dirty: usize,
}

// ============================================================================
// Multi-Tier Coherency Protocol (#48)
// ============================================================================

/// Memory tier identifier for multi-tier coherency
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Tier {
    /// GPU VRAM
    Vram = 0,
    /// Host RAM
    Host = 1,
    /// Disk storage
    Disk = 2,
}

impl Tier {
    /// Get tier priority (lower = faster)
    pub fn priority(&self) -> u8 {
        match self {
            Tier::Vram => 0,
            Tier::Host => 1,
            Tier::Disk => 2,
        }
    }
}

/// Bitmask tracking which tiers have valid copies of data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TierMask(u8);

impl TierMask {
    /// Empty mask (no valid copies)
    pub const NONE: TierMask = TierMask(0);
    /// VRAM tier bit
    pub const VRAM: TierMask = TierMask(1 << 0);
    /// Host tier bit
    pub const HOST: TierMask = TierMask(1 << 1);
    /// Disk tier bit
    pub const DISK: TierMask = TierMask(1 << 2);

    /// Create a new mask from a tier
    pub fn from_tier(tier: Tier) -> Self {
        match tier {
            Tier::Vram => Self::VRAM,
            Tier::Host => Self::HOST,
            Tier::Disk => Self::DISK,
        }
    }

    /// Check if tier has valid copy
    pub fn has(&self, tier: Tier) -> bool {
        let bit = match tier {
            Tier::Vram => Self::VRAM.0,
            Tier::Host => Self::HOST.0,
            Tier::Disk => Self::DISK.0,
        };
        (self.0 & bit) != 0
    }

    /// Add a tier to the mask
    pub fn add(&mut self, tier: Tier) {
        self.0 |= TierMask::from_tier(tier).0;
    }

    /// Remove a tier from the mask
    pub fn remove(&mut self, tier: Tier) {
        self.0 &= !TierMask::from_tier(tier).0;
    }

    /// Union of two masks
    pub fn union(&self, other: TierMask) -> TierMask {
        TierMask(self.0 | other.0)
    }

    /// Count number of valid tiers
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Check if any tier has valid copy
    pub fn any(&self) -> bool {
        self.0 != 0
    }

    /// Get the fastest tier with valid copy
    pub fn fastest(&self) -> Option<Tier> {
        if self.has(Tier::Vram) {
            Some(Tier::Vram)
        } else if self.has(Tier::Host) {
            Some(Tier::Host)
        } else if self.has(Tier::Disk) {
            Some(Tier::Disk)
        } else {
            None
        }
    }

    /// Iterator over valid tiers
    pub fn iter(&self) -> impl Iterator<Item = Tier> {
        let mask = *self;
        [Tier::Vram, Tier::Host, Tier::Disk]
            .into_iter()
            .filter(move |&t| mask.has(t))
    }
}

/// Write policy for multi-tier coherency
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WritePolicy {
    /// Write to fastest tier only, mark others stale
    #[default]
    WriteBack,
    /// Write to fastest tier and immediately propagate to home tier
    WriteThrough,
    /// Write to all tiers with valid copies
    WriteAll,
}

/// Extended coherency state for multi-tier systems
#[derive(Clone, Debug)]
pub struct TieredState {
    /// Mask of tiers with valid copies
    valid: TierMask,
    /// Mask of tiers with dirty (modified) data
    dirty: TierMask,
    /// The tier that was most recently written
    owner: Option<Tier>,
    /// Home tier (where data should be persisted)
    home: Tier,
    /// Current epoch for sync tracking
    epoch: u64,
}

impl TieredState {
    /// Create a new state with data on one tier
    pub fn new(tier: Tier, home: Tier) -> Self {
        Self {
            valid: TierMask::from_tier(tier),
            dirty: TierMask::NONE,
            owner: Some(tier),
            home,
            epoch: 0,
        }
    }

    /// Create state for host-resident data
    pub fn host_resident() -> Self {
        Self::new(Tier::Host, Tier::Host)
    }

    /// Create state for disk-resident data
    pub fn disk_resident() -> Self {
        Self::new(Tier::Disk, Tier::Disk)
    }

    /// Check if tier has valid copy
    pub fn is_valid(&self, tier: Tier) -> bool {
        self.valid.has(tier)
    }

    /// Check if tier is dirty
    pub fn is_dirty(&self, tier: Tier) -> bool {
        self.dirty.has(tier)
    }

    /// Check if any tier needs sync
    pub fn needs_sync(&self) -> bool {
        self.dirty.any()
    }

    /// Get fastest tier with valid data
    pub fn fastest_valid(&self) -> Option<Tier> {
        self.valid.fastest()
    }

    /// Get the owner tier (most recent write)
    pub fn owner(&self) -> Option<Tier> {
        self.owner
    }

    /// Get current epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Check whether the given tier currently has a valid copy of the data.
    pub fn has_valid_copy(&self, tier: Tier) -> bool {
        self.valid.has(tier)
    }

    /// Record a write to a tier
    pub fn record_write(&mut self, tier: Tier, policy: WritePolicy) {
        match policy {
            WritePolicy::WriteBack => {
                // Invalidate all other copies
                self.valid = TierMask::from_tier(tier);
                self.dirty = TierMask::NONE;
                self.dirty.add(tier);
            }
            WritePolicy::WriteThrough => {
                // Keep valid on writer and home tier
                self.valid = TierMask::from_tier(tier);
                self.valid.add(self.home);
                self.dirty = TierMask::NONE;
            }
            WritePolicy::WriteAll => {
                // All valid copies remain valid; ensure writer tier is marked valid
                self.valid.add(tier);
                self.dirty = TierMask::NONE;
            }
        }
        self.owner = Some(tier);
        self.epoch += 1;
    }

    /// Mark a tier as synced (has current data)
    pub fn mark_synced(&mut self, tier: Tier) {
        self.valid.add(tier);
        self.dirty.remove(tier);
    }

    /// Invalidate a tier (remove from valid set)
    pub fn invalidate(&mut self, tier: Tier) {
        self.valid.remove(tier);
        self.dirty.remove(tier);
        if self.owner == Some(tier) {
            self.owner = self.valid.fastest();
        }
    }

    /// Sync dirty data from owner to target tier
    /// Returns true if sync was performed
    pub fn sync_to(&mut self, target: Tier) -> bool {
        if !self.needs_sync() || self.is_valid(target) {
            return false;
        }
        self.valid.add(target);
        if target == self.home {
            self.dirty = TierMask::NONE;
        }
        true
    }

    /// Get tiers that need sync
    pub fn tiers_needing_sync(&self) -> Vec<Tier> {
        if !self.needs_sync() {
            return vec![];
        }
        // If dirty, we need to sync to home tier
        if !self.is_valid(self.home) {
            vec![self.home]
        } else {
            vec![]
        }
    }
}

/// Multi-tier coherent data block
#[derive(Debug)]
pub struct TieredBlock {
    /// Block ID
    pub id: u64,
    /// Size in bytes
    pub size: usize,
    /// Coherency state
    state: TieredState,
    /// Version for optimistic concurrency
    version: AtomicU64,
}

impl TieredBlock {
    /// Create a new block on host tier
    pub fn new_host(id: u64, size: usize) -> Self {
        Self {
            id,
            size,
            state: TieredState::host_resident(),
            version: AtomicU64::new(0),
        }
    }

    /// Create a new block on disk tier
    pub fn new_disk(id: u64, size: usize) -> Self {
        Self {
            id,
            size,
            state: TieredState::disk_resident(),
            version: AtomicU64::new(0),
        }
    }

    /// Get current state
    pub fn state(&self) -> &TieredState {
        &self.state
    }

    /// Get mutable state
    pub fn state_mut(&mut self) -> &mut TieredState {
        &mut self.state
    }

    /// Get version
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Bump version
    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::AcqRel);
    }

    /// Record a read from tier
    /// Check if a tier has a valid copy for reading
    pub fn read(&self, tier: Tier) -> bool {
        self.state.has_valid_copy(tier)
    }

    /// Record a write to tier
    pub fn write(&mut self, tier: Tier, policy: WritePolicy) {
        self.state.record_write(tier, policy);
        self.bump_version();
    }

    /// Check if sync is needed
    pub fn needs_sync(&self) -> bool {
        self.state.needs_sync()
    }
}

/// Sync protocol for batched coherency operations
#[derive(Debug, Default)]
pub struct SyncProtocol {
    /// Current global epoch
    epoch: AtomicU64,
    /// Write policy
    policy: WritePolicy,
    /// Pending syncs (block_id, source_tier, target_tier)
    pending: std::sync::RwLock<Vec<(u64, Tier, Tier)>>,
}

impl SyncProtocol {
    /// Create with default write policy
    pub fn new() -> Self {
        Self {
            epoch: AtomicU64::new(0),
            policy: WritePolicy::default(),
            pending: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Create with specific write policy
    pub fn with_policy(policy: WritePolicy) -> Self {
        Self {
            epoch: AtomicU64::new(0),
            policy,
            pending: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Get current epoch
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    /// Advance epoch (called after barrier sync)
    pub fn advance_epoch(&self) -> u64 {
        self.epoch.fetch_add(1, Ordering::AcqRel)
    }

    /// Get write policy
    pub fn policy(&self) -> WritePolicy {
        self.policy
    }

    /// Queue a sync operation
    pub fn queue_sync(&self, block_id: u64, from: Tier, to: Tier) {
        let mut pending = self
            .pending
            .write()
            .expect("SyncProtocol pending lock poisoned in queue_sync");
        pending.push((block_id, from, to));
    }

    /// Get and clear pending syncs
    pub fn drain_pending(&self) -> Vec<(u64, Tier, Tier)> {
        let mut pending = self
            .pending
            .write()
            .expect("SyncProtocol pending lock poisoned in drain_pending");
        std::mem::take(&mut *pending)
    }

    /// Check if any syncs are pending
    pub fn has_pending(&self) -> bool {
        let pending = self
            .pending
            .read()
            .expect("SyncProtocol pending lock poisoned in has_pending");
        !pending.is_empty()
    }

    /// Count pending syncs
    pub fn pending_count(&self) -> usize {
        let pending = self
            .pending
            .read()
            .expect("SyncProtocol pending lock poisoned in pending_count");
        pending.len()
    }
}

/// Statistics for multi-tier coherency
#[derive(Clone, Debug, Default)]
pub struct TieredCoherencyStats {
    /// Total blocks tracked
    pub total_blocks: usize,
    /// Blocks with VRAM copy
    pub vram_copies: usize,
    /// Blocks with host copy
    pub host_copies: usize,
    /// Blocks with disk copy
    pub disk_copies: usize,
    /// Blocks needing sync
    pub dirty_blocks: usize,
    /// Total syncs performed
    pub sync_count: u64,
    /// Current epoch
    pub epoch: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coherency_state_checks() {
        assert!(CoherencyState::HostOnly.host_is_current());
        assert!(!CoherencyState::HostOnly.device_is_current());

        assert!(CoherencyState::Synced.host_is_current());
        assert!(CoherencyState::Synced.device_is_current());

        assert!(CoherencyState::HostDirty.needs_sync());
        assert!(CoherencyState::DeviceDirty.needs_sync());
        assert!(!CoherencyState::Synced.needs_sync());
    }

    #[test]
    fn test_coherent_engram_new() {
        let data = vec![1, 2, 3, 4];
        let engram = CoherentEngram::new(data.clone());

        assert_eq!(engram.state(), CoherencyState::HostOnly);
        assert_eq!(engram.host_data(), &data);
        assert!(!engram.is_on_device());
    }

    #[test]
    fn test_coherent_engram_sparse_vec_roundtrip() {
        let vec = SparseVec {
            pos: vec![1, 5, 10, 100],
            neg: vec![2, 7, 50],
        };

        let engram = CoherentEngram::from_sparse_vec(&vec);
        let recovered = engram.to_sparse_vec().unwrap();

        assert_eq!(recovered.pos, vec.pos);
        assert_eq!(recovered.neg, vec.neg);
    }

    #[test]
    fn test_coherent_engram_modify_marks_dirty() {
        let mut engram = CoherentEngram::new(vec![1, 2, 3]);
        assert_eq!(engram.state(), CoherencyState::HostOnly);

        // Modifying host data when Synced should mark as HostDirty
        engram.state = CoherencyState::Synced;
        let _ = engram.host_data_mut();
        assert_eq!(engram.state(), CoherencyState::HostDirty);
    }

    #[test]
    fn test_coherency_manager() {
        let mut manager = CoherencyManager::new();

        let e1 = CoherentEngram::new(vec![1, 2, 3]);
        let e2 = CoherentEngram::new(vec![4, 5, 6]);

        let id1 = manager.register(e1);
        let id2 = manager.register(e2);

        assert!(manager.get(id1).is_some());
        assert!(manager.get(id2).is_some());
        assert!(manager.get(999).is_none());

        let stats = manager.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.host_only, 2);
    }

    // Multi-tier coherency tests (#48)

    #[test]
    fn test_tier_priority() {
        assert_eq!(Tier::Vram.priority(), 0);
        assert_eq!(Tier::Host.priority(), 1);
        assert_eq!(Tier::Disk.priority(), 2);
    }

    #[test]
    fn test_tier_mask_operations() {
        let mut mask = TierMask::NONE;
        assert!(!mask.any());
        assert_eq!(mask.count(), 0);

        mask.add(Tier::Host);
        assert!(mask.has(Tier::Host));
        assert!(!mask.has(Tier::Vram));
        assert_eq!(mask.count(), 1);

        mask.add(Tier::Vram);
        assert!(mask.has(Tier::Host));
        assert!(mask.has(Tier::Vram));
        assert_eq!(mask.count(), 2);

        mask.remove(Tier::Host);
        assert!(!mask.has(Tier::Host));
        assert!(mask.has(Tier::Vram));
        assert_eq!(mask.count(), 1);
    }

    #[test]
    fn test_tier_mask_fastest() {
        let mut mask = TierMask::NONE;
        assert_eq!(mask.fastest(), None);

        mask.add(Tier::Disk);
        assert_eq!(mask.fastest(), Some(Tier::Disk));

        mask.add(Tier::Host);
        assert_eq!(mask.fastest(), Some(Tier::Host));

        mask.add(Tier::Vram);
        assert_eq!(mask.fastest(), Some(Tier::Vram));
    }

    #[test]
    fn test_tier_mask_union() {
        let a = TierMask::HOST;
        let b = TierMask::DISK;
        let union = a.union(b);

        assert!(union.has(Tier::Host));
        assert!(union.has(Tier::Disk));
        assert!(!union.has(Tier::Vram));
    }

    #[test]
    fn test_tiered_state_new() {
        let state = TieredState::host_resident();
        assert!(state.is_valid(Tier::Host));
        assert!(!state.is_valid(Tier::Vram));
        assert!(!state.is_valid(Tier::Disk));
        assert!(!state.needs_sync());
    }

    #[test]
    fn test_tiered_state_write_back() {
        let mut state = TieredState::host_resident();

        // Write to VRAM with writeback policy
        state.record_write(Tier::Vram, WritePolicy::WriteBack);

        assert!(state.is_valid(Tier::Vram));
        assert!(!state.is_valid(Tier::Host)); // Invalidated
        assert!(state.is_dirty(Tier::Vram));
        assert!(state.needs_sync());
        assert_eq!(state.owner(), Some(Tier::Vram));
    }

    #[test]
    fn test_tiered_state_write_through() {
        let mut state = TieredState::host_resident();

        // Write to VRAM with writethrough policy
        state.record_write(Tier::Vram, WritePolicy::WriteThrough);

        assert!(state.is_valid(Tier::Vram));
        assert!(state.is_valid(Tier::Host)); // Still valid (home tier)
        assert!(!state.needs_sync()); // No sync needed
    }

    #[test]
    fn test_tiered_state_write_all() {
        let mut state = TieredState::host_resident();

        // Mark multiple tiers as valid
        state.mark_synced(Tier::Vram);
        state.mark_synced(Tier::Disk);
        assert!(state.is_valid(Tier::Host));
        assert!(state.is_valid(Tier::Vram));
        assert!(state.is_valid(Tier::Disk));

        // Write with WriteAll policy - all copies remain valid
        state.record_write(Tier::Host, WritePolicy::WriteAll);

        // Writer tier should be valid
        assert!(state.is_valid(Tier::Host));
        // WriteAll keeps existing valid copies (unlike WriteBack which invalidates)
        // Note: WriteAll means the write is broadcast to all valid copies
        assert!(!state.needs_sync()); // No sync needed with WriteAll
        assert_eq!(state.owner(), Some(Tier::Host));
    }

    #[test]
    fn test_has_valid_copy() {
        let state = TieredState::host_resident();

        assert!(state.has_valid_copy(Tier::Host));
        assert!(!state.has_valid_copy(Tier::Vram));
        assert!(!state.has_valid_copy(Tier::Disk));
    }

    #[test]
    fn test_tiered_state_sync() {
        let mut state = TieredState::host_resident();
        state.record_write(Tier::Vram, WritePolicy::WriteBack);

        // Sync to host
        let synced = state.sync_to(Tier::Host);
        assert!(synced);
        assert!(state.is_valid(Tier::Host));
        assert!(state.is_valid(Tier::Vram));
        assert!(!state.needs_sync());
    }

    #[test]
    fn test_tiered_state_invalidate() {
        let mut state = TieredState::host_resident();
        state.mark_synced(Tier::Vram);
        assert!(state.is_valid(Tier::Vram));

        state.invalidate(Tier::Vram);
        assert!(!state.is_valid(Tier::Vram));
        assert!(state.is_valid(Tier::Host));
    }

    #[test]
    fn test_tiered_block() {
        let mut block = TieredBlock::new_host(1, 1024);
        assert_eq!(block.id, 1);
        assert_eq!(block.size, 1024);
        assert_eq!(block.version(), 0);

        block.write(Tier::Host, WritePolicy::WriteBack);
        assert_eq!(block.version(), 1);
        assert!(block.needs_sync());
    }

    #[test]
    fn test_sync_protocol() {
        let protocol = SyncProtocol::new();
        assert_eq!(protocol.epoch(), 0);
        assert!(!protocol.has_pending());

        protocol.queue_sync(1, Tier::Vram, Tier::Host);
        protocol.queue_sync(2, Tier::Host, Tier::Disk);
        assert!(protocol.has_pending());
        assert_eq!(protocol.pending_count(), 2);

        let pending = protocol.drain_pending();
        assert_eq!(pending.len(), 2);
        assert!(!protocol.has_pending());

        let new_epoch = protocol.advance_epoch();
        assert_eq!(new_epoch, 0);
        assert_eq!(protocol.epoch(), 1);
    }

    #[test]
    fn test_sync_protocol_policy() {
        let default = SyncProtocol::new();
        assert_eq!(default.policy(), WritePolicy::WriteBack);

        let writethrough = SyncProtocol::with_policy(WritePolicy::WriteThrough);
        assert_eq!(writethrough.policy(), WritePolicy::WriteThrough);
    }
}
