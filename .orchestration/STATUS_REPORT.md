# Embeddenator Ecosystem Status Report

> **Report Date**: January 9, 2026  
> **Analysis Scope**: 11 repositories  
> **Last Updated**: January 9, 2026 - Alpha Publishing Complete

---

## Executive Summary

The Embeddenator project is a **Rust-based holographic computing substrate** implementing Vector Symbolic Architecture (VSA) with sparse ternary vectors. The project is mid-way through **Phase 2A component decomposition** — transitioning from a monolithic architecture to modular crates.

### Key Findings

| Category | Status | Details |
|----------|--------|---------|
| **Critical Blocker** | 🔴 | Main repo `dev` branch is 71 commits ahead of `main` |
| **Component Extraction** | 🟡 | ~60% complete, some modules disabled |
| **CI/CD** | 🟡 | Components have CI, main repo is manual-only |
| **Documentation** | 🟡 | Architecture docs exist, some gaps |
| **Test Coverage** | 🟠 | Contract benchmarks exist, testkit minimal |
| **Publishing** | ⚪ | No crates published to crates.io yet |

---

## Repository Health Matrix

| Repository | Build | Docs | Tests | CI | Version |
|------------|:-----:|:----:|:-----:|:--:|---------|
| embeddenator | ✅ | ✅ | ✅ | 🟡 | v0.19.4 |
| embeddenator-vsa | ✅ | ✅ | ✅ | ✅ | v0.1.0 |
| embeddenator-retrieval | ✅ | ✅ | ⚠️ | ✅ | v0.2.0 |
| embeddenator-fs | ✅ | ✅ | ✅ | ✅ | v0.2.0 |
| embeddenator-obs | ✅ | ✅ | ✅ | ✅ | v0.1.1 |
| embeddenator-io | ✅ | ✅ | ✅ | ✅ | v0.1.1 |
| embeddenator-interop | ✅ | ⚠️ | ⚠️ | ✅ | v0.2.0 |
| embeddenator-cli | ✅ | ✅ | ⚠️ | ✅ | v0.1.0 |
| embeddenator-contract-bench | ✅ | ✅ | ✅ | ⚠️ | v0.2.1 |
| embeddenator-testkit | ⚠️ | ⚠️ | ⚠️ | ⚠️ | v0.1.1 |
| embeddenator-workspace | ⚠️ | ⚠️ | ⚠️ | ⚠️ | v0.1.0-alpha.1 |

*✅ = Good, 🟡 = Needs attention, ⚠️ = Incomplete/Issues, ❌ = Broken*

---

## Architecture Overview

```
                    ┌─────────────────┐
                    │  embeddenator   │  ← Main repo (being decomposed)
                    │    (v0.19.4)    │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐  ┌──────────────────┐  ┌─────────────┐
│    Core       │  │   Infrastructure │  │   User      │
├───────────────┤  ├──────────────────┤  ├─────────────┤
│ • vsa         │  │ • obs            │  │ • cli       │
│ • retrieval   │  │ • io             │  │ • workspace │
│ • fs          │  │ • interop        │  └─────────────┘
└───────────────┘  └──────────────────┘

┌─────────────────────────────────────────────────────┐
│                  Dev/Test                           │
├─────────────────────────────────────────────────────┤
│ • contract-bench    • testkit                       │
└─────────────────────────────────────────────────────┘
```

---

## Code Metrics

| Metric | Value |
|--------|-------|
| **Total LOC** | ~32,000+ |
| **Languages** | Rust (100%) |
| **Repositories** | 11 |
| **Crate Dependencies** | 6 internal, ~30 external |

### Lines of Code by Component

```
embeddenator          ████████████████████ 18,013
embeddenator-vsa      ████████             4,311
embeddenator-fs       ███████              3,698
embeddenator-contract █████                2,186
embeddenator-retrieval███                  1,425
embeddenator-cli      ███                  1,323
embeddenator-obs      ██                     969
embeddenator-interop  █                      328
embeddenator-io       █                      184
embeddenator-workspace█                       63
embeddenator-testkit  ▏                        3
```

---

## Git Status Summary

### Branch Divergence

| Repository | Main vs Dev | Feature Branches | Dependabot |
|------------|-------------|------------------|------------|
| embeddenator | **+71** 🔴 | 5 | 0 |
| embeddenator-vsa | +1 | 0 | 0 |
| embeddenator-retrieval | +1 | 0 | 0 |
| embeddenator-obs | +2 | 1 🟡 | 0 |
| embeddenator-io | +2 | 1 🟡 | 0 |
| embeddenator-interop | +1 | 0 | 0 |
| embeddenator-fs | +1 | 0 | 0 |
| embeddenator-cli | - | 0 | 6 🟠 |
| embeddenator-contract-bench | - | 0 | 2 |
| embeddenator-testkit | - | 0 | 4 🟠 |
| embeddenator-workspace | - | 0 | 2 |

### Total PRs/Branches Needing Action

| Priority | Count | Description |
|----------|-------|-------------|
| P0 | 1 | Critical merge (embeddenator dev→main) |
| P1 | 2 | Feature branch merges (obs, io) |
| P2 | ~6 | Invalid/stale dependabot PRs to close |
| P3 | ~8 | Valid dependabot PRs to review |

---

## Known Issues

### Critical
1. **dev branch divergence** — 71 commits of Phase 2A work not on main

### High
2. **Feature branches not merged** — obs and io have v0.2.0 work in branches
3. **Version mismatch** — testkit references v0.20.0, main is v0.19.4

### Medium
4. **Code duplication** — SignatureStore in 3 locations
5. **Disabled modules** — retrieval/signature waiting on interop
6. **Manual CI** — main repo needs automated triggers

### Low
7. **Deprecation warnings** — `from_data()` → `encode_data()`
8. **Minimal implementations** — testkit (3 LOC), workspace (63 LOC)
9. **Stale dependabot PRs** — 14+ across repos

---

## Immediate Actions Required

### This Week

1. **Merge embeddenator dev → main**
   ```bash
   cd embeddenator && git merge origin/dev
   ```

2. **Merge feature branches in obs/io**
   ```bash
   cd embeddenator-obs && git merge origin/feat/extract-obs
   cd embeddenator-io && git merge origin/feat/extract-io
   ```

3. **Close invalid dependabot PRs**
   - embeddenator-testkit: `embeddenator-v1.0.0`
   - embeddenator-cli: stale `embeddenator-*-v0.19.4` PRs

### Next Steps

See [ACTION_PLAN.md](ACTION_PLAN.md) for detailed execution plan.

---

## Files in This Documentation Set

| File | Purpose |
|------|---------|
| [README.md](README.md) | This documentation index |
| [PROJECT_INDEX.md](PROJECT_INDEX.md) | Component catalog and details |
| [DEPENDENCY_GRAPH.md](DEPENDENCY_GRAPH.md) | Inter-component dependencies |
| [BRANCH_STATUS.md](BRANCH_STATUS.md) | Git branch analysis |
| [ACTION_PLAN.md](ACTION_PLAN.md) | Prioritized action items |
| [STATUS_REPORT.md](STATUS_REPORT.md) | This executive summary |

---

## Contact & Resources

- **Main Repository**: https://github.com/tzervas/embeddenator
- **Component Repos**: https://github.com/tzervas/embeddenator-*

---

*Report generated by workspace orchestration analysis.*
