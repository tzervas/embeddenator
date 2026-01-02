# ARM64 Self-Hosted Runner Setup Guide

This guide covers setting up self-hosted GitHub Actions runners for building ARM64 images, either on native ARM64 hardware or using QEMU emulation on a powerful x86_64 host.

## Hardware Requirements

### Option 1: One Large Runner (Recommended for Desktop)
- **CPU:** 10 cores dedicated
- **RAM:** 16GB
- **Disk:** 100GB free space
- **OS:** Ubuntu 22.04+ or Debian 12+
- **Best for:** Building all ARM64 configs in parallel

### Option 2: Multiple Small Runners
- **Per Runner:**
  - CPU: 4 cores
  - RAM: 6GB
  - Disk: 30GB free space each
- **Quantity:** 4 runners
- **Total Resources:** 16 cores, 24GB RAM, 120GB disk
- **Best for:** Distributing load across multiple VMs/containers

### Option 3: Native ARM64 Hardware Options

#### Cloud Providers:
- **AWS Graviton (t4g/c7g):** Cost-effective ARM64 instances
- **Oracle Cloud ARM:** Free tier available with Ampere CPUs
- **Azure Dpsv5:** ARM64-based VMs
- **Google Cloud Tau T2A:** ARM64 instances

#### On-Premise/Edge:
- **Raspberry Pi 5:** 4-8GB RAM, suitable for light workloads
- **Rock5 Model B:** Up to 32GB RAM, powerful SBC
- **NVIDIA Jetson:** GPU-enabled ARM64 (future use)
- **Apple Silicon (M1/M2/M3):** Via macOS runners (requires additional setup)

## Setup Instructions

### QEMU Emulation Setup (x86_64 Host)

If you're running on x86_64 hardware and want to emulate ARM64:

1. **Install QEMU and KVM:**
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y qemu-system-aarch64 qemu-user-static binfmt-support

# Enable ARM64 emulation
sudo update-binfmts --enable qemu-aarch64

# Verify
docker run --rm --platform linux/arm64 arm64v8/ubuntu uname -m
# Should output: aarch64
```

2. **Install Docker with multi-platform support:**
```bash
# Enable Docker buildx
docker buildx create --name multiarch --driver docker-container --use
docker buildx inspect --bootstrap
```

### Option 1: Single Large Runner Setup

1. **Create runner directory:**
```bash
mkdir -p ~/actions-runner-arm64-large && cd ~/actions-runner-arm64-large
```

2. **Download GitHub Actions runner:**
```bash
# For x86_64 host (QEMU emulation)
curl -o actions-runner-linux-x64.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-x64-2.311.0.tar.gz
tar xzf ./actions-runner-linux-x64.tar.gz

# For native ARM64 host
curl -o actions-runner-linux-arm64.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-arm64-2.311.0.tar.gz
tar xzf ./actions-runner-linux-arm64.tar.gz
```

3. **Configure the runner:**
```bash
# Get token from: https://github.com/tzervas/embeddenator/settings/actions/runners/new
./config.sh \
  --url https://github.com/tzervas/embeddenator \
  --token YOUR_RUNNER_TOKEN \
  --name arm64-large-runner \
  --labels self-hosted,linux,ARM64,large \
  --work _work
```

4. **Configure resource limits (systemd service):**
```bash
sudo ./svc.sh install

# Edit service file to set resource limits
sudo systemctl edit actions.runner.tzervas-embeddenator.arm64-large-runner.service
```

Add this content:
```ini
[Service]
# CPU limit: 10 cores
CPUQuota=1000%

# Memory limit: 16GB
MemoryMax=16G
MemoryHigh=15G

# Disk I/O priority
IOWeight=500
```

5. **Start the runner:**
```bash
sudo ./svc.sh start
sudo ./svc.sh status
```

### Option 2: Multiple Small Runners Setup

For 4 runners (runner-1 through runner-4):

```bash
for i in {1..4}; do
  mkdir -p ~/actions-runner-arm64-$i && cd ~/actions-runner-arm64-$i
  
  # Download and extract (use appropriate architecture)
  curl -o actions-runner.tar.gz -L \
    https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-x64-2.311.0.tar.gz
  tar xzf ./actions-runner.tar.gz
  
  # Configure (get separate token for each runner)
  ./config.sh \
    --url https://github.com/tzervas/embeddenator \
    --token YOUR_RUNNER_TOKEN_$i \
    --name arm64-runner-$i \
    --labels self-hosted,linux,ARM64 \
    --work _work
  
  # Install and start service
  sudo ./svc.sh install
  sudo ./svc.sh start
done
```

Add resource limits for each runner:
```bash
for i in {1..4}; do
  sudo systemctl edit actions.runner.tzervas-embeddenator.arm64-runner-$i.service
done
```

Per-runner limits:
```ini
[Service]
# CPU limit: 4 cores
CPUQuota=400%

# Memory limit: 6GB
MemoryMax=6G
MemoryHigh=5.5G

# Disk I/O priority
IOWeight=250
```

## Disk Management Strategy

To prevent disk space issues during builds:

### 1. Regular Cleanup Cron Job

Create `/etc/cron.daily/docker-cleanup`:
```bash
#!/bin/bash
# Clean up Docker resources daily

# Remove stopped containers older than 1 day
docker container prune -f --filter "until=24h"

# Remove dangling images
docker image prune -f

# Remove unused build cache older than 3 days
docker builder prune -f --filter "until=72h"

# Remove unused volumes
docker volume prune -f

# Log disk usage
df -h >> /var/log/docker-cleanup.log
docker system df >> /var/log/docker-cleanup.log
```

Make it executable:
```bash
sudo chmod +x /etc/cron.daily/docker-cleanup
```

## Automated Runner Management (Recommended)

The Embeddenator project includes a comprehensive Python-based automation system that handles runner lifecycle management automatically. This is the **recommended approach** for production deployments.

### Why Use Automation?

✅ **Automatic registration** with short-lived tokens  
✅ **Health monitoring** and auto-recovery  
✅ **Auto-scaling** with idle timeout  
✅ **Multi-runner coordination**  
✅ **Zero manual intervention** required  

### Quick Start with runner_manager.py

#### 1. Configure Environment
```bash
# In the embeddenator repository root
cp .env.example .env

# Edit .env and set:
GITHUB_REPOSITORY=tzervas/embeddenator
GITHUB_TOKEN=ghp_your_personal_access_token
RUNNER_LABELS=self-hosted,linux,ARM64
RUNNER_TARGET_ARCHITECTURES=arm64
```

#### 2. Run in Auto Mode (Cost Optimized)
```bash
# Single runner with auto-deregistration after 5 minutes idle
python3 runner_manager.py run

# Multiple runners for parallel builds
RUNNER_COUNT=4 python3 runner_manager.py run
```

#### 3. Run in Manual Mode (Persistent)
```bash
# Keep runner alive until manually stopped
RUNNER_MODE=manual python3 runner_manager.py run
```

### Configuration Options

Key environment variables for ARM64 runners:

```bash
# Runner Configuration
RUNNER_NAME=arm64-runner-1           # Unique name for each runner
RUNNER_LABELS=self-hosted,linux,ARM64  # Required labels for workflow
RUNNER_COUNT=1                       # Number of runners to deploy

# Architecture Support
RUNNER_TARGET_ARCHITECTURES=arm64    # Target architecture
RUNNER_ENABLE_EMULATION=true         # Enable QEMU if on x86_64
RUNNER_EMULATION_AUTO_INSTALL=false  # Auto-install QEMU (requires sudo)

# Lifecycle Management
RUNNER_MODE=auto                     # auto|manual
RUNNER_IDLE_TIMEOUT=300              # Auto-deregister after 5 min idle
RUNNER_EPHEMERAL=false               # Single-use runners (deregister after 1 job)

# Resource Management
RUNNER_WORK_DIR=_work                # Working directory for jobs
RUNNER_CLEANUP_ON_EXIT=true          # Clean up on shutdown
```

### Advanced Deployment Scenarios

#### Scenario 1: ARM64 with QEMU Emulation (Development)
```bash
# On x86_64 host, emulate ARM64 runners
RUNNER_TARGET_ARCHITECTURES=arm64 \
RUNNER_ENABLE_EMULATION=true \
RUNNER_EMULATION_METHOD=docker \
RUNNER_COUNT=2 \
python3 runner_manager.py run
```

#### Scenario 2: Native ARM64 (Production)
```bash
# On ARM64 hardware (or Graviton/Oracle Cloud)
RUNNER_TARGET_ARCHITECTURES=arm64 \
RUNNER_MODE=manual \
RUNNER_COUNT=4 \
python3 runner_manager.py run
```

#### Scenario 3: Auto-Scaling for CI/CD
```bash
# Auto-register on demand, auto-deregister when idle
RUNNER_MODE=auto \
RUNNER_IDLE_TIMEOUT=300 \
RUNNER_COUNT=2 \
python3 runner_manager.py run
```

### Monitoring and Management

#### Check Runner Status
```bash
python3 runner_manager.py status
```

#### View Logs
```bash
# Real-time logs
python3 runner_manager.py run --verbose

# Check runner_manager logs
tail -f logs/runner_manager.log
```

#### Stop Runners
```bash
# Graceful shutdown
python3 runner_manager.py stop

# Or Ctrl+C in the terminal running the manager
```

### Integration with Systemd (Production)

For production deployments, run runner_manager.py as a systemd service:

```bash
# Create service file
sudo tee /etc/systemd/system/github-runner-manager.service > /dev/null <<EOF
[Unit]
Description=GitHub Actions Runner Manager for Embeddenator
After=network.target docker.service

[Service]
Type=simple
User=github-runner
WorkingDirectory=/home/github-runner/embeddenator
EnvironmentFile=/home/github-runner/embeddenator/.env
ExecStart=/usr/bin/python3 /home/github-runner/embeddenator/runner_manager.py run
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable github-runner-manager
sudo systemctl start github-runner-manager

# Check status
sudo systemctl status github-runner-manager
```

### Comparison: Manual vs Automated Setup

| Feature | Manual Setup | Automated (runner_manager.py) |
|---------|-------------|-------------------------------|
| Registration | Manual token management | Automatic with short-lived tokens |
| Lifecycle | Manual start/stop | Fully automated |
| Health Monitoring | External monitoring needed | Built-in |
| Multi-Runner | Complex scripting | Simple config |
| Auto-Scaling | Not supported | Built-in with idle timeout |
| Recovery | Manual intervention | Automatic restart |
| Emulation | Manual QEMU setup | Auto-detected and configured |
| **Recommended For** | Quick testing, learning | Production & active development |

### Further Documentation

For complete automation features, see:
- [Runner Automation Guide](../../docs/RUNNER_AUTOMATION.md)
- [Self-Hosted CI Project Spec](../../docs/SELF_HOSTED_CI_PROJECT_SPEC.md)

### 2. Docker Daemon Configuration

Edit `/etc/docker/daemon.json`:
```json
{
  "data-root": "/path/to/large/disk",
  "storage-driver": "overlay2",
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  },
  "default-ulimits": {
    "nofile": {
      "Name": "nofile",
      "Hard": 64000,
      "Soft": 64000
    }
  }
}
```

Restart Docker:
```bash
sudo systemctl restart docker
```

### 3. Monitoring Disk Usage

Add to runner's crontab:
```bash
crontab -e
```

Add:
```
# Check disk space every hour, alert if < 20GB free
0 * * * * df -h / | awk 'NR==2 {if ($4 < 20) print "Low disk space: "$4" free"}' | logger -t github-runner
```

## Workflow Usage

### Triggering ARM64 Builds

1. **Go to:** https://github.com/tzervas/embeddenator/actions/workflows/build-push-arm64.yml
2. **Click:** "Run workflow"
3. **Select:**
   - Runner type: `large` or `multi` based on your setup
   - OS selections: e.g., `debian-stable-arm64,ubuntu-stable-arm64`
   - Push to GHCR: `true`
4. **Click:** "Run workflow"

### Example Configurations

**Quick test (single config):**
```
os_selections: debian-stable-arm64
runner_type: large
push_to_ghcr: false
run_tests: true
```

**Production build (all configs):**
```
os_selections: debian-stable-arm64,debian-testing-arm64,ubuntu-stable-arm64,ubuntu-testing-arm64
runner_type: large
push_to_ghcr: true
run_tests: true
```

**Distributed build (multiple runners):**
```
os_selections: debian-stable-arm64,debian-testing-arm64,ubuntu-stable-arm64,ubuntu-testing-arm64
runner_type: multi
push_to_ghcr: true
run_tests: false
```

## Troubleshooting

### Check Runner Status
```bash
# Via systemd
sudo systemctl status actions.runner.tzervas-embeddenator.arm64-*

# Via runner script
cd ~/actions-runner-arm64-*/
./run.sh --check
```

### View Runner Logs
```bash
# Systemd logs
sudo journalctl -u actions.runner.tzervas-embeddenator.arm64-large-runner -f

# Runner logs
tail -f ~/actions-runner-arm64-*/_diag/*.log
```

### Check Disk Space
```bash
df -h
docker system df
docker system df -v
```

### Clean Everything (Emergency)
```bash
# Stop all runners
sudo systemctl stop actions.runner.tzervas-embeddenator.arm64-*

# Clean Docker
docker system prune -a -f --volumes

# Restart runners
sudo systemctl start actions.runner.tzervas-embeddenator.arm64-*
```

### QEMU Performance Issues

If builds are too slow with QEMU emulation:

1. **Reduce parallelism:** Use `max-parallel: 1` or `2`
2. **Increase CPU allocation:** Dedicate more cores
3. **Use KVM acceleration:** Ensure `/dev/kvm` is accessible
4. **Consider native ARM64:** Cloud instances (AWS Graviton, Oracle ARM) or SBC (Raspberry Pi 5, Rock5)

## Performance Expectations

### Native ARM64:
- Build time per image: 8-12 minutes
- 4 images in parallel: ~15 minutes total

### QEMU Emulation (10 cores):
- Build time per image: 25-35 minutes
- 4 images with max-parallel: 2: ~60 minutes total
- 4 images with max-parallel: 4: ~45 minutes total (higher load)

### Resource Usage During Build:
- CPU: 80-100% of allocated cores
- RAM: 4-8GB per parallel build
- Disk: 15-25GB per build (cleaned up after)

## Security Considerations

1. **Runner Isolation:** Each runner should run in its own user context
2. **Network Access:** Runners need access to GitHub and GHCR
3. **Secrets:** Never log secrets; use GitHub's secret management
4. **Updates:** Keep runner software updated
5. **Monitoring:** Set up alerts for unusual activity

### Best Practices:

#### 1. Use Dedicated User Account
```bash
# Create dedicated user for runners
sudo useradd -m -s /bin/bash github-runner
sudo usermod -aG docker github-runner

# Run runner installation as this user
sudo -u github-runner bash
```

#### 2. Network Firewall Configuration
```bash
# Only allow outbound HTTPS to GitHub
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow from 192.168.1.0/24 to any port 22  # SSH from local network only
sudo ufw enable
```

#### 3. Rotate Registration Tokens
- Never use long-lived tokens
- Tokens expire after 1 hour
- Use the runner_manager.py automation for automatic token management

#### 4. Secure Secrets Storage
```bash
# Store GitHub PAT securely
chmod 600 ~/.env
# Never commit .env to version control
```

## Maintenance

### Weekly Tasks:
- Check disk usage
- Review runner logs for errors
- Update runner software if new version available

### Monthly Tasks:
- Prune Docker system completely
- Review and optimize resource allocations
- Check for OS/package updates

## Uninstalling

```bash
# Stop and remove service
cd ~/actions-runner-arm64-*
sudo ./svc.sh stop
sudo ./svc.sh uninstall

# Remove runner from GitHub (--token needed)
./config.sh remove --token YOUR_TOKEN

# Clean up
cd ~
rm -rf ~/actions-runner-arm64-*
```

## Validation and Testing

After setting up your ARM64 runner, follow these validation steps to ensure everything works correctly.

### Phase 1: Basic Validation

#### 1.1 Verify Runner Registration
```bash
# Check runner appears in GitHub
# Go to: https://github.com/tzervas/embeddenator/settings/actions/runners
# Look for your runner name (e.g., arm64-runner-1) with "Idle" status
```

#### 1.2 Test Architecture Detection
```bash
# Create a simple test workflow or manually trigger ci-arm64.yml
# Check the "Verify Architecture" step output
# Expected: uname -m shows "aarch64" or "arm64"
```

#### 1.3 Verify Docker Access
```bash
# On the runner machine
docker run --rm hello-world
docker run --rm --platform linux/arm64 arm64v8/ubuntu uname -m
# Should output: aarch64
```

### Phase 2: CI Workflow Testing

#### 2.1 Manual Workflow Trigger
1. Navigate to: https://github.com/tzervas/embeddenator/actions/workflows/ci-arm64.yml
2. Click "Run workflow" button
3. Select branch (e.g., `main`)
4. Click "Run workflow"

#### 2.2 Monitor Execution
Watch the workflow run and verify:
- ✅ Runner picks up the job (status changes from "Queued" to "Running")
- ✅ Architecture detection shows aarch64
- ✅ Rust build completes successfully
- ✅ All tests pass
- ✅ Integration tests complete
- ✅ Build artifacts are uploaded (if job fails)

#### 2.3 Expected Timings
- **Native ARM64:** 8-15 minutes for full test suite
- **QEMU Emulation:** 25-45 minutes (depending on CPU cores)

### Phase 3: Performance Validation

#### 3.1 Run Benchmarks
```bash
# On the runner machine, clone and test locally
git clone https://github.com/tzervas/embeddenator.git
cd embeddenator
time cargo build --release
time cargo test --all --verbose
```

#### 3.2 Check Resource Usage During Build
```bash
# In another terminal while build is running
htop  # Monitor CPU usage
free -h  # Monitor memory
df -h  # Monitor disk space
docker stats  # Monitor Docker containers
```

#### 3.3 Expected Resource Usage
- **CPU:** 80-100% of allocated cores during compilation
- **Memory:** 4-8GB during peak (with 4-8 cores)
- **Disk I/O:** Moderate to high during dependency downloads
- **Network:** Spikes during crate downloads from crates.io

### Phase 4: Multi-Runner Testing (if applicable)

#### 4.1 Deploy Multiple Runners
```bash
# Using runner_manager.py
RUNNER_COUNT=2 python3 runner_manager.py run

# Or manually deploy 2 separate runners
```

#### 4.2 Trigger Parallel Workflows
Create 2-3 workflow runs simultaneously and verify:
- ✅ Jobs are distributed across runners
- ✅ Both runners show "Busy" status
- ✅ No job failures due to resource contention
- ✅ Build times are consistent across runners

### Phase 5: Failure Recovery Testing

#### 5.1 Test Runner Crash Recovery (with automation)
```bash
# If using runner_manager.py with auto mode
# Find the runner process and kill it
ps aux | grep runner
kill -9 <PID>

# Verify:
# - runner_manager.py detects failure
# - Automatic restart attempt (if configured)
# - Runner re-registers and becomes available
```

#### 5.2 Test Network Interruption
```bash
# Simulate network loss
sudo iptables -A OUTPUT -p tcp --dport 443 -j DROP

# Wait 2 minutes, then restore
sudo iptables -D OUTPUT -p tcp --dport 443 -j DROP

# Verify:
# - Runner reconnects automatically
# - Queued jobs execute after reconnection
```

#### 5.3 Test Disk Space Exhaustion
```bash
# Fill disk to >90% (create large file in /tmp)
dd if=/dev/zero of=/tmp/largefile bs=1M count=10000

# Trigger a build and verify:
# - Build fails gracefully with clear error
# - Cleanup scripts run (if configured)
# - After cleanup, runner becomes available again

# Clean up test file
rm /tmp/largefile
```

### Phase 6: Security Validation

#### 6.1 Verify Runner Isolation
```bash
# During a workflow run, check process ownership
ps aux | grep runner

# Verify runner process runs as non-root user
# Verify Docker containers run in separate namespace
```

#### 6.2 Check Secret Handling
```bash
# Add a test secret to repository
# Create a workflow that uses the secret
# After workflow completes, verify:
# - Secret not visible in logs
# - Secret not persisted in filesystem
# - Environment cleaned after job
```

#### 6.3 Audit Runner Logs
```bash
# Check for security events
sudo journalctl -u actions.runner.* | grep -i "error\|fail\|security"

# Review runner diagnostic logs
ls -la ~/actions-runner-*/_diag/
tail -100 ~/actions-runner-*/_diag/Runner_*.log
```

### Phase 7: Production Readiness Checklist

Before enabling automatic triggering on the main branch:

- [ ] ✅ Manual workflow runs complete successfully 3+ times
- [ ] ✅ All tests pass consistently
- [ ] ✅ Build time within acceptable range (<15 min for native ARM64)
- [ ] ✅ Resource usage monitored and within limits
- [ ] ✅ Disk cleanup automation working
- [ ] ✅ Security validation passed
- [ ] ✅ Failure recovery tested
- [ ] ✅ Runner uptime >99% over 1 week test period
- [ ] ✅ Documentation reviewed and accurate
- [ ] ✅ Team trained on monitoring and troubleshooting

### Troubleshooting Validation Issues

#### Issue: Runner doesn't pick up jobs
**Diagnosis:**
```bash
# Check runner status
sudo systemctl status actions.runner.*

# Check runner connectivity
curl -I https://api.github.com

# Verify labels match workflow requirements
# Workflow requires: ["self-hosted", "linux", "ARM64"]
```

**Resolution:**
- Verify runner is online in GitHub UI
- Check runner labels match workflow exactly
- Restart runner service
- Check firewall rules

#### Issue: Tests fail on ARM64 but pass on AMD64
**Diagnosis:**
```bash
# Run specific failing test locally
cargo test <test_name> --verbose -- --nocapture

# Check for architecture-specific code
grep -r "cfg(target_arch" src/
```

**Resolution:**
- Review test output for architecture-specific failures
- Check for endianness issues
- Verify all dependencies support ARM64
- Report architecture-specific bugs in GitHub issues

#### Issue: Build times too slow
**Diagnosis:**
```bash
# Check if using emulation vs native
uname -m  # Should show "aarch64" for native

# Check CPU allocation
nproc
lscpu

# Check for throttling
dmesg | grep -i throttl
```

**Resolution:**
- If emulated: Consider native ARM64 hardware
- Increase CPU core allocation
- Enable build caching
- Reduce parallelism if memory constrained

## Post-Setup Next Steps

After validation is complete:

1. **Enable Auto-Trigger** (see TASK-005 in TASK_REGISTRY.md):
   ```yaml
   # In .github/workflows/ci-arm64.yml
   on:
     push:
       branches: [main]  # Enable automatic runs on main
     workflow_dispatch:  # Keep manual trigger option
   ```

2. **Set up Monitoring**:
   - GitHub Actions dashboard for run history
   - Runner health checks (if using runner_manager.py)
   - Disk space alerts
   - Build time tracking

3. **Document Your Setup**:
   - Record runner configuration details
   - Document any custom modifications
   - Note performance characteristics
   - Create runbook for common operations

4. **Train Team**:
   - Share troubleshooting procedures
   - Document escalation paths
   - Establish maintenance schedule

## Additional Resources

- [GitHub Actions Self-Hosted Runner Docs](https://docs.github.com/en/actions/hosting-your-own-runners)
- [Runner Automation Guide](../../docs/RUNNER_AUTOMATION.md)
- [Self-Hosted CI Project Spec](../../docs/SELF_HOSTED_CI_PROJECT_SPEC.md)
- [CI/CD Workflow Documentation](./README.md)
- [Embeddenator Task Registry](../../TASK_REGISTRY.md)

---

**Last Updated:** 2025-12-22  
**Document Version:** 2.0  
**Status:** Production Ready
