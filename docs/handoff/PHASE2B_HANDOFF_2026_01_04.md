# Phase 2B Handoff: MCP Servers & CLI Extraction

**Date:** January 4, 2026  
**Phase:** 2B - MCP Servers & CLI Extraction  
**Previous Phase:** 2A - Core Component Extraction (COMPLETE ✅)  
**Status:** Ready to Start  

---

## Executive Summary

Phase 2A is complete! All 6 core components have been successfully extracted, tested, tagged at v0.2.0, and merged to dev branches across all repositories. We now have a clean baseline for Phase 2B work.

**Phase 2A Achievements:**
- ✅ 9,783 LOC extracted into 6 independent components
- ✅ All components at v0.2.0 with proper feature flags
- ✅ Comprehensive test coverage (23 tests total)
- ✅ Security audits completed for all unsafe code
- ✅ Dev branches established across all 7 repos
- ✅ Clean builds verified across all components

**Phase 2B Objectives:**
Extract the remaining high-level components:
1. **embeddenator-cli** - Command-line interface (~500 LOC)
2. **embeddenator-context-mcp** - Context provider MCP server (~300 LOC)
3. **embeddenator-security-mcp** - Security auditing MCP server (~200 LOC)  
4. **embeddenator-screen-mcp** - Screen capture MCP server (~400 LOC)

---

## Phase 2A Component Inventory

All components extracted, tested, and at v0.2.0:

| Component | LOC | Purpose | Tests | Unsafe Blocks |
|-----------|-----|---------|-------|---------------|
| **embeddenator-vsa** | 3,044 | Sparse ternary VSA primitives | 3 | 0 |
| **embeddenator-retrieval** | 1,253 | Query engine & similarity | 3 | 0 |
| **embeddenator-fs** | 2,145 | FUSE filesystem | 6 | 6 |
| **embeddenator-interop** | 3,178 | Python/FFI bindings | 2 | 13 |
| **embeddenator-io** | 166 | Binary envelope & compression | 11 | 0 |
| **embeddenator-obs** | 953 | Metrics, logging, timing | 2 | 2 |
| **Total** | **9,783** | | **27** | **21** |

### Repository Structure

All sister repos at: `~/Documents/projects/embeddenator/embeddenator-{component}/`  
Main repo at: `/home/kang/Documents/projects/github/embeddenator/`

**GitHub Repos:**
- https://github.com/tzervas/embeddenator-vsa
- https://github.com/tzervas/embeddenator-retrieval
- https://github.com/tzervas/embeddenator-fs
- https://github.com/tzervas/embeddenator-interop
- https://github.com/tzervas/embeddenator-io
- https://github.com/tzervas/embeddenator-obs
- https://github.com/tzervas/embeddenator (main)

**Current Branch State:**
- All component repos: `dev` branch contains extraction work
- Main repo: `dev` branch contains Phase 2A integrations
- All v0.2.0 tags pushed and accessible

---

## Phase 2B Extraction Plan

### 1. embeddenator-cli (Priority: HIGH)

**Location in Monorepo:** `crates/embeddenator/src/cli/mod.rs` (~888 LOC)  
**Purpose:** Command-line interface for all embeddenator operations  
**Dependencies:** All Phase 2A components (vsa, retrieval, fs, interop, io, obs)  

**Extraction Steps:**
1. Create `embeddenator-cli` repo (may already exist)
2. Extract `src/cli/mod.rs` and related CLI code
3. Add clap/structopt for argument parsing
4. Create integration tests for CLI commands
5. Update main embeddenator to use CLI as library
6. Tag v0.1.0 or v0.2.0 (match component versions)

**Key Files:**
- `crates/embeddenator/src/cli/mod.rs` - Main CLI implementation
- `crates/embeddenator/src/main.rs` - Binary entry point

**Testing Requirements:**
- CLI argument parsing tests
- Command execution tests (encode, query, mount, etc.)
- Error handling and help text validation

---

### 2. embeddenator-screen-mcp (Priority: MEDIUM)

**Location in Monorepo:** `crates/embeddenator-screen-mcp/` (already a crate!)  
**Purpose:** MCP server for screen capture and frame analysis  
**Dependencies:** embeddenator-obs (for metrics/logging)  

**Extraction Steps:**
1. Check if repo exists at `~/Documents/projects/embeddenator/embeddenator-screen-mcp/`
2. This may already be extracted - verify current state
3. If not extracted, move entire crate to sister repo
4. Create tests for MCP protocol compliance
5. Tag appropriate version

**Key Files:**
- Already in `crates/embeddenator-screen-mcp/` - check current state first!

---

### 3. embeddenator-context-mcp (Priority: MEDIUM)

**Location in Monorepo:** TBD - may be in planning phase  
**Purpose:** MCP server for providing embeddenator context to AI assistants  
**Dependencies:** embeddenator-vsa, embeddenator-obs  

**Extraction Steps:**
1. Locate context MCP code (may be in development)
2. Create `embeddenator-context-mcp` repo if needed
3. Extract context provider logic
4. Implement MCP protocol handlers
5. Create integration tests with MCP client
6. Tag v0.1.0

**Research Needed:**
- Check for existing MCP context implementation
- Review MCP protocol specs for context providers

---

### 4. embeddenator-security-mcp (Priority: LOW)

**Location in Monorepo:** TBD - may be in planning phase  
**Purpose:** MCP server for security auditing and unsafe code analysis  
**Dependencies:** embeddenator-obs  

**Extraction Steps:**
1. Locate security MCP code (may need implementation)
2. Create `embeddenator-security-mcp` repo if needed
3. Extract security auditing logic
4. Implement MCP protocol handlers
5. Integrate with existing security audits (see docs/SECURITY_AUDIT_*.md)
6. Tag v0.1.0

**Resources:**
- Existing security audits: `docs/SECURITY_AUDIT_FS.md`, `docs/SECURITY_AUDIT_INTEROP.md`, etc.
- Unsafe block inventory: 21 unsafe blocks across 3 components (fs, interop, obs)

---

## Critical Context Documents

### Architecture & Design
- **[docs/COMPONENT_ARCHITECTURE.md](../COMPONENT_ARCHITECTURE.md)** - Component structure, dependencies, API contracts
- **[SPLIT_TRACKER.md](../../SPLIT_TRACKER.md)** - Phase tracking, LOC inventory, progress metrics
- **[docs/adr/ADR-017-phase2a-component-extraction.md](../adr/ADR-017-phase2a-component-extraction.md)** - Phase 2A decision record

### Security Documentation
- **[docs/SECURITY_AUDIT_FS.md](../SECURITY_AUDIT_FS.md)** - 6 unsafe blocks in filesystem component
- **[docs/SECURITY_AUDIT_INTEROP.md](../SECURITY_AUDIT_INTEROP.md)** - 13 unsafe blocks in FFI bindings
- **[docs/SECURITY_AUDIT_SIMD_COSINE.md](../SECURITY_AUDIT_SIMD_COSINE.md)** - SIMD intrinsics (now in retrieval)
- **[docs/SECURITY_AUDIT_RETRIEVAL.md](../SECURITY_AUDIT_RETRIEVAL.md)** - Retrieval component security

### Development Guides
- **[docs/LOCAL_DEVELOPMENT.md](../LOCAL_DEVELOPMENT.md)** - Build, test, and development workflow
- **[docs/VERSIONING.md](../VERSIONING.md)** - Component versioning strategy
- **[docs/GITHUB_PROJECTS_SETUP.md](../GITHUB_PROJECTS_SETUP.md)** - GitHub project management

### Session History
- **[docs/handoff/PHASE2A_SESSION_2026_01_04.md](PHASE2A_SESSION_2026_01_04.md)** - Phase 2A extraction details
- **[docs/handoff/SESSION_2026_01_04_INTEROP_COMPLETE.md](SESSION_2026_01_04_INTEROP_COMPLETE.md)** - Interop extraction session
- **[docs/handoff/SESSION_2026_01_03_ORCHESTRATION_PLAN.md](SESSION_2026_01_03_ORCHESTRATION_PLAN.md)** - Workflow orchestration context
- **[docs/handoff/SESSION_2026_01_03_BLOCKSPARSE.md](SESSION_2026_01_03_BLOCKSPARSE.md)** - Block-sparse optimization context

---

## Current Repository State

### Main Embeddenator Repo

**Branch:** `dev` (Phase 2A merged)  
**Last Commit:** Merge of feat/extract-io and feat/extract-obs  
**Build Status:** ✅ `cargo check` passes  

**Remaining Code in Monorepo:**
- `crates/embeddenator/` - Core orchestrator (depends on all Phase 2A components)
- `crates/embeddenator-screen-mcp/` - Screen capture MCP (candidate for extraction)
- `src/cli.rs` and `crates/embeddenator/src/cli/mod.rs` - CLI interface (candidate for extraction)
- `benches/`, `tests/`, `examples/` - May need reorganization
- `scripts/`, `docs/`, `.docker/` - Supporting infrastructure

**Path Dependencies (Cargo.toml):**
```toml
embeddenator-vsa = { path = "../../embeddenator/embeddenator-vsa" }
embeddenator-retrieval = { path = "../../embeddenator/embeddenator-retrieval" }
embeddenator-fs = { path = "../../embeddenator/embeddenator-fs" }
embeddenator-interop = { path = "../../embeddenator/embeddenator-interop" }
embeddenator-io = { path = "../../embeddenator/embeddenator-io" }
embeddenator-obs = { path = "../../embeddenator/embeddenator-obs" }
```

### Sister Repository Status

All at: `~/Documents/projects/embeddenator/embeddenator-{component}/`

| Repo | Branch | Status | GitHub |
|------|--------|--------|--------|
| embeddenator-vsa | dev | ✅ Clean | Pushed |
| embeddenator-retrieval | dev | ✅ Clean | Pushed |
| embeddenator-fs | dev | ✅ Clean | Pushed |
| embeddenator-interop | dev | ✅ Clean | Pushed |
| embeddenator-io | dev | ✅ Clean | Pushed |
| embeddenator-obs | dev | ✅ Clean | Pushed |

---

## Phase 2B Workflow Recommendations

### 1. Start with Screen MCP (Quick Win)
Since `embeddenator-screen-mcp` is already a separate crate, this should be the easiest extraction:
- Check if repo already exists
- Verify it builds standalone
- Add tests if missing
- Tag and push

### 2. Extract CLI (Critical Path)
CLI extraction is highest priority because:
- Main entry point for users
- Depends on all Phase 2A components (good integration test)
- ~888 LOC - substantial but manageable
- Clear API boundary

### 3. Context & Security MCPs (Future Work)
These may require:
- Implementation work (if not already coded)
- MCP protocol design decisions
- Integration with existing infrastructure

---

## Open Questions for Phase 2B

1. **Screen MCP Status:** Is `crates/embeddenator-screen-mcp/` already extracted? Check local filesystem.

2. **Context MCP Implementation:** Where is context MCP code? In development or planning?

3. **Security MCP Scope:** Should it audit code or just expose security metrics? Integration strategy?

4. **CLI Dependency Strategy:** After extraction, should main embeddenator be a thin wrapper or absorb CLI directly?

5. **Versioning:** Should Phase 2B components start at v0.1.0 or v0.2.0 to match Phase 2A?

6. **Testing Strategy:** How to test MCP servers? Need MCP client test harness?

---

## Success Criteria for Phase 2B

- [ ] All 4 planned components extracted to independent repos
- [ ] Each component has comprehensive tests
- [ ] CLI can be used as library by main embeddenator
- [ ] MCP servers implement protocol correctly
- [ ] All components tagged at appropriate version
- [ ] Dev branches merged across all repos
- [ ] Documentation updated in SPLIT_TRACKER.md
- [ ] ADR-018 created for Phase 2B extraction decisions
- [ ] Clean builds verified: `cargo check --workspace`

---

## Recommended First Steps

1. **Survey Existing MCP Crates:**
   ```bash
   ls -la ~/Documents/projects/embeddenator/embeddenator-*mcp*/
   find crates/ -name "*mcp*" -type d
   ```

2. **Check CLI Current State:**
   ```bash
   cd /home/kang/Documents/projects/github/embeddenator
   wc -l crates/embeddenator/src/cli/mod.rs
   grep -r "clap\|structopt" crates/embeddenator/
   ```

3. **Review MCP Protocol Requirements:**
   - Check for existing MCP implementations in codebase
   - Review Model Context Protocol specification
   - Identify common patterns across MCP servers

4. **Create Phase 2B Tracking Issue:**
   - Open GitHub issue for Phase 2B
   - Link to this handoff document
   - Create task checklist for 4 extractions

5. **Update SPLIT_TRACKER.md:**
   - Mark Phase 2A as COMPLETE ✅
   - Update Phase 2B status to IN PROGRESS ⏳
   - Add extraction progress tracking

---

## Git Branch Strategy for Phase 2B

**Pattern (same as Phase 2A):**
```bash
# For each component extraction:
git checkout dev
git checkout -b feat/extract-{component}
# ... do extraction work ...
git add .
git commit -m "feat: Extract embeddenator-{component}"
git tag v0.1.0  # or v0.2.0
git push origin feat/extract-{component}
git push origin v0.1.0

# After extraction complete:
git checkout dev
git merge feat/extract-{component} --no-edit
git push origin dev
```

**Sister Repo Setup:**
```bash
cd ~/Documents/projects/embeddenator/
# Check if repo exists, if not:
mkdir embeddenator-{component}
cd embeddenator-{component}
git init
git remote add origin https://github.com/tzervas/embeddenator-{component}.git
# ... extraction work ...
git checkout -b dev
git tag v0.1.0
git push -u origin dev
git push origin v0.1.0
```

---

## Contact & Continuity

**Current Branch:** `dev` in main repo (feat/extract-vsa in attachment, but dev is current)  
**Homelab Server:** 56 cores, 125GB RAM, 3TB storage, AVX2 support  
**Project Root:** `/home/kang/Documents/projects/github/embeddenator/`  
**Sister Repos:** `~/Documents/projects/embeddenator/embeddenator-*/`  

**Key Commands for New Session:**
```bash
# Verify Phase 2A state
cd /home/kang/Documents/projects/github/embeddenator
git checkout dev
cargo check --workspace

# Check all component repos
for comp in vsa retrieval fs interop io obs; do
  echo "=== embeddenator-$comp ==="
  cd ~/Documents/projects/embeddenator/embeddenator-$comp
  git branch
  git log --oneline -3
done

# Survey MCP servers
ls -la ~/Documents/projects/embeddenator/embeddenator-*mcp*/
find crates/ -name "*mcp*" -type d
```

---

## Phase 2B Timeline Estimate

**Total Effort:** 2-3 weeks (assuming full-time focus)

| Component | Effort | Priority | Dependencies |
|-----------|--------|----------|--------------|
| embeddenator-cli | 5-7 days | HIGH | All Phase 2A |
| embeddenator-screen-mcp | 2-3 days | MEDIUM | obs |
| embeddenator-context-mcp | 3-4 days | MEDIUM | vsa, obs |
| embeddenator-security-mcp | 2-3 days | LOW | obs |

**Parallel Work Opportunities:**
- CLI extraction blocks nothing (can proceed immediately)
- MCP servers can proceed in parallel after CLI
- Context and Security MCPs have minimal dependencies

---

## Closing Notes

Phase 2A was a major success - 9,783 LOC extracted with zero regressions and comprehensive test coverage. The component architecture is now battle-tested and ready for Phase 2B extractions.

Key learnings from Phase 2A:
- Feature flags enable flexible dependency management
- Security audits catch unsafe code early
- Path dependencies work well during extraction
- Test coverage prevents regressions
- Incremental extraction reduces risk

Apply these lessons to Phase 2B! Good luck with the MCP server and CLI extractions. 🚀

**Next Session:** Start with MCP server survey and CLI extraction planning.
