# Migration Guide: Monolithic to Component Architecture

**Date:** January 16, 2026  
**Status:** All 10 component repos fully migrated and production-ready

## Overview

The Embeddenator project has migrated from a monolithic repository to a **modular component architecture** with 10 independent repositories. This guide helps users and developers transition to the new structure.

## Breaking Changes Summary

### Repository Structure

**OLD (Monolithic):**
```
embeddenator/
├── src/
│   ├── vsa/          # VSA operations
│   ├── fs/           # Filesystem operations
│   ├── retrieval/    # Search and retrieval
│   ├── io/           # I/O and serialization
│   ├── obs/          # Observability
│   └── interop/      # FFI bindings
└── Cargo.toml
```

**NEW (Component Architecture):**
```
embdntr/  (workspace root)
├── embeddenator/              # Legacy monolithic repo (deprecated)
├── embeddenator-vsa/          # VSA primitives
├── embeddenator-fs/           # Filesystem operations (EmbrFS)
├── embeddenator-retrieval/    # Search and retrieval
├── embeddenator-io/           # I/O and serialization
├── embeddenator-obs/          # Observability
├── embeddenator-interop/      # FFI and language bindings
├── embeddenator-cli/          # Command-line interface
├── embeddenator-testkit/      # Testing utilities
├── embeddenator-workspace/    # Workspace management
└── embeddenator-contract-bench/ # Contract testing
```

### Import Path Changes

#### Rust Imports

**OLD:**
```rust
use embeddenator::SparseVec;
use embeddenator::EmbrFS;
use embeddenator::TernaryInvertedIndex;
```

**NEW:**
```rust
use embeddenator_vsa::SparseVec;
use embeddenator_fs::EmbrFS;
use embeddenator_retrieval::TernaryInvertedIndex;
```

#### Cargo.toml Dependencies

**OLD:**
```toml
[dependencies]
embeddenator = { version = "0.20.0", features = ["simd", "fuse"] }
```

**NEW:**
```toml
[dependencies]
embeddenator-vsa = { version = "0.20.0", features = ["simd"] }
embeddenator-fs = { version = "0.20.0", features = ["fuse"] }
embeddenator-retrieval = "0.20.0"
embeddenator-io = "0.20.0"
embeddenator-obs = { version = "0.20.0", features = ["full"] }
```

### CLI Changes

**OLD:**
```bash
# Monolithic CLI
cargo run --release -- ingest -i ./data -e data.engram -m manifest.json
```

**NEW:**
```bash
# Component CLI (embeddenator-cli or embeddenator-fs)
cd embeddenator-fs
cargo run --release -- ingest -i ./data -e data.engram -m manifest.json
```

### GitHub Repository URLs

**OLD:**
- Main repo: `https://github.com/tzervas/embeddenator`
- All code in single repo

**NEW:**
- Workspace: `https://github.com/tzervas/embeddenator` (now workspace root)
- VSA: `https://github.com/tzervas/embeddenator-vsa`
- Filesystem: `https://github.com/tzervas/embeddenator-fs`
- Retrieval: `https://github.com/tzervas/embeddenator-retrieval`
- I/O: `https://github.com/tzervas/embeddenator-io`
- Observability: `https://github.com/tzervas/embeddenator-obs`
- Interop: `https://github.com/tzervas/embeddenator-interop`
- CLI: `https://github.com/tzervas/embeddenator-cli`
- Testkit: `https://github.com/tzervas/embeddenator-testkit`
- Workspace: `https://github.com/tzervas/embeddenator-workspace`
- Contract Bench: `https://github.com/tzervas/embeddenator-contract-bench`

## Component Mapping

### Core Components

| Old Path | New Component | Repository |
|----------|---------------|------------|
| `embeddenator/src/vsa/` | `embeddenator-vsa` | [embeddenator-vsa](https://github.com/tzervas/embeddenator-vsa) |
| `embeddenator/src/fs/` | `embeddenator-fs` | [embeddenator-fs](https://github.com/tzervas/embeddenator-fs) |
| `embeddenator/src/retrieval/` | `embeddenator-retrieval` | [embeddenator-retrieval](https://github.com/tzervas/embeddenator-retrieval) |
| `embeddenator/src/io/` | `embeddenator-io` | [embeddenator-io](https://github.com/tzervas/embeddenator-io) |
| `embeddenator/src/obs/` | `embeddenator-obs` | [embeddenator-obs](https://github.com/tzervas/embeddenator-obs) |
| `embeddenator/src/interop/` | `embeddenator-interop` | [embeddenator-interop](https://github.com/tzervas/embeddenator-interop) |
| `embeddenator/src/cli.rs` | `embeddenator-cli` | [embeddenator-cli](https://github.com/tzervas/embeddenator-cli) |
| `embeddenator/src/testing/` | `embeddenator-testkit` | [embeddenator-testkit](https://github.com/tzervas/embeddenator-testkit) |

### Supporting Components

| Component | Purpose | Repository |
|-----------|---------|------------|
| `embeddenator-workspace` | Workspace management and health checks | [embeddenator-workspace](https://github.com/tzervas/embeddenator-workspace) |
| `embeddenator-contract-bench` | Contract testing and benchmarks | [embeddenator-contract-bench](https://github.com/tzervas/embeddenator-contract-bench) |
| `embeddenator-integration-tests` | Cross-component integration tests | Local only |

## Migration Examples

### Example 1: Basic VSA Operations

**OLD CODE:**
```rust
use embeddenator::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let vec1 = SparseVec::encode_data(b"hello", &config, None);
let vec2 = SparseVec::encode_data(b"world", &config, None);
let bundled = vec1.bundle(&vec2);
```

**NEW CODE:**
```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let vec1 = SparseVec::encode_data(b"hello", &config, None);
let vec2 = SparseVec::encode_data(b"world", &config, None);
let bundled = vec1.bundle(&vec2);
```

**Changes:**
- Import path: `embeddenator::` → `embeddenator_vsa::`
- All other code remains identical

### Example 2: Filesystem Operations

**OLD CODE:**
```rust
use embeddenator::{EmbrFS, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let mut fs = EmbrFS::new(config);
fs.ingest_directory("./data", None)?;
fs.save("data.engram", "manifest.json")?;
```

**NEW CODE:**
```rust
use embeddenator_vsa::ReversibleVSAConfig;
use embeddenator_fs::EmbrFS;

let config = ReversibleVSAConfig::default();
let mut fs = EmbrFS::new(config);
fs.ingest_directory("./data", None)?;
fs.save("data.engram", "manifest.json")?;
```

**Changes:**
- Split imports: VSA config from `embeddenator_vsa`, EmbrFS from `embeddenator_fs`
- All other code remains identical

### Example 3: Search and Retrieval

**OLD CODE:**
```rust
use embeddenator::{
    TernaryInvertedIndex,
    SparseVec,
    search::two_stage_search,
    search::SearchConfig,
};

let mut index = TernaryInvertedIndex::new();
// ... build index
let results = two_stage_search(&query, &index, &vectors, &config, 5);
```

**NEW CODE:**
```rust
use embeddenator_vsa::SparseVec;
use embeddenator_retrieval::{
    TernaryInvertedIndex,
    search::two_stage_search,
    search::SearchConfig,
};

let mut index = TernaryInvertedIndex::new();
// ... build index
let results = two_stage_search(&query, &index, &vectors, &config, 5);
```

**Changes:**
- SparseVec from `embeddenator_vsa`
- Retrieval components from `embeddenator_retrieval`
- All other code remains identical

### Example 4: CLI Usage

**OLD:**
```bash
# From monolithic repo
cd embeddenator
cargo run --release -- ingest -i ./data -e out.engram -m manifest.json

cargo run --release -- extract -e out.engram -m manifest.json -o ./restored

cargo run --release -- query -e out.engram -q ./test.txt
```

**NEW:**
```bash
# From embeddenator-fs component
cd embeddenator-fs
cargo run --release -- ingest -i ./data -e out.engram -m manifest.json

cargo run --release -- extract -e out.engram -m manifest.json -o ./restored

cargo run --release -- query -e out.engram -q ./test.txt
```

**Changes:**
- Use component-specific CLIs (embeddenator-fs or embeddenator-cli)
- All command-line arguments remain identical

## Development Workflow Changes

### Building the Project

**OLD:**
```bash
cd embeddenator
cargo build --release
```

**NEW:**
```bash
# Build all components
cd embdntr
cargo build --release --workspace

# Or build specific component
cd embeddenator-vsa
cargo build --release
```

### Running Tests

**OLD:**
```bash
cd embeddenator
cargo test
```

**NEW:**
```bash
# Test all components
cd embdntr
cargo test --workspace

# Or test specific component
cd embeddenator-retrieval
cargo test
```

### Cross-Component Development

When working on multiple components locally, use Cargo patches:

```toml
# In embeddenator-fs/Cargo.toml
[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.20.0" }

# For local development, add patch:
[patch."https://github.com/tzervas/embeddenator-vsa"]
embeddenator-vsa = { path = "../embeddenator-vsa" }
```

See [embeddenator-workspace/PATCH_MANAGEMENT_GUIDE.md](embeddenator-workspace/PATCH_MANAGEMENT_GUIDE.md) for details.

## Documentation Changes

### README Updates

Each component now has its own README:

- [embeddenator-vsa/README.md](embeddenator-vsa/README.md) - VSA operations
- [embeddenator-fs/README.md](embeddenator-fs/README.md) - Filesystem operations
- [embeddenator-retrieval/README.md](embeddenator-retrieval/README.md) - Search and retrieval
- [embeddenator-io/README.md](embeddenator-io/README.md) - I/O operations
- [embeddenator-obs/README.md](embeddenator-obs/README.md) - Observability
- [embeddenator-interop/README.md](embeddenator-interop/README.md) - FFI and language bindings

### Architecture Documentation

- Workspace overview: [README.md](README.md)
- Component architecture: [embeddenator/docs/COMPONENT_ARCHITECTURE.md](embeddenator/docs/COMPONENT_ARCHITECTURE.md)
- ADR-016: [embeddenator/docs/adr/ADR-016-component-decomposition.md](embeddenator/docs/adr/ADR-016-component-decomposition.md)

## Benefits of Migration

### For Users

1. **Targeted Dependencies**: Only include components you need
2. **Smaller Build Times**: Compile only required components
3. **Independent Versioning**: Update components individually
4. **Clearer Documentation**: Component-specific guides

### For Developers

1. **Independent Development**: Work on components without affecting others
2. **Faster CI/CD**: Parallel testing and building
3. **Clearer Ownership**: Component-specific maintainers
4. **Better Testing**: Component-specific test suites

### For the Project

1. **Modular Architecture**: Clean separation of concerns
2. **Independent Release Cycles**: Version components separately
3. **Reduced Complexity**: Smaller, focused codebases
4. **Better Maintainability**: Easier to understand and modify

## Deprecation Timeline

- **v0.20.0-alpha (January 2026)**: Component architecture complete, monolithic repo deprecated
- **v0.21.0 (Q1 2026)**: Monolithic repo archived, component repos recommended
- **v1.0.0 (Q2-Q3 2026)**: Full transition to component architecture

## Support and Resources

### Getting Help

- **Issues**: Report in specific component repositories
- **Discussions**: Use workspace repo discussions
- **Documentation**: Check component-specific READMEs

### Workspace Management

The `embeddenator-workspace` component provides tools for managing the multi-repo workspace:

```bash
cd embeddenator-workspace
cargo build --release

# Check workspace health
./target/release/embeddenator-workspace health

# Update all repos
cd ..
./update_all.sh
```

## Common Migration Issues

### Issue 1: Import Not Found

**Error:**
```
error[E0432]: unresolved import `embeddenator::SparseVec`
```

**Solution:**
Update imports to use component-specific crates:
```rust
// Change this:
use embeddenator::SparseVec;

// To this:
use embeddenator_vsa::SparseVec;
```

### Issue 2: Missing Dependency

**Error:**
```
error: failed to load source for dependency `embeddenator-vsa`
```

**Solution:**
Add the component to your `Cargo.toml`:
```toml
[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa", tag = "v0.20.0" }
```

### Issue 3: Version Mismatch

**Error:**
```
error: multiple versions of embeddenator-vsa are in use
```

**Solution:**
Use the workspace-level version consistency tools:
```bash
cd embeddenator-workspace
cargo run -- check-versions
```

## FAQ

### Q: Why did the project split into components?

**A:** The monolithic architecture was limiting scalability, independent versioning, and parallel development. Component architecture provides cleaner boundaries, faster builds, and better maintainability.

### Q: Can I still use the monolithic embeddenator repo?

**A:** The monolithic repo is deprecated but still available at `embeddenator/`. It will be archived in v0.21.0. We strongly recommend migrating to the component architecture.

### Q: How do I migrate my existing code?

**A:** Follow the import path changes in this guide. Most code remains identical except for import statements and Cargo.toml dependencies.

### Q: Will there be breaking changes in the future?

**A:** We're working toward API stability for v1.0.0. Breaking changes will be clearly documented in changelogs and migration guides.

### Q: How do I contribute to a specific component?

**A:** Clone the component repository, make changes, and submit PRs to that component. See the component's CONTRIBUTING.md for guidelines.

### Q: What about cross-component changes?

**A:** Use Cargo patches for local development across components. See the [Patch Management Guide](embeddenator-workspace/PATCH_MANAGEMENT_GUIDE.md).

## Next Steps

1. **Review Component READMEs**: Read documentation for components you use
2. **Update Dependencies**: Modify Cargo.toml to use component crates
3. **Update Imports**: Change import paths in your code
4. **Test**: Run tests to ensure everything works
5. **Review Docs**: Check for updated examples and guides

## Additional Resources

- [Workspace README](README.md) - Workspace overview
- [Component Architecture](embeddenator/docs/COMPONENT_ARCHITECTURE.md) - Technical details
- [ADR-016](embeddenator/docs/adr/ADR-016-component-decomposition.md) - Design rationale
- [Patch Management Guide](embeddenator-workspace/PATCH_MANAGEMENT_GUIDE.md) - Cross-repo development

---

**Last Updated:** January 16, 2026  
**Version:** v0.20.0-alpha.1
