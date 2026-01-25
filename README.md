# Embeddenator Workspace

> **Holographic Computing Substrate** - Multi-component Rust workspace for Vector Symbolic Architecture (VSA)

> ## 🚨 DEPRECATION NOTICE
> 
> **The monolithic `embeddenator` repository in this workspace is DEPRECATED as of v0.20.0-alpha.1 (January 16, 2026).**
>
> All functionality has been migrated to 10 production-ready component repositories. The monolithic crate will be:
> - **Q2 2026 (v0.21.0)**: Archived (read-only, no updates)
> - **Q3 2026 (v1.0.0)**: Removed from workspace
>
> **➡️ For new projects, use component repositories directly (see table below).**  
> **➡️ Existing users: See [DEPRECATION_NOTICE.md](./DEPRECATION_NOTICE.md) for migration guide.**

[![CI](https://github.com/tzervas/embeddenator/workflows/Workspace%20CI/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/ci-workspace.yml)
[![Benchmarks](https://github.com/tzervas/embeddenator/workflows/Benchmark%20Regression%20Check/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/bench-workspace.yml)
[![Health](https://github.com/tzervas/embeddenator/workflows/Workspace%20Health/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/health-workspace.yml)
[![Docs](https://github.com/tzervas/embeddenator/workflows/Documentation/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/docs-workspace.yml)
[![Security](https://github.com/tzervas/embeddenator/workflows/Security%20Scan/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/security-workspace.yml)

![Version](https://img.shields.io/badge/version-0.20.0--alpha.1-orange)
![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.84+-000000?logo=rust)

---

## Overview

This is the **Embeddenator monorepo workspace** containing 11 production-ready component repositories that implement a holographic computing substrate using Vector Symbolic Architecture (VSA).

**Current Status:** All components are **100% production-ready** with comprehensive testing, benchmarking, and documentation.

> **⚠️ MONOLITHIC REPOSITORY DEPRECATED:** The `embeddenator/` directory contains the deprecated monolithic repository. **Do not use it for new projects.** All functionality has been migrated to component repositories listed below. See [DEPRECATION_NOTICE.md](./DEPRECATION_NOTICE.md) for migration information.

---

## Component Repositories

### Core Components

| Component | Description | Repository | Status |
|-----------|-------------|------------|--------|
| **[embeddenator](./embeddenator/)** | Legacy monolithic repo (deprecated) | [GitHub](https://github.com/tzervas/embeddenator) | ⚠️ Deprecated |
| **[embeddenator-vsa](./embeddenator-vsa/)** | Sparse ternary VSA primitives | [GitHub](https://github.com/tzervas/embeddenator-vsa) | ✅ Production |
| **[embeddenator-fs](./embeddenator-fs/)** | Filesystem operations (EmbrFS) | [GitHub](https://github.com/tzervas/embeddenator-fs) | ✅ Production |
| **[embeddenator-io](./embeddenator-io/)** | I/O operations and serialization | [GitHub](https://github.com/tzervas/embeddenator-io) | ✅ Production |
| **[embeddenator-retrieval](./embeddenator-retrieval/)** | Information retrieval and search | [GitHub](https://github.com/tzervas/embeddenator-retrieval) | ✅ Production |

### Supporting Components

| Component | Description | Repository | Status |
|-----------|-------------|------------|--------|
| **[embeddenator-obs](./embeddenator-obs/)** | Observability and metrics | [GitHub](https://github.com/tzervas/embeddenator-obs) | ✅ Production |
| **[embeddenator-interop](./embeddenator-interop/)** | FFI and language interoperability | [GitHub](https://github.com/tzervas/embeddenator-interop) | ✅ Production |
| **[embeddenator-cli](./embeddenator-cli/)** | Command-line interface | [GitHub](https://github.com/tzervas/embeddenator-cli) | ✅ Production |
| **[embeddenator-workspace](./embeddenator-workspace/)** | Workspace management tools | [GitHub](https://github.com/tzervas/embeddenator-workspace) | ✅ Production |
| **[embeddenator-testkit](./embeddenator-testkit/)** | Testing utilities, fixtures, and integration tests | [GitHub](https://github.com/tzervas/embeddenator-testkit) | ✅ Production |
| **[embeddenator-contract-bench](./embeddenator-contract-bench/)** | Contract testing and benchmarks | [GitHub](https://github.com/tzervas/embeddenator-contract-bench) | ✅ Production |

---

## Quick Start

### Prerequisites

- Rust 1.84 or later
- Git

### Clone and Build

```bash
# Clone the workspace
git clone https://github.com/tzervas/embeddenator.git
cd embeddenator

# Build all components
cargo build --release

# Run tests
cargo test --all-features --workspace

# Build documentation
cargo doc --all-features --no-deps --open
```

### Sync All Repositories

```bash
# Update all component repos
./update_all.sh
```

---

## Workspace Management

The workspace includes a powerful CLI tool for managing the monorepo:

```bash
# Build workspace tool
cd embeddenator-workspace
cargo build --release

# Check workspace health
./target/release/embeddenator-workspace health

# Check version consistency
./target/release/embeddenator-workspace check-versions

# Bump versions
./target/release/embeddenator-workspace bump-version --prerelease
```

See [embeddenator-workspace README](./embeddenator-workspace/README.md) for full documentation.

---

## CI/CD Infrastructure

The workspace has comprehensive CI/CD automation via GitHub Actions:

### Workflows

- **Workspace CI** - Tests all 11 components in parallel (~20 min)
- **Benchmark Regression** - Detects performance regressions (~30 min)
- **Workspace Health** - Daily health monitoring with auto-issue creation
- **Documentation** - Builds and deploys docs to GitHub Pages
- **Security Scan** - Weekly vulnerability scanning
- **Release Workflow** - Automated version bumping and releases

See [CI/CD Guide](./CI_CD_GUIDE.md) for comprehensive documentation.

---

## Documentation

- **[Migration Guide](./MIGRATION_GUIDE.md)** - Transition from monolithic to component architecture
- **[CI/CD Guide](./CI_CD_GUIDE.md)** - Complete CI/CD documentation
- **[Workspace Tracker](./WORKSPACE_TRACKER.md)** - Development progress
- **[Component Architecture](./embeddenator/docs/COMPONENT_ARCHITECTURE.md)** - Architecture overview
- **[API Documentation](https://tzervas.github.io/embeddenator/)** - Generated rustdoc

---

## Development

### Running Tests

```bash
# Run all tests
cargo test --all-features --workspace

# Run tests for specific component
cargo test -p embeddenator-vsa

# Run integration tests (part of embeddenator-testkit)
cd embeddenator-testkit
cargo test --features integration
```

### Running Benchmarks

```bash
# Run benchmarks for all components
for repo in embeddenator-*; do
  if [ -d "$repo/benches" ]; then
    cargo bench --manifest-path "$repo/Cargo.toml"
  fi
done

# Run specific benchmark
cargo bench -p embeddenator-vsa
```

### Building Documentation

```bash
# Build all docs
cargo doc --all-features --no-deps

# Build docs for specific component
cargo doc -p embeddenator-fs --all-features --no-deps

# Open documentation
open target/doc/embeddenator/index.html
```

---

## Repository Structure

```
embeddenator/
├── .github/
│   ├── workflows/          # GitHub Actions CI/CD
│   │   ├── ci-workspace.yml
│   │   ├── bench-workspace.yml
│   │   ├── health-workspace.yml
│   │   ├── docs-workspace.yml
│   │   ├── security-workspace.yml
│   │   └── release-workspace.yml
│   ├── CODEOWNERS          # Code ownership
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── ISSUE_TEMPLATE/
├── embeddenator/           # Main application
├── embeddenator-vsa/       # VSA implementation
├── embeddenator-fs/        # Filesystem operations
├── embeddenator-io/        # I/O operations
├── embeddenator-obs/       # Observable pattern
├── embeddenator-retrieval/ # Information retrieval
├── embeddenator-interop/   # FFI bindings
├── embeddenator-cli/       # CLI interface
├── embeddenator-workspace/ # Workspace tools
├── embeddenator-testkit/   # Testing utilities + integration tests
├── embeddenator-contract-bench/ # Contract tests
├── update_all.sh           # Sync script
├── CI_CD_GUIDE.md          # CI/CD documentation
└── README.md               # This file
```

---

## Version Management

All components maintain version consistency. Current version: **0.20.0-alpha.1**

To bump versions:

```bash
cd embeddenator-workspace
cargo build --release
./target/release/embeddenator-workspace bump-version --prerelease
```

See [Versioning Documentation](./embeddenator/docs/VERSIONING.md) for details.

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test --all-features --workspace`)
5. Run health check (`./embeddenator-workspace/target/release/embeddenator-workspace health`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

See [PULL_REQUEST_TEMPLATE.md](./.github/PULL_REQUEST_TEMPLATE.md) for the PR checklist.

---

## License

All components are licensed under the MIT License. See [LICENSE](./LICENSE) file for details.

---

## Status Reports

- [Phase 2 Complete](./PHASE2_COMPLETE.md)
- [Migration Complete](./MIGRATION_COMPLETE.md)
- [Integration Tests Summary](./INTEGRATION_TESTS_SUMMARY.md)
- [CI/CD Implementation Summary](./CI_CD_IMPLEMENTATION_SUMMARY.md)

---

## Links

- **Documentation:** https://tzervas.github.io/embeddenator/
- **Repository:** https://github.com/tzervas/embeddenator
- **Issues:** https://github.com/tzervas/embeddenator/issues
- **CI/CD:** https://github.com/tzervas/embeddenator/actions

---

**Last Updated:** January 16, 2026  
**Maintained by:** @tzervas
