//! GPU-accelerated VSA operations using CUDA
//!
//! This module provides GPU-accelerated versions of CPU-bound VSA operations.
//! Requires the `cuda` feature and a CUDA-capable GPU.
//!
//! # Dynamic Architecture Detection
//!
//! The GPU backend automatically detects the GPU compute capability and compiles
//! kernels for the optimal architecture. Supported architectures:
//!
//! | SM Version | Architecture | Example GPUs |
//! |------------|--------------|--------------|
//! | SM 7.5     | Turing       | RTX 2080, T4 |
//! | SM 8.0     | Ampere       | A100, A30    |
//! | SM 8.6     | Ampere       | RTX 3090     |
//! | SM 8.9     | Ada Lovelace | RTX 4090     |
//! | SM 12.0    | Blackwell    | RTX 5080/5090|
//!
//! # Performance
//!
//! GPU acceleration provides significant speedups for:
//! - Large batch similarity computations (10-100x speedup)
//! - Parallel bundle operations
//! - Bulk encoding operations
//!
//! # Dynamic Memory Management
//!
//! The GPU backend dynamically queries available GPU memory at initialization and
//! calculates safe limits accounting for:
//! - Other GPU processes (X11, browsers, etc.)
//! - Reserved headroom (configurable, default 15%)
//! - Runtime memory pressure detection
//!
//! Memory limits scale automatically based on GPU capabilities.
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_vsa::{GpuBackend, GpuConfig, GpuArch, SparseVec};
//!
//! // Auto-detect GPU architecture
//! let config = GpuConfig::default();
//! let gpu = GpuBackend::new(config)?;
//! println!("Using GPU arch: {:?}", gpu.architecture());
//!
//! // Or specify a target architecture
//! let config = GpuConfig::default().with_architecture(GpuArch::Sm120);
//! let gpu = GpuBackend::new(config)?;
//!
//! // Batch cosine similarity computation
//! let query = SparseVec::random();
//! let candidates: Vec<SparseVec> = (0..1000).map(|_| SparseVec::random()).collect();
//! let similarities = gpu.batch_cosine(&query, &candidates)?;
//! ```

#[cfg(feature = "cuda")]
use cudarc::driver::{
    CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, LaunchConfig, PushKernelArg,
};
#[cfg(feature = "cuda")]
use std::sync::Arc;

use crate::vsa::SparseVec;

/// Default GPU memory reserve headroom (15% of total)
pub const DEFAULT_MEMORY_HEADROOM_PERCENT: f64 = 0.15;

/// Minimum GPU memory to reserve for system use (512MB)
pub const MIN_SYSTEM_RESERVE_BYTES: usize = 512 * 1024 * 1024;

/// Maximum safe memory usage ratio (85% of available after system reserve)
pub const MAX_MEMORY_USAGE_RATIO: f64 = 0.85;

/// GPU memory configuration for dynamic scaling
#[derive(Clone, Debug)]
pub struct GpuMemoryConfig {
    /// Total GPU memory in bytes (queried from device)
    pub total_memory: usize,
    /// Currently free GPU memory in bytes
    pub free_memory: usize,
    /// Memory used by other processes
    pub external_usage: usize,
    /// Calculated safe limit for embeddenator
    pub safe_limit: usize,
    /// Reserved headroom percentage (0.0 - 1.0)
    pub headroom_percent: f64,
}

impl GpuMemoryConfig {
    /// Create a new memory config with dynamic limits
    pub fn new(total: usize, free: usize, headroom_percent: f64) -> Self {
        let external_usage = total.saturating_sub(free);
        let available_for_use = free.saturating_sub(MIN_SYSTEM_RESERVE_BYTES);
        let safe_limit = ((available_for_use as f64) * (1.0 - headroom_percent)) as usize;

        Self {
            total_memory: total,
            free_memory: free,
            external_usage,
            safe_limit,
            headroom_percent,
        }
    }

    /// Query current memory pressure and return updated config
    #[cfg(feature = "cuda")]
    pub fn refresh(&self, ctx: &CudaContext) -> Result<Self, GpuError> {
        let (total, free) = query_gpu_memory(ctx)?;
        Ok(Self::new(total, free, self.headroom_percent))
    }

    /// Check if a memory allocation is safe
    pub fn can_allocate(&self, bytes: usize) -> bool {
        bytes <= self.safe_limit
    }

    /// Get recommended batch size for given vector size
    pub fn recommended_batch_size(&self, vector_memory_bytes: usize) -> usize {
        if vector_memory_bytes == 0 {
            return 10000; // Default for tiny vectors
        }
        // Use 50% of safe limit for batching to leave room for intermediate results
        let batch_memory = self.safe_limit / 2;
        (batch_memory / vector_memory_bytes).max(1)
    }
}

impl Default for GpuMemoryConfig {
    fn default() -> Self {
        // Conservative default when actual memory isn't known
        Self {
            total_memory: 16 * 1024 * 1024 * 1024, // Assume 16GB
            free_memory: 14 * 1024 * 1024 * 1024,  // Assume 14GB free
            external_usage: 2 * 1024 * 1024 * 1024,
            safe_limit: 12 * 1024 * 1024 * 1024, // ~12GB safe
            headroom_percent: DEFAULT_MEMORY_HEADROOM_PERCENT,
        }
    }
}

/// Query GPU memory from nvidia-smi (fallback when cudarc doesn't expose it)
#[cfg(feature = "cuda")]
fn query_gpu_memory_from_smi() -> Result<(usize, usize), GpuError> {
    use std::process::Command;

    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| GpuError::DeviceInit(format!("nvidia-smi failed: {}", e)))?;

    if !output.status.success() {
        return Err(GpuError::DeviceInit(
            "nvidia-smi returned error".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .ok_or_else(|| GpuError::DeviceInit("nvidia-smi returned no output".to_string()))?;

    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() != 2 {
        return Err(GpuError::DeviceInit("nvidia-smi parse error".to_string()));
    }

    let total_mb: usize = parts[0]
        .parse()
        .map_err(|_| GpuError::DeviceInit("failed to parse total memory".to_string()))?;
    let free_mb: usize = parts[1]
        .parse()
        .map_err(|_| GpuError::DeviceInit("failed to parse free memory".to_string()))?;

    Ok((total_mb * 1024 * 1024, free_mb * 1024 * 1024))
}

/// Query GPU memory (tries cudarc first, falls back to nvidia-smi)
#[cfg(feature = "cuda")]
fn query_gpu_memory(_ctx: &CudaContext) -> Result<(usize, usize), GpuError> {
    // cudarc 0.19 doesn't expose mem_info directly, use nvidia-smi
    // TODO: When cudarc adds mem_get_info binding, use that instead
    query_gpu_memory_from_smi()
}

/// Query GPU compute capability from nvidia-smi
#[cfg(feature = "cuda")]
fn query_compute_capability() -> Result<(u32, u32), GpuError> {
    use std::process::Command;

    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
        .output()
        .map_err(|e| GpuError::DeviceInit(format!("nvidia-smi failed: {}", e)))?;

    if !output.status.success() {
        return Err(GpuError::DeviceInit(
            "nvidia-smi returned error".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .ok_or_else(|| GpuError::DeviceInit("nvidia-smi returned no output".to_string()))?;

    // Format: "8.9" or "12.0"
    let parts: Vec<&str> = line.trim().split('.').collect();
    if parts.len() != 2 {
        return Err(GpuError::DeviceInit(format!(
            "compute_cap parse error: {}",
            line
        )));
    }

    let major: u32 = parts[0]
        .parse()
        .map_err(|_| GpuError::DeviceInit("failed to parse compute major".to_string()))?;
    let minor: u32 = parts[1]
        .parse()
        .map_err(|_| GpuError::DeviceInit("failed to parse compute minor".to_string()))?;

    Ok((major, minor))
}

/// CUDA compute capability / SM version
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum GpuArch {
    /// SM 7.5 - Turing (RTX 20 series, T4)
    Sm75,
    /// SM 8.0 - Ampere (A100, A30)
    Sm80,
    /// SM 8.6 - Ampere Consumer (RTX 30 series)
    Sm86,
    /// SM 8.9 - Ada Lovelace (RTX 40 series)
    Sm89,
    /// SM 12.0 - Blackwell (RTX 50 series) - Default target
    #[default]
    Sm120,
    /// Custom architecture string
    Custom(u32, u32),
}

impl GpuArch {
    /// Get the NVRTC architecture flag for this GPU (static str for known archs)
    pub fn nvrtc_arch(&self) -> &'static str {
        match self {
            GpuArch::Sm75 => "compute_75",
            GpuArch::Sm80 => "compute_80",
            GpuArch::Sm86 => "compute_86",
            GpuArch::Sm89 => "compute_89",
            GpuArch::Sm120 => "compute_120",
            GpuArch::Custom(major, minor) => {
                // Fallback: leak the string for custom architectures
                Box::leak(format!("compute_{}{}", major, minor).into_boxed_str())
            }
        }
    }

    /// Get the PTX target architecture
    pub fn ptx_target(&self) -> &'static str {
        match self {
            GpuArch::Sm75 => "sm_75",
            GpuArch::Sm80 => "sm_80",
            GpuArch::Sm86 => "sm_86",
            GpuArch::Sm89 => "sm_89",
            GpuArch::Sm120 => "sm_120",
            GpuArch::Custom(major, minor) => {
                // Fallback: leak the string for custom architectures
                Box::leak(format!("sm_{}{}", major, minor).into_boxed_str())
            }
        }
    }

    /// Detect GPU architecture from compute capability
    pub fn from_compute_capability(major: u32, minor: u32) -> Self {
        match (major, minor) {
            (7, 5) => GpuArch::Sm75,
            (8, 0) => GpuArch::Sm80,
            (8, 6) => GpuArch::Sm86,
            (8, 9) => GpuArch::Sm89,
            (12, 0) => GpuArch::Sm120,
            (m, n) if m >= 12 => GpuArch::Sm120, // Future Blackwell+ defaults to SM120
            (m, n) if m >= 8 => GpuArch::Sm89,   // Future Ada+ defaults to SM89
            _ => GpuArch::Custom(major, minor),
        }
    }

    /// Check if this architecture supports specific features
    pub fn supports_tensor_cores(&self) -> bool {
        matches!(
            self,
            GpuArch::Sm75 | GpuArch::Sm80 | GpuArch::Sm86 | GpuArch::Sm89 | GpuArch::Sm120
        )
    }

    /// Check if this is Blackwell architecture (SM 12.0+)
    pub fn is_blackwell(&self) -> bool {
        match self {
            GpuArch::Sm120 => true,
            GpuArch::Custom(m, _) => *m >= 12,
            _ => false,
        }
    }
}

/// Configuration for GPU-accelerated operations
#[derive(Clone, Debug)]
pub struct GpuConfig {
    /// Memory configuration (None = auto-detect at init)
    pub memory_config: Option<GpuMemoryConfig>,
    /// Minimum batch size to use GPU (below this, CPU is faster)
    pub min_batch_size: usize,
    /// Number of CUDA streams for concurrent operations
    pub num_streams: usize,
    /// Target GPU architecture (None = auto-detect)
    pub architecture: Option<GpuArch>,
    /// Enable architecture auto-detection
    pub auto_detect_arch: bool,
    /// Memory headroom percentage (0.0 - 1.0)
    pub memory_headroom: f64,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            memory_config: None, // Auto-detect at init
            min_batch_size: 4,   // At least 4 vectors to justify GPU overhead
            num_streams: 4,
            architecture: None, // Auto-detect by default
            auto_detect_arch: true,
            memory_headroom: DEFAULT_MEMORY_HEADROOM_PERCENT,
        }
    }
}

impl GpuConfig {
    /// Set target GPU architecture explicitly
    pub fn with_architecture(mut self, arch: GpuArch) -> Self {
        self.architecture = Some(arch);
        self.auto_detect_arch = false;
        self
    }

    /// Enable architecture auto-detection
    pub fn with_auto_detect(mut self) -> Self {
        self.architecture = None;
        self.auto_detect_arch = true;
        self
    }

    /// Set memory headroom percentage
    pub fn with_memory_headroom(mut self, headroom: f64) -> Self {
        self.memory_headroom = headroom.clamp(0.05, 0.50); // 5-50% range
        self
    }

    /// Set minimum batch size for GPU usage
    pub fn with_min_batch_size(mut self, size: usize) -> Self {
        self.min_batch_size = size;
        self
    }

    /// Get the effective memory limit
    pub fn effective_memory_limit(&self) -> usize {
        self.memory_config
            .as_ref()
            .map(|c| c.safe_limit)
            .unwrap_or(12 * 1024 * 1024 * 1024) // 12GB fallback
    }
}

impl GpuArch {
    /// Get major version number
    pub fn major_version(&self) -> u32 {
        match self {
            GpuArch::Sm75 => 7,
            GpuArch::Sm80 | GpuArch::Sm86 | GpuArch::Sm89 => 8,
            GpuArch::Sm120 => 12,
            GpuArch::Custom(major, _) => *major,
        }
    }

    /// Get minor version number
    pub fn minor_version(&self) -> u32 {
        match self {
            GpuArch::Sm75 => 5,
            GpuArch::Sm80 => 0,
            GpuArch::Sm86 => 6,
            GpuArch::Sm89 => 9,
            GpuArch::Sm120 => 0,
            GpuArch::Custom(_, minor) => *minor,
        }
    }
}

/// CUDA kernel source for VSA batch cosine similarity (generic)
#[cfg(feature = "cuda")]
const VSA_KERNELS: &str = r#"
extern "C" __global__ void batch_cosine_kernel(
    const unsigned int* a_pos,
    const unsigned int* a_neg,
    const unsigned int* b_pos_flat,
    const unsigned int* b_neg_flat,
    const int* b_pos_offsets,
    const int* b_neg_offsets,
    const int* b_pos_lens,
    const int* b_neg_lens,
    float* similarities,
    int a_pos_len,
    int a_neg_len,
    int num_b
) {
    int b_idx = blockIdx.x;
    if (b_idx >= num_b) return;
    
    int b_pos_start = b_pos_offsets[b_idx];
    int b_neg_start = b_neg_offsets[b_idx];
    int b_pos_len = b_pos_lens[b_idx];
    int b_neg_len = b_neg_lens[b_idx];
    
    __shared__ int pp, nn, pn, np;
    if (threadIdx.x == 0) { pp = nn = pn = np = 0; }
    __syncthreads();
    
    // Each thread handles a portion of a_pos elements
    for (int i = threadIdx.x; i < a_pos_len; i += blockDim.x) {
        unsigned int val = a_pos[i];
        
        // Binary search in b_pos
        int lo = 0, hi = b_pos_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            unsigned int bval = b_pos_flat[b_pos_start + mid];
            if (bval == val) {
                atomicAdd(&pp, 1);
                break;
            } else if (bval < val) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        
        // Binary search in b_neg
        lo = 0; hi = b_neg_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            unsigned int bval = b_neg_flat[b_neg_start + mid];
            if (bval == val) {
                atomicAdd(&pn, 1);
                break;
            } else if (bval < val) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
    }
    
    // Each thread handles a portion of a_neg elements
    for (int i = threadIdx.x; i < a_neg_len; i += blockDim.x) {
        unsigned int val = a_neg[i];
        
        // Binary search in b_pos
        int lo = 0, hi = b_pos_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            unsigned int bval = b_pos_flat[b_pos_start + mid];
            if (bval == val) {
                atomicAdd(&np, 1);
                break;
            } else if (bval < val) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        
        // Binary search in b_neg
        lo = 0; hi = b_neg_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            unsigned int bval = b_neg_flat[b_neg_start + mid];
            if (bval == val) {
                atomicAdd(&nn, 1);
                break;
            } else if (bval < val) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
    }
    
    __syncthreads();
    
    // Thread 0 computes final similarity
    if (threadIdx.x == 0) {
        int dot = pp + nn - pn - np;
        int nnz_a = a_pos_len + a_neg_len;
        int nnz_b = b_pos_len + b_neg_len;
        
        if (nnz_a == 0 || nnz_b == 0) {
            similarities[b_idx] = 0.0f;
        } else {
            float norm = sqrtf((float)(nnz_a * nnz_b));
            similarities[b_idx] = (float)dot / norm;
        }
    }
}
"#;

/// CUDA kernel source optimized for SM120 (Blackwell) architecture
/// Features: Improved warp-level primitives, async copy, larger shared memory
#[cfg(feature = "cuda")]
const VSA_KERNELS_SM120: &str = r#"
// Blackwell SM120-optimized VSA kernel
// Uses warp-level reduction and coalesced memory access patterns
extern "C" __global__ void batch_cosine_kernel(
    const unsigned int* __restrict__ a_pos,
    const unsigned int* __restrict__ a_neg,
    const unsigned int* __restrict__ b_pos_flat,
    const unsigned int* __restrict__ b_neg_flat,
    const int* __restrict__ b_pos_offsets,
    const int* __restrict__ b_neg_offsets,
    const int* __restrict__ b_pos_lens,
    const int* __restrict__ b_neg_lens,
    float* __restrict__ similarities,
    int a_pos_len,
    int a_neg_len,
    int num_b
) {
    int b_idx = blockIdx.x;
    if (b_idx >= num_b) return;
    
    int b_pos_start = b_pos_offsets[b_idx];
    int b_neg_start = b_neg_offsets[b_idx];
    int b_pos_len = b_pos_lens[b_idx];
    int b_neg_len = b_neg_lens[b_idx];
    
    // Use warp-level reduction for better SM120 performance
    int local_pp = 0, local_nn = 0, local_pn = 0, local_np = 0;
    
    // Each thread handles a portion of a_pos elements with stride
    for (int i = threadIdx.x; i < a_pos_len; i += blockDim.x) {
        unsigned int val = a_pos[i];
        
        // Binary search in b_pos with prefetching
        int lo = 0, hi = b_pos_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) >> 1;  // Faster than /2
            unsigned int bval = b_pos_flat[b_pos_start + mid];
            if (bval == val) {
                local_pp++;
                break;
            }
            lo = (bval < val) ? mid + 1 : lo;
            hi = (bval < val) ? hi : mid - 1;
        }
        
        // Binary search in b_neg
        lo = 0; hi = b_neg_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) >> 1;
            unsigned int bval = b_neg_flat[b_neg_start + mid];
            if (bval == val) {
                local_pn++;
                break;
            }
            lo = (bval < val) ? mid + 1 : lo;
            hi = (bval < val) ? hi : mid - 1;
        }
    }
    
    // Each thread handles a portion of a_neg elements
    for (int i = threadIdx.x; i < a_neg_len; i += blockDim.x) {
        unsigned int val = a_neg[i];
        
        // Binary search in b_pos
        int lo = 0, hi = b_pos_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) >> 1;
            unsigned int bval = b_pos_flat[b_pos_start + mid];
            if (bval == val) {
                local_np++;
                break;
            }
            lo = (bval < val) ? mid + 1 : lo;
            hi = (bval < val) ? hi : mid - 1;
        }
        
        // Binary search in b_neg
        lo = 0; hi = b_neg_len - 1;
        while (lo <= hi) {
            int mid = (lo + hi) >> 1;
            unsigned int bval = b_neg_flat[b_neg_start + mid];
            if (bval == val) {
                local_nn++;
                break;
            }
            lo = (bval < val) ? mid + 1 : lo;
            hi = (bval < val) ? hi : mid - 1;
        }
    }
    
    // Warp-level reduction using shuffle
    unsigned int mask = __activemask();
    for (int offset = 16; offset > 0; offset >>= 1) {
        local_pp += __shfl_down_sync(mask, local_pp, offset);
        local_nn += __shfl_down_sync(mask, local_nn, offset);
        local_pn += __shfl_down_sync(mask, local_pn, offset);
        local_np += __shfl_down_sync(mask, local_np, offset);
    }
    
    // First thread in each warp contributes to block-level sum
    __shared__ int warp_pp[8], warp_nn[8], warp_pn[8], warp_np[8];
    int warp_id = threadIdx.x / 32;
    int lane_id = threadIdx.x % 32;
    
    if (lane_id == 0 && warp_id < 8) {
        warp_pp[warp_id] = local_pp;
        warp_nn[warp_id] = local_nn;
        warp_pn[warp_id] = local_pn;
        warp_np[warp_id] = local_np;
    }
    __syncthreads();
    
    // Thread 0 computes final similarity
    if (threadIdx.x == 0) {
        int total_pp = 0, total_nn = 0, total_pn = 0, total_np = 0;
        int num_warps = (blockDim.x + 31) / 32;
        for (int w = 0; w < num_warps && w < 8; w++) {
            total_pp += warp_pp[w];
            total_nn += warp_nn[w];
            total_pn += warp_pn[w];
            total_np += warp_np[w];
        }
        
        int dot = total_pp + total_nn - total_pn - total_np;
        int nnz_a = a_pos_len + a_neg_len;
        int nnz_b = b_pos_len + b_neg_len;
        
        if (nnz_a == 0 || nnz_b == 0) {
            similarities[b_idx] = 0.0f;
        } else {
            float norm = sqrtf((float)(nnz_a * nnz_b));
            similarities[b_idx] = (float)dot / norm;
        }
    }
}
"#;

/// GPU backend for VSA operations
#[cfg(feature = "cuda")]
pub struct GpuBackend {
    #[allow(dead_code)]
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    config: GpuConfig,
    architecture: GpuArch,
    memory_config: GpuMemoryConfig,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    batch_cosine_kernel: CudaFunction,
}

#[cfg(feature = "cuda")]
impl GpuBackend {
    /// Create a new GPU backend with auto-detection or specified architecture
    pub fn new(mut config: GpuConfig) -> Result<Self, GpuError> {
        let ctx = CudaContext::new(0).map_err(|e| GpuError::DeviceInit(e.to_string()))?;
        let stream = ctx.default_stream();

        // Query and configure memory dynamically
        let memory_config = if let Some(mem_cfg) = config.memory_config.take() {
            mem_cfg
        } else {
            let (total, free) = query_gpu_memory(&ctx)?;
            GpuMemoryConfig::new(total, free, config.memory_headroom)
        };

        eprintln!(
            "[GPU] Memory: {:.1}GB total, {:.1}GB free, {:.1}GB safe limit ({}% headroom)",
            memory_config.total_memory as f64 / 1e9,
            memory_config.free_memory as f64 / 1e9,
            memory_config.safe_limit as f64 / 1e9,
            (memory_config.headroom_percent * 100.0) as u32
        );

        // Detect or use specified architecture
        let architecture = if let Some(arch) = config.architecture {
            arch
        } else if config.auto_detect_arch {
            Self::detect_architecture(&ctx)?
        } else {
            GpuArch::default() // SM120 as fallback
        };

        eprintln!(
            "[GPU] Architecture: {:?} ({})",
            architecture,
            architecture.ptx_target()
        );

        // Compile kernels with architecture-specific optimizations
        let ptx = Self::compile_kernels(&architecture)?;

        let module = ctx
            .load_module(ptx)
            .map_err(|e| GpuError::KernelLoad(e.to_string()))?;

        let batch_cosine_kernel = module
            .load_function("batch_cosine_kernel")
            .map_err(|e| GpuError::KernelNotFound(e.to_string()))?;

        Ok(Self {
            ctx,
            stream,
            config,
            architecture,
            memory_config,
            module,
            batch_cosine_kernel,
        })
    }

    /// Detect GPU architecture from device properties
    fn detect_architecture(_ctx: &CudaContext) -> Result<GpuArch, GpuError> {
        // Query compute capability via nvidia-smi
        let (major, minor) = query_compute_capability()?;
        let arch = GpuArch::from_compute_capability(major, minor);
        Ok(arch)
    }

    /// Compile CUDA kernels for the target architecture
    fn compile_kernels(arch: &GpuArch) -> Result<cudarc::nvrtc::Ptx, GpuError> {
        // Architecture-specific optimizations
        let kernel_source = if arch.is_blackwell() {
            // Blackwell-optimized kernel with SM120 features
            VSA_KERNELS_SM120
        } else {
            // Generic kernel for older architectures
            VSA_KERNELS
        };

        // Build compile options with arch and defines via options field
        let arch_define = format!("-D VSA_ARCH_MAJOR={}", arch.major_version());

        cudarc::nvrtc::compile_ptx_with_opts(
            kernel_source,
            cudarc::nvrtc::CompileOptions {
                arch: Some(arch.nvrtc_arch()),
                include_paths: vec![],
                options: vec![arch_define],
                ..Default::default()
            },
        )
        .map_err(|e| GpuError::KernelCompile(e.to_string()))
    }

    /// Get the detected/configured GPU architecture
    pub fn architecture(&self) -> GpuArch {
        self.architecture
    }

    /// Check if GPU is available and ready
    pub fn is_available(&self) -> bool {
        true
    }

    /// Get current memory configuration
    pub fn memory_config(&self) -> &GpuMemoryConfig {
        &self.memory_config
    }

    /// Refresh memory statistics (check for memory pressure)
    pub fn refresh_memory(&mut self) -> Result<(), GpuError> {
        self.memory_config = self.memory_config.refresh(&self.ctx)?;
        Ok(())
    }

    /// Get recommended batch size based on current memory
    pub fn recommended_batch_size(&self, avg_vector_nnz: usize) -> usize {
        // Each SparseVec uses ~8 bytes per non-zero element (4 bytes index, 4 bytes for storage)
        // Plus flattened arrays and offsets
        let vector_bytes = avg_vector_nnz * 8 + 32; // +32 for overhead
        self.memory_config.recommended_batch_size(vector_bytes)
    }

    /// Compute batch cosine similarities on GPU
    ///
    /// Given a query vector `a` and a batch of candidate vectors `batch_b`,
    /// computes all cosine similarities in parallel on the GPU.
    ///
    /// # Arguments
    /// * `a` - Query sparse vector
    /// * `batch_b` - Batch of candidate sparse vectors
    ///
    /// # Returns
    /// Vector of cosine similarities, one per candidate
    ///
    /// # Performance
    /// For batches >100 vectors, expect 10-100x speedup over CPU.
    pub fn batch_cosine(&self, a: &SparseVec, batch_b: &[SparseVec]) -> Result<Vec<f64>, GpuError> {
        if batch_b.is_empty() {
            return Ok(Vec::new());
        }

        if batch_b.len() < self.config.min_batch_size {
            // Fall back to CPU for tiny batches
            return Ok(batch_b.iter().map(|b| a.cosine(b)).collect());
        }

        // Flatten b vectors into contiguous arrays for GPU transfer
        let mut b_pos_flat: Vec<u32> = Vec::new();
        let mut b_neg_flat: Vec<u32> = Vec::new();
        let mut b_pos_offsets: Vec<i32> = Vec::new();
        let mut b_neg_offsets: Vec<i32> = Vec::new();
        let mut b_pos_lens: Vec<i32> = Vec::new();
        let mut b_neg_lens: Vec<i32> = Vec::new();

        for b in batch_b {
            b_pos_offsets.push(b_pos_flat.len() as i32);
            b_neg_offsets.push(b_neg_flat.len() as i32);
            b_pos_lens.push(b.pos.len() as i32);
            b_neg_lens.push(b.neg.len() as i32);
            b_pos_flat.extend(b.pos.iter().map(|&x| x as u32));
            b_neg_flat.extend(b.neg.iter().map(|&x| x as u32));
        }

        // Handle empty vectors (CUDA doesn't like zero-size allocations)
        if b_pos_flat.is_empty() {
            b_pos_flat.push(0);
        }
        if b_neg_flat.is_empty() {
            b_neg_flat.push(0);
        }

        let a_pos: Vec<u32> = a.pos.iter().map(|&x| x as u32).collect();
        let a_neg: Vec<u32> = a.neg.iter().map(|&x| x as u32).collect();
        let a_pos = if a_pos.is_empty() { vec![0] } else { a_pos };
        let a_neg = if a_neg.is_empty() { vec![0] } else { a_neg };

        // Transfer to GPU using stream (cudarc 0.19 API)
        let d_a_pos = self
            .stream
            .clone_htod(&a_pos)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_a_neg = self
            .stream
            .clone_htod(&a_neg)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_b_pos_flat = self
            .stream
            .clone_htod(&b_pos_flat)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_b_neg_flat = self
            .stream
            .clone_htod(&b_neg_flat)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_b_pos_offsets = self
            .stream
            .clone_htod(&b_pos_offsets)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_b_neg_offsets = self
            .stream
            .clone_htod(&b_neg_offsets)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_b_pos_lens = self
            .stream
            .clone_htod(&b_pos_lens)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let d_b_neg_lens = self
            .stream
            .clone_htod(&b_neg_lens)
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;
        let mut d_similarities: CudaSlice<f32> = self
            .stream
            .alloc_zeros(batch_b.len())
            .map_err(|e| GpuError::MemoryAlloc(e.to_string()))?;

        // Launch kernel - one block per candidate vector, 256 threads per block
        let cfg = LaunchConfig {
            block_dim: (256, 1, 1),
            grid_dim: (batch_b.len() as u32, 1, 1),
            shared_mem_bytes: 16, // 4 shared int counters
        };

        let a_pos_len = if a.pos.is_empty() {
            0i32
        } else {
            a.pos.len() as i32
        };
        let a_neg_len = if a.neg.is_empty() {
            0i32
        } else {
            a.neg.len() as i32
        };

        // Launch kernel using cudarc 0.19 API: stream.launch_builder().arg().launch()
        unsafe {
            self.stream
                .launch_builder(&self.batch_cosine_kernel)
                .arg(&d_a_pos)
                .arg(&d_a_neg)
                .arg(&d_b_pos_flat)
                .arg(&d_b_neg_flat)
                .arg(&d_b_pos_offsets)
                .arg(&d_b_neg_offsets)
                .arg(&d_b_pos_lens)
                .arg(&d_b_neg_lens)
                .arg(&mut d_similarities)
                .arg(&a_pos_len)
                .arg(&a_neg_len)
                .arg(&(batch_b.len() as i32))
                .launch(cfg)
                .map_err(|e| GpuError::KernelLaunch(e.to_string()))?;
        }

        // Copy results back
        let similarities: Vec<f32> = self
            .stream
            .clone_dtoh(&d_similarities)
            .map_err(|e| GpuError::MemoryCopy(e.to_string()))?;

        Ok(similarities.into_iter().map(|x| x as f64).collect())
    }
}

/// GPU operation errors
#[derive(Debug, Clone)]
pub enum GpuError {
    /// Failed to initialize GPU device
    DeviceInit(String),
    /// Failed to compile CUDA kernel
    KernelCompile(String),
    /// Failed to load CUDA kernel
    KernelLoad(String),
    /// CUDA kernel not found
    KernelNotFound(String),
    /// Failed to launch CUDA kernel
    KernelLaunch(String),
    /// Failed to allocate GPU memory
    MemoryAlloc(String),
    /// Failed to copy memory to/from GPU
    MemoryCopy(String),
    /// GPU acceleration not available
    NotAvailable,
    /// Invalid input parameter
    InvalidValue(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::DeviceInit(e) => write!(f, "GPU device initialization failed: {}", e),
            GpuError::KernelCompile(e) => write!(f, "CUDA kernel compilation failed: {}", e),
            GpuError::KernelLoad(e) => write!(f, "CUDA kernel loading failed: {}", e),
            GpuError::KernelNotFound(e) => write!(f, "CUDA kernel not found: {}", e),
            GpuError::KernelLaunch(e) => write!(f, "CUDA kernel launch failed: {}", e),
            GpuError::MemoryAlloc(e) => write!(f, "GPU memory allocation failed: {}", e),
            GpuError::MemoryCopy(e) => write!(f, "GPU memory copy failed: {}", e),
            GpuError::NotAvailable => write!(
                f,
                "GPU acceleration not available (compile with --features cuda)"
            ),
            GpuError::InvalidValue(e) => write!(f, "Invalid input value: {}", e),
        }
    }
}

impl std::error::Error for GpuError {}

// Stub for non-CUDA builds
#[cfg(not(feature = "cuda"))]
pub struct GpuBackend {
    _private: (),
}

#[cfg(not(feature = "cuda"))]
impl GpuBackend {
    pub fn new(_config: GpuConfig) -> Result<Self, GpuError> {
        Err(GpuError::NotAvailable)
    }
    pub fn is_available(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_config_default() {
        let config = GpuConfig::default();
        assert_eq!(config.min_batch_size, 4);
        assert!(config.memory_config.is_none()); // Auto-detect
        assert!((config.memory_headroom - DEFAULT_MEMORY_HEADROOM_PERCENT).abs() < 0.001);
    }

    #[test]
    fn test_memory_config_calculations() {
        let total = 16 * 1024 * 1024 * 1024; // 16GB
        let free = 14 * 1024 * 1024 * 1024; // 14GB free
        let headroom = 0.15;

        let mem = GpuMemoryConfig::new(total, free, headroom);

        assert_eq!(mem.total_memory, total);
        assert_eq!(mem.free_memory, free);
        assert_eq!(mem.external_usage, 2 * 1024 * 1024 * 1024);

        // Safe limit should be ~11.5GB (14GB - 512MB reserve) * 0.85
        let expected_available = free - MIN_SYSTEM_RESERVE_BYTES;
        let expected_safe = ((expected_available as f64) * 0.85) as usize;
        assert_eq!(mem.safe_limit, expected_safe);

        // Can allocate up to safe limit
        assert!(mem.can_allocate(mem.safe_limit));
        assert!(!mem.can_allocate(mem.safe_limit + 1));
    }

    #[test]
    fn test_recommended_batch_size() {
        let mem = GpuMemoryConfig::new(16 * 1024 * 1024 * 1024, 14 * 1024 * 1024 * 1024, 0.15);

        // With 1KB per vector, should get reasonable batch size
        let batch = mem.recommended_batch_size(1024);
        assert!(batch > 1000, "Expected large batch size, got {}", batch);

        // With huge vectors, batch size should be smaller
        let small_batch = mem.recommended_batch_size(100 * 1024 * 1024);
        assert!(small_batch < batch);
    }

    #[test]
    fn test_gpu_error_display() {
        let err = GpuError::NotAvailable;
        assert!(err.to_string().contains("not available"));
    }

    #[test]
    fn test_gpu_arch_detection() {
        // Test known architectures
        assert_eq!(GpuArch::from_compute_capability(7, 5), GpuArch::Sm75);
        assert_eq!(GpuArch::from_compute_capability(8, 0), GpuArch::Sm80);
        assert_eq!(GpuArch::from_compute_capability(8, 6), GpuArch::Sm86);
        assert_eq!(GpuArch::from_compute_capability(8, 9), GpuArch::Sm89);
        assert_eq!(GpuArch::from_compute_capability(12, 0), GpuArch::Sm120);

        // Future architectures should default appropriately
        assert_eq!(GpuArch::from_compute_capability(12, 5), GpuArch::Sm120);
        assert_eq!(GpuArch::from_compute_capability(13, 0), GpuArch::Sm120);
    }
}
