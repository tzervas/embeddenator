# Embeddenator Action Plan

> **Created**: January 9, 2026  
> **Priority Levels**: P0 (Critical) → P3 (Low)

---

## Phase 1: Critical Merges (P0)

### 1.1 Merge embeddenator dev → main

**Status**: 🔴 CRITICAL  
**Impact**: Blocks all downstream work  
**Effort**: Low (merge only)

The `dev` branch is **71 commits ahead** of `main` and contains:
- Phase 2A/2B component extraction work
- CI standardization
- ADR-015 (component extraction sequence)
- Architecture documentation

**Steps**:
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator
git checkout main
git pull origin main
git merge origin/dev
# Resolve any conflicts
git push origin main
```

**Risks**:
- Potential merge conflicts (likely minimal - dev has been sole development branch)
- Should verify CI passes after merge

---

## Phase 2: Feature Branch Merges (P1)

### 2.1 Merge embeddenator-obs feature branch

**Status**: 🟡 Important  
**Reason**: Contains v0.2.0 tagged release

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-obs
git checkout main
git merge origin/feat/extract-obs
git push origin main
```

### 2.2 Merge embeddenator-io feature branch

**Status**: 🟡 Important  
**Reason**: Contains v0.2.0 tagged release

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-io
git checkout main
git merge origin/feat/extract-io
git push origin main
```

---

## Phase 3: Dependabot Cleanup (P2)

### 3.1 Close Invalid PRs

| Repository | Branch | Action | Reason |
|------------|--------|--------|--------|
| embeddenator-testkit | `dependabot/cargo/embeddenator-v1.0.0` | **Close** | v1.0.0 doesn't exist |
| embeddenator-cli | `dependabot/cargo/embeddenator-*-v0.19.4` (4 PRs) | **Close** | Stale, wrong approach |

### 3.2 Review Valid Dependabot PRs

| Repository | Branch | Action | Notes |
|------------|--------|--------|-------|
| embeddenator-cli | `bincode-3.0` | Review | Major version - needs testing |
| embeddenator-cli | `serde-1.0.219` | Merge | Safe minor update |
| embeddenator-contract-bench | `rand_chacha-0.9.0` | Review | Check compatibility |
| embeddenator-contract-bench | `sha2-0.11.0` | Review | Check compatibility |
| embeddenator-workspace | `clap-4.5.28` | Merge | Safe minor update |
| embeddenator-workspace | `walkdir-2.6.0` | Merge | Safe minor update |
| embeddenator-testkit | `rand-0.9.1` | Review | Check compatibility |

---

## Phase 4: Code Consolidation (P2)

### 4.1 Consolidate SignatureStore

**Current State**: Duplicated in 3 locations
- `embeddenator/src/retrieval/signature.rs`
- `embeddenator-retrieval/src/signature.rs` (disabled)
- `embeddenator-fs/src/embr/signature.rs`

**Action**:
1. Enable `signature` module in `embeddenator-retrieval`
2. Remove duplicate from `embeddenator-fs`
3. Update imports in `embeddenator-fs` to use `embeddenator_retrieval::SignatureStore`

### 4.2 Fix Deprecation Warnings

**Issue**: `from_data()` deprecated in favor of `encode_data()`

**Files affected**: `embeddenator-vsa/src/lib.rs`

**Action**: Update all 6 call sites to use `encode_data()`

---

## Phase 5: Enable CI (P2)

### 5.1 Update embeddenator CI triggers

**Current**: Manual only (`workflow_dispatch`)  
**Target**: Auto on push/PR

**File**: `embeddenator/.github/workflows/ci.yml`

```yaml
# Change from:
on:
  workflow_dispatch:

# To:
on:
  push:
    branches: [main, dev]
  pull_request:
    branches: [main]
  workflow_dispatch:
```

---

## Phase 6: Complete Implementations (P3)

### 6.1 Implement EmbrFS Update Methods

**Location**: `embeddenator-fs/src/embr/mod.rs` or related

**Methods needed**:
- `add_file()` - Add new file to holographic store
- `remove_file()` - Remove file from store
- `modify_file()` - Update existing file
- `compact()` - Optimize storage

**Blocked by**: These are needed for `embeddenator-cli` update commands

### 6.2 Enable embeddenator-retrieval Signature Module

**Blocked by**: `embeddenator-interop` extraction completion

**File**: `embeddenator-retrieval/src/lib.rs`

```rust
// Uncomment:
// pub mod signature;
```

### 6.3 Expand embeddenator-testkit

**Current state**: 3 lines of code (smoke test only)

**Needed**:
- Test fixture generators
- Mock holographic vectors
- Deterministic test data
- Property-based testing utilities

### 6.4 Complete embeddenator-workspace

**Current state**: 63 lines of code (minimal)

**Needed**:
- Full workspace management
- Multi-repo coordination
- Version alignment tools

---

## Phase 7: Documentation (P3)

### 7.1 Add Missing ADR-016

**Issue**: Referenced in code but file not found

**Action**: Create `embeddenator/docs/adr/ADR-016-*.md`

### 7.2 Component READMEs

Ensure each component repo has a complete README with:
- Purpose
- Installation
- Usage examples
- API documentation
- Contributing guide

---

## Phase 8: Version Alignment (P3)

### 8.1 Standardize Versions

**Target versions after merges**:

| Component | Current | Target |
|-----------|---------|--------|
| embeddenator | v0.19.4 | v0.20.0 |
| embeddenator-vsa | v0.1.0 | v0.2.0 |
| embeddenator-retrieval | v0.2.0 | ✓ |
| embeddenator-fs | v0.2.0 | ✓ |
| embeddenator-obs | v0.1.1 | v0.2.0 |
| embeddenator-io | v0.1.1 | v0.2.0 |
| embeddenator-interop | v0.2.0 | ✓ |
| embeddenator-cli | v0.1.0 | v0.2.0 |

### 8.2 Dependency Strategy Decision

**Options**:
1. **Path dependencies** - For development only
2. **Git dependencies** - For pre-publication
3. **crates.io** - For stable releases

**Recommendation**: 
- Use Cargo workspace for local development
- Publish to crates.io when stable

---

## Execution Checklist

### Week 1: Critical Merges
- [ ] Merge `embeddenator` dev → main
- [ ] Merge `embeddenator-obs` feature branch
- [ ] Merge `embeddenator-io` feature branch
- [ ] Close invalid dependabot PRs

### Week 2: Cleanup
- [ ] Review valid dependabot PRs
- [ ] Consolidate SignatureStore
- [ ] Fix deprecation warnings
- [ ] Enable CI on main repo

### Week 3: Implementation
- [ ] Implement EmbrFS update methods
- [ ] Enable retrieval signature module
- [ ] Add missing documentation

### Week 4: Polish
- [ ] Version alignment
- [ ] Expand testkit
- [ ] Complete workspace tooling
- [ ] Final documentation pass

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Merge conflicts in embeddenator | Low | Medium | Backup before merge |
| Breaking changes in deps | Medium | Medium | Test in isolation first |
| Missing interop blocks signature | High | Low | Document as known limitation |
| Version mismatch breaks builds | Medium | High | Coordinate version bumps |
