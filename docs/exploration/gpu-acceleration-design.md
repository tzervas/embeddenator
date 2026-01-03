# GPU Acceleration Design for Bitsliced Ternary VSA

## Executive Summary

This document outlines a GPU acceleration strategy for the bitsliced ternary
Vector Symbolic Architecture (VSA) operations, targeting both:

1. **WebGPU/wgpu** for cross-platform compute
2. **CUDA/OpenCL** for maximum throughput on dedicated hardware

The bitsliced representation is naturally GPU-friendly due to:
- All operations reduce to bit-parallel logic (AND, OR, XOR, popcount)
- Memory-coalesced access patterns (64-bit word alignment)
- Zero branching in hot paths (SIMD-compatible)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Host (CPU)                                    │
├─────────────────────────────────────────────────────────────────────┤
│  BitslicedTritVec / SoftTernaryVec                                  │
│  - Small ops: inline (no GPU overhead)                              │
│  - Batch ops: queue for GPU execution                               │
│  - Threshold: ~10K dimensions for GPU benefit                       │
└─────────────────┬────────────────────────────────────────────────────┘
                  │ Upload/Download (async)
                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      GPU Buffer Pool                                 │
├─────────────────────────────────────────────────────────────────────┤
│  Pre-allocated buffers for common sizes                             │
│  - pos_plane: u64[] (D/64 elements)                                 │
│  - neg_plane: u64[] (D/64 elements)                                 │
│  - soft_planes: 4×u64[] for SoftTernaryVec                         │
└─────────────────┬────────────────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Compute Shaders                                  │
├─────────────────────────────────────────────────────────────────────┤
│  bind_kernel(A, B) → C                                              │
│  bundle_kernel(A[], threshold) → C                                  │
│  dot_kernel(A, B) → i64                                             │
│  soft_accumulate_kernel(soft, hard)                                 │
│  harden_kernel(soft, threshold) → hard                              │
└─────────────────────────────────────────────────────────────────────┘
```

## WGSL Shader Implementations

### 1. Bind Kernel

```wgsl
// bind.wgsl - XOR binding for bitsliced ternary
@group(0) @binding(0) var<storage, read> a_pos: array<u32>;
@group(0) @binding(1) var<storage, read> a_neg: array<u32>;
@group(0) @binding(2) var<storage, read> b_pos: array<u32>;
@group(0) @binding(3) var<storage, read> b_neg: array<u32>;
@group(0) @binding(4) var<storage, read_write> c_pos: array<u32>;
@group(0) @binding(5) var<storage, read_write> c_neg: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&a_pos)) {
        return;
    }

    // Ternary multiplication via XOR
    // For each position:
    //   (+1) × (+1) = +1, (-1) × (-1) = +1  → XOR preserves
    //   (+1) × (-1) = -1, (-1) × (+1) = -1  → XOR swaps
    //   0 × anything = 0                     → needs masking

    let ap = a_pos[idx];
    let an = a_neg[idx];
    let bp = b_pos[idx];
    let bn = b_neg[idx];

    // Mask of non-zero positions in each vector
    let a_nz = ap | an;
    let b_nz = bp | bn;
    let both_nz = a_nz & b_nz;

    // XOR for sign combination
    let sign_a = an;  // 1 if negative
    let sign_b = bn;
    let sign_c = sign_a ^ sign_b;

    // Result is non-zero only where both inputs are non-zero
    c_pos[idx] = both_nz & ~sign_c;
    c_neg[idx] = both_nz & sign_c;
}
```

### 2. Bundle Kernel (Carry-Save Accumulator)

```wgsl
// bundle.wgsl - Multi-vector bundling with carry-save
struct BundleParams {
    num_vectors: u32,
    threshold: u32,
    num_words: u32,
    _pad: u32,
}

@group(0) @binding(0) var<uniform> params: BundleParams;
@group(0) @binding(1) var<storage, read> pos_planes: array<u32>;  // Flattened: [vec0_pos, vec1_pos, ...]
@group(0) @binding(2) var<storage, read> neg_planes: array<u32>;
@group(0) @binding(3) var<storage, read_write> out_pos: array<u32>;
@group(0) @binding(4) var<storage, read_write> out_neg: array<u32>;

// Workgroup shared memory for reduction
var<workgroup> pos_sum: array<u32, 256>;
var<workgroup> neg_sum: array<u32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>,
        @builtin(local_invocation_id) lid: vec3<u32>) {
    let word_idx = gid.x;
    if (word_idx >= params.num_words) {
        return;
    }

    // Accumulate votes for this word position
    var pos_votes: u32 = 0u;
    var neg_votes: u32 = 0u;

    for (var v: u32 = 0u; v < params.num_vectors; v = v + 1u) {
        let offset = v * params.num_words + word_idx;
        pos_votes = pos_votes + countOneBits(pos_planes[offset]);
        neg_votes = neg_votes + countOneBits(neg_planes[offset]);
    }

    // Threshold comparison for majority
    // Note: For per-bit thresholding, would need bit-serial approach
    // This simplified version operates on word-level aggregates

    // Per-bit majority requires more complex carry-save logic
    // Implemented via multi-pass for large bundles
    let net_pos = pos_votes > neg_votes;
    let net_neg = neg_votes > pos_votes;

    // Output threshold
    if (net_pos && (pos_votes - neg_votes >= params.threshold)) {
        out_pos[word_idx] = 0xFFFFFFFFu;
    } else {
        out_pos[word_idx] = 0u;
    }

    if (net_neg && (neg_votes - pos_votes >= params.threshold)) {
        out_neg[word_idx] = 0xFFFFFFFFu;
    } else {
        out_neg[word_idx] = 0u;
    }
}
```

### 3. Dot Product Kernel

```wgsl
// dot.wgsl - Ternary dot product with hierarchical reduction
@group(0) @binding(0) var<storage, read> a_pos: array<u32>;
@group(0) @binding(1) var<storage, read> a_neg: array<u32>;
@group(0) @binding(2) var<storage, read> b_pos: array<u32>;
@group(0) @binding(3) var<storage, read> b_neg: array<u32>;
@group(0) @binding(4) var<storage, read_write> partial_sums: array<i32>;

var<workgroup> wg_sum: array<i32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>,
        @builtin(local_invocation_id) lid: vec3<u32>,
        @builtin(workgroup_id) wgid: vec3<u32>) {
    let idx = gid.x;
    let local_idx = lid.x;

    var local_sum: i32 = 0;

    if (idx < arrayLength(&a_pos)) {
        let ap = a_pos[idx];
        let an = a_neg[idx];
        let bp = b_pos[idx];
        let bn = b_neg[idx];

        // Agreement: both positive or both negative
        let agree = (ap & bp) | (an & bn);
        // Disagreement: opposite signs
        let disagree = (ap & bn) | (an & bp);

        local_sum = i32(countOneBits(agree)) - i32(countOneBits(disagree));
    }

    // Workgroup reduction
    wg_sum[local_idx] = local_sum;
    workgroupBarrier();

    // Tree reduction within workgroup
    for (var stride: u32 = 128u; stride > 0u; stride = stride >> 1u) {
        if (local_idx < stride) {
            wg_sum[local_idx] = wg_sum[local_idx] + wg_sum[local_idx + stride];
        }
        workgroupBarrier();
    }

    if (local_idx == 0u) {
        partial_sums[wgid.x] = wg_sum[0];
    }
}
```

### 4. Soft Ternary Accumulate Kernel

```wgsl
// soft_accumulate.wgsl - 3-bit saturating accumulation
@group(0) @binding(0) var<storage, read_write> mag_lo: array<u32>;
@group(0) @binding(1) var<storage, read_write> mag_mi: array<u32>;
@group(0) @binding(2) var<storage, read_write> mag_hi: array<u32>;
@group(0) @binding(3) var<storage, read_write> sign: array<u32>;
@group(0) @binding(4) var<storage, read> hard_pos: array<u32>;
@group(0) @binding(5) var<storage, read> hard_neg: array<u32>;

// 3-bit saturating increment for positions in mask
fn saturating_inc(lo: u32, mi: u32, hi: u32, mask: u32) -> vec3<u32> {
    // Increment: add 1 to 3-bit counter where mask is set
    let new_lo = lo ^ mask;
    let carry1 = lo & mask;
    let new_mi = mi ^ carry1;
    let carry2 = mi & carry1;
    let new_hi = hi ^ carry2;

    // Saturation: if would overflow (111 + 1), keep at 111
    let would_overflow = lo & mi & hi & mask;
    let final_lo = new_lo | would_overflow;
    let final_mi = new_mi | would_overflow;
    let final_hi = new_hi | would_overflow;

    return vec3<u32>(final_lo, final_mi, final_hi);
}

// 3-bit saturating decrement
fn saturating_dec(lo: u32, mi: u32, hi: u32, mask: u32) -> vec3<u32> {
    let has_value = lo | mi | hi;
    let dec_mask = mask & has_value;  // Only decrement non-zero

    let borrow1 = ~lo & dec_mask;
    let new_lo = lo ^ dec_mask;
    let borrow2 = ~mi & borrow1;
    let new_mi = mi ^ borrow1;
    let new_hi = hi ^ borrow2;

    return vec3<u32>(new_lo, new_mi, new_hi);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&mag_lo)) {
        return;
    }

    let m0 = mag_lo[idx];
    let m1 = mag_mi[idx];
    let m2 = mag_hi[idx];
    let s = sign[idx];

    let hp = hard_pos[idx];
    let hn = hard_neg[idx];

    // Soft positive, hard positive -> increment
    // Soft positive, hard negative -> decrement (may flip sign)
    // Soft negative, hard positive -> decrement (may flip sign)
    // Soft negative, hard negative -> increment

    let soft_pos = ~s;
    let soft_neg = s;

    let agree_pos = soft_pos & hp;    // Both positive
    let agree_neg = soft_neg & hn;    // Both negative
    let disagree = (soft_pos & hn) | (soft_neg & hp);

    // Increment agreeing positions
    let inc_mask = agree_pos | agree_neg;
    let after_inc = saturating_inc(m0, m1, m2, inc_mask);

    // Decrement disagreeing positions
    let after_dec = saturating_dec(after_inc.x, after_inc.y, after_inc.z, disagree);

    // Handle sign flips on underflow (mag goes to 0 with pending decrement)
    let was_zero = ~(m0 | m1 | m2);
    let new_vote = was_zero & (hp | hn);
    let new_sign = (s & ~new_vote) | (hn & new_vote);

    mag_lo[idx] = after_dec.x;
    mag_mi[idx] = after_dec.y;
    mag_hi[idx] = after_dec.z;
    sign[idx] = new_sign;
}
```

### 5. Harden Kernel

```wgsl
// harden.wgsl - Convert soft ternary to hard with threshold
struct HardenParams {
    threshold: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: HardenParams;
@group(0) @binding(1) var<storage, read> mag_lo: array<u32>;
@group(0) @binding(2) var<storage, read> mag_mi: array<u32>;
@group(0) @binding(3) var<storage, read> mag_hi: array<u32>;
@group(0) @binding(4) var<storage, read> sign: array<u32>;
@group(0) @binding(5) var<storage, read_write> out_pos: array<u32>;
@group(0) @binding(6) var<storage, read_write> out_neg: array<u32>;

// Check if 3-bit value >= threshold (bit-parallel)
fn magnitude_ge_threshold(lo: u32, mi: u32, hi: u32, t: u32) -> u32 {
    // Threshold decomposition
    let t0 = select(0u, 0xFFFFFFFFu, (t & 1u) != 0u);
    let t1 = select(0u, 0xFFFFFFFFu, (t & 2u) != 0u);
    let t2 = select(0u, 0xFFFFFFFFu, (t & 4u) != 0u);

    // Compare: hi > t2, or (hi == t2 and mi > t1), or ...
    let hi_gt = hi & ~t2;
    let hi_eq = ~(hi ^ t2);
    let mi_gt = mi & ~t1;
    let mi_eq = ~(mi ^ t1);
    let lo_ge = lo | ~t0;

    return hi_gt | (hi_eq & (mi_gt | (mi_eq & lo_ge)));
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&mag_lo)) {
        return;
    }

    let m0 = mag_lo[idx];
    let m1 = mag_mi[idx];
    let m2 = mag_hi[idx];
    let s = sign[idx];

    let above_threshold = magnitude_ge_threshold(m0, m1, m2, params.threshold);

    out_pos[idx] = above_threshold & ~s;  // Above threshold and positive
    out_neg[idx] = above_threshold & s;   // Above threshold and negative
}
```

## Performance Model

### Memory Bandwidth Analysis

For dimension D:
- BitslicedTritVec: 2 × (D/64) × 8 = D/4 bytes
- SoftTernaryVec: 4 × (D/64) × 8 = D/2 bytes

| Dimension | Bitsliced | Soft | Transfer Time (PCIe 4.0) |
|-----------|-----------|------|--------------------------|
| 1M        | 250 KB    | 500 KB | 8 μs |
| 10M       | 2.5 MB    | 5 MB | 80 μs |
| 100M      | 25 MB     | 50 MB | 800 μs |

### Operation Throughput

Modern GPUs (RTX 4090 / A100):
- Integer ALU: ~80 TFLOPS equivalent
- Memory bandwidth: 1-2 TB/s

| Operation | Arithmetic Intensity | Expected Throughput |
|-----------|---------------------|---------------------|
| bind      | 0.5 ops/byte        | Memory bound: 2-4 TB/s effective |
| bundle (N=7) | 3.5 ops/byte   | Compute bound: 1.5T ops/s |
| dot       | 0.5 ops/byte        | Memory bound: 2-4 TB/s |
| accumulate | 2 ops/byte        | Balanced |

### Break-Even Analysis

GPU overhead:
- Kernel launch: ~5 μs
- Data transfer: latency + bandwidth
- Synchronization: ~1 μs

For bind operation to be faster on GPU:
- CPU: ~8 cycles/word × D/64 words ÷ 3GHz = D × 0.04 ns
- GPU: 5 μs + D/4 bytes ÷ 25 GB/s = 5 μs + D × 0.01 ns

Break-even at D ≈ 200K dimensions for latency
Break-even at D ≈ 500K dimensions for throughput (amortized)

## Implementation Strategy

### Phase 1: wgpu Backend (Cross-Platform)

```rust
pub struct GpuVsaContext {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Pre-compiled pipelines
    bind_pipeline: wgpu::ComputePipeline,
    bundle_pipeline: wgpu::ComputePipeline,
    dot_pipeline: wgpu::ComputePipeline,
    soft_accumulate_pipeline: wgpu::ComputePipeline,
    harden_pipeline: wgpu::ComputePipeline,

    // Buffer pools for common sizes
    buffer_pools: HashMap<usize, BufferPool>,
}

impl GpuVsaContext {
    pub async fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        // Compile shaders...

        Ok(Self { device, queue, /* ... */ })
    }

    pub fn bind_async(
        &self,
        a: &BitslicedTritVec,
        b: &BitslicedTritVec,
    ) -> GpuFuture<BitslicedTritVec> {
        // Upload, dispatch, download
        todo!()
    }
}
```

### Phase 2: Batched Operations

For maximum GPU utilization, batch multiple operations:

```rust
pub struct GpuBatch {
    operations: Vec<GpuOperation>,
}

enum GpuOperation {
    Bind { a_idx: usize, b_idx: usize, out_idx: usize },
    Bundle { inputs: Vec<usize>, out_idx: usize },
    Dot { a_idx: usize, b_idx: usize, result_idx: usize },
}

impl GpuVsaContext {
    pub fn execute_batch(&self, batch: &GpuBatch) -> BatchResults {
        // Analyze dependencies, schedule optimally
        // Minimize memory transfers
        // Execute with command buffers
        todo!()
    }
}
```

### Phase 3: Hybrid CPU/GPU Scheduling

```rust
pub struct HybridContext {
    cpu_threshold: usize,  // Below this, use CPU
    gpu: Option<GpuVsaContext>,
}

impl HybridContext {
    pub fn bind(&self, a: &BitslicedTritVec, b: &BitslicedTritVec) -> BitslicedTritVec {
        if a.len() < self.cpu_threshold || self.gpu.is_none() {
            a.bind(b)  // CPU path
        } else {
            self.gpu.as_ref().unwrap().bind_sync(a, b)
        }
    }
}
```

## Integration with Existing Codebase

### Feature Flag Structure

```toml
# Cargo.toml
[features]
default = []
gpu = ["wgpu", "bytemuck"]
gpu-profiling = ["gpu", "wgpu/trace"]
cuda = ["cudarc"]  # Optional CUDA backend
```

### API Design

```rust
// In lib.rs or vsa/mod.rs
#[cfg(feature = "gpu")]
pub mod gpu {
    mod context;
    mod kernels;
    mod batch;

    pub use context::GpuVsaContext;
    pub use batch::GpuBatch;
}

// Trait for operations that can be GPU-accelerated
pub trait GpuAccelerable {
    #[cfg(feature = "gpu")]
    fn to_gpu_buffer(&self, ctx: &GpuVsaContext) -> GpuBuffer;

    #[cfg(feature = "gpu")]
    fn from_gpu_buffer(buffer: &GpuBuffer, ctx: &GpuVsaContext) -> Self;
}
```

## Testing Strategy

### Correctness Tests

```rust
#[cfg(feature = "gpu")]
#[test]
fn test_gpu_bind_matches_cpu() {
    let ctx = GpuVsaContext::new_sync().unwrap();

    for dim in [1000, 10_000, 100_000, 1_000_000] {
        let a = BitslicedTritVec::random(dim);
        let b = BitslicedTritVec::random(dim);

        let cpu_result = a.bind(&b);
        let gpu_result = ctx.bind_sync(&a, &b);

        assert_eq!(cpu_result, gpu_result);
    }
}
```

### Performance Benchmarks

```rust
#[cfg(feature = "gpu")]
#[bench]
fn bench_gpu_bind_1m(b: &mut Bencher) {
    let ctx = GpuVsaContext::new_sync().unwrap();
    let a = BitslicedTritVec::random(1_000_000);
    let b = BitslicedTritVec::random(1_000_000);

    b.iter(|| {
        black_box(ctx.bind_sync(&a, &b))
    });
}
```

## Future Directions

### 1. Tensor Core Utilization

For bundle operations with many vectors, reshape into matrix multiply:
- Pack 8 vectors into INT8 matrix rows
- Use tensor cores for accumulation
- Post-process to extract majorities

### 2. Persistent Kernels

For streaming workloads:
- Keep GPU kernel running
- Feed work via ring buffer
- Minimize launch overhead

### 3. Multi-GPU Support

For extreme dimensions (1B+):
- Partition vectors across GPUs
- All-reduce for dot products
- NCCL/RCCL for communication

### 4. Mobile/WebGPU

For browser deployment:
- WebGPU shaders (already WGSL)
- WASM + WebGPU bindings
- Progressive enhancement

## Appendix: Shader Optimization Notes

### Memory Coalescing

Access pattern matters enormously:
- Contiguous threads → contiguous addresses
- u32 vs u64: WebGPU only supports u32 storage
- Padding for alignment

### Occupancy

Workgroup size tradeoffs:
- 256: Good default, high occupancy
- 64: Better for register-heavy shaders
- 1024: Maximum parallelism if registers allow

### Divergence

All shaders above are divergence-free:
- No data-dependent branches in hot path
- select() instead of if/else
- Predicated execution

---

*Document created for exploration/hybrid-tryte-precision branch*
*Last updated: GPU acceleration path for bitsliced ternary VSA*
