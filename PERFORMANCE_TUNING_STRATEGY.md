# Embeddenator Performance Tuning - Complete Strategy

## Overview

Systematic performance optimization roadmap for Embeddenator to achieve **sub-100ms operations** and **<1GB memory for TB-scale data**, building on baseline performance of 16.6 MB/s ingestion and 40.8 MB/s extraction.

---

## Phase 1: Quick Wins (This Week) ✅ IMPLEMENTED

### Objective
50-80% performance improvement with minimal code changes.

### Changes Implemented

#### 1. Compiler Optimization Profile
- **opt-level = 3**: Maximum optimization
- **lto = "fat"**: Full Link Time Optimization
- **codegen-units = 1**: Single codegen unit
- **panic = "abort"**: No unwinding overhead
- **New profile**: `release-perf` for explicit tuning

**Expected Impact: +10-15% throughput**

#### 2. Performance Dependencies Added
```toml
rayon = "1.7"           # Data parallelism
num_cpus = "1.16"       # CPU detection
parking_lot = "0.12"    # Faster locks
smallvec = "1.11"       # Stack-allocated vectors
```

**Expected Impact: Foundation for parallelism**

#### 3. Performance Configuration Module
Created `src/perf.rs` with:

```rust
pub struct PerfConfig {
    // Detected at runtime
    cpu_cores: usize,
    ingest_threads: usize,    // cpu_cores / 2 (I/O bound)
    extract_threads: usize,   // cpu_cores (CPU intensive)
    query_threads: usize,     // cpu_cores * 2/3 (balanced)
    batch_size: usize,        // 262,144 (8MB L3 cache)
    chunk_size: usize,        // 512 (per-thread)
}

impl PerfConfig {
    pub fn detect() -> Self          // Auto-detect
    pub fn ingest_pool() -> ThreadPool   // I/O optimized
    pub fn extract_pool() -> ThreadPool  // Compute optimized
    pub fn query_pool() -> ThreadPool    // Balanced
}
```

**Expected Impact: +20-30% from proper thread pool sizing**

#### 4. CLI Integration
Updated CLI to display perf config when verbose:
```
Performance Configuration:
  CPU Cores: 28
  Ingest Threads: 14
  Batch Size: 262,144 vectors
  Target: 30+ MB/s
```

#### 5. Validation Script
Created `perf_validate.sh` for automated benchmarking:
- Tests 100MB, 500MB, 1GB datasets
- Measures ingestion/extraction rates
- Compares vs baseline
- Reports if targets met

### Expected Targets (Phase 1)
```
Ingestion:  16.6 MB/s → 25-33 MB/s  (50% improvement)
Extraction: 40.8 MB/s → 60+ MB/s    (50% improvement)
Memory:     No change yet
```

---

## Phase 2: Memory Optimization (Weeks 2-3)

### Objective
30-50% memory overhead reduction while maintaining performance.

### Planned Changes

#### 2.1 Sparse Vector Packing
Current: 128-bit per vector
Target: 39-40 trits per 64-bit word

```rust
// Current efficiency: 1 trinary value ≈ 1.585 bits
// Can pack 40 trits into 63.4 bits ≈ 64-bit word
// Reduces memory by 50%

struct SparseVecOptimized {
    data: u64,              // 40 ternary values packed
    indices: SmallVec<[u16; 8]>,  // Non-zero positions
}
```

**Impact: 40-50% memory reduction, minimal latency cost**

#### 2.2 Compression Integration
Add zstd compression to engram pipeline:

```rust
// Fast decompression: ~2GB/s
// Compression ratio: 50-70%
// CPU overhead: <5%

const COMPRESSION_LEVEL: i32 = 10;  // 1-22 scale
let compressed = zstd::encode_all(&engram_data, COMPRESSION_LEVEL)?;
```

**Impact: 35-50% storage reduction**

#### 2.3 Memory-Mapped I/O
For files >500MB, use mmap:

```rust
use memmap2::Mmap;

let file = File::open("large.engram")?;
let mmap = unsafe { Mmap::map(&file)? };

// Lazy loading in 1MB windows
for window in mmap.chunks(1024 * 1024) {
    process_window(window);
}
```

**Impact: O(1) memory regardless of file size**

### Phase 2 Targets
```
Ingestion:  30+ MB/s → 50+ MB/s   (additional 65%)
Extraction: 60+ MB/s → 100+ MB/s  (additional 50%)
Memory:     286% overhead → 200% (30% reduction)
Storage:    Compressed 35-50% smaller
```

---

## Phase 3: Algorithmic Acceleration (Next Quarter)

### 3.1 GPU Acceleration (CUDA)

Offload most expensive operations to GPU:

```rust
// GPU-optimized operations (5-20x speedup)
// 1. Cosine distance calculations (embarrassingly parallel)
// 2. Vector binding operations (matrix ops)
// 3. Unbinding/reconstruction (linear algebra)

// Memory transfer optimization
// - Batch transfers >100MB for efficiency
// - Pinned memory for DMA
// - Overlapped compute/transfer with streams
```

**Impact: 5-20x speedup for GPU-suitable ops**

### 3.2 Distributed Processing

Multi-node scaling:

```rust
// Sharded ingestion (each node processes partition)
// Distributed cosine search (hierarchical aggregation)
// Hierarchical reconstruction (bottom-up merging)
```

**Impact: Near-linear scaling to multiple nodes**

### 3.3 Incremental Updates

Delta encoding for fast updates:

```rust
// Only re-encode modified chunks
// Merge new vectors with existing engram
// Update manifest incrementally
// 100-1000x faster on small changes
```

---

## Detailed Performance Roadmap

### Timeline
```
Week 1-2:  Phase 1 - Quick Wins (50-80% improvement)
Week 3-4:  Phase 2 - Memory Optimization (additional 65% improvement)
Month 2:   Validation & Benchmarking on 20GB+ datasets
Q2 2026:   Phase 3 - GPU Acceleration (5-20x speedup)
```

### Cumulative Performance Impact

| Phase | Ingestion | Extraction | Memory | Timeline |
|-------|-----------|-----------|--------|----------|
| Baseline | 16.6 MB/s | 40.8 MB/s | 286% | Current |
| Phase 1 | 25-33 | 60-65 | 286% | Week 1-2 |
| Phase 1+2 | 40-50 | 90-100 | 200% | Week 3-4 |
| Phase 1+2+3 | 200+ | 500+ | <1GB/TB | Q2 2026 |

### Final Targets

```
100MB dataset:
  Ingestion:  16.6s → 0.5s   (33x speedup)
  Extraction: 2.5s  → 0.2s   (12x speedup)

1GB dataset:
  Ingestion:  166s → 20s     (8x speedup)
  Extraction: 25s → 10s      (2.5x speedup)
  Memory:     <100MB resident (from ~300MB)

100GB dataset:
  Ingestion:  ~30 minutes (with mmap + streaming)
  Extraction: ~12 minutes
  Memory:     <1GB resident (key requirement)

Operations:   <100ms latency (from 6-10s currently)
```

---

## Profiling and Validation

### Current Profiling Strategy

```bash
# Identify hotspots (Linux)
perf record -F 99 ./target/release/embeddenator ingest -i large.bin -e out.engram
perf report

# Memory profiling
valgrind --tool=massif ./target/release/embeddenator ingest
ms_print massif.out.*

# Cache efficiency
perf stat -e cache-references,cache-misses,cycles,instructions \
  ./target/release/embeddenator ingest

# Flamegraph visualization
cargo install flamegraph
cargo flamegraph --release -- ingest -i large.bin -e out.engram
```

### Benchmark Suite

```bash
# Performance validation
cargo bench --bench performance_validation

# Optimization validation
cargo bench --bench optimization_validation

# Large-scale operations
cargo bench --bench large_scale_operations --features large-scale
```

### Automated Validation

```bash
# Phase 1 validation script (included)
chmod +x perf_validate.sh
./perf_validate.sh

# Expected output: Ingestion 25-33 MB/s, Extraction 60+ MB/s
```

---

## Implementation Priority Matrix

| Task | Effort | Impact | Risk | Priority |
|------|--------|--------|------|----------|
| Compiler opts | LOW | 15% | NONE | 🔴 CRITICAL |
| Thread pool tuning | LOW | 30% | NONE | 🔴 CRITICAL |
| Batch optimization | LOW | 25% | NONE | 🔴 CRITICAL |
| SIMD depth | MEDIUM | 40% | LOW | 🟠 HIGH |
| Sparse packing | MEDIUM | 50% | MEDIUM | 🟠 HIGH |
| Compression | MEDIUM | 50% | LOW | 🟠 HIGH |
| Memory-mapped I/O | MEDIUM | UNBOUNDED | MEDIUM | 🟠 HIGH |
| GPU acceleration | HIGH | 500% | MEDIUM | 🟡 MEDIUM |
| Distributed | HIGH | SCALING | HIGH | 🟡 MEDIUM |

---

## Component Architecture Considerations

The actual ingestion/extraction logic is in external component repos:

### embeddenator-fs (File System Layer)
Location: `https://github.com/tzervas/embeddenator-fs`

Methods to optimize:
- `EmbrFS::ingest_directory()` - Directory walk and file ingestion
- `EmbrFS::ingest_file()` - Individual file chunking and encoding
- `EmbrFS::extract()` - Extraction and reconstruction

**Should use:**
```rust
let pool = embeddenator::perf::config().ingest_pool();
pool.install(|| {
    // Process files in parallel
});
```

### embeddenator-vsa (VSA Implementation)
Location: `https://github.com/tzervas/embeddenator-vsa`

Methods to optimize:
- `SparseVec::encode_data()` - Core encoding operation
- `SparseVec::decode_data()` - Core decoding operation
- `SparseVec::bind()` - Vector composition
- `SparseVec::bundle()` - Vector superposition

**Should use:**
```rust
let cfg = embeddenator::perf::config();
// Use batch_size for processing: cfg.batch_size
// Use chunk_size for prefetching: cfg.chunk_size
```

---

## Regression Prevention

### Automated Testing

```rust
#[test]
fn perf_no_regression() {
    let baseline = 16.6;  // MB/s
    let minimum = baseline * 0.95;  // Allow 5% variance
    
    let measured = benchmark_ingest();
    assert!(measured >= minimum, 
            "Regression: {} < {}", measured, minimum);
}
```

### Continuous Monitoring

Track metrics in every PR:
- Ingestion throughput (MB/s)
- Extraction throughput (MB/s)
- Memory overhead (%)
- Cache miss rate (%)
- Latency p50/p95/p99

---

## Success Criteria

### Phase 1 (This Week) ✅
- [x] Compiler optimizations applied
- [x] Performance config module created
- [x] Dependencies added
- [x] Validation script prepared
- [ ] Benchmark shows 25-33 MB/s ingestion
- [ ] Benchmark shows 60+ MB/s extraction

### Phase 2 (Weeks 2-3)
- [ ] Sparse packing implemented
- [ ] Compression integrated
- [ ] Memory-mapped I/O working
- [ ] 50+ MB/s ingestion achieved
- [ ] 100+ MB/s extraction achieved
- [ ] 200% memory overhead (down from 286%)

### Phase 3 (Q2)
- [ ] GPU backend implemented
- [ ] 200+ MB/s ingestion with GPU
- [ ] 500+ MB/s extraction with GPU
- [ ] <100ms latency on 1GB datasets
- [ ] <1GB memory for TB-scale data

---

## Quick Start

### Build Optimized Binary
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator
cargo build --release --features "bt-phase-2,simd"
```

### View Performance Config
```bash
./target/release/embeddenator ingest -i test.bin -e out.engram -m out.json -v
```

### Run Validation
```bash
chmod +x perf_validate.sh
./perf_validate.sh
```

### Benchmark Operations
```bash
cargo bench --bench performance_validation
```

---

## References

- Evaluation Results: `/home/kang/Documents/projects/embdntr/embeddenator-testkit/EVALUATION_RESULTS.md`
- Thread Pool Tuning: `src/perf.rs`
- CLI Integration: `src/cli.rs`
- Validation: `perf_validate.sh`

---

**Status: Phase 1 Implementation Complete - Ready for Benchmarking**

Next: Run `./perf_validate.sh` to measure improvements
