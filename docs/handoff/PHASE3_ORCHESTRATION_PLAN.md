# Phase 3 Orchestration Plan: Integration & Publishing

**Date:** January 4, 2026  
**Orchestrator:** Workflow Orchestrator  
**Status:** Ready to Begin  
**Prerequisites:** ✅ Phase 2A Complete | ✅ Phase 2B Complete

---

## Executive Summary

Phase 3 finalizes the Embeddenator decomposition with integration testing, publishing preparation, and monorepo cleanup. All 7 components are extracted (10,957 LOC, 64.5% of codebase) and ready for publishing.

### Phase 3 Goals

1. **Integration Testing** - Verify all components work together in production configuration
2. **Publishing Preparation** - Prepare crates.io releases for all components
3. **Cleanup** - Remove extracted code from monorepo, update documentation
4. **Security Audit** - Final pass on all 54 unsafe blocks across 6 files
5. **Performance Validation** - Benchmark regression testing on homelab

### Completion Criteria

- ✅ All components pass integration tests
- ✅ All components published to crates.io at v0.2.0
- ✅ Monorepo updated to use published crates (no path dependencies)
- ✅ Security audit complete for all unsafe code
- ✅ Performance benchmarks show <5% regression
- ✅ Documentation complete (README, CHANGELOG, migration guide)

---

## Phase 3 Timeline

**Duration:** 2 weeks (Jan 4-18, 2026)

### Week 1: Integration & Security (Jan 4-11)

**Days 1-2: Integration Testing**
- Integration test suite for all 7 components
- Cross-component API verification
- Error handling validation
- Memory safety checks

**Days 3-4: Security Audit**
- Review all 54 unsafe blocks (vsa, fs, obs)
- Verify SAFETY comments are comprehensive
- Validate invariants and preconditions
- Document security posture for publishing

**Days 5-7: Performance Validation**
- Run full benchmark suite on homelab (56-core server)
- Compare against Phase 2A baselines
- Investigate any regressions >5%
- Generate performance report

### Week 2: Publishing & Cleanup (Jan 11-18)

**Days 8-10: Publishing Preparation**
- Update all Cargo.toml metadata (authors, license, keywords)
- Verify all READMEs are crates.io-ready
- Generate CHANGELOG for each component
- Dry-run publish for all crates
- Create Git tags for all releases

**Days 11-12: Publish to crates.io**
- Publish in dependency order (Level 0 → Level 3)
- Verify crates.io pages and documentation
- Update monorepo to use published versions

**Days 13-14: Monorepo Cleanup**
- Remove extracted code from monorepo
- Update monorepo Cargo.toml with published deps
- Archive handoff documents
- Update PROJECT README
- Close all phase issues (#18-#24)

---

## Phase 3 Task Breakdown

### 3.1 Integration Testing

**Owner:** QA Tester  
**Dependencies:** All Phase 2A/2B extractions complete  
**Estimated LOE:** 2 days

#### Subtasks

1. **Cross-Component API Tests**
   - VSA → Retrieval integration
   - Retrieval → FS integration
   - FS → Interop integration
   - IO and Obs standalone validation

2. **CLI Integration Tests**
   - CLI commands with stub implementations
   - Command parsing and routing
   - Error handling across component boundaries

3. **Error Recovery Tests**
   - RwLock poisoning recovery (fs)
   - Index corruption handling (retrieval)
   - SIMD fallback paths (vsa)

4. **Memory Safety Tests**
   - Run miri on all unsafe blocks
   - Valgrind memory leak detection
   - Address sanitizer run

#### Deliverables

- `tests/phase3_integration.rs` - Integration test suite
- Integration test report with pass/fail summary
- List of any issues requiring fixes

---

### 3.2 Security Audit

**Owner:** Security Audit Agent  
**Dependencies:** 3.1 Integration Testing  
**Estimated LOE:** 2 days

#### Unsafe Block Inventory

From prior sessions, we have **54 unsafe blocks across 6 files**:

| File | Unsafe Blocks | Status | Notes |
|------|---------------|--------|-------|
| vsa/block_sparse.rs | ~20 | ⏹️ Needs review | SIMD intrinsics, validated correctness |
| vsa/simd_ops_x86.rs | ~15 | ⏹️ Needs review | AVX2 intrinsics, fallback paths |
| fs/engram_file.rs | ~8 | ⏹️ Needs review | Memory mapping, POSIX calls |
| fs/thread_pool.rs | ~5 | ⏹️ Needs review | Thread synchronization |
| obs/timing.rs | ~4 | ⏹️ Needs review | TSC reads, validated safe |
| obs/metrics.rs | ~2 | ⏹️ Needs review | Atomic operations |

#### Security Review Checklist

For each unsafe block:

1. **SAFETY Comment Present** - All invariants documented
2. **Invariant Validation** - Preconditions verified at call sites
3. **Memory Safety** - No use-after-free, double-free, or invalid pointers
4. **Soundness** - No data races or undefined behavior
5. **Testing** - Unsafe code covered by tests (unit, property, integration)
6. **Documentation** - Public APIs document unsafe usage

#### Security Audit Workflow

```
For each component with unsafe code:
1. Math Agent: Verify mathematical correctness of SIMD operations
2. Rust Implementer: Review code structure and idioms
3. QA Tester: Run miri, sanitizers, property tests
4. Documentation Writer: Verify SAFETY comments are comprehensive
5. Workflow Orchestrator: Sign off on security posture
```

#### Deliverables

- Security audit report for each component
- Updated SAFETY comments where needed
- ADR documenting security posture for publishing
- Sign-off for crates.io release

---

### 3.3 Performance Validation

**Owner:** Performance Tuner  
**Dependencies:** 3.1 Integration Testing  
**Estimated LOE:** 3 days

#### Benchmark Suite

Run on homelab (56-core server, AVX2):

1. **VSA Operations** (`benches/vsa_ops.rs`)
   - Bundle, bind, permute operations
   - Block-sparse SIMD performance
   - Baseline: ~4,500 ops/sec (hierarchical)

2. **SIMD Cosine** (`benches/simd_cosine.rs`)
   - AVX2 intrinsics performance
   - Baseline: <10µs per operation

3. **Hierarchical Scale** (`benches/hierarchical_scale.rs`)
   - Deep hierarchy unfolding (10K, 100K, 1M items)
   - Baseline: <2s for 100K items

4. **Query Hierarchical** (`benches/query_hierarchical.rs`)
   - Retrieval with hierarchical engrams
   - Baseline: <50ms for 1K queries

5. **Retrieval** (`benches/retrieval.rs`)
   - Index creation and querying
   - Baseline: <100ms for 10K items

#### Performance Regression Criteria

- **Critical:** >10% regression - blocks publishing
- **Major:** 5-10% regression - requires investigation
- **Minor:** <5% regression - acceptable, document in CHANGELOG

#### Deliverables

- Benchmark report with comparison to baselines
- Investigation notes for any regressions
- Performance chapter in component READMEs

---

### 3.4 Publishing Preparation

**Owner:** Documentation Writer  
**Dependencies:** 3.2 Security Audit, 3.3 Performance Validation  
**Estimated LOE:** 3 days

#### Publishing Checklist (per component)

**Cargo.toml Metadata:**
- [ ] `name = "embeddenator-<component>"`
- [ ] `version = "0.2.0"`
- [ ] `authors = ["Thomas Zervas <tzervas@example.com>"]`
- [ ] `license = "MIT OR Apache-2.0"`
- [ ] `description = "<component description>"`
- [ ] `homepage = "https://github.com/tzervas/embeddenator"`
- [ ] `repository = "https://github.com/tzervas/embeddenator-<component>"`
- [ ] `keywords = ["vsa", "holographic", "sparse", "ternary", "embeddings"]`
- [ ] `categories = ["algorithms", "data-structures", "science"]`
- [ ] `readme = "README.md"`
- [ ] `edition = "2021"`

**README.md:**
- [ ] Component overview and purpose
- [ ] Installation instructions (`cargo add embeddenator-<component>`)
- [ ] Quick start examples
- [ ] API documentation link (docs.rs)
- [ ] Feature flags documented
- [ ] Performance characteristics
- [ ] Security notes (if unsafe code present)
- [ ] License and contributing

**CHANGELOG.md:**
- [ ] v0.2.0 release notes
- [ ] Breaking changes (if any)
- [ ] New features
- [ ] Bug fixes
- [ ] Performance improvements

**Documentation:**
- [ ] All public APIs have rustdoc comments
- [ ] Examples in rustdoc run successfully
- [ ] Module-level documentation present
- [ ] Unsafe code documented with SAFETY comments

**Testing:**
- [ ] All tests pass (`cargo test`)
- [ ] Doctests pass (`cargo test --doc`)
- [ ] No warnings (`cargo clippy`)
- [ ] Formatting consistent (`cargo fmt --check`)

**Dry Run:**
- [ ] `cargo publish --dry-run` succeeds
- [ ] Package size <10MB (check with `cargo package --list`)
- [ ] No unnecessary files included (check .gitignore)

#### Publishing Order

Publish in dependency order to avoid broken dependencies:

1. **Level 0:** `embeddenator-io`, `embeddenator-obs`
2. **Level 1:** `embeddenator-vsa`
3. **Level 2:** `embeddenator-retrieval`
4. **Level 3:** `embeddenator-fs`
5. **Level 4:** `embeddenator-interop`
6. **Level 5:** `embeddenator-cli`

**Wait 10 minutes between publishes** to allow crates.io to index.

#### Deliverables

- All 7 components pass publishing checklist
- Dry-run logs saved to `reports/publishing/`
- Publishing runbook with commands

---

### 3.5 Publish to crates.io

**Owner:** Workflow Orchestrator  
**Dependencies:** 3.4 Publishing Preparation  
**Estimated LOE:** 1 day

#### Publishing Workflow

For each component in dependency order:

```bash
cd ~/Documents/projects/embeddenator/embeddenator-<component>/
git pull origin main
git tag v0.2.0
git push origin v0.2.0
cargo publish
# Wait 10 minutes for crates.io indexing
```

#### Post-Publishing Verification

For each published component:

1. **crates.io Page** - Verify metadata, README renders correctly
2. **docs.rs** - Verify documentation builds and links work
3. **Dependency Resolution** - Next component can depend on published version

#### Rollback Plan

If a publish fails or has critical issues:

1. **Yank bad version:** `cargo yank --vers 0.2.0 embeddenator-<component>`
2. **Fix issue in separate branch**
3. **Increment version to 0.2.1**
4. **Re-run publishing checklist**
5. **Publish 0.2.1**

#### Deliverables

- All 7 components published to crates.io at v0.2.0
- crates.io and docs.rs URLs logged
- Publishing report with timestamps

---

### 3.6 Monorepo Cleanup

**Owner:** Rust Implementer  
**Dependencies:** 3.5 Publish to crates.io  
**Estimated LOE:** 2 days

#### Cleanup Tasks

**1. Update Cargo.toml Dependencies**

Replace all path dependencies with published versions:

```toml
# Before (path dependencies)
embeddenator-vsa = { path = "../embeddenator-vsa" }

# After (published dependencies)
embeddenator-vsa = "0.2.0"
```

**2. Remove Extracted Code**

Remove the following from monorepo:
- `src/vsa/` → moved to embeddenator-vsa
- `src/retrieval/` → moved to embeddenator-retrieval
- `src/fs/` → moved to embeddenator-fs
- `src/interop/` → moved to embeddenator-interop
- `src/io/` → moved to embeddenator-io
- `src/obs/` → moved to embeddenator-obs
- `src/cli.rs` → moved to embeddenator-cli

**3. Update Imports**

Update import paths in remaining monorepo code:

```rust
// Before
use crate::vsa::SparseVec;

// After
use embeddenator_vsa::SparseVec;
```

**4. Verify Monorepo Builds**

```bash
cargo clean
cargo build --release
cargo test --workspace
cargo bench --no-run
```

**5. Update Documentation**

- Update main README.md with component links
- Update CHANGELOG.md with Phase 3 completion
- Archive all Phase 2A/2B handoff docs to `docs/handoff/archive/`
- Update VERSION_ROADMAP.md

**6. Close GitHub Issues**

Close all completed phase issues:
- #18 - embeddenator-vsa extraction
- #19 - embeddenator-retrieval extraction
- #20 - embeddenator-fs extraction
- #21 - embeddenator-interop extraction
- #22 - embeddenator-io extraction
- #23 - embeddenator-obs extraction
- #24 - Phase 2A epic

#### Deliverables

- Monorepo builds with published dependencies
- All tests passing
- Documentation updated
- Git commit: "feat: Phase 3 complete - published v0.2.0 components"
- Git tag: `v0.2.0` on main branch

---

## Risk Management

### Identified Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Publishing order error | Low | High | Automate with script, verify deps |
| Performance regression | Medium | Medium | Thorough benchmarking before publish |
| Unsafe code issues | Low | High | Security audit before publishing |
| crates.io downtime | Low | Low | Wait and retry |
| Breaking API changes | Low | High | Thorough integration testing |

### Rollback Strategy

If Phase 3 encounters critical issues:

1. **DO NOT** yank published crates unless security issue
2. Fix forward with patch release (v0.2.1)
3. Keep path dependencies in monorepo as fallback
4. Document issues in CHANGELOG

---

## Success Metrics

### Quantitative

- **100%** of components published to crates.io
- **<5%** performance regression across all benchmarks
- **0** critical security issues in unsafe code
- **100%** of tests passing (integration + unit + property)
- **<10MB** package size for all components

### Qualitative

- All crates.io pages render correctly
- docs.rs documentation builds without errors
- Monorepo builds with published dependencies
- Community can use components independently
- Clear migration path for existing users

---

## Handoff Procedure

### After Phase 3 Completion

**Documentation Writer** creates:
- `PHASE3_COMPLETE_<date>.md` - Final handoff document
- `MIGRATION_GUIDE.md` - Guide for users to upgrade
- Updated `SPLIT_TRACKER.md` with 100% completion

**Workflow Orchestrator** ensures:
- All agents signed off on their deliverables
- All GitHub issues closed
- All Git tags pushed
- All crates published and indexed

**QA Tester** provides:
- Final integration test report
- Performance regression report
- Security audit summary

### Post-Phase 3

- Monitor crates.io download stats
- Watch for community issues
- Prepare for v0.3.0 roadmap (feature additions)
- Consider blog post announcement

---

## Agent Assignments

| Agent | Primary Responsibility | Deliverable |
|-------|------------------------|-------------|
| QA Tester | Integration testing, memory safety | Integration test suite, test report |
| Security Audit Agent | Unsafe code review, security posture | Security audit reports, sign-off |
| Performance Tuner | Benchmark execution, regression analysis | Performance report, baseline comparison |
| Documentation Writer | Publishing prep, READMEs, CHANGELOGs | All component documentation, publishing checklist |
| Rust Implementer | Monorepo cleanup, import updates | Clean monorepo with published deps |
| Workflow Orchestrator | Coordination, publishing execution, handoffs | Phase 3 complete document, Git tags |

---

## Next Steps

**Immediate Actions (Today - Jan 4):**

1. ✅ Create Phase 3 Orchestration Plan (this document)
2. → Update SPLIT_TRACKER.md with Phase 2B completion
3. → Begin Task 3.1: Integration Testing (handoff to QA Tester)

**Week 1 Focus:**
- Integration testing
- Security audit (54 unsafe blocks)
- Performance validation

**Week 2 Focus:**
- Publishing preparation
- crates.io publishing
- Monorepo cleanup

---

## References

- [Phase 2A Handoff](docs/handoff/PHASE2A_SESSION_2026_01_04.md)
- [Phase 2B CLI Complete](docs/handoff/PHASE2B_CLI_COMPLETE_2026_01_04.md)
- [ADR-017: Component Extraction Strategy](docs/adr/ADR-017-phase2a-component-extraction.md)
- [Security Audit: SIMD Cosine](docs/SECURITY_AUDIT_SIMD_COSINE.md)
- [Security Audit: FS](docs/SECURITY_AUDIT_FS.md)
- [SPLIT_TRACKER.md](SPLIT_TRACKER.md)

---

**Status:** Ready for Week 1 execution  
**Next Handoff:** After integration testing (Task 3.1)  
**Estimated Completion:** January 18, 2026
