# TASK-004 Quick Start: Deploy ARM64 Runner and Test

This guide provides the fastest path to unblock TASK-004 and validate the ARM64 workflow.

## Prerequisites Check

```bash
# 1. Verify you're in the project directory
pwd
# Should output: /home/kang/Documents/projects/embeddenator

# 2. Check gh CLI authentication
gh auth status
# Should show: Logged in to github.com

# 3. Verify Python and dependencies
python3 --version  # Should be 3.7+
pip3 list | grep -E "requests|python-dotenv"

# 4. Check Docker availability
docker --version
docker ps
```

## Option 1: QEMU Emulation (Fastest, Any Machine)

**Time:** 5 minutes setup + 15-20 minutes first build  
**Cost:** Free  
**Performance:** Slow (5-10x slower than native)  
**Use Case:** Quick validation, development testing

```bash
# Create config file
cat > .env << 'EOF'
GITHUB_REPOSITORY=tzervas/embeddenator
GITHUB_TOKEN=your_github_token_here
RUNNER_MODE=manual
RUNNER_COUNT=1
RUNNER_LABELS=self-hosted,linux,ARM64
RUNNER_NAME=arm64-emulated-01
EOF

# Deploy runner with QEMU emulation
python3 runner_manager.py run

# In another terminal, verify runner is online
gh api repos/tzervas/embeddenator/actions/runners --jq '.runners[] | {name: .name, status: .status, busy: .busy}'

# Should show:
# {
#   "name": "arm64-emulated-01",
#   "status": "online",
#   "busy": false
# }
```

## Option 2: Oracle Cloud Free Tier (Best Value)

**Time:** 30-60 minutes (includes account setup)  
**Cost:** FREE (4 vCPU ARM64, 24GB RAM)  
**Performance:** Native ARM64, excellent  
**Use Case:** Production-ready, cost-effective

```bash
# 1. Create Oracle Cloud account (if needed)
#    https://cloud.oracle.com/free
#    Navigate to: Compute → Instances → Create Instance
#    Shape: Ampere A1 (ARM64) - 4 OCPU, 24GB RAM
#    OS: Ubuntu 22.04 ARM64

# 2. SSH into the instance
ssh ubuntu@<instance-ip>

# 3. Install dependencies
sudo apt update
sudo apt install -y git python3 python3-pip docker.io
sudo usermod -aG docker ubuntu
newgrp docker

# 4. Clone repository
git clone https://github.com/tzervas/embeddenator.git
cd embeddenator

# 5. Configure runner
cat > .env << 'EOF'
GITHUB_REPOSITORY=tzervas/embeddenator
GITHUB_TOKEN=your_github_token_here
RUNNER_MODE=auto
RUNNER_COUNT=1
IDLE_TIMEOUT=1800
RUNNER_LABELS=self-hosted,linux,ARM64,native
EOF

# 6. Deploy runner
python3 runner_manager.py run

# Runner will auto-deregister after 30 minutes of inactivity
```

## Option 3: AWS EC2 Graviton (Fastest Setup)

**Time:** 10-15 minutes  
**Cost:** ~$24/month (t4g.medium) or ~$96/month (t4g.xlarge)  
**Performance:** Native ARM64, production-grade  
**Use Case:** Commercial deployment, reliable

```bash
# 1. Launch EC2 instance
#    AWS Console → EC2 → Launch Instance
#    AMI: Ubuntu 22.04 ARM64
#    Instance Type: t4g.medium (2 vCPU, 4GB) or t4g.xlarge (4 vCPU, 16GB)
#    Security Group: Allow outbound HTTPS (443)
#    Storage: 50GB gp3

# 2. SSH and setup (same as Oracle Cloud steps 2-6)
ssh ubuntu@<instance-ip>
# ... follow steps 3-6 from Option 2
```

## After Runner Deployment: Run Tests

Once runner is online (any option above), execute the workflow:

```bash
# 1. Trigger workflow manually (minimal test)
gh workflow run build-push-arm64.yml \
  --field os_selections="debian-stable-arm64" \
  --field tag_suffix="-test" \
  --field push_to_ghcr=false \
  --field run_tests=true \
  --field runner_type="multi"

# 2. Monitor execution
gh run watch

# Or list runs
gh run list --workflow=build-push-arm64.yml --limit 5

# 3. Get detailed run info (replace RUN_ID)
gh run view <RUN_ID> --log

# 4. Check runner status during execution
gh api repos/tzervas/embeddenator/actions/runners --jq '.runners[] | {name: .name, status: .status, busy: .busy}'
```

## Expected Test Output

### Phase 1: Test Suite (if run_tests=true)
```
✅ Verify architecture
   Architecture: aarch64
✅ Set up Rust
✅ Run test suite
   Running 24 tests...
✅ Verify test count
   Total Tests: 24
```

### Phase 2: Docker Build
```
✅ Parse OS configuration
   os=debian, version=stable, arch=arm64
✅ Build holographic OS image
   Building: debian-stable-arm64-test
   Duration: ~15-20 minutes (native) or ~60-90 minutes (QEMU)
✅ Tag and push (if enabled)
```

### Success Criteria
- ✅ All 24 tests pass
- ✅ Docker image builds successfully
- ✅ Architecture verified as aarch64/arm64
- ✅ No platform-specific errors
- ✅ Total duration < 30 minutes (native) or < 2 hours (QEMU)

## Capture Results

After successful run:

```bash
# Get run details
RUN_ID=$(gh run list --workflow=build-push-arm64.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run view $RUN_ID --json conclusion,createdAt,updatedAt,jobs > arm64_run_results.json

# Extract test results
gh run view $RUN_ID --log | grep -E "(test result|passed|failed)" > arm64_test_results.txt

# Get timing information
gh run view $RUN_ID --json jobs --jq '.jobs[] | {name: .name, conclusion: .conclusion, startedAt: .startedAt, completedAt: .completedAt}'

# Check built images (if pushed to GHCR)
docker pull ghcr.io/tzervas/embeddenator:debian-stable-arm64-test
docker inspect ghcr.io/tzervas/embeddenator:debian-stable-arm64-test | jq '.[0].Architecture'
# Should output: "arm64"
```

## Update Documentation

After successful test run:

```bash
# Update ARM64_TEST_RESULTS.md with actual results
# Include:
# - Test date and run ID
# - Runner configuration used
# - All test pass/fail counts
# - Build duration and performance
# - Any issues discovered
# - Go/no-go recommendation for TASK-005

# Edit the file
nano docs/ARM64_TEST_RESULTS.md

# Commit results
git add docs/ARM64_TEST_RESULTS.md
git commit -m "TASK-004: Document ARM64 workflow test results

- Runner: [type used, e.g., Oracle Cloud ARM64]
- Tests: 24/24 passed
- Build time: [actual time]
- Status: VALIDATED, ready for TASK-005"
```

## Troubleshooting

### Runner not appearing online
```bash
# Check runner logs
python3 runner_manager.py status

# Check GitHub API directly
gh api repos/tzervas/embeddenator/actions/runners

# Verify runner registration token is valid (expires after 1 hour)
# Re-run registration if needed
python3 runner_manager.py stop
python3 runner_manager.py run
```

### Workflow fails to start
```bash
# Check workflow syntax
gh workflow view build-push-arm64.yml

# Verify runner labels match workflow requirements
# Workflow expects: ["self-hosted", "linux", "ARM64"]
gh api repos/tzervas/embeddenator/actions/runners --jq '.runners[].labels[].name'
```

### Tests fail on ARM64
```bash
# Run tests locally on ARM64 runner first
ssh into-runner-host
cd embeddenator
python3 test_runner.py

# Check for architecture-specific issues
uname -m  # Should be aarch64
rustc --version --verbose | grep host
```

### QEMU emulation too slow
```bash
# Consider:
# 1. Use native ARM64 hardware (Options 2 or 3)
# 2. Reduce test scope for validation:
gh workflow run build-push-arm64.yml \
  --field os_selections="debian-stable-arm64" \
  --field run_tests=false  # Skip tests, just build
  
# 3. Or increase QEMU CPUs (if on powerful host)
# Edit .env: RUNNER_CPU_COUNT=8
```

### Out of disk space during build
```bash
# On runner host
docker system prune -a -f
docker builder prune -a -f

# Check available space
df -h

# The workflow includes automatic cleanup steps
# If still failing, increase disk size (50GB → 100GB)
```

## Next Steps After Validation

Once the workflow completes successfully:

1. ✅ Mark TASK-004 as complete
2. ✅ Update ARM64_TEST_RESULTS.md with actual results
3. ✅ Proceed to TASK-005: Enable auto-trigger on main branch
4. ✅ Consider adding ARM64 as required check for PRs
5. ✅ Monitor runner costs and performance over time

## Quick Reference Commands

```bash
# Deploy runner (QEMU)
python3 runner_manager.py run

# Check runner status
gh api repos/tzervas/embeddenator/actions/runners --jq '.runners[]'

# Trigger test
gh workflow run build-push-arm64.yml --field os_selections="debian-stable-arm64" --field run_tests=true --field push_to_ghcr=false

# Monitor
gh run watch

# View results
gh run list --workflow=build-push-arm64.yml --limit 5

# Stop runner
python3 runner_manager.py stop

# View logs
python3 runner_manager.py status
```

---

**Choose your path based on:**
- **Fastest validation:** Option 1 (QEMU)
- **Best value long-term:** Option 2 (Oracle Free Tier)
- **Production deployment:** Option 3 (AWS Graviton)

**Expected total time to complete TASK-004:**
- QEMU: 2-3 hours (5 min setup + 1-2 hour build)
- Oracle Cloud: 3-4 hours (1 hour setup + 30 min build + testing)
- AWS Graviton: 1-2 hours (15 min setup + 30 min build + testing)
