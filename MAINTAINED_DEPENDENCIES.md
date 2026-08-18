# Maintained Dependencies Ecosystem

**Document Version:** 2.2.0
**Last Updated:** August 18, 2026
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

This document describes the maintained fork ecosystem created to address unmaintained dependencies in the Rust ML ecosystem (`paste`, `gemm`, and candle's use of both).

### Maintained Forks (verified 2026-08-18)

| Original Crate | Maintained Fork | Version | Distribution |
|----------------|-----------------|---------|--------------|
| `paste` | `qlora-paste` | **1.0.21** | [crates.io](https://crates.io/crates/qlora-paste) |
| `gemm` | `qlora-gemm` | **0.20.0** | [crates.io](https://crates.io/crates/qlora-gemm) |
| `candle-*` | `qlora-candle` (**keeps package names** `candle-core` / `candle-nn` / `candle-transformers`) | **0.9.2** | **Git only** — [tzervas/qlora-candle](https://github.com/tzervas/qlora-candle) branch `use-qlora-gemm` |

There is **no** `qlora-candle-core` (or `qlora-candle-nn` / `qlora-candle-transformers`) crate on crates.io. Doc v2.1.0 (2026-01-26) claimed a 0.8.4 crates.io publish. That never happened. Do not `cargo add qlora-candle-core`.

### Upstream PR

- **PR #3335**: [Replace unmaintained gemm with qlora-gemm fork](https://github.com/huggingface/candle/pull/3335)
  - Status: **OPEN** (as of 2026-08-18)
  - Author: @tzervas
  - Repository: huggingface/candle
  - Upstream candle on crates.io is **0.11.0** and still depends on `gemm = "0.19.0"`.
  - This workspace does **not** bump candle to 0.11 until the fork is rebased and #3335 lands or is replaced.

---


## Two workspaces and trit-vsa

Embeddenator pins `trit-vsa = 0.3.0` from crates.io (`default-features = false`); it does not vendor trit-vsa source. `rust-ai` remains a second, separate workspace and is not imported here. Never enable both CUDA features in one build: `embeddenator-vsa/cuda` resolves `cudarc`, while `trit-vsa/cuda` resolves `cubecl`.

In-tree workspace members (git subtrees): `crates/embeddenator-vsa`, `crates/embeddenator-io`, `crates/embeddenator-obs`, `crates/embeddenator-retrieval`. `embeddenator-retrieval` depends on `embeddenator-vsa` via `workspace = true` (first workspace-dep rewrite). `embeddenator-fs` remains a root gitlink; the crates.io `embeddenator-fs = "0.25"` pin is left in `crates/embeddenator-retrieval/Cargo.toml` as a comment (a live dev-dep would pull a second retrieval 0.22.0 and break `cargo test -p`). Do not run retrieval benches in this PR. `embeddenator-io` and `embeddenator-obs` are true leaves (no `embeddenator-vsa`, `trit-vsa`, or `rust-ai` dependency).

## Maintained Fork Ecosystem

### qlora-paste v1.0.21

**Problem:** The `paste` crate (token pasting macros) has been unmaintained since 2023.

**Solution:** `qlora-paste` is a maintained fork:

- Active maintenance and security patches
- Full API compatibility with `paste`
- Published to crates.io

```toml
[dependencies]
qlora-paste = "1.0.21"
```

```rust
use qlora_paste::paste;
```

**Repository:** https://github.com/tzervas/qlora-paste

---

### qlora-gemm v0.20.0

**Problem:** The `gemm` crate depends on unmaintained `paste`.

**Solution:** `qlora-gemm` uses `qlora-paste` and keeps the `gemm` v0.19.x API.

**Crate family (all 0.20.0 on crates.io):** `qlora-gemm`, `qlora-gemm-common`, `qlora-gemm-f16`, `qlora-gemm-f32`, `qlora-gemm-f64`, `qlora-gemm-c32`, `qlora-gemm-c64`.

```toml
[dependencies]
qlora-gemm = "0.20.0"
```

```rust
use qlora_gemm as gemm;
```

**Repository:** https://github.com/tzervas/qlora-gemm

---

### qlora-candle (Candle Fork) — git, not crates.io

**Problem:** Hugging Face `candle` depends on unmaintained `gemm` → `paste`.

**Solution:** `tzervas/qlora-candle` is a **git fork** of candle 0.9.2 that:

- Replaces `gemm` with `qlora-gemm = "0.20.0"`
- **Keeps crate names** `candle-core`, `candle-nn`, `candle-transformers` so `[patch.crates-io]` works
- Is consumed on branch **`use-qlora-gemm`** (default `main` is still vanilla candle 0.9.2 + `gemm 0.19.0` — do not clone `main` and expect the fork)

**Not published.** Renaming the packages to `qlora-candle-*` would **break** `[patch.crates-io]` because cargo patches by package name.

**Repository:** https://github.com/tzervas/qlora-candle/tree/use-qlora-gemm

---

## Dependency Chain

```
Your crate
    └── candle-core / candle-nn / candle-transformers  (= 0.9.2)
            └── [patch.crates-io] → git tzervas/qlora-candle @ use-qlora-gemm
                    └── qlora-gemm v0.20.0          (crates.io)
                            └── qlora-paste v1.0.21 (crates.io)
```

Public standalone sisters (`trit-vsa`, `bitnet-quantize`, `tritter-accel`, `vsa-optim-rs`, `rust-ai-core`) currently declare `candle-core = "0.9"` against **huggingface crates.io** (still `gemm`/`paste`) unless the consumer adds the same patch. Embeddenator itself does **not** depend on candle (GPU path is `cudarc`).

---

## Upstream Contributions

### PR #3335: Replace unmaintained gemm with qlora-gemm

**URL:** https://github.com/huggingface/candle/pull/3335

**Status:** OPEN — awaiting human review. If merged, candle users get maintained `qlora-gemm` without a fork.

Until then, rust-ai (training workspace) uses the git patch below. Do not rebase this pin to candle 0.11 in a docs change.

---

## Integration Guide

### For projects that already depend on `candle-*` (the working pattern)

Depend on huggingface crate names + 0.9.2, then redirect:

```toml
[dependencies]
candle-core = { version = "0.9.2", default-features = false }
candle-nn = { version = "0.9.2", default-features = false }
candle-transformers = { version = "0.9.2", default-features = false }

[patch.crates-io]
candle-core = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
candle-nn = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
candle-transformers = { git = "https://github.com/tzervas/qlora-candle.git", branch = "use-qlora-gemm" }
```

This is what `tzervas/rust-ai` does. Imports stay `use candle_core::...`.

### Do not

```toml
# THESE CRATES DO NOT EXIST ON crates.io
qlora-candle-core = "0.8"
qlora-candle-nn = "0.8"
qlora-candle-transformers = "0.8"
```

### For projects using gemm directly

```toml
qlora-gemm = "0.20.0"
```

```rust
use qlora_gemm as gemm;
```

### For projects using paste directly

```toml
qlora-paste = "1.0.21"
```

```rust
use qlora_paste::paste;
```

---

## For Embeddenator Users

Embeddenator **does not depend on candle**. GPU acceleration in `embeddenator-vsa` is **native CUDA via `cudarc`**.

| Feature | Status | Details |
|---------|--------|---------|
| CUDA via cudarc | Implemented | Optional `cuda` feature |
| SIMD (AVX2) | Implemented | x86_64 acceleration |
| SIMD (NEON) | Implemented | ARM64 acceleration |
| GPU Arch Detection | Implemented | SM 7.5 - SM 12.0 |

If a future `embeddenator-ml` crate needs candle, it will use the **git patch** above (0.9.2, package names unchanged). It will not depend on a non-existent `qlora-candle-core`.

Do **not** enable `embeddenator-vsa` `cuda` (cudarc) and `trit-vsa` `cuda` (cubecl) in one build until a single GPU backend is chosen.

---

## For External Projects

1. **Audit:**

   ```bash
   cargo tree | grep -E "paste|gemm|candle|qlora"
   ```

2. **Paste / gemm:** switch to crates.io `qlora-paste` / `qlora-gemm`.

3. **Candle:** add the `[patch.crates-io]` git snippet. Verify:

   ```bash
   cargo tree -i candle-core
   # source should be tzervas/qlora-candle, not crates.io huggingface/candle
   cargo tree | grep qlora-gemm
   ```

4. **Do not** bump to candle 0.11 in lockstep with this guide. Upstream 0.11 still uses `gemm 0.19.0`.

---

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2026-08-18 | 2.2.0 | **Retract** crates.io `qlora-candle-*` 0.8.4 claim. Candle is git-only @ `use-qlora-gemm`, package names `candle-*` 0.9.2. `qlora-paste` is 1.0.21. Upstream candle is 0.11.0; #3335 still open. |
| 2026-01-26 | 2.1.0 | *(incorrect)* claimed qlora-candle published to crates.io as v0.8.4 |
| 2026-01-26 | 2.0.0 | Expanded to comprehensive maintained dependencies guide |
| 2026-01-26 | 1.0.0 | Initial paste → qlora-paste migration guide |

---

## Related Documentation

- [CI_CD_GUIDE.md](CI_CD_GUIDE.md) — CI/CD documentation (if present in this checkout)
- rust-ai (private training workspace): `MAINTAINED_FORKS_ADOPTION.md` — already matches this 2.2.0 truth

---

## Links

### Maintained Forks

- **qlora-paste:** https://crates.io/crates/qlora-paste
- **qlora-gemm:** https://crates.io/crates/qlora-gemm
- **qlora-candle (git, `use-qlora-gemm`):** https://github.com/tzervas/qlora-candle/tree/use-qlora-gemm

### Upstream

- **Candle PR #3335:** https://github.com/huggingface/candle/pull/3335
- **Candle Repository:** https://github.com/huggingface/candle
- **crates.io candle-core:** https://crates.io/crates/candle-core (huggingface; latest 0.11.0)

### Original (Unmaintained)

- **paste:** https://crates.io/crates/paste
- **gemm:** https://crates.io/crates/gemm

---

**Maintained by:** @tzervas
**Parent epic:** [embeddenator#66](https://github.com/tzervas/embeddenator/issues/66)
