# CI/CD Quick Start Guide

> **Quick reference for testing and using the CI/CD workflows**

---

## Initial Setup

### 1. Enable GitHub Actions

```bash
# In GitHub repository settings:
# Settings → Actions → General → Actions permissions
# Select: "Allow all actions and reusable workflows"
```

### 2. Configure GitHub Pages

```bash
# Settings → Pages
# Source: GitHub Actions
# Branch: main (will be deployed automatically)
```

### 3. Set Branch Protection

```bash
# Settings → Branches → Add rule
# Branch name pattern: main
# Apply settings from CI_CD_GUIDE.md
```

---

## Testing Workflows Locally

### Option 1: Using `act` (Recommended)

```bash
# Install act
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Test CI workflow
act -W .github/workflows/ci-workspace.yml

# Test specific job
act -j format-check -W .github/workflows/ci-workspace.yml

# Test with secrets
echo "GITHUB_TOKEN=your_token" > .secrets
act --secret-file .secrets
```

### Option 2: Create Test PR

```bash
# Create test branch
git checkout -b test-ci

# Make small change
echo "# CI Test" > test-ci.md
git add test-ci.md
git commit -m "test: validate CI workflows"

# Push and create PR
git push origin test-ci
gh pr create \
  --title "Test: Validate CI/CD Workflows" \
  --body "Testing new CI/CD infrastructure"

# Watch workflow execution
gh pr checks --watch
gh run list --workflow=ci-workspace.yml --limit 5
```

### Option 3: Manual Workflow Dispatch

```bash
# Trigger health check manually
gh workflow run health-workspace.yml

# Trigger with custom inputs
gh workflow run release-workspace.yml \
  -f version_bump=prerelease \
  -f dry_run=true

# Watch execution
gh run watch
```

---

## Common Commands

### View Workflow Status

```bash
# List recent runs
gh run list --limit 10

# View specific workflow runs
gh run list --workflow=ci-workspace.yml

# View run details
gh run view <run-id>

# Download artifacts
gh run download <run-id>
```

### Re-run Failed Jobs

```bash
# Re-run failed jobs only
gh run rerun <run-id> --failed

# Re-run entire workflow
gh run rerun <run-id>
```

### Cancel Running Workflow

```bash
gh run cancel <run-id>
```

### View Logs

```bash
# Stream logs in real-time
gh run watch

# View logs for specific run
gh run view <run-id> --log

# View logs for specific job
gh run view <run-id> --job=<job-id> --log
```

---

## Testing Each Workflow

### 1. Workspace CI

```bash
# Trigger via PR
git checkout -b feature/test
echo "test" > test.txt
git add test.txt
git commit -m "test: CI trigger"
git push origin feature/test
gh pr create --fill

# Expected: All checks pass in ~20 minutes
# - Format check (5 min)
# - Version check (5 min)
# - Tests (15 min)
# - Integration tests (20 min)
# - Health check (10 min)
# - Docs (15 min)
```

### 2. Benchmarks

```bash
# Manual trigger
gh workflow run bench-workspace.yml

# Expected: Benchmarks complete in ~30 minutes
# Artifacts: benchmark results, HTML reports

# View results
gh run list --workflow=bench-workspace.yml --limit 1
gh run view <run-id>
gh run download <run-id>
```

### 3. Health Check

```bash
# Manual trigger with custom checks
gh workflow run health-workspace.yml \
  -f checks=git,version,tests

# Expected: Health report generated
# Artifact: health-report.md, health-report.json

# View health status
gh run list --workflow=health-workspace.yml --limit 1
```

### 4. Documentation

```bash
# Trigger via push to main
git checkout main
git pull
echo "# Docs update" >> README.md
git add README.md
git commit -m "docs: trigger doc build"
git push

# Expected: Docs built and deployed
# View: https://tzervas.github.io/embeddenator/
```

### 5. Security Scan

```bash
# Manual trigger
gh workflow run security-workspace.yml

# Expected: Security reports generated
# - Audit results
# - Outdated dependencies
# - License report

# View results
gh run download <run-id>
```

### 6. Release (Dry Run)

```bash
# Test release process (no actual release)
gh workflow run release-workspace.yml \
  -f version_bump=prerelease \
  -f dry_run=true

# Expected: 
# - Validation passes
# - Version bump simulated
# - No actual changes

# View results
gh run watch
```

---

## Validating Workflow Files

### Syntax Validation

```bash
# Validate all workflows
for workflow in .github/workflows/*.yml; do
  echo "Validating $workflow..."
  python3 -c "import yaml; yaml.safe_load(open('$workflow'))"
done
```

### Action Validation

```bash
# Check for outdated actions
for workflow in .github/workflows/*.yml; do
  echo "Checking $workflow..."
  grep -E "uses: .+@v[0-9]+" "$workflow"
done
```

---

## Monitoring Workflows

### GitHub UI

1. Go to **Actions** tab
2. Select workflow from left sidebar
3. View recent runs
4. Click run for details
5. View job logs

### Command Line

```bash
# Install GitHub CLI (if not installed)
brew install gh  # macOS
# or
sudo apt install gh  # Linux

# Authenticate
gh auth login

# View all runs
gh run list --limit 20

# Filter by workflow
gh run list --workflow=ci-workspace.yml

# Filter by status
gh run list --status=failure
gh run list --status=success

# Watch current run
gh run watch
```

---

## Troubleshooting

### Workflow Not Triggering

**Check:**
1. Actions enabled in settings?
2. Correct branch name?
3. Correct trigger conditions?
4. YAML syntax valid?

```bash
# Validate YAML
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci-workspace.yml'))"

# Check triggers
grep -A5 "^on:" .github/workflows/ci-workspace.yml
```

### Cache Not Working

**Solution:**
```bash
# Clear cache via GitHub UI
# Settings → Actions → Caches → Delete

# Or update cache key
# Edit workflow: shared-key: workspace-ci-v2
```

### Jobs Timing Out

**Solution:**
```yaml
# Increase timeout in workflow
timeout-minutes: 30  # Increase as needed
```

### Tests Failing

**Debug locally:**
```bash
# Run exact same commands as CI
cd <package>
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
cargo test --all-features --verbose
```

---

## Performance Optimization

### Reduce CI Time

1. **Enable parallel execution:** Already configured ✅
2. **Use caching:** Already configured ✅
3. **Skip unnecessary jobs:**
   ```yaml
   if: github.event_name != 'pull_request'
   ```
4. **Use path filters:**
   ```yaml
   on:
     push:
       paths:
         - 'embeddenator-vsa/**'
   ```

### Reduce Cost

1. **Skip ARM64 for PRs:** Already configured ✅
2. **Use self-hosted runners** (if available)
3. **Limit concurrent runs:**
   ```yaml
   concurrency:
     group: ${{ github.workflow }}-${{ github.ref }}
     cancel-in-progress: true
   ```

---

## Next Steps

### After Initial Setup

1. ✅ **Test all workflows** with test PR
2. ✅ **Verify badges** display correctly
3. ✅ **Check documentation** deployment
4. ✅ **Run health check** to establish baseline
5. ✅ **Run benchmarks** to create baselines

### Optional Enhancements

- [ ] Add `CARGO_REGISTRY_TOKEN` for crates.io publishing
- [ ] Set up self-hosted ARM64 runner
- [ ] Configure Dependabot for automated updates
- [ ] Add code coverage reporting (tarpaulin)
- [ ] Implement changelog automation (git-cliff)

---

## Quick Reference Card

| Task | Command |
|------|---------|
| **List workflows** | `gh workflow list` |
| **Run workflow** | `gh workflow run <workflow-name>` |
| **List runs** | `gh run list --limit 10` |
| **Watch run** | `gh run watch` |
| **View run** | `gh run view <run-id>` |
| **Re-run** | `gh run rerun <run-id>` |
| **Cancel** | `gh run cancel <run-id>` |
| **Download** | `gh run download <run-id>` |
| **PR checks** | `gh pr checks --watch` |
| **Validate YAML** | `python3 -c "import yaml; yaml.safe_load(open('file.yml'))"` |

---

## Expected Workflow Times

| Workflow | Cold Cache | Warm Cache |
|----------|------------|------------|
| **CI** | ~20 min | ~8 min |
| **Benchmarks** | ~30 min | ~15 min |
| **Health** | ~30 min | ~10 min |
| **Docs** | ~20 min | ~8 min |
| **Security** | ~20 min | ~5 min |
| **Release** | ~45 min | ~30 min |

---

## Support

- **CI/CD Guide:** [CI_CD_GUIDE.md](../CI_CD_GUIDE.md)
- **Implementation Summary:** [CI_CD_IMPLEMENTATION_SUMMARY.md](../CI_CD_IMPLEMENTATION_SUMMARY.md)
- **GitHub Actions Docs:** https://docs.github.com/en/actions
- **GitHub CLI Docs:** https://cli.github.com/manual/

---

**Last Updated:** January 16, 2026
