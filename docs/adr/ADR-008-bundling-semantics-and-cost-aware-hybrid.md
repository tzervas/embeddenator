# ADR-008: Bundling Semantics and Cost-Aware Hybrid (Design Notes)

## Status

Proposed

## Date

2026-01-01

## Context

Embeddenator uses sparse ternary VSA vectors for several distinct roles:

- **Reversible encoding / reconstruction**: bit-perfect reconstruction is enforced via the codebook + correction store.
- **Retrieval and ranking**: cosine similarity and retrieval indices depend on the statistical quality of the vectors.
- **Hierarchical aggregation**: directories and multi-level engrams combine many child vectors.

Bundling is the core superposition operator, and we now support multiple semantics with different trade-offs:

- A fast, sparse **pairwise conflict-cancel** merge.
- A slower but **associative (order-independent)** multiway accumulation.

The project also has an optional packed ternary backend (`bt-phase-2`) that can accelerate operations once vectors become dense.

## Decision

### 1) Make bundling semantics explicit

We keep and document both bundling behaviors:

- `SparseVec::bundle` (pairwise conflict-cancel)
  - Intended for: very sparse regimes, fast merges, and compatibility with legacy behavior.
  - Properties: commutative; generally **not associative** across 3+ vectors.

- `SparseVec::bundle_sum_many` (multiway sum then sign-threshold)
  - Intended for: order-independent aggregation, especially in hierarchical multiway merges.
  - Properties: commutative; **associative** (order-independent) by construction.

### 2) Provide a conservative hybrid selector

- `SparseVec::bundle_hybrid_many` chooses between the above using a constant-time collision estimate.
- Current policy is **integrity/conservatism biased**: if expected collisions exceed a small budget, prefer `bundle_sum_many`.

## Rationale

- Pairwise conflict-cancel bundling is fast and preserves sparsity, but early cancellation discards multiplicity information; across 3+ vectors, order can affect results.
- Multiway sum-then-threshold preserves multiplicity until the end, making it stable under re-ordering and better aligned with “majority over many inputs”.
- For hierarchical engrams, determinism and order-independence are important to prevent subtle drift when directory traversal order changes.

## Cost-Aware Hybrid (Future Work)

A future **cost-aware** hybrid may choose pairwise folding even in regimes where collisions are non-trivial, purely for speed.

This is attractive when:

- The bundled vector is used only as a **retrieval signature** (candidate generation) rather than as the sole source of truth.
- Packed ternary paths make pairwise operations materially cheaper at higher densities.

However, this introduces an accuracy risk:

- Pairwise folding can deviate from the majority-over-all-inputs result.
- In dense multiway merges, this can affect cosine similarity, candidate recall, and determinism under re-grouping.

### Possible mitigations / “tricks up the sleeve”

The goal is to localize any approximation to retrieval-only pathways while keeping correctness guarantees intact.

1) Retrieval-only usage
- Allow cost-aware bundling only for:
  - inverted-index signatures
  - query-time sketching
  - intermediate rerank pruning
- Keep authoritative hierarchical vectors (used for structural traversal) in `bundle_sum_many` form.

2) Confidence gating
- If retrieval results are ambiguous (tight score gaps / too many near-ties), fall back to:
  - a more faithful bundling mode, or
  - an exact verification step (rerank by exact cosine against leaf vectors / node-local codebook).

3) Two-pass “sketch then refine”
- Pass 1: fast cost-aware bundle to generate candidates.
- Pass 2: recompute a faithful aggregation (or a faithful local slice) only on the candidate set.

4) Deterministic group structure
- If using pairwise folding, make it deterministic by:
  - stable ordering of inputs, and
  - fixed reduction tree shape (e.g., balanced tree) rather than sequential fold.

5) Error budgeting
- Parameterize cost-aware selection by a tunable error budget (expected colliding dimensions / expected cancellation rate).
- Prefer “fail-safe”: when uncertain, choose `bundle_sum_many`.

## Consequences

### Positive
- Clear semantics for callers: they can choose deterministic/associative behavior when needed.
- Hybrid provides a simple default that is cheap to evaluate.
- Creates a path to later add cost-aware modes safely, with explicit scope and mitigations.

### Negative
- More API surface area (multiple bundling modes).
- Requires documentation + discipline so hierarchical aggregation does not accidentally become order-sensitive.

## Implementation Notes

- The current hybrid selector uses an expected-collisions estimate based on $\lambda = \tfrac{\text{total nnz}}{\text{DIM}}$ and $P(K\ge2)$ under a Poisson model, with a small collision budget.
- Any cost-aware variant should be introduced as an **explicit opt-in**, likely via a strategy enum/config, not by silently changing defaults.
