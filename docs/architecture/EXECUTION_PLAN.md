# Embeddenator Holographic FS Maturity Execution Plan

**Version:** 2.0.0 (Audited)
**Created:** 2026-01-26
**Last Audited:** 2026-01-26
**Target:** Full VSA holographic computational paradigm with mutable latent-space filesystem

---

## ⚠️ AUDIT RESULTS (2026-01-26)

**Many items in this plan were already implemented.** This version reflects actual status.

### Already Implemented (Code Verified)

| Component | Location | Lines | Status |
|-----------|----------|-------|--------|
| Block-sparse | `embeddenator-vsa/src/block_sparse.rs` | 767 | ✅ Complete with tests |
| Signal handlers | `embeddenator-fs/src/fs/signal.rs` | 380 | ✅ SIGINT/SIGTERM/SIGHUP |
| Extended attributes | `embeddenator-fs/src/fs/versioned_fuse.rs` | ~200 | ✅ 4 xattr methods |
| WAL/Journal | `embeddenator-fs/src/fs/versioned/journal.rs` | 972 | ✅ 3 durability modes |
| FUSE write support | `embeddenator-fs/src/fs/versioned_fuse.rs` | 2000+ | ✅ All write ops |
| GPU Blackwell SM 12.0 | `embeddenator-vsa/src/gpu.rs` | 1058 | ✅ Auto-detection |
| AVX2 SIMD | `embeddenator-vsa/src/simd_cosine.rs` | 480 | ✅ x86_64 optimized |
| NEON SIMD | `embeddenator-vsa/src/simd_cosine.rs` | 480 | ✅ aarch64 optimized |
| Correction layer | `embeddenator-fs/src/fs/correction.rs` | 539 | ✅ 5 strategies |
| CLI update commands | `embeddenator-cli/src/commands/update.rs` | ~500 | ✅ add/remove/modify/compact |
| Codebook with outliers | `embeddenator-vsa/src/codebook.rs` | 1016 | ✅ Semantic detection |
| Adaptive dimensionality | `embeddenator-vsa/src/dimensional.rs` | 990 | ✅ DimensionalConfig |
| ReversibleVSAEncoder | `embeddenator-vsa/src/reversible_encoding.rs` | 309 | ✅ 94% accuracy |

### Actual Remaining Gaps

| Gap | GitHub Issue | Priority | Effort |
|-----|--------------|----------|--------|
| Wire ReversibleVSAEncoder into VersionedEmbrFS | #31 | CRITICAL | Large |
| GPU throughput optimization | #32 | HIGH | Medium |
| AVX-512 SIMD | #33 | MEDIUM | Small |
| DeterministicPhaseTrainer integration | #34 | MEDIUM | Medium |

---

## Executive Summary

This plan closes all gaps to achieve a production-ready holographic filesystem with:
- **Full write support** in latent/engram space ✅ DONE
- **Adaptive dimensionality** (10K floor, 250K ceiling) ✅ DONE
- **Improved accuracy** with <2% correction overhead target ⚠️ PARTIAL (need #31)
- **Maximized storage density** and performance ⚠️ PARTIAL (need #32)
- **128 chunks/level hierarchical fanout** for deep paths ✅ DONE

---

## Configuration Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Dimension/Density | Adaptive 10K-250K | Balance accuracy vs. performance based on content entropy |
| Correction Strategy | Heuristic → Hybrid (future) | Start simple, evolve to entropy-threshold hybrid classifier |
| Hierarchy Fanout | 128 chunks/level | Supports ~128^3 = 2M chunks (8GB @ 4KB) per 3-level tree |

---

## Phase 1: VSA Foundation Hardening

### 1.1 ARM NEON Explicit Intrinsics

**Status:** ✅ COMPLETE

Implementation in `embeddenator-vsa/src/simd_cosine.rs`:
- NEON intrinsics via `uint64x2_t` and galloping search
- Runtime feature detection
- Tests pass on aarch64

### 1.2 Block-Sparse Production Hardening

**Status:** ✅ COMPLETE (767 lines)

Implementation in `embeddenator-vsa/src/block_sparse.rs`:
- `BlockSparseTritVec` with full VSA operations
- Invariant validation (sorted blocks, no overlap, no zero blocks)
- Roundtrip tests with SparseVec
- Memory efficiency tracking

### 1.3 Adaptive Dimensionality Controller

**Status:** ✅ COMPLETE (990 lines)

Implementation in `embeddenator-vsa/src/dimensional.rs`:
- `DimensionalConfig` struct
- Entropy-based dimension selection
- `DifferentialEncoder` and `DifferentialEncoding`

---

## Phase 2: Large-File & Hierarchy Fixes

### 2.1 Increase Hierarchical Fanout to 128

**Status:** ✅ COMPLETE

Implementation in `embeddenator-fs/src/fs/versioned/manifest.rs`:
- `HierarchicalManifest` with configurable fanout
- O(log n) lookup performance

### 2.2 Large-File Quality Validation

**Status:** ✅ COMPLETE

Implementation in `embeddenator-fs/src/large_file.rs`:
- Adaptive chunking for large files
- Streaming encode/decode support
- Test coverage exists

### 2.3 Deep Hierarchy Path Validation

**Status:** ⚠️ NEEDS TESTING (infrastructure exists)

The hierarchy infrastructure is complete, but TB-scale validation tests should be added.

---

## Phase 3: CLI Update Commands

### 3.1 & 3.2 CLI Update Operations

**Status:** ✅ COMPLETE

Implementation in `embeddenator-cli/src/commands/update.rs`:
- `add-file` command functional
- `remove-file` command functional
- `modify-file` command functional
- `compact` command functional

All commands are wired to `VersionedEmbrFS` operations.

---

## Phase 4: FUSE Write Hardening

### 4.1 Error Recovery & Retry Logic

**Status:** ✅ COMPLETE

Implementation in `embeddenator-fs/src/fs/versioned/journal.rs`:
- 972 lines of WAL implementation
- 3 durability modes: Immediate, GroupCommit, Relaxed
- CRC32 checksum verification on replay
- Transaction ID tracking

### 4.2 Signal Handling & Graceful Unmount

**Status:** ✅ COMPLETE

Implementation in `embeddenator-fs/src/fs/signal.rs`:
- 380 lines of signal handling
- SIGINT, SIGTERM, SIGHUP handlers
- `ShutdownSignal` thread-safe flag
- `mount_versioned_fs_with_signals()` for graceful unmount

### 4.3 Extended Attributes

**Status:** ✅ COMPLETE

Implementation in `embeddenator-fs/src/fs/versioned_fuse.rs` (lines 1094-1280):
- `getxattr()` - read engram metadata
- `setxattr()` - set custom attributes
- `listxattr()` - list all attributes
- `removexattr()` - remove attributes

Built-in attributes: `user.engram.version`, `user.engram.file_count`, etc.

---

## Phase 5: Codebook & Correction Optimization

### 5.1 Semantic Outlier Early Detection

**Status:** ✅ COMPLETE

Implementation in `embeddenator-vsa/src/codebook.rs`:
- `SemanticOutlier` detection
- `WordMetadata` with outlier marking
- Entropy-based region detection

### 5.2 Adaptive Correction Selection

**Status:** ✅ COMPLETE

Implementation in `embeddenator-fs/src/fs/correction.rs` (539 lines):
- 5 correction strategies: None, BitFlips, TritFlips, BlockReplace, Verbatim
- Intelligent selection based on error patterns
- CorrectionStore with statistics tracking

### 5.3 Codebook Basis Vector Optimization

**Status:** ⚠️ PARTIAL - DeterministicPhaseTrainer integration pending (Issue #34)

Current state:
- Static basis vectors in `Codebook`
- No learned optimization yet
- vsa-optim-rs has `DeterministicPhaseTrainer` ready for integration

---

## Phase 6: Scale Validation & GPU Optimization

### 6.1 TB-Level Ingestion Testing

**Status:** 🔴 TODO

Need to create scale validation tests in `embeddenator-testkit`.

### 6.2 GPU Kernel Optimization

**Status:** ⚠️ PARTIAL (Issue #32)

Current state:
- GPU kernels exist (Turing through Blackwell SM 12.0)
- Batch cosine works with auto memory management
- Throughput is far below target (0.06 MB/s vs 100 MB/s target)

**Root cause:** Encode/decode loop is sequential, not batched.

### 6.3 Concurrent Access Stress Testing

**Status:** ⚠️ PARTIAL

Optimistic locking exists, but no formal stress test suite.

---

## 🔴 CRITICAL GAP: Wire ReversibleVSAEncoder into VersionedEmbrFS (Issue #31)

**This is the PRIMARY blocker for true holographic storage.**

### Current State

`VersionedEmbrFS.write_file_holographic()` uses `Codebook.project()`:
- `Codebook.project()` achieves ~0% accuracy (fundamentally broken)
- Result: 100% correction overhead (data stored verbatim)
- No actual holographic storage happening

### Target State

Replace with `ReversibleVSAEncoder.encode_chunked()`:
- Achieves 94% accuracy with position-aware binding
- Reduces correction overhead from 100% to <6%
- Enables true holographic superposition storage

### Implementation Plan

**Files to modify:**
1. `embeddenator-fs/src/fs/versioned_embrfs.rs`
   - Add `ReversibleVSAEncoder` instance
   - Replace `write_file_holographic()` internals
   - Update `read_file_holographic()` internals

2. `embeddenator-fs/src/fs/versioned/chunk_store.rs`
   - Adapt chunk storage for reversible encoding format

3. `embeddenator-fs/Cargo.toml`
   - Ensure `embeddenator-vsa` dep has required features

**Migration path:**
- Add manifest version field
- Old engrams: continue using verbatim
- New engrams: use ReversibleVSAEncoder

### Validation

```bash
# After implementation:
embeddenator ingest ./testdir -e test.engram
# Should show: "Raw accuracy: 94%, Correction overhead: 5%"

embeddenator mount -e test.engram /tmp/mnt
cat /tmp/mnt/testfile.txt
# Should reconstruct correctly
```

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

## Optimal Development Order

### Parallel Track A: Foundation (Weeks 1-2)
```
#31: Wire ReversibleVSAEncoder into VersionedEmbrFS
     - Replace Codebook.project() with ReversibleVSAEncoder.encode_chunked()
     - Update read/write paths
     - Add migration for existing engrams
```

### Parallel Track B: CPU Optimization (Weeks 1-2)
```
#33: AVX-512 SIMD (INDEPENDENT - can start immediately)
     - Add AVX-512 path to simd_cosine.rs
     - Benchmark on 14700K
```

### Sequential Track C: After #31 (Weeks 2-4)
```
#32: GPU Throughput Optimization
     - Batch encode/decode operations
     - Profile and optimize CUDA kernels

#34: DeterministicPhaseTrainer Integration
     - Train codebook from representative data
     - Validate accuracy improvement
```

---

## Non-Blocking Work Summary

| Work Item | Can Start | Blocks | Parallelizable With |
|-----------|-----------|--------|---------------------|
| **#31 FS Integration** | Immediately | #32, #34 validation | #33 |
| **#33 AVX-512** | Immediately | Nothing | #31 |
| **#32 GPU Throughput** | After #31 basics | Nothing | #34 |
| **#34 Trainer** | After #31 basics | Nothing | #32 |

---

## Kickoff Prompt (UPDATED for #31)

```
You are implementing Issue #31: Wire ReversibleVSAEncoder into VersionedEmbrFS.

CONTEXT:
- Embeddenator is a Rust-based holographic filesystem using Vector Symbolic Architecture (VSA)
- ReversibleVSAEncoder exists in embeddenator-vsa and achieves 94% accuracy
- VersionedEmbrFS.write_file_holographic() currently uses Codebook.project() which has 0% accuracy
- This is causing 100% correction overhead (data stored verbatim)

YOUR TASK:
Replace Codebook.project() with ReversibleVSAEncoder.encode_chunked()

FILES TO MODIFY:
1. embeddenator-fs/src/fs/versioned_embrfs.rs
   - Add ReversibleVSAEncoder instance
   - Replace write_file_holographic() to use chunked encoding
   - Update read_file_holographic() to use chunked decoding

2. embeddenator-fs/Cargo.toml
   - Ensure embeddenator-vsa dependency is correct

REQUIREMENTS:
1. Achieve >90% uncorrected accuracy
2. Correction overhead should drop from 100% to <10%
3. Add manifest versioning for migration
4. Existing engrams should continue to work (backwards compatibility)

VALIDATION:
embeddenator ingest ./testdir -e test.engram
# Should show accuracy > 90% and correction < 10%

DO NOT:
- Remove existing write_file() or write_file_compressed() methods
- Break FUSE mount functionality
- Remove tests
```

---

*Plan audited: 2026-01-26 | Last update: 2026-01-26 | Owner: TBD*
