# CI/CD Guide for Embeddenator Workspace

[![CI Status](https://github.com/tzervas/embeddenator/workflows/Workspace%20CI/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/ci-workspace.yml)
[![Benchmarks](https://github.com/tzervas/embeddenator/workflows/Benchmark%20Regression%20Check/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/bench-workspace.yml)
[![Health Check](https://github.com/tzervas/embeddenator/workflows/Workspace%20Health/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/health-workspace.yml)
[![Documentation](https://github.com/tzervas/embeddenator/workflows/Documentation/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/docs-workspace.yml)
[![Security](https://github.com/tzervas/embeddenator/workflows/Security%20Scan/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/security-workspace.yml)

> **Last Updated:** January 16, 2026

This guide provides comprehensive documentation for the CI/CD infrastructure of the Embeddenator workspace.

## Table of Contents

- [Overview](#overview)
- [Workflow Architecture](#workflow-architecture)
- [Workflows](#workflows)
- [Branch Protection](#branch-protection)
- [Performance Estimates](#performance-estimates)
- [Adding New Checks](#adding-new-checks)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

---

## Overview

The Embeddenator workspace uses GitHub Actions for continuous integration and deployment. The CI/CD system is designed to:

- **Fast Feedback**: Most checks complete in under 10 minutes
- **Parallel Execution**: Tests run in parallel using matrix strategies
- **Intelligent Caching**: Rust dependencies cached across runs
- **Comprehensive Coverage**: Tests, benchmarks, docs, security, and health checks
- **Automated Quality Gates**: Enforce code quality, formatting, and version consistency

### Key Features

 Automated testing across all 11 component repos  
 Benchmark regression detection with historical baselines  
 Workspace health monitoring with automated issue creation  
 Documentation builds with GitHub Pages deployment  
 Security scanning with cargo-audit  
 Automated release workflow with version bumping  
 Multi-architecture build support (x86_64, ARM64)  

---

## Workflow Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Pull Request / Push                     │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ├─► Format Check (5 min)
                       │   └─► Fast fail if formatting issues
                       │
                       ├─► Version Check (5 min)
                       │   └─► Verify version consistency
                       │
                       ├─► Matrix Tests (15 min)
                       │   ├─► embeddenator
                       │   ├─► embeddenator-cli
                       │   ├─► embeddenator-contract-bench
                       │   ├─► embeddenator-fs
                       │   ├─► embeddenator-interop
                       │   ├─► embeddenator-io
                       │   ├─► embeddenator-obs
                       │   ├─► embeddenator-retrieval
                       │   ├─► embeddenator-testkit
                       │   ├─► embeddenator-vsa
                       │   └─► embeddenator-workspace
                       │
                       ├─► Integration Tests (20 min)
                       │
                       ├─► Health Check (10 min)
                       │   └─► Comment PR with report
                       │
                       ├─► Documentation Build (15 min)
                       │
                       └─► Multi-Version Tests (15 min)
                           ├─► Rust Stable
                           └─► Rust Nightly
```

---

## Workflows

### 1. Workspace CI (`ci-workspace.yml`)

**Trigger:** Push to `main`/`develop`, Pull Requests

**Purpose:** Core continuous integration for all components

**Jobs:**
- **format-check**: Verify code formatting with `cargo fmt`
- **test**: Matrix-based parallel testing of all 11 packages
- **integration-tests**: Cross-component integration testing
- **health-check**: Run workspace health checks using `embeddenator-workspace` tool
- **version-check**: Ensure version consistency across workspace
- **docs**: Build documentation for all packages
- **rust-versions**: Test on stable and nightly Rust
- **ci-success**: Final gate that all jobs passed

**Performance:** ~20 minutes total (with parallelization)

**Artifacts:**
- Health report (markdown + JSON)
- Documentation builds

**Example Usage:**
```bash
# Workflow runs automatically on PR
# View results at: https://github.com/tzervas/embeddenator/actions/workflows/ci-workspace.yml
```

---

### 2. Benchmark Regression Check (`bench-workspace.yml`)

**Trigger:** Push to `main`/`develop`, Pull Requests, Scheduled (nightly), Manual dispatch

**Purpose:** Detect performance regressions using Criterion benchmarks

**Jobs:**
- **benchmark**: Run benchmarks for all packages with benches
- **compare-baseline**: Compare PR benchmarks against main branch
- **update-baseline**: Update baselines after merging to main
- **export-metrics**: Export metrics for performance dashboards

**Performance:** ~30 minutes total

**Artifacts:**
- Benchmark results (bencher format)
- Criterion HTML reports
- Performance metrics (JSON)
- Baseline comparison reports

**Key Features:**
- Historical baseline tracking using GitHub Actions cache
- Automatic PR comments on regression detection
- Nightly baseline updates
- 90-day artifact retention

**Example Usage:**
```bash
# Run benchmarks manually
gh workflow run bench-workspace.yml

# View benchmark results
gh run list --workflow=bench-workspace.yml
gh run view <run-id> --log
```

---

### 3. Workspace Health (`health-workspace.yml`)

**Trigger:** Scheduled (daily), Push to `main`, Manual dispatch

**Purpose:** Monitor workspace health and create issues on failures

**Jobs:**
- **health-check**: Comprehensive health checks
  - Git status (uncommitted changes, sync status)
  - Version alignment
  - Test coverage
  - Documentation coverage
  - Spec coverage
- **git-status**: Check branch synchronization

**Performance:** ~30 minutes total

**Artifacts:**
- Health report (markdown + JSON)

**Key Features:**
- Automated issue creation on health check failures
- Automatic issue closure when health restored
- Daily scheduled runs
- Supports custom check selection via workflow dispatch

**Example Usage:**
```bash
# Run specific health checks
gh workflow run health-workspace.yml \
  -f checks=git,version,tests

# View health status
gh run list --workflow=health-workspace.yml --limit 5
```

---

### 4. Documentation (`docs-workspace.yml`)

**Trigger:** Push to `main`, Pull Requests, Manual dispatch

**Purpose:** Build and deploy documentation to GitHub Pages

**Jobs:**
- **build-docs**: Build rustdoc for all packages
- **deploy**: Deploy to GitHub Pages (main branch only)
- **doc-coverage**: Check documentation coverage (nightly)

**Performance:** ~20 minutes total

**Artifacts:**
- Complete documentation bundle
- GitHub Pages artifact

**Key Features:**
- Custom documentation index page
- Warnings treated as errors
- Automatic GitHub Pages deployment
- Documentation coverage reporting

**Documentation URL:** `https://tzervas.github.io/embeddenator/`

**Example Usage:**
```bash
# Build docs locally
for repo in embeddenator-*; do
  cargo doc --manifest-path "$repo/Cargo.toml" --all-features --no-deps
done

# Open local docs
open target/doc/index.html
```

---

### 5. Release Workflow (`release-workspace.yml`)

**Trigger:** Manual dispatch only

**Purpose:** Automated version bumping and release creation

**Jobs:**
- **validate**: Pre-release validation (tests, health, git status)
- **bump-versions**: Version bumping using `embeddenator-workspace` tool
- **build-artifacts**: Multi-platform binary builds
- **create-release**: GitHub release with artifacts
- **publish-crates**: Publish to crates.io (optional)

**Performance:** ~45 minutes total

**Supported Platforms:**
- Linux x86_64
- Linux ARM64
- macOS x86_64
- macOS ARM64

**Workflow Inputs:**
- `version_bump`: prerelease | patch | minor | major
- `dry_run`: true | false (default: true)
- `packages`: Comma-separated list or "all"

**Example Usage:**
```bash
# Dry run release (no actual changes)
gh workflow run release-workspace.yml \
  -f version_bump=prerelease \
  -f dry_run=true

# Actual release
gh workflow run release-workspace.yml \
  -f version_bump=patch \
  -f dry_run=false

# Manual version bump locally
cd embeddenator-workspace
cargo build --release
./target/release/embeddenator-workspace bump-version --prerelease --dry-run
```

---

### 6. Security Scan (`security-workspace.yml`)

**Trigger:** Push to `main`/`develop`, Pull Requests, Scheduled (weekly), Manual dispatch

**Purpose:** Security vulnerability scanning and dependency auditing

**Jobs:**
- **cargo-audit**: Check for known vulnerabilities
- **cargo-outdated**: Check for outdated dependencies
- **clippy-security**: Run security-focused Clippy lints
- **license-check**: Verify license compliance

**Performance:** ~20 minutes total

**Artifacts:**
- Audit results (JSON)
- Outdated dependencies report
- License report

**Key Features:**
- Weekly automated scans
- Automatic issue creation on vulnerabilities
- Comprehensive license reporting
- Security-focused Clippy lints

**Example Usage:**
```bash
# Run security audit locally
cargo install cargo-audit
for repo in embeddenator-*; do
  cargo audit --manifest-path "$repo/Cargo.toml"
done

# Check outdated dependencies
cargo install cargo-outdated
cargo outdated --workspace --root-deps-only
```

---

## Branch Protection

### Recommended Settings for `main` Branch

Configure these settings at: `Settings → Branches → Branch protection rules`

**Branch name pattern:** `main`

#### Protection Rules

 **Require a pull request before merging**
- Require approvals: 1
- Dismiss stale pull request approvals when new commits are pushed
- Require review from Code Owners

 **Require status checks to pass before merging**
- Require branches to be up to date before merging
- Required status checks:
  - `Format Check`
  - `Version Consistency`
  - `Test (embeddenator)`
  - `Test (embeddenator-cli)`
  - `Test (embeddenator-fs)`
  - `Test (embeddenator-vsa)`
  - `Test (embeddenator-workspace)`
  - `Integration Tests`
  - `Workspace Health Check`
  - `Documentation Build`
  - `CI Success`

 **Require conversation resolution before merging**

 **Require signed commits** (optional but recommended)

 **Include administrators**

 **Allow force pushes** (disabled)

 **Allow deletions** (disabled)

#### Status Check Timeout

Default timeout for status checks: **30 minutes**

---

## Performance Estimates

### Workflow Timing (with parallelization)

| Workflow | Duration | Parallel Jobs | Total Compute Time |
|----------|----------|---------------|-------------------|
| **Workspace CI** | ~20 min | 11 packages + 6 jobs | ~180 min |
| **Benchmarks** | ~30 min | 8 packages | ~240 min |
| **Health Check** | ~30 min | Sequential | ~30 min |
| **Documentation** | ~20 min | Sequential | ~20 min |
| **Security Scan** | ~20 min | Sequential | ~20 min |
| **Release** | ~45 min | 4 platforms | ~180 min |

### Cache Effectiveness

With `Swatinem/rust-cache@v2`:
- **First run (cold cache):** 15-20 minutes
- **Subsequent runs (warm cache):** 3-5 minutes
- **Cache hit rate:** >90% for typical PRs

### Cost Optimization

- **Free tier usage:** ~2,000 minutes/month
- **Expected usage:** ~1,500 minutes/month (typical activity)
- **Peak usage:** ~3,000 minutes/month (high activity)

**Optimization strategies:**
- Skip ARM64 builds for PRs (use `continue-on-error`)
- Cache criterion baselines (90-day retention)
- Parallel matrix execution
- Early failure detection (format check first)

---

## Adding New Checks

### Adding a New Package to CI

1. **Update `ci-workspace.yml`:**

```yaml
env:
  REPOS: >
    embeddenator
    embeddenator-cli
    # ... existing packages ...
    embeddenator-newpackage  # Add here
```

2. **Update matrix in test job:**

```yaml
matrix:
  package:
    - embeddenator
    # ... existing packages ...
    - embeddenator-newpackage  # Add here
```

3. **Add to benchmark workflow** (if package has benchmarks):

```yaml
matrix:
  package:
    # ... existing packages ...
    - embeddenator-newpackage  # Add here
```

4. **Update CODEOWNERS:**

```gitignore
/embeddenator-newpackage/ @tzervas
```

### Adding a Custom Health Check

Edit `embeddenator-workspace` source to add new health check categories:

```rust
// In embeddenator-workspace/src/health/mod.rs

pub enum HealthCheckCategory {
    Git,
    Version,
    Tests,
    Docs,
    Specs,
    CustomCheck,  // Add new category
}

// Implement check logic
impl HealthChecker {
    fn check_custom(&self) -> HealthCheckResult {
        // Your check logic here
    }
}
```

Then rebuild and update workflows:

```bash
cd embeddenator-workspace
cargo build --release
```

### Adding a New Workflow

1. Create workflow file:

```bash
touch .github/workflows/my-workflow.yml
```

2. Use this template:

```yaml
name: My Custom Workflow

on:
  push:
    branches: [main, develop]
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always

jobs:
  my-job:
    name: My Job
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          shared-key: my-workflow
      
      - name: Run my checks
        run: |
          # Your commands here
```

3. Add status badge to README:

```markdown
[![My Workflow](https://github.com/tzervas/embeddenator/workflows/My%20Custom%20Workflow/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/my-workflow.yml)
```

---

## Troubleshooting

### Common Issues

#### 1. Workflow fails with "No space left on device"

**Cause:** Large build artifacts filling disk

**Solution:**
```yaml
- name: Free disk space
  run: |
    sudo rm -rf /usr/share/dotnet
    sudo rm -rf /opt/ghc
    sudo rm -rf /usr/local/share/boost
    df -h
```

#### 2. Cache not restoring

**Cause:** Cache key mismatch or cache corruption

**Solution:**
```bash
# Clear cache manually (GitHub UI)
# Settings → Actions → Caches → Delete

# Or update cache key in workflow
shared-key: workspace-ci-v2  # Increment version
```

#### 3. Benchmark comparison fails

**Cause:** No baseline exists for comparison

**Solution:**
```bash
# Run benchmarks on main branch first
git checkout main
cargo bench --all-features

# Commit criterion baseline
git add target/criterion
git commit -m "chore: update criterion baseline"
git push
```

#### 4. Tests hang or timeout

**Cause:** Deadlock or infinite loop in tests

**Solution:**
```yaml
# Add timeout to specific steps
- name: Run tests
  timeout-minutes: 10  # Add timeout
  run: cargo test --all-features
```

#### 5. Documentation build warnings

**Cause:** Missing or incorrect doc comments

**Solution:**
```bash
# Build docs locally with warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps

# Fix warnings and rebuild
cargo doc --all-features --no-deps
```

#### 6. Version check fails

**Cause:** Version mismatch between packages

**Solution:**
```bash
# Check version consistency
cd embeddenator-workspace
cargo build --release
./target/release/embeddenator-workspace check-versions --verbose

# Fix versions
./target/release/embeddenator-workspace bump-version --prerelease
```

---

## Best Practices

### Writing CI-Friendly Code

1. **Fast Tests**: Keep unit tests under 100ms
2. **Parallel Tests**: Avoid global state
3. **Deterministic**: No random behavior without seeds
4. **Isolated**: Tests don't depend on external services
5. **Documented**: All public APIs have doc comments

### Optimizing Workflows

1. **Use caching**: Always use `Swatinem/rust-cache@v2`
2. **Parallel execution**: Use matrix strategies
3. **Early failure**: Put fast checks first
4. **Selective runs**: Use path filters for monorepos
5. **Timeouts**: Always set job timeouts

### Security Best Practices

1. **Use secrets**: Never hardcode credentials
2. **Minimal permissions**: Use least-privilege principle
3. **Pin actions**: Use specific versions (`actions/checkout@v4`)
4. **Audit dependencies**: Run cargo-audit regularly
5. **Signed commits**: Require GPG signatures

### Release Best Practices

1. **Always dry-run first**: Test release process
2. **Update CHANGELOG**: Document all changes
3. **Tag semantically**: Use SemVer (v1.2.3)
4. **Test artifacts**: Verify binaries work
5. **Announce releases**: Update documentation

---

## Workflow Status Badges

Add these badges to your repository README:

```markdown
## CI/CD Status

[![CI](https://github.com/tzervas/embeddenator/workflows/Workspace%20CI/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/ci-workspace.yml)
[![Benchmarks](https://github.com/tzervas/embeddenator/workflows/Benchmark%20Regression%20Check/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/bench-workspace.yml)
[![Health](https://github.com/tzervas/embeddenator/workflows/Workspace%20Health/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/health-workspace.yml)
[![Docs](https://github.com/tzervas/embeddenator/workflows/Documentation/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/docs-workspace.yml)
[![Security](https://github.com/tzervas/embeddenator/workflows/Security%20Scan/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/security-workspace.yml)
```

---

## Quick Reference

### Useful Commands

```bash
# View workflow runs
gh run list --workflow=ci-workspace.yml --limit 10

# View specific run
gh run view <run-id> --log

# Re-run failed jobs
gh run rerun <run-id> --failed

# Cancel running workflow
gh run cancel <run-id>

# Download artifacts
gh run download <run-id>

# Trigger workflow manually
gh workflow run bench-workspace.yml

# View workflow status
gh workflow view ci-workspace.yml

# List all workflows
gh workflow list
```

### Local Testing

```bash
# Install act (https://github.com/nektos/act)
brew install act  # macOS
# or
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Run workflow locally
act -W .github/workflows/ci-workspace.yml

# Run specific job
act -j test -W .github/workflows/ci-workspace.yml

# Run with secrets
act --secret-file .secrets
```

---

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust CI/CD Best Practices](https://github.com/actions-rs/meta/blob/master/recipes/quickstart.md)
- [Swatinem/rust-cache Documentation](https://github.com/Swatinem/rust-cache)
- [Embeddenator Workspace Tool README](../embeddenator-workspace/README.md)
- [Workflow Syntax Reference](https://docs.github.com/en/actions/reference/workflow-syntax-for-github-actions)

---

## Support

For issues with CI/CD workflows:

1. Check this guide first
2. Review workflow logs in GitHub Actions UI
3. Check [Troubleshooting](#troubleshooting) section
4. Create an issue with the `ci-cd` label

---

**Generated:** January 16, 2026  
**Maintainer:** @tzervas
