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
