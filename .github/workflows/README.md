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

### 2. **ci-amd64.yml** - AMD64 Build and Test (Required Pre-Merge Check)
**Triggers:** Every pull request, every push to `main`, after pre-checks pass

**Purpose:** Full build and test on AMD64 (x86_64) architecture - **MUST PASS FOR MERGE**

**Jobs:**
- Build release binary
- Run full test suite
- Run integration tests via orchestrator
- Upload artifacts on failure

**Runner:** `ubuntu-latest` (GitHub-hosted, amd64)

**Runtime:** ~5-7 minutes

**Status:** ✅ **ACTIVE** (Required status check for PR merges)

**Note:** This workflow is configured as a **required pre-merge check**. Pull requests cannot be merged to `main` until this workflow completes successfully. This ensures the build is not broken before code reaches the main branch.

---

### 3. **ci-arm64.yml** - ARM64 Build and Test (Self-Hosted Configuration)
**Triggers:** Manual (`workflow_dispatch`) only (pending self-hosted runner setup)

**Purpose:** Full build and test on ARM64 (aarch64) architecture

**Jobs:**
- Verify architecture
- Build release binary
- Run full test suite
- Run integration tests via orchestrator
- Upload artifacts on failure

**Runner:** Self-hosted ARM64 with labels `["self-hosted", "linux", "ARM64"]`

**Runtime:** 
- Self-hosted native: ~8-12 minutes (estimated)

**Status:** ⚠️ **CONFIGURED BUT DISABLED** - Ready for self-hosted runners

**Deployment Plan:**
1. **Phase 1** (Current): Manual testing with self-hosted runners via `workflow_dispatch`
2. **Phase 2** (After validation): Enable automatic trigger on merge to `main` only
3. **Phase 3** (Future): Consider making it a required check if ARM64 becomes critical

**Why Self-Hosted?**
ARM64 support requires self-hosted runners because GitHub Actions does not provide standard hosted ARM64 runners. Self-hosted runners will be deployed and configured with the required labels before this workflow is fully enabled.

#### ARM64 Configuration and Deployment Roadmap

**Current Status:**
The ARM64 workflow is fully configured for self-hosted runners but currently disabled pending runner deployment.

**Root Cause of Previous Issues:**

The ARM64 workflow was hanging because:

1. **Invalid Runner Labels**: Previous attempts used `ubuntu-24.04-arm64-4core` and `ubuntu-24.04-arm64` which don't exist
2. **GitHub Actions Limitation**: Standard GitHub-hosted runners are AMD64 only - no native ARM64 support
3. **Hanging Behavior**: Jobs queued indefinitely waiting for non-existent runners

**Previous Attempts (Resolved):**
- Commit b968753: Used `ubuntu-24.04-arm64` ❌ (invalid label)
- Commit 9790fd3: Used `ubuntu-24.04-arm64-4core` ❌ (invalid label)
- Commit 7252015: Temporarily disabled (correct action)
- Commit 4502381: Diagnosed root cause and documented solutions
- **Current**: Configured for self-hosted with labels `["self-hosted", "linux", "ARM64"]` ✅

**Deployment Roadmap:**

**Phase 1: Infrastructure Setup** (Pending)
- Deploy ARM64 hardware (physical server, VM, or cloud instance)
- Install GitHub Actions runner software on ARM64 hardware
- Register runner with labels: `self-hosted`, `linux`, `ARM64`
- Verify runner appears in: Settings → Actions → Runners

**Phase 2: Testing and Validation**
- Manual workflow trigger testing via `workflow_dispatch`
- Verify architecture detection (`uname -m` should show `aarch64`)
- Run full test suite and validate results
- Performance benchmarking and optimization

**Phase 3: Production Enablement**
- Enable automatic trigger on merge to `main` branch only
- Monitor performance and stability
- Document any ARM64-specific issues or workarounds

**Phase 4: Future Enhancements** (Optional)
- Consider making ARM64 a required check if it becomes critical
- Evaluate multi-runner setup for redundancy
- Optimize build caching for ARM64

**Why Self-Hosted?**

Self-hosted ARM64 runners are the **only practical solution** because:
- ✅ Fast native execution (no emulation overhead)
- ✅ Full control over hardware specs
- ✅ Cost-effective for frequent builds
- ✅ Can be deployed on existing infrastructure
- ❌ GitHub doesn't provide standard hosted ARM64 runners
- ❌ QEMU emulation is 5-10x slower and unreliable

---

## Automated Runner Management System

🎉 **NEW**: Embeddenator now includes a comprehensive Python-based automation system for managing self-hosted runners!

### Overview

The `runner_manager.py` script provides complete lifecycle automation:
- ✨ Automated registration with short-lived tokens
- 🔄 Complete lifecycle management (register → run → deregister)
- ⏱️ Auto-deregistration after configurable idle timeout
- 🎯 Manual mode for persistent runners
- 🚀 Multi-runner deployment support
- 📊 Health monitoring and status reporting

### Quick Start

```bash
# 1. Configure (required: GITHUB_REPOSITORY and GITHUB_TOKEN)
cp .env.example .env
# Edit .env with your repository and GitHub PAT

# 2. Run in auto mode (registers, starts, monitors, auto-deregisters when idle)
python3 runner_manager.py run

# 3. Or run in manual mode (keeps running until stopped)
RUNNER_MODE=manual python3 runner_manager.py run
```

### Key Features

**Auto Mode** (Cost Optimized):
- Registers runner automatically
- Monitors job queue
- Auto-deregisters after idle timeout (default: 5 minutes)
- Perfect for sporadic CI/CD builds

**Manual Mode** (Persistent):
- Keeps runner alive until explicitly stopped
- Ideal for development environments
- Full control over lifecycle

**Multi-Runner Support**:
```bash
# Deploy 4 runners for parallel builds
python3 runner_manager.py run --runner-count 4
```

### Documentation

For complete documentation, see: [`docs/RUNNER_AUTOMATION.md`](../../docs/RUNNER_AUTOMATION.md)

Topics covered:
- Configuration options (50+ environment variables)
- Deployment modes and strategies
- Advanced features (ephemeral runners, resource management)
- Troubleshooting and security best practices
- Integration examples (systemd, Docker, cron)

---

## Manual Self-Hosted Runner Setup

If you prefer manual setup instead of using the automation system:

**Self-Hosted Runner Setup Guide:**

```bash
# On ARM64 hardware (aarch64):

# 1. Download runner
mkdir actions-runner && cd actions-runner
curl -o actions-runner-linux-arm64-2.311.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-arm64-2.311.0.tar.gz
tar xzf ./actions-runner-linux-arm64-2.311.0.tar.gz

# 2. Configure with labels
./config.sh --url https://github.com/tzervas/embeddenator \
  --token YOUR_TOKEN \
  --labels self-hosted,linux,ARM64 \
  --name arm64-runner-1

# 3. Install as service
sudo ./svc.sh install
sudo ./svc.sh start

# 4. Verify
sudo ./svc.sh status
```

**Monitoring:**
- Runner status: Settings → Actions → Runners
- Workflow runs: Actions tab → CI arm64 Build and Test
- Logs: Check runner machine logs at `_diag/` directory

---

### 4. **build-holographic-os.yml** - OS Container Builds
**Triggers:** Manual (`workflow_dispatch`)

**Purpose:** Build holographic OS containers for specific configurations

**Status:** ✅ Active (manual trigger)

---

### 5. **build-push-images.yml** - Multi-OS Image Pipeline
**Triggers:** Manual (`workflow_dispatch`)

**Purpose:** Build and push multiple OS configurations to GHCR

**Default Targets:** AMD64 only (debian-stable, debian-testing, ubuntu-stable)
- ARM64 builds available via manual input but not recommended in CI (emulation too slow)

**Features:**
- Matrix-based parallel builds
- Configurable via comma-separated input
- Optional test execution before build
- GHCR push with proper tagging

**Status:** ✅ Active (manual trigger)

---

### 6. **nightly-builds.yml** - Automated Nightly Builds
**Triggers:** Daily at 2 AM UTC (`cron: '0 2 * * *'`)

**Purpose:** Build bleeding-edge images with nightly Rust and latest OS packages

**Targets:** AMD64 only (ARM64 emulation too slow for CI)
- Debian Testing/Sid (amd64)
- Ubuntu Devel/Rolling (amd64)

**Note:** ARM64 builds should be done locally or on self-hosted ARM64 runners

**Status:** ✅ Active (scheduled)

---

### 7. **build-push-arm64.yml** - ARM64 Image Builds (Self-Hosted)
**Triggers:** Manual (`workflow_dispatch`)

**Purpose:** Build and push ARM64 images using self-hosted runners (local or QEMU-emulated)

**Runner Options:**
- **Large runner:** 10 cores, 16GB RAM - builds 4 configs in parallel
- **Multi runners:** 4x runners with 4 cores, 6GB RAM each - distributed builds
- **Native ARM64:** Physical ARM64 hardware

**Features:**
- Disk space management (automatic cleanup)
- Configurable parallelism based on runner type
- GHCR push with proper tagging
- Build metrics and monitoring

**Default Targets:** debian-stable-arm64, debian-testing-arm64, ubuntu-stable-arm64

**Setup:** See [ARM64_RUNNER_SETUP.md](./ARM64_RUNNER_SETUP.md) for detailed instructions

**Status:** ✅ Active (manual trigger, requires self-hosted runner)

---

## Workflow Dependencies

```
On Pull Request:
  ├─ ci-pre-checks.yml (runs first, fast validation)
  │   └─ If successful:
  │       └─ ci-amd64.yml (REQUIRED FOR MERGE - must pass)
  │
  └─ ci-arm64.yml (DISABLED - awaiting self-hosted runners)
                   (Future: will run on merge to main only)

On Push to Main:
  ├─ ci-pre-checks.yml
  ├─ ci-amd64.yml
  └─ ci-arm64.yml (will be enabled post-deployment)

Manual/Scheduled:
  ├─ build-holographic-os.yml (manual)
  ├─ build-push-images.yml (manual - AMD64 only)
  ├─ build-push-arm64.yml (manual - ARM64 only, self-hosted)
  └─ nightly-builds.yml (scheduled daily at 2 AM UTC - AMD64 only)
```

**Key Points:**
- **Pre-checks** run on every PR commit (fast feedback)
- **AMD64** is a **required check** - PR cannot merge if it fails
- **ARM64** will trigger only on merge to main (post-deployment) to reduce costs
- Manual workflows remain independent

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

| Workflow | Runtime | Status | Notes |
|----------|---------|--------|-------|
| ci-pre-checks.yml | ~3-5 min | ✅ Active | Every PR/push |
| ci-amd64.yml | ~5-7 min | ✅ Active | **Required for merge** |
| ci-arm64.yml | ~8-12 min (est.) | ⚠️ Configured, pending deployment | Self-hosted only |
| **Total (current)** | **~5-7 min** | **-50% vs before** | AMD64 only |
| **Total (future w/ ARM64)** | **~10-15 min** | **Merge to main only** | Parallel execution |

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

## Architecture Support Roadmap

| Architecture | Status | Runner Type | Trigger | Notes |
|--------------|--------|-------------|---------|-------|
| **amd64 (x86_64)** | ✅ Production | GitHub-hosted | Every PR (required) | Stable, fast |
| **arm64 (aarch64)** | 🚧 Configured | Self-hosted | Manual only (pending) | Ready for deployment |

**Future ARM64 Timeline:**
1. ✅ **Completed**: Root cause analysis and workflow configuration
2. 🚧 **In Progress**: Self-hosted runner infrastructure setup
3. ⏳ **Next**: Manual testing and validation
4. ⏳ **Future**: Auto-trigger on merge to main
5. ⏳ **Future**: Evaluate as required check (optional)

---

**Last Updated:** 2025-12-16
**Maintained by:** @tzervas, @copilot

**Document Version:** 2.0 (Self-Hosted ARM64 Configuration)
