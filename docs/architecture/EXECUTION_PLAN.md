# Embeddenator Holographic FS Maturity Execution Plan

**Version:** 1.0.0
**Created:** 2026-01-26
**Target:** Full VSA holographic computational paradigm with mutable latent-space filesystem

---

## Executive Summary

This plan closes all gaps to achieve a production-ready holographic filesystem with:
- **Full write support** in latent/engram space
- **Adaptive dimensionality** (10K floor, 250K ceiling)
- **Improved accuracy** with <2% correction overhead target
- **Maximized storage density** and performance
- **128 chunks/level hierarchical fanout** for deep paths

---

## Configuration Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Dimension/Density | Adaptive 10K-250K | Balance accuracy vs. performance based on content entropy |
| Correction Strategy | Heuristic → Hybrid (future) | Start simple, evolve to entropy-threshold hybrid classifier |
| Hierarchy Fanout | 128 chunks/level | Supports ~128^3 = 2M chunks (8GB @ 4KB) per 3-level tree |

---

## Phase 1: VSA Foundation Hardening (Week 1-2)

### 1.1 ARM NEON Explicit Intrinsics

**File:** `embeddenator-vsa/src/simd.rs`

**Tasks:**
- [ ] Implement explicit `vld1q_s8`, `vmulq_s8`, `vaddq_s32` for trit multiply-accumulate
- [ ] Add `vpaddlq_s16` for horizontal reduction
- [ ] Benchmark against auto-vectorized baseline (target: 2x speedup)
- [ ] Add feature detection for NEON vs. fallback

**Acceptance Criteria:**
- [ ] `cargo bench --features simd` shows ≥2x speedup on aarch64
- [ ] All existing tests pass on ARM targets

### 1.2 Block-Sparse Production Hardening

**File:** `embeddenator-vsa/src/block_sparse.rs`

**Tasks:**
- [ ] Add bounds checking for block index operations
- [ ] Implement `BlockSparseVec::validate()` integrity check
- [ ] Add panic recovery with graceful degradation to dense
- [ ] Document block size tuning guidelines (64, 128, 256)

**Acceptance Criteria:**
- [ ] `cargo test --features block-sparse` passes with no panics
- [ ] Fuzz testing for 1M random operations without crash

### 1.3 Adaptive Dimensionality Controller

**File:** `embeddenator-vsa/src/config.rs` (new)

**Tasks:**
- [ ] Create `AdaptiveDimensionConfig` struct:
  ```rust
  pub struct AdaptiveDimensionConfig {
      pub floor: usize,      // 10_000
      pub ceiling: usize,    // 250_000
      pub entropy_threshold_low: f64,   // 0.3 → use floor
      pub entropy_threshold_high: f64,  // 0.8 → use ceiling
      pub scaling_curve: ScalingCurve,  // Linear, Logarithmic, Stepped
  }
  ```
- [ ] Implement entropy estimation from first N bytes of content
- [ ] Add `select_dimension(entropy: f64) -> usize` function
- [ ] Wire into `EncodingConfig::for_content(data: &[u8])`

**Acceptance Criteria:**
- [ ] Text files encode at ~10K-20K dimensions
- [ ] Binary/compressed files encode at ~100K-250K dimensions
- [ ] Dimension selection completes in <1ms for 1MB sample

---

## Phase 2: Large-File & Hierarchy Fixes (Week 2-3)

### 2.1 Increase Hierarchical Fanout to 128

**File:** `embeddenator-fs/src/hierarchical.rs`

**Tasks:**
- [ ] Change `MAX_CHUNKS_PER_LEVEL` from 64 to 128
- [ ] Update `HierarchicalManifest` serialization version to 2
- [ ] Add migration path for v1 manifests
- [ ] Recalculate capacity: 128^3 = 2,097,152 chunks (~8GB @ 4KB chunks)

**Acceptance Criteria:**
- [ ] Existing v1 manifests load and auto-upgrade
- [ ] New manifests use fanout=128

### 2.2 Large-File Quality Validation

**File:** `embeddenator-fs/src/large_file.rs`

**Tasks:**
- [ ] Create test corpus: 1MB, 10MB, 100MB, 1GB files (mixed entropy)
- [ ] Measure raw accuracy (pre-correction) at each size
- [ ] Measure correction overhead percentage
- [ ] Tune `HIERARCHICAL_THRESHOLD` and `CHUNK_SIZE_ADAPTIVE` thresholds
- [ ] Target: >95% raw accuracy for 10MB+ structured data

**Test Matrix:**

| File Size | Type | Target Raw Accuracy | Max Correction Overhead |
|-----------|------|---------------------|------------------------|
| 1MB | Text | 98% | 1% |
| 10MB | Text | 96% | 2% |
| 100MB | Mixed | 94% | 3% |
| 1GB | Binary | 90% | 5% |

**Acceptance Criteria:**
- [ ] All targets met in test matrix
- [ ] No quality degradation vs. baseline at any size

### 2.3 Deep Hierarchy Path Validation

**File:** `embeddenator-fs/tests/deep_hierarchy.rs` (new)

**Tasks:**
- [ ] Generate test tree with 30 levels, 10 files per level
- [ ] Encode entire tree to engram
- [ ] Verify bit-perfect reconstruction of all paths
- [ ] Measure latency for leaf file access

**Acceptance Criteria:**
- [ ] 30-level paths reconstruct correctly
- [ ] Leaf access latency <100ms for cached manifest

---

## Phase 3: CLI Update Commands (Week 3-4)

### 3.1 Implement Core Update Operations

**File:** `embeddenator-fs/src/lib.rs`

**Tasks:**
- [ ] Implement `EmbrFS::add_file(path, data) -> Result<()>`
  - Encode new file to chunk vectors
  - Bundle into existing root engram
  - Update manifest with new entry
  - Generate correction layer
- [ ] Implement `EmbrFS::remove_file(path) -> Result<()>`
  - Mark file as deleted in manifest (soft delete)
  - Optionally unbundle from root (hard delete)
- [ ] Implement `EmbrFS::modify_file(path, data) -> Result<()>`
  - Unbundle old chunks, bundle new chunks
  - Delta-encode correction layer
- [ ] Implement `EmbrFS::compact() -> Result<CompactStats>`
  - Remove soft-deleted entries
  - Rebuild root engram from active files
  - Recalculate optimal dimension

**Acceptance Criteria:**
- [ ] `embeddenator update add-file` works end-to-end
- [ ] `embeddenator update remove-file` works end-to-end
- [ ] `embeddenator update modify-file` works end-to-end
- [ ] `embeddenator update compact` reduces engram size

### 3.2 Wire CLI Commands

**File:** `embeddenator-cli/src/commands/update.rs`

**Tasks:**
- [ ] Replace TODO stubs with actual calls to `embeddenator-fs`
- [ ] Add progress reporting for large operations
- [ ] Add `--dry-run` flag for all update commands

**Acceptance Criteria:**
- [ ] All four commands execute without "not implemented" error
- [ ] Help text documents all options

---

## Phase 4: FUSE Write Hardening (Week 4-5)

### 4.1 Error Recovery & Retry Logic

**File:** `embeddenator-fs/src/fuse/versioned_fuse.rs`

**Tasks:**
- [ ] Wrap all I/O operations in retry logic (3 attempts, exponential backoff)
- [ ] Add `sync` call after write batches
- [ ] Implement write-ahead log for crash recovery
- [ ] Add `fsck` equivalent for engram integrity check

**Acceptance Criteria:**
- [ ] Kill -9 during write → data recoverable on remount
- [ ] No silent data corruption under stress

### 4.2 Signal Handling & Graceful Unmount

**File:** `embeddenator-fs/src/fuse/versioned_fuse.rs`

**Tasks:**
- [ ] Register SIGINT, SIGTERM, SIGHUP handlers
- [ ] Flush pending writes on signal
- [ ] Release FUSE session cleanly
- [ ] Add `--foreground` and `--debug` mount options

**Acceptance Criteria:**
- [ ] Ctrl+C unmounts cleanly
- [ ] `umount /mnt/engram` succeeds without force

### 4.3 Extended Attributes

**File:** `embeddenator-fs/src/fuse/versioned_fuse.rs`

**Tasks:**
- [ ] Implement `getxattr` for reading engram metadata
- [ ] Implement `setxattr` for custom attributes
- [ ] Implement `listxattr` and `removexattr`
- [ ] Store xattrs in manifest metadata field

**Standard Attributes:**
- `user.engram.version` - Engram format version
- `user.engram.dimension` - Vector dimension used
- `user.engram.accuracy` - Raw accuracy before correction
- `user.engram.correction_overhead` - Correction bytes percentage

**Acceptance Criteria:**
- [ ] `getfattr -d file` shows engram attributes
- [ ] `setfattr -n user.custom -v value file` persists across remount

---

## Phase 5: Codebook & Correction Optimization (Week 5-6)

### 5.1 Semantic Outlier Early Detection

**File:** `embeddenator-vsa/src/codebook.rs`

**Tasks:**
- [ ] Add `detect_outlier_regions(data: &[u8]) -> Vec<Range<usize>>`
- [ ] Use sliding window entropy estimation
- [ ] Mark high-entropy regions with `WordMetadata::SemanticOutlier`
- [ ] Skip VSA encoding for outlier regions, use `Verbatim` correction

**Acceptance Criteria:**
- [ ] JPEG/PNG/encrypted regions detected in <10ms per MB
- [ ] Outlier regions use verbatim storage (no wasted encoding)

### 5.2 Adaptive Correction Selection (Heuristic → Hybrid Foundation)

**File:** `embeddenator-fs/src/correction.rs`

**Tasks:**
- [ ] Refactor correction selection into `CorrectionStrategy` trait
- [ ] Implement `HeuristicStrategy` (current behavior)
- [ ] Add `entropy_threshold` parameter for hybrid preparation
- [ ] Log correction decisions for future ML training data
- [ ] Create `correction_decisions.jsonl` format for training

**Correction Selection Heuristic:**
```
if error_count == 0 → None
elif error_count < 10 && scattered → BitFlips
elif error_count < 50 && clustered → BlockReplace  
elif error_rate > 0.3 → Verbatim
else → TritFlips
```

**Acceptance Criteria:**
- [ ] Correction overhead reduced from ~5% to <3% average
- [ ] Decision logging captures 100% of correction events

### 5.3 Codebook Basis Vector Optimization

**File:** `embeddenator-vsa/src/codebook.rs`

**Tasks:**
- [ ] Implement `Codebook::optimize_basis(corpus: &[&[u8]])`
- [ ] Use SVD/PCA to find optimal differential basis
- [ ] Add `--pretrain` flag to CLI for codebook optimization
- [ ] Store optimized codebook in engram header

**Acceptance Criteria:**
- [ ] Domain-specific codebook improves raw accuracy by 5-10%
- [ ] Codebook serialization adds <1KB overhead

---

## Phase 6: Scale Validation & GPU Optimization (Week 6-8)

### 6.1 TB-Level Ingestion Testing

**File:** `embeddenator-testkit/tests/scale.rs` (new)

**Tasks:**
- [ ] Create 1TB test corpus (Linux kernel source × 100)
- [ ] Measure ingestion throughput (target: >100 MB/s)
- [ ] Measure extraction throughput (target: >80 MB/s)
- [ ] Monitor memory usage (target: <4GB peak)
- [ ] Verify bit-perfect reconstruction for random sample

**Acceptance Criteria:**
- [ ] 1TB ingestion completes in <3 hours
- [ ] Memory stays bounded (streaming works)
- [ ] 100% reconstruction accuracy on sampled files

### 6.2 GPU Kernel Optimization

**File:** `embeddenator-vsa/src/gpu.rs`

**Tasks:**
- [ ] Profile CUDA kernels with `nvprof`
- [ ] Optimize memory coalescing for batch cosine
- [ ] Implement shared memory tiling for large batches
- [ ] Add multi-GPU support for >1M vector searches

**Acceptance Criteria:**
- [ ] Batch cosine 50x faster than CPU for 10K+ vectors
- [ ] GPU memory auto-scaling works without OOM

### 6.3 Concurrent Access Stress Testing

**File:** `embeddenator-fs/tests/concurrent.rs` (new)

**Tasks:**
- [ ] Mount engram filesystem
- [ ] Spawn 100 reader threads, 10 writer threads
- [ ] Run for 1 hour with random read/write/delete
- [ ] Verify no data races, deadlocks, or corruption

**Acceptance Criteria:**
- [ ] No panics or hangs under stress
- [ ] All written data reconstructs correctly
- [ ] Lock contention <5% of wall time

---

## Phase 7: Integration & Documentation (Week 8)

### 7.1 Feature Flag Audit

**Tasks:**
- [ ] Verify all feature combinations compile
- [ ] Document recommended feature sets:
  - `minimal`: No optional features
  - `performance`: `simd` + `block-sparse`
  - `full`: All features enabled
- [ ] Update all README.md files with feature tables

### 7.2 API Documentation

**Tasks:**
- [ ] Add doc comments to all public APIs
- [ ] Create `embeddenator-core/examples/` showcasing:
  - Basic encode/decode
  - Adaptive dimension selection
  - FUSE mount with write
  - Large file handling
- [ ] Generate and publish docs.rs documentation

### 7.3 Version Sync & Release

**Tasks:**
- [ ] Bump all crates to 0.24.0
- [ ] Update `embeddenator-core` dependency table
- [ ] Create GitHub release with changelog
- [ ] Publish to crates.io

---

## Success Criteria Summary

| Metric | Target | Validation |
|--------|--------|------------|
| Write support | Full CRUD via FUSE | Manual + automated tests |
| Raw accuracy (structured) | >95% | Large file test matrix |
| Correction overhead | <3% average | Benchmark suite |
| Ingestion throughput | >100 MB/s | TB-scale test |
| Extraction throughput | >80 MB/s | TB-scale test |
| Max file size | 1GB+ | Large file validation |
| Max path depth | 30+ levels | Deep hierarchy test |
| Concurrent writers | 10+ | Stress test |
| Dimension range | 10K-250K adaptive | Config validation |
| Hierarchy fanout | 128 chunks/level | Manifest inspection |

---

## Dependency Graph

```
Phase 1 (VSA Foundation)
    │
    ├──► Phase 2 (Large-File & Hierarchy)
    │         │
    │         └──► Phase 3 (CLI Update Commands)
    │                   │
    │                   └──► Phase 4 (FUSE Write Hardening)
    │
    └──► Phase 5 (Codebook & Correction) ──► Phase 6 (Scale & GPU)
                                                    │
                                                    └──► Phase 7 (Integration)
```

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| NEON intrinsics slower than auto-vec | Medium | Low | Benchmark before merging, keep fallback |
| 128 fanout breaks existing engrams | Low | High | Version manifest, add migration |
| GPU OOM on large batches | Medium | Medium | Dynamic batch sizing based on free memory |
| Write-ahead log performance overhead | Medium | Medium | Make WAL optional via mount flag |
| Correction hybrid classifier accuracy | Unknown | Medium | Ship heuristic first, classifier behind feature flag |

---

## File Locations Reference

| Component | Primary File | Purpose |
|-----------|--------------|---------|
| SIMD | `embeddenator-vsa/src/simd.rs` | Platform-specific optimizations |
| Block-Sparse | `embeddenator-vsa/src/block_sparse.rs` | Sparse block representation |
| Adaptive Config | `embeddenator-vsa/src/config.rs` | Dimension/density control |
| Hierarchical | `embeddenator-fs/src/hierarchical.rs` | Multi-level sub-engram trees |
| Large File | `embeddenator-fs/src/large_file.rs` | >10MB file handling |
| Correction | `embeddenator-fs/src/correction.rs` | Bit-perfect reconstruction |
| FUSE Write | `embeddenator-fs/src/fuse/versioned_fuse.rs` | Mutable FUSE layer |
| Versioned FS | `embeddenator-fs/src/versioned.rs` | Mutable filesystem core |
| Codebook | `embeddenator-vsa/src/codebook.rs` | Encoding basis vectors |
| GPU | `embeddenator-vsa/src/gpu.rs` | CUDA acceleration |
| CLI Update | `embeddenator-cli/src/commands/update.rs` | Update command stubs |

---

## Kickoff Prompt

```
You are implementing the Embeddenator Holographic FS Maturity Execution Plan.

CONTEXT:
- Embeddenator is a Rust-based holographic filesystem using Vector Symbolic Architecture (VSA)
- Files are encoded into high-dimensional sparse ternary vectors ("engrams")
- Algebraic correction layers ensure 100% bit-perfect reconstruction
- Current state: read-only FUSE works, write support is incomplete

CONFIGURATION DECISIONS (LOCKED):
1. Adaptive dimensionality: 10K floor, 250K ceiling, entropy-based scaling
2. Correction strategy: Heuristic now, evolve to entropy-threshold hybrid
3. Hierarchy fanout: 128 chunks/level (up from 64)

YOUR TASK:
Begin with Phase 1.1 - ARM NEON Explicit Intrinsics in embeddenator-vsa/src/simd.rs

REQUIREMENTS:
1. Read the existing simd.rs implementation to understand current structure
2. Implement explicit NEON intrinsics for trit multiply-accumulate operations
3. Add feature detection to select NEON vs. scalar fallback at runtime
4. Ensure all existing tests pass
5. Add benchmarks comparing NEON vs. auto-vectorized performance

DELIVERABLES:
- Modified simd.rs with explicit NEON intrinsics
- New benchmark in embeddenator-vsa/benches/neon_bench.rs
- Updated Cargo.toml if new dependencies needed

DO NOT:
- Change the public API
- Break x86_64 SIMD implementation
- Remove existing tests

Start by reading embeddenator-vsa/src/simd.rs and embeddenator-vsa/Cargo.toml to understand the current implementation.
```

---

*Plan generated: 2026-01-26 | Review cycle: Weekly | Owner: TBD*
