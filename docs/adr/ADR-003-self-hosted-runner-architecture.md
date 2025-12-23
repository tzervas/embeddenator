# ADR-003: Self-Hosted Runner Architecture

## Status

Accepted

## Date

2025-12-22

## Context

The Embeddenator project needed a CI/CD infrastructure that could:
- Support multi-architecture builds (AMD64, ARM64)
- Handle intensive workloads (OS container builds, comprehensive testing)
- Provide cost-effective continuous integration
- Enable GPU-accelerated testing capabilities
- Scale dynamically based on workload

GitHub-hosted runners had limitations:
- ARM64 support requires emulation (QEMU), which is extremely slow (40+ minutes per build)
- No native GPU support
- Limited resources for intensive builds
- Cost scaling for private repositories
- Cannot customize hardware configuration

## Decision

We designed a self-hosted runner architecture with the following components:

### 1. Runner Management System
**Python-based automation** (`runner_automation/` package):
- Multi-platform support (GitHub, GitLab, Gitea)
- Lifecycle management (register, start, stop, monitor)
- Auto-scaling capabilities (lazy, balanced, aggressive modes)
- Resource optimization with CPU affinity and memory limits

### 2. Hardware Capabilities Database
**Comprehensive hardware detection** (`hardware_capabilities.py`):
- 104 CPU models (Intel 10th gen+, AMD Zen 1-5, Xeon, EPYC)
- 117 GPU models (NVIDIA Blackwell-Turing, AMD MI300X-Vega, Intel Arc, Apple M1-M3)
- Automatic capability classification (inference vs training)
- Workload-appropriate runner assignment

### 3. Multi-Architecture Support
**Native and emulated execution**:
- Native: ARM64 runners on ARM64 hardware
- Emulated: ARM64 on x86_64 via QEMU with binfmt_misc
- Container runtimes: Docker, Podman, or standalone QEMU
- Architecture detection and validation

### 4. Deployment Configurations
**Flexible runner setups**:
- **Standard ARM64**: 4-8 cores, 8-16GB RAM for regular builds
- **Large ARM64**: 10+ cores, 16GB+ RAM for OS container builds
- **Multi-runner**: Multiple 4-core instances for parallel execution
- **GPU-enabled**: Specialized runners with GPU passthrough

### 5. Lifecycle Modes
**Configurable runner behavior**:
- **Auto mode**: Deregister after idle timeout (cost-optimized)
- **Manual mode**: Persistent until explicitly stopped
- **Ephemeral mode**: Single-job runners for clean state

## Consequences

### Positive

- **Native ARM64 Performance**: No emulation overhead
  - Build times: < 5 minutes (vs 40+ minutes with QEMU)
  - Can actually run ARM64 CI in practical timeframes
  - Enables ARM64 as a viable platform target

- **Cost Control**: Pay only for hardware you own/rent
  - No per-minute charges for compute
  - Can use existing hardware
  - Cloud instances more economical for sustained workloads

- **Hardware Flexibility**: Choose appropriate resources per workload
  - Assign GPU runners to GPU-dependent tests
  - Scale CPU/memory for different build types
  - Optimize cost vs performance trade-offs

- **Multi-Platform Support**: Not locked to GitHub
  - Works with GitLab, Gitea, or other platforms
  - Consistent runner management across platforms
  - Easy migration if needed

- **Auto-Scaling**: Efficient resource utilization
  - Scale up for busy periods
  - Scale down to save costs
  - Load balancing across runner pool

### Negative

- **Infrastructure Management**: Requires maintaining hardware/VMs
  - Installation and configuration overhead
  - Monitoring and maintenance burden
  - Security updates and patching
  - Network and storage management

- **Upfront Cost**: Need to provision hardware
  - Initial investment in hardware or cloud VMs
  - Cannot start with zero infrastructure
  - Must estimate capacity needs

- **Complexity**: More components than GitHub-hosted
  - Runner registration and token management
  - Network configuration for GitHub connectivity
  - Debugging runner-specific issues
  - Version compatibility tracking

- **Availability**: Self-managed uptime
  - No SLA from GitHub for runner availability
  - Must handle hardware failures
  - Network outages affect CI

### Neutral

- **Hybrid Approach Possible**: Can mix hosted and self-hosted
  - AMD64 on GitHub-hosted (fast enough)
  - ARM64 on self-hosted (native performance)
  - GPU tests on specialized self-hosted runners
  - Cost-optimize per workflow type

## Implementation Details

### Runner Manager (`runner_manager.py`)
```bash
# Example: Auto mode with ARM64 emulation
# - RUNNER_TARGET_ARCHITECTURES: Which architectures to support (arm64, x64, riscv64)
# - RUNNER_ENABLE_EMULATION: Enable QEMU emulation for non-native architectures (default: false)
# - RUNNER_MODE: Runner lifecycle mode (auto|manual, default: manual)
RUNNER_TARGET_ARCHITECTURES=arm64 \
RUNNER_ENABLE_EMULATION=true \
RUNNER_MODE=auto \
python3 runner_manager.py run
```

### Deployment Validation (7 phases)
1. Basic installation and connectivity
2. CI workflow execution
3. Performance benchmarking
4. Multi-runner coordination
5. Failure recovery
6. Security hardening
7. Production readiness checklist

### Security Considerations
- Dedicated runner user with minimal privileges
- Firewall rules limiting outbound connections
- Short-lived registration tokens
- Secrets stored in GitHub (not on runner)
- Regular security updates
- Isolated execution environments

## References

- .github/workflows/ARM64_RUNNER_SETUP.md - Complete deployment guide
- runner_automation/ - Python automation package
- docs/RUNNER_AUTOMATION.md - Architecture documentation
- docs/SELF_HOSTED_CI_PROJECT_SPEC.md - Project specification
- .github/workflows/ci-arm64.yml - ARM64 workflow configuration
