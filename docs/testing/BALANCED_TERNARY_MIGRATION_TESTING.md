# Balanced Ternary Migration Testing (Phased)

This project is migrating from a sparse-index ternary representation (`SparseVec`) toward a ternary-native packed substrate (`PackedTritVec` and successors) as part of the "balanced ternary everywhere" refactor.

To avoid regressions, testing shifts in **phases**. Each phase enables stricter equivalence suites and invariants.

## Feature flags

- `bt-phase-1`
  - Enables heavy refactor invariants (`ternary-refactor`) and adds phase-1 equivalence checks.
  - Goal: prove that `PackedTritVec` basic ops match existing `SparseVec` semantics.

- `bt-phase-2`
  - Reserved for when packed fast paths start replacing hot `SparseVec` internals.
  - Will add higher-coverage invariants and cross-backend comparisons (e.g., packed vs word-wise packed).

- `bt-phase-3`
  - Reserved for when balanced-ternary-native execution is the default substrate.
  - Will add end-to-end invariants that assert reconstruction/correction remains sacred.

- `bt-migration`
  - Convenience umbrella for the highest currently-implemented phase.

## How to run

Baseline (fast default suite):

- `cargo test`

Phase 1 (packed equivalence + refactor invariants):

- `cargo test --features bt-phase-1`

All migration suites currently implemented:

- `cargo test --features bt-migration`

## Notes

- These phases are intentionally feature-gated so default CI/dev loops remain fast.
- As we land new ternary-native implementations, we move assertions from feature-gated tests into the default suite only when stable.
