# Holographic Storage Refactor Plan

## Current Status

### What Works (Proof of Concept)
- **ReversibleVSAEncoder**: Achieves 94.12% accuracy with chunked encoding
- Uses bind(position_vector, byte_vector) for position-aware encoding
- Chunked approach (8-byte chunks) maintains accuracy at scale
- No corrections needed for >94% accuracy

### What Needs Work
- Current Codebook approach: 0% accuracy (fundamentally broken design)
- SparseVec::encode_data: 10% accuracy (collision from dedup)
- VersionedEmbrFS: Uses 100% verbatim corrections (passthrough storage)

## Target Architecture

### Goals
1. **>94% uncorrected accuracy** for any data type
2. **≤6% manifest overhead** (directory structure, filenames, sparse metadata)
3. **True holographic storage** where data is IN the engram, not alongside it
4. **Leverage ML infrastructure** from rust-ai-core for training/optimization
5. **GPU acceleration** using trit-vsa and vsa-optim-rs kernels

### Architecture Vision

```
┌─────────────────────────────────────────────────────────────────────┐
│                       User Data Input                                │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Trained Universal Codebook                        │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐     │
│  │ Byte Basis  │  │ Position Basis│  │ Pattern Basis (learned)│     │
│  │ (256 vecs)  │  │ (per position)│  │ (from training data)   │     │
│  └─────────────┘  └──────────────┘  └────────────────────────┘     │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│               Position-Aware Holographic Encoding                    │
│                                                                      │
│  For each byte at position i:                                        │
│    encoded[i] = bind(pos_vec[i], byte_vec[data[i]])                 │
│                                                                      │
│  Chunk encoding (8-64 bytes per chunk):                             │
│    chunk_engram = bundle(encoded[0..chunk_size])                    │
│                                                                      │
│  File encoding:                                                      │
│    file_engram = bundle(chunk_engrams[0..n])                        │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Engram Storage                               │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │ Root Engram: Holographic superposition of all data              ││
│  │ - Contains ALL file content holographically                     ││
│  │ - Searchable via similarity                                     ││
│  │ - Decodable with bind/bundle operations                         ││
│  └─────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │ Manifest (≤6% of total size):                                   ││
│  │ - Directory structure (paths)                                   ││
│  │ - File sizes (for decode length)                                ││
│  │ - Chunk boundaries (for chunked decode)                         ││
│  │ - NO raw data, only structural metadata                         ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

## Integration with rust-ai Ecosystem

### rust-ai-core Integration
```rust
// Device selection for GPU acceleration
use rust_ai_core::{get_device, DeviceConfig};

// Memory management for large engrams
use rust_ai_core::memory::{MemoryEstimator, track_allocation};

// Unified error handling
use rust_ai_core::error::CoreError;
```

### trit-vsa Integration
```rust
// Use trit-vsa's optimized VSA operations
use trit_vsa::{PackedTritVec, vsa};

// SIMD-accelerated operations
use trit_vsa::simd::{bundle_simd, bind_simd};

// GPU kernels (when available)
#[cfg(feature = "cuda")]
use trit_vsa::gpu::GpuTritVec;
```

### tritter-accel Integration
```rust
// High-performance ternary operations
use tritter_accel::core::{
    ternary::{PackedTernary, matmul},
    quantization::{quantize_absmean, QuantizeConfig},
};

// VSA gradient compression (applicable to residuals)
use tritter_accel::core::training::GradientCompressor;

// GPU-accelerated ternary matmul for batch encoding
#[cfg(feature = "cuda")]
use tritter_accel::gpu::ternary_matmul_gpu;
```

### bitnet-quantize Integration
```rust
// BitNet b1.58 ternary quantization
use bitnet_quantize::{BitLinear, BitNetConfig};

// AbsMean quantization for position/byte vectors
use bitnet_quantize::quantization::absmean_quantize;

// Efficient ternary weight storage
use bitnet_quantize::layer::TernaryWeight;
```

### vsa-optim-rs Integration
```rust
// Gradient-inspired compression techniques
use vsa_optim_rs::vsa::VSAGradientCompressor;

// Ternary quantization for efficient storage
use vsa_optim_rs::vsa::ternary_quantize_packed;

// CRITICAL: Deterministic phase training for codebook optimization
use vsa_optim_rs::phase::{
    DeterministicPhaseTrainer, DeterministicPhaseConfig, DeterministicPhase
};
```

### DeterministicPhaseTrainer for Codebook Training

The `DeterministicPhaseTrainer` from vsa-optim-rs implements a hybrid training loop:

```text
WARMUP ──► FULL ──► PREDICT ──► CORRECT ──► FULL ──► ...
  │                    │            │
  │                    │            └─► Extract residual, refit model
  │                    └─► Use deterministic predicted gradients
  └─► Build gradient history for model fitting
```

**Key Properties:**
- **Deterministic**: Same seed + same data = identical training trajectory
- **No stochastic operations** in prediction phase
- **Residuals ensure** predictions converge to actual gradients over time

**Application to Codebook Training:**

```rust
use vsa_optim_rs::phase::{DeterministicPhaseTrainer, DeterministicPhaseConfig};

// Configure deterministic training for basis vector optimization
let config = DeterministicPhaseConfig::default()
    .with_warmup_steps(50)       // Build gradient history from data patterns
    .with_full_steps(10)         // Traditional training for accuracy
    .with_predict_steps(40)      // Deterministic predictions for speed
    .with_correct_every(8);      // Periodic correction to prevent drift

// Train codebook basis vectors
let mut trainer = DeterministicPhaseTrainer::new(
    &basis_param_shapes,
    config,
    &device
)?;

for batch in training_data {
    let step_info = trainer.begin_step()?;

    match step_info.phase {
        DeterministicPhase::Warmup |
        DeterministicPhase::Full |
        DeterministicPhase::Correct => {
            // Compute full gradients via backprop
            let loss = compute_reconstruction_loss(&codebook, &batch);
            let gradients = loss.backward()?;
            trainer.record_full_gradients(&gradients)?;
        }
        DeterministicPhase::Predict => {
            // Use deterministic predicted gradients (fast)
            let predicted_grads = trainer.get_predicted_gradients()?;
            apply_gradients(&mut codebook, &predicted_grads);
        }
    }

    trainer.end_step(loss_value)?;
}

// Stats show effective speedup from prediction phases
let stats = trainer.get_stats();
println!("Speedup: {:.2}x", stats.speedup);
```

**Why This Matters for Embeddenator:**
1. **Fast training**: Prediction phase skips expensive backward pass
2. **Deterministic**: Reproducible codebook = reproducible engrams
3. **Correction loop**: Prevents drift, ensures convergence
4. **Adaptive phases**: Auto-adjusts based on loss dynamics

### Integration Architecture

```text
┌─────────────────────────────────────────────────────────────────────┐
│                    Holographic Storage Pipeline                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. BASIS GENERATION (uses bitnet-quantize)                         │
│     ┌──────────────────────────────────────────────┐                │
│     │ // Generate optimal ternary basis vectors    │                │
│     │ position_basis = absmean_quantize(rand_mat)  │                │
│     │ byte_basis = absmean_quantize(byte_patterns) │                │
│     │ // Store efficiently as TernaryWeight        │                │
│     └──────────────────────────────────────────────┘                │
│                                                                      │
│  2. ENCODING (uses trit-vsa + tritter-accel)                        │
│     ┌──────────────────────────────────────────────┐                │
│     │ // Batch encoding with GPU acceleration      │                │
│     │ encoded = ternary_matmul_gpu(                │                │
│     │     position_matrix,                          │                │
│     │     byte_matrix,                              │                │
│     │     QuantizeConfig::absmean()                 │                │
│     │ );                                            │                │
│     │ engram = trit_vsa::bundle_many(&encoded);    │                │
│     └──────────────────────────────────────────────┘                │
│                                                                      │
│  3. RESIDUAL COMPRESSION (uses vsa-optim-rs)                        │
│     ┌──────────────────────────────────────────────┐                │
│     │ // Compress <6% residual using VSA           │                │
│     │ compressor = GradientCompressor::new(config);│                │
│     │ residual_compressed = compressor.compress(   │                │
│     │     residual_bytes, threshold                 │                │
│     │ );                                            │                │
│     └──────────────────────────────────────────────┘                │
│                                                                      │
│  4. ML ENHANCEMENT (uses rust-ai-core + bitnet)                     │
│     ┌──────────────────────────────────────────────┐                │
│     │ // Train BitLinear residual predictor        │                │
│     │ device = rust_ai_core::get_device(config)?;  │                │
│     │ predictor = BitLinear::from_weight(          │                │
│     │     learned_correction_patterns,              │                │
│     │     None, &BitNetConfig::default()            │                │
│     │ )?;                                           │                │
│     │ // 1.58 bits/param = minimal storage         │                │
│     └──────────────────────────────────────────────┘                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Why These Specific Integrations

| Crate | Why It's Useful | Key Feature |
|-------|-----------------|-------------|
| rust-ai-core | Device selection, memory management | CUDA-first with CPU fallback |
| trit-vsa | Native ternary VSA operations | SIMD bind/bundle |
| tritter-accel | GPU ternary matmul | Batch encoding acceleration |
| bitnet-quantize | AbsMean quantization | Optimal basis generation |
| vsa-optim-rs | Gradient compression + deterministic training | DeterministicPhaseTrainer + residual compression |

## Implementation Phases

### Phase 1: Core Reversible Encoding (DONE)
- [x] ReversibleVSAEncoder with position-aware encoding
- [x] Byte basis vectors (256)
- [x] Position basis vectors (dynamic)
- [x] Chunked encode/decode
- [x] 94.12% accuracy achieved

### Phase 2: Codebook Training Infrastructure
- [ ] Integrate with rust-ai-core for device selection
- [ ] **Use DeterministicPhaseTrainer for basis optimization**
  - WARMUP: Collect gradient history from n-gram/pattern analysis
  - FULL: Traditional training to establish accurate basis vectors
  - PREDICT: Deterministic predictions for 4x+ training speedup
  - CORRECT: Residual extraction to prevent drift
- [ ] Implement n-gram frequency analysis for training data
- [ ] Learn pattern basis from representative data
- [ ] Cross-entropy loss for basis optimization
- [ ] GPU-accelerated training via CubeCL

### Phase 3: Filesystem Integration
- [ ] Replace VersionedEmbrFS encoding with ReversibleVSAEncoder
- [ ] Remove verbatim correction layer
- [ ] Update manifest to store only structural metadata
- [ ] Implement chunked file encoding (64-256 byte chunks)
- [ ] Add streaming encode/decode for large files

### Phase 4: Performance Optimization
- [ ] Use trit-vsa's SIMD operations for encode/decode
- [ ] Implement GPU batch encoding via rust-ai-core
- [ ] Memory-mapped engram files for large datasets
- [ ] Parallel chunk processing

### Phase 5: ML-Enhanced Accuracy (Optional)
- [ ] Residual prediction network for <6% error cases
- [ ] Use aphelion-framework for model training
- [ ] Quantized correction model (BitNet-style)
- [ ] Online learning for user-specific patterns

## Key Design Decisions

### Why Position-Aware Binding Works
```
bind(pos_vec, byte_vec) creates a unique representation for each (position, byte) pair.

In ternary VSA:
- bind is self-inverse: bind(bind(a, b), b) ≈ a
- Near-orthogonality: random vectors are almost orthogonal
- Superposition: bundle preserves retrievability

This is why 94% accuracy is achievable without learned components.
```

### Why Codebook Projection Failed
```
The Codebook approach tried to express data as:
  data ≈ Σ (coefficient_i × basis_i)

This fails because:
1. Only 9 basis vectors (too few)
2. Weighted combination doesn't preserve byte values
3. Modulo mapping causes collisions

The fix: Use binding (not weighting) with position vectors.
```

### Chunk Size Trade-offs
| Chunk Size | Accuracy | Retrieval Speed | Memory |
|------------|----------|-----------------|--------|
| 1 byte     | 100%     | Slow (N searches) | High |
| 8 bytes    | 94%      | Fast             | Low |
| 64 bytes   | 85-90%   | Very fast        | Very low |
| 256 bytes  | 70-80%   | Fastest          | Minimal |

Recommended: 8-16 bytes for balance of accuracy and speed.

## Files to Modify

### embeddenator-vsa
- `src/reversible_encoding.rs` - DONE
- `src/codebook.rs` - Add training infrastructure
- `src/lib.rs` - Export new types - DONE

### embeddenator-fs
- `src/fs/versioned_embrfs.rs` - Use ReversibleVSAEncoder
- `src/fs/correction.rs` - Simplify to residual-only
- `src/fs/versioned/manifest.rs` - Reduce to structural metadata

### New Dependencies
```toml
[dependencies]
# Core infrastructure
rust-ai-core = { version = "0.1", features = ["cuda"] }

# Ternary VSA operations
trit-vsa = { version = "0.1", features = ["simd", "cuda"] }

# GPU-accelerated ternary ops
tritter-accel = { version = "0.1", features = ["cuda"] }

# BitNet quantization for basis vectors
bitnet-quantize = { version = "0.1", features = ["cuda"] }

# Gradient compression for residuals
vsa-optim-rs = { version = "0.1" }

# Optional: ML model training
aphelion-framework = { version = "0.1", optional = true }
```

## Success Criteria

1. **Accuracy**: >94% uncorrected for text, >90% for binary
2. **Overhead**: Manifest <6% of total data size
3. **Performance**: Encode >100MB/s, Decode >200MB/s (with GPU)
4. **Compatibility**: Existing engrams can be migrated
5. **Simplicity**: Remove verbatim correction complexity

## Timeline Estimate

| Phase | Description | Complexity |
|-------|-------------|------------|
| 1 | Core encoding | DONE |
| 2 | Training infra | Medium |
| 3 | FS integration | High |
| 4 | Optimization | Medium |
| 5 | ML enhancement | Optional |

## Notes

This plan takes inspiration from:
- Original embeddenator holographic memory concepts
- Kanerva's Sparse Distributed Memory
- VSA gradient compression from vsa-optim-rs
- Modern ML training infrastructure from rust-ai-core

The key insight: **bind(position, content) + bundle** achieves >94% accuracy
without any learned components. Training the codebook can push this higher.
