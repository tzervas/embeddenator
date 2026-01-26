# ADR-001: VSA Superposition Storage Architecture

**Status**: Accepted
**Date**: 2026-01-26
**Deciders**: @tzervas
**Categories**: Architecture, Storage, Performance

## Context

The embeddenator project aims to provide holographic storage using Vector Symbolic Architecture (VSA). Initial implementation revealed critical storage efficiency issues:

- **Per-chunk storage**: 8 bytes of data required ~33KB of engram storage (4,125x overhead)
- **Root cause**: SparseVec stores indices as 8-byte usize values
- **Dimension**: 10,000 dimensions per chunk creates massive index lists

This made the system unusable for the intended use cases:
- System memory replacement (RAM)
- System storage (filesystem)
- Compute in latent/engram space

## Decision

We will use **hierarchical superposition** as the primary storage strategy:

1. **Chunk-level encoding**: Encode data in small chunks (8-64 bytes) using position-aware binding
2. **Root engram superposition**: Bundle all chunk engrams into a single root engram
3. **Manifest metadata**: Store chunk boundaries and file metadata separately (O(n) but tiny)
4. **Hierarchical indexing**: Optional tree structure for O(log n) retrieval

### Storage Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Superposition Storage Model                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Input Data (N bytes)                                               │
│  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐                │
│  │ C0  │ C1  │ C2  │ C3  │ C4  │ C5  │ ... │ Cn  │  Chunks        │
│  └──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴─────┴──┬──┘                │
│     │     │     │     │     │     │           │                    │
│     ▼     ▼     ▼     ▼     ▼     ▼           ▼                    │
│  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐                │
│  │ E0  │ E1  │ E2  │ E3  │ E4  │ E5  │ ... │ En  │  Engrams       │
│  └──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴─────┴──┬──┘                │
│     │     │     │     │     │     │           │                    │
│     └─────┴─────┴──┬──┴─────┴─────┴───────────┘                    │
│                    │ bundle()                                       │
│                    ▼                                                 │
│           ┌───────────────┐                                         │
│           │  Root Engram  │  Fixed size (~50KB regardless of N)    │
│           │  (superposed) │  Contains ALL data holographically     │
│           └───────────────┘                                         │
│                    +                                                 │
│           ┌───────────────┐                                         │
│           │   Manifest    │  O(n) but ~8 bytes per chunk           │
│           │  (metadata)   │  Contains: boundaries, sizes, paths    │
│           └───────────────┘                                         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Why This Works

1. **Index saturation**: SparseVec indices saturate at ~DIM/2 regardless of chunk count
2. **Sublinear growth**: Root engram grows from ~33KB to ~52KB as data increases to 1MB
3. **Holographic retrieval**: Position vectors can unbind any byte from root
4. **Similarity search**: Root engram enables content-based search across all data

### Benchmark Results

| Data Size | Per-Chunk Storage | Superposed Root | Overhead |
|-----------|-------------------|-----------------|----------|
| 100 bytes | 3.14 KB | 2.88 KB | 2952% |
| 1 KB | 32 KB | 22 KB | 2294% |
| 10 KB | 330 KB | 51 KB | 522% |
| 100 KB | 3.23 MB | 52 KB | **52.8%** |
| 1 MB | 32.4 MB | 52 KB | **5.3%** |

## Consequences

### Positive

- **Dramatic storage reduction**: 5.3% overhead for 1MB vs 3400% per-chunk
- **Holographic properties preserved**: Similarity search, content addressing
- **Scalable**: Storage grows sublinearly with data size
- **System memory viable**: 50KB working set engram fits in L2 cache

### Negative

- **Manifest required**: O(n) metadata for chunk boundaries
- **Sequential decode**: Must know chunk boundaries to decode
- **Accuracy trade-off**: Superposition may reduce per-byte accuracy slightly
- **Complexity**: Two-tier storage (engram + manifest)

### Risks

- **Large manifests**: For millions of files, manifest becomes significant
- **Decode performance**: Must iterate through manifest for random access
- **Index collision**: Heavy superposition may cause retrieval errors

### Mitigations

- **Hierarchical manifests**: Tree structure for O(log n) lookup
- **Block-sparse representation**: Reduce index storage overhead
- **Adaptive chunking**: Larger chunks for repetitive data

## Alternatives Considered

### 1. Per-Chunk Storage (Rejected)
- 4,125x storage overhead
- Unusable for any practical application

### 2. Reduced Dimensions (Partially Adopted)
- Smaller dimensions reduce per-chunk overhead
- But still linear with chunk count
- Used in combination with superposition

### 3. Hierarchical Tree Bundling (Rejected)
- Creates intermediate nodes with overhead
- Benchmark showed 15,000% overhead for 1MB
- Worse than per-chunk at scale

### 4. PackedTritVec Only (Considered for Future)
- 2 bits per dimension = 2.5KB for 10K dims
- Still 312x overhead per chunk
- Good for dense vectors, not sparse

## Implementation Notes

### Phase 1 (Current)
- ReversibleVSAEncoder with chunked encoding
- Manual superposition via bundle()
- Separate manifest storage

### Phase 2 (Planned)
- Integrate into VersionedEmbrFS
- Automatic root bundling on ingest
- Manifest auto-generation

### Phase 3 (Future)
- Block-sparse representation
- GPU-accelerated bundling
- System memory integration

## References

- Benchmark: `embeddenator-vsa/examples/benchmark_superposition.rs`
- Gap Analysis: `docs/benchmarks/GAP_ANALYSIS.md`
- Refactor Plan: `docs/architecture/HOLOGRAPHIC_REFACTOR_PLAN.md`
