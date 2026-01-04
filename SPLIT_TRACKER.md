# Embeddenator Component Split Tracker

**Purpose:** Track progress across all phases of monorepo decomposition  
**Status:** Phase 2B In Progress (1/4 components extracted)  
**Last Updated:** 2026-01-04

---

## Overview

The Embeddenator project is undergoing systematic decomposition from a monolithic repository into modular component libraries. This enables:
- Independent versioning and releases
- Faster compilation times
- Better separation of concerns
- Easier maintenance and testing
- Clearer dependency boundaries

**Phases:**
1. ✅ **Phase 1** - Repository setup, ADRs, CI foundation
2. ✅ **Phase 2A** - Core component extraction (100% complete)
3. ⏳ **Phase 2B** - MCP server & CLI extraction (25% complete)
4. ⏹️ **Phase 3** - Final integration & cleanup

---

## Phase 1: Foundation ✅ COMPLETE

**Timeline:** Dec 2025  
**Status:** Complete

| Task | Status | Notes |
|------|--------|-------|
| Create sister repositories (14 repos) | ✅ | All at ~/Documents/projects/embeddenator/ |
| Document architecture (ADRs) | ✅ | ADR-001 through ADR-017 |
| Set up CI/CD | ✅ | Self-hosted runners |
| Stabilize sister projects | ✅ | All 14 repos build successfully |
| Fix embeddenator-contract-bench | ✅ | Corrected v0.20.0 → path dep |

**Deliverables:**
- 14 sister repositories initialized
- ADR documentation framework
- CI/CD infrastructure
- All repos in buildable state

---

## Phase 2A: Core Component Extraction ⏳ IN PROGRESS

**Timeline:** Jan 1-28, 2026 (4 weeks)  
**Status:** 6/6 components complete (100%) ✅ **COMPLETE**  
**Epic Issue:** [#24](https://github.com/tzervas/embeddenator/issues/24)

### Progress Table

| # | Component | Issue | Depends On | LOC | Status | Release | Notes |
|---|-----------|-------|------------|-----|--------|---------|-------|
| 1 | embeddenator-vsa | [#18](https://github.com/tzervas/embeddenator/issues/18) | - | ~4,252 | ✅ **DONE** | v0.2.0 | Security audit complete, all tests pass |
| 2 | embeddenator-retrieval | [#19](https://github.com/tzervas/embeddenator/issues/19) | vsa | ~578 | ✅ **DONE** | v0.2.0 | No unsafe code, signature.rs deferred |
| 3 | embeddenator-fs | [#20](https://github.com/tzervas/embeddenator/issues/20) | vsa, retrieval | ~3,675 | ✅ **DONE** | v0.2.0 | 2 safe unsafe blocks (POSIX) |
| 4 | embeddenator-interop | [#21](https://github.com/tzervas/embeddenator/issues/21) | vsa, fs | ~159 | ✅ **DONE** | v0.2.0 | No unsafe code, trait-based abstractions |
| 5 | embeddenator-io | [#22](https://github.com/tzervas/embeddenator/issues/22) | - | ~166 | ✅ **DONE** | v0.2.0 | No unsafe code, 11 tests, compression codecs |
| 6 | embeddenator-obs | [#23](https://github.com/tzervas/embeddenator/issues/23) | - | ~953 | ✅ **DONE** | v0.2.0 | 2 safe unsafe blocks (TSC), metrics/logging/timing |

**Total LOC to extract:** ~9,783  
**Extracted:** ~9,783 (100% - Phase 2A Complete!)

### Weekly Schedule

**Week 1 (Jan 1-7):**
- ✅ Security audit (SIMD cosine)
- ✅ Extract embeddenator-vsa
- ✅ Tag v0.2.0, close #18
- ✅ Security audit (retrieval)
- ✅ Extract embeddenator-retrieval
- ✅ Tag v0.2.0, close #19

**Week 2 (Jan 7-14):**
- ✅ Extract embeddenator-fs
- ✅ Tag v0.2.0, close #20
- ✅ Extract embeddenator-interop
- ✅ Tag v0.2.0, close #21
- ✅ Extract embeddenator-io
- ✅ Tag v0.2.0, close #22
- → Extract embeddenator-obs

**Week 3 (Jan 14-21):**
- → Extract embeddenator-interop
- → Extract embeddenator-io (parallel)
- → Extract embeddenator-obs (parallel)

**Week 4 (Jan 21-28):**
- → Integration testing
- → Performance benchmarking
- → Documentation updates
- → Phase 2A complete

### Critical Path

```
vsa (✅) → retrieval → fs → interop
         ↘ io (independent)
         ↘ obs (independent)
```

**Bottlenecks:**
- retrieval blocks fs
- fs blocks interop
- io and obs can proceed in parallel

---

## Phase 2B: MCP Servers & CLI ⏳ IN PROGRESS

**Timeline:** Jan 2026  
**Status:** 1/4 components complete (25%)  
**Epic Issue:** TBD

### Planned Extractions

| Component | Purpose | Dependencies | LOC Est. | Status | Version |
|-----------|---------|--------------|----------|--------|---------|
| embeddenator-cli | CLI interface | All Phase 2A | ~1,174 | ✅ **DONE** | v0.2.0 |
| embeddenator-context-mcp | Context provider | vsa, obs | ~300 | ⏹️ Planned | - |
| embeddenator-security-mcp | Security auditing | vsa, obs | ~200 | ⏹️ Planned | - |
| embeddenator-screen-mcp | Screen capture | obs | ~400 | ⏹️ Planned | - |

**embeddenator-cli Complete ✅**
- 1,174 LOC extracted from src/cli.rs
- 7 main commands: Ingest, Extract, Query, QueryText, BundleHier, Mount, Update
- 4 update subcommands: Add, Remove, Modify, Compact
- Tagged v0.2.0, integrated into main repo
- Command implementations are stubs awaiting embrfs integration

**Prerequisites:**
- ✅ Phase 2A must complete
- ⏹️ MCP servers need stabilization first
- ⏹️ CLI requires full embrfs integration for command implementations

---

## Phase 3: Integration & Cleanup ⏹️ PLANNED

**Timeline:** Mar 2026 (estimated)  
**Status:** Not started

### Tasks

- [ ] Merge all feat/extract-* branches
- [ ] Update all path deps to published versions
- [ ] Performance regression testing
- [ ] Documentation overhaul
- [ ] Remove obsolete code from monorepo
- [ ] Publish all components to crates.io
- [ ] Update README and CHANGELOG
- [ ] Archive handoff documents
- [ ] Close all phase issues

---

## Metrics

### Component Extraction Progress

```
Phase 2A: [████████████████] 100% (6/6) ✅
Phase 2B: [████░░░░░░░░░░░░] 25% (1/4)
Phase 3: [░░░░░░░░░░░░░░░░] 0% (0/1)

Overall: [███████████░░░░░] 63.6% (7/11)
```

### LOC Migration

- **Total codebase:** ~17,000 LOC (estimated)
- **Phase 2A extracted:** ~9,783 LOC (100% of Phase 2A)
- **Phase 2B extracted:** ~1,174 LOC (56.6% of Phase 2B target)
- **Total extracted:** ~10,957 LOC (64.5% of total)

### Build Status

| Repository | Status | Tests | Issues |
|------------|--------|-------|--------|
| embeddenator (monorepo) | ✅ Building | ✅ 19/19 pass | 0 |
| embeddenator-vsa | ✅ Building | ✅ Passing | 0 |
| embeddenator-retrieval | ✅ Building | ✅ 18/18 pass | 0 |
| embeddenator-fs | ✅ Bui✅ Building | ✅ Passing | 0 |
| embeddenator-io | ✅ Building | ✅ 11/11 pass | 0 |
| embeddenator-obs | ✅ Building | ✅ Passing | 0 |
| embeddenator-cli | ✅ Building | ⏹️ Stubs only| 0 |
| embeddenator-obs | 📦 Skeleton | - | 0 |

---

## Dependencies

### Phase 2A Dependency Graph

```
Level 0 (foundation):
  └─ vsa ✅

Level 1 (depends on vsa):
  └─ retrieval ✅

Level 2 (depends on retrieval):
  └─ fs ✅

Level 3 (depends on fs):
  └─ interop

Independent:
  ├─ io
  └─ obs
```

### External Dependencies

All components depend on:
- `rand = "0.8"`
- `rayon = "1.10"`
- `thiserror = "2.0"`
- Platform-specific: `simd-json` (AVX2), `arm-neon` (ARM64)

---

## Risk Assessment

### Current Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Import path conflicts | Medium | Systematic sed-based updates |
| SIMD platform divergence | Medium | Conditional compilation, thorough testing |
| Test coverage gaps | Low | Migrate tests with modules |
| Performance regression | Low | Benchmark at each extraction |
| Dependency cycles | Low | Follow strict extraction order |

### Completed Mitigations

- ✅ Security audit process established
- ✅ Feature branching strategy validated
- ✅ Path dependency workflow proven
- ✅ Import update patterns documented

---

## References

- [ADR-017: Phase 2A Component Extraction Strategy](docs/adr/ADR-017-phase2a-component-extraction.md)
- [Phase 2A Handoff Document](docs/handoff/PHASE2A_SESSION_2026_01_04.md)
- [Security Audit: SIMD Cosine](docs/SECURITY_AUDIT_SIMD_COSINE.md)
- [Crate Structure Documentation](docs/CRATE_STRUCTURE_AND_CONCURRENCY.md)
- [Local Development Guide](docs/LOCAL_DEVELOPMENT.md)

**GitHub Project:** https://github.com/tzervas/embeddenator  
**Sister Repos:** ~/Documents/projects/embeddenator/

---

## Update History

| Date | Phase | Milestone | Updated By |
|------|-------|-----------|------------|
| 2026-01-04 | 2A | embeddenator-fs complete (v0.2.0) | Workflow Orchestrator |
| 2026-01-04 | 2A | embeddenator-retrieval complete (v0.2.0) | Workflow Orchestrator |
| 2026-01-04 | 2A | embeddenator-vsa complete (v0.2.0) | Workflow Orchestrator |
| 2026-01-03 | 2A | Security audit, ADR-017 created | Workflow Orchestrator |
| 2025-12-31 | 1 | Sister projects stabilized | System |

---

**Next Update:** After embeddenator-interop extraction (Issue #21)
