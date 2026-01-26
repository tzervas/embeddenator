# Maintained Dependencies Ecosystem

**Document Version:** 2.1.0
**Last Updated:** January 26, 2026
**Status:** Active

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Maintained Fork Ecosystem](#maintained-fork-ecosystem)
3. [Dependency Chain](#dependency-chain)
4. [Upstream Contributions](#upstream-contributions)
5. [Integration Guide](#integration-guide)
6. [For Embeddenator Users](#for-embeddenator-users)
7. [For External Projects](#for-external-projects)

---

## Executive Summary

This document describes the maintained fork ecosystem created to address unmaintained dependencies in the Rust ML ecosystem. These forks provide drop-in replacements with active maintenance, security patches, and compatibility with modern Rust toolchains.

### Maintained Forks

| Original Crate | Maintained Fork | Version | crates.io |
|----------------|-----------------|---------|-----------|
| `paste` | `qlora-paste` | 1.0.20 | [qlora-paste](https://crates.io/crates/qlora-paste) |
| `gemm` | `qlora-gemm` | 0.20.0 | [qlora-gemm](https://crates.io/crates/qlora-gemm) |
| `candle-*` | `qlora-candle-*` | 0.8.4 | [qlora-candle-core](https://crates.io/crates/qlora-candle-core) |

### Upstream PR

These changes are being contributed back upstream:

- **PR #3335**: [Replace unmaintained gemm with qlora-gemm fork](https://github.com/huggingface/candle/pull/3335)
  - Status: **OPEN**
  - Author: @tzervas
  - Repository: huggingface/candle

---

## Maintained Fork Ecosystem

### qlora-paste v1.0.20

**Problem:** The `paste` crate (token pasting macros) has been unmaintained since 2023, with no responses to issues or PRs.

**Solution:** `qlora-paste` is a maintained fork providing:
- Active maintenance and security patches
- Full API compatibility with `paste`
- Compatible with Rust 1.84+
- Published to crates.io

**Usage:**
```toml
[dependencies]
qlora-paste = "1.0.20"
```

```rust
use qlora_paste::paste;

paste! {
    fn [<my_ function>]() {
        // Token pasting works identically
    }
}
```

**Repository:** https://github.com/tzervas/qlora-paste

---

### qlora-gemm v0.20.0

**Problem:** The `gemm` crate (matrix multiplication) depends on unmaintained `paste` and has compatibility issues.

**Solution:** `qlora-gemm` is a maintained fork providing:
- Uses `qlora-paste` instead of unmaintained `paste`
- Full API compatibility with `gemm` v0.19.x
- Active maintenance
- Published to crates.io

**Crate Family:**
| Crate | Version | Description |
|-------|---------|-------------|
| `qlora-gemm` | 0.20.0 | Core matrix multiplication |
| `qlora-gemm-common` | 0.20.0 | Common utilities |
| `qlora-gemm-f16` | 0.20.0 | f16 matrix multiplication |
| `qlora-gemm-f32` | 0.20.0 | f32 matrix multiplication |
| `qlora-gemm-f64` | 0.20.0 | f64 matrix multiplication |

**Usage:**
```toml
[dependencies]
qlora-gemm = "0.20.0"
```

```rust
use qlora_gemm as gemm;  // Drop-in replacement

// All gemm APIs work identically
```

**Repository:** https://github.com/tzervas/qlora-gemm

---

### qlora-candle (Candle Fork)

**Problem:** Hugging Face's `candle` ML framework depends on unmaintained `gemm`, which depends on unmaintained `paste`.

**Solution:** `qlora-candle` is a fork of candle that:
- Uses `qlora-gemm` instead of `gemm`
- Transitively uses `qlora-paste` instead of `paste`
- Maintains full API compatibility with upstream candle
- **Published to crates.io** (as of January 2026)

**Crate Family:**
| Crate | Version | Description |
|-------|---------|-------------|
| `qlora-candle-core` | 0.8.4 | Core tensor operations |
| `qlora-candle-nn` | 0.8.4 | Neural network layers |
| `qlora-candle-transformers` | 0.8.4 | Transformer implementations |

**Repository:** https://github.com/tzervas/qlora-candle

---

## Dependency Chain

```
Your Project
    └── qlora-candle (candle fork)
            └── qlora-gemm v0.20.0
                    └── qlora-paste v1.0.20
```

This chain ensures all dependencies are actively maintained with no unmaintained transitive dependencies.

---

## Upstream Contributions

### PR #3335: Replace unmaintained gemm with qlora-gemm fork

**URL:** https://github.com/huggingface/candle/pull/3335

**Summary:** This PR proposes replacing the unmaintained `gemm` dependency in candle with `qlora-gemm`.

**Changes:**
- `Cargo.toml`: `gemm = "0.19.0"` → `qlora-gemm = "0.20.0"`
- `candle-core/Cargo.toml`: `gemm` → `qlora-gemm`
- `candle-core/src/cpu_backend/mod.rs`: `use gemm` → `use qlora_gemm`

**Status:** OPEN - Awaiting review from Hugging Face maintainers

**Impact:** If merged, all candle users will automatically benefit from maintained dependencies without needing to use the fork.

---

## Integration Guide

### For New Projects

If starting a new project that needs candle with maintained dependencies:

```toml
[dependencies]
qlora-candle-core = "0.8"
qlora-candle-nn = "0.8"
qlora-candle-transformers = "0.8"
```

Update imports to use the qlora prefixed crates:
```rust
use qlora_candle_core as candle_core;  // Drop-in replacement
use qlora_candle_nn as candle_nn;
use qlora_candle_transformers as candle_transformers;
```

### For Existing Projects Using Candle

Add patch directives to replace candle with the maintained fork:

```toml
[patch.crates-io]
candle-core = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
candle-nn = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
candle-transformers = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
```

Or switch to the crates.io published versions directly (recommended):
```toml
[dependencies]
qlora-candle-core = "0.8"
qlora-candle-nn = "0.8"
qlora-candle-transformers = "0.8"
```

### For Projects Using gemm Directly

Replace `gemm` with `qlora-gemm`:

```toml
[dependencies]
# Before:
# gemm = "0.19"

# After:
qlora-gemm = "0.20.0"
```

Update imports:
```rust
// Before:
use gemm::gemm;

// After:
use qlora_gemm::gemm;
// Or for minimal code changes:
use qlora_gemm as gemm;
```

### For Projects Using paste Directly

Replace `paste` with `qlora-paste`:

```toml
[dependencies]
# Before:
# paste = "1.0"

# After:
qlora-paste = "1.0.20"
```

Update imports:
```rust
// Before:
use paste::paste;

// After:
use qlora_paste::paste;
```

---

## For Embeddenator Users

### Current Status

The Embeddenator workspace currently uses **native CUDA via `cudarc`** for GPU acceleration, not candle. This provides:

- Direct CUDA kernel execution
- Fine-grained control over GPU memory
- Optimized for Vector Symbolic Architecture operations

### Future Candle Integration

If candle integration is added to Embeddenator (e.g., for neural network-based embedding generation), it will use the maintained fork ecosystem:

```toml
# Future embeddenator-ml/Cargo.toml (hypothetical)
[dependencies]
qlora-candle-core = "0.8"
qlora-candle-nn = "0.8"
```

### GPU Acceleration in Embeddenator

Current GPU support in `embeddenator-vsa`:

| Feature | Status | Details |
|---------|--------|---------|
| CUDA via cudarc | Implemented | Optional `cuda` feature |
| SIMD (AVX2) | Implemented | x86_64 acceleration |
| SIMD (NEON) | Implemented | ARM64 acceleration |
| GPU Arch Detection | Implemented | SM 7.5 - SM 12.0 |

---

## For External Projects

### Adopting the Maintained Ecosystem

1. **Audit your dependencies:**
   ```bash
   cargo tree | grep -E "paste|gemm|candle"
   ```

2. **Option A: Direct crates.io dependencies (recommended):**
   ```toml
   [dependencies]
   qlora-candle-core = "0.8"
   qlora-candle-nn = "0.8"
   qlora-candle-transformers = "0.8"
   ```

3. **Option B: Patch existing candle dependencies:**
   ```toml
   [patch.crates-io]
   candle-core = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
   candle-nn = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
   candle-transformers = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
   ```

4. **Verify the maintained dependencies are applied:**
   ```bash
   cargo tree | grep qlora
   # Should show qlora-gemm and qlora-paste in the tree
   ```

5. **Run tests:**
   ```bash
   cargo test --all-features
   ```

### Security Considerations

Using maintained forks provides:
- Active vulnerability monitoring
- Timely security patches
- Compatibility with `cargo audit` and `cargo deny`

---

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2026-01-26 | 2.1.0 | qlora-candle published to crates.io (v0.8.4); updated integration guide |
| 2026-01-26 | 2.0.0 | Expanded to comprehensive maintained dependencies guide |
| 2026-01-26 | 1.0.0 | Initial paste → qlora-paste migration guide |

---

## Related Documentation

- [SECURITY.md](SECURITY.md) - Security policy
- [CI_CD_GUIDE.md](CI_CD_GUIDE.md) - CI/CD documentation
- [deny.toml](deny.toml) - Supply chain security configuration

---

## Links

### Maintained Forks
- **qlora-paste:** https://crates.io/crates/qlora-paste
- **qlora-gemm:** https://crates.io/crates/qlora-gemm
- **qlora-candle-core:** https://crates.io/crates/qlora-candle-core
- **qlora-candle (GitHub):** https://github.com/tzervas/qlora-candle

### Upstream
- **Candle PR #3335:** https://github.com/huggingface/candle/pull/3335
- **Candle Repository:** https://github.com/huggingface/candle

### Original (Unmaintained)
- **paste:** https://crates.io/crates/paste (unmaintained)
- **gemm:** https://crates.io/crates/gemm (unmaintained)

---

**Maintained by:** @tzervas
**Questions?** Open an issue on GitHub
