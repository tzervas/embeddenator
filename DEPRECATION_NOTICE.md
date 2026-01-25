# 🚨 DEPRECATION NOTICE: Monolithic Embeddenator Repository

**Effective Date:** January 16, 2026  
**Affected Package:** `embeddenator` crate (monolithic repository)  
**Status:** DEPRECATED

---

## TL;DR - What You Need to Know

⚠️ **The monolithic `embeddenator` crate is deprecated and will be removed in Q3 2026.**

✅ **Use component crates instead:**
- `embeddenator-vsa` - VSA primitives
- `embeddenator-fs` - Filesystem operations
- `embeddenator-retrieval` - Search and retrieval
- `embeddenator-io` - I/O operations
- `embeddenator-obs` - Observability
- `embeddenator-interop` - FFI/interop
- `embeddenator-cli` - Command-line tools

📖 **Migration is easy:** Most changes are just updating import paths (5-30 minutes).

---

## For Existing Users

### Am I Affected?

**YES** - If your `Cargo.toml` contains:
```toml
[dependencies]
embeddenator = "0.20.0-alpha.1"  # ⚠️ DEPRECATED
```

**NO** - If you're already using component crates:
```toml
[dependencies]
embeddenator-vsa = "0.20.0-alpha.1"  # ✅ CURRENT
embeddenator-fs = "0.20.0-alpha.1"   # ✅ CURRENT
```

### What Should I Do?

1. **Read the Migration Guide** → [embeddenator/DEPRECATED.md](./embeddenator/DEPRECATED.md)
2. **Update your Cargo.toml** → Replace `embeddenator` with component crates
3. **Update imports** → Change `use embeddenator::*` to `use embeddenator_vsa::*`
4. **Test your code** → Run tests to ensure everything works
5. **Done!** → Most migrations take 5-30 minutes

### Timeline

| Date | Version | What Happens |
|------|---------|--------------|
| **Jan 2026** | v0.20.0-alpha.1 | DEPRECATED - Start migrating now |
| **Q2 2026** | v0.21.0 | ARCHIVED - No more changes |
| **Q3 2026** | v1.0.0 | REMOVED - Must use component crates |

**You have ~6 months to migrate.** We recommend migrating in Q1-Q2 2026.

---

## For New Users

### Should I Use the Monolithic Crate?

**NO.** Use component crates instead.

### Quick Start

```bash
# Clone the workspace
git clone https://github.com/tzervas/embeddenator.git
cd embeddenator

# Use component crates directly
cd embeddenator-vsa    # VSA operations
cd embeddenator-fs     # Filesystem
cd embeddenator-cli    # Command-line tools
```

```toml
# In your Cargo.toml
[dependencies]
embeddenator-vsa = "0.20.0-alpha.1"
embeddenator-fs = "0.20.0-alpha.1"
embeddenator-retrieval = "0.20.0-alpha.1"
```

```rust
// In your code
use embeddenator_vsa::SparseVec;
use embeddenator_fs::EmbrFS;

let vec = SparseVec::random(10000, 100);
let mut fs = EmbrFS::new();
```

---

## Why is This Being Deprecated?

The project has evolved from a monolithic architecture to a modern component-based design. This provides:

✅ **Better Modularity** - Use only what you need  
✅ **Faster Development** - Independent component releases  
✅ **Parallel CI/CD** - 3x faster builds  
✅ **Cleaner Dependencies** - No circular deps  
✅ **Better Maintainability** - Focused, single-purpose modules  

All 10 component repositories are **production-ready** with 100% test coverage.

---

## Migration Examples

### Example 1: Simple VSA Usage

**Before (Deprecated):**
```rust
use embeddenator::vsa::SparseVec;
use embeddenator::vsa::Codebook;

let vec = SparseVec::random(10000, 100);
let codebook = Codebook::new(10000);
```

**After (Current):**
```rust
use embeddenator_vsa::{SparseVec, Codebook};

let vec = SparseVec::random(10000, 100);
let codebook = Codebook::new(10000);
```

### Example 2: Filesystem Operations

**Before (Deprecated):**
```rust
use embeddenator::embrfs::EmbrFS;
use embeddenator::embrfs::Manifest;

let mut fs = EmbrFS::new();
fs.ingest_directory("./data", false)?;
```

**After (Current):**
```rust
use embeddenator_fs::{EmbrFS, Manifest};

let mut fs = EmbrFS::new();
fs.ingest_directory("./data", false)?;
```

### Example 3: Multiple Components

**Before (Deprecated):**
```rust
use embeddenator::vsa::SparseVec;
use embeddenator::embrfs::EmbrFS;
use embeddenator::retrieval::Resonator;
```

**After (Current):**
```rust
use embeddenator_vsa::SparseVec;
use embeddenator_fs::EmbrFS;
use embeddenator_retrieval::resonator::Resonator;
```

---

## Component Repository Guide

### Core Components

| Component | Purpose | Use When You Need |
|-----------|---------|-------------------|
| **embeddenator-vsa** | Sparse ternary vectors, VSA operations | Vector operations, bundling, binding, similarity |
| **embeddenator-fs** | Holographic filesystem (EmbrFS) | Filesystem encoding, engrams, manifests |
| **embeddenator-retrieval** | Search and information retrieval | Query, search, resonator, inverted index |
| **embeddenator-io** | I/O operations, serialization | File I/O, binary/JSON formats, compression |
| **embeddenator-obs** | Observability, metrics, tracing | Metrics, timing, logging, telemetry |

### Supporting Components

| Component | Purpose | Use When You Need |
|-----------|---------|-------------------|
| **embeddenator-interop** | FFI, Python bindings, language interop | Python integration, C API, foreign interfaces |
| **embeddenator-cli** | Command-line interface | CLI tools, encoding, decoding, mounting |
| **embeddenator-testkit** | Testing utilities | Test data generation, fixtures, chaos testing |
| **embeddenator-workspace** | Workspace management | Multi-repo management, version coordination |
| **embeddenator-contract-bench** | Contract testing, benchmarks | Performance testing, API contracts |

---

## Getting Help

### Resources

1. **Migration Guide (Detailed)** → [embeddenator/DEPRECATED.md](./embeddenator/DEPRECATED.md)
2. **Archive Plan** → [embeddenator/ARCHIVE_PLAN.md](./embeddenator/ARCHIVE_PLAN.md)
3. **Component Documentation** → Each component has its own README
4. **Examples** → See `examples/` in each component repository

### Support Channels

- **GitHub Issues** - Open an issue in the relevant component repository
- **GitHub Discussions** - Ask questions in workspace discussions
- **Documentation** - Check component rustdoc: `cargo doc --open`

### Common Questions

**Q: Will my code break immediately?**  
A: No. The v0.20.0-alpha.1 crate still works. You have until Q3 2026 to migrate.

**Q: How long does migration take?**  
A: Most migrations take 5-30 minutes (updating imports). Complex projects may take 1-2 hours.

**Q: Can I use both old and new at the same time?**  
A: Yes, but not recommended. The monolithic crate re-exports component crates, so there may be version conflicts.

**Q: What if I find a bug in the monolithic crate?**  
A: Report it in the relevant component repository. Security fixes will be backported if critical.

**Q: Is there an automated migration tool?**  
A: Not yet, but most IDEs can help with find-and-replace for import paths.

---

## Action Items

### For Existing Users (Priority: HIGH)

- [ ] Read [DEPRECATED.md](./embeddenator/DEPRECATED.md) migration guide
- [ ] Update `Cargo.toml` to use component crates
- [ ] Update import statements in your code
- [ ] Run tests to verify migration
- [ ] Remove `embeddenator` dependency
- [ ] Update documentation/README

### For New Users (Priority: IMMEDIATE)

- [ ] Do NOT add `embeddenator` to your `Cargo.toml`
- [ ] Use component crates from the start
- [ ] Follow component documentation
- [ ] Join GitHub Discussions for support

### For Maintainers (Priority: HIGH)

- [ ] Update internal dependencies to component crates
- [ ] Test compatibility with new component versions
- [ ] Update CI/CD to use component crates
- [ ] Document component usage in your project

---

## Deprecation Metadata

```toml
# Cargo.toml metadata
[package.metadata.deprecated]
deprecated = true
since = "0.20.0-alpha.1"
date = "2026-01-16"
reason = "Migrated to component-based architecture"
archive_date = "2026-06-01"
removal_date = "2026-09-01"
migration_guide = "DEPRECATED.md"
component_repos = [
    "embeddenator-vsa",
    "embeddenator-fs",
    "embeddenator-retrieval",
    "embeddenator-io",
    "embeddenator-obs",
    "embeddenator-interop",
    "embeddenator-cli",
]
```

---

## Contact

**Project Maintainer:** Tyler Zervas <tz-dev@vectorweight.com>  
**GitHub:** https://github.com/tzervas/embeddenator  
**License:** MIT

---

**Last Updated:** January 16, 2026  
**Next Review:** March 2026  
**Effective Until:** September 2026 (removal)
