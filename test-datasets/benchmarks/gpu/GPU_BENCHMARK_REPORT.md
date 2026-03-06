# Embeddenator GPU Benchmark Report

**Date:** 2026-01-29T03:22:12-05:00
**Host:** akula-prime

## GPU Configuration

```
name, memory.total [MiB], memory.free [MiB], driver_version, compute_cap
NVIDIA GeForce RTX 5080, 16303 MiB, 10379 MiB, 590.48.01, 12.0
```

---

## Benchmark Results

### 1. VSA Core Operations (CUDA)

Testing sparse ternary vector operations with CUDA acceleration.


#### CUDA Kernel Benchmarks

```
                 change:
                        time:   [−2.6582% −2.2887% −1.9472%] (p = 0.00 < 0.05)
                        thrpt:  [+1.9858% +2.3423% +2.7308%]
                        Performance has improved.
Found 7 outliers among 100 measurements (7.00%)
  4 (4.00%) high mild
  3 (3.00%) high severe
Benchmarking cuda_batch_cosine/gpu_batch/1000
Benchmarking cuda_batch_cosine/gpu_batch/1000: Warming up for 3.0000 s
Benchmarking cuda_batch_cosine/gpu_batch/1000: Collecting 100 samples in estimated 5.3120 s (45k iterations)
Benchmarking cuda_batch_cosine/gpu_batch/1000: Analyzing
cuda_batch_cosine/gpu_batch/1000
                        time:   [114.48 µs 114.81 µs 115.23 µs]
                        thrpt:  [8.6781 Melem/s 8.7098 Melem/s 8.7352 Melem/s]
                 change:
                        time:   [−3.8016% −2.8521% −1.9871%] (p = 0.00 < 0.05)
                        thrpt:  [+2.0274% +2.9358% +3.9518%]
                        Performance has improved.
Found 8 outliers among 100 measurements (8.00%)
  4 (4.00%) high mild
  4 (4.00%) high severe
Benchmarking cuda_batch_cosine/cpu_sequential/5000
Benchmarking cuda_batch_cosine/cpu_sequential/5000: Warming up for 3.0000 s
Benchmarking cuda_batch_cosine/cpu_sequential/5000: Collecting 100 samples in estimated 5.9057 s (500 iterations)
Benchmarking cuda_batch_cosine/cpu_sequential/5000: Analyzing
cuda_batch_cosine/cpu_sequential/5000
                        time:   [11.777 ms 11.832 ms 11.894 ms]
                        thrpt:  [420.37 Kelem/s 422.60 Kelem/s 424.57 Kelem/s]
                 change:
                        time:   [−1.3473% −0.2394% +0.6300%] (p = 0.67 > 0.05)
                        thrpt:  [−0.6261% +0.2400% +1.3657%]
                        No change in performance detected.
Found 16 outliers among 100 measurements (16.00%)
  6 (6.00%) high mild
  10 (10.00%) high severe
Benchmarking cuda_batch_cosine/gpu_batch/5000
Benchmarking cuda_batch_cosine/gpu_batch/5000: Warming up for 3.0000 s
Benchmarking cuda_batch_cosine/gpu_batch/5000: Collecting 100 samples in estimated 5.1290 s (2200 iterations)
Benchmarking cuda_batch_cosine/gpu_batch/5000: Analyzing
cuda_batch_cosine/gpu_batch/5000
                        time:   [2.2703 ms 2.3018 ms 2.3360 ms]
                        thrpt:  [2.1404 Melem/s 2.1722 Melem/s 2.2024 Melem/s]
                 change:
                        time:   [+1.0280% +2.7329% +4.3704%] (p = 0.00 < 0.05)
                        thrpt:  [−4.1874% −2.6602% −1.0176%]
                        Performance has regressed.
Found 11 outliers among 100 measurements (11.00%)
  9 (9.00%) high mild
  2 (2.00%) high severe

```

### 2. CUDA Unit Tests

```
test test_encode_block_directly ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test src/coherency.rs - coherency (line 28) ... ignored
test src/gpu.rs - gpu (line 38) ... ignored
test src/gpu_encoding.rs - gpu_encoding (line 16) ... ignored
test src/gpu_kernels.rs - gpu_kernels (line 21) ... ignored
test src/lib.rs - (line 23) ... ok
test src/phase_training.rs - phase_training (line 23) ... ignored
test src/resonator.rs - resonator (line 34) ... ignored
test src/reversible_encoding.rs - reversible_encoding::ReversibleVSAEncoder::batch_encode (line 359) ... ignored
test src/reversible_encoding.rs - reversible_encoding::ReversibleVSAEncoder::ensure_positions (line 281) ... ignored
test src/virtual_memory.rs - virtual_memory (line 37) ... ignored
test src/vram_pool.rs - vram_pool (line 15) ... ignored
test src/vsa.rs - vsa::SparseVec::bind (line 1241) ... ok
test src/vsa.rs - vsa::SparseVec::bundle (line 1042) ... ok
test src/vsa.rs - vsa::SparseVec::bundle_with_config (line 1002) ... ok
test src/vsa.rs - vsa::SparseVec::cosine (line 1377) ... ok
test src/vsa.rs - vsa::SparseVec::decode_data (line 740) - compile ... ok
test src/vsa.rs - vsa::SparseVec::encode_data (line 665) ... ok
test src/vsa.rs - vsa::SparseVec::from_data (line 948) ... ok
test src/vsa.rs - vsa::SparseVec::inverse_permute (line 1500) ... ok
test src/vsa.rs - vsa::SparseVec::new (line 564) ... ok
test src/vsa.rs - vsa::SparseVec::permute (line 1466) ... ok
test src/vsa.rs - vsa::SparseVec::random (line 582) ... ok
test src/vsa.rs - vsa::SparseVec::random_with_config (line 618) ... ok
test src/vsa.rs - vsa::VsaConfig (line 28) ... ok
test src/vsa.rs - vsa::VsaConfig::for_accuracy (line 188) ... ok
test src/vsa.rs - vsa::VsaConfig::from_schema (line 231) ... ok
test src/vsa.rs - vsa::VsaConfigSchema (line 312) ... ok
test result: ok. 17 passed; 0 failed; 10 ignored; 0 measured; 0 filtered out; finished in 0.80s
```

### 3. GPU Memory Transfer Performance

| Operation | Size | Time | Bandwidth |
|-----------|------|------|-----------|
| GPU Test Run | - | - | Peak VRAM: 0MB |

### 4. Batch Encoding: CPU vs GPU

Testing 10MB batch encoding...

#### CPU Baseline
- Time: 507.090118029s
- Throughput: .01 MB/s


---

## Summary

### GPU Utilization
- Peak VRAM: 0MB additional during tests
- GPU Driver: 590.48.01

### Performance Analysis

The GPU benchmarks test:
1. **CUDA Kernel Operations** - Batch decode, cosine similarity
2. **Memory Transfers** - Host-to-device and device-to-host bandwidth
3. **Batch Processing** - Parallel encoding/decoding throughput

### Recommendations

- GPU acceleration is most beneficial for:
  - Batch operations (>1000 vectors)
  - Large-scale cosine similarity searches
  - Parallel decoding of multiple positions

- CPU is preferred for:
  - Small datasets (<1MB)
  - Single-vector operations
  - Sequential access patterns

---

*Generated by embeddenator GPU benchmark suite*
