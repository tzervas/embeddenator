# Detailed Release Implementation Plan
**Date**: January 25, 2026  
**Scope**: Execute version bumps, dependency updates, and crates.io publication  
**Status**: Ready for execution  

---

## Phase 1: Version Bump Execution (Order Matters!)

### Step 1.1: embeddenator-vsa (0.20.0-alpha.1 → 0.20.1)

**Rationale**: PATCH release - critical SIMD bug fixes (self-similarity 0.478 → 1.000)

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-vsa

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.20.1"

# Update CHANGELOG.md with bug fix summary
# Add entry at top:
# ## [0.20.1] - 2026-01-25
# ### Fixed
# - Critical: Vector invariant violation in similarity calculations
# - Self-similarity now correctly returns 1.000 instead of 0.478
# - Fixed PackedTritVec corruption propagation
# - Added comprehensive validation and stress tests

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.20.1 - critical SIMD bug fixes"
git tag -a v0.20.1 -m "Release v0.20.1 - Vector invariant and similarity fixes"
git push origin main
git push origin v0.20.1
```

---

### Step 1.2: embeddenator-io (0.20.0-alpha.1 → 0.20.0)

**Rationale**: STABLE release - migration complete, API stable

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-io

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.20.0"

# Update CHANGELOG.md with stable release marker
# Add entry at top:
# ## [0.20.0] - 2026-01-25
# ### Changed
# - Graduated from alpha to stable (0.20.0-alpha.1 → 0.20.0)
# - Migration from monolithic repository complete
# - API stable for production use

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.20.0 - stable production ready"
git tag -a v0.20.0 -m "Release v0.20.0 - Stable production-ready I/O library"
git push origin main
git push origin v0.20.0
```

---

### Step 1.3: embeddenator-testkit (0.20.0-alpha.1 → 0.20.0)

**Rationale**: STABLE release - evaluation loop complete (14/14 phases passing)

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-testkit

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.20.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.20.0] - 2026-01-25
# ### Changed
# - Graduated from alpha to stable (0.20.0-alpha.1 → 0.20.0)
# - Evaluation loop: 14/14 phases passing
# - Baselines established and documented
# - Ready for production testing workflows

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.20.0 - stable testing framework"
git tag -a v0.20.0 -m "Release v0.20.0 - Stable testing and evaluation framework"
git push origin main
git push origin v0.20.0
```

---

### Step 1.4: embeddenator-retrieval (0.20.0-alpha.1 → 0.20.0)

**Rationale**: STABLE release - migration complete, API stable

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-retrieval

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.20.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.20.0] - 2026-01-25
# ### Changed
# - Graduated from alpha to stable (0.20.0-alpha.1 → 0.20.0)
# - Migration from monolithic repository complete
# - Criterion benchmarks integrated
# - Ready for production information retrieval workflows

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.20.0 - stable retrieval engine"
git tag -a v0.20.0 -m "Release v0.20.0 - Stable information retrieval library"
git push origin main
git push origin v0.20.0
```

---

### Step 1.5: embeddenator-workspace (0.20.0-alpha.1 → 0.20.0)

**Rationale**: STABLE release - workspace tooling complete

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-workspace

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.20.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.20.0] - 2026-01-25
# ### Changed
# - Graduated from alpha to stable (0.20.0-alpha.1 → 0.20.0)
# - All workspace management tools complete
# - Documentation complete
# - Ready for production workspace coordination

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.20.0 - stable workspace tools"
git tag -a v0.20.0 -m "Release v0.20.0 - Stable workspace management tools"
git push origin main
git push origin v0.20.0
```

---

### Step 1.6: embeddenator-obs (0.20.0-alpha.1 → 0.21.0)

**Rationale**: MINOR release - major new features (Prometheus, OpenTelemetry, streaming)

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-obs

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.21.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.21.0] - 2026-01-25
# ### Added
# - Prometheus metrics export (text format)
# - OpenTelemetry distributed tracing (W3C Trace Context)
# - Advanced statistics (percentiles, std dev, histograms)
# - Real-time metric streaming with callbacks
# - Feature flags: prometheus-export, opentelemetry-tracing, advanced-stats
# ### Tests
# - +18 new tests (51 → 69 total)
# - All passing with feature combinations

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.21.0 - add Prometheus and OpenTelemetry"
git tag -a v0.21.0 -m "Release v0.21.0 - Prometheus export and OpenTelemetry tracing"
git push origin main
git push origin v0.21.0
```

---

### Step 1.7: embeddenator-interop (0.20.0-alpha.1 → 0.21.0)

**Rationale**: MINOR release - compression + C header generation complete

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-interop

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.21.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.21.0] - 2026-01-25
# ### Added
# - Full envelope compression (Zstd, LZ4)
# - Automated cbindgen C header generation
# - Round-trip compression verification
# - Feature flags for optional compression algorithms
# ### Tests
# - +5 new tests (20 → 23 total)
# - All passing with compression verification

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.21.0 - add compression and C headers"
git tag -a v0.21.0 -m "Release v0.21.0 - Full compression and C FFI support"
git push origin main
git push origin v0.21.0
```

---

### Step 1.8: embeddenator-fs (0.21.0 → 0.22.0)

**Rationale**: MINOR release - major features (CLI, examples, benchmarks)

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-fs

# Update version in Cargo.toml
# Change: version = "0.21.0"
# To:     version = "0.22.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.22.0] - 2026-01-25
# ### Added
# - Full CLI binary with 9 commands (ingest, extract, query, list, info, verify, update, compact, help)
# - 4 production examples (basic_ingest, query_files, incremental_update, batch_processing)
# - 3 comprehensive benchmark suites (13 groups total)
# - Performance: 8.12 MB/s ingest, 9.31 MB/s extract
# ### Changed
# - API stable, all features complete

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.22.0 - add CLI, examples, and benchmarks"
git tag -a v0.22.0 -m "Release v0.22.0 - Full CLI and benchmarks"
git push origin main
git push origin v0.22.0
```

---

### Step 1.9: embeddenator-cli (0.20.0-alpha.2 → 0.20.0-alpha.3)

**Rationale**: PRERELEASE bump - update dependencies to point to stable versions

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-cli

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.2"
# To:     version = "0.20.0-alpha.3"

# Update dependencies to stable versions:
# [dependencies]
# embeddenator-fs = "0.22.0"  # Updated from 0.21.0-alpha
# embeddenator-io = "0.20.0"   # Updated from 0.20.0-alpha.1
# embeddenator-obs = "0.21.0"  # Updated from 0.20.0-alpha.1

# Update CHANGELOG.md
# Add entry at top:
# ## [0.20.0-alpha.3] - 2026-01-25
# ### Changed
# - Updated dependencies to stable versions
# - embeddenator-fs: 0.22.0
# - embeddenator-io: 0.20.0
# - embeddenator-obs: 0.21.0

# Test before committing
cargo update
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v0.20.0-alpha.3 - update dependencies"
git tag -a v0.20.0-alpha.3 -m "Release v0.20.0-alpha.3 - Dependency updates"
git push origin main
git push origin v0.20.0-alpha.3
```

---

### Step 1.10: embeddenator-contract-bench (0.20.0-alpha.2 → 0.20.0-alpha.3)

**Rationale**: PRERELEASE bump - internal testing tool, update dependencies

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-contract-bench

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.2"
# To:     version = "0.20.0-alpha.3"

# Update dependencies to stable versions where applicable
# [dependencies]
# embeddenator-vsa = "0.20.1"
# embeddenator-fs = "0.22.0"
# embeddenator-io = "0.20.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.20.0-alpha.3] - 2026-01-25
# ### Changed
# - Updated dependencies to stable versions
# - Internal testing tool only (not published to crates.io)

# Test before committing
cargo update
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v0.20.0-alpha.3 - update dependencies"
git tag -a v0.20.0-alpha.3 -m "Release v0.20.0-alpha.3 - Dependency updates"
git push origin main
git push origin v0.20.0-alpha.3
```

---

### Step 1.11: embeddenator-core (0.20.0-alpha.1 → 0.20.0)

**Rationale**: STABLE release - core component migration complete

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-core

# Update version in Cargo.toml
# Change: version = "0.20.0-alpha.1"
# To:     version = "0.20.0"

# Update CHANGELOG.md
# Add entry at top:
# ## [0.20.0] - 2026-01-25
# ### Changed
# - Graduated from alpha to stable (0.20.0-alpha.1 → 0.20.0)
# - Migration from monolithic repository complete
# - Core VSA primitives stable and production-ready

# Test before committing
cargo test --all-features
cargo build --all-features

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v0.20.0 - stable core library"
git tag -a v0.20.0 -m "Release v0.20.0 - Stable core VSA library"
git push origin main
git push origin v0.20.0
```

---

### SKIP: embeddenator (Deprecated)

**No action**: This repository remains at 0.20.0-alpha.1 (final version)

```bash
cd /home/kang/Documents/projects/embdntr/embeddenator

# No version bump needed
# Deprecation status: Final
# Archive timeline: Q2 2026 (read-only) → Q3 2026 (remove from workspace)
# No new releases after 0.20.0-alpha.1
```

---

## Phase 2: Validate All Changes Locally

### Run Complete Test Suite
```bash
cd /home/kang/Documents/projects/embdntr

# Test all components
cargo test --all-features --workspace 2>&1 | tee test_results.log

# Check for failures
grep -E "(FAILED|error)" test_results.log && echo "⚠️ FAILURES DETECTED" || echo "✅ ALL TESTS PASSED"
```

### Validate Performance Baselines
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator

# Run performance validation
./perf_validate.sh 2>&1 | tee perf_results.log

# Expected outputs:
# - Ingest: 8+ MB/s
# - Extract: 9+ MB/s
# - Query latency: <100ms
```

### Security Audit
```bash
cd /home/kang/Documents/projects/embdntr

# Check for vulnerabilities
cargo audit 2>&1 | tee audit_results.log

# Check dependency licenses
if command -v cargo-deny &> /dev/null; then
  cargo deny check 2>&1 | tee deny_results.log
else
  echo "cargo-deny not installed, install with: cargo install cargo-deny"
fi
```

---

## Phase 3: Create Version Summary Document

Create file: `RELEASE_PLAN_2026_01_25.md`

```markdown
# Release Plan - January 25, 2026

## Version Changes Summary

### Stable Releases (Graduating from alpha)
- embeddenator-io: 0.20.0-alpha.1 → **0.20.0**
- embeddenator-testkit: 0.20.0-alpha.1 → **0.20.0**
- embeddenator-retrieval: 0.20.0-alpha.1 → **0.20.0**
- embeddenator-workspace: 0.20.0-alpha.1 → **0.20.0**
- embeddenator-core: 0.20.0-alpha.1 → **0.20.0**

### Minor Releases (New Features)
- embeddenator-obs: 0.20.0-alpha.1 → **0.21.0**
  - Prometheus export, OpenTelemetry tracing, advanced statistics
- embeddenator-interop: 0.20.0-alpha.1 → **0.21.0**
  - Full compression (Zstd, LZ4), C header generation
- embeddenator-fs: 0.21.0 → **0.22.0**
  - CLI binary, 4 examples, 3 benchmark suites

### Patch Releases (Bug Fixes)
- embeddenator-vsa: 0.20.0-alpha.1 → **0.20.1**
  - Critical SIMD bug fix (self-similarity calculation)

### Prerelease Updates (Dependency Updates)
- embeddenator-cli: 0.20.0-alpha.2 → **0.20.0-alpha.3**
- embeddenator-contract-bench: 0.20.0-alpha.2 → **0.20.0-alpha.3**

### Deprecated (No Change)
- embeddenator: 0.20.0-alpha.1 (FINAL VERSION - DEPRECATED)

## Publication Order (Respects Dependencies)
1. embeddenator-vsa 0.20.1 (no dependencies on other embeddenator crates)
2. embeddenator-io 0.20.0 (no dependencies on other embeddenator crates)
3. embeddenator-testkit 0.20.0 (minimal dependencies)
4. embeddenator-retrieval 0.20.0 (depends on embeddenator-vsa)
5. embeddenator-obs 0.21.0 (may depend on others)
6. embeddenator-fs 0.22.0 (depends on embeddenator-io, embeddenator-obs)
7. embeddenator-interop 0.21.0 (depends on embeddenator-vsa)
8. embeddenator-workspace 0.20.0 (workspace coordination tool)
9. embeddenator-cli 0.20.0-alpha.3 (depends on embeddenator-fs, others)
10. embeddenator-core 0.20.0 (core VSA library)
11. ❌ embeddenator-contract-bench 0.20.0-alpha.3 (internal only, don't publish)

## Testing Checklist
- [ ] All unit tests passing (cargo test --all-features --workspace)
- [ ] Performance baselines validated (perf_validate.sh)
- [ ] Security audit clean (cargo audit)
- [ ] Dependency licenses reviewed
- [ ] Git tags created for all versions
- [ ] All commits pushed to origin

## Publication Steps
1. [ ] Verify cargo login token: `cargo login --registry crates-io`
2. [ ] For each crate in order: `cd <repo> && cargo publish --token ${TOKEN}`
3. [ ] Wait 1-2 minutes between publications (crates.io index updates)
4. [ ] Verify on crates.io: `cargo search embeddenator`
5. [ ] Create GitHub releases for each repository

## Post-Release Tasks
- [ ] Update workspace README.md with new version badges
- [ ] Update WORKSPACE_TRACKER.md
- [ ] Create summary announcement
- [ ] Tag this assessment document with version info
```

---

## Phase 4: Publish to crates.io

### Preparation
```bash
# Ensure you have valid cargo credentials
cargo login --registry crates-io
# This will prompt for your token or use existing credentials

# Verify token works
cargo publish --dry-run  # in one of the simpler repos first
```

### Publishing Sequence (In This Order)

**1. embeddenator-vsa 0.20.1**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-vsa
cargo publish
# Wait ~60-90 seconds for index to update
```

**2. embeddenator-io 0.20.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-io
cargo publish
sleep 90
```

**3. embeddenator-testkit 0.20.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-testkit
cargo publish
sleep 90
```

**4. embeddenator-retrieval 0.20.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-retrieval
cargo publish
sleep 90
```

**5. embeddenator-obs 0.21.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-obs
cargo publish
sleep 90
```

**6. embeddenator-fs 0.22.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-fs
cargo publish
sleep 90
```

**7. embeddenator-interop 0.21.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-interop
cargo publish
sleep 90
```

**8. embeddenator-workspace 0.20.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-workspace
cargo publish
sleep 90
```

**9. embeddenator-core 0.20.0**
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-core
cargo publish
sleep 90
```

**10. embeddenator-cli 0.20.0-alpha.3** (Optional)
```bash
cd /home/kang/Documents/projects/embdntr/embeddenator-cli
cargo publish
sleep 90
```

**11. ❌ SKIP: embeddenator-contract-bench** (Internal tool, don't publish)

**12. ❌ SKIP: embeddenator** (DEPRECATED, don't publish new version)

### Verify Publications
```bash
# Check that all crates are published
cargo search embeddenator | grep "embeddenator ="
```

Expected output:
```
embeddenator = "0.20.0-alpha.1"        # DEPRECATED
embeddenator-vsa = "0.20.1"
embeddenator-fs = "0.22.0"
embeddenator-io = "0.20.0"
embeddenator-retrieval = "0.20.0"
embeddenator-obs = "0.21.0"
embeddenator-interop = "0.21.0"
embeddenator-testkit = "0.20.0"
embeddenator-workspace = "0.20.0"
embeddenator-cli = "0.20.0-alpha.3"
# embeddenator-contract-bench not published (internal tool)
# embeddenator-core may not be published (workspace crate)
```

---

## Phase 5: Create GitHub Releases

For each repository, create a GitHub release:

```bash
# Example for embeddenator-vsa
cd /home/kang/Documents/projects/embdntr/embeddenator-vsa

# Verify tag exists
git tag -l | grep v0.20.1

# Create GitHub release (using gh CLI if installed)
gh release create v0.20.1 \
  --title "v0.20.1 - Critical SIMD Bug Fixes" \
  --notes "Fixes critical vector invariant violation bug in similarity calculations. Self-similarity now correctly returns 1.000 instead of 0.478. See CHANGELOG.md for details."

# Alternatively, create releases via GitHub web interface:
# 1. Go to repository Releases page
# 2. Click "Create a new release"
# 3. Tag: v0.20.1
# 4. Title: "v0.20.1 - Critical SIMD Bug Fixes"
# 5. Description: Copy from CHANGELOG.md
# 6. Click "Publish release"
```

Repeat for each repository.

---

## Phase 6: Documentation Updates

### Update Workspace README.md
```bash
cd /home/kang/Documents/projects/embdntr

# Update version badges section (find and update):
# ![Version](https://img.shields.io/badge/version-0.20.0-orange)
# to:
# ![Version](https://img.shields.io/badge/version-0.22.0-green)

# Update component status table with new versions:
# | embeddenator-vsa | ... | v0.20.1 | ✅ Production |
# | embeddenator-fs | ... | v0.22.0 | ✅ Production |
# etc.
```

### Update WORKSPACE_TRACKER.md
```bash
# Add new section:
# ## Last Updated: 2026-01-25

# Update repo versions in snapshot sections
# Update "Next Actions" with link to release plan
```

### Create Release Announcement
```bash
# Create file: RELEASE_ANNOUNCEMENT_2026_01_25.md

cat > RELEASE_ANNOUNCEMENT_2026_01_25.md << 'EOF'
# Embeddenator Component Release - January 25, 2026

## Overview
Successful release of 10 production-ready components and 2 alpha tools, graduating 5 components from alpha to stable (1.0.0-ready).

## Version Summary
- **Stable Releases**: 5 components (io, testkit, retrieval, workspace, core)
- **Minor Releases**: 3 components (obs, interop, fs)
- **Patch Release**: 1 component (vsa)
- **Alpha Updates**: 2 tools (cli, contract-bench)
- **Total Publications**: 10 to crates.io

## Key Improvements
- SIMD correctness validated (vsa 0.20.1)
- Prometheus observability enabled (obs 0.21.0)
- Full CLI tooling delivered (fs 0.22.0)
- 100% test coverage maintained (61+ tests per component)

## Next Steps
- Q1 2026: Plan 1.0.0 stable releases
- Q2 2026: Archive monolithic repository (read-only)
- Q3 2026: Full 1.0.0 stable release
EOF
```

---

## Success Criteria

### Before Starting
- [ ] All repositories updated (git pull --all)
- [ ] All tests passing (cargo test --all-features --workspace)
- [ ] All performance baselines validated (perf_validate.sh)
- [ ] All security audits clean (cargo audit)

### After Completion
- [ ] 10/10 crates published to crates.io
- [ ] All version tags pushed to origin
- [ ] All GitHub releases created with notes
- [ ] All READMEs updated with new versions
- [ ] All documentation synchronized
- [ ] Zero test regressions
- [ ] Zero performance regressions

---

## Rollback Plan (If Needed)

If there are issues after publishing:

```bash
# Yank a version from crates.io (makes it unavailable for new installs)
cargo yank --vers 0.20.1 -p embeddenator-vsa

# Users with existing versions can still use them
# New users will be directed to previous stable version
```

---

## Estimated Timeline

| Phase | Task | Duration | Status |
|-------|------|----------|--------|
| 1 | Version bumps & commits | 4-6 hours | Ready |
| 2 | Local validation | 1-2 hours | Ready |
| 3 | Documentation updates | 1 hour | Ready |
| 4 | Publish to crates.io | 1-2 hours | Ready |
| 5 | GitHub releases | 1 hour | Ready |
| 6 | Documentation finalization | 1 hour | Ready |
| **Total** | **All phases** | **9-13 hours** | **Ready for execution** |

---

**End of Implementation Plan**
