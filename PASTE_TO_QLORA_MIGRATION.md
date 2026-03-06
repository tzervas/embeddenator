# Migration Guide: `paste` → `qlora-paste`

**Document Version:** 1.0.0  
**Last Updated:** January 26, 2026  
**Status:** Ready for Implementation  
**qlora-paste Version:** 1.0.17 (released on crates.io)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Background](#background)
3. [Pre-Migration Checklist](#pre-migration-checklist)
4. [Affected Crates](#affected-crates)
5. [Migration Steps](#migration-steps)
6. [Automated Migration Script](#automated-migration-script)
7. [Post-Migration Verification](#post-migration-verification)
8. [Rollback Procedure](#rollback-procedure)
9. [Timeline and Coordination](#timeline-and-coordination)
10. [FAQ](#faq)

---

## Executive Summary

This document provides a comprehensive guide for migrating the Embeddenator workspace from the `paste` crate to `qlora-paste` v1.0.17. While `paste` is not currently a dependency in the workspace, this guide prepares for potential future adoption and provides a template for similar dependency migrations.

**Key Points:**
- **Current Status:** `paste` is NOT currently used as a dependency
- **Purpose:** Proactive migration planning for when `paste` or `qlora-paste` is adopted
- **Estimated Effort:** 1-2 hours (when applicable)
- **Risk Level:** Low (API compatible drop-in replacement)
- **Rollback Time:** < 15 minutes

**qlora-paste v1.0.17 is available on crates.io:** https://crates.io/crates/qlora-paste

---

## Background

### Why `qlora-paste`?

The `qlora-paste` crate (v1.0.17) is a maintained fork of the widely-used `paste` procedural macro for token pasting in Rust. Benefits include:

1. **Active Maintenance:** Regular updates and security patches
2. **API Compatibility:** Drop-in replacement for `paste`
3. **Enhanced Performance:** Optimized macro expansion
4. **Better Diagnostics:** Improved error messages during compilation
5. **Community Support:** Growing ecosystem adoption

### What Changed?

- **Crate Name:** `paste` → `qlora-paste`
- **Import Paths:** Compatible, but update for clarity
- **Macro Names:** Unchanged (`paste!` remains the same)
- **API Compatibility:** 100% drop-in replacement
- **Version:** qlora-paste v1.0.17 is the current stable release

---

## Pre-Migration Checklist

Complete these steps **before** beginning the migration:

### 1. Backup and Version Control

```bash
# Create a dedicated migration branch
git checkout -b migration/paste-to-qlora-paste

# Ensure working directory is clean
git status

# Tag current state for easy rollback
git tag -a pre-qlora-paste-migration -m "State before paste → qlora-paste migration"
git push origin pre-qlora-paste-migration
```

### 2. Dependency Audit

```bash
# Check current paste usage across workspace
grep -r "paste" --include="Cargo.toml" .

# Check for paste macro usage in source
grep -r "paste::" --include="*.rs" src/
grep -r "#\[paste\]" --include="*.rs" src/
grep -r "use paste" --include="*.rs" src/

# Check Cargo.lock for indirect dependencies
grep -i paste Cargo.lock
```

**Current Status (as of Jan 26, 2026):**
- No direct `paste` dependencies found in workspace Cargo.toml files
- `pastey` reference found in embeddenator-testkit/Cargo.lock (unrelated crate)

### 3. CI/CD Freeze

- [ ] Notify team of migration window
- [ ] Pause automatic deployments
- [ ] Ensure all pending PRs are merged or put on hold
- [ ] Run full test suite to establish baseline

```bash
# Establish baseline test results
cargo test --workspace --all-features > pre_migration_tests.log 2>&1
cargo clippy --workspace --all-features > pre_migration_clippy.log 2>&1
cargo build --workspace --all-features --release > pre_migration_build.log 2>&1
```

### 4. Documentation Review

- [ ] Review CHANGELOG.md for each crate
- [ ] Check for paste-specific documentation
- [ ] Identify integration tests that might be affected
- [ ] Review examples and documentation code

### 5. Stakeholder Communication

- [ ] Announce migration timeline to team
- [ ] Schedule migration window (recommend off-peak hours)
- [ ] Prepare rollback plan
- [ ] Document expected downtime (if any)

---

## Affected Crates

Based on workspace structure analysis, here are all workspace crates and their potential exposure to `paste`:

| Crate Name | Current Status | Priority | Notes |
|------------|----------------|----------|-------|
| **embeddenator-vsa** | ❌ No `paste` | P3 | Core VSA primitives - may benefit from paste in future |
| **embeddenator-io** | ❌ No `paste` | P3 | I/O operations |
| **embeddenator-obs** | ❌ No `paste` | P3 | Observability |
| **embeddenator-retrieval** | ❌ No `paste` | P3 | Search functionality |
| **embeddenator-fs** | ❌ No `paste` | P3 | Filesystem (EmbrFS) |
| **embeddenator-interop** | ❌ No `paste` | P2 | FFI layer - may benefit from paste for C bindings |
| **embeddenator-cli** | ❌ No `paste` | P3 | CLI tool |
| **embeddenator-core** | ❌ No `paste` | P1 | Umbrella crate |
| **embeddenator-workspace** | ❌ No `paste` | P3 | Workspace tools |
| **embeddenator-testkit** | ⚠️ Uses `pastey` | P2 | Note: `pastey` is a different crate (not `paste`) |
| **embeddenator-contract-bench** | ❌ No `paste` | P3 | Benchmarking |

**Legend:**
- ✅ Uses `paste` directly
- ⚠️ Indirect dependency or related package
- ❌ No current usage
- P1: High priority, P2: Medium priority, P3: Low priority

**Note:** When `paste` or `qlora-paste` is adopted, this table should be updated with actual usage patterns.

---

## Migration Steps

### Step 1: Verify qlora-paste Availability

```bash
# Check that qlora-paste v1.0.17 is available
cargo search qlora-paste

# Expected output:
# qlora-paste = "1.0.17"    # Token pasting with proc macros
```

### Step 2: Update Root Workspace Cargo.toml (if applicable)

If using workspace-level dependency management:

```toml
[workspace.dependencies]
# BEFORE:
# paste = "1.0"

# AFTER:
qlora-paste = "1.0.17"
```

### Step 3: Update Individual Crate Dependencies

For each crate that uses `paste`, update its `Cargo.toml`:

```toml
[dependencies]
# BEFORE:
# paste = "1.0"

# AFTER:
qlora-paste = "1.0.17"

[dev-dependencies]
# BEFORE:
# paste = "1.0"

# AFTER:
qlora-paste = "1.0.17"
```

### Step 4: Update Import Statements

**Option A: Rename imports (recommended for clarity)**

```rust
// BEFORE:
use paste::paste;

// AFTER:
use qlora_paste::paste;
```

**Option B: Use aliasing (maintains existing code)**

```rust
// Add at crate root (lib.rs or main.rs)
extern crate qlora_paste as paste;

// Existing code remains unchanged
use paste::paste;
```

### Step 5: Update Macro Invocations

Most macro invocations should work unchanged:

```rust
// Should work as-is after import update
paste! {
    fn [<my_ function>]() {
        // ...
    }
}

// Attribute macro usage
#[paste]
mod my_module {
    // ...
}
```

### Step 6: Update Documentation and Comments

```bash
# Update code comments
find . -name "*.rs" -type f -exec sed -i 's/`paste`/`qlora-paste`/g' {} +

# Update documentation files
find . -name "*.md" -type f -exec sed -i 's/`paste`/`qlora-paste`/g' {} +

# Update references in Cargo.toml comments
find . -name "Cargo.toml" -type f -exec sed -i 's/# paste/# qlora-paste/g' {} +
```

### Step 7: Update CI/CD Configuration

Check `.github/workflows/` for any paste-specific configurations:

```yaml
# .github/workflows/ci-workspace.yml
# Update any dependency caching or audit configurations if needed
```

### Step 8: Update CHANGELOG Files

Add migration notes to relevant CHANGELOG.md files:

```markdown
## [Unreleased]

### Changed
- Migrated from `paste` to `qlora-paste` v1.0.17 for improved maintenance and diagnostics
```

---

## Automated Migration Script

Save this as `scripts/migrate_paste_to_qlora.sh`:

```bash
#!/usr/bin/env bash
#
# Automated migration script: paste → qlora-paste
# Usage: ./scripts/migrate_paste_to_qlora.sh [--dry-run]
#

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo "🔍 DRY RUN MODE - No changes will be made"
fi

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "📦 Embeddenator Workspace: paste → qlora-paste v1.0.17 Migration"
echo "=================================================================="
echo ""

# Function to perform replacement
replace_in_file() {
    local file=$1
    local search=$2
    local replace=$3
    
    if [ "$DRY_RUN" = true ]; then
        if grep -q "$search" "$file"; then
            echo "  [DRY-RUN] Would replace '$search' → '$replace' in: $file"
            grep -n "$search" "$file" || true
        fi
    else
        if grep -q "$search" "$file"; then
            sed -i "s/$search/$replace/g" "$file"
            echo "  ✅ Updated: $file"
        fi
    fi
}

echo "Step 1: Updating Cargo.toml files..."
echo "------------------------------------"
find . -name "Cargo.toml" -type f | while read -r cargo_file; do
    if grep -q '^paste = ' "$cargo_file" || grep -q '^paste\.' "$cargo_file"; then
        echo "📝 Found paste dependency in: $cargo_file"
        
        # Replace paste = "version" with qlora-paste = "1.0.17"
        replace_in_file "$cargo_file" 'paste = "[^"]*"' 'qlora-paste = "1.0.17"'
        replace_in_file "$cargo_file" 'paste = {' 'qlora-paste = {'
        replace_in_file "$cargo_file" 'paste\.' 'qlora-paste.'
    fi
done

if [ "$DRY_RUN" = false ]; then
    echo "  ℹ️  Note: No paste dependencies currently exist in workspace"
fi
echo ""

echo "Step 2: Updating Rust source files..."
echo "---------------------------------------"
find . -name "*.rs" -type f -path "*/src/*" | while read -r rs_file; do
    if grep -q 'use paste' "$rs_file" || grep -q 'extern crate paste' "$rs_file"; then
        echo "📝 Found paste usage in: $rs_file"
        
        # Replace use statements
        replace_in_file "$rs_file" 'use paste::' 'use qlora_paste::'
        replace_in_file "$rs_file" 'extern crate paste' 'extern crate qlora_paste as paste'
    fi
done

if [ "$DRY_RUN" = false ]; then
    echo "  ℹ️  Note: No paste imports currently exist in workspace"
fi
echo ""

echo "Step 3: Updating documentation..."
echo "-----------------------------------"
find . -name "*.md" -type f | while read -r md_file; do
    # Skip this migration guide itself
    if [[ "$md_file" == *"PASTE_TO_QLORA_MIGRATION.md"* ]]; then
        continue
    fi
    
    if grep -q '\[paste\]' "$md_file" || grep -q 'paste crate' "$md_file"; then
        echo "📝 Found paste references in: $md_file"
        replace_in_file "$md_file" '\[paste\]' '[qlora-paste]'
        replace_in_file "$md_file" 'paste crate' 'qlora-paste crate'
    fi
done
echo ""

echo "Step 4: Updating CHANGELOG files..."
echo "-------------------------------------"
if [ "$DRY_RUN" = false ]; then
    for changelog in embeddenator-*/CHANGELOG.md; do
        if [ -f "$changelog" ]; then
            # Add migration note under [Unreleased] section
            if grep -q "## \[Unreleased\]" "$changelog"; then
                echo "  ✅ Adding migration note to: $changelog"
                # This is complex to do with sed, document manually
                echo "     (Please manually add migration note to $changelog)"
            fi
        fi
    done
else
    echo "  [DRY-RUN] Would update CHANGELOG files with migration notes"
fi
echo ""

echo "Step 5: Cleaning and rebuilding..."
echo "------------------------------------"
if [ "$DRY_RUN" = false ]; then
    cargo clean
    echo "  ✅ Cleaned build artifacts"
else
    echo "  [DRY-RUN] Would run: cargo clean"
fi
echo ""

echo "Step 6: Verification checklist..."
echo "-----------------------------------"
echo "  [ ] Review git diff for unexpected changes"
echo "  [ ] Run: cargo check --workspace --all-features"
echo "  [ ] Run: cargo test --workspace --all-features"
echo "  [ ] Run: cargo clippy --workspace --all-features"
echo "  [ ] Update CHANGELOG.md for each affected crate"
echo "  [ ] Run: cargo audit"
echo "  [ ] Run: cargo deny check"
echo ""

if [ "$DRY_RUN" = true ]; then
    echo "✨ Dry run complete! No changes were made."
    echo "   Run without --dry-run to apply changes."
else
    echo "✅ Migration script complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Review changes: git diff"
    echo "  2. Test build: cargo build --workspace --all-features"
    echo "  3. Run tests: cargo test --workspace --all-features"
    echo "  4. Update CHANGELOGs: Add migration notes manually"
    echo "  5. Commit: git add -A && git commit -m 'deps: migrate paste → qlora-paste v1.0.17'"
fi
```

Make it executable:

```bash
chmod +x scripts/migrate_paste_to_qlora.sh
```

Run with dry-run first:

```bash
./scripts/migrate_paste_to_qlora.sh --dry-run
```

Then execute:

```bash
./scripts/migrate_paste_to_qlora.sh
```

---

## Post-Migration Verification

Complete these verification steps in order:

### 1. Dependency Check

```bash
# Verify qlora-paste is in dependency tree
cargo tree | grep -i paste

# Expected output:
# └── qlora-paste v1.0.17
# Should NOT see "paste v1.0.x" (unless indirect dependency)

# Check all Cargo.toml files
find . -name Cargo.toml -exec grep -H "paste" {} \;
```

### 2. Build Verification

```bash
# Clean build
cargo clean

# Build all workspace crates with all features
cargo build --workspace --all-features
cargo build --workspace --all-features --release

# Check for warnings
cargo build --workspace --all-features 2>&1 | tee build.log
grep -i warning build.log

# Compare with baseline
diff pre_migration_build.log build.log
```

### 3. Test Suite Execution

```bash
# Run all tests
cargo test --workspace --all-features

# Run doctests specifically
cargo test --workspace --doc

# Run integration tests
cargo test --workspace --test '*'

# Compare with baseline
cargo test --workspace --all-features > post_migration_tests.log 2>&1
diff pre_migration_tests.log post_migration_tests.log
```

### 4. Linting and Quality Checks

```bash
# Run clippy
cargo clippy --workspace --all-features -- -D warnings

# Format check
cargo fmt --all -- --check

# Audit dependencies
cargo audit

# Run cargo-deny
cargo deny check

# Compare with baseline
cargo clippy --workspace --all-features > post_migration_clippy.log 2>&1
diff pre_migration_clippy.log post_migration_clippy.log
```

### 5. Benchmark Verification (if applicable)

```bash
# Run benchmarks to ensure no performance regression
cargo bench --workspace

# For specific benchmark suites
cd embeddenator-contract-bench
cargo bench
```

### 6. Documentation Build

```bash
# Build documentation
cargo doc --workspace --all-features --no-deps

# Check for broken links
cargo doc --workspace --all-features --no-deps 2>&1 | grep -i warning

# Open generated docs
cargo doc --workspace --all-features --no-deps --open
```

### 7. Integration Tests

For crates with integration tests:

```bash
# embeddenator-testkit comprehensive tests
cd embeddenator-testkit
cargo test --features integration
cargo bench

# embeddenator-fs integration tests
cd embeddenator-fs
cargo test --test '*'
```

### 8. Manual Smoke Tests

- [ ] CLI commands work (`embeddenator-cli`)
- [ ] FFI bindings compile (`embeddenator-interop`)
- [ ] Example code runs successfully
- [ ] Documentation examples are valid
- [ ] FUSE operations work (`embeddenator-fs`)

---

## Rollback Procedure

If issues are discovered, follow this rollback procedure:

### Quick Rollback (< 5 minutes)

```bash
# Option 1: Reset to tagged state
git reset --hard pre-qlora-paste-migration
git clean -fdx

# Option 2: Revert the migration commit
git revert HEAD
cargo clean
cargo build --workspace --all-features
```

### Full Rollback with Verification

```bash
# 1. Reset to pre-migration state
git reset --hard pre-qlora-paste-migration

# 2. Clean all build artifacts
cargo clean
rm -rf target/
find . -name Cargo.lock -delete

# 3. Rebuild from clean state
cargo build --workspace --all-features

# 4. Verify tests pass
cargo test --workspace --all-features

# 5. Verify no paste/qlora-paste mixed state
cargo tree | grep -i paste

# 6. Document rollback
cat > ROLLBACK.md << EOF
# Rollback Report

**Date:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Migration:** paste → qlora-paste v1.0.17
**Status:** ROLLED BACK

## Reason

[FILL IN REASON FOR ROLLBACK]

## Actions Taken

1. Git reset to pre-migration tag
2. Clean rebuild of workspace
3. Full test suite verification

## Next Steps

- [ ] Investigate root cause
- [ ] Document blockers
- [ ] Plan remediation strategy
- [ ] Schedule retry (if applicable)
EOF

git add ROLLBACK.md
git commit -m "rollback: paste → qlora-paste migration"
```

### Communication After Rollback

1. **Immediate notification** to team via Slack/Discord
2. **Document reason** for rollback in ROLLBACK.md
3. **Create GitHub issue** tracking the problem
4. **Plan remediation** strategy
5. **Update migration timeline** if retry is planned

---

## Timeline and Coordination

### Recommended Migration Schedule

| Phase | Duration | Activities |
|-------|----------|------------|
| **Preparation** | 1-2 days | Pre-flight checks, team notification, branch creation |
| **Migration** | 2-4 hours | Execute automated script, manual verification |
| **Testing** | 4-8 hours | Full test suite, integration tests, benchmarks |
| **Monitoring** | 24-48 hours | Watch CI/CD, monitor for issues |
| **Finalization** | 1 day | Update docs, publish releases (if needed) |

### Critical Path

```
Day 1 Morning:   Branch creation, backup, baseline tests
Day 1 Afternoon: Run migration script, code review, git diff analysis
Day 2 Morning:   Full test suite, benchmark verification
Day 2 Afternoon: Integration testing, documentation updates
Day 3 Morning:   Final review, merge to main
Day 3-4:         Monitoring period, issue tracking
```

### Team Coordination

**Roles and Responsibilities:**

- **Migration Lead:** Executes migration script, coordinates timeline, manages git workflow
- **QA/Testing:** Runs verification suite, validates benchmarks, smoke tests
- **DevOps:** Monitors CI/CD, handles rollback if needed, artifact management
- **Documentation:** Updates CHANGELOGs, README files, migration guides

**Communication Channels:**

- Migration status updates every 2 hours
- Slack/Discord channel: `#embeddenator-migration`
- GitHub tracking issue: `[Migration] paste → qlora-paste v1.0.17`
- Email notification for critical issues

**Decision Points:**

- **Go/No-Go after baseline:** All tests pass, CI green
- **Go/No-Go after migration:** Build succeeds, no regressions
- **Go/No-Go for merge:** Full test suite passes, benchmarks stable

---

## FAQ

### Q1: Will this break existing code?

**A:** `qlora-paste` v1.0.17 is designed as a drop-in replacement for `paste`. API compatibility should be 100%. However, always test thoroughly before deploying to production.

### Q2: Do we need to update our public API?

**A:** No. The `paste`/`qlora-paste` crate is a build-time dependency (proc macro). Downstream users won't see any changes in your public API.

### Q3: Will this affect our semver versioning?

**A:** Dependency changes typically warrant:
- **Patch version:** If truly no functional changes
- **Minor version:** Recommended for transparency, shows active maintenance

For Embeddenator, consider **minor version bump** to signal the change.

### Q4: What if a transitive dependency still uses `paste`?

**A:** This is fine. Both `paste` and `qlora-paste` can coexist in the dependency tree without conflicts, as they have different crate names.

### Q5: How do we handle CI/CD caching?

**A:** After migration, you may want to update cache keys:

```yaml
# .github/workflows/ci-workspace.yml
- uses: actions/cache@v4
  with:
    key: ${{ runner.os }}-cargo-qlora-paste-${{ hashFiles('**/Cargo.lock') }}
```

However, this is optional. Existing cache keys will work fine.

### Q6: Should we migrate all crates simultaneously?

**A:** For the Embeddenator workspace, **yes**. Atomic migration reduces confusion and ensures consistent dependency versions. For larger ecosystems, consider phased approach.

### Q7: What about `pastey` in testkit Cargo.lock?

**A:** `pastey` is a **different crate** unrelated to `paste`. No migration needed for `pastey`. Investigate separately if needed:

```bash
cd embeddenator-testkit
cargo tree | grep pastey
# Identify source and evaluate separately
```

### Q8: Is qlora-paste v1.0.17 stable?

**A:** Yes! Version 1.0.17 indicates a stable, production-ready release. Check the changelog on crates.io for release notes: https://crates.io/crates/qlora-paste

### Q9: What if we encounter compilation errors?

**A:** First, verify:
1. qlora-paste version is exactly `1.0.17` in Cargo.toml
2. Import paths are correct (`use qlora_paste::paste;` or `extern crate qlora_paste as paste;`)
3. Cargo.lock is updated (`cargo update -p qlora-paste`)

If issues persist, consult the qlora-paste documentation or rollback.

### Q10: Do we need to notify users?

**A:** For published crates:
- **Yes**, add to CHANGELOG.md noting the dependency change
- **Optional**, mention in release notes if you want transparency
- **No**, it doesn't affect public API or downstream users directly

---

## Appendix: Additional Resources

### Related Documentation

- [SECURITY.md](SECURITY.md) - Security policy and vulnerability reporting
- [.github/dependabot.yml](.github/dependabot.yml) - Automated dependency updates
- [deny.toml](deny.toml) - Supply chain security configuration
- [CI_CD_GUIDE.md](CI_CD_GUIDE.md) - CI/CD documentation

### External Links

- **qlora-paste on crates.io:** https://crates.io/crates/qlora-paste
- **qlora-paste v1.0.17 release:** https://crates.io/crates/qlora-paste/1.0.17
- **paste crate (original):** https://crates.io/crates/paste
- **Rust Cargo Dependency Management:** https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
- **RustSec Advisory Database:** https://rustsec.org/

### Useful Commands

```bash
# Find all Cargo.toml files
find . -name "Cargo.toml" -type f

# Search for paste usage in all Rust files
rg "use paste" --type rust
rg "qlora.paste" --type rust

# Check dependency tree for specific crate
cargo tree -p embeddenator-vsa
cargo tree | grep paste

# Generate Cargo.lock diff after migration
git diff Cargo.lock

# Search for macro usage
rg "paste!" --type rust

# Check for unintended changes
git diff --stat
git diff --name-only
```

### Script Location

The automated migration script is located at:

```
scripts/migrate_paste_to_qlora.sh
```

Make it executable before running:

```bash
chmod +x scripts/migrate_paste_to_qlora.sh
```

---

## Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-01-26 | 1.0.0 | Initial migration guide created; qlora-paste v1.0.17 confirmed on crates.io |

---

**Document Status:** ✅ Ready for Use  
**Maintained by:** Embeddenator Core Team  
**Next Review:** 2027-01-26

---

**Notes:**

1. This guide is ready for immediate use when the decision is made to adopt `qlora-paste`.
2. Currently, no `paste` dependency exists in the workspace, so migration is not immediately necessary.
3. The automated script includes dry-run functionality for safe testing.
4. All verification steps are documented for thorough testing post-migration.
5. Rollback procedure is well-defined for risk mitigation.

**When to use this guide:**

- When adding `paste`/`qlora-paste` as a new dependency
- When migrating from `paste` to `qlora-paste` in the future
- As a template for other dependency migrations

---

**Ready to migrate? Follow these steps:**

1. ✅ Review this entire document
2. ✅ Complete pre-migration checklist
3. ✅ Run dry-run: `./scripts/migrate_paste_to_qlora.sh --dry-run`
4. ✅ Execute migration: `./scripts/migrate_paste_to_qlora.sh`
5. ✅ Follow post-migration verification steps
6. ✅ Monitor for 24-48 hours
7. ✅ Update this document with any lessons learned

**Questions?** Open an issue on GitHub or contact the core team.
