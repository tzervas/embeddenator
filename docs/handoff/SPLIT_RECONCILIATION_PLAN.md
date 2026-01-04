# Embeddenator Project Split Reconciliation Plan

**Date:** January 4, 2026  
**Purpose:** Verify and document the monorepo → sister project split  
**Status:** PLANNING (Pending Approval)

---

## 🚨 KEY FINDINGS (Requires Decision)

### ✅ Phase 2A Components: ALL VERIFIED
All 6 Phase 2A components properly extracted and synced:
- embeddenator-vsa (v0.2.0) ✅
- embeddenator-retrieval (v0.2.0) ✅
- embeddenator-fs (v0.2.0) ✅
- embeddenator-interop (v0.2.0) ✅
- embeddenator-io (v0.2.0) ✅
- embeddenator-obs (v0.2.0) ✅

### ⚠️ Phase 2B CLI: CODE DIVERGENCE DETECTED

| Location | LOC | Structure | Status |
|----------|-----|-----------|--------|
| `crates/embeddenator-cli/` | 1,323 | Modular (commands/, utils/) | More complete |
| `sister embeddenator-cli/` | 715 | Flat (commands.rs, config.rs) | Different impl |

**DECISION REQUIRED:** Which CLI implementation is canonical?
- **Option A (Recommended):** Use crates/ version → copy to sister, push to GitHub
- **Option B:** Use sister version → different implementation
- **Option C:** Merge both implementations

### ✅ Independent Projects: ALL VERIFIED
All MCP servers and tool crates have remotes and are synced.

### 📊 Summary

| Category | Count | Status |
|----------|-------|--------|
| Phase 2A Extractions | 6/6 | ✅ All verified |
| Phase 2B Extractions | 0/1 | ⚠️ CLI needs resolution |
| Independent Projects | 8/8 | ✅ All synced |
| **Total Sister Repos** | 15 | 14 verified, 1 needs fix |

---

## Executive Summary

This plan systematically audits the Embeddenator project split to ensure:
1. All code properly extracted from monorepo to sister repos
2. Documentation tracks exact commit provenance
3. No erroneous duplicates exist in `crates/` that should be in sister repos
4. All sister repos are properly synced with GitHub remotes

---

## 1. Project Inventory Discovery

### 1.1 Directory Structure

**Primary Locations:**
```
~/Documents/projects/
├── embeddenator/           # Sister project repos (PRIMARY)
│   ├── embeddenator/       # Duplicate? (nested)
│   ├── embeddenator-vsa/
│   ├── embeddenator-retrieval/
│   ├── embeddenator-fs/
│   ├── embeddenator-interop/
│   ├── embeddenator-io/
│   ├── embeddenator-obs/
│   ├── embeddenator-cli/
│   ├── embeddenator-agent-mcp/
│   ├── embeddenator-context-mcp/
│   ├── embeddenator-security-mcp/
│   ├── embeddenator-webpuppet/
│   ├── embeddenator-webpuppet-mcp/
│   ├── embeddenator-contract-bench/
│   ├── embeddenator-testkit/
│   └── embeddenator-workspace/
│
└── github/
    └── embeddenator/       # Main repo (ORCHESTRATION)
        └── crates/
            ├── embeddenator/           # Core library crate
            ├── embeddenator-cli/       # DUPLICATE? Should be sister repo
            └── embeddenator-screen-mcp/ # New MCP server?
```

### 1.2 Identified Issues

| Issue | Severity | Description |
|-------|----------|-------------|
| **CLI Code Divergence** | HIGH | `crates/embeddenator-cli/` has 1,323 LOC vs sister repo 715 LOC - DIFFERENT CODE! |
| **Nested embeddenator** | LOW | `~/Documents/projects/embeddenator/embeddenator/` is a CLONE of main GitHub repo (expected) |
| **CLI Remote Missing** | HIGH | Sister `embeddenator-cli` has no remote configured (master branch, not pushed) |
| **screen-mcp empty** | LOW | `crates/embeddenator-screen-mcp/` is empty placeholder |
| **CLI Structure Diff** | HIGH | crates/ has commands/ dir + utils/, sister has flat file structure |

### 1.3 Critical Finding: CLI Code Divergence

**crates/embeddenator-cli/src/** (1,323 LOC):
- `commands/` directory structure
- `utils/` directory
- `lib.rs` (19,348 bytes)
- `main.rs`

**sister embeddenator-cli/src/** (715 LOC):
- Flat structure: `commands.rs`, `config.rs`, `lib.rs`, `output.rs`
- `lib.rs` (8,174 bytes)
- NO main.rs

**Conclusion:** These are TWO DIFFERENT implementations! The sister repo appears to be an incomplete or different extraction.

---

## 2. Source → Target Mapping Table

### 2.1 Phase 2A Components (Extracted from Monorepo)

| Component | Source Branch | Extraction Commit | Target Repo | Target Version | Target Commit | Remote Sync | Status |
|-----------|---------------|-------------------|-------------|----------------|---------------|-------------|--------|
| **embeddenator-vsa** | feat/extract-vsa | b9ad4e6 | embeddenator-vsa/ | v0.2.0 | 3302bec | origin/dev ✅ | ✅ Verified |
| **embeddenator-retrieval** | feat/extract-retrieval | dd5c087 | embeddenator-retrieval/ | v0.2.0 | 52c4dd3 | origin/dev ✅ | ✅ Verified |
| **embeddenator-fs** | feat/extract-fs | faf86b1 | embeddenator-fs/ | v0.2.0 | c6a9cb2 | origin/dev ✅ | ✅ Verified |
| **embeddenator-interop** | feat/extract-interop | fe85433 | embeddenator-interop/ | v0.2.0 | a0bda4c | origin/dev ✅ | ✅ Verified |
| **embeddenator-io** | feat/extract-io | e7417ce | embeddenator-io/ | v0.2.0 | 907180a | origin/dev ✅ | ✅ Verified |
| **embeddenator-obs** | feat/extract-obs | d5f077e | embeddenator-obs/ | v0.2.0 | b292268 | origin/dev ✅ | ✅ Verified |

### 2.2 Phase 2B Components (CLI)

| Component | Source Branch | Source Commit | Target Repo | Target Version | Status | Issue |
|-----------|---------------|---------------|-------------|----------------|--------|-------|
| **embeddenator-cli** | dev | dbb630c | embeddenator-cli/ | v0.2.0 | ⚠️ DIVERGED | Code differs: 1,323 vs 715 LOC |

**CLI Divergence Details:**
- **crates/embeddenator-cli** (1,323 LOC): Full extraction with modular structure
- **sister embeddenator-cli** (715 LOC): Different implementation, incomplete

**Resolution Required:** Determine canonical version and sync

### 2.3 Independent Sister Projects (NOT Extractions)

| Component | Location | GitHub Remote | Version | Status |
|-----------|----------|---------------|---------|--------|
| **embeddenator-agent-mcp** | ~/Documents/projects/embeddenator/embeddenator-agent-mcp/ | tzervas/embeddenator-agent-mcp | v0.1.0-alpha.1 | ✅ Independent |
| **embeddenator-context-mcp** | ~/Documents/projects/embeddenator/embeddenator-context-mcp/ | tzervas/embeddenator-context-mcp | v0.1.0-alpha.1 | ✅ Independent |
| **embeddenator-security-mcp** | ~/Documents/projects/embeddenator/embeddenator-security-mcp/ | tzervas/embeddenator-security-mcp | v0.1.0-alpha.1 | ✅ Independent |
| **embeddenator-webpuppet** | ~/Documents/projects/embeddenator/embeddenator-webpuppet/ | tzervas/embeddenator-webpuppet | v0.1.0-alpha.2 | ✅ Independent |
| **embeddenator-webpuppet-mcp** | ~/Documents/projects/embeddenator/embeddenator-webpuppet-mcp/ | tzervas/embeddenator-webpuppet-mcp | v0.1.0-alpha.2 | ✅ Independent |
| **embeddenator-contract-bench** | ~/Documents/projects/embeddenator/embeddenator-contract-bench/ | tzervas/embeddenator-contract-bench | v0.2.1 | ✅ Tool crate |
| **embeddenator-testkit** | ~/Documents/projects/embeddenator/embeddenator-testkit/ | tzervas/embeddenator-testkit | v0.1.1 | ✅ Tool crate |
| **embeddenator-workspace** | ~/Documents/projects/embeddenator/embeddenator-workspace/ | tzervas/embeddenator-workspace | v0.1.0-alpha.1 | ✅ Tool crate |

### 2.4 Clarified Items

| Item | Location | Finding | Action |
|------|----------|---------|--------|
| **embeddenator (nested)** | ~/Documents/projects/embeddenator/embeddenator/ | Clone of main GitHub repo (main branch, 1632c00) | ✅ Expected - local working copy |
| **embeddenator-screen-mcp** | ~/Documents/projects/github/embeddenator/crates/ | Empty directory (placeholder) | Low priority - create when needed |
| **crates/embeddenator** | ~/Documents/projects/github/embeddenator/crates/embeddenator/ | Core library v0.20.0 | ✅ Expected - workspace member |

---

## 3. Reconciliation Tasks

### ⚠️ CRITICAL: CLI Code Divergence Resolution

**Problem:** Two different CLI implementations exist:
1. `crates/embeddenator-cli/` - 1,323 LOC, modular structure (commands/, utils/)
2. `embeddenator-cli/` (sister) - 715 LOC, flat structure, different code

**Decision Required:**
- [ ] **Option A:** Use crates/ version (more complete) → Copy to sister repo, push
- [ ] **Option B:** Use sister version (already committed) → Update crates/ to match
- [ ] **Option C:** Merge both → Combine features from both implementations

**Recommended: Option A** - The crates/ version has more code and modular structure.

### Phase R1: Audit & Document (30 min)

| # | Task | Priority | Est. Time | Status |
|---|------|----------|-----------|--------|
| R1.1 | Get exact commits from each feat/extract-* branch | HIGH | 15 min | ✅ DONE |
| R1.2 | Compare code between crates/ and sister repos | HIGH | 30 min | ✅ DONE (CLI diverged) |
| R1.3 | Verify all sister repos have correct GitHub remotes | HIGH | 15 min | ✅ DONE |
| R1.4 | Document the nested embeddenator/embeddenator/ purpose | MEDIUM | 10 min | ✅ DONE (clone of main) |
| R1.5 | Check embeddenator-screen-mcp status | MEDIUM | 10 min | ✅ DONE (empty placeholder) |
| R1.6 | Verify all tags exist on remotes | HIGH | 15 min | ⏳ Pending |

### Phase R2: Fix Discrepancies (45 min)

| # | Task | Priority | Est. Time |
|---|------|----------|-----------|
| R2.0 | **CRITICAL: Resolve CLI divergence** (see decision above) | CRITICAL | 20 min |
| R2.1 | Create GitHub repo for embeddenator-cli if not exists | HIGH | 5 min |
| R2.2 | Add remote to embeddenator-cli sister repo | HIGH | 5 min |
| R2.3 | Push embeddenator-cli to GitHub with correct code | HIGH | 5 min |
| R2.4 | Push any unpushed tags to GitHub (all repos) | HIGH | 10 min |
| R2.5 | Remove or mark deprecated: crates/embeddenator-cli after sync | MEDIUM | 5 min |

### Phase R3: Documentation & Tracking (30 min)

| # | Task | Priority | Est. Time |
|---|------|----------|-----------|
| R3.1 | Create REPO_PROVENANCE.md with full mapping | HIGH | 20 min |
| R3.2 | Update SPLIT_TRACKER.md with verified data | HIGH | 10 min |
| R3.3 | Commit provenance doc to all repos | MEDIUM | 15 min |

### Phase R4: Validation (15-30 min)

| # | Task | Priority | Est. Time |
|---|------|----------|-----------|
| R4.1 | Build all sister repos independently | HIGH | 10 min |
| R4.2 | Run tests on each component | HIGH | 15 min |
| R4.3 | Verify cross-repo dependencies resolve | HIGH | 10 min |

---

## 4. Detailed Execution Plan

### Task R1.1: Get Exact Commits from Extraction Branches

```bash
# For each feat/extract-* branch, get the extraction commit
cd ~/Documents/projects/github/embeddenator
for branch in feat/extract-vsa feat/extract-retrieval feat/extract-fs \
               feat/extract-interop feat/extract-io feat/extract-obs; do
    echo "=== $branch ==="
    git log "$branch" --oneline -1
done
```

### Task R1.2: Compare Code Between Locations

For each component, compare:
1. `crates/<component>/src/` vs `~/Documents/projects/embeddenator/<component>/src/`
2. Check for differences in Cargo.toml
3. Identify which location has the "canonical" code

```bash
# Example for CLI
diff -rq ~/Documents/projects/github/embeddenator/crates/embeddenator-cli/src \
         ~/Documents/projects/embeddenator/embeddenator-cli/src
```

### Task R1.3: Verify GitHub Remotes

```bash
# Check all remotes
for repo in ~/Documents/projects/embeddenator/embeddenator-*/; do
    echo "=== $(basename $repo) ==="
    cd "$repo"
    git remote -v
done
```

### Task R2.1: Fix embeddenator-cli Remote

```bash
cd ~/Documents/projects/embeddenator/embeddenator-cli
git remote add origin https://github.com/tzervas/embeddenator-cli.git
git push -u origin master
git push --tags
```

### Task R2.2: Clean Up crates/embeddenator-cli

Options:
1. **Delete** - Remove from crates/ entirely
2. **Redirect** - Replace with symbolic link or README pointing to sister repo
3. **Keep as Workspace Member** - If needed for workspace builds

Recommended: **Delete** after verifying sister repo is complete

### Task R3.1: Create REPO_PROVENANCE.md

Create in each sister repo:
```markdown
# Repository Provenance

## Origin
- **Source Repository:** https://github.com/tzervas/embeddenator
- **Extraction Branch:** feat/extract-<component>
- **Extraction Commit:** <sha>
- **Extraction Date:** <date>
- **Extracted By:** <agent/person>

## Initial Version
- **Version at Extraction:** v0.2.0
- **LOC at Extraction:** <count>
- **Files Extracted:** <list>

## Changelog
See CHANGELOG.md for version history post-extraction.
```

---

## 5. Priority Order for Execution

### HIGH Priority (Execute First)

1. **R1.1** - Document extraction commits
2. **R1.2** - Compare crates/ vs sister repos for CLI
3. **R2.1** - Fix embeddenator-cli remote
4. **R2.2** - Remove crates/embeddenator-cli duplicate

### MEDIUM Priority (After HIGH Complete)

5. **R1.4** - Document nested embeddenator/embeddenator
6. **R1.5** - Check embeddenator-screen-mcp status
7. **R2.3** - Create screen-mcp sister repo if needed

### LOW Priority (Cleanup)

8. **R3.1** - Create REPO_PROVENANCE.md
9. **R3.2** - Update SPLIT_TRACKER.md
10. **R4.x** - Full validation

---

## 6. Expected Outcome

After reconciliation:

### Directory Structure (Clean)
```
~/Documents/projects/
├── embeddenator/                    # Sister project space
│   ├── embeddenator-vsa/           ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-retrieval/     ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-fs/            ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-interop/       ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-io/            ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-obs/           ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-cli/           ✅ v0.2.0, synced with GitHub
│   ├── embeddenator-agent-mcp/     ✅ Independent, synced
│   ├── embeddenator-context-mcp/   ✅ Independent, synced
│   ├── embeddenator-security-mcp/  ✅ Independent, synced
│   ├── embeddenator-webpuppet/     ✅ Independent, synced
│   ├── embeddenator-webpuppet-mcp/ ✅ Independent, synced
│   ├── embeddenator-contract-bench/ ✅ Tool, synced
│   ├── embeddenator-testkit/       ✅ Tool, synced
│   ├── embeddenator-workspace/     ✅ Tool, synced
│   └── embeddenator-screen-mcp/    ✅ New repo (if created)
│
└── github/
    └── embeddenator/               # Main orchestration repo
        └── crates/
            └── embeddenator/       # Core library only (no duplicates)
```

### Documentation Deliverables

1. **REPO_PROVENANCE.md** - In each sister repo
2. **SPLIT_TRACKER.md** - Updated with verified commits
3. **SISTER_PROJECT_STABILITY_REPORT.md** - Updated
4. **REPO_MAPPING_AUDIT.md** - Full audit trail

---

## 7. Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Code drift between crates/ and sister | HIGH | MEDIUM | Compare before deleting crates/ versions |
| Missing commits not in sister repos | MEDIUM | HIGH | Careful diff analysis, cherry-pick if needed |
| Breaking dependencies | MEDIUM | HIGH | Test builds after changes |
| Lost work in crates/ | LOW | HIGH | Backup before deletion |

---

## 8. Approval Checklist

Before proceeding to execution, confirm:

- [ ] Plan reviewed and understood
- [ ] Priority order agreed
- [ ] Risk mitigations acceptable
- [ ] Backup strategy confirmed
- [ ] Estimated time acceptable (~2-3 hours total)

---

## 9. Execution Commands (Ready to Run)

Once approved, we will execute in this order:

```bash
# Phase R1: Audit
# R1.1 - Get extraction commits
# R1.2 - Compare code
# R1.3 - Verify remotes

# Phase R2: Fix
# R2.1 - Add CLI remote
# R2.2 - Remove crates/embeddenator-cli

# Phase R3: Document
# R3.1 - Create provenance docs

# Phase R4: Validate
# R4.1-R4.3 - Full build and test
```

---

**Status:** AWAITING APPROVAL  
**Estimated Total Time:** 2-3 hours  
**Agent:** Workflow Orchestrator  
**Next Step:** User approval, then execute Phase R1

---

## Appendix: Quick Reference Commands

### Check All Repo Status
```bash
for repo in ~/Documents/projects/embeddenator/embeddenator-*/; do
    echo "=== $(basename $repo) ===" 
    cd "$repo"
    git status --short
    git log --oneline -1
done
```

### Push All to Remotes
```bash
for repo in ~/Documents/projects/embeddenator/embeddenator-*/; do
    cd "$repo"
    git push origin --all 2>&1 | head -3
    git push origin --tags 2>&1 | head -2
done
```

### Build All Sister Projects
```bash
for repo in ~/Documents/projects/embeddenator/embeddenator-*/; do
    echo "=== Building $(basename $repo) ===" 
    cd "$repo"
    cargo build 2>&1 | tail -3
done
```
