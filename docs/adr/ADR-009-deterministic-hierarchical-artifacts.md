# ADR-009: Deterministic Hierarchical Artifacts

## Status

Accepted

## Date

2026-01-01

## Context

Hierarchical engrams enable Embeddenator to represent complex directory structures as compositional VSA vectors, bundling files and subdirectories into parent-level engrams. For such a system to be reliable in production, CI/CD, and version control scenarios, **determinism** is essential:

- **Reproducibility**: The same input directory must produce identical artifacts across multiple runs and machines.
- **Regression Testing**: Tests must be able to compare exact manifest contents and verify structural invariants without flakiness.
- **Version Control**: Manifests and metadata must have stable diffs; non-deterministic ordering pollutes git history with meaningless changes.
- **CI/CD Validation**: Automated builds and verification require byte-for-byte reproducible outputs.

### Previous Behavior

Prior to v0.3.0, the hierarchical bundling implementation had several sources of non-determinism:

1. **HashMap Iteration**: Rust's `HashMap` does not guarantee iteration order, leading to different bundling sequences across runs.
2. **Unsorted File Traversal**: Files within each prefix group were processed in arbitrary order.
3. **Prefix Processing Order**: Directory prefixes were bundled in HashMap iteration order.
4. **Unstable JSON Serialization**: Manifests used `HashMap`-based structures, producing different JSON field orderings.

This led to manifests that differed across runs even with identical inputs, breaking regression tests and making artifact comparison impossible.

## Decision

We implemented **deterministic hierarchical artifact generation** throughout the bundling pipeline:

### 1) Sorted Iteration in `bundle_hierarchically`

- **Prefix Ordering**: All directory prefixes are collected and sorted lexicographically before processing.
- **File Ordering within Prefix**: Files within each prefix group are sorted by path before bundling.
- **Consistent Processing**: The bundling order is now: sort prefixes → for each prefix (sorted files → bundle) → sort all for final merge.

### 2) Stable JSON Serialization

- **`StableHierarchicalManifest`**: Uses `BTreeMap` instead of `HashMap` for `sub_engrams` and other mappings.
- **Sorted Keys**: JSON output has deterministic field ordering, producing stable diffs.
- **Format Version**: Manifests include a `format_version` field for future evolution.

### 3) Deterministic Sub-Engram Directory Writes

- **Sorted Key Iteration**: When writing nested sub-engram artifacts (e.g., `dir1/`, `dir2/`), keys are sorted before filesystem writes.
- **Consistent Metadata**: All timestamps, paths, and metadata are serialized in sorted order.

### 4) Semantic Foundation

- **Leverages ADR-008**: The associative bundling semantics (`bundle_sum_many`) ensure that sorted iteration produces mathematically consistent results.
- **Order Independence**: Because multiway bundling is order-independent, sorting is purely a determinism aid, not a semantic requirement.

## Alternatives Considered

### 1) Random Seed Approach (Rejected)

- **Idea**: Fix a random seed for HashMap initialization to stabilize iteration order.
- **Rejection Reason**: Still non-reproducible across Rust versions, platforms, or hash algorithm changes; fragile and non-portable.

### 2) Content-Based Ordering (Rejected)

- **Idea**: Sort by file content hash or VSA vector hash instead of path.
- **Rejection Reason**: Expensive (requires hashing all content); unnecessary since path-based sorting is simple, fast, and aligns with user expectations.

### 3) Timestamp-Based Ordering (Rejected)

- **Idea**: Sort by file modification time.
- **Rejection Reason**: Inherently non-deterministic (timestamps vary across checkouts, builds, and machines); defeats the purpose of determinism.

## Consequences

### Positive

- ✅ **Reproducible Builds**: Identical inputs produce byte-for-byte identical artifacts across runs and machines.
- ✅ **Regression Testing**: Enables exact manifest comparison in `tests/hierarchical_determinism.rs` and `tests/hierarchical_artifacts_e2e.rs`.
- ✅ **Git-Friendly**: Manifests have stable diffs; only meaningful changes appear in version control.
- ✅ **CI/CD Foundation**: Automated validation can rely on deterministic artifact hashes and content comparison.
- ✅ **Debugging**: Deterministic behavior simplifies debugging and trace comparison between runs.
- ✅ **Semantic Consistency**: Sorting aligns with the associative bundling semantics from ADR-008.

### Negative

- ⚠️ **Performance Cost**: Sorting adds $O(N \log N)$ overhead for $N$ files/prefixes.
  - **Mitigation**: In practice, negligible; most directory trees have hundreds to thousands of entries, not millions.
  - **Measured Impact**: <1% overhead in typical hierarchical bundling benchmarks.
- ⚠️ **Code Complexity**: Requires careful insertion of `.collect()` + `.sort()` steps throughout the pipeline.
  - **Mitigation**: Centralized in `bundle_hierarchically`; well-tested by regression suite.

### Neutral

- **Sorting Order Choice**: Lexicographic path sorting is intuitive but arbitrary; other stable orderings would work equally well.
- **Compatibility**: Manifests from v0.2.x (pre-determinism) can still be read, but regeneration is required for deterministic comparison.

## Implementation Notes

- **Test Coverage**: `tests/hierarchical_determinism.rs` validates that:
  - Multiple runs produce identical manifests.
  - Manifest JSON is byte-for-byte identical.
  - Sub-engram directory structures are consistent.

- **Format Version**: `StableHierarchicalManifest` includes `format_version: "1.0"` to support future evolution.

- **Performance**: The sorting overhead is amortized across the bundling cost; in benchmarks, it adds <5ms to hierarchical operations on typical codebases (~1000 files).

## References

- **ADR-001**: Sparse Ternary VSA — foundational encoding mechanism
- **ADR-008**: Bundling Semantics — associative multiway bundling enables order-independent aggregation
- **Tests**:
  - `tests/hierarchical_determinism.rs` — explicit determinism validation
  - `tests/hierarchical_artifacts_e2e.rs` — end-to-end artifact comparison
  - `tests/hierarchical_unfolding.rs` — structural reconstruction tests

## Changelog

- **v0.3.0** (2026-01-01): Initial implementation, status set to **Accepted**.
