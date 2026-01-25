# CI/CD Activation Checklist

> **Step-by-step checklist to activate the CI/CD infrastructure**

**Status:** Ready for activation  
**Date:** January 16, 2026

---

## Pre-Activation Checklist

### ✅ Implementation Verification

- [x] All 6 workflows created
- [x] YAML syntax validated
- [x] Documentation completed
- [x] CODEOWNERS configured
- [x] PR/issue templates created
- [x] Status badges added
- [x] Root README updated

**Status:** ✅ All implementation tasks complete

---

## Activation Steps

### Step 1: GitHub Repository Settings

#### 1.1 Enable GitHub Actions

```
Location: Settings → Actions → General
Action: 
  1. Actions permissions → Allow all actions and reusable workflows
  2. Workflow permissions → Read and write permissions
  3. Save changes
```

- [ ] Actions enabled
- [ ] Workflow permissions configured

#### 1.2 Configure GitHub Pages

```
Location: Settings → Pages
Action:
  1. Source → Deploy from a branch → Change to "GitHub Actions"
  2. Save changes
```

- [ ] GitHub Pages configured
- [ ] Deployment source set to "GitHub Actions"

#### 1.3 Set Up Secrets (Optional)

```
Location: Settings → Secrets and variables → Actions
Optional secrets:
  - CARGO_REGISTRY_TOKEN (for crates.io publishing)
```

- [ ] Secrets configured (if needed)

---

### Step 2: Branch Protection

#### 2.1 Configure Main Branch Protection

```
Location: Settings → Branches → Add rule
Branch name pattern: main
```

**Required settings:**

- [ ] Require a pull request before merging
  - [ ] Require approvals: 1
  - [ ] Dismiss stale pull request approvals when new commits are pushed
  - [ ] Require review from Code Owners

- [ ] Require status checks to pass before merging
  - [ ] Require branches to be up to date before merging
  - [ ] Status checks (add these):
    - [ ] Format Check
    - [ ] Version Consistency
    - [ ] Test (embeddenator)
    - [ ] Test (embeddenator-cli)
    - [ ] Test (embeddenator-contract-bench)
    - [ ] Test (embeddenator-fs)
    - [ ] Test (embeddenator-interop)
    - [ ] Test (embeddenator-io)
    - [ ] Test (embeddenator-obs)
    - [ ] Test (embeddenator-retrieval)
    - [ ] Test (embeddenator-testkit)
    - [ ] Test (embeddenator-vsa)
    - [ ] Test (embeddenator-workspace)
    - [ ] Integration Tests
    - [ ] Workspace Health Check
    - [ ] Documentation Build
    - [ ] CI Success

- [ ] Require conversation resolution before merging
- [ ] Include administrators
- [ ] Do not allow bypassing the above settings

**Save protection rules**

---

### Step 3: Initial Testing

#### 3.1 Test with Sample PR

```bash
# Create test branch
git checkout -b test-ci-activation
echo "# CI/CD Activation Test" > ci-activation-test.md
git add ci-activation-test.md
git commit -m "test: CI/CD activation verification"
git push origin test-ci-activation

# Create PR
gh pr create \
  --title "Test: CI/CD Activation" \
  --body "Testing CI/CD workflows after activation"
```

- [ ] Test branch created
- [ ] PR opened
- [ ] Workflows triggered

#### 3.2 Verify Workflow Execution

```bash
# Watch PR checks
gh pr checks --watch

# View workflow runs
gh run list --limit 5
```

**Expected workflows to run:**
- [ ] Workspace CI
- [ ] Documentation
- [ ] Security Scan (if on main/develop)

**Verify:**
- [ ] Format check passes
- [ ] Version check passes
- [ ] All test jobs run
- [ ] Integration tests run
- [ ] Health check runs
- [ ] Documentation builds
- [ ] All checks pass (green)

#### 3.3 Verify PR Comments

- [ ] Health report commented on PR
- [ ] No regression warnings (first run)

---

### Step 4: Manual Workflow Testing

#### 4.1 Test Health Check

```bash
gh workflow run health-workspace.yml
gh run watch
```

- [ ] Health check completes
- [ ] Artifacts generated
- [ ] Report available

#### 4.2 Test Benchmark Workflow

```bash
gh workflow run bench-workspace.yml
gh run watch
```

- [ ] Benchmarks complete
- [ ] Baseline created
- [ ] Artifacts generated

#### 4.3 Test Security Scan

```bash
gh workflow run security-workspace.yml
gh run watch
```

- [ ] Security scan completes
- [ ] No critical vulnerabilities
- [ ] Reports generated

#### 4.4 Test Release (Dry Run)

```bash
gh workflow run release-workspace.yml \
  -f version_bump=prerelease \
  -f dry_run=true
gh run watch
```

- [ ] Validation passes
- [ ] Version bump simulated
- [ ] No actual changes made

---

### Step 5: Verify Documentation Deployment

#### 5.1 Check Pages Deployment

```
URL: https://tzervas.github.io/embeddenator/
```

- [ ] Documentation deployed
- [ ] Index page loads
- [ ] Component docs accessible
- [ ] No build errors

#### 5.2 Verify Badges

**Check badges in README files:**

- [ ] CI badge displays correctly
- [ ] Benchmarks badge displays
- [ ] Health badge displays
- [ ] Docs badge displays
- [ ] Security badge displays

**Badge URLs:**
- CI: `https://github.com/tzervas/embeddenator/workflows/Workspace%20CI/badge.svg`
- Benchmarks: `https://github.com/tzervas/embeddenator/workflows/Benchmark%20Regression%20Check/badge.svg`
- Health: `https://github.com/tzervas/embeddenator/workflows/Workspace%20Health/badge.svg`
- Docs: `https://github.com/tzervas/embeddenator/workflows/Documentation/badge.svg`
- Security: `https://github.com/tzervas/embeddenator/workflows/Security%20Scan/badge.svg`

---

### Step 6: Merge Test PR

#### 6.1 Verify All Checks Pass

- [ ] All required checks green
- [ ] No failing tests
- [ ] No regressions detected
- [ ] Documentation built successfully

#### 6.2 Merge Test PR

```bash
# Merge via GitHub CLI
gh pr merge test-ci-activation --squash --delete-branch

# Or via GitHub UI
# 1. Go to PR
# 2. Click "Squash and merge"
# 3. Confirm
# 4. Delete branch
```

- [ ] PR merged successfully
- [ ] Branch deleted
- [ ] Post-merge workflows run

#### 6.3 Verify Post-Merge Actions

- [ ] Documentation deployed to Pages
- [ ] Benchmark baselines updated
- [ ] Health check runs (if scheduled)

---

### Step 7: Configure Scheduled Workflows

#### 7.1 Verify Schedule Configuration

**Benchmarks:**
- Schedule: Nightly at 2 AM UTC
- [ ] Cron expression correct

**Health Check:**
- Schedule: Daily at 6 AM UTC
- [ ] Cron expression correct

**Security Scan:**
- Schedule: Weekly on Monday at 9 AM UTC
- [ ] Cron expression correct

#### 7.2 Test Scheduled Triggers (Wait or Manual)

```bash
# Manually trigger to test (don't wait for schedule)
gh workflow run health-workspace.yml
gh workflow run bench-workspace.yml
gh workflow run security-workspace.yml
```

- [ ] Scheduled workflows work when triggered manually

---

## Post-Activation Checklist

### Documentation

- [ ] Review CI_CD_GUIDE.md
- [ ] Review CI_CD_QUICK_START.md
- [ ] Share documentation with team
- [ ] Bookmark workflow URLs

### Monitoring

- [ ] Set up notifications for workflow failures
  - GitHub: Settings → Notifications → Actions
- [ ] Add workflow links to team documentation
- [ ] Subscribe to workflow runs

### Optional Enhancements

- [ ] Add CARGO_REGISTRY_TOKEN for publishing
- [ ] Configure Dependabot
- [ ] Set up code coverage reporting
- [ ] Add changelog automation
- [ ] Set up self-hosted runners (if needed)

---

## Validation Checklist

### All Workflows Tested

- [ ] ci-workspace.yml - Tested via PR
- [ ] bench-workspace.yml - Tested manually
- [ ] health-workspace.yml - Tested manually
- [ ] docs-workspace.yml - Tested via PR
- [ ] security-workspace.yml - Tested manually
- [ ] release-workspace.yml - Tested in dry-run mode

### All Features Verified

- [ ] Parallel test execution
- [ ] Rust dependency caching
- [ ] Benchmark regression detection
- [ ] Health check automation
- [ ] Documentation deployment
- [ ] Security vulnerability scanning
- [ ] PR comments (health reports)
- [ ] Issue automation (health failures)
- [ ] Status badges working
- [ ] Branch protection enforced

---

## Troubleshooting

### If Workflows Don't Trigger

1. Check Actions are enabled (Settings → Actions)
2. Verify workflow file syntax (YAML validation)
3. Check trigger conditions (branch names, paths)
4. View workflow logs for errors

### If Tests Fail

1. Run tests locally: `cargo test --all-features --workspace`
2. Check for environment differences
3. Review workflow logs
4. Verify dependencies are up to date

### If Caching Doesn't Work

1. Check cache keys match
2. Verify cache storage limits
3. Clear cache: Settings → Actions → Caches
4. Update cache key version

### If Documentation Doesn't Deploy

1. Verify Pages configured (GitHub Actions source)
2. Check docs workflow succeeded
3. Review deployment logs
4. Wait 5-10 minutes for DNS propagation

---

## Success Criteria

### All Green ✅

- [x] All workflows created and validated
- [ ] GitHub Actions enabled
- [ ] Branch protection configured
- [ ] Test PR merged successfully
- [ ] All workflows tested
- [ ] Documentation deployed
- [ ] Status badges working
- [ ] Health check passing
- [ ] No security vulnerabilities
- [ ] Team notified

### Ready for Production

When all checkboxes are complete:

✅ CI/CD infrastructure is **LIVE AND OPERATIONAL**

---

## Quick Reference

### Essential Commands

```bash
# View workflow runs
gh run list --limit 10

# Watch current run
gh run watch

# Trigger workflow
gh workflow run <workflow-name>

# View PR checks
gh pr checks --watch

# View logs
gh run view <run-id> --log
```

### Essential URLs

- Actions: `https://github.com/tzervas/embeddenator/actions`
- Settings: `https://github.com/tzervas/embeddenator/settings`
- Pages: `https://tzervas.github.io/embeddenator/`
- Workflows: `https://github.com/tzervas/embeddenator/actions/workflows`

---

## Support

- **Documentation:** [CI_CD_GUIDE.md](./CI_CD_GUIDE.md)
- **Quick Start:** [CI_CD_QUICK_START.md](./CI_CD_QUICK_START.md)
- **Implementation Report:** [CI_CD_IMPLEMENTATION_REPORT.md](./CI_CD_IMPLEMENTATION_REPORT.md)
- **GitHub Actions Docs:** https://docs.github.com/en/actions

---

**Created:** January 16, 2026  
**Status:** Ready for activation  
**Estimated Time to Complete:** 30-45 minutes
