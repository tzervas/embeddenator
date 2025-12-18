# GitHub Actions Runner Automation Guide

This guide covers the comprehensive automation system for managing GitHub Actions self-hosted runners in the Embeddenator project.

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Configuration](#configuration)
4. [Usage Examples](#usage-examples)
5. [Deployment Modes](#deployment-modes)
6. [Advanced Features](#advanced-features)
7. [Troubleshooting](#troubleshooting)
8. [Security Best Practices](#security-best-practices)

## Overview

The `runner_manager.py` script provides complete lifecycle automation for GitHub Actions self-hosted runners:

- ✨ **Automated Registration**: Uses short-lived tokens for secure registration
- 🔄 **Complete Lifecycle Management**: Register → Run → Monitor → Deregister
- ⏱️ **Auto-Deregistration**: Configurable idle timeout for cost optimization
- 🎯 **Manual Mode**: Keep runners alive until explicitly stopped
- 🚀 **Multi-Runner Support**: Deploy multiple runners with different configurations
- 📊 **Health Monitoring**: Track runner status and job queue
- 🧹 **Automatic Cleanup**: Clean up Docker resources and installations
- ⚙️ **Flexible Configuration**: Configure via .env file or CLI arguments

## Quick Start

### 1. Setup Configuration

Copy the example environment file and configure it:

```bash
cp .env.example .env
```

Edit `.env` and set the required variables:

```bash
# Required: Your repository and access token
GITHUB_REPOSITORY=tzervas/embeddenator
GITHUB_TOKEN=ghp_your_personal_access_token_here
```

To create a GitHub Personal Access Token (PAT):
1. Go to https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Select scope: `repo` (Full control of private repositories)
4. Copy the token and add it to your `.env` file

### 2. Run in Auto Mode

The simplest way to get started is to use the `run` command, which handles everything:

```bash
python3 runner_manager.py run
```

This will:
1. Register the runner with GitHub
2. Start the runner service
3. Monitor for jobs
4. Automatically deregister after 5 minutes of inactivity (default)

### 3. Run in Manual Mode

To keep the runner running until you manually stop it:

```bash
RUNNER_MODE=manual python3 runner_manager.py run
```

Or edit `.env`:
```bash
RUNNER_MODE=manual
```

Then run:
```bash
python3 runner_manager.py run
```

To stop:
```bash
# Press Ctrl+C or in another terminal:
python3 runner_manager.py stop
```

## Configuration

### Environment Variables

All configuration can be set via environment variables or the `.env` file. See `.env.example` for the complete list.

#### Essential Configuration

```bash
# Repository to register runners for (required)
GITHUB_REPOSITORY=owner/repository

# GitHub Personal Access Token with repo scope (required)
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx

# Runner name prefix (default: embeddenator-runner)
RUNNER_NAME_PREFIX=my-runner

# Comma-separated labels (default: self-hosted,linux,ARM64)
RUNNER_LABELS=self-hosted,linux,ARM64,gpu
```

#### Lifecycle Configuration

```bash
# Deployment mode: auto | manual (default: auto)
RUNNER_MODE=auto

# Auto-deregister timeout in seconds (default: 300 = 5 minutes)
# Only applies when RUNNER_MODE=auto
RUNNER_IDLE_TIMEOUT=600

# Check interval in seconds (default: 30)
RUNNER_CHECK_INTERVAL=30

# Maximum lifetime in seconds (default: 0 = unlimited)
# Deregister after this time regardless of activity
RUNNER_MAX_LIFETIME=3600
```

#### Multi-Runner Configuration

```bash
# Number of runners to deploy (default: 1)
RUNNER_COUNT=4

# Deployment strategy: sequential | parallel (default: sequential)
RUNNER_DEPLOYMENT_STRATEGY=parallel

# Delay between sequential deployments in seconds (default: 5)
RUNNER_DEPLOYMENT_STAGGER=10
```

#### Advanced Configuration

```bash
# Enable ephemeral runners (deregister after one job)
RUNNER_EPHEMERAL=true

# Replace existing runner with same name
RUNNER_REPLACE_EXISTING=true

# Disable automatic runner updates
RUNNER_DISABLE_AUTO_UPDATE=false

# Log level: DEBUG | INFO | WARNING | ERROR | CRITICAL
LOG_LEVEL=INFO

# Clean installation on deregister
RUNNER_CLEAN_ON_DEREGISTER=true

# Clean Docker resources on deregister
RUNNER_CLEAN_DOCKER=true
```

### CLI Configuration

Override configuration with command-line arguments:

```bash
# Deploy multiple runners
python3 runner_manager.py run --runner-count 4

# Custom labels
python3 runner_manager.py register --labels self-hosted,linux,ARM64,large

# Combine environment and CLI
RUNNER_MODE=auto python3 runner_manager.py run --runner-count 2
```

## Usage Examples

### Example 1: Simple Auto-Deregistering Runner

Perfect for CI/CD pipelines where you want runners to terminate when idle:

```bash
# .env configuration
GITHUB_REPOSITORY=tzervas/embeddenator
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
RUNNER_MODE=auto
RUNNER_IDLE_TIMEOUT=300  # 5 minutes

# Run
python3 runner_manager.py run
```

The runner will:
- Register and start immediately
- Process jobs as they come
- Automatically deregister after 5 minutes with no jobs
- Clean up all resources

### Example 2: Development Runner (Manual Mode)

For persistent development environments:

```bash
# .env configuration
RUNNER_MODE=manual
RUNNER_NAME_PREFIX=dev-runner

# Start
python3 runner_manager.py run

# In another terminal, check status
python3 runner_manager.py status

# Stop when done
python3 runner_manager.py stop
```

### Example 3: Multi-Runner Deployment

Deploy 4 runners for parallel builds:

```bash
# .env configuration
RUNNER_COUNT=4
RUNNER_DEPLOYMENT_STRATEGY=sequential
RUNNER_DEPLOYMENT_STAGGER=10
RUNNER_LABELS=self-hosted,linux,ARM64

# Deploy
python3 runner_manager.py run --runner-count 4
```

This creates:
- `embeddenator-runner-1`
- `embeddenator-runner-2`
- `embeddenator-runner-3`
- `embeddenator-runner-4`

Each runner can process jobs independently.

### Example 4: Ephemeral Runners

Runners that deregister after completing one job:

```bash
# .env configuration
RUNNER_EPHEMERAL=true
RUNNER_MODE=auto

# Run
python3 runner_manager.py run
```

Perfect for:
- Ensuring clean state for each job
- Security-sensitive builds
- Resource isolation

### Example 5: Step-by-Step Control

For more control over the lifecycle:

```bash
# 1. Register runners
python3 runner_manager.py register --runner-count 2

# 2. Start runner processes
python3 runner_manager.py start

# 3. Check status
python3 runner_manager.py status

# 4. Monitor in background (auto mode)
python3 runner_manager.py monitor &

# 5. When done, stop everything
python3 runner_manager.py stop
```

### Example 6: Time-Limited Runner

Runner that automatically deregisters after 1 hour:

```bash
# .env configuration
RUNNER_MODE=auto
RUNNER_MAX_LIFETIME=3600  # 1 hour
RUNNER_IDLE_TIMEOUT=300   # Also stop if idle for 5 minutes

# Run
python3 runner_manager.py run
```

Runner will deregister when:
- Idle for 5 minutes, OR
- Running for 1 hour (whichever comes first)

## Deployment Modes

### Auto Mode (Default)

**Use Case**: Cost optimization, sporadic builds, CI/CD automation

**Behavior**:
- Registers runner with GitHub
- Starts runner service
- Monitors job queue at regular intervals
- Automatically deregisters after idle timeout
- Cleans up resources on exit

**Configuration**:
```bash
RUNNER_MODE=auto
RUNNER_IDLE_TIMEOUT=300  # Seconds
RUNNER_CHECK_INTERVAL=30  # Seconds
```

**Benefits**:
- Cost-effective (only runs when needed)
- Automatic resource management
- Zero maintenance required

**Example Use Cases**:
- Nightly builds
- PR validation
- Scheduled workflows
- Event-triggered deployments

### Manual Mode

**Use Case**: Development, persistent infrastructure, interactive debugging

**Behavior**:
- Registers runner with GitHub
- Starts runner service
- Keeps running until explicitly stopped
- No automatic deregistration
- Health monitoring only

**Configuration**:
```bash
RUNNER_MODE=manual
```

**Benefits**:
- Always available
- Predictable behavior
- Good for development
- Full control over lifecycle

**Example Use Cases**:
- Development environments
- Long-running tasks
- Interactive debugging
- Persistent test environments

## Advanced Features

### Monitoring and Status

Check runner status at any time:

```bash
python3 runner_manager.py status
```

Output includes:
- Runner names and IDs
- Process status (running/stopped)
- GitHub registration status
- Current job status (busy/idle)
- Uptime information
- Queue status

### Logging

Logs are written to both console and file:

```bash
# Check logs
tail -f runner_manager.log

# Increase verbosity
LOG_LEVEL=DEBUG python3 runner_manager.py run
```

### Resource Management

The runner manager automatically monitors and manages resources:

**Disk Space**: 
- Warns when free space < 20GB (configurable)
- Automatically cleans Docker resources when needed

**Docker Cleanup**:
```bash
RUNNER_CLEAN_DOCKER=true
DOCKER_CLEANUP_THRESHOLD_GB=10
```

**Memory Management**:
- Option to set memory limits per runner (requires systemd)

### Signal Handling

The runner manager handles signals gracefully:

- `Ctrl+C` (SIGINT): Graceful shutdown
- `SIGTERM`: Graceful shutdown
- Automatic cleanup on shutdown

### Staggered Deployment

When deploying multiple runners sequentially:

```bash
RUNNER_DEPLOYMENT_STRATEGY=sequential
RUNNER_DEPLOYMENT_STAGGER=10  # Wait 10s between each
```

Prevents:
- Rate limiting from GitHub API
- Resource contention during startup
- Network congestion

### Custom Labels

Runners can have custom labels for targeted workflow execution:

```bash
# .env
RUNNER_LABELS=self-hosted,linux,ARM64,gpu,large

# Or CLI
python3 runner_manager.py register --labels self-hosted,linux,ARM64,gpu
```

Use in workflows:
```yaml
jobs:
  build:
    runs-on: [self-hosted, linux, ARM64, gpu]
```

## Troubleshooting

### Registration Fails

**Problem**: Runner registration fails with token error

**Solution**:
1. Verify `GITHUB_TOKEN` is valid
2. Check token has `repo` scope
3. Ensure repository name is correct: `owner/repo`
4. Token must not be expired

```bash
# Test token
curl -H "Authorization: token $GITHUB_TOKEN" \
  https://api.github.com/repos/tzervas/embeddenator
```

### Runner Not Appearing in GitHub

**Problem**: Runner registered but not visible in GitHub UI

**Solution**:
1. Check GitHub Settings → Actions → Runners
2. Verify runner process is actually running:
   ```bash
   python3 runner_manager.py status
   ```
3. Check logs for errors:
   ```bash
   tail -f runner_manager.log
   ```

### Idle Timeout Not Working

**Problem**: Runner doesn't deregister after idle timeout

**Solution**:
1. Verify mode is set to `auto`:
   ```bash
   grep RUNNER_MODE .env
   ```
2. Check if there are jobs in queue:
   ```bash
   python3 runner_manager.py status
   ```
3. Check logs for monitoring activity:
   ```bash
   grep "Idle for" runner_manager.log
   ```

### Disk Space Issues

**Problem**: Builds fail due to disk space

**Solution**:
1. Enable Docker cleanup:
   ```bash
   RUNNER_CLEAN_DOCKER=true
   ```
2. Lower cleanup threshold:
   ```bash
   DOCKER_CLEANUP_THRESHOLD_GB=15
   ```
3. Manual cleanup:
   ```bash
   docker system prune -a -f
   ```

### Multiple Runners Not Starting

**Problem**: Only first runner starts when deploying multiple

**Solution**:
1. Check for registration errors in logs
2. Try sequential deployment:
   ```bash
   RUNNER_DEPLOYMENT_STRATEGY=sequential
   RUNNER_DEPLOYMENT_STAGGER=15
   ```
3. Increase stagger delay to avoid rate limiting

### Runner Crashed/Stopped Unexpectedly

**Problem**: Runner process stopped but not cleaned up

**Solution**:
1. Stop and cleanup:
   ```bash
   python3 runner_manager.py stop
   ```
2. Manually remove if needed:
   ```bash
   rm -rf actions-runner-*
   ```
3. Start fresh:
   ```bash
   python3 runner_manager.py run
   ```

## Security Best Practices

### Token Management

1. **Use Personal Access Tokens (PAT)**, not OAuth tokens
2. **Limit token scope** to only `repo` access
3. **Store tokens securely**:
   - Never commit `.env` to git (already in `.gitignore`)
   - Use environment variables in production
   - Consider using secrets management systems
4. **Rotate tokens regularly**
5. **Use separate tokens** for different environments

### Runner Security

1. **Run runners in isolated environments**:
   - Dedicated VMs or containers
   - Separate network segments
   - Limited permissions

2. **Keep runners updated**:
   - The script automatically downloads latest runner version
   - Update the script regularly: `git pull`

3. **Use ephemeral runners** for sensitive builds:
   ```bash
   RUNNER_EPHEMERAL=true
   ```

4. **Monitor runner activity**:
   ```bash
   python3 runner_manager.py status
   tail -f runner_manager.log
   ```

5. **Cleanup after use**:
   ```bash
   RUNNER_CLEAN_ON_DEREGISTER=true
   RUNNER_CLEAN_DOCKER=true
   ```

### Network Security

1. **Firewall rules**: Only allow outbound HTTPS (443)
2. **Proxy support**: Configure via environment if needed:
   ```bash
   export https_proxy=http://proxy:8080
   ```
3. **VPN**: Run runners inside VPN for added security

### Audit and Compliance

1. **Enable logging**:
   ```bash
   LOG_LEVEL=INFO
   LOG_FILE=./runner_manager.log
   ```

2. **Review logs regularly**:
   ```bash
   grep ERROR runner_manager.log
   ```

3. **Monitor GitHub audit log**:
   - Settings → Actions → Runners → View audit log

4. **Set maximum lifetime** for compliance:
   ```bash
   RUNNER_MAX_LIFETIME=3600  # 1 hour max
   ```

## Integration Examples

### Systemd Service

Create `/etc/systemd/system/github-runner.service`:

```ini
[Unit]
Description=GitHub Actions Runner Manager
After=network.target

[Service]
Type=simple
User=runner
WorkingDirectory=/home/runner/embeddenator
Environment="PATH=/usr/local/bin:/usr/bin:/bin"
EnvironmentFile=/home/runner/embeddenator/.env
ExecStart=/usr/bin/python3 /home/runner/embeddenator/runner_manager.py run
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable github-runner
sudo systemctl start github-runner
sudo systemctl status github-runner
```

### Cron Job

For scheduled runner deployment:

```bash
# Start runner at 9 AM, auto-deregister when idle
0 9 * * * cd /home/runner/embeddenator && python3 runner_manager.py run
```

### Docker Container

Run the runner manager in a container:

```dockerfile
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    python3 python3-pip curl tar git docker.io

COPY runner_manager.py /app/
COPY .env /app/

WORKDIR /app
CMD ["python3", "runner_manager.py", "run"]
```

Build and run:
```bash
docker build -t runner-manager .
docker run -v /var/run/docker.sock:/var/run/docker.sock \
  --env-file .env runner-manager
```

## Contributing

Improvements and bug fixes are welcome! Please:

1. Test changes thoroughly
2. Update documentation
3. Follow existing code style
4. Submit pull requests to the main repository

## Support

For issues and questions:
- GitHub Issues: https://github.com/tzervas/embeddenator/issues
- Documentation: https://github.com/tzervas/embeddenator
- Workflow Documentation: `.github/workflows/README.md`

## License

This runner automation system is part of the Embeddenator project and is licensed under the MIT License.
