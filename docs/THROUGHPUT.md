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

## Benchmarks and invariants

### Criterion benches
- Run: `cargo bench`
- Benches:
  - `benches/vsa_ops.rs` (bundle/bind/cosine + reversible encode/decode)
  - `benches/retrieval.rs` (inverted index build/query)

### Ternary-refactor invariant tests
These tests compare the current implementation against a slow reference implementation to ensure refactors remain aligned.

- Run: `cargo test --features ternary-refactor --test ternary_refactor_invariants`
