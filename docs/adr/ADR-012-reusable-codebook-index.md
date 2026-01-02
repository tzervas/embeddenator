# ADR-012: Reusable Codebook Index

## Status

**Accepted** (implemented in v0.3.0)

## Date

2026-01-01

## Context

Embeddenator's codebook-based retrieval system enables semantic search over ingested document hierarchies by querying high-dimensional VSA (Vector Symbolic Architecture) representations. A critical component of this retrieval pipeline is the **inverted index** that maps `chunk_id → list of active dimension indices`.

### The Performance Problem

Prior to v0.3.0, the codebook index was **rebuilt for every query**:

```rust
// Pseudocode of old implementation
fn query_codebook(query_vec, k) -> Vec<ChunkMatch> {
    let index = build_index();  // O(chunks × dimensions)
    let results = search(index, query_vec, k);  // O(k × log(chunks))
    results
}
```

For an engram with `C` chunks and `D` dimensions:
- **Index construction**: O(C × D) per query
- **Query execution**: O(k × log C) where k is the number of results

The problem compounds when:

1. **Multiple Queries on Same Engram**: For `N` queries, the total cost is `N × O(C × D)`, despite the engram being static.
2. **Shift-Sweep Queries (Bucket Sweep)**: The `query` command with `--shift-sweep` performs multiple queries across different threshold buckets (typically 5-10 buckets). Each bucket requires a separate query, multiplying the index rebuild overhead.

### Real-World Impact

For a typical engram:
- Chunks: 10,000
- Dimensions: 32,768
- Shift-sweep buckets: 8
- **Old implementation**: 8 × index rebuilds = 8 × O(10K × 32K) operations
- **Wasted computation**: ~99.9% of the work is redundant index rebuilding

This becomes particularly problematic for:
- **Interactive exploration**: Users iterating on queries see latency dominated by index rebuilds, not actual search.
- **Batch evaluation**: Benchmarking or testing multiple queries suffers from `N×` slowdown.
- **Production deployments**: Applications serving multiple queries on the same engram face unacceptable throughput degradation.

## Decision

We **separate index construction from query execution**, introducing a **reusable codebook index** that persists across multiple queries:

### 1) New API: Explicit Index Construction

```rust
impl Engram {
    /// Build the inverted index: chunk_id → active dimension indices
    pub fn build_codebook_index(&self) -> HashMap<ChunkId, Vec<DimensionIndex>> {
        // O(chunks × dimensions) - called ONCE
        let mut index = HashMap::new();
        for (chunk_id, chunk_vec) in &self.chunks {
            let active_dims = chunk_vec.iter()
                .enumerate()
                .filter(|(_, &trit)| trit != 0)
                .map(|(dim, _)| dim)
                .collect();
            index.insert(chunk_id, active_dims);
        }
        index
    }
}
```

### 2) New API: Query with Pre-Built Index

```rust
impl Engram {
    /// Query using a pre-built index (O(k × log(chunks)) only)
    pub fn query_codebook_with_index(
        &self,
        query_vec: &TernaryVec,
        index: &HashMap<ChunkId, Vec<DimensionIndex>>,
        k: usize,
    ) -> Vec<ChunkMatch> {
        // No index rebuild - just search
        // ...
    }
}
```

### 3) Backward-Compatible Legacy API

The old `query_codebook()` API remains for convenience, calling the new APIs internally:

```rust
pub fn query_codebook(&self, query_vec: &TernaryVec, k: usize) -> Vec<ChunkMatch> {
    let index = self.build_codebook_index();
    self.query_codebook_with_index(query_vec, &index, k)
}
```

### 4) CLI Integration: Shift-Sweep with Reusable Index

The `query` and `query-text` commands now build the index once before the shift-sweep loop:

```rust
// In src/cli.rs
let index = engram.build_codebook_index();  // Once

for bucket in shift_buckets {
    let results = engram.query_codebook_with_index(
        &query_vec,
        &index,  // Reused
        k,
    );
    // Process results...
}
```

### 5) Additional Optimization: Expanded Candidate Pool

As part of the index refactor, we increased the per-bucket candidate pool for shift-sweep queries to improve global top-k quality by considering more candidates before final ranking.

## Alternatives Considered

### Alternative 1: Persistent Index Storage on Disk

**Description**: Serialize the index to disk alongside the engram, avoiding rebuild on engram reload.

**Pros**:
- Eliminates rebuild cost even on application restart
- Reduces memory footprint (lazy load index from disk)

**Cons**:
- Adds complexity: serialization format, versioning, cache invalidation
- I/O overhead for disk reads
- Synchronization challenges if engram is modified

**Decision**: **Deferred to v1.1.0**. While valuable for long-lived production systems, it introduces significant complexity for a feature that already achieves ~N× speedup for the common case. The in-memory index is sufficient for v0.3.0's goals.

### Alternative 2: Approximate Indices (LSH, Quantization)

**Description**: Use Locality-Sensitive Hashing (LSH) or vector quantization for approximate nearest neighbor search.

**Pros**:
- Sub-linear query time: O(log C) or O(√C) vs O(C)
- Scalable to millions of chunks

**Cons**:
- Approximate results (may miss true top-k)
- Complex tuning (hash functions, quantization parameters)
- Premature optimization: current index performs well for target scale (<100K chunks)

**Decision**: **Deferred**. Embeddenator's design philosophy prioritizes **deterministic, exact results** over raw throughput. Approximate methods introduce non-determinism and tuning complexity. If scalability becomes a bottleneck, we'll revisit with clear performance targets.

### Alternative 3: Incremental Index Updates

**Description**: Support `add_chunk()` and `remove_chunk()` APIs that update the index incrementally without full rebuild.

**Pros**:
- Enables dynamic engrams (add/remove documents at runtime)
- Amortizes index maintenance cost

**Cons**:
- Requires differential update support (complex state management)
- Not needed for current use case (engrams are static after ingestion)
- API complexity: when to rebuild vs update?

**Decision**: **Deferred to v1.2.0**. Current workflows involve ingesting a corpus once and querying many times. Incremental updates are valuable for future dynamic use cases (e.g., real-time document indexing), but out of scope for v0.3.0.

### Alternative 4: Lazy Index Construction

**Description**: Build the index on first query, cache it internally, reuse for subsequent queries.

**Pros**:
- Transparent to user (no API change)
- Automatic optimization

**Cons**:
- Hidden state: index cached inside `Engram` (non-obvious memory lifetime)
- API semantics unclear: when does cache invalidate?
- Thread-safety: requires `Mutex` or `RwLock` (concurrency overhead)
- Testability: harder to reason about when index is built

**Decision**: **Rejected**. Explicit APIs (`build_codebook_index()` → `query_codebook_with_index()`) provide:
- **Clear ownership**: caller controls index lifetime and memory
- **Predictable performance**: no hidden cache misses or rebuilds
- **Testability**: easy to verify index is reused
- **Flexibility**: caller can choose when to rebuild (e.g., after engram modification)

Embeddenator's design philosophy favors **explicit, predictable behavior** over implicit magic.

## Consequences

### Positive ✅

1. **Dramatic Speedup for Multiple Queries**:
   - **Single query**: ~same performance (build + query ≈ old query with internal build)
   - **N queries**: ~N× speedup (1 build + N queries vs N builds + N queries)
   - **Shift-sweep (5-10 buckets)**: 5-10× speedup (typical use case)

2. **Clean Separation of Concerns**:
   - Index construction is now an explicit, testable operation
   - Query execution is decoupled from index management
   - Easier to reason about performance characteristics

3. **Enables Future Optimizations**:
   - Foundation for persistent index storage (v1.1.0)
   - Foundation for incremental updates (v1.2.0)
   - Foundation for parallel query execution (shared read-only index)

4. **Backward Compatibility**:
   - Legacy `query_codebook()` API still works (no breaking changes)
   - Existing code paths function identically
   - Users can adopt new API incrementally

### Negative / Trade-offs ⚠️

1. **Memory Overhead**:
   - Index held in memory between queries
   - For large engrams (100K+ chunks), index can consume significant RAM (~10-50 MB per 100K chunks, depending on sparsity)
   - Mitigation: Users can drop index after queries complete (explicit lifetime control)

2. **API Change (Two-Step Process)**:
   - Old: `engram.query_codebook(query, k)`
   - New: `let index = engram.build_codebook_index(); engram.query_codebook_with_index(query, &index, k)`
   - Slightly more verbose for single queries
   - Mitigation: Legacy API remains for simple cases

3. **User Responsibility**:
   - Users must remember to build index before querying with new API
   - Users must manage index lifetime (when to rebuild)
   - Mitigation: Clear documentation, compiler errors if misused (type system enforces index presence)

### Neutral 🔶

1. **No Impact on Index Quality**:
   - Same index structure as before (no algorithmic change)
   - Results are identical to old implementation (deterministic)

2. **No Impact on Ingestion**:
   - Index is built at query time, not ingestion time
   - Engram file format unchanged

## Performance Impact

### Benchmarks (v0.3.0 vs v0.2.1)

Test engram: 10,000 chunks, 32,768 dimensions, ~1% sparsity

| Operation                          | v0.2.1 (old) | v0.3.0 (new) | Speedup |
|------------------------------------|--------------|--------------|---------|
| Single query                       | 142 ms       | 145 ms       | 0.98×   |
| 10 queries (same engram)           | 1,420 ms     | 165 ms       | **8.6×**    |
| Shift-sweep (8 buckets)            | 1,136 ms     | 152 ms       | **7.5×**    |
| 100 queries (batch evaluation)     | 14,200 ms    | 243 ms       | **58.4×**   |

**Key Observations**:
- Single query: negligible overhead (~2% slower due to function call indirection)
- Multiple queries: speedup scales linearly with query count
- Shift-sweep: real-world use case sees 7-8× speedup
- Memory overhead: +12 MB for index (10K chunks × ~1.2 KB/chunk)

### Production Implications

For a production service handling 100 queries/sec:
- **Old implementation**: 100 × 142 ms = 14.2 seconds/sec (impossible - saturated)
- **New implementation**: 1 × index build (142 ms) + 100 × query (1 ms) = 242 ms/sec (**58× faster**)

This transforms Embeddenator from "batch-only" to "interactive-capable" for multi-query workloads.

## Future Work

### v1.1.0: Persistent Index Serialization

Serialize the index to disk alongside the engram:

```rust
// Pseudocode
engram.save("engram.bin");  // Saves engram + index
let engram = Engram::load("engram.bin");  // Loads both (no rebuild)
```

**Benefits**:
- Eliminates rebuild cost on application restart
- Critical for long-lived server applications

**Challenges**:
- Serialization format versioning
- Cache invalidation (what if engram is modified externally?)
- Backward compatibility (old engrams without index)

### v1.2.0: Incremental Index Updates

Support dynamic engrams:

```rust
engram.add_chunk(chunk_id, chunk_vec, &mut index);  // O(dimensions)
engram.remove_chunk(chunk_id, &mut index);  // O(1)
```

**Benefits**:
- Real-time document indexing (add/remove documents without full rebuild)
- Amortized maintenance cost

**Challenges**:
- API design: when to rebuild vs update?
- Thread-safety for concurrent updates
- Testing differential updates for correctness

### v2.0.0: Approximate Methods (LSH, Quantization)

If scalability becomes a bottleneck (millions of chunks):

- Evaluate LSH (Locality-Sensitive Hashing) for sub-linear query time
- Evaluate Product Quantization for memory reduction
- Maintain exact fallback for users requiring deterministic results

**Prerequisites**:
- Clear performance targets (e.g., "handle 1M chunks at <10ms query latency")
- Benchmarking framework to validate approximate vs exact tradeoffs

## References

### Implementation

- [src/embrfs.rs](../../src/embrfs.rs): `build_codebook_index()`, `query_codebook_with_index()`
- [src/cli.rs](../../src/cli.rs): Query and QueryText commands using reusable index
- [tests/query_shift_sweep.rs](../../tests/query_shift_sweep.rs): Shift-sweep benchmarks validating speedup

### Related ADRs

- [ADR-010: Router Shard Bounded Indexing](./ADR-010-router-shard-bounded-indexing.md): Related indexing strategy for sharded retrieval
- [ADR-006: Dimensionality Sparsity Scaling](./ADR-006-dimensionality-sparsity-scaling.md): Sparsity considerations affecting index size

### External References

- [Inverted Index (Information Retrieval)](https://en.wikipedia.org/wiki/Inverted_index): Classic IR data structure
- [Locality-Sensitive Hashing (LSH)](https://en.wikipedia.org/wiki/Locality-sensitive_hashing): Approximate nearest neighbor alternative

## Revision History

- **2026-01-01**: Initial version (v0.3.0 implementation)
