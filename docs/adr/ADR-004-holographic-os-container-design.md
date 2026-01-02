# ADR-004: Holographic OS Container Design

## Status

Accepted

## Date

2025-12-15

## Context

The Embeddenator project needed a way to:
- Demonstrate holographic encoding at scale
- Validate bit-perfect reconstruction with complex real-world data
- Provide practical use cases beyond toy examples
- Test the system with diverse file types (binaries, configs, scripts)
- Prove the technology works with production-scale data (GB-range)

Traditional test approaches had limitations:
- Unit tests validate individual components but not end-to-end workflows
- Synthetic test data doesn't exercise real-world edge cases
- Small test files don't stress memory and performance characteristics
- Docker images seemed like a perfect test case: diverse, structured, reproducible

## Decision

We implemented a Holographic OS Container system with the following design:

### 1. Configuration-Driven Builder
**Python orchestrator** (`build_holographic_os.py`):
- YAML configuration file (`os_config.yaml`)
- Support for multiple OS distributions and architectures
- Automated extraction → encoding → validation → rebuild pipeline
- Version management integration with Cargo.toml

### 2. Supported Configurations (8 total)
**Debian**:
- `debian-stable` (amd64, arm64) - Debian 12 "Bookworm" LTS
- `debian-testing` (amd64, arm64) - Latest testing packages

**Ubuntu**:
- `ubuntu-stable` (amd64, arm64) - Ubuntu 24.04 LTS "Noble Numbat"
- `ubuntu-devel` (amd64, arm64) - Latest development release

### 3. Dual Versioning Strategy
**LTS + Nightly releases**:
- **LTS tags**: Stable versions tied to Embeddenator releases (e.g., `v0.2.0`)
- **Nightly tags**: Latest bleeding-edge builds (`nightly-20251222`)
- **Custom suffixes**: Support for `-dev`, `-rc1`, etc.

### 4. Validation Pipeline
**Multi-stage verification**:
1. Extract base OS rootfs from Docker image
2. Ingest to engram format with 4KB chunks
3. Generate manifest with file metadata
4. Validate engram integrity
5. Extract from engram to new directory
6. Rebuild Docker image from reconstructed filesystem
7. Test reconstructed container (basic smoke tests)

### 5. GitHub Actions Integration
**Parameterized workflow** (`build-push-images.yml`):
- Dropdown OS selection (configurable, multi-select)
- Tag suffix input for versioning flexibility
- Optional test suite execution
- Automatic GHCR push with authentication
- Matrix builds for multi-architecture support

## Consequences

### Positive

- **Real-World Validation**: OS containers are non-trivial test cases
  - Thousands of files with diverse types
  - Mix of text configs, binaries, symlinks, permissions
  - Size range: 100MB-500MB compressed, GB+ uncompressed
  - If it works for entire OS, it works for smaller datasets

- **Demonstrable Technology**: Concrete use case
  - "Entire Debian in a single engram" is compelling
  - Easy to understand and visualize
  - Practical applications obvious (backup, versioning, diff)

- **Reproducible Testing**: Docker ensures consistency
  - Same base images across environments
  - Deterministic file contents
  - Easy to run validation locally or in CI

- **Multi-Distribution Support**: Proves generality
  - Works across Debian and Ubuntu
  - Both LTS and rolling release tested
  - AMD64 and ARM64 architectures

- **CI Integration**: Automated quality gates
  - Every commit can trigger OS build test
  - Matrix builds validate multi-arch
  - GHCR provides artifact storage

### Negative

- **Build Time**: OS ingestion is resource-intensive
  - 5-15 minutes per OS configuration
  - Multiplied by architecture (2x) = 10-30 minutes total
  - Can be slow for iterative development

- **Storage Requirements**: Engrams + Docker images = significant space
  - Each OS engram: 200-400MB
  - Docker images: 100-500MB each
  - Artifacts accumulate over time
  - Need cleanup strategy

- **Maintenance Burden**: Multiple OS versions to track
  - Debian/Ubuntu base images update regularly
  - Need to validate on each base image change
  - Configuration drift possible

- **Complexity**: More moving parts than simple tests
  - Python orchestrator, YAML config, Docker, workflows
  - Debugging failures requires understanding full pipeline
  - New contributors face steeper learning curve

### Neutral

- **GHCR as Registry**: Ties us to GitHub ecosystem
  - Could push to Docker Hub or other registries instead
  - GitHub Container Registry convenient for GitHub-hosted project
  - Private visibility possible if needed

- **Flexibility vs Consistency**: Configuration tradeoff
  - YAML allows easy addition of new OS versions
  - But more configurations = more test permutations
  - Need to balance coverage vs CI time

## Use Cases Enabled

1. **Technology Demonstration**
   - Show Embeddenator can handle real workloads
   - Proof of bit-perfect reconstruction at scale

2. **Regression Testing**
   - Ensure encoding/decoding works across versions
   - Catch breaking changes in VSA implementation

3. **Performance Benchmarking**
   - Track ingestion and extraction times
   - Monitor memory usage at scale
   - Identify performance regressions

4. **Platform Validation**
   - Verify multi-architecture support
   - Test on actual diverse filesystems
   - Validate permission/metadata handling

## Future Enhancements

- Hierarchical encoding for OS containers (when TASK-006 complete)
- Delta encoding for OS version updates
- Compression integration (when TASK-008 complete)
- More OS distributions (Alpine, Fedora, Arch)
- **Package isolation and factoralization** (see ADR-005):
  - Isolate individual packages from holographic OS containers
  - Bundle everything except target package(s) for selective updates
  - Factorialized hologram representation using balanced ternary encoding
  - Enable package updates without full container reconstruction
  - Differential distribution of package updates

## Related Work

### Package Isolation via Hologram Factoralization

A key advanced feature for holographic OS containers is the ability to isolate and manipulate individual packages without full reconstruction. This is described in detail in [ADR-005: Hologram-Based Package Isolation and Factoralization](ADR-005-hologram-package-isolation.md).

**Core Concept**: Given a holographic container with N packages, we can:
1. **Isolate** a target package by bundling all other packages into a complementary hologram
2. **Factorialize** the representation into target + complementary pair
3. **Encode** using balanced ternary for compact storage (~39× compression)
4. **Update** by replacing just the target hologram and rebundling

**Key Benefits for OS Containers**:
- **Selective Package Updates**: Update nginx without touching the rest of the system
- **Differential Distribution**: Ship only updated packages as compact holograms (~40 bytes)
- **A/B Testing**: Test multiple package versions against same base system
- **Package Removal**: Use complementary hologram as new base (target excluded)
- **Hardware Efficiency**: Optimized for 64-bit registers (40 trits per register)

**Example Use Case**:
```bash
# Factorialize Debian container to isolate Python package
embeddenator factorialize \
  --engram debian-12.engram \
  --package python3.11 \
  --output python-isolated.hologram \
  --complementary debian-12-no-python.hologram

# Update to Python 3.12
embeddenator bundle \
  --base debian-12-no-python.hologram \
  --package python3.12.hologram \
  --output debian-12-python312.engram
```

This enables OS containers to support package-level granularity while maintaining the holographic algebraic properties.

## References

- build_holographic_os.py - Orchestrator implementation
- os_config.yaml - Configuration file
- .github/workflows/build-push-images.yml - CI workflow
- .github/workflows/build-holographic-os.yml - Alternative workflow
- Dockerfile.holographic - Container definition
- tests/e2e_regression.rs - End-to-end validation tests
- ADR-005: Hologram-Based Package Isolation and Factoralization
