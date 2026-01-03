# Phase 1: Bitsliced Integration Plan

**Date**: January 3, 2026  
**Status**: Planning  
**Branch**: `feature/phase1-bitsliced-integration` (to be created off `dev`)  
**Target**: v1.1.0

---

## Executive Summary

Phase 1 integrates the validated bitsliced ternary representation from `exploration/bitsliced-ternary` into the production codebase as the default for dense operations (≥0.5% density), while preserving sparse representation for very low density workloads. This phase also adds AVX-512 runtime detection and optimizes permutation operations for bitsliced vectors.

### Validated Performance Gains

| Operation | Packed (baseline) | Bitsliced | Speedup |
|-----------|-------------------|-----------|---------|
| Bind @ 10K | 266 ns | 116 ns | **2.30×** |
| Bundle @ 10K | 273 ns | 112 ns | **2.44×** |
| Dot @ 10K | 494 ns | 383 ns | **1.29×** |
| Bind @ 100K | 2.66 µs | 1.04 µs | **2.55×** |
| Bundle @ 100K | 2.73 µs | 1.02 µs | **2.69×** |

---

## Architecture Overview

### Current State (v1.0.0)

```
SparseVec (default) ←→ PackedTritVec (bt-phase-2 gated)
                            ↓
                   BitslicedTritVec (exploration branch)
```

**Issues**:
1. `PackedTritVec` used for dense operations (bt-phase-2 gate)
2. `BitslicedTritVec` not integrated into main code paths
3. No automatic representation selection
4. Permute operation is O(D) element-by-element for bitsliced

### Target State (Phase 1)

```
                      ┌─────────────────────────────────────┐
                      │         HybridTritVec               │
                      │   (Unified representation layer)    │
                      └──────────────┬──────────────────────┘
                                     │
              ┌──────────────────────┼──────────────────────┐
              │                      │                      │
              ▼                      ▼                      ▼
        SparseVec            BitslicedTritVec         PackedTritVec
    (density < 0.5%)       (density ≥ 0.5%)         (deprecated path)
                                     │
                           ┌─────────┴─────────┐
                           │                   │
                      Scalar Path         AVX-512 Path
                      (fallback)         (runtime detect)
```

### Key Design Decisions

1. **Density Threshold**: 0.5% (~50 nnz at D=10K) as cutoff
   - Below: Keep sparse (O(nnz) operations)
   - Above: Convert to bitsliced (O(D/64) with SIMD potential)

2. **Runtime AVX-512 Detection**: Use `std::arch::is_x86_feature_detected!`
   - Hot path operations dispatch to AVX-512 when available
   - Scalar fallback always available

3. **Permute Optimization**: Word-level bit rotation for large shifts
   - Full word rotation: O(D/64) instead of O(D)
   - Intra-word shifts use barrel shifter operations

---

## Implementation Tasks

### Task 1: Merge Exploration Branch

**Files to merge from `exploration/bitsliced-ternary`**:
- `src/vsa/bitsliced.rs` (974 lines) - Already exists, check for updates
- `tests/bitsliced_equivalence.rs` (275 lines)
- `benches/vsa_ops.rs` additions (142 lines diff)
- Documentation (keep exploration docs as reference)

**Actions**:
```bash
git checkout dev
git checkout -b feature/phase1-bitsliced-integration
git merge exploration/bitsliced-ternary --no-commit
# Resolve any conflicts, verify tests pass
```

### Task 2: Create HybridTritVec Wrapper

**New file**: `src/vsa/hybrid.rs`

```rust
//! Hybrid Ternary Vector - Automatic representation selection
//!
//! Transparently selects between sparse and bitsliced representations
//! based on density, providing optimal performance across all scales.

use crate::vsa::{BitslicedTritVec, SparseVec, DIM};

/// Density threshold for switching to bitsliced representation.
/// Below this, sparse operations are more efficient.
pub const DENSITY_THRESHOLD: f64 = 0.005; // 0.5%

/// Hybrid ternary vector that automatically selects representation.
#[derive(Clone, Debug)]
pub enum HybridTritVec {
    /// Sparse representation for low-density vectors
    Sparse(SparseVec),
    /// Bitsliced representation for dense vectors  
    Bitsliced(BitslicedTritVec),
}

impl HybridTritVec {
    /// Create from sparse, auto-selecting representation.
    pub fn from_sparse(sparse: SparseVec, dim: usize) -> Self {
        let density = sparse.nnz() as f64 / dim as f64;
        if density < DENSITY_THRESHOLD {
            HybridTritVec::Sparse(sparse)
        } else {
            HybridTritVec::Bitsliced(BitslicedTritVec::from_sparse(&sparse, dim))
        }
    }
    
    /// Force conversion to bitsliced (for batch operations).
    pub fn to_bitsliced(&self, dim: usize) -> BitslicedTritVec {
        match self {
            HybridTritVec::Sparse(s) => BitslicedTritVec::from_sparse(s, dim),
            HybridTritVec::Bitsliced(b) => b.clone(),
        }
    }
    
    /// Force conversion to sparse.
    pub fn to_sparse(&self) -> SparseVec {
        match self {
            HybridTritVec::Sparse(s) => s.clone(),
            HybridTritVec::Bitsliced(b) => b.to_sparse(),
        }
    }
    
    /// Get current density estimate.
    pub fn density(&self, dim: usize) -> f64 {
        match self {
            HybridTritVec::Sparse(s) => s.nnz() as f64 / dim as f64,
            HybridTritVec::Bitsliced(b) => b.nnz() as f64 / b.len() as f64,
        }
    }
    
    // ... VSA operations with automatic dispatch
}
```

### Task 3: Runtime AVX-512 Detection

**Modify**: `src/vsa/bitsliced.rs`

```rust
use std::sync::atomic::{AtomicU8, Ordering};

/// CPU feature detection cache.
/// 0 = not checked, 1 = not available, 2 = available
static AVX512_AVAILABLE: AtomicU8 = AtomicU8::new(0);

/// Check if AVX-512 is available (cached).
#[inline]
pub fn has_avx512() -> bool {
    match AVX512_AVAILABLE.load(Ordering::Relaxed) {
        0 => {
            let available = is_x86_feature_detected!("avx512f");
            AVX512_AVAILABLE.store(if available { 2 } else { 1 }, Ordering::Relaxed);
            available
        }
        2 => true,
        _ => false,
    }
}

impl BitslicedTritVec {
    /// Bind with automatic SIMD dispatch.
    pub fn bind_dispatch(&self, other: &Self) -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            if has_avx512() && self.len >= 512 {
                let mut out = Self::new_zero(self.len.min(other.len));
                // Safety: We just checked for AVX-512 support
                unsafe { avx512::bind_avx512(self, other, &mut out) };
                return out;
            }
        }
        self.bind(other) // Scalar fallback
    }
    
    // Similar for bundle_dispatch, dot_dispatch...
}
```

### Task 4: Optimized Permute (Bit Rotation)

**Add to**: `src/vsa/bitsliced.rs`

```rust
impl BitslicedTritVec {
    /// Optimized permute using word-level rotation.
    ///
    /// For shift amounts that are multiples of 64, this is O(words) memory moves.
    /// For arbitrary shifts, we use a combination of word rotation and intra-word shifts.
    pub fn permute_optimized(&self, shift: usize) -> Self {
        if self.len == 0 || shift == 0 {
            return self.clone();
        }
        
        let shift = shift % self.len;
        let words = Self::word_count(self.len);
        
        // Decompose shift: word_shift * 64 + bit_shift
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        
        let mut out = Self::new_zero(self.len);
        
        if bit_shift == 0 {
            // Pure word rotation - very fast
            for w in 0..words {
                let src_w = (w + words - word_shift) % words;
                out.pos[w] = self.pos[src_w];
                out.neg[w] = self.neg[src_w];
            }
        } else {
            // Mixed rotation: combine adjacent words
            let complement = 64 - bit_shift;
            
            for w in 0..words {
                // Need bits from two source words
                let src_w_hi = (w + words - word_shift) % words;
                let src_w_lo = (w + words - word_shift - 1) % words;
                
                // Combine shifted portions
                out.pos[w] = (self.pos[src_w_hi] << bit_shift) 
                           | (self.pos[src_w_lo] >> complement);
                out.neg[w] = (self.neg[src_w_hi] << bit_shift) 
                           | (self.neg[src_w_lo] >> complement);
            }
        }
        
        // Mask last word
        if !out.pos.is_empty() {
            let last = out.pos.len() - 1;
            let mask = Self::last_word_mask(self.len);
            out.pos[last] &= mask;
            out.neg[last] &= mask;
        }
        
        out
    }
}
```

**Complexity Analysis**:
- Current `permute()`: O(D) with D get/set operations
- `permute_optimized()`: O(D/64) with 2 memory ops per word
- **Speedup**: ~32-64× for large D

### Task 5: Extended Benchmarks (100M Dimensions)

**Add to**: `benches/vsa_ops.rs`

```rust
fn bench_extreme_scale(c: &mut Criterion) {
    // Only run on systems with sufficient RAM (>8GB free)
    let large_dims = [1_000_000, 10_000_000, 100_000_000];
    
    for dim in large_dims {
        // Check if we have enough memory (~25MB per vector at 100M)
        let estimated_mb = (dim as f64 * 0.25) / 1_000_000.0;
        if estimated_mb > 4000.0 {
            println!("Skipping D={} (requires {}MB)", dim, estimated_mb);
            continue;
        }
        
        let mut group = c.benchmark_group(format!("extreme_scale_dim_{}", dim));
        group.sample_size(10); // Fewer samples for very large ops
        group.measurement_time(std::time::Duration::from_secs(20));
        
        // Create sparse test vectors (~0.1% density)
        let nnz = (dim as f64 * 0.001) as usize;
        let sparse_a = make_sparse_deterministic(nnz, dim, 42);
        let sparse_b = make_sparse_deterministic(nnz, dim, 123);
        
        let bitsliced_a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let bitsliced_b = BitslicedTritVec::from_sparse(&sparse_b, dim);
        
        group.bench_function("bitsliced_bind", |bencher| {
            bencher.iter(|| {
                let result = black_box(&bitsliced_a).bind(black_box(&bitsliced_b));
                black_box(result)
            })
        });
        
        group.bench_function("bitsliced_bundle", |bencher| {
            bencher.iter(|| {
                let result = black_box(&bitsliced_a).bundle(black_box(&bitsliced_b));
                black_box(result)
            })
        });
        
        group.bench_function("bitsliced_permute", |bencher| {
            bencher.iter(|| {
                let result = black_box(&bitsliced_a).permute_optimized(1234567);
                black_box(result)
            })
        });
        
        group.bench_function("bitsliced_dot", |bencher| {
            bencher.iter(|| {
                let result = black_box(&bitsliced_a).dot(black_box(&bitsliced_b));
                black_box(result)
            })
        });
        
        group.finish();
    }
}

fn make_sparse_deterministic(nnz: usize, dim: usize, seed: u64) -> SparseVec {
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    
    let mut pos = Vec::with_capacity(nnz);
    let mut neg = Vec::with_capacity(nnz);
    
    for i in 0..nnz {
        if i % 2 == 0 {
            pos.push(rng.gen_range(0..dim));
        } else {
            neg.push(rng.gen_range(0..dim));
        }
    }
    
    pos.sort_unstable();
    pos.dedup();
    neg.sort_unstable();
    neg.dedup();
    
    SparseVec { pos, neg }
}
```

### Task 6: Feature Flag Integration

**Modify**: `Cargo.toml`

```toml
[features]
# ... existing features ...

# Phase 1: Bitsliced as default for dense operations
bitsliced-default = []

# AVX-512 optimizations (auto-detected at runtime, this enables the code paths)
avx512 = []

# All Phase 1 features
phase1-bitsliced = ["bitsliced-default", "avx512"]
```

### Task 7: ADR Update

**New file**: `docs/adr/ADR-009-bitsliced-default.md`

```markdown
# ADR-009: Bitsliced Ternary as Default Dense Representation

## Status
Accepted

## Date
2026-01-03

## Context
The exploration/bitsliced-ternary branch validated 2.3-2.7× speedup for 
bind/bundle operations at production dimensions (10K-100K). This ADR 
formalizes the integration strategy.

## Decision
1. BitslicedTritVec becomes the default for dense operations (≥0.5% density)
2. SparseVec remains optimal for very sparse vectors (<0.5% density)
3. PackedTritVec is deprecated but retained for backward compatibility
4. Runtime AVX-512 detection enables additional acceleration
5. HybridTritVec provides automatic representation selection

## Consequences
### Positive
- 2-2.7× faster bind/bundle at typical workloads
- Near-optimal memory bandwidth utilization
- SIMD-ready architecture for future GPU/AVX-512 optimization
- Clean abstraction via HybridTritVec

### Negative
- Slightly increased complexity in representation layer
- 2× memory for bitsliced vs packed (acceptable trade-off)
- Permute optimization adds code complexity

## Implementation
See Phase 1 integration plan in docs/roadmap/phase1-bitsliced-integration-plan.md
```

---

## Test Plan

### Unit Tests (New)

1. **Hybrid selection tests**: Verify threshold behavior
2. **AVX-512 equivalence**: Results match scalar path
3. **Permute optimization**: Verify against naive implementation
4. **Extreme scale**: Correctness at 1M, 10M dimensions

### Integration Tests

1. **Full encode/decode roundtrip** with bitsliced backend
2. **Hierarchical bundling** with HybridTritVec
3. **Cosine retrieval** accuracy unchanged

### Regression Tests

1. **All existing tests must pass** (zero regressions)
2. **Benchmark comparison**: No performance regression on any path

---

## Acceptance Criteria

- [ ] All existing tests pass
- [ ] Bitsliced bind/bundle ≥2× faster than packed at D=10K
- [ ] Permute optimization ≥10× faster than naive at D=10K
- [ ] AVX-512 path produces identical results to scalar
- [ ] 100M dimension benchmarks complete successfully
- [ ] Zero Clippy warnings
- [ ] ADR-009 approved

---

## Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| AVX-512 edge cases | Low | Medium | Extensive equivalence tests |
| Permute bugs at boundaries | Medium | High | Property-based testing |
| Memory pressure at 100M | Medium | Low | Graceful degradation |
| Regression in sparse path | Low | High | Preserve existing code paths |

---

## Timeline Estimate

| Task | Effort | Dependencies |
|------|--------|--------------|
| Merge exploration branch | 1h | None |
| HybridTritVec implementation | 4h | Merge |
| AVX-512 runtime detection | 2h | Merge |
| Permute optimization | 3h | Merge |
| Extended benchmarks | 2h | All above |
| Tests and documentation | 3h | All above |
| Review and polish | 2h | All above |
| **Total** | **~17h** | |

---

## Next Steps

1. Create `feature/phase1-bitsliced-integration` branch
2. Implement tasks in order
3. Run full test suite after each task
4. Benchmark comparison report
5. Code review
6. Merge to dev
