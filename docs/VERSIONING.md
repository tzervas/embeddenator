# Versioning Strategy

**Version:** 0.20.0  
**Last Updated:** January 4, 2026

## Overview

Embeddenator uses **semantic versioning** (SemVer) with **independent component releases** and **git tag dependencies**. This document defines:

- Version numbering scheme
- Release coordination across components
- Git tagging workflow
- Compatibility guarantees
- Docker image tagging

## Semantic Versioning

All Embeddenator packages follow [SemVer 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH

- MAJOR: Incompatible API changes
- MINOR: Backwards-compatible functionality
- PATCH: Backwards-compatible bug fixes
```

### Pre-1.0 Semantics (Current)

For **0.x.y versions** (pre-stable):

- **0.MAJOR.MINOR** - MAJOR increments indicate breaking changes
- Example: `0.1.x` → `0.2.0` = breaking change
- Rapid iteration expected; API not yet stable
- **1.0.0** milestone = API stability commitment

### Post-1.0 Semantics (Future)

For **1.x.y+ versions** (stable):

- **MAJOR** = Breaking changes (1.x → 2.0)
- **MINOR** = New features, backwards-compatible (1.0 → 1.1)
- **PATCH** = Bug fixes only (1.0.0 → 1.0.1)
- Strict API compatibility within major versions

## Package Versioning

### Core Orchestrator (embeddenator)

- **Current:** v0.20.0
- **Git Repo:** [tzervas/embeddenator](https://github.com/tzervas/embeddenator)
- **Release Cadence:** Monthly (feature-driven)
- **Breaking Changes:** Tolerated pre-1.0, documented in CHANGELOG

**Version History:**
- `v0.20.0` - Component architecture refactor (Jan 2026)
- `v0.3.0` - Deterministic hierarchical artifacts (Dec 2025)
- `v0.2.0` - Test runner overhaul (Dec 2025)
- `v0.1.0` - Initial public release

### Components (embeddenator-*)

| Component | Current | Release Cadence | Breaking Change Policy |
|-----------|---------|-----------------|------------------------|
| **embeddenator-vsa** | v0.1.0 | As needed | Pre-1.0: Tolerated with MINOR bump |
| **embeddenator-io** | v0.1.0 | As needed | Pre-1.0: Tolerated with MINOR bump |
| **embeddenator-retrieval** | v0.1.0 | As needed | Pre-1.0: Tolerated with MINOR bump |
| **embeddenator-fs** | v0.1.0 | As needed | Pre-1.0: Tolerated with MINOR bump |
| **embeddenator-interop** | v0.1.0 | As needed | Pre-1.0: Tolerated with MINOR bump |
| **embeddenator-obs** | v0.1.0 | As needed | Pre-1.0: Tolerated with MINOR bump |
| **embeddenator-testkit** | v0.1.1 | As needed | Test utilities (relaxed policy) |
| **embeddenator-contract-bench** | v0.2.1 | As needed | Benchmarks (relaxed policy) |

**Version Ranges:**
- **0.1.x:** Initial API, subject to refinement
- **0.2.x:** First API stabilization pass
- **1.0.0:** Production-ready, API frozen

### Release Coordination

Components release **independently**, but follow these rules:

1. **Dependency Order:** Release dependencies before consumers
   - Example: `embeddenator-vsa` → `embeddenator-io` → `embeddenator-retrieval` → `embeddenator` (core)

2. **Version Pinning:** Consumers specify **exact tags**:
   ```toml
   [dependencies]
   embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.1.0" }
   ```

3. **Major Version Sync:** All components move to 1.0 together
   - Pre-1.0: Independent versioning allowed
   - Post-1.0: Major versions advance in lockstep

## Git Tag Workflow

### Creating a Release

#### 1. Update Version in Cargo.toml

```bash
cd ~/embeddenator-workspace/embeddenator-vsa
vim Cargo.toml

# Change:
# version = "0.1.0"
# To:
# version = "0.1.1"
```

#### 2. Update CHANGELOG.md

```bash
vim CHANGELOG.md

# Add new section:
# ## [0.1.1] - 2026-01-04
# ### Fixed
# - Cosine similarity precision loss in edge cases
```

#### 3. Verify Build

```bash
cargo build --release
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
```

#### 4. Commit and Tag

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "Release v0.1.1: Fix cosine similarity precision"

# Annotated tag (required for proper GitHub release)
git tag -a v0.1.1 -m "v0.1.1: Precision improvements

- Fix precision loss in cosine similarity calculation
- Add regression tests for edge cases
- Performance: 5% faster on average"

# Push commit and tag
git push origin main
git push origin v0.1.1
```

#### 5. Update Consumers

```bash
cd ~/embeddenator-workspace/embeddenator
vim Cargo.toml

# Update dependency:
[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.1.1" }

# Update lockfile
cargo update -p embeddenator-vsa

# Test integration
cargo test --all

# Commit
git commit -am "Update embeddenator-vsa to v0.1.1"
git push origin main
```

### Tagging Best Practices

✅ **Do:**
- Use annotated tags: `git tag -a v0.1.0 -m "Release notes"`
- Include version prefix: `v0.1.0` not `0.1.0`
- Write descriptive tag messages
- Push tags explicitly: `git push origin v0.1.0`
- Verify tag on GitHub: https://github.com/tzervas/embeddenator-vsa/tags

❌ **Don't:**
- Use lightweight tags: `git tag v0.1.0` (no `-a`)
- Forget to push tags: `git push` doesn't push tags by default
- Delete/rewrite tags after pushing (breaks consumers)
- Tag without updating Cargo.toml version
- Skip CHANGELOG updates

### Tag Naming Conventions

| Tag Pattern | Usage | Example |
|-------------|-------|---------|
| `vX.Y.Z` | Stable release | `v0.1.0`, `v1.2.3` |
| `vX.Y.Z-beta.N` | Beta release | `v0.2.0-beta.1` |
| `vX.Y.Z-alpha.N` | Alpha release | `v0.2.0-alpha.1` |
| `vX.Y.Z-rc.N` | Release candidate | `v1.0.0-rc.1` |

**Pre-release order (SemVer):**
```
v1.0.0-alpha.1 < v1.0.0-alpha.2 < v1.0.0-beta.1 < v1.0.0-rc.1 < v1.0.0
```

## Docker Image Tagging

### Registry

All Docker images pushed to **GitHub Container Registry (GHCR)**:
```
ghcr.io/tzervas/embeddenator
```

### Tag Strategy

| Tag | Description | Updates | Example |
|-----|-------------|---------|---------|
| `latest` | Most recent stable release | Every release | `ghcr.io/tzervas/embeddenator:latest` |
| `vX.Y.Z` | Specific version (immutable) | Never | `ghcr.io/tzervas/embeddenator:v0.20.0` |
| `vX.Y` | Latest patch for minor | Every patch | `ghcr.io/tzervas/embeddenator:v0.20` |
| `vX` | Latest minor for major | Every minor | `ghcr.io/tzervas/embeddenator:v0` |
| `vX.Y.Z-ARCH` | Architecture-specific | Every release | `ghcr.io/tzervas/embeddenator:v0.20.0-amd64` |

### Multi-Arch Manifests

Docker images support **linux/amd64** and **linux/arm64**:

```bash
# Pull manifest (auto-selects architecture)
docker pull ghcr.io/tzervas/embeddenator:v0.20.0

# Pull specific architecture
docker pull ghcr.io/tzervas/embeddenator:v0.20.0-amd64
docker pull ghcr.io/tzervas/embeddenator:v0.20.0-arm64

# Inspect manifest
docker manifest inspect ghcr.io/tzervas/embeddenator:v0.20.0
```

### Tagging Workflow

Automated via [.github/workflows/docker-build.yml](../.github/workflows/docker-build.yml):

```yaml
# Triggered on git tags matching v*
on:
  push:
    tags:
      - 'v*'

# Generates tags:
# 1. latest
# 2. vX.Y.Z
# 3. vX.Y
# 4. vX
# 5. vX.Y.Z-amd64, vX.Y.Z-arm64
```

**Manual build:**
```bash
cd ~/Documents/projects/github/embeddenator

# Build for amd64
docker build -f .docker/Dockerfile.embr-ci -t ghcr.io/tzervas/embeddenator:v0.20.0-amd64 .

# Build for arm64 (requires QEMU or arm64 host)
docker buildx build --platform linux/arm64 \
  -f .docker/Dockerfile.embr-ci \
  -t ghcr.io/tzervas/embeddenator:v0.20.0-arm64 .

# Create manifest
python3 .docker/generate_manifest.py v0.20.0

# Push (requires GHCR authentication)
docker push ghcr.io/tzervas/embeddenator:v0.20.0
```

## Compatibility Guarantees

### API Stability (Code)

| Version Range | API Stability | Example |
|---------------|---------------|---------|
| **0.1.x** | Breaking changes in MINOR | `0.1.0` → `0.2.0` may break |
| **1.x.y** | Stable within MAJOR | `1.0.0` → `1.9.9` compatible |
| **2.x.y** | New MAJOR = breaks from 1.x | `1.9.9` → `2.0.0` may break |

**Deprecation Policy (Post-1.0):**
1. Deprecated features marked with `#[deprecated]` in 1.x
2. Kept for at least one minor version (1.0 → 1.1 → 1.2)
3. Removed in next major version (2.0)

### Binary Compatibility (CLI)

| Version Range | CLI Stability | Example |
|---------------|---------------|---------|
| **0.x.y** | Unstable, may change | Flags renamed, output format changes |
| **1.x.y** | Stable, backwards-compatible | New flags only, old flags kept |

**Engram Format Compatibility:**
- Engrams created with v0.x readable by all future v0.x
- Engrams created with v1.x readable by all v1.x and v2.x
- **Migration tools** provided for major version changes

### Docker Image Compatibility

| Tag | Stability | Recommendation |
|-----|-----------|----------------|
| `latest` | Unstable (always newest) | ❌ Avoid in production |
| `vX.Y.Z` | Immutable | ✅ Use in production |
| `vX.Y` | Updates with patches | ⚠️ Use if auto-patching desired |
| `vX` | Updates with features | ❌ Too unstable for production |

## Deprecation Process

### Pre-1.0 (Current)

No formal deprecation. Breaking changes allowed with:
- CHANGELOG entry
- Migration guide in docs
- GitHub release notes

**Example (v0.1.0 → v0.2.0):**
```rust
// REMOVED in v0.2.0 (no deprecation warning)
pub fn old_function() { }

// NEW in v0.2.0
pub fn new_function() { }
```

Users upgrade at their own pace via git tags.

### Post-1.0 (Future)

Formal deprecation with **one minor version notice**:

**Step 1: Deprecate (v1.0.0)**
```rust
#[deprecated(since = "1.1.0", note = "Use `new_function` instead")]
pub fn old_function() { }

pub fn new_function() { }
```

**Step 2: Keep deprecated (v1.1.0)**
```rust
// Still available, warnings emitted
#[deprecated(since = "1.1.0", note = "Use `new_function` instead")]
pub fn old_function() { }
```

**Step 3: Remove (v2.0.0)**
```rust
// Removed entirely (breaking change)
// pub fn old_function() { }  // GONE

pub fn new_function() { }
```

## Version Matrix

### Current Dependencies (v0.20.0)

| Consumer | embeddenator-vsa | embeddenator-io | embeddenator-retrieval | embeddenator-fs |
|----------|------------------|-----------------|------------------------|-----------------|
| **embeddenator** | v0.1.0 | v0.1.0 | v0.1.0 | v0.1.0 |
| **embeddenator-io** | v0.1.0 | - | - | - |
| **embeddenator-retrieval** | v0.1.0 | v0.1.0 | - | - |
| **embeddenator-fs** | v0.1.0 | v0.1.0 | v0.1.0 | - |
| **embeddenator-testkit** | v0.1.0 | v0.1.0 | - | - |
| **embeddenator-contract-bench** | v0.1.0 | v0.1.0 | v0.1.0 | - |

### Planned Major Releases

| Version | Target Date | Key Features | Breaking Changes |
|---------|-------------|--------------|------------------|
| **v0.21.0** | Feb 2026 | Incremental engram updates | Manifest format change |
| **v0.22.0** | Mar 2026 | Advanced retrieval algorithms | Query API refactor |
| **v0.23.0** | Apr 2026 | Compression layer | Engram format v2 |
| **v1.0.0** | Q2 2026 | API stability commitment | Final API cleanup |

## Release Checklist

Before tagging any version:

- [ ] **Version Numbers Match**
  - [ ] Cargo.toml version = git tag version
  - [ ] CHANGELOG.md has entry for this version
  - [ ] README.md updated with new version (if applicable)

- [ ] **Code Quality**
  - [ ] `cargo build --release` succeeds
  - [ ] `cargo test --all` passes
  - [ ] `cargo clippy --all-targets -- -D warnings` clean
  - [ ] `cargo doc --no-deps` generates without warnings

- [ ] **Documentation**
  - [ ] CHANGELOG.md updated with all changes
  - [ ] Migration guide written (if breaking changes)
  - [ ] API docs updated
  - [ ] Examples tested

- [ ] **Integration Testing**
  - [ ] Test in consumer repos (embeddenator core, testkit, etc.)
  - [ ] Verify backwards compatibility (if patch/minor)
  - [ ] Run full E2E test suite

- [ ] **Git Workflow**
  - [ ] All changes committed to main
  - [ ] Create annotated tag: `git tag -a vX.Y.Z -m "Release notes"`
  - [ ] Push commits: `git push origin main`
  - [ ] Push tag: `git push origin vX.Y.Z`

- [ ] **Post-Release**
  - [ ] Verify tag on GitHub: https://github.com/tzervas/REPO/tags
  - [ ] Create GitHub Release (optional, for major versions)
  - [ ] Announce in discussions/Discord (if applicable)
  - [ ] Update dependent repos

- [ ] **Docker (Core Only)**
  - [ ] Verify CI build succeeded: https://github.com/tzervas/embeddenator/actions
  - [ ] Check GHCR: https://github.com/tzervas/embeddenator/pkgs/container/embeddenator
  - [ ] Test image: `docker pull ghcr.io/tzervas/embeddenator:vX.Y.Z`
  - [ ] Verify both amd64 and arm64 manifests

## Troubleshooting

### "Version mismatch" Error

**Error:**
```
error: the listed checksum of `embeddenator-vsa v0.1.0` has changed
```

**Cause:** Git tag was rewritten/deleted and recreated.

**Fix:**
```bash
# In consumer repo
cargo clean
rm Cargo.lock
cargo update -p embeddenator-vsa

# Or use specific commit SHA instead of tag
[dependencies]
embeddenator-vsa = { git = "...", rev = "a1b2c3d" }
```

### "Unresolved dependency" Error

**Error:**
```
error: failed to load source for dependency `embeddenator-vsa`
```

**Cause:** Git tag doesn't exist or isn't pushed.

**Fix:**
```bash
# In component repo
git tag -l | grep v0.1.0  # Check if tag exists
git push origin v0.1.0    # Push tag
```

### Docker Tag Not Found

**Error:**
```
Error response from daemon: manifest unknown: manifest unknown
```

**Cause:** Docker image wasn't built/pushed for that tag.

**Fix:**
```bash
# Check GitHub Actions
# https://github.com/tzervas/embeddenator/actions

# Manually trigger workflow (if needed)
gh workflow run docker-build.yml

# Or build locally
docker build -f .docker/Dockerfile.embr-ci -t ghcr.io/tzervas/embeddenator:vX.Y.Z .
docker push ghcr.io/tzervas/embeddenator:vX.Y.Z
```

## References

- [Semantic Versioning 2.0.0](https://semver.org/)
- [Cargo Book: SemVer Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)
- [Git Tagging Best Practices](https://git-scm.com/book/en/v2/Git-Basics-Tagging)
- [COMPONENT_ARCHITECTURE.md](COMPONENT_ARCHITECTURE.md) - Dependency structure
- [LOCAL_DEVELOPMENT.md](LOCAL_DEVELOPMENT.md) - Development workflows
