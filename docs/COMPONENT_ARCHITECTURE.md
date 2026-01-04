# Component Architecture

**Version:** 0.20.0  
**Status:** Active  
**Last Updated:** January 4, 2026

## Overview

Embeddenator has been refactored into a **modular component architecture** with 6 independent library crates and 3 binary/tool crates. This architecture enables:

- **Granular dependency management** - Use only what you need
- **Independent versioning** - Components evolve at their own pace
- **Parallel development** - Teams work on isolated components
- **Reduced build times** - Incremental compilation per component
- **Clear API boundaries** - Explicit contracts between layers

## Architecture Diagram

```
embeddenator (core orchestrator v0.20.0)
├── embeddenator-vsa         → Sparse ternary VSA primitives
├── embeddenator-io           → Codebook, manifest, engram I/O
├── embeddenator-retrieval    → Query, shift-sweep, cosine similarity
├── embeddenator-fs           → FUSE filesystem integration
├── embeddenator-interop      → Python/FFI bindings
└── embeddenator-obs          → Observability, metrics, logging

embeddenator-testkit          → Shared testing utilities
embeddenator-contract-bench   → Performance benchmarking
embeddenator-workspace        → CLI wrapper for workspace operations
```

## Component Repositories

All components are hosted under the `tzervas` GitHub organization:

| Component | Repository | Version | Description |
|-----------|------------|---------|-------------|
| **embeddenator-vsa** | [tzervas/embeddenator-vsa](https://github.com/tzervas/embeddenator-vsa) | v0.1.0 | Sparse ternary vector operations (bundle, bind, permute, cosine similarity) |
| **embeddenator-io** | [tzervas/embeddenator-io](https://github.com/tzervas/embeddenator-io) | v0.1.0 | Serialization/deserialization for codebooks, manifests, engrams |
| **embeddenator-retrieval** | [tzervas/embeddenator-retrieval](https://github.com/tzervas/embeddenator-retrieval) | v0.1.0 | Query engine with shift-sweep search and similarity metrics |
| **embeddenator-fs** | [tzervas/embeddenator-fs](https://github.com/tzervas/embeddenator-fs) | v0.1.0 | FUSE filesystem for mounting engrams as virtual filesystems |
| **embeddenator-interop** | [tzervas/embeddenator-interop](https://github.com/tzervas/embeddenator-interop) | v0.1.0 | Foreign Function Interface (FFI) for Python/C integration |
| **embeddenator-obs** | [tzervas/embeddenator-obs](https://github.com/tzervas/embeddenator-obs) | v0.1.0 | Observability, metrics collection, structured logging |
| **embeddenator-testkit** | [tzervas/embeddenator-testkit](https://github.com/tzervas/embeddenator-testkit) | v0.1.1 | Shared test utilities, fixtures, property-based testing harnesses |
| **embeddenator-contract-bench** | [tzervas/embeddenator-contract-bench](https://github.com/tzervas/embeddenator-contract-bench) | v0.2.1 | Performance benchmarking suite with contract validation |

## Dependency Graph

```
embeddenator (core)
├── embeddenator-vsa (no deps)
├── embeddenator-io
│   └── embeddenator-vsa
├── embeddenator-retrieval
│   ├── embeddenator-vsa
│   └── embeddenator-io
├── embeddenator-fs
│   ├── embeddenator-vsa
│   ├── embeddenator-io
│   └── embeddenator-retrieval
├── embeddenator-interop
│   ├── embeddenator-vsa
│   └── embeddenator-io
└── embeddenator-obs (no deps)

embeddenator-testkit
├── embeddenator-vsa
└── embeddenator-io

embeddenator-contract-bench
├── embeddenator-vsa
├── embeddenator-io
└── embeddenator-retrieval
```

## Using Components

### As Git Dependencies (Production)

Components are consumed via **git tags** for stable releases:

```toml
[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.1.0" }
embeddenator-io = { git = "https://github.com/tzervas/embeddenator-io", tag = "v0.1.0" }
```

### As Path Dependencies (Local Development)

For **local development** with multiple components, use `[patch.crates-io]`:

```toml
# In embeddenator/Cargo.toml (or workspace root)
[patch.crates-io]
embeddenator-vsa = { path = "../embeddenator-vsa" }
embeddenator-io = { path = "../embeddenator-io" }
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
embeddenator-fs = { path = "../embeddenator-fs" }
embeddenator-interop = { path = "../embeddenator-interop" }
embeddenator-obs = { path = "../embeddenator-obs" }
```

**Workflow:**
1. Clone all component repos into a common parent directory:
   ```bash
   mkdir ~/embeddenator-workspace
   cd ~/embeddenator-workspace
   git clone https://github.com/tzervas/embeddenator
   git clone https://github.com/tzervas/embeddenator-vsa
   git clone https://github.com/tzervas/embeddenator-io
   # ... etc for all components
   ```

2. Add `[patch.crates-io]` to your `Cargo.toml` (see above)

3. Develop across components with instant feedback:
   ```bash
   cd embeddenator-vsa
   # Make changes...
   cd ../embeddenator
   cargo build  # Uses local path, not git tag
   ```

4. Before committing:
   - Remove or comment out `[patch.crates-io]`
   - Verify builds work with git tag dependencies
   - Commit and push component changes first, then core

See [LOCAL_DEVELOPMENT.md](LOCAL_DEVELOPMENT.md) for detailed workflows.

## Versioning Strategy

### Core Orchestrator (embeddenator)
- **Semver:** Major.Minor.Patch (e.g., v0.20.0)
- **Release Cadence:** Feature-driven, ~monthly
- **Git Tags:** `v0.20.0`, `v0.21.0`, etc.
- **Breaking Changes:** Increments major version (0.x → 1.0 for stable)

### Components (embeddenator-*)
- **Semver:** Major.Minor.Patch (e.g., v0.1.0, v0.1.1)
- **Release Cadence:** Independent, as needed
- **Git Tags:** Per-repo tags (e.g., `v0.1.0` in embeddenator-vsa)
- **Compatibility:** Components maintain API stability within minor versions

### Tagging Workflow
1. **Update version in Cargo.toml:**
   ```bash
   cd embeddenator-vsa
   # Edit Cargo.toml: version = "0.1.1"
   cargo build --release  # Verify
   ```

2. **Commit and tag:**
   ```bash
   git add Cargo.toml
   git commit -m "Release v0.1.1: Fix cosine similarity precision"
   git tag -a v0.1.1 -m "v0.1.1: Precision improvements"
   git push origin main --tags
   ```

3. **Update consumers:**
   ```bash
   cd ../embeddenator
   # Edit Cargo.toml: tag = "v0.1.1"
   cargo update -p embeddenator-vsa
   cargo test --all
   git commit -am "Update embeddenator-vsa to v0.1.1"
   ```

See [VERSIONING.md](VERSIONING.md) for compatibility guarantees.

## CI/CD Integration

### Shared Workflows

All component repos use **centralized CI workflows** from [tzervas/.github-workflows](https://github.com/tzervas/.github-workflows):

```yaml
# .github/workflows/ci.yml (in each component repo)
name: CI
on: [push, pull_request]

jobs:
  ci:
    uses: tzervas/.github-workflows/.github/workflows/reusable-ci.yml@v1
    with:
      rust-version: '1.84'
      run-tests: true
      cache-key-prefix: 'embeddenator-vsa'
```

**Benefits:**
- **DRY principle** - Update CI logic once, propagate to all repos
- **Versioning** - Use `@v1` floating tag for automatic updates
- **Specialized actions** - FUSE setup, benchmark isolation, cargo caching
- **Consistency** - All repos build/test identically

### Docker Images (Core Only)

The **embeddenator core** provides multi-arch Docker images:

```bash
# Pull latest stable
docker pull ghcr.io/tzervas/embeddenator:latest

# Pull specific version
docker pull ghcr.io/tzervas/embeddenator:v0.20.0

# Architecture-specific
docker pull ghcr.io/tzervas/embeddenator:v0.20.0-amd64
docker pull ghcr.io/tzervas/embeddenator:v0.20.0-arm64
```

**Features:**
- Multi-stage Alpine builds (~15MB final image)
- Multi-arch manifests (amd64 + arm64)
- Automated Trivy security scanning
- Non-root user (embr:1000)
- GHCR integration with GitHub Packages

See [.docker/README.md](.docker/README.md) for building and deploying.

## Migration from Monolith

### Pre-v0.20.0 (Monolith)
```rust
// Single crate with all functionality
use embeddenator::vsa::SparseVec;
use embeddenator::io::codebook::Codebook;
use embeddenator::retrieval::query::QueryEngine;
```

### Post-v0.20.0 (Components)
```rust
// Import from dedicated crates
use embeddenator_vsa::SparseVec;
use embeddenator_io::codebook::Codebook;
use embeddenator_retrieval::query::QueryEngine;
```

### Breaking Changes
- **Module paths:** `embeddenator::module::*` → `embeddenator_module::*`
- **Cargo.toml:** Single dependency → Multiple component dependencies
- **Feature flags:** Top-level features removed (components have own features)

### Compatibility Shim
For gradual migration, use re-exports in `embeddenator` core:

```rust
// embeddenator/src/lib.rs
pub use embeddenator_vsa as vsa;
pub use embeddenator_io as io;
pub use embeddenator_retrieval as retrieval;

// Allows legacy code to work:
// use embeddenator::vsa::SparseVec; ✅
```

See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) for step-by-step instructions.

## Development Workflow

### Adding a New Component

1. **Create repository:**
   ```bash
   cd ~/Documents/projects
   cargo new --lib embeddenator-new-component
   cd embeddenator-new-component
   git init
   git remote add origin https://github.com/tzervas/embeddenator-new-component
   ```

2. **Add shared CI:**
   ```bash
   mkdir -p .github/workflows
   # Create ci.yml using shared workflow (see examples)
   git add .github/
   git commit -m "Add CI workflow"
   ```

3. **Implement and test:**
   ```bash
   cargo build
   cargo test
   cargo doc --no-deps
   ```

4. **Release:**
   ```bash
   git tag -a v0.1.0 -m "Initial release"
   git push origin main --tags
   ```

5. **Integrate into core:**
   ```bash
   cd ../embeddenator
   # Add to Cargo.toml with git tag
   cargo update -p embeddenator-new-component
   ```

### Cross-Component Changes

For changes spanning multiple components:

1. **Branch all affected repos:**
   ```bash
   cd embeddenator-vsa && git checkout -b feat/new-feature
   cd ../embeddenator-io && git checkout -b feat/new-feature
   cd ../embeddenator && git checkout -b feat/new-feature
   ```

2. **Use local path dependencies:**
   ```bash
   cd embeddenator
   # Add [patch.crates-io] for affected components
   ```

3. **Develop and test iteratively:**
   ```bash
   cargo test --all
   ```

4. **Release in dependency order:**
   ```bash
   # 1. Release vsa (no deps)
   cd embeddenator-vsa && git tag v0.1.1 && git push origin main --tags
   
   # 2. Update io to use vsa v0.1.1, release
   cd ../embeddenator-io
   # Edit Cargo.toml: embeddenator-vsa tag = "v0.1.1"
   git tag v0.1.1 && git push origin main --tags
   
   # 3. Update core to use new component versions
   cd ../embeddenator
   # Edit Cargo.toml: update all component tags
   cargo update
   git commit -am "Update components: vsa v0.1.1, io v0.1.1"
   ```

## Testing Strategy

### Unit Tests (Per Component)
Each component has its own test suite:
```bash
cd embeddenator-vsa
cargo test
```

### Integration Tests (Core)
Core repo tests component interactions:
```bash
cd embeddenator
cargo test --test integration
```

### Contract Tests (embeddenator-contract-bench)
Validate API contracts between components:
```bash
cd embeddenator-contract-bench
cargo bench
```

### E2E Tests (Core)
Full pipeline tests with real engrams:
```bash
cd embeddenator
cargo test --test e2e
```

## Performance Considerations

### Build Times
- **Monolith:** ~3-5 minutes full rebuild
- **Components:** ~30s per component, ~2 minutes core (with caching)
- **Incremental:** 5-10s for single-component changes

### Binary Size
- **Core:** ~8MB (release, stripped)
- **Components (static link):** No overhead (no dynamic linking)
- **Docker image:** ~15MB (Alpine + musl binary)

### Runtime Performance
- **Zero overhead:** Component boundaries compile to direct function calls
- **LTO:** Link-time optimization across component boundaries
- **Inlining:** Compiler inlines across crate boundaries with LTO

## Future Plans

### Roadmap
- [ ] **v0.21.0:** Incremental engram updates (ADR-014)
- [ ] **v0.22.0:** Advanced retrieval algorithms (semantic search, clustering)
- [ ] **v1.0.0:** API stability guarantee, production-ready
- [ ] **Post-1.0:** Plugin system for user-defined components

### Component Additions (Planned)
- **embeddenator-compression:** Adaptive sparsity and compression
- **embeddenator-distributed:** Multi-node query distribution
- **embeddenator-ml:** Machine learning integration (embeddings, classifiers)
- **embeddenator-gpu:** CUDA/OpenCL acceleration for VSA operations

## References

- [ADR-002: Multi-Agent Workflow System](adr/ADR-002-multi-agent-workflow-system.md) - Original component extraction rationale
- [ADR-005: Hologram Package Isolation](adr/ADR-005-hologram-package-isolation.md) - Component boundary design
- [Shared Workflows Repository](https://github.com/tzervas/.github-workflows) - Centralized CI/CD
- [Docker Registry](https://github.com/tzervas/embeddenator/pkgs/container/embeddenator) - Container images

## Support

- **Issues:** Report component-specific issues in their respective repos
- **Discussions:** Use [embeddenator/discussions](https://github.com/tzervas/embeddenator/discussions) for cross-component questions
- **Security:** See [SECURITY.md](../SECURITY.md) for vulnerability reporting
