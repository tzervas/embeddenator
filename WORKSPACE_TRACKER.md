# Embeddenator Workspace Tracker

Last updated: 2026-01-16

---

## Overall Goals
- Unify component repos under clear release plan (alpha → beta → RC → stable)
- Stabilize APIs and harden FUSE + CLI for real-world usage
- Achieve performance targets: sub-100ms ops, 50–100+ MB/s throughput, <1GB memory for TB-scale via phase 2
- Establish reproducible CI across amd64 and enable ARM64 when infra available

## Repo Snapshots

- embeddenator
  - Branch: docs/qa-documentation-update-2026-01-10 ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator/docs%2Fqa-documentation-update-2026-01-10) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator)
  - Remote: https://github.com/tzervas/embeddenator.git
  - Key docs: [embeddenator/README.md](embeddenator/README.md), [embeddenator/VERSION_ROADMAP.md](embeddenator/VERSION_ROADMAP.md), [embeddenator/PROJECT_STATUS.md](embeddenator/PROJECT_STATUS.md), [embeddenator/PERFORMANCE_TUNING_STRATEGY.md](embeddenator/PERFORMANCE_TUNING_STRATEGY.md)
  - Status: Internal v1.0.0 achieved; public v0.20.0-alpha.1; test coverage high; SIMD optional; several P2 items deferred
  - Next: Compression options (zstd/lz4), GPU runner research, FUSE hardening

- embeddenator-fs
  - Branch: main ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-fs/main) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-fs)
  - Remote: https://github.com/tzervas/embeddenator-fs.git
  - Key docs: [embeddenator-fs/ROADMAP.md](embeddenator-fs/ROADMAP.md), [embeddenator-fs/IMPLEMENTATION_PLAN_V2.md](embeddenator-fs/IMPLEMENTATION_PLAN_V2.md), [embeddenator-fs/MUTABILITY_PLAN.md](embeddenator-fs/MUTABILITY_PLAN.md), [embeddenator-fs/GAP_ANALYSIS.md](embeddenator-fs/GAP_ANALYSIS.md), [embeddenator-fs/docs/STATUS.md](embeddenator-fs/docs/STATUS.md)
  - Status: Alpha (0.20.0-alpha.3); docs and tests strong; critical pivot to read-write engrams; FUSE presently read-only
  - Next: P0 CLI app + examples; integration tests and benches; implement transactional write model + fine-grained locks; update docs to remove read-only assumptions

- embeddenator-cli
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-cli/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-cli)
  - Remote: https://github.com/tzervas/embeddenator-cli.git
  - Status: CLI commands defined; update subcommands stubbed pending FS methods
  - Next: Wire add_file/remove_file/modify_file/compact; docs + tests; publish alpha

- embeddenator-io
  - Branch: fix/formatting ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-io/fix%2Fformatting) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-io)
  - Remote: https://github.com/tzervas/embeddenator-io.git
  - Status: IO primitives; formatting branch active
  - Next: Align versions with workspace; finalize formatting and docs

- embeddenator-retrieval
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-retrieval/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-retrieval)
  - Remote: https://github.com/tzervas/embeddenator-retrieval.git
  - Status: Retrieval/resonator basic; benches via criterion
  - Next: Performance characterization; hierarchical query integration

- embeddenator-interop
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-interop/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-interop)
  - Remote: https://github.com/tzervas/embeddenator-interop.git
  - Status: Python/FFI bindings extracted; ADR references
  - Next: Build/test wheels; API surface stabilization

- embeddenator-obs
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-obs/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-obs)
  - Remote: https://github.com/tzervas/embeddenator-obs.git
  - Status: Observability foundations
  - Next: tracing integration across crates; dashboards

- embeddenator-testkit
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-testkit/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-testkit)
  - Remote: https://github.com/tzervas/embeddenator-testkit.git
  - Key docs: [embeddenator-testkit/INDEX.md](embeddenator-testkit/INDEX.md), [embeddenator-testkit/EVALUATION_RESULTS.md](embeddenator-testkit/EVALUATION_RESULTS.md), [embeddenator-testkit/EVALUATION_LOOP_GUIDE.md](embeddenator-testkit/EVALUATION_LOOP_GUIDE.md)
  - Status: 14/14 phases passing; strong evaluation loop; baselines established
  - Next: Integrate with CI; expand large-scale datasets; perf regression detection

- embeddenator-vsa
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-vsa/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-vsa)
  - Remote: https://github.com/tzervas/embeddenator-vsa.git
  - Key doc: [embeddenator-vsa/REFACTORING_ANALYSIS.md](embeddenator-vsa/REFACTORING_ANALYSIS.md)
  - Status: Tests passing; dependabot fixes merged; refactor plan identified
  - Next: Complete bundle_hybrid_many(), implement real AVX2/NEON, unify error handling (VsaError), integrate PackedTritVec, externalize projection constants

- embeddenator-workspace
  - Branch: release/alpha-publish ![last commit](https://img.shields.io/github/last-commit/tzervas/embeddenator-workspace/release%2Falpha-publish) ![issues](https://img.shields.io/github/issues/tzervas/embeddenator-workspace)
  - Remote: https://github.com/tzervas/embeddenator-workspace.git
  - Status: Internal helper tooling
  - Next: Consolidate orchestration and shared CI utilities

## Cross-Cutting Workstreams
- Release Coordination: tag alpha/beta across repos; align Cargo versions
- Performance Phase 2: memory optimizations (packing, mmap), optional compression
- Read-Write Pivot: transactional write model in FS; cascade updates to CLI/docs/tests
- Observability: `tracing` across crates; structured logs; perf dashboards
- CI & Publishing: amd64 CI green; enable ARM64 when infra ready; crates.io publish steps

## Release Readiness Checklist
- Docs: READMEs accurate; remove read-only claims where pivoted
- Tests: unit/integration/e2e passing; doctests updated (fs)
- Perf: validate targets via [embeddenator/perf_validate.sh](embeddenator/perf_validate.sh)
- Security: no unsafe issues; audit dependencies
- CI: amd64 required checks; arm64 queued (deferred)
- Versioning: CHANGELOGs updated; semver policy clear

## Next Actions Template
- Owner:
- Repo:
- Milestone:
- Action:
- Priority (P0/P1/P2):
- Due:
- Notes:

## Risks & Blockers
- ARM64 CI infra unavailable (deferred)
- Read-only assumptions pervasive in FS docs/tests (requires coordinated updates)
- SIMD correctness not fully validated for edge cases

## Test/Eval/Perf Summary
- Evaluation loop: see [embeddenator-testkit/INDEX.md](embeddenator-testkit/INDEX.md) and [embeddenator-testkit/EVALUATION_RESULTS.md](embeddenator-testkit/EVALUATION_RESULTS.md)
- Performance plan: see [embeddenator/PERFORMANCE_TUNING_STRATEGY.md](embeddenator/PERFORMANCE_TUNING_STRATEGY.md)
- Profiling toolkit: see [embeddenator/PROFILING_TOOLKIT.md](embeddenator/PROFILING_TOOLKIT.md) and [embeddenator/PROFILING_QUICK_START.md](embeddenator/PROFILING_QUICK_START.md)

---

### Update All Repos (fetch + pull)
Run the new helper script from workspace root:

```bash
cd /home/kang/Documents/projects/embdntr
chmod +x update_all.sh
./update_all.sh
```

If a repo reports a conflict or timeout, resolve in that repo and rerun.
