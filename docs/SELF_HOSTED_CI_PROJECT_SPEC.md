# Self-Hosted CI Infrastructure Project Specification

**Project:** Embeddenator Self-Hosted CI/CD System  
**Version:** 1.0  
**Last Updated:** 2025-12-22  
**Status:** Active Development  
**Document Owner:** DevOps Engineering Team

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Project Overview](#project-overview)
3. [Current State](#current-state)
4. [System Architecture](#system-architecture)
5. [Requirements](#requirements)
6. [Runner Types and Configurations](#runner-types-and-configurations)
7. [Testing and Validation](#testing-and-validation)
8. [Future Roadmap](#future-roadmap)
9. [Appendix](#appendix)

---

## Executive Summary

The Embeddenator project requires self-hosted GitHub Actions runners to support multi-architecture builds (ARM64, x86_64) and specialized hardware (GPU) that are not available in GitHub's standard hosted runner fleet. This document specifies the design, implementation, and operational requirements for a robust, automated self-hosted CI infrastructure.

### Key Objectives

1. **Multi-Architecture Support**: Native ARM64 builds without emulation overhead
2. **GPU Acceleration**: Support for GPU-accelerated workloads in future development
3. **Cost Optimization**: Auto-scaling runners with idle timeout management
4. **Automation**: Fully automated runner lifecycle management
5. **Reliability**: High availability with automatic recovery and health monitoring

### Success Criteria

- ✅ ARM64 CI workflow completes successfully on self-hosted runners
- ✅ Runner automation system manages lifecycle without manual intervention
- ✅ Cost savings >50% compared to cloud-based alternatives
- ✅ Build time <15 minutes for full test suite on ARM64
- ✅ Zero manual runner management required during normal operation

---

## Project Overview

### Background

Embeddenator is a Rust-based holographic computing substrate that requires testing and building on multiple architectures:

- **AMD64 (x86_64)**: Primary architecture, fully supported by GitHub-hosted runners
- **ARM64 (aarch64)**: Required for multi-arch Docker images, not available in standard GitHub runners
- **GPU Support**: Future requirement for VSA acceleration research

The project previously attempted to use ARM64 GitHub-hosted runners, but these do not exist in the standard offering. QEMU emulation on x86_64 runners proved too slow (5-10x overhead) and unreliable for CI/CD purposes.

### Solution Approach

Implement self-hosted runner infrastructure with:

1. **Automated Runner Management**: Python-based lifecycle automation (`runner_manager.py`)
2. **Multi-Runner Support**: Deploy multiple runners for parallel builds
3. **Cost Optimization**: Auto-deregistration after idle timeout
4. **Cross-Architecture Support**: Native execution on target architectures
5. **GPU Capability**: Infrastructure ready for GPU-accelerated workloads

---

## Current State

### Completed ✅

- **v0.1.0 Release**: Core VSA implementation with AMD64 CI
- **v0.2.0 Release**: Comprehensive test suite, clippy fixes
- **Runner Automation Framework**: Complete Python-based automation system
  - Auto-registration with short-lived tokens
  - Lifecycle management (register → run → monitor → deregister)
  - Multi-runner deployment support
  - Multi-architecture support (x64, ARM64, RISC-V)
  - QEMU emulation support for cross-architecture testing
  - GPU runner support with hardware detection
  - Auto-scaling based on workload
- **CI/CD Workflow Separation**: Three workflows (pre-checks, amd64, arm64)
- **Documentation**: Comprehensive README, workflow docs, runner automation guide

### In Progress 🚧

- **ARM64 Infrastructure Deployment**: Hardware/VM provisioning
- **Self-Hosted Runner Deployment**: Physical deployment pending
- **ARM64 CI Testing**: Validation workflow ready but untested

### Pending ⏳

- **ARM64 Auto-Trigger**: Enable automatic runs on main branch
- **GPU Runner Configuration**: Hardware acquisition and setup
- **Production Monitoring**: Observability and alerting setup

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     GitHub Actions                          │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Pre-Checks   │  │ AMD64 Build  │  │ ARM64 Build  │    │
│  │ (hosted)     │  │ (hosted)     │  │ (self-hosted)│    │
│  └──────────────┘  └──────────────┘  └──────┬───────┘    │
│                                              │             │
└──────────────────────────────────────────────┼─────────────┘
                                               │
                                               │ HTTPS
                                               │
┌──────────────────────────────────────────────▼─────────────┐
│              Self-Hosted Infrastructure                     │
│                                                             │
│  ┌────────────────────────────────────────────────────┐   │
│  │           Runner Manager (runner_manager.py)       │   │
│  │  - Registration automation                         │   │
│  │  - Lifecycle management                           │   │
│  │  - Health monitoring                              │   │
│  │  - Auto-scaling                                   │   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ ARM64 Runner │  │ ARM64 Runner │  │ GPU Runner   │    │
│  │ #1 (4 core) │  │ #2 (4 core) │  │ (8 core+GPU) │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐ │
│  │           Shared Storage & Cache                     │ │
│  │  - Build artifacts                                   │ │
│  │  - Cargo registry cache                              │ │
│  │  - Docker layer cache                                │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐ │
│  │           Monitoring & Logging                       │ │
│  │  - Runner health metrics                             │ │
│  │  - Build performance tracking                        │ │
│  │  - Resource utilization                              │ │
│  └──────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Component Architecture

#### 1. Runner Manager (`runner_manager.py`)

**Purpose**: Centralized automation for runner lifecycle management

**Modules**:
- `runner_automation/config.py`: Configuration management
- `runner_automation/github_api.py`: GitHub API client
- `runner_automation/installer.py`: Runner installation
- `runner_automation/runner.py`: Individual runner lifecycle
- `runner_automation/manager.py`: Multi-runner orchestration
- `runner_automation/emulation.py`: QEMU cross-architecture support
- `runner_automation/cli.py`: Command-line interface

**Key Features**:
- Token management (registration tokens, PAT validation)
- Multi-runner coordination
- Health checks and monitoring
- Automatic cleanup and recovery
- Cross-architecture emulation support
- GPU hardware detection and management

---

## Requirements

### Functional Requirements

#### FR-1: Multi-Architecture Build Support
- **Priority**: P0 (Critical)
- **Description**: Support native builds on ARM64 architecture
- **Acceptance**: ARM64 CI workflow completes successfully with all tests passing

#### FR-2: Automated Runner Lifecycle
- **Priority**: P0 (Critical)
- **Description**: Automated registration, startup, monitoring, and deregistration
- **Acceptance**: Zero manual intervention required for normal operations

#### FR-3: Multi-Runner Deployment
- **Priority**: P1 (High)
- **Description**: Deploy and manage multiple runners simultaneously
- **Acceptance**: Support 2+ concurrent runners with independent lifecycles

#### FR-4: Auto-Scaling
- **Priority**: P1 (High)
- **Description**: Scale runners based on workload and idle timeout
- **Acceptance**: Runners auto-deregister after configurable idle period

#### FR-5: GPU Support
- **Priority**: P2 (Medium)
- **Description**: Support GPU-accelerated workloads
- **Acceptance**: GPU runner can be deployed and detected by workflows

### Non-Functional Requirements

#### NFR-1: Performance
- **Build Time**: Full test suite <15 minutes on ARM64
- **Startup Time**: Runner registration and startup <2 minutes
- **Network**: HTTPS connection to GitHub Actions <100ms latency

#### NFR-2: Reliability
- **Uptime**: 99.5% runner availability during business hours
- **Auto-Recovery**: Automatic restart on runner failure
- **Monitoring**: Health checks every 30 seconds

#### NFR-3: Security
- **Token Management**: Short-lived registration tokens (<1 hour)
- **Access Control**: Runners in private network with minimal exposure
- **Secrets**: No secrets stored in runner filesystem
- **Updates**: Automatic security updates for runner software

---

## Runner Types and Configurations

### Standard ARM64 Runner

**Specifications**:
- **Architecture**: ARM64 (aarch64)
- **CPU**: 4 cores (minimum), 8 cores (recommended)
- **Memory**: 8GB (minimum), 16GB (recommended)
- **Storage**: 100GB SSD (minimum), 200GB (recommended)
- **Network**: 100Mbps (minimum)

**Labels**: `["self-hosted", "linux", "ARM64"]`

**Use Cases**:
- ARM64 CI workflow builds
- ARM64 Docker image builds
- ARM64 integration tests

---

## Testing and Validation

### Deployment Validation

#### Phase 1: Manual Testing
1. Deploy single ARM64 runner
2. Manually trigger `ci-arm64.yml` workflow
3. Verify architecture detection (`uname -m` → `aarch64`)
4. Confirm all tests pass
5. Validate artifact generation

**Success Criteria**:
- ✅ Workflow completes in <15 minutes
- ✅ All tests pass
- ✅ Artifacts uploaded correctly

#### Phase 2: Auto Mode Testing
1. Enable auto mode with 5-minute idle timeout
2. Trigger multiple workflows back-to-back
3. Let runner idle and verify auto-deregistration
4. Trigger new workflow and verify auto-registration

**Success Criteria**:
- ✅ Auto-deregistration after idle timeout
- ✅ Re-registration on new job
- ✅ No manual intervention required

---

## Future Roadmap

### Phase 1: Current (2025 Q1)
- ✅ Runner automation framework complete
- 🚧 ARM64 runner deployment
- ⏳ ARM64 CI enablement

### Phase 2: GPU Support (2025 Q2)
- GPU runner deployment
- CUDA/ROCm environment setup
- GPU-accelerated VSA prototypes

### Phase 3: Advanced Features (2025 Q3)
- Auto-scaling based on queue depth
- Multi-region runner deployment
- Advanced caching strategies

---

## Appendix

### A. Reference Documentation

- [GitHub Actions Runner Documentation](https://docs.github.com/en/actions/hosting-your-own-runners)
- [Runner Automation Guide](RUNNER_AUTOMATION.md)
- [ARM64 Setup Guide](../.github/workflows/ARM64_RUNNER_SETUP.md)
- [Workflow README](../.github/workflows/README.md)

---

**Document Status**: ACTIVE  
**Review Schedule**: Quarterly  
**Next Review**: 2025-03-22
