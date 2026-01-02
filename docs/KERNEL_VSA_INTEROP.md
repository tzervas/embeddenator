# Kernel ↔ VSA Interop (Non-FUSE builds)

## Why this exists
In non-FUSE builds, we still need a stable seam between the “kernel boundary” (CLI / orchestrator / future holographic OS runtime) and the VSA substrate.

FUSE is just one presentation layer. The underlying “kernel” still needs:
- a way to encode/decode bytes to/from vectors (reversible layer + correction)
- a way to combine and compare vectors (bundle/bind/dot/cosine)
- a way to query stored vectors efficiently (index + rerank)

This interop layer is intended to:
- keep non-FUSE builds first-class
- reduce feature-flag entanglement
- make future subcrate splits easier (kernel/runtime shouldn’t depend on internal VSA representations)

## Minimal API surface (traits)

### `VsaBackend`
A trait capturing the operations the kernel needs, without committing to `SparseVec` vs a future ternary-native packed representation.

Implemented in-tree today in [src/kernel_interop.rs](../src/kernel_interop.rs) as:
- `VsaBackend` (trait)
- `SparseVecBackend` (default implementation)

```rust
use embeddenator::kernel_interop::{SparseVecBackend, VsaBackend};
use embeddenator::ReversibleVSAConfig;

let backend = SparseVecBackend;
let cfg = ReversibleVSAConfig::default();

let v1 = backend.encode_data(b"hello", &cfg, None);
let v2 = backend.encode_data(b"world", &cfg, None);
let bundled = backend.bundle(&v1, &v2);
let sim = backend.cosine(&v1, &bundled);
assert!(sim > 0.0);
```

### `VectorStore`
A trait for “codebook-like” storage the kernel queries.

In-tree this is intentionally minimal (`get(id)` only) to keep the boundary stable.

### Retrieval interop
The kernel path should be:
1) candidate generation: inverted index → candidate IDs / approximate scores
2) rerank: exact cosine on those IDs using the store’s vectors

The interop module provides a backend/store-driven helper:
- `rerank_top_k_by_cosine(backend, store, query, candidate_ids, k)`

## Where this lives (now)
- Add a new module: `src/kernel_interop.rs`
- Keep it dependency-light: only depends on `vsa` types and (optionally) `retrieval`.
- Expose it from `src/lib.rs` so the CLI/orchestrator can use it.

## Call paths

### CLI (non-FUSE)
- parse query input
- load Engram
- query `Engram.codebook` using an inverted index + cosine rerank
- print top chunk matches + overall root similarity

### Future runtime/kernel
- wrap Engram as a `VectorStore`
- select backend (`SparseVec` first; later packed) behind `VsaOps`

## Migration story (balanced ternary)
- Phase 1: `SparseVecBackend` implements `VsaBackend` by delegating to existing `SparseVec` operations.
- Phase 2: add a packed-ternary backend and validate via `bt-phase-*` equivalence tests.
- Phase 3: kernel chooses backend based on config/heuristics.
