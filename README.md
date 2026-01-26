# Embeddenator Workspace

Multi-component Rust workspace for Vector Symbolic Architecture (VSA) based holographic data encoding.

Embeddenator is an encoding method and data model. It is not a security implementation.

[![CI](https://github.com/tzervas/embeddenator/workflows/Workspace%20CI/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/ci-workspace.yml)
[![Benchmarks](https://github.com/tzervas/embeddenator/workflows/Benchmark%20Regression%20Check/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/bench-workspace.yml)
[![Health](https://github.com/tzervas/embeddenator/workflows/Workspace%20Health/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/health-workspace.yml)
[![Docs](https://github.com/tzervas/embeddenator/workflows/Documentation/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/docs-workspace.yml)
[![Security](https://github.com/tzervas/embeddenator/workflows/Security%20Scan/badge.svg)](https://github.com/tzervas/embeddenator/actions/workflows/security-workspace.yml)
[![codecov](https://codecov.io/gh/tzervas/embeddenator/branch/main/graph/badge.svg)](https://codecov.io/gh/tzervas/embeddenator)

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.84+-orange.svg)](https://www.rust-lang.org/)
[![Dependencies](https://deps.rs/repo/github/tzervas/embeddenator/status.svg)](https://deps.rs/repo/github/tzervas/embeddenator)

---

## Overview

This workspace contains 8 published library crates implementing holographic data encoding using sparse ternary Vector Symbolic Architecture.

---

## Published Crates

### Core Libraries

#### embeddenator-vsa

[![Crate](https://img.shields.io/crates/v/embeddenator-vsa.svg)](https://crates.io/crates/embeddenator-vsa)
[![Downloads](https://img.shields.io/crates/d/embeddenator-vsa.svg)](https://crates.io/crates/embeddenator-vsa)
[![Docs](https://docs.rs/embeddenator-vsa/badge.svg)](https://docs.rs/embeddenator-vsa)

Sparse ternary Vector Symbolic Architecture primitives and operations.

**Repository:** https://github.com/tzervas/embeddenator-vsa

#### embeddenator-io

[![Crate](https://img.shields.io/crates/v/embeddenator-io.svg)](https://crates.io/crates/embeddenator-io)
[![Downloads](https://img.shields.io/crates/d/embeddenator-io.svg)](https://crates.io/crates/embeddenator-io)
[![Docs](https://docs.rs/embeddenator-io/badge.svg)](https://docs.rs/embeddenator-io)

I/O operations, serialization, and data persistence.

**Repository:** https://github.com/tzervas/embeddenator-io

#### embeddenator-obs

[![Crate](https://img.shields.io/crates/v/embeddenator-obs.svg)](https://crates.io/crates/embeddenator-obs)
[![Downloads](https://img.shields.io/crates/d/embeddenator-obs.svg)](https://crates.io/crates/embeddenator-obs)
[![Docs](https://docs.rs/embeddenator-obs/badge.svg)](https://docs.rs/embeddenator-obs)

Observability, metrics, and tracing infrastructure.

**Repository:** https://github.com/tzervas/embeddenator-obs

#### embeddenator-retrieval

[![Crate](https://img.shields.io/crates/v/embeddenator-retrieval.svg)](https://crates.io/crates/embeddenator-retrieval)
[![Downloads](https://img.shields.io/crates/d/embeddenator-retrieval.svg)](https://crates.io/crates/embeddenator-retrieval)
[![Docs](https://docs.rs/embeddenator-retrieval/badge.svg)](https://docs.rs/embeddenator-retrieval)

Information retrieval, search, and similarity matching.

**Repository:** https://github.com/tzervas/embeddenator-retrieval

#### embeddenator-fs

[![Crate](https://img.shields.io/crates/v/embeddenator-fs.svg)](https://crates.io/crates/embeddenator-fs)
[![Downloads](https://img.shields.io/crates/d/embeddenator-fs.svg)](https://crates.io/crates/embeddenator-fs)
[![Docs](https://docs.rs/embeddenator-fs/badge.svg)](https://docs.rs/embeddenator-fs)

EmbrFS: Holographic filesystem operations and versioning.

**Repository:** https://github.com/tzervas/embeddenator-fs

#### embeddenator-interop

[![Crate](https://img.shields.io/crates/v/embeddenator-interop.svg)](https://crates.io/crates/embeddenator-interop)
[![Downloads](https://img.shields.io/crates/d/embeddenator-interop.svg)](https://crates.io/crates/embeddenator-interop)
[![Docs](https://docs.rs/embeddenator-interop/badge.svg)](https://docs.rs/embeddenator-interop)

Foreign Function Interface (FFI) and language interoperability.

**Repository:** https://github.com/tzervas/embeddenator-interop

### Tools

#### embeddenator-cli

[![Crate](https://img.shields.io/crates/v/embeddenator-cli.svg)](https://crates.io/crates/embeddenator-cli)
[![Downloads](https://img.shields.io/crates/d/embeddenator-cli.svg)](https://crates.io/crates/embeddenator-cli)
[![Docs](https://docs.rs/embeddenator-cli/badge.svg)](https://docs.rs/embeddenator-cli)

Command-line interface for Embeddenator operations.

**Repository:** https://github.com/tzervas/embeddenator-cli

### Umbrella Crate

#### embeddenator-core

[![Crate](https://img.shields.io/crates/v/embeddenator-core.svg)](https://crates.io/crates/embeddenator-core)
[![Downloads](https://img.shields.io/crates/d/embeddenator-core.svg)](https://crates.io/crates/embeddenator-core)
[![Docs](https://docs.rs/embeddenator-core/badge.svg)](https://docs.rs/embeddenator-core)

Comprehensive re-export of all Embeddenator functionality.

**Repository:** https://github.com/tzervas/embeddenator-core

---

## Component Overview (Legacy Table)

| Component | Description | crates.io | Version |
|-----------|-------------|-----------|---------|
| [embeddenator-vsa](./embeddenator-vsa/) | Sparse ternary VSA primitives | [crates.io](https://crates.io/crates/embeddenator-vsa) | 0.21.0 |
| [embeddenator-io](./embeddenator-io/) | I/O operations and serialization | [crates.io](https://crates.io/crates/embeddenator-io) | 0.21.0 |
| [embeddenator-obs](./embeddenator-obs/) | Observability and metrics | [crates.io](https://crates.io/crates/embeddenator-obs) | 0.21.0 |
| [embeddenator-retrieval](./embeddenator-retrieval/) | Information retrieval and search | [crates.io](https://crates.io/crates/embeddenator-retrieval) | 0.21.0 |
| [embeddenator-fs](./embeddenator-fs/) | Filesystem operations (EmbrFS) | [crates.io](https://crates.io/crates/embeddenator-fs) | 0.23.0 |
| [embeddenator-interop](./embeddenator-interop/) | FFI and language interoperability | [crates.io](https://crates.io/crates/embeddenator-interop) | 0.22.0 |
| [embeddenator-cli](./embeddenator-cli/) | Command-line interface | [crates.io](https://crates.io/crates/embeddenator-cli) | 0.21.0 |
| [embeddenator-core](./embeddenator-core/) | Umbrella crate (re-exports all) | [crates.io](https://crates.io/crates/embeddenator-core) | 0.22.0 |

### Development Tools

| Component | Description |
|-----------|-------------|
| [embeddenator-workspace](./embeddenator-workspace/) | Workspace management tools |
| [embeddenator-testkit](./embeddenator-testkit/) | Testing utilities and fixtures |
| [embeddenator-contract-bench](./embeddenator-contract-bench/) | Contract testing and benchmarks |

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

### Project Documentation

| Document | Description |
|----------|-------------|
| **[docs/](./docs/)** | All project documentation |
| **[Project Charter](./docs/project-management/PROJECT_CHARTER.md)** | Vision, objectives, scope, success criteria |
| **[Requirements](./docs/requirements/REQUIREMENTS.md)** | User stories, functional requirements |
| **[Gap Analysis](./docs/benchmarks/GAP_ANALYSIS.md)** | Current performance vs targets |
| **[Refactor Plan](./docs/architecture/HOLOGRAPHIC_REFACTOR_PLAN.md)** | Holographic storage implementation roadmap |

### Architecture Decisions

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-001](./docs/adr/ADR-001-vsa-superposition-storage.md) | VSA Superposition Storage | Accepted |

### Guides

- **[CI/CD Guide](./docs/guides/CI_CD_GUIDE.md)** - Complete CI/CD documentation
- **[Maintained Dependencies](./MAINTAINED_DEPENDENCIES.md)** - Candle and related ML dependency ecosystem
- **[API Documentation](https://tzervas.github.io/embeddenator/)** - Generated rustdoc

### Archive

- **[Migration Guide](./archive/MIGRATION_GUIDE.md)** - Transition from monolithic to component architecture (historical)

---

## Maintained Dependencies Ecosystem

This project maintains forks of several unmaintained crates critical to the Rust ML ecosystem:

| Fork | Original | crates.io | Purpose |
|------|----------|-----------|---------|
| `qlora-paste` | `paste` | [qlora-paste](https://crates.io/crates/qlora-paste) | Token pasting macros |
| `qlora-gemm` | `gemm` | [qlora-gemm](https://crates.io/crates/qlora-gemm) | Matrix multiplication |
| `qlora-candle-*` | `candle-*` | [qlora-candle-core](https://crates.io/crates/qlora-candle-core) | ML framework |

**Upstream Contribution:** [PR #3335](https://github.com/huggingface/candle/pull/3335) submitted to merge maintained dependencies into huggingface/candle.

See [MAINTAINED_DEPENDENCIES.md](./MAINTAINED_DEPENDENCIES.md) for full integration guide.

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
│   └── ISSUE_TEMPLATE/     # Issue templates
├── docs/                   # Project documentation
│   ├── adr/               # Architecture Decision Records
│   ├── architecture/      # Design documents
│   ├── benchmarks/        # Performance analysis
│   ├── guides/            # How-to guides
│   ├── project-management/ # Project tracking
│   └── requirements/      # Specifications
├── embeddenator-vsa/       # VSA implementation (submodule)
├── embeddenator-fs/        # Filesystem operations (submodule)
├── embeddenator-io/        # I/O operations (submodule)
├── embeddenator-obs/       # Observable pattern (submodule)
├── embeddenator-retrieval/ # Information retrieval (submodule)
├── embeddenator-interop/   # FFI bindings (submodule)
├── embeddenator-cli/       # CLI interface (submodule)
├── embeddenator-core/      # Umbrella crate (submodule)
├── embeddenator-workspace/ # Workspace tools (submodule)
├── embeddenator-testkit/   # Testing utilities (submodule)
├── embeddenator-contract-bench/ # Contract tests (submodule)
├── archive/                # Historical documents
└── README.md               # This file
```

---

## Version Management

Current published versions vary by component (see table above).

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test --all-features --workspace`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

---

## License

All components are licensed under the MIT License. See [LICENSE](./LICENSE) file for details.

---

## Links

- **Documentation:** https://docs.rs/embeddenator
- **Repository:** https://github.com/tzervas/embeddenator
- **Issues:** https://github.com/tzervas/embeddenator/issues

---

**Last Updated:** January 26, 2026
**Maintained by:** @tzervas
