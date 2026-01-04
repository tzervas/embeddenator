# Local Development Guide

**Version:** 0.20.0  
**Last Updated:** January 4, 2026

## Overview

This guide covers **local development workflows** for working across multiple Embeddenator components simultaneously. It focuses on:

- Setting up a multi-repo workspace
- Using `[patch.crates-io]` for local path dependencies
- Development iteration patterns
- Testing strategies
- Pre-release validation

## Prerequisites

- **Rust:** 1.84 or later (`rustup update`)
- **Git:** 2.40+ recommended
- **Disk Space:** ~2GB for all component repos + build artifacts
- **Optional:** FUSE libraries for `embeddenator-fs` development

## Workspace Setup

### Directory Structure

Clone all component repos into a common parent directory:

```bash
mkdir ~/embeddenator-workspace
cd ~/embeddenator-workspace

# Clone core orchestrator
git clone https://github.com/tzervas/embeddenator

# Clone components (libraries)
git clone https://github.com/tzervas/embeddenator-vsa
git clone https://github.com/tzervas/embeddenator-io
git clone https://github.com/tzervas/embeddenator-retrieval
git clone https://github.com/tzervas/embeddenator-fs
git clone https://github.com/tzervas/embeddenator-interop
git clone https://github.com/tzervas/embeddenator-obs

# Clone tools (optional)
git clone https://github.com/tzervas/embeddenator-testkit
git clone https://github.com/tzervas/embeddenator-contract-bench
git clone https://github.com/tzervas/embeddenator-workspace
```

**Final structure:**
```
~/embeddenator-workspace/
├── embeddenator/                    (core)
├── embeddenator-vsa/                (components)
├── embeddenator-io/
├── embeddenator-retrieval/
├── embeddenator-fs/
├── embeddenator-interop/
├── embeddenator-obs/
├── embeddenator-testkit/            (tools)
├── embeddenator-contract-bench/
└── embeddenator-workspace/
```

### Verify Setup

Test that all repos are healthy:

```bash
cd ~/embeddenator-workspace

for repo in embeddenator-*; do
  echo "Testing $repo..."
  (cd "$repo" && cargo check 2>&1 | grep -E "(Compiling|Finished|error)" | head -5)
done

echo "Testing embeddenator core..."
(cd embeddenator && cargo check 2>&1 | grep -E "(Compiling|Finished|error)" | head -5)
```

Expected output: `Finished dev [unoptimized + debuginfo] target(s)` for each repo.

## Using [patch.crates-io]

### What is [patch.crates-io]?

Cargo's `[patch]` mechanism allows **temporary override** of dependencies. When you specify:

```toml
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
```

Cargo will:
1. Use the **local path** version instead of the git tag
2. Apply this override **transitively** (all crates in the workspace)
3. Ignore version mismatches (uses whatever is in the local path)

**Critical:** `[patch.crates-io]` is for **development only**. Never commit it to production code.

### When to Use [patch.crates-io]

✅ **Use when:**
- Developing features spanning multiple components
- Debugging cross-component issues
- Testing API changes before releasing
- Rapid iteration with immediate feedback

❌ **Don't use when:**
- Preparing for release (must test with git tags)
- Code review (reviewers need reproducible builds)
- CI/CD pipelines (patches break reproducibility)
- Single-component changes (work directly in that repo)

### Adding [patch.crates-io]

**Option 1: Workspace-level (recommended for core development)**

In `embeddenator/Cargo.toml` (root), add at the bottom:

```toml
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
embeddenator-fs = { path = "../embeddenator-fs" }
embeddenator-interop = { path = "../embeddenator-interop" }
embeddenator-obs = { path = "../embeddenator-obs" }
```

**Option 2: Component-level (for testing component changes in isolation)**

In a component's `Cargo.toml` (e.g., `embeddenator-retrieval/Cargo.toml`):

```toml
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
```

### Removing [patch.crates-io]

Before committing or releasing:

```bash
cd embeddenator

# Option 1: Comment out (preserves setup)
sed -i '/\[patch.crates-io\]/,/^$/s/^/# /' Cargo.toml

# Option 2: Delete entirely
# Edit Cargo.toml and remove [patch.crates-io] section

# Verify it's gone
grep -A 10 "\[patch.crates-io\]" Cargo.toml || echo "Patches removed ✓"

# Update to use git tags again
cargo update
cargo build --release
cargo test --all
```

## Development Workflows

### Workflow 1: Single Component Change

**Scenario:** Fix a bug in `embeddenator-vsa`, test in core.

```bash
# 1. Work in component repo
cd ~/embeddenator-workspace/embeddenator-vsa
git checkout -b fix/cosine-precision

# Make changes to src/similarity.rs
vim src/similarity.rs

# Test locally
cargo test

# 2. Test in core with [patch.crates-io]
cd ../embeddenator

# Add patch (if not already present)
echo '
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
' >> Cargo.toml

# Test integration
cargo test --all

# 3. Release component
cd ../embeddenator-vsa
git add src/similarity.rs
git commit -m "Fix cosine similarity precision loss"
git tag -a v0.1.1 -m "v0.1.1: Precision fix"
git push origin main --tags

# 4. Update core to use new tag
cd ../embeddenator

# Remove patch
sed -i '/\[patch.crates-io\]/,/^$/d' Cargo.toml

# Update dependency
vim Cargo.toml  # Change embeddenator-vsa tag = "v0.1.1"
cargo update -p embeddenator-vsa

# Test with git tag
cargo test --all

# Commit core update
git commit -am "Update embeddenator-vsa to v0.1.1"
git push origin main
```

### Workflow 2: Cross-Component Feature

**Scenario:** Add new query algorithm affecting vsa, io, and retrieval.

```bash
# 1. Branch all affected repos
cd ~/embeddenator-workspace
for repo in embeddenator-vsa embeddenator-io embeddenator-retrieval embeddenator; do
  (cd "$repo" && git checkout -b feat/semantic-search)
done

# 2. Enable local paths in core
cd embeddenator
cat >> Cargo.toml <<'EOF'

[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
EOF

# 3. Develop iteratively
cd ../embeddenator-vsa
# Add semantic distance metric
vim src/similarity.rs
cargo test

cd ../embeddenator-io
# Add metadata for semantic queries
vim src/manifest.rs
cargo test

cd ../embeddenator-retrieval
# Implement semantic search
vim src/semantic.rs
cargo test

cd ../embeddenator
# Wire up CLI interface
vim src/cli/query.rs
cargo test --all

# 4. Pre-release validation (remove patches, test with tags)
cd ~/embeddenator-workspace/embeddenator
sed -i '/\[patch.crates-io\]/,/^$/d' Cargo.toml

# This will FAIL because new versions aren't tagged yet - that's expected!
cargo build 2>&1 | grep "error"

# 5. Release in dependency order
cd ../embeddenator-vsa
git push origin feat/semantic-search
# Create PR, merge to main
git checkout main && git pull
git tag -a v0.2.0 -m "v0.2.0: Semantic distance metric"
git push origin --tags

cd ../embeddenator-io
# Update vsa dependency to v0.2.0
vim Cargo.toml
cargo update -p embeddenator-vsa
git commit -am "Update embeddenator-vsa to v0.2.0"
git push origin feat/semantic-search
# Create PR, merge to main
git checkout main && git pull
git tag -a v0.2.0 -m "v0.2.0: Semantic metadata"
git push origin --tags

cd ../embeddenator-retrieval
# Update dependencies to v0.2.0
vim Cargo.toml
cargo update -p embeddenator-vsa -p embeddenator-io
git commit -am "Update dependencies to v0.2.0"
git push origin feat/semantic-search
# Create PR, merge to main
git checkout main && git pull
git tag -a v0.2.0 -m "v0.2.0: Semantic search engine"
git push origin --tags

cd ../embeddenator
# Update all dependencies to v0.2.0
vim Cargo.toml
cargo update
cargo test --all
git commit -am "Add semantic search (vsa v0.2.0, io v0.2.0, retrieval v0.2.0)"
git push origin feat/semantic-search
# Create PR, merge to main
```

### Workflow 3: Rapid Prototyping

**Scenario:** Experiment with API changes without git overhead.

```bash
# 1. Set up persistent patches
cd ~/embeddenator-workspace/embeddenator
cat > Cargo.local.toml <<'EOF'
# Local development patches - DO NOT COMMIT
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
embeddenator-fs = { path = "../embeddenator-fs" }
embeddenator-interop = { path = "../embeddenator-interop" }
embeddenator-obs = { path = "../embeddenator-obs" }
EOF

# Link into main Cargo.toml
echo '
# Local development overrides (see Cargo.local.toml)
include = "Cargo.local.toml"
' >> Cargo.toml

# Add to .gitignore
echo "Cargo.local.toml" >> .gitignore

# 2. Develop freely
cd ../embeddenator-vsa
# Try breaking API change
vim src/lib.rs

cd ../embeddenator
cargo test --all  # Instant feedback!

# 3. When done, commit components first
cd ../embeddenator-vsa
git commit -am "Refactor: Simplify SparseVec API"
git tag v0.2.0 && git push origin main --tags

cd ../embeddenator
# Remove include line
vim Cargo.toml  # Delete 'include = "Cargo.local.toml"'
cargo test --all
git commit -am "Update to embeddenator-vsa v0.2.0"
```

## Testing Strategies

### Unit Tests (Per Component)

Test each component in isolation:

```bash
cd ~/embeddenator-workspace/embeddenator-vsa
cargo test
cargo test --doc  # Doc tests
cargo test --release  # Optimized builds (slower, more realistic)
```

### Integration Tests (Cross-Component)

Test component interactions:

```bash
cd ~/embeddenator-workspace/embeddenator
cargo test --test integration_retrieval  # Tests vsa + io + retrieval
cargo test --test integration_fs         # Tests fs + io + vsa
```

### Contract Tests (API Stability)

Validate component contracts:

```bash
cd ~/embeddenator-workspace/embeddenator-contract-bench
cargo bench --no-run  # Compile without running
cargo bench           # Run and measure
```

### E2E Tests (Full Pipeline)

Test complete workflows:

```bash
cd ~/embeddenator-workspace/embeddenator
cargo test --test e2e -- --test-threads=1 --nocapture
```

### Test with Local Paths

```bash
cd ~/embeddenator-workspace/embeddenator

# Add patches
echo '
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
' >> Cargo.toml

# Run full test suite
cargo test --all --all-features

# Check for warnings
cargo clippy --all-targets --all-features -- -D warnings

# Build docs
cargo doc --no-deps --all-features
```

### Test with Git Tags (Pre-Release)

```bash
cd ~/embeddenator-workspace/embeddenator

# Remove patches
sed -i '/\[patch.crates-io\]/,/^$/d' Cargo.toml

# Clean and rebuild
cargo clean
cargo build --release
cargo test --all --release

# Test installation
cargo install --path . --force
embeddenator --version
```

## Common Issues

### Issue 1: "Failed to resolve patches"

**Error:**
```
error: failed to resolve patches for `https://github.com/rust-lang/crates.io-index`
Caused by: patch for `embeddenator-vsa` in `https://github.com/rust-lang/crates.io-index` points to the same source
```

**Cause:** Component is specified as both a git dependency AND a path patch, but versions don't match.

**Fix:**
```bash
# Option 1: Remove version constraint in [dependencies]
[dependencies]
embeddenator-vsa = { git = "..." }  # No tag = accepts any version

# Option 2: Update path version to match tag
cd ../embeddenator-vsa
vim Cargo.toml  # Set version = "0.1.1"
```

### Issue 2: "No such file or directory"

**Error:**
```
error: failed to load source for dependency `embeddenator-vsa`
Caused by: Unable to update file:///home/user/embeddenator-workspace/embeddenator-vsa
```

**Cause:** Path in `[patch.crates-io]` is incorrect.

**Fix:**
```bash
# Check actual location
ls -la ../embeddenator-vsa

# Update Cargo.toml with correct relative path
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }  # From embeddenator/ dir
```

### Issue 3: Changes not reflected

**Error:** Code changes in component don't appear in core builds.

**Cause:** Cargo cache not invalidated.

**Fix:**
```bash
# Force clean rebuild
cd ~/embeddenator-workspace/embeddenator
cargo clean
rm -rf target/
cargo build

# Or use touch to force recompilation
cd ../embeddenator-vsa
touch src/lib.rs
cd ../embeddenator
cargo build
```

### Issue 4: Clippy warnings differ

**Error:** Clippy passes in component but fails in core (or vice versa).

**Cause:** Different Rust toolchains or clippy versions.

**Fix:**
```bash
# Standardize toolchain
rustup update
rustup default stable

# Run clippy consistently
cd ~/embeddenator-workspace/embeddenator-vsa
cargo clippy --all-targets -- -D warnings

cd ../embeddenator
cargo clippy --all-targets -- -D warnings
```

## Pre-Release Checklist

Before releasing any component:

- [ ] **Remove `[patch.crates-io]`** from all Cargo.toml files
- [ ] **Update git tag dependencies** to new versions
- [ ] **Run full test suite:**
  ```bash
  cargo clean
  cargo test --all --all-features --release
  ```
- [ ] **Check for warnings:**
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- [ ] **Build docs:**
  ```bash
  cargo doc --no-deps --all-features
  ```
- [ ] **Verify version numbers:**
  ```bash
  grep '^version' Cargo.toml
  git tag -l | tail -1
  ```
- [ ] **Test installation:**
  ```bash
  cargo install --path . --force
  embeddenator --version
  ```
- [ ] **Update CHANGELOG.md** with release notes
- [ ] **Commit, tag, and push:**
  ```bash
  git commit -am "Release v0.X.Y"
  git tag -a v0.X.Y -m "v0.X.Y: Summary"
  git push origin main --tags
  ```

## Advanced: Scripted Workflows

### Script: Update All Components

```bash
#!/usr/bin/env bash
# update-all.sh - Update all component repos to latest main

set -euo pipefail

WORKSPACE=~/embeddenator-workspace
COMPONENTS=(
  embeddenator-vsa
  embeddenator-io
  embeddenator-retrieval
  embeddenator-fs
  embeddenator-interop
  embeddenator-obs
  embeddenator-testkit
  embeddenator-contract-bench
  embeddenator-workspace
)

for comp in "${COMPONENTS[@]}"; do
  echo "Updating $comp..."
  (
    cd "$WORKSPACE/$comp"
    git checkout main
    git pull --tags
    cargo update
  ) || {
    echo "⚠️  Failed to update $comp (skipping)"
  }
done

echo "✓ All components updated"
```

### Script: Enable/Disable Patches

```bash
#!/usr/bin/env bash
# patches.sh - Toggle [patch.crates-io] in embeddenator core

set -euo pipefail

CARGO_TOML=~/embeddenator-workspace/embeddenator/Cargo.toml
PATCH_MARKER="# LOCAL_DEV_PATCHES"

enable_patches() {
  if grep -q "$PATCH_MARKER" "$CARGO_TOML"; then
    echo "Patches already enabled"
    return
  fi
  
  cat >> "$CARGO_TOML" <<EOF

$PATCH_MARKER
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
embeddenator-fs = { path = "../embeddenator-fs" }
embeddenator-interop = { path = "../embeddenator-interop" }
embeddenator-obs = { path = "../embeddenator-obs" }
EOF
  
  echo "✓ Patches enabled"
}

disable_patches() {
  if ! grep -q "$PATCH_MARKER" "$CARGO_TOML"; then
    echo "Patches already disabled"
    return
  fi
  
  sed -i "/$PATCH_MARKER/,\$d" "$CARGO_TOML"
  echo "✓ Patches disabled"
}

case "${1:-}" in
  on|enable)
    enable_patches
    ;;
  off|disable)
    disable_patches
    ;;
  *)
    echo "Usage: $0 {on|off}"
    exit 1
    ;;
esac
```

**Usage:**
```bash
chmod +x patches.sh
./patches.sh on   # Enable local development
./patches.sh off  # Prepare for release
```

## See Also

- [COMPONENT_ARCHITECTURE.md](COMPONENT_ARCHITECTURE.md) - Architecture overview
- [VERSIONING.md](VERSIONING.md) - Versioning strategy
- [Cargo Book: Overriding Dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)
