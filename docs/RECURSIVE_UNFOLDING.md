# Recursive Selective Unfolding (Hierarchical Engrams)

Goal: enable *selective retrieval* of only the sub-engram(s) needed for an operation so we minimize active RAM and avoid globally materializing the full dataset.

## Concept
A hierarchical engram is a tree (or DAG) of bundles:
- Root engram summarizes the whole corpus.
- Each sub-engram summarizes a partition (directory, shard, time window, etc.).
- Leaves represent chunk vectors (or per-file/per-chunk bundles).

Selective unfolding means:
1. Use the root to decide *which sub-engram(s) to descend into*.
2. Load/index only those sub-engrams.
3. Query inside them.
4. Repeat until the desired granularity is reached.

## Retrieval algorithm (beam, non-explosive)
Maintain a frontier of nodes (engrams/sub-engrams) with scores.

At each step:
- Expand only top-$b$ nodes (beam width).
- Stop when:
  - depth exceeds $D$,
  - or marginal score gain falls below a threshold,
  - or enough results are found.

This prevents exponential recursion.

## Memory discipline
- Keep only the active frontier and the currently opened sub-engrams.
- Use LRU / generation counters to evict sub-engrams and indices.
- Treat extraction buffers as streaming: decode/apply correction/write, never hold whole large files.

## Differential update strategy (modification operations)
We want to avoid full re-ingest when only a small subset changes.

High-level invariant:
- Unchanged shards/paths keep their existing vectors.
- Only the changed leaf vectors and the bundles on the path to the root are recomputed.

Plan:
1. Detect changes: added/removed/modified files, and which chunks changed.
2. Recompute leaf chunk vectors only for changed chunks.
3. Recompute parent bundles along the affected path(s) only.
4. Validate reconstruction of changed files and update correction store entries.
5. Persist updated engram + manifest.

Note: this will become much cleaner once baseline codebook subcrates land, because we’ll have stable codebook generation + deterministic shard boundaries.

## Preliminary plan for TASK-RET-003 (2026-01-01)

## Implementation status (2026-01-01)

Completed (foundation):
- Added a public hierarchical query entrypoint `query_hierarchical_codebook(...)` with bounds via `HierarchicalQueryBounds`.
- Added a store/loader seam (`SubEngramStore`) and a store-backed query entrypoint (`query_hierarchical_codebook_with_store(...)`).
- Beam-limited traversal with deterministic tie-breaking.
- Per-node inverted index construction over `SubEngram.chunk_ids` with a bounded LRU cache for indices.
- Bounded LRU cache for loaded sub-engrams (via `max_open_engrams`).
- `SubEngram` now carries `chunk_ids` so retrieval can avoid indexing the entire global codebook.
- Tests cover determinism, bounded recursion (`beam_width`, `max_depth`, `max_expansions`), and child descent.

Remaining (to finish TASK-RET-003 fully):
Completed since this note was written:
- Directory-backed `SubEngramStore` (`DirectorySubEngramStore`) with `.subengram` blobs.
- CLI workflow wiring via `bundle-hier` (artifact build) and `query` / `query-text` flags (`--hierarchical-manifest`, `--sub-engrams-dir`).

## End-to-end workflow (CLI)

1) Ingest as usual:
- `embeddenator ingest -i ./input -e root.engram -m manifest.json`

2) Build hierarchical retrieval artifacts (manifest + sub-engrams directory):
- `embeddenator bundle-hier -e root.engram -m manifest.json --out-hierarchical-manifest hier.json --out-sub-engrams-dir ./sub_engrams`

3) Query with selective unfolding:
- `embeddenator query-text -e root.engram --text "search" --hierarchical-manifest hier.json --sub-engrams-dir ./sub_engrams`

### What we are building
Add a retrieval path that searches hierarchical engrams without materializing or indexing the entire corpus at once.

Output goal: top-$k$ chunk hits (and/or file hits) produced via:
1) candidate generation at each node (sub-engram)
2) cosine rerank inside the node
3) controlled descent into child nodes

### Proposed API surface (minimal)
- `Engram::query_codebook(query, k) -> Vec<RerankedResult>` already exists for flat codebooks.
- Add a hierarchical query entrypoint (names TBD):
  - input: root hierarchical manifest + loader for sub-engrams + query vector + bounds
  - output: globally merged top-$k$ results with provenance `(sub_engram_id, chunk_id)`

### Beam traversal algorithm (sketch)
State:
- frontier = max-heap of `(score, sub_engram_id, depth)`
- results = min-heap of best global hits to keep only $k$
- caches:
  - `sub_engram_cache`: id → loaded sub-engram (LRU)
  - `index_cache`: id → built inverted index (LRU)

Loop:
1. Pop best node from frontier.
2. Load node’s engram and build/get its inverted index.
3. Query for `candidate_k` and rerank to get node-local top hits.
4. Merge node-local hits into global results.
5. Decide whether to descend into children:
   - push children into frontier with propagated score (e.g., best local cosine, or a conservative upper bound)
6. Stop on bounds:
   - expansions >= max_expansions
   - depth >= max_depth
   - frontier best score <= current kth-best score (if monotonic scoring is enforceable)

### Memory and I/O discipline
- Sub-engrams loaded on demand; do not retain more than `max_open_engrams`.
- Inverted indices are expensive: cache them separately with a cap (LRU by bytes or by count).
- Prefer lazy decoding/extraction: retrieval shouldn’t decode chunk payloads unless explicitly requested.

### Determinism requirements
- Tie-breaking must be deterministic (id order) so tests don’t flap.
- Cache eviction must not change results given the same bounds.

### Test plan (initial)
- Fixture hierarchy test:
  - Build a small hierarchical engram tree with known content distribution.
  - Ensure selective unfolding finds the known best hits without scanning every node.
- Bounded recursion test:
  - Force deep tree; verify `max_depth` / `max_expansions` bounds are respected.
- Cache behavior test:
  - Configure tiny caches; ensure results remain stable even with evictions.

### Open questions (for deeper research)
- Scoring propagation: what is a safe upper bound for a node’s descendants to justify pruning?
- Granularity: do we retrieve chunk-level only, or add a file-level aggregation layer?
- Storage format: where/how sub-engrams are stored/loaded in practice (single file vs directory vs content-addressed store).
