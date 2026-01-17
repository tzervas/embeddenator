# ⚠️ DEPRECATED: Monolithic Embeddenator Repository

**Status:** DEPRECATED as of v0.20.0-alpha.1 (January 16, 2026)  
**Replacement:** Component-based architecture with 10 independent repositories  
**Migration Guide:** [../MIGRATION_GUIDE.md](../MIGRATION_GUIDE.md)

---

## Deprecation Timeline

| Version | Date | Status | Action |
|---------|------|--------|--------|
| **v0.20.0-alpha.1** | Jan 16, 2026 | **DEPRECATED** | Official deprecation announced |
| **v0.21.0** | Q2 2026 | **ARCHIVED** | Repository marked read-only, no new features |
| **v1.0.0** | Q3 2026 | **REMOVED** | Repository removed from workspace |

---

## Why This Repository is Deprecated

The monolithic `embeddenator` repository has been **fully migrated** to a modern component-based architecture. All functionality has been extracted into 10 independent, production-ready repositories that offer:

✅ **Better Modularity** - Use only the components you need  
✅ **Independent Versioning** - Components evolve at their own pace  
✅ **Faster CI/CD** - Parallel testing and building  
✅ **Cleaner Dependencies** - No circular dependencies  
✅ **Easier Maintenance** - Focused, single-responsibility modules  

---

## Migration Path

### What Replaces This Repository?

Every feature in the monolithic repository has been migrated to a component repository:

| Old Monolithic Module | New Component Repository | Migration Guide |
|-----------------------|--------------------------|-----------------|
| `embeddenator::vsa::*` | [embeddenator-vsa](../embeddenator-vsa/) | [VSA Migration](#vsa-migration) |
| `embeddenator::embrfs::*` | [embeddenator-fs](../embeddenator-fs/) | [Filesystem Migration](#filesystem-migration) |
| `embeddenator::retrieval::*` | [embeddenator-retrieval](../embeddenator-retrieval/) | [Retrieval Migration](#retrieval-migration) |
| `embeddenator::io::*` | [embeddenator-io](../embeddenator-io/) | [I/O Migration](#io-migration) |
| `embeddenator::obs::*` | [embeddenator-obs](../embeddenator-obs/) | [Observability Migration](#observability-migration) |
| `embeddenator::interop::*` | [embeddenator-interop](../embeddenator-interop/) | [Interop Migration](#interop-migration) |
| `embeddenator` CLI | [embeddenator-cli](../embeddenator-cli/) | [CLI Migration](#cli-migration) |
| Test utilities | [embeddenator-testkit](../embeddenator-testkit/) | [Testing Migration](#testing-migration) |

---

## Component Migration Examples

### VSA Migration

**Before (Deprecated):**
```rust
use embeddenator::vsa::SparseVec;
use embeddenator::vsa::Codebook;

let vec = SparseVec::random(10000, 100);
let mut codebook = Codebook::new(10000);
```

**After (Current):**
```rust
use embeddenator_vsa::{SparseVec, Codebook};

let vec = SparseVec::random(10000, 100);
let mut codebook = Codebook::new(10000);
```

### Filesystem Migration

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

### Retrieval Migration

**Before (Deprecated):**
```rust
use embeddenator::retrieval::Resonator;

let resonator = Resonator::new(codebook);
let results = resonator.query(&query_vec, 10);
```

**After (Current):**
```rust
use embeddenator_retrieval::resonator::Resonator;

let resonator = Resonator::new(codebook);
let results = resonator.query(&query_vec, 10);
```

### I/O Migration

**Before (Deprecated):**
```rust
use embeddenator::io::{save_engram, load_engram};

save_engram("root.engram", &engram)?;
let engram = load_engram("root.engram")?;
```

**After (Current):**
```rust
use embeddenator_io::{save_engram, load_engram};

save_engram("root.engram", &engram)?;
let engram = load_engram("root.engram")?;
```

### Observability Migration

**Before (Deprecated):**
```rust
use embeddenator::obs::{Metrics, Timer};

let mut metrics = Metrics::new();
let timer = Timer::start();
```

**After (Current):**
```rust
use embeddenator_obs::{Metrics, Timer};

let mut metrics = Metrics::new();
let timer = Timer::start();
```

### Interop Migration

**Before (Deprecated):**
```rust
use embeddenator::interop::python::PyEmbrFS;

// Python interop
```

**After (Current):**
```rust
use embeddenator_interop::python::PyEmbrFS;

// Python interop
```

### CLI Migration

**Before (Deprecated):**
```bash
# Using monolithic binary
cargo run --release -- encode ./input -o root.engram
```

**After (Current):**
```bash
# Using component-based CLI
cd embeddenator-cli
cargo run --release -- encode ./input -o root.engram
```

### Testing Migration

**Before (Deprecated):**
```rust
// Tests using internal test utilities
use embeddenator::test_utils::*;
```

**After (Current):**
```rust
use embeddenator_testkit::{generators, fixtures, metrics};

// Dedicated testing utilities
let test_data = generators::random_bytes(1024);
```

---

## Cargo.toml Migration

### Before (Deprecated)

```toml
[dependencies]
embeddenator = "0.20.0-alpha.1"
```

### After (Current)

```toml
[dependencies]
# Use only the components you need
embeddenator-vsa = "0.20.0-alpha.1"
embeddenator-fs = "0.20.0-alpha.1"
embeddenator-retrieval = "0.20.0-alpha.1"
embeddenator-io = "0.20.0-alpha.1"
embeddenator-obs = "0.20.0-alpha.1"
```

Or use all components:

```toml
[dependencies]
embeddenator-vsa = "0.20.0-alpha.1"
embeddenator-fs = "0.20.0-alpha.1"
embeddenator-retrieval = "0.20.0-alpha.1"
embeddenator-io = "0.20.0-alpha.1"
embeddenator-obs = "0.20.0-alpha.1"
embeddenator-interop = "0.20.0-alpha.1"
embeddenator-cli = "0.20.0-alpha.1"
embeddenator-testkit = { version = "0.20.0-alpha.1", optional = true }
embeddenator-workspace = { version = "0.20.0-alpha.1", optional = true }
```

---

## What Happens Next?

### v0.20.0-alpha.1 (Current - DEPRECATED)
- ✅ All functionality migrated to component repositories
- ✅ Deprecation notices added to README and documentation
- ✅ Monolithic repository marked as deprecated
- ⚠️ **No new features will be added**
- ⚠️ **Security fixes only for critical vulnerabilities**
- ℹ️ Users should migrate to component repositories

### v0.21.0 (Q2 2026 - ARCHIVED)
- 🔒 Repository marked as **read-only**
- 📦 Final release with deprecation warnings
- 🚫 No pull requests accepted
- 🚫 No issues tracked
- ℹ️ Documentation redirects to component repositories

### v1.0.0 (Q3 2026 - REMOVED)
- 🗑️ Monolithic repository **removed from workspace**
- 📚 Historical archive maintained in Git history
- 🔗 All references updated to component repositories
- ✅ Component repositories become the canonical source

---

## Support and Help

### Getting Help with Migration

1. **Read the Migration Guide**: [../MIGRATION_GUIDE.md](../MIGRATION_GUIDE.md)
2. **Check Component Documentation**: Each component has comprehensive documentation
3. **Review Examples**: See [../examples/](../examples/) for component-based examples
4. **Ask Questions**: Open an issue in the relevant component repository

### Component Repository Links

- **VSA**: https://github.com/tzervas/embeddenator-vsa
- **Filesystem**: https://github.com/tzervas/embeddenator-fs
- **Retrieval**: https://github.com/tzervas/embeddenator-retrieval
- **I/O**: https://github.com/tzervas/embeddenator-io
- **Observability**: https://github.com/tzervas/embeddenator-obs
- **Interop**: https://github.com/tzervas/embeddenator-interop
- **CLI**: https://github.com/tzervas/embeddenator-cli
- **Testing**: https://github.com/tzervas/embeddenator-testkit
- **Workspace**: https://github.com/tzervas/embeddenator-workspace

---

## Frequently Asked Questions

### Q: Why deprecate the monolithic repository?

**A:** The component-based architecture offers better modularity, faster CI/CD, independent versioning, and cleaner dependencies. The monolithic repository became difficult to maintain as the project grew.

### Q: Will my existing code break?

**A:** Not immediately. The monolithic repository will continue to work for v0.20.0-alpha.1 and v0.21.0. However, you should migrate to component repositories before v1.0.0 (Q3 2026).

### Q: How long do I have to migrate?

**A:** You have approximately **6 months** (until Q3 2026) to migrate. We recommend migrating during Q2 2026 before the repository is archived.

### Q: Can I still use the monolithic repository?

**A:** Yes, but it's deprecated and will not receive new features. Security fixes will be provided only for critical vulnerabilities. We strongly recommend migrating to component repositories.

### Q: What if I only need one feature (e.g., VSA)?

**A:** Perfect! With the component architecture, you can depend on only `embeddenator-vsa` instead of the entire monolithic crate. This reduces compile times and binary size.

### Q: Is the migration difficult?

**A:** No! Most migrations require only updating `use` statements from `embeddenator::*` to `embeddenator_*::*`. The APIs remain identical.

### Q: Will there be breaking changes in the component repositories?

**A:** The component repositories follow semantic versioning. Breaking changes will be communicated clearly through version bumps (e.g., v0.20.x → v0.21.0 for minor breaks, v0.x.x → v1.0.0 for major breaks).

### Q: Where can I find migration examples?

**A:** Check [MIGRATION_GUIDE.md](../MIGRATION_GUIDE.md) for comprehensive examples, or look at the component repository examples directories.

---

## Acknowledgments

Thank you to all contributors who helped build the monolithic embeddenator repository. Your work lives on in the component repositories, which inherit all the hard work and lessons learned from this codebase.

---

## License

This repository remains under the MIT License. See [LICENSE](LICENSE) for details.

---

**Last Updated:** January 16, 2026  
**Deprecation Effective:** v0.20.0-alpha.1  
**Archival Date:** Q2 2026 (v0.21.0)  
**Removal Date:** Q3 2026 (v1.0.0)
