# Quick Reference: Release Execution Checklist

**Date**: January 25, 2026  
**Purpose**: Quick checklist for executing the release plan  
**Status**: Ready to execute

---

## ✅ Pre-Release Validation (Already Complete)

- [x] All repositories updated from remotes (git pull --all)
- [x] All branches current (0 commits ahead/behind)
- [x] All tests passing (61-69 tests per component)
- [x] All performance baselines validated
- [x] All security audits passing (cargo audit)
- [x] All CI/CD workflows active (6 GitHub Actions)
- [x] Assessment documents created (3 comprehensive docs)

---

## 📋 Phase 1: Version Bumps (4-6 hours)

### Step 1: embeddenator-vsa → 0.20.1
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md
- [ ] Run `cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.20.1 - critical SIMD bug fixes"`
- [ ] Tag: `git tag -a v0.20.1 -m "Release v0.20.1"`
- [ ] Push: `git push origin main && git push origin v0.20.1`

### Step 2: embeddenator-io → 0.20.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md
- [ ] Run `cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.20.0 - stable production ready"`
- [ ] Tag: `git tag -a v0.20.0 -m "Release v0.20.0"`
- [ ] Push: `git push origin main && git push origin v0.20.0`

### Step 3: embeddenator-testkit → 0.20.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md
- [ ] Run `cargo test --all-features`
- [ ] Commit and tag (see Step 2 template)
- [ ] Push to origin

### Step 4: embeddenator-retrieval → 0.20.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md
- [ ] Run `cargo test --all-features`
- [ ] Commit and tag (see Step 2 template)
- [ ] Push to origin

### Step 5: embeddenator-workspace → 0.20.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md
- [ ] Run `cargo test --all-features`
- [ ] Commit and tag (see Step 2 template)
- [ ] Push to origin

### Step 6: embeddenator-obs → 0.21.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md with feature summary
- [ ] Run `cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.21.0 - Prometheus and OpenTelemetry"`
- [ ] Tag: `git tag -a v0.21.0 -m "Release v0.21.0"`
- [ ] Push to origin

### Step 7: embeddenator-interop → 0.21.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md with feature summary
- [ ] Run `cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.21.0 - compression and C headers"`
- [ ] Tag: `git tag -a v0.21.0 -m "Release v0.21.0"`
- [ ] Push to origin

### Step 8: embeddenator-fs → 0.22.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md with feature summary
- [ ] Run `cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.22.0 - CLI, examples, benchmarks"`
- [ ] Tag: `git tag -a v0.22.0 -m "Release v0.22.0"`
- [ ] Push to origin

### Step 9: embeddenator-cli → 0.20.0-alpha.3
- [ ] Update Cargo.toml version
- [ ] Update dependencies:
  - embeddenator-fs = "0.22.0"
  - embeddenator-io = "0.20.0"
  - embeddenator-obs = "0.21.0"
- [ ] Run `cargo update && cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.20.0-alpha.3 - update dependencies"`
- [ ] Tag: `git tag -a v0.20.0-alpha.3 -m "Release v0.20.0-alpha.3"`
- [ ] Push to origin

### Step 10: embeddenator-contract-bench → 0.20.0-alpha.3
- [ ] Update Cargo.toml version
- [ ] Update dependencies to stable versions
- [ ] Run `cargo update && cargo test --all-features`
- [ ] Commit and tag (see Step 9 template)
- [ ] Push to origin

### Step 11: embeddenator-core → 0.20.0
- [ ] Update Cargo.toml version
- [ ] Update CHANGELOG.md
- [ ] Run `cargo test --all-features`
- [ ] Commit: `git commit -m "chore: release v0.20.0 - stable core library"`
- [ ] Tag: `git tag -a v0.20.0 -m "Release v0.20.0"`
- [ ] Push to origin

---

## 🧪 Phase 2: Validation (1-2 hours)

- [ ] Run full test suite: `cargo test --all-features --workspace`
- [ ] Validate performance: `cd embeddenator && ./perf_validate.sh`
- [ ] Run security audit: `cargo audit`
- [ ] Verify all tags pushed: `git tag -l | wc -l` (should show all new tags)
- [ ] Verify all commits pushed: `git log origin/main..main` (should be empty)

---

## 📦 Phase 3: Publish to crates.io (1-2 hours)

### Prepare
- [ ] Verify login: `cargo login --registry crates-io`
- [ ] Test dry-run: `cd embeddenator-vsa && cargo publish --dry-run`

### Publish in Order (Wait 60-90 seconds between each)

1. [ ] embeddenator-vsa 0.20.1
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds
   - [ ] Verify: `cargo search embeddenator-vsa | grep "= \"0.20.1"`

2. [ ] embeddenator-io 0.20.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

3. [ ] embeddenator-testkit 0.20.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

4. [ ] embeddenator-retrieval 0.20.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

5. [ ] embeddenator-obs 0.21.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

6. [ ] embeddenator-fs 0.22.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

7. [ ] embeddenator-interop 0.21.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

8. [ ] embeddenator-workspace 0.20.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

9. [ ] embeddenator-core 0.20.0
   - [ ] `cargo publish`
   - [ ] Wait 90 seconds

10. [ ] embeddenator-cli 0.20.0-alpha.3
    - [ ] `cargo publish`
    - [ ] Wait 90 seconds

11. ❌ **SKIP embeddenator-contract-bench** (internal tool)

### Final Verification
- [ ] `cargo search embeddenator | head -20`
- [ ] Verify all 10 crates published with correct versions

---

## 🚀 Phase 4: GitHub Releases (1 hour)

For each repository, create a release:

- [ ] embeddenator-vsa v0.20.1
  - [ ] Navigate to GitHub releases
  - [ ] Create release from tag v0.20.1
  - [ ] Title: "v0.20.1 - Critical SIMD Bug Fixes"
  - [ ] Description: (copy from CHANGELOG.md)

- [ ] embeddenator-io v0.20.0
  - [ ] Create release from tag v0.20.0
  - [ ] Title: "v0.20.0 - Stable I/O Library"

- [ ] embeddenator-testkit v0.20.0
  - [ ] Create release

- [ ] embeddenator-retrieval v0.20.0
  - [ ] Create release

- [ ] embeddenator-workspace v0.20.0
  - [ ] Create release

- [ ] embeddenator-obs v0.21.0
  - [ ] Create release
  - [ ] Highlight: Prometheus export, OpenTelemetry tracing

- [ ] embeddenator-interop v0.21.0
  - [ ] Create release
  - [ ] Highlight: Full compression, C headers

- [ ] embeddenator-fs v0.22.0
  - [ ] Create release
  - [ ] Highlight: CLI, examples, benchmarks

- [ ] embeddenator-cli v0.20.0-alpha.3
  - [ ] Create release
  - [ ] Note: Pre-release

- [ ] embeddenator-core v0.20.0
  - [ ] Create release

---

## 📝 Phase 5: Documentation Updates (1-2 hours)

### Workspace README.md
- [ ] Update version badge section
  - [ ] Change orange badge version to latest
  - [ ] Update component status table with new versions

### WORKSPACE_TRACKER.md
- [ ] Add new "Last Updated: 2026-01-25" section
- [ ] Update all repo snapshot versions
- [ ] Update status for each component

### Create Release Announcement
- [ ] Create RELEASE_ANNOUNCEMENT_2026_01_25.md
- [ ] Document all version bumps
- [ ] Highlight major features
- [ ] List publication dates

### Archive This Assessment
- [ ] Keep WORKSPACE_ASSESSMENT_2026_01_25.md
- [ ] Keep RELEASE_IMPLEMENTATION_PLAN_2026_01_25.md
- [ ] Keep this checklist for reference

---

## ✅ Post-Release Checklist

### Verification
- [ ] All 10 crates on crates.io with correct versions
- [ ] All 10 GitHub releases published
- [ ] All documentation updated and consistent
- [ ] All git tags pushed to origin
- [ ] All branches current (no unpushed commits)

### Testing
- [ ] Fresh clone test: `cargo new test_project && cd test_project`
- [ ] Add dependencies to Cargo.toml:
  ```toml
  embeddenator-vsa = "0.20.1"
  embeddenator-fs = "0.22.0"
  embeddenator-io = "0.20.0"
  ```
- [ ] Run `cargo build` (should download from crates.io)

### Communication
- [ ] Post release announcement (internal/external)
- [ ] Update project status (WORKSPACE_TRACKER.md)
- [ ] Notify users/contributors
- [ ] Archive this checklist with completion date

---

## 🔧 Troubleshooting

### If a cargo publish fails
```bash
# Check error message
cargo publish --token ${TOKEN}

# Common issues:
# 1. Missing CHANGELOG.md - Add entry with version and date
# 2. Version already published - Check crates.io, may need to wait
# 3. Dependency version mismatch - Update Cargo.toml dependencies first
# 4. Missing token - Run: cargo login --registry crates-io
```

### If git push fails
```bash
# Ensure branch is set to track origin
git branch -u origin/main main

# Or push explicitly
git push origin main --force-with-lease  # Use with caution!
git push origin v0.20.1
```

### If tests fail after version bump
```bash
# Likely cause: Dependency version mismatch
# Solution: Update Cargo.lock
cargo update

# Or rebuild clean
rm -rf target
cargo clean
cargo test --all-features
```

---

## ⏱️ Timeline

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Version Bumps | 4-6 hrs | Day 1 | Day 1-2 |
| Validation | 1-2 hrs | Day 2 | Day 2 |
| Publication | 1-2 hrs | Day 2 | Day 2 |
| GitHub Releases | 1 hr | Day 2 | Day 2 |
| Documentation | 1-2 hrs | Day 2 | Day 3 |
| **TOTAL** | **9-13 hrs** | **Day 1** | **Day 3** |

**Target**: Complete by end of January 2026 (or following week)

---

## 📞 Support

### Reference Documents
- **Assessment**: WORKSPACE_ASSESSMENT_2026_01_25.md
- **Implementation**: RELEASE_IMPLEMENTATION_PLAN_2026_01_25.md
- **Quick Summary**: EXECUTIVE_SUMMARY_2026_01_25.md
- **Versioning**: VERSIONING_STRATEGY.md

### Key Contacts
- **Workspace Lead**: Check WORKSPACE_TRACKER.md
- **Component Leads**: Check individual README.md files
- **CI/CD Issues**: See CI_CD_GUIDE.md

---

## ✨ Success Indicators

### During Execution
- ✅ All tests passing during version bumps
- ✅ All git operations succeeding
- ✅ No dependency conflicts

### After Execution
- ✅ All 10 crates on crates.io
- ✅ All GitHub releases created
- ✅ All documentation synchronized
- ✅ Workspace README reflects new versions
- ✅ Zero regressions in tests or performance

---

**Ready to Execute!**

Print this checklist and mark off items as you complete them.

**Start Date**: (To be filled in when execution begins)  
**Completion Date**: (Will be filled in when done)  
**Status**: ⏳ Ready for execution

---
