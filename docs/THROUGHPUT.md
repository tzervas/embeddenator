# Throughput Notes

This document tracks *throughput-focused* changes (encode/decode, ingest/extract) and provides a stable place to anchor profiling/benchmark work.

## 2026-01-01: Reversible VSA encode/decode hot path

### What changed
- `SparseVec::encode_block` no longer allocates per-block permutation vectors.
  - Previously it built `block_indices` + `permuted_indices` to compute `base_idx`.
  - Now it computes `base_idx = (i + shift) % DIM` directly.

- `SparseVec::decode_data` / `decode_block` are now bounded by `expected_size`.
  - `decode_block` accepts `max_len` so decoding work scales with the caller’s expected output size.

- `SparseVec::decode_block` membership checks no longer use `Vec::contains`.
  - It now uses `binary_search` on sorted `pos`/`neg` indices.
  - This removes the nested linear scan that dominated decode time.

### Why it matters
- Ingest/extract calls reversible encode/decode per chunk; decode membership checks were previously the most obvious algorithmic hotspot.
- Bounding decode by the caller’s expected output size prevents unnecessary probe work on short final chunks.

### Verification
- `cargo test` passes including `tests/qa_comprehensive.rs`.

### Next throughput targets (not implemented yet)
- Replace `HashSet`-heavy `bundle`, `bind`, and `cosine` with merge-based set operations on sorted indices.
- Precompute and cache `path_shift` per file path during ingest/extract.
- Stream ingest/extract I/O (avoid whole-file buffers), then parallelize per-chunk encode/decode.

## 2026-01-01: Bundling modes baselines (pairwise vs sum-many vs hybrid)

This project now exposes multiple bundling semantics:

- `SparseVec::bundle` (pairwise conflict-cancel): fast and sparse, but not associative across 3+ vectors.
- `SparseVec::bundle_sum_many`: associative (order-independent), but higher constant-factor cost.
- `SparseVec::bundle_hybrid_many`: chooses between the above using a constant-time collision-risk estimate.

### Recorded benchmarks
From `cargo bench --bench vsa_ops` (release):

- `bundle_modes/pairwise_sparse`: ~81.5 ns
- `bundle_modes/sum_many_sparse`: ~180.4 ns
- `bundle_modes/hybrid_sparse`: ~100.2 ns
- `bundle_modes/pairwise_dense`: ~31.7 µs
- `bundle_modes/sum_many_dense`: ~105.1 µs
- `bundle_modes/hybrid_dense`: ~105.1 µs

Mid-density (packed-threshold probe; `mid_lo` is below `DIM/4` for a pairwise op, `mid_hi` is above):

- `bundle_modes/pairwise_mid_lo`: ~9.93 µs
- `bundle_modes/sum_many_mid_lo`: ~24.45 µs
- `bundle_modes/hybrid_mid_lo`: ~24.63 µs
- `bundle_modes/pairwise_mid_hi`: ~11.44 µs
- `bundle_modes/sum_many_mid_hi`: ~28.72 µs
- `bundle_modes/hybrid_mid_hi`: ~28.70 µs

From `cargo bench --features bt-phase-2 --bench vsa_ops` (release):

- `bundle_modes/pairwise_sparse`: ~83.0 ns
- `bundle_modes/sum_many_sparse`: ~170.8 ns
- `bundle_modes/hybrid_sparse`: ~109.1 ns
- `bundle_modes/pairwise_dense`: ~19.8 µs
- `bundle_modes/sum_many_dense`: ~102.4 µs
- `bundle_modes/hybrid_dense`: ~103.9 µs

Mid-density (same inputs as above):

- `bundle_modes/pairwise_mid_lo`: ~8.79 µs
- `bundle_modes/sum_many_mid_lo`: ~24.90 µs
- `bundle_modes/hybrid_mid_lo`: ~25.05 µs
- `bundle_modes/pairwise_mid_hi`: ~9.11 µs
- `bundle_modes/sum_many_mid_hi`: ~28.81 µs
- `bundle_modes/hybrid_mid_hi`: ~29.38 µs

### Notes and tuning rationale
- The `bt-phase-2` packed fast path materially improves *dense pairwise* bundling.
- The packed bind fast path is now gated to require both operands be individually dense, to avoid
  penalizing sparse workloads under `bt-phase-2`.
- The hybrid currently stays conservative for dense multiway bundles: it selects `bundle_sum_many` when
  expected collisions are above a small budget (currently 32 dimensions), to avoid order sensitivity.
- A future “cost-aware” hybrid mode may intentionally choose pairwise fold in some dense regimes for
  performance, paired with mitigations/corrections for the inaccuracy it introduces.

## 2026-01-01: Packed scratch reuse (bt-phase-2)

### What changed
- `PackedTritVec` now supports allocation reuse via `fill_from_sparsevec`, plus in-place packed ops
  (`bundle_into`, `bind_into`).
- The `bt-phase-2` fast paths in `SparseVec::{bundle, bind, cosine}` reuse thread-local packed scratch
  buffers instead of allocating packed vectors per call.

### Recorded benchmarks
From a second full run of `cargo bench --features bt-phase-2 --bench vsa_ops`:

- `bundle_modes/pairwise_dense`: ~18.14 µs (prior baseline ~19.8 µs)
- `bundle_modes/pairwise_mid_lo`: ~8.17 µs (prior baseline ~8.79 µs)
- `bundle_modes/pairwise_mid_hi`: ~8.10 µs (prior baseline ~9.11 µs)

From `cargo bench --features bt-phase-2 --bench vsa_ops -- packed_path` (packed fast-path isolation):

- `packed_path/bundle_dense_nnz8000_each`: ~17.7 µs
- `packed_path/bind_dense_nnz8000_each`: ~14.7 µs
- `packed_path/cosine_dense_nnz8000_each`: ~10.6 µs

Notes:
- The packed scratch reuse primarily affects benchmarks that *actually take* the packed path
  (notably `bundle_modes/pairwise_dense` and `pairwise_mid_*`).
- `sum_many_*` does not currently use packed operations, so it should not be expected to improve from
  this change; small run-to-run deltas here are typically measurement variance.
- Some nanosecond-scale `sparsevec_ops/*` benches can be noisy; interpret their deltas cautiously.

## Benchmarks and invariants

### Criterion benches
- Run: `cargo bench`
- Benches:
  - `benches/vsa_ops.rs` (bundle/bind/cosine + reversible encode/decode)
  - `benches/retrieval.rs` (inverted index build/query)

Packed-path isolation:
- `cargo bench --features bt-phase-2 --bench vsa_ops -- packed_path`

### Ternary-refactor invariant tests
These tests compare the current implementation against a slow reference implementation to ensure refactors remain aligned.

- Run: `cargo test --features ternary-refactor --test ternary_refactor_invariants`
