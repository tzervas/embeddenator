# ADR-010: Router-Shard Bounded Indexing

## Status

**Accepted** (implemented in v0.3.0)

## Date

2026-01-01

## Context

Hierarchical engrams in Embeddenator enable compositional VSA bundling of entire directory structures. However, as directory sizes grow, a critical scalability bottleneck emerges: **unbounded `chunk_ids` growth per node** leads to memory bloat, slow indexing, and degraded query performance.

### The Problem

Prior to v0.3.0, each hierarchical node could accumulate an unbounded list of `chunk_ids` (file paths or identifiers):

- **Large Directories**: A single directory with thousands of files results in one node holding thousands of `chunk_ids`.
- **Memory Bloat**: Each `chunk_id` must be stored in memory during bundling and serialized to the manifest.
- **Index Size Growth**: The inverted index used for retrieval scales as $O(\text{chunks} \times \text{dimensions})$. With thousands of chunks in one node, index construction and lookup becomes prohibitively expensive.
- **Query Performance Degradation**: Retrieval queries must scan all `chunk_ids` in a node to compute distances and rank results, leading to slow response times for large nodes.

### Real-World Impact

In practice:

- A directory with 5,000 source files produces a single node with 5,000 `chunk_ids`.
- The inverted index for such a node consumes significant memory (tens to hundreds of MB).
- Query latency increases linearly with the number of `chunk_ids` per node.
- Systems with limited memory (e.g., embedded devices, CI runners) become unusable.

This unbounded growth violates the principle of **predictable, bounded resource consumption** necessary for production deployments.

## Decision

We introduce **router-shard bounded indexing** to cap the number of `chunk_ids` per node, ensuring bounded memory and index size:

### 1) API Extension: `bundle_hierarchically_with_options`

A new API function allows users to specify a maximum `chunk_ids` per node:

```rust
pub fn bundle_hierarchically_with_options(
    &mut self,
    prefix: &str,
    max_chunks_per_node: Option<usize>,
) -> Result<HierarchicalManifest>
```

- **Parameter**: `max_chunks_per_node: Option<usize>` — if `Some(N)`, nodes exceeding `N` chunks are automatically sharded.
- **Default Behavior**: `None` preserves backward compatibility (unbounded nodes).

### 2) CLI Flag: `--max-chunks-per-node`

```bash
embeddenator bundle --hierarchical --max-chunks-per-node 100 corpus/
```

Users can specify the cap directly from the CLI.

### 3) Router-Shard Mechanism

When a node's `chunk_ids` list exceeds the configured cap:

1. **Router Node Creation**: The original node is converted to a **router node** with no `chunk_ids` itself.
2. **Shard Generation**: Child nodes named `__shard_0000`, `__shard_0001`, ..., `__shard_NNNN` are created.
3. **Deterministic Distribution**: Chunks are distributed to shards based on their **index order** (sorted lexicographically, per ADR-009).
   - Chunks 0–99 → `__shard_0000`
   - Chunks 100–199 → `__shard_0001`
   - And so on.
4. **Shard Engrams**: Each shard holds a subset of the original `chunk_ids` (up to the cap) and bundles them into a shard-level engram.
5. **Router Engram**: The router node's engram is the **associative sum** of all shard engrams (consistent with ADR-008 bundling semantics).

### 4) Deterministic Shard IDs

Shard naming is deterministic and zero-padded:

- `__shard_0000`, `__shard_0001`, ..., `__shard_9999`
- Padding ensures lexicographic sorting matches numeric order.
- Shard structure is reproducible across runs (per ADR-009).

### 5) Query Traversal

During retrieval:

- **Router Nodes**: Query descends into all shard children.
- **Shard Nodes**: Each shard is queried independently; results are merged.
- **Cost Model**: Query cost increases proportionally to the number of shards, but each shard has bounded index size.

## Alternatives Considered

### 1) Fixed-Depth Hierarchies (Rejected)

- **Idea**: Enforce a fixed directory depth limit (e.g., max 4 levels).
- **Rejection Reason**: Inflexible; does not adapt to data distribution. Some directories may need deeper hierarchies while others are shallow.

### 2) Dynamic Rebalancing (Rejected)

- **Idea**: Dynamically redistribute chunks across shards to balance load as data changes.
- **Rejection Reason**: Breaks determinism from ADR-009. Non-deterministic rebalancing would produce different manifest structures across runs, violating reproducibility requirements.

### 3) Bloom Filters for Chunk Lookup (Rejected)

- **Idea**: Use Bloom filters to quickly exclude nodes during queries without loading all `chunk_ids`.
- **Rejection Reason**: Does not address the **fundamental scaling issue** — the inverted index and memory footprint still grow unbounded. Bloom filters are a query optimization, not a solution to memory bloat.

### 4) No Cap (Status Quo - Rejected)

- **Idea**: Keep the existing unbounded behavior.
- **Rejection Reason**: Unacceptable for production systems handling large datasets. Memory and index size grow without bound, leading to system failures.

## Consequences

### Positive

- ✅ **Bounded Per-Node Memory**: Each node (router or shard) holds at most `max_chunks_per_node` chunks, ensuring predictable memory consumption.
- ✅ **Bounded Index Size**: Inverted index size per shard is capped, improving index construction speed and memory efficiency.
- ✅ **Predictable Query Cost**: Query latency per node is bounded; total cost scales with the number of shards (known at manifest-load time).
- ✅ **Deterministic Shard Structure**: Shard generation is reproducible (per ADR-009), enabling regression testing and version control.
- ✅ **Backward Compatible**: The `Option<usize>` parameter defaults to `None`, preserving existing behavior for users who don't need sharding.
- ✅ **Scalability**: Systems can now handle directories with tens of thousands of files without memory exhaustion.

### Negative

- ⚠️ **Increased Traversal Cost**: Queries must visit all shards in a router node, increasing the number of node visits.
  - **Mitigation**: Each shard has bounded cost; total cost is still predictable and acceptable for large datasets.
  - **Future Work**: Parallel shard querying can amortize this cost.
- ⚠️ **Manifest Size Growth**: Router nodes and shard nodes increase the total number of nodes in the manifest.
  - **Mitigation**: Manifest compression and lazy-loading can reduce storage overhead.
  - **Trade-off**: Manifest size growth is acceptable compared to the alternative (unbounded memory bloat).
- ⚠️ **Configuration Complexity**: Users must choose an appropriate `max_chunks_per_node` value.
  - **Mitigation**: Documentation provides guidance (e.g., 100–1000 is typical); CLI defaults to unbounded for simplicity.

### Neutral

- **Shard Granularity**: The optimal `max_chunks_per_node` value depends on available memory and query latency requirements.
  - **Typical Range**: 100–1000 chunks per node balances memory and query performance.
  - **Future Work**: Adaptive thresholding based on available system memory.

## Implementation Notes

### Code Structure

- **API**: `EmbrFS::bundle_hierarchically_with_options(prefix, max_chunks_per_node)` in [src/embrfs.rs](src/embrfs.rs).
- **CLI**: `--max-chunks-per-node N` flag in [src/cli.rs](src/cli.rs).
- **Sharding Logic**: Implemented in the hierarchical bundling loop; when a node exceeds the cap:
  - Sort `chunk_ids` lexicographically.
  - Partition into shards of size `max_chunks_per_node`.
  - Create router node with `sub_engrams` mapping to shard nodes.
- **Manifest Format**: Shard nodes appear as `__shard_NNNN` entries in the `sub_engrams` map of router nodes.

### Test Coverage

- **`tests/hierarchical_determinism.rs`**: Validates that sharded manifests are deterministic across runs.
- **Manual Testing**: Verified with large directories (5,000+ files) to confirm memory and index size reductions.
- **Regression**: Ensures that existing tests pass with `max_chunks_per_node: None`.

### Performance Characteristics

- **Memory Reduction**: For a directory with 5,000 files and `max_chunks_per_node = 100`:
  - **Before**: 1 node with 5,000 `chunk_ids`.
  - **After**: 1 router + 50 shards, each with ≤100 `chunk_ids`.
  - **Savings**: Per-node memory reduced by 50×; total memory slightly higher due to router overhead, but much more manageable.
- **Query Latency**: Query must visit 50 shards instead of 1 node, but each shard's index is 50× smaller, resulting in similar or better total latency.

## Future Work

### 1) Adaptive Threshold

- **Goal**: Automatically adjust `max_chunks_per_node` based on available system memory.
- **Approach**: Monitor memory usage during bundling; dynamically reduce the cap if memory pressure is detected.

### 2) Shard Consolidation

- **Goal**: Merge underfull shards (e.g., shards with <10 chunks) to reduce manifest bloat.
- **Challenge**: Must maintain determinism (per ADR-009).
- **Approach**: Only consolidate during explicit "optimize" operations, not during incremental updates.

### 3) Parallel Shard Querying

- **Goal**: Query all shards in a router node concurrently to reduce latency.
- **Approach**: Use Rayon or async tasks to parallelize shard retrieval.
- **Expected Speedup**: Linear with the number of CPU cores, up to the number of shards.

### 4) Shard-Level Indexing Optimization

- **Goal**: Use specialized index structures (e.g., hierarchical navigable small world graphs) within each shard.
- **Benefit**: Further reduce per-shard query latency.

## References

- **ADR-001**: Sparse Ternary VSA — foundational encoding mechanism.
- **ADR-008**: Bundling Semantics — associative multiway bundling enables router engrams as sums of shard engrams.
- **ADR-009**: Deterministic Hierarchical Artifacts — ensures shard structure is reproducible.
- **Implementation**:
  - `EmbrFS::bundle_hierarchically_with_options()` in [src/embrfs.rs](src/embrfs.rs).
  - CLI flag `--max-chunks-per-node` in [src/cli.rs](src/cli.rs).
- **Tests**:
  - `tests/hierarchical_determinism.rs` — validates deterministic shard generation.
  - Manual testing with large directories (5,000+ files).

## Changelog

- **v0.3.0** (2026-01-01): Initial implementation, status set to **Accepted**.

