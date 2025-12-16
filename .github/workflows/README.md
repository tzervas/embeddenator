# GitHub Actions Workflows

This directory contains the CI/CD workflows for Embeddenator.

## Workflow Structure

The CI/CD pipeline is split into separate workflows to avoid duplication and provide clear separation of concerns:

### 1. **ci-pre-checks.yml** - Pre-build Validation
**Triggers:** Every push to `main`, every pull request

**Purpose:** Fast checks that must pass before platform-specific builds

**Jobs:**
- Code formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Unit tests (`cargo test --lib --bins`)
- Doc tests (`cargo test --doc`)
- Python syntax validation
- YAML validation

**Runtime:** ~3-5 minutes

**Note:** This workflow runs on every push/PR and must pass before other workflows run.

---

### 2. **ci-amd64.yml** - AMD64 Build and Test
**Triggers:** Every push to `main`, every pull request, after pre-checks pass

**Purpose:** Full build and test on AMD64 (x86_64) architecture

**Jobs:**
- Build release binary
- Run full test suite (33 tests)
- Run integration tests via orchestrator
- Upload artifacts on failure

**Runner:** `ubuntu-latest` (GitHub-hosted, amd64)

**Runtime:** ~5-7 minutes

**Status:** ✅ **ACTIVE**

---

### 3. **ci-arm64.yml** - ARM64 Build and Test
**Triggers:** Manual (`workflow_dispatch`) only

**Purpose:** Full build and test on ARM64 (aarch64) architecture

**Jobs:**
- Verify architecture
- Build release binary
- Run full test suite (33 tests)
- Run integration tests via orchestrator
- Upload artifacts on failure

**Runner:** Configurable (self-hosted, emulation, or test-only)

**Runtime:** 
- Self-hosted native: ~8-12 minutes
- Emulation: ~30-45 minutes

**Status:** ⚠️ **DISABLED** (manual trigger only, pending runner setup)

#### Why is ARM64 Disabled?

**Root Cause Analysis:**

The ARM64 workflow was hanging during startup because:

1. **Invalid Runner Label**: The workflow specified `ubuntu-24.04-arm64-4core` which doesn't exist in GitHub's runner pool
2. **GitHub Actions Limitation**: Standard GitHub-hosted runners are AMD64 only (no native ARM64)
3. **Hanging Behavior**: Jobs queued indefinitely waiting for a runner that would never become available

**Previous Attempts:**
- Commit b968753: Used `ubuntu-24.04-arm64` ❌ (doesn't exist)
- Commit 9790fd3: Used `ubuntu-24.04-arm64-4core` ❌ (doesn't exist)
- Commit 7252015: Temporarily disabled ✅ (current state)

**Solutions to Re-enable ARM64:**

**Option 1: Self-hosted ARM64 Runner** (RECOMMENDED)
- Setup: Install GitHub Actions runner on ARM64 hardware
- Configuration: Use labels `["self-hosted", "linux", "ARM64"]`
- Pros: Fast native execution, full control
- Cons: Requires infrastructure setup

**Option 2: GitHub Larger Runners** (if available)
- Availability: Requires GitHub Team or Enterprise plan
- Labels: Check with `gh api /repos/OWNER/REPO/actions/runners`
- Possible: `ubuntu-24.04-arm` or similar
- Pros: Managed by GitHub
- Cons: May not be available, costs money

**Option 3: QEMU Emulation** (fallback)
- Setup: Use `ubuntu-latest` with QEMU
- Performance: 5-10x slower than native
- Pros: Works everywhere, no setup needed
- Cons: Very slow, may timeout

**To Re-enable:**
1. Choose a solution above
2. Update `runs-on:` in `ci-arm64.yml`
3. Test with manual trigger first
4. Uncomment automatic triggers if successful

---

### 4. **build-holographic-os.yml** - OS Container Builds
**Triggers:** Manual (`workflow_dispatch`)

**Purpose:** Build holographic OS containers for specific configurations

**Status:** ✅ Active (manual trigger)

---

### 5. **build-push-images.yml** - Multi-OS Image Pipeline
**Triggers:** Manual (`workflow_dispatch`)

**Purpose:** Build and push multiple OS configurations to GHCR

**Status:** ✅ Active (manual trigger)

---

### 6. **nightly-builds.yml** - Automated Nightly Builds
**Triggers:** Daily at 2 AM UTC (`cron: '0 2 * * *'`)

**Purpose:** Build bleeding-edge images with nightly Rust and latest OS packages

**Targets:**
- Debian Testing/Sid
- Ubuntu Devel/Rolling

**Status:** ✅ Active (scheduled)

---

## Workflow Dependencies

```
On Push/PR:
  ├─ ci-pre-checks.yml (runs first)
  │   └─ If successful:
  │       └─ ci-amd64.yml (runs automatically)
  │
  └─ ci-arm64.yml (DISABLED - manual only)

Manual/Scheduled:
  ├─ build-holographic-os.yml
  ├─ build-push-images.yml
  └─ nightly-builds.yml
```

## Key Improvements from Previous Version

### Before (Old ci.yml):
❌ Two separate jobs: `build-test` + `multi-arch`
❌ Duplicate runs: amd64 tests ran twice
❌ Sequential execution: `multi-arch` waited for `build-test`
❌ Invalid ARM64 runner causing hangs
❌ Total time: ~25-30 minutes (with duplicates and waits)

### After (New Structure):
✅ Three separate workflows: pre-checks, amd64, arm64
✅ No duplication: each test runs once
✅ Parallel execution: pre-checks and platform builds can overlap
✅ ARM64 properly diagnosed and documented
✅ Total time: ~5-7 minutes (amd64 only, no duplicates)

## Performance Metrics

| Workflow | Runtime | Status |
|----------|---------|--------|
| ci-pre-checks.yml | ~3-5 min | ✅ Active |
| ci-amd64.yml | ~5-7 min | ✅ Active |
| ci-arm64.yml | N/A | ⚠️ Disabled |
| **Total (current)** | **~5-7 min** | **-50% vs before** |

## Testing Locally

### Pre-checks:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --bins --verbose
cargo test --doc --verbose
```

### AMD64 Build:
```bash
cargo build --release --verbose
cargo test --verbose
python3 orchestrator.py --mode build --verbose
python3 orchestrator.py --mode test --verbose
```

### ARM64 (if self-hosted runner available):
```bash
# On ARM64 hardware:
./ci_build_monitor.sh linux/arm64 full 3600
```

## Troubleshooting

### "Workflow not triggering"
- Check: `.github/workflows/` files have correct `on:` triggers
- Check: YAML syntax is valid (`python3 -c "import yaml; yaml.safe_load(open('file.yml'))"`)
- Check: Workflow file is committed and pushed

### "Workflow hanging/queued forever"
- **Most likely:** Invalid runner label
- Check: Runner actually exists (`gh api /repos/OWNER/REPO/actions/runners`)
- Fix: Use valid runner label or setup self-hosted

### "ARM64 tests failing"
- Check: Architecture with `uname -m` (should show `aarch64` or `arm64`)
- Check: If emulation, ensure QEMU is properly setup
- Check: Timeout values are sufficient (emulation is slow)

## Contributing

When adding new workflows:
1. Choose appropriate triggers (avoid duplication)
2. Use descriptive job/step names
3. Add timeout values to prevent hangs
4. Upload artifacts on failure for debugging
5. Document runner requirements
6. Test manually before enabling automatic triggers

## Security

- All workflows use pinned action versions (`@v4`, not `@latest`)
- Permissions are explicitly declared (`contents: read`)
- Secrets are only used where necessary
- Self-hosted runners should be on private repos only

---

**Last Updated:** 2025-12-16
**Maintained by:** @tzervas, @copilot
