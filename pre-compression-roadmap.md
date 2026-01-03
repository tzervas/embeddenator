# Pre-Compression/Encryption Roadmap

**Based on:** Codebase Analysis vs. [refactor.md](refactor.md) Proposals  
**Date:** January 2, 2026  
**Goal:** Bridge current implementation to proposed improvements before compression/encryption layers

---

## Gap Analysis Summary

| Aspect | Current State | Proposed (refactor.md) | Gap Severity |
|--------|---------------|------------------------|--------------|
| **Dimensions** | Fixed 10,000 | Dynamic 100K-10M | 🔴 Critical |
| **Trit Storage** | `usize` index lists | 27-trit in u64 registers | 🔴 Critical |
| **Sparsity Rep** | `Vec<usize>` pos/neg | Sparse index encoding | 🟡 Medium |
| **Dense Path** | 2-bit PackedTritVec | 54-bit payload + metadata | 🟡 Medium |
| **Quantization** | None | Lloyd-Max + outlier sidecar | 🟡 Medium |
| **Memory Tiers** | Heap-allocated | Register/L1/L2/RAM tiering | 🟠 High |
| **Bind/Unbind** | Approximate (merge-join) | Exact via NTT | 🟠 High |
| **Search** | Full codebook scan | Merkle-DAG selective unfold | 🟡 Medium |
| **SIMD** | Stub implementations | AVX-512 ternary ops | 🟢 Low (for now) |
| **GPU** | None | CUDA/ROCm NTT kernels | 🟢 Low (deferred) |

---

## Recommended Implementation Phases

### Phase 1: Core Ternary Integer Foundation (Weeks 1-2)

**Objective:** Implement the 27-trit integer type that becomes the atomic unit for all operations.

#### 1.1 Create `Trit27` Type

**File:** `src/vsa/trit27.rs` (new)

```rust
/// 27-trit balanced ternary integer in 64-bit register
/// Range: ±3,812,798,742,493 (±3.8T)
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Trit27(i64);

impl Trit27 {
    pub const ZERO: Self = Trit27(0);
    pub const MAX: Self = Trit27(3_812_798_742_493);  // (3^27 - 1) / 2
    pub const MIN: Self = Trit27(-3_812_798_742_493);
    
    /// Saturating ternary addition
    pub fn add(self, other: Self) -> Self { ... }
    
    /// Modular ternary multiplication
    pub fn mul(self, other: Self) -> Self { ... }
    
    /// Extract individual trit at position 0-26
    pub fn trit_at(&self, pos: u8) -> Trit { ... }
    
    /// Set trit at position 0-26
    pub fn set_trit(&mut self, pos: u8, trit: Trit) { ... }
}
```

**Tasks:**
1. [ ] Define `Trit27` with `#[repr(transparent)]` for zero-cost i64 conversion
2. [ ] Implement balanced ternary encode/decode (similar to existing `BalancedTernaryWord`)
3. [ ] Add saturating arithmetic that respects 27-trit range
4. [ ] Implement trit extraction/injection for sparse↔dense conversion
5. [ ] Add comprehensive tests (exhaustive for small values, proptest for large)

**Integration Point:** Replace `BalancedTernaryWord`'s 38-trit encoding with 27-trit, freeing bits for metadata.

#### 1.2 Sparse Trit27 Register

**File:** `src/vsa/sparse_trit27.rs` (new)

```rust
/// Sparse encoding of ≤8 non-zero trits
pub struct SparseTrit27Reg {
    indices: [u8; 8],    // Positions 0-26
    values: [Trit; 8],   // Corresponding trit values
    count: u8,           // Number of non-zero (0-8)
}
```

**Tasks:**
1. [ ] Implement `SparseTrit27Reg` for ultra-sparse vectors (≤10% density)
2. [ ] Conversion functions: `Trit27 ↔ SparseTrit27Reg`
3. [ ] Sparse arithmetic operations that don't expand to dense

#### 1.3 Benchmark Comparison

**Tasks:**
1. [ ] Benchmark `Trit27` operations vs current `SparseVec`
2. [ ] Measure memory footprint: 8 bytes vs ~1.6KB per vector
3. [ ] Document crossover points for sparse vs dense representation

---

### Phase 2: Dynamic Dimensionality (Weeks 3-4)

**Objective:** Scale dimension count based on data entropy, not fixed 10,000.

#### 2.1 Entropy-Based Dimension Calculator

**File:** `src/vsa/dimensional_config.rs` (extend existing `dimensional.rs`)

```rust
pub struct DynamicDimensionConfig {
    pub min_dim: usize,      // 100K default
    pub max_dim: usize,      // 10M default
    pub quantum: usize,      // 1024 (NTT-friendly)
}

impl DynamicDimensionConfig {
    pub fn recommend_dimension(&self, chunk: &[u8]) -> usize {
        let entropy = shannon_entropy(chunk);
        // Low entropy → fewer dims, high entropy → more dims
        ...
    }
}
```

**Tasks:**
1. [ ] Implement Shannon entropy calculator (already pseudocode in refactor.md)
2. [ ] Map entropy to dimension scale factor
3. [ ] Round to NTT-friendly quantum (power of 2 × small prime)
4. [ ] Add config option to CLI: `--dimension-mode {fixed|dynamic}`

#### 2.2 Variable-Dimension SparseVec

**Current Constraint:** `pub const DIM: usize = 10_000` is hardcoded.

**Tasks:**
1. [ ] Make `DIM` a runtime parameter in `SparseVec` (or carry dimension in struct)
2. [ ] Update all operations to use `self.dim` instead of global constant
3. [ ] Validate dimension compatibility in binary operations
4. [ ] Update serialization to include dimension metadata

#### 2.3 Hierarchical Dimension Mapping

For hierarchical engrams, different levels may use different dimensions:

**Tasks:**
1. [ ] Add `dimension` field to `SubEngram`
2. [ ] Handle cross-dimension similarity (project to common dimension)
3. [ ] Update inverted index to support variable-dimension vectors

---

### Phase 3: Memory Tier Awareness (Week 5)

**Objective:** Keep hot data in registers/L1, spill to L2/L3/RAM as needed.

#### 3.1 Storage Tier Enum

**File:** `src/vsa/storage_tier.rs` (new)

```rust
pub enum EngramStorage {
    /// ≤512 non-zero trits → fits in AVX-512 registers
    RegisterResident {
        vectors: [__m512i; 16],  // 1KB total
        count: u8,
    },
    /// ≤4K non-zero trits → L1 cache (32KB)
    L1Resident {
        data: Box<[u64; 4096]>,
        sparsity: f32,
    },
    /// ≤128K non-zero trits → L2 cache (1MB)
    L2Resident {
        data: Box<[u64]>,
        dimension: usize,
    },
    /// Larger → heap/RAM
    RamResident {
        data: Vec<u64>,
        dimension: usize,
    },
}
```

**Tasks:**
1. [ ] Define tier thresholds based on CPU cache sizes
2. [ ] Implement automatic tier promotion/demotion
3. [ ] Add prefetch hints for L2→L1 promotion during hot loops
4. [ ] Benchmark tier-aware operations vs heap-only baseline

#### 3.2 Operation Dispatch by Tier

**Tasks:**
1. [ ] Implement `bundle()` variants for each tier
2. [ ] Fastest path: register-only operations with zero memory access
3. [ ] Fallback: streaming operations for RAM-resident data

---

### Phase 4: Quantization Layer (Week 6)

**Objective:** Handle dense data (images, audio) via quantization + outlier sidecar.

#### 4.1 Lloyd-Max Quantizer

**File:** `src/core/quantizer.rs` (new)

```rust
pub struct Quantizer {
    pub bins: [Trit27; 8],  // 8 cluster centers
    pub outlier_threshold: f64,  // 3σ typical
}

pub struct QuantizedEngram {
    pub quantized: SparseVec,          // Main payload
    pub outliers: Vec<(usize, Trit27)>, // Exact values for outliers
    pub quantization_table: [Trit27; 8],
}
```

**Tasks:**
1. [ ] Implement Lloyd-Max clustering (iterative k-means for scalar data)
2. [ ] Define outlier detection (distance from nearest bin > threshold)
3. [ ] Encode: assign each value to bin or mark as outlier
4. [ ] Decode: reconstruct from bin centers + outlier overrides
5. [ ] Benchmark compression ratio on various data types

#### 4.2 Adaptive Quantization

**Tasks:**
1. [ ] Auto-detect data type (text/code/image/audio) from entropy profile
2. [ ] Skip quantization for naturally sparse data (entropy < 2)
3. [ ] Apply quantization for dense data (entropy > 6)
4. [ ] Store quantization metadata in engram header

---

### Phase 5: NTT for Exact Bind/Unbind (Weeks 7-8)

**Objective:** Make bind operation algebraically invertible via Number Theoretic Transform.

**Current Problem:** `bind()` uses element-wise multiplication on sparse support intersection. This is approximate—elements outside the intersection are lost.

**Proposed Solution:** NTT-based polynomial multiplication enables exact unbind via modular inverse.

#### 5.1 NTT Primitives

**File:** `src/vsa/ntt.rs` (new)

```rust
/// Number Theoretic Transform for exact polynomial ops
pub struct NTT {
    pub modulus: u64,    // Prime P where DIM | (P-1)
    pub root: u64,       // Primitive DIM-th root of unity mod P
    pub root_inv: u64,   // Modular inverse of root
}

impl NTT {
    pub fn forward(&self, coeffs: &mut [u64]) { ... }
    pub fn inverse(&self, coeffs: &mut [u64]) { ... }
    pub fn multiply(&self, a: &[u64], b: &[u64]) -> Vec<u64> { ... }
}
```

**Tasks:**
1. [ ] Find suitable primes for each dimension quantum
2. [ ] Implement Cooley-Tukey FFT in Z/pZ
3. [ ] Implement modular inverse for unbind
4. [ ] Validate: `(A ⊙ B) ⊙ B⁻¹ = A` exactly

#### 5.2 Integration with SparseVec

**Tasks:**
1. [ ] Add `bind_exact()` using NTT
2. [ ] Add `unbind_exact()` using NTT + modular inverse
3. [ ] Benchmark NTT vs current merge-join bind
4. [ ] Feature-gate behind `ntt` flag (NTT is expensive for sparse vectors)

---

### Phase 6: Selective Unfold / Merkle-DAG (Weeks 9-10)

**Objective:** Query without decoding entire engram; prune non-matching subtrees.

#### 6.1 Engram Tree Structure

**Enhance existing `HierarchicalManifest`:**

```rust
pub struct EngramTree {
    pub root: SparseVec,
    pub children: Vec<EngramNode>,
    pub codebook: HashMap<Blake3Hash, SparseVec>,
}

pub enum EngramNode {
    Leaf { hash: [u8; 32], engram: SparseVec },
    Branch { hash: [u8; 32], children: Vec<[u8; 32]>, engram: SparseVec },
}
```

**Tasks:**
1. [ ] Refactor `SubEngram` to `EngramNode` with explicit tree structure
2. [ ] Compute content hashes for integrity verification
3. [ ] Implement lazy child loading via `SubEngramStore`

#### 6.2 Selective Search

```rust
impl EngramTree {
    pub fn selective_search(&self, query: &SparseVec, threshold: f64) -> Vec<Hash> {
        // DFS with pruning: only expand nodes with similarity > threshold
    }
    
    pub fn selective_unfold(&self, hashes: &[Hash]) -> Vec<Vec<u8>> {
        // Decode only matched chunks
    }
}
```

**Tasks:**
1. [ ] Implement threshold-based pruning (0.8 default)
2. [ ] Add beam width limit (current `HierarchicalQueryBounds`)
3. [ ] Benchmark selective vs full scan on large engrams
4. [ ] Document expected speedups (10-1000x for sparse queries)

---

## Integration Checkpoints

### Checkpoint 1: After Phase 2 (Week 4)

**Validation:**
- [ ] `cargo test --all-features` passes
- [ ] Benchmarks show no regression for existing code paths
- [ ] Dynamic dimensions work for at least 2 dimension scales

**Deliverable:** PR merging `Trit27` and dynamic dimensions behind feature flag.

### Checkpoint 2: After Phase 4 (Week 6)

**Validation:**
- [ ] Quantized engrams reconstruct with ≤1% outlier overhead for images
- [ ] Text/code data skips quantization automatically
- [ ] Correction layer handles residuals correctly

**Deliverable:** PR adding quantization layer, enabled by default for dense data.

### Checkpoint 3: After Phase 6 (Week 10)

**Validation:**
- [ ] NTT bind/unbind is algebraically exact
- [ ] Selective unfold achieves >10x speedup on 10K+ chunk engrams
- [ ] Memory usage stays within tier bounds

**Deliverable:** Full pre-compression improvement suite ready for v0.20.0.

---

## Dependencies & Prerequisites

### Before Starting

1. **Stabilize bt-phase-2:** Ensure `PackedTritVec` integration is solid
2. **Benchmark Baseline:** Record current performance metrics
3. **Test Coverage:** Ensure >80% coverage on core VSA ops

### External Dependencies

| Dependency | Purpose | When Needed |
|------------|---------|-------------|
| `criterion` | Benchmarking | Throughout |
| `proptest` | Property-based testing | Throughout |
| `num-bigint` | Large integer NTT | Phase 5 |
| `blake3` | Content hashing | Phase 6 |

### Hardware Requirements

- **Development:** Any x86_64/aarch64 with 8GB+ RAM
- **Benchmarking:** Machine with identifiable L1/L2/L3 sizes
- **NTT Testing:** Large RAM (16GB+) for high-dimension transforms

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `Trit27` slower than sparse | Medium | High | Keep both paths, switch dynamically |
| NTT too slow for sparse | High | Medium | Feature-gate, use only when needed |
| Dynamic dims break compatibility | Medium | High | Version engram format, migrate |
| Quantization loses precision | Low | Medium | Correction layer guarantees recovery |
| Cache tiers non-portable | Medium | Low | Fallback to RAM-only on detection failure |

---

## Post-Roadmap: Compression & Encryption

After completing these phases, the codebase will be ready for:

### Compression Layer

- **Zstd on engrams:** Already supported via `compression-zstd` feature
- **Internal sparsity:** Trit27 + quantization inherently compress
- **Delta encoding:** Store differences from codebook basis vectors

### Encryption Layer

**Order:** Compress first (reduces entropy), then encrypt.

**Approach:**
1. Generate per-engram symmetric key
2. Encrypt codebook separately (acts as private key)
3. Engram without codebook is information-theoretically secure

---

## Summary

This roadmap bridges the current functional implementation to the performance characteristics outlined in refactor.md. Key priorities:

1. **Trit27 type** — Foundational for everything else
2. **Dynamic dimensions** — Critical for efficiency across data types
3. **Memory tiers** — Unlock register-resident performance
4. **Quantization** — Handle dense data gracefully
5. **NTT** — Enable exact algebraic operations
6. **Selective unfold** — Scale to large engrams

Total estimated effort: **10 weeks** for one engineer, parallelizable to **5-6 weeks** with two.

---

*Pre-Compression/Encryption Roadmap v1.0*
