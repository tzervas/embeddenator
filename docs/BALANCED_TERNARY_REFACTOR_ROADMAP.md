# Balanced Ternary Refactor Roadmap

Goal: refactor Embeddenator to use balanced ternary representations and operations as the *default substrate* wherever it improves performance, resilience, and correctness, while preserving the project’s reconstruction guarantees.

This is a **roadmap + mapping document** (what to refactor, in what order, and how to validate alignment with the current implementation).

## Principles
- Preserve bit-perfect reconstruction invariants (CorrectionStore remains the ground truth).
- Prefer *packed ternary* representations (Tryte/Word) over per-element heap allocations.
- Keep interfaces stable until subcrate split; use adapters during transition.
- Prove equivalence by tests/benches at each stage.

## Current substrate inventory
- Balanced ternary primitives exist (`Trit`, `Tryte3`, `Word6`, `BalancedTernaryWord`) and are already tested.
- VSA vectors are currently modeled as sparse index lists (`SparseVec { pos, neg }`).
- Retrieval currently uses an inverted index over sparse dimensions (`TernaryInvertedIndex`).
- Reconstruction guarantee is enforced at EmbrFS layer via `CorrectionStore`.

## Target state (near-future)
- A *ternary-native vector representation* becomes the primary internal form for operations that are currently HashSet/Vec heavy.
- Sparse index representation remains available as an interchange format (and for extremely sparse regimes), but is no longer the only performant path.
- Retrieval uses ternary-native signatures and/or ternary posting lists, with cosine/dot rerank.

## Phased refactor plan

### Phase A — Measurement + invariants (pre-refactor guardrails)
- Add focused benches for:
  - bundle/bind/cosine
  - encode/decode per chunk
  - retrieval candidate generation + rerank
- Add invariant tests that compare old vs new implementations on randomized inputs (feature-gated if needed).

### Phase B — Ternary-native SparseVec operations
Scope: speed + determinism without changing public APIs.
- Implement optional “packed ternary” fast paths for dot/cosine and bind/bundle.
- Use merge-based sparse ops where it’s best (already done), then add packed paths when density increases.
- Ensure sorting + dedup invariants remain valid.

### Phase C — Ternary-native retrieval
- Add a rerank stage (cosine) over candidates.
- Add ternary signatures (e.g., tryte-packed SimHash / LSH) to avoid large postings scans.
- Add recursive selective unfolding hooks: index sub-engrams on demand and evict.

### Phase D — EmbrFS / manifest alignment for differential updates
- Formalize shard boundaries (directory/sub-engram boundaries).
- Implement differential update recomputation (only changed leaves + ancestor bundles).
- Ensure CorrectionStore updates only for changed chunks.

### Phase E — Subcrate split (next milestone)
This roadmap assumes a later milestone introduces subcrates; when that happens:
- `embeddenator-ternary`: balanced ternary primitives + packed word operations
- `embeddenator-vsa`: vector ops + reversible encoding
- `embeddenator-retrieval`: indexing + search
- `embeddenator-engrams`: EmbrFS + manifests + extraction

## Validation checklist for each phase
- `cargo test` passes.
- QA suite passes.
- Bench deltas are recorded (throughput + memory).
- Reconstruction guarantee tests remain bit-perfect.

## Non-goals (for now)
- Breaking public APIs in the main crate.
- Committing large baseline codebook artifacts into the repo.
