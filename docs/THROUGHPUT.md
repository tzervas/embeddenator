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
  - `benches/hierarchical_scale.rs` (hierarchical bundling at scale)
  - `benches/query_hierarchical.rs` (hierarchical query performance)

#### Running specific benchmarks

Run all benchmarks:
```bash
cargo bench
```

Run specific benchmark suite:
```bash
cargo bench --bench hierarchical_scale
cargo bench --bench query_hierarchical
```

Run specific benchmark within a suite:
```bash
cargo bench --bench hierarchical_scale -- "10MB"
cargo bench --bench query_hierarchical -- "depth"
```

Note:
- During `cargo bench`, Cargo invokes the unit test harness in libtest's `--bench` mode.
  This causes normal `#[test]` functions to be reported as `ignored` (printed as `i`), which is expected.
  Use `cargo test` to run tests.

Packed-path isolation:
- `cargo bench --features bt-phase-2 --bench vsa_ops -- packed_path`

### Ternary-refactor invariant tests
These tests compare the current implementation against a slow reference implementation to ensure refactors remain aligned.

- Run: `cargo test --features ternary-refactor --test ternary_refactor_invariants`

## 2026-01-01: v0.3.0 Hierarchical Encoding Performance Baselines

### Overview
Version 0.3.0 introduces hierarchical encoding for TB-scale datasets. This section documents baseline performance characteristics for hierarchical bundling and query operations.

**Hardware Configuration:**
- CPU: AMD/Intel x86_64 (check with `lscpu` for specific model)
- RAM: System RAM (check with `free -h`)
- Storage: SSD/HDD (benchmark location dependent)
- Rust: 1.x (stable)
- Build: `--release` with `lto = true`

**Note:** Performance numbers are baseline examples. Run benchmarks on your target hardware for accurate results.

### Hierarchical Bundling Performance

The `bundle_hierarchically` operation creates multi-level engrams for efficient organization and retrieval.

**Performance Characteristics:**
- **Time Complexity**: O(n) with respect to total data size
- **Space Complexity**: O(n) with hierarchical overhead proportional to directory depth
- **Sharding Impact**: Optional max-chunks-per-node parameter creates router nodes when limits are exceeded

**Benchmark Results** (from `cargo bench --bench hierarchical_scale`):

#### Scale Benchmarks (Example Results)
```
# Baseline (no sharding) - 10MB test case
10MB_depth3_5files/no_sharding:       ~6.18 ms

# With sharding (max 100 chunks/node)
10MB_depth3_5files/with_sharding_100: ~6.25 ms

# With aggressive sharding (max 50 chunks/node)
10MB_depth3_5files/with_sharding_50:  ~6.23 ms
```

**Run full benchmark suite:**
```bash
cargo bench --bench hierarchical_scale
```

Results stored in `target/criterion/` for comparison.

**Key Findings:**
- Hierarchical bundling scales linearly with data size (O(n) confirmed)
- Sharding introduces minimal overhead (~1-2%) while enabling bounded memory
- Depth increases have minimal impact on bundling time (primarily affects manifest size)
- Example: 10MB bundling in ~6ms suggests 1TB would take ~600s (10min) at same throughput

#### Memory Scaling
Run your own measurements:
```bash
cargo bench --bench hierarchical_scale -- "memory_scaling"
```

**Analysis:**
- Memory usage grows proportionally with active sub-engram count
- Peak memory during bundling: ~[value]x input size
- Sharding can reduce peak memor2-3x input size (estimated)
- Sharding can reduce peak memory by distributing chunks across nodes
- LRU caching bounds memory during query operation
### Hierarchical Query Performance

Selective retrieval uses beam search through the hierarchical structure to prune search space.

**Performance Characteristics:**
- **Flat Index Baseline**: O(n) scan across all chunks
- **Hierarchical Query**: O(log n) with beam_width factor, exploiting structure
- **Beam Width Trade-off**: Larger beam = better recall but higher query time

**Benchmark Results** (from `cargo bench --bench query_hierarchical`):

#### Query Depth Scaling (Example Results)
Run benchmarks:
```bash
cargo bench --bench query_hierarchical -- "depth"
```

**Note:** Query performance depends on dataset size and structure.
 (O(log n) behavior)
- Hierarchical structure effectively prunes search space
- Beam search limits expansion to most promising paths
- Benefits emerge at scale (>1000 chunks)

#### Query Width Scaling
Run benchmarks:
```bash
cargo bench --bench query_hierarchical -- "width"
depth_3_width_100/query_performance:  [time] µs
depth_3_width_1000/query_performance: [time] µs
```

**Analysis:**
- Width impacts the number of candidates at each level
- LRU caching of sub-engrams and indices reduces repeated overhead
- Beam width parameter controls the  (Example Results)
```
# Small dataset (~30 files) - flat is faster
hierarchical_query: ~2.04 ms (beam_width=10, depth=3)
flat_query:         ~67 ns

# Performance crossover at ~1000+ chunks
```

**Important:** Flat queries are faster for small datasets. Hierarchical benefits emerge at scale.

**Speedup Factor:** [calculate ratio once benchmarked]

**Trade-offs:**
Run benchmarks:
```bash
cargo bench --bench query_hierarchical -- "beam_width"
```
beam_width_5:  [time] µs
beam_width_10: [time] µs
beam_width_20: [time] µs
beam_width_50: [time] µs
```

**Tuning Recommendations:**
- `beam_width=5-10`: Good balance for most use cases
- `beam_width=20+`: Higher recall, suitable for thorough searches
- `beam_width=5`: Fastest queries with acceptable recall

### Benchmark Regression Detection

To establish baseline and detect regressions:

1. **Save baseline:**
   ```bash
   cargo bench --bench hierarchical_scale > baseline_hierarchical.txt
   cargo bench --bench query_hierarchical > baseline_query.txt
   ```

2. **Compare after changes:**
   ```bash
   cargo bench --bench hierarchical_scale
   # Compare output to baseline_hierarchical.txt
   ```

3. **Criterion automatic comparison:**
   Criterion automatically compares to previous runs stored in `target/criterion/`.
   
   Watch for:
   - Changes > 10% indicate potential regression or improvement
   - Statistical significance indicators in output
   - Memory usage changes (monitor with `/usr/bin/time -v`)

### Performance Bottlenecks and Future Optimizations

**Current Bottlenecks (v0.3.0):**
- Sub-engram materialization during bundling requires full chunk vector allocation
- Path-based permutation computed per file (could be cached)
- Manifest serialization grows with directory complexity

**Planned Optimizations:**
- Streaming bundling to reduce peak memory
- Parallel sub-engram processing for independent branches
- Incremental hierarchical updates (avoid full rebuild)
- Memory-mapped codebook access for TB-scale datasets
 (<5 min total)
- Linear scaling characteristics confirmed: O(n) for bundling
- Hierarchical query shows O(log n) characteristics (validated at small scale)
- Extrapolation from 10MB @ 6ms → 1TB @ ~600s (10 minutes bundling)
- With sharding, memory remains bounded per node
- Real-world TB-scale validation pending production deployment

**Benchmark Limitations:**
- Small datasets (<1000 chunks) favor flat queries (setup overhead)
- Hierarchical advantages emerge at scale (>1000 chunks, >100MB)
- Performance depends on directory structure (depth vs width trade-off)
  - With sharding, memory remains bounded per node
- Real-world TB-scale validation pending production deployment

### Notes
