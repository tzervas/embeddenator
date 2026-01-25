# Git Branch Status Report

> **Analysis Date**: January 9, 2026  
> **Last Updated**: January 9, 2026 - Post Alpha Publishing

## ⚠️ Critical: Branch Divergence in Main Repository

The `main` and `dev` branches in the main embeddenator repository have **no common history**. They appear to have been reset/rewritten at different times:

```
origin/main (2 commits):
  1632c00 Add manual-only CI workflow
  bb7bdb2 split baseline: workspace restructure + subcrates + benchmarks + tests

origin/dev (~71 commits, starting from):
  572a6ab Initial commit
  ...
  ec702e1 Add specialized AI assistant models architecture
```

**Resolution Options**:
1. Force-push dev to main: `git push origin dev:main --force`
2. Keep main as-is and continue development on dev
3. Create a new reconciliation branch

The dev branch contains all Phase 2A/2B/3 work and is aligned with the v0.20.0-alpha.1 crates.io releases.

## Summary

| Repository | Main State | Dev State | Feature Branches | Dependabot | Action Needed |
|------------|-----------|-----------|------------------|------------|---------------|
| embeddenator | 2 commits | **+71 ahead** 🔴 | 5 branches | 0 | **CRITICAL** |
| embeddenator-vsa | 6 commits | +1 ahead | 0 | 0 | Minor |
| embeddenator-retrieval | 7 commits | +1 ahead | 0 | 0 | Minor |
| embeddenator-obs | 5 commits | +2 ahead | 1 branch 🟡 | 0 | Merge feature |
| embeddenator-io | 5 commits | +2 ahead | 1 branch 🟡 | 0 | Merge feature |
| embeddenator-interop | 6 commits | +1 ahead | 0 | 0 | Minor |
| embeddenator-fs | 7 commits | +1 ahead | 0 | 0 | Minor |
| embeddenator-cli | 2 commits | - | 0 | **6 branches** 🟠 | Review deps |
| embeddenator-contract-bench | 6 commits | - | 0 | 2 branches | Low priority |
| embeddenator-testkit | 4 commits | - | 0 | **4 branches** 🟠 | Version issue |
| embeddenator-workspace | 4 commits | - | 0 | 2 branches | Low priority |

---

## Detailed Branch Analysis

### 🔴 embeddenator (Main Repository) - CRITICAL

```
Branches:
  main                          [2 commits]
  origin/dev                    [71 commits ahead of main]
  origin/feat/benchmark-standards
  origin/feat/holographic-substrate
  origin/feat/hierarchical-unfolding-workflow
  origin/feat/unified-vsa-substrate
```

**Issue**: The `dev` branch contains **71 commits** of Phase 2A/2B work that has not been merged to main. This includes:
- CI standardization
- Component extraction
- Benchmark infrastructure
- Architecture decisions

**Recent commits on dev**:
```
3a647fd (HEAD -> dev, origin/dev) docs(ARCHITECTURE): refine Phase 2A scope and emphasize contract-first testing
a8e25cc ci(workflows): standardize GitHub Actions workflows
90ce6c6 ci(workflows): standardize CI workflow with separate build, lint, test jobs
6e8d64c docs(ADR-015): document Phase 2A component extraction sequence
56dbe3b chore(workspace): transition to Phase 2A component decomposition
```

**Recommendation**: 
```bash
cd embeddenator
git checkout main
git merge origin/dev
git push origin main
```

---

### 🟡 embeddenator-obs

```
Branches:
  main                          [5 commits]
  origin/dev                    [+2 ahead]
  origin/feat/extract-obs       [tagged v0.2.0]
```

**Issue**: Feature branch `feat/extract-obs` contains v0.2.0 release work.

**Recommendation**:
```bash
cd embeddenator-obs
git checkout main
git merge origin/feat/extract-obs
git push origin main
```

---

### 🟡 embeddenator-io

```
Branches:
  main                          [5 commits]
  origin/dev                    [+2 ahead]
  origin/feat/extract-io        [tagged v0.2.0]
```

**Issue**: Feature branch `feat/extract-io` contains v0.2.0 release work.

**Recommendation**:
```bash
cd embeddenator-io
git checkout main
git merge origin/feat/extract-io
git push origin main
```

---

### 🟠 embeddenator-cli (Dependabot)

```
Dependabot branches:
  dependabot/cargo/bincode-3.0
  dependabot/cargo/embeddenator-fs-v0.19.4
  dependabot/cargo/embeddenator-io-v0.19.4
  dependabot/cargo/embeddenator-retrieval-v0.19.4
  dependabot/cargo/embeddenator-vsa-v0.19.4
  dependabot/cargo/serde-1.0.219
```

**Issues**:
1. `bincode-3.0` - Major version bump, needs testing
2. `embeddenator-*-v0.19.4` - These PRs are trying to bump git dependencies but versions don't align

**Recommendation**:
- Close the embeddenator-* bumps (stale, wrong approach)
- Test bincode-3.0 separately
- Update serde if tests pass

---

### 🟠 embeddenator-testkit (Dependabot - Version Conflict)

```
Dependabot branches:
  dependabot/cargo/embeddenator-v0.20.0
  dependabot/cargo/embeddenator-v1.0.0    <- PROBLEMATIC
  dependabot/cargo/rand-0.9.0
  dependabot/cargo/rand-0.9.1
```

**Issue**: Dependabot is trying to bump to `embeddenator v1.0.0` which doesn't exist (main is at v0.19.4).

**Recommendation**:
- Close the v1.0.0 bump (invalid)
- Review v0.20.0 bump if/when that version is tagged
- Update rand separately

---

### embeddenator-contract-bench (Dependabot)

```
Dependabot branches:
  dependabot/cargo/rand_chacha-0.9.0
  dependabot/cargo/sha2-0.11.0
```

**Recommendation**: Low priority. Review when doing a general dependency update pass.

---

### embeddenator-workspace (Dependabot)

```
Dependabot branches:
  dependabot/cargo/clap-4.5.28
  dependabot/cargo/walkdir-2.6.0
```

**Recommendation**: Low priority. Standard dependency updates.

---

## Feature Branches Requiring Review

### embeddenator - Active Development

| Branch | Status | Description |
|--------|--------|-------------|
| `feat/hierarchical-unfolding-workflow` | Active | Hierarchical unfolding implementation |
| `feat/benchmark-standards` | Active | Benchmark infrastructure |
| `feat/holographic-substrate` | Experimental | VSA substrate work |
| `feat/unified-vsa-substrate` | Unknown | May be merged into dev |

---

## Merge Priority Queue

### Immediate (P0)

1. **embeddenator**: `dev` → `main`
   - 71 commits of critical work
   - Blocks all other version alignment

### High (P1)

2. **embeddenator-obs**: `feat/extract-obs` → `main`
3. **embeddenator-io**: `feat/extract-io` → `main`

### Medium (P2)

4. **embeddenator-cli**: Close stale dependabot PRs
5. **embeddenator-testkit**: Close invalid v1.0.0 PR

### Low (P3)

6. Review remaining dependabot PRs across all repos
7. Review feature branches in main repo

---

## Git Commands Quick Reference

### Fetch all remotes (run in each repo)
```bash
for dir in embeddenator*; do
  (cd "$dir" && git fetch --all)
done
```

### Check divergence
```bash
git log --oneline main..origin/dev
```

### Merge dev to main
```bash
git checkout main
git merge origin/dev
git push origin main
```

### Delete merged branch
```bash
git push origin --delete <branch-name>
```

### List all remote branches
```bash
git branch -r
```
