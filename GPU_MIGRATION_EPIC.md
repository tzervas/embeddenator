# GPU Migration Epic: cudarc 0.12 → 0.19

**Status**: Planned  
**Priority**: High  
**Estimated Effort**: 2-3 days  
**Target**: embeddenator-vsa 0.21.0

## Executive Summary

The GPU acceleration module in `embeddenator-vsa/src/gpu.rs` provides 10-100x speedup for batch similarity computations but is currently disabled pending migration from cudarc 0.12 API patterns to cudarc 0.19. This epic tracks the full re-enablement of GPU support.

---

## Current State

### What Exists (390 lines in gpu.rs)

| Component | Status | Description |
|-----------|--------|-------------|
| `GpuConfig` | ✅ Complete | Configuration struct (14GB memory limit, 4 streams, min batch 4) |
| `GpuBackend` | ✅ Complete | Main GPU backend struct with device and kernel handles |
| `GpuBackend::new()` | ⚠️ Needs Migration | Device init, PTX compilation, kernel loading |
| `GpuBackend::batch_cosine()` | ⚠️ Needs Migration | Main batch similarity computation |
| `GpuError` enum | ✅ Complete | Comprehensive error types |
| Non-CUDA stub | ✅ Complete | Graceful fallback when CUDA unavailable |
| CUDA kernel source | ✅ Complete | `batch_cosine_kernel` inline PTX |

### Why Disabled

```toml
# embeddenator-vsa/Cargo.toml
[features]
# cuda = ["dep:cudarc"]  # Disabled pending cudarc 0.19 API migration
```

```rust
// embeddenator-vsa/src/lib.rs
// GPU module placeholder - pending cudarc 0.19 API migration
// #[cfg(feature = "cuda")]
// pub mod gpu;
```

---

## Migration Tasks

### Task 1: Update Cargo.toml Dependencies

**File**: `embeddenator-vsa/Cargo.toml`

```toml
[dependencies]
# Before (0.12 era)
cudarc = { version = "0.12", optional = true }

# After (0.19)
cudarc = { version = ">=0.19, <1.0", optional = true }

[features]
cuda = ["dep:cudarc"]
```

### Task 2: Device Initialization API Changes

**File**: `embeddenator-vsa/src/gpu.rs`  
**Location**: `GpuBackend::new()`

#### 0.12 Pattern (Current)
```rust
use cudarc::driver::{CudaDevice, CudaSlice, LaunchAsync, LaunchConfig};

let device = CudaDevice::new(0).map_err(|e| GpuError::DeviceInit(e.to_string()))?;
let device = Arc::new(device);
```

#### 0.19 Pattern (Required)
```rust
use cudarc::driver::safe::{CudaDevice, CudaSlice};
use cudarc::driver::{LaunchAsync, LaunchConfig};

// Device creation may require explicit context management
let device = CudaDevice::new(0).map_err(|e| GpuError::DeviceInit(e.to_string()))?;
// Arc wrapping may differ - check if CudaDevice is already Arc-wrapped
```

**Action Items**:
- [ ] Verify `CudaDevice::new()` signature in 0.19
- [ ] Check if `Arc` wrapping is still required
- [ ] Update import paths (`driver::safe` vs `driver`)

### Task 3: PTX Compilation API Changes

**File**: `embeddenator-vsa/src/gpu.rs`  
**Location**: `GpuBackend::new()` kernel compilation

#### 0.12 Pattern (Current)
```rust
let ptx = cudarc::nvrtc::compile_ptx(VSA_KERNELS)
    .map_err(|e| GpuError::KernelCompile(e.to_string()))?;

device.load_ptx(ptx, "vsa_kernels", &["batch_cosine_kernel"])
    .map_err(|e| GpuError::KernelLoad(e.to_string()))?;

let batch_cosine_kernel = device.get_func("vsa_kernels", "batch_cosine_kernel")
    .ok_or(GpuError::KernelNotFound("batch_cosine_kernel".into()))?;
```

#### 0.19 Pattern (Required)
```rust
use cudarc::nvrtc::Ptx;

// compile_ptx may return Ptx directly or require different handling
let ptx = cudarc::nvrtc::compile_ptx(VSA_KERNELS)
    .map_err(|e| GpuError::KernelCompile(e.to_string()))?;

// load_ptx signature may have changed
// get_func may return CudaFunction differently
```

**Action Items**:
- [ ] Check `nvrtc::compile_ptx()` return type
- [ ] Verify `device.load_ptx()` signature
- [ ] Check `device.get_func()` return type
- [ ] Update `CudaFunction` storage pattern

### Task 4: Memory Transfer API Changes

**File**: `embeddenator-vsa/src/gpu.rs`  
**Location**: `GpuBackend::batch_cosine()`

#### 0.12 Pattern (Current)
```rust
// Host to Device
let d_a_pos = self.device.htod_copy(a_pos.clone())
    .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;

// Allocate zeros
let d_similarities: CudaSlice<f32> = self.device.alloc_zeros(batch_b.len())
    .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;

// Device to Host
let similarities: Vec<f32> = self.device.dtoh_sync_copy(&d_similarities)
    .map_err(|e| GpuError::MemoryCopy(e.to_string()))?;
```

#### 0.19 Pattern (Required)
```rust
// Methods may be renamed or on different types
// htod_copy -> htod_sync_copy or similar
// dtoh_sync_copy -> dtoh_sync or different signature
// alloc_zeros may require explicit size type
```

**Action Items**:
- [ ] Verify `htod_copy()` method name and signature
- [ ] Verify `dtoh_sync_copy()` method name and signature  
- [ ] Verify `alloc_zeros()` method and type requirements
- [ ] Check `CudaSlice` type parameters

### Task 5: Kernel Launch API Changes

**File**: `embeddenator-vsa/src/gpu.rs`  
**Location**: `GpuBackend::batch_cosine()` kernel launch

#### 0.12 Pattern (Current)
```rust
let cfg = LaunchConfig {
    block_dim: (256, 1, 1),
    grid_dim: (batch_b.len() as u32, 1, 1),
    shared_mem_bytes: 16,
};

unsafe {
    self.batch_cosine_kernel.clone().launch(cfg, (
        &d_a_pos,
        &d_a_neg,
        // ... 12 kernel parameters
    )).map_err(|e| GpuError::KernelLaunch(e.to_string()))?;
}
```

#### 0.19 Pattern (Required)
```rust
// LaunchConfig struct fields may have changed
// launch() method signature may differ
// Tuple parameter passing may need adjustment
```

**Action Items**:
- [ ] Verify `LaunchConfig` struct fields
- [ ] Check `launch()` method signature
- [ ] Verify kernel parameter passing syntax
- [ ] Test unsafe block requirements

### Task 6: Re-enable Feature and Module

**Files**: 
- `embeddenator-vsa/Cargo.toml`
- `embeddenator-vsa/src/lib.rs`

```toml
# Cargo.toml - uncomment
[features]
cuda = ["dep:cudarc"]
```

```rust
// lib.rs - uncomment
#[cfg(feature = "cuda")]
pub mod gpu;
#[cfg(feature = "cuda")]
pub use gpu::{GpuBackend, GpuConfig, GpuError};
```

### Task 7: Add Integration Tests

**File**: `embeddenator-vsa/tests/gpu_tests.rs` (new)

```rust
#![cfg(feature = "cuda")]

use embeddenator_vsa::{GpuBackend, GpuConfig, SparseVec, ReversibleVSAConfig};

#[test]
fn test_gpu_backend_init() {
    let config = GpuConfig::default();
    let backend = GpuBackend::new(config);
    // May fail if no GPU, that's OK
    if let Ok(backend) = backend {
        assert!(backend.is_available());
    }
}

#[test]
fn test_batch_cosine_matches_cpu() {
    let config = GpuConfig::default();
    let Ok(backend) = GpuBackend::new(config) else { return };
    
    let vsa_config = ReversibleVSAConfig::default();
    let query = SparseVec::encode_data(b"query", &vsa_config, None);
    let candidates: Vec<_> = (0..100)
        .map(|i| SparseVec::encode_data(format!("doc-{}", i).as_bytes(), &vsa_config, None))
        .collect();
    
    let gpu_results = backend.batch_cosine(&query, &candidates).unwrap();
    let cpu_results: Vec<f64> = candidates.iter().map(|c| query.cosine(c)).collect();
    
    for (gpu, cpu) in gpu_results.iter().zip(cpu_results.iter()) {
        assert!((gpu - cpu).abs() < 1e-5, "GPU and CPU results differ");
    }
}
```

### Task 8: Update CI for GPU Testing (Optional)

**File**: `embeddenator-workflows/.github/workflows/reusable-ci.yml`

```yaml
inputs:
  enable-cuda-tests:
    description: 'Run CUDA tests (requires GPU runner)'
    required: false
    type: boolean
    default: false

# In jobs section
- name: Run GPU tests
  if: inputs.enable-cuda-tests
  run: cargo test --features cuda --release
```

---

## Validation Checklist

- [ ] `cargo build --features cuda` compiles
- [ ] `cargo test --features cuda` passes (on GPU machine)
- [ ] `cargo clippy --features cuda -- -D warnings` clean
- [ ] Batch cosine GPU results match CPU within 1e-5
- [ ] GPU acceleration shows >10x speedup for batch >100
- [ ] Graceful fallback when CUDA unavailable
- [ ] Documentation updated with CUDA requirements

---

## Dependencies

### Before This Epic
- ✅ All clippy fixes merged
- ✅ Rand version alignment complete
- ✅ CI workflow updated

### After This Epic
- GPU benchmarks in embeddenator-contract-bench
- GPU support in embeddenator-retrieval (optional)
- CUDA CI runner setup

---

## Resources

- [cudarc 0.19 Changelog](https://github.com/coreylowman/cudarc/releases)
- [cudarc Migration Guide](https://github.com/coreylowman/cudarc/blob/main/CHANGELOG.md)
- [CUDA Programming Guide](https://docs.nvidia.com/cuda/cuda-c-programming-guide/)

---

## Timeline

| Phase | Duration | Tasks |
|-------|----------|-------|
| Research | 0.5 day | Review cudarc 0.19 API, document exact changes |
| Implementation | 1 day | Tasks 1-6 |
| Testing | 0.5 day | Task 7, manual GPU testing |
| CI/CD | 0.5 day | Task 8 (if GPU runner available) |
| **Total** | **2-3 days** | |

---

*Last Updated: 2026-01-25*
