# Crates.io Publishing Status

> **Date**: January 9, 2026  
> **Version**: 0.20.0-alpha.1

## Published Crates ✅

All core crates have been published to crates.io as `0.20.0-alpha.1`:

| Crate | crates.io | Status |
|-------|-----------|--------|
| [embeddenator-vsa](https://crates.io/crates/embeddenator-vsa) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-obs](https://crates.io/crates/embeddenator-obs) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-io](https://crates.io/crates/embeddenator-io) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-workspace](https://crates.io/crates/embeddenator-workspace) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-retrieval](https://crates.io/crates/embeddenator-retrieval) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-fs](https://crates.io/crates/embeddenator-fs) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-interop](https://crates.io/crates/embeddenator-interop) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-cli](https://crates.io/crates/embeddenator-cli) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator](https://crates.io/crates/embeddenator) | v0.20.0-alpha.1 | ✅ Published |
| [embeddenator-testkit](https://crates.io/crates/embeddenator-testkit) | v0.20.0-alpha.1 | ✅ Published |

## Pending Crates ⚠️

| Crate | Status | Issue |
|-------|--------|-------|
| embeddenator-contract-bench | ⚠️ Not published | API mismatch - references types not exported by published crates |

### Contract-Bench Issues

The contract-bench crate references the following that don't exist in the published API:
- `embeddenator::BitslicedTritVec`
- `embeddenator::BlockSparseTritVec`
- `embeddenator::CarrySaveBundle`
- `embeddenator::envelope`
- `EmbrFS::save_engram_with_options()`

These need to be either:
1. Added to the public API of the main embeddenator crate
2. Removed/refactored from contract-bench

## Release Branches

All repos have `release/alpha-publish` branches pushed to origin:

```
embeddenator/release/alpha-publish
embeddenator-vsa/release/alpha-publish
embeddenator-retrieval/release/alpha-publish
embeddenator-fs/release/alpha-publish
embeddenator-obs/release/alpha-publish
embeddenator-io/release/alpha-publish
embeddenator-interop/release/alpha-publish
embeddenator-cli/release/alpha-publish
embeddenator-contract-bench/release/alpha-publish
embeddenator-testkit/release/alpha-publish
embeddenator-workspace/release/alpha-publish
```

## Dependencies

The crates were published in dependency order:

```
Tier 1 (no internal deps):
  embeddenator-vsa
  embeddenator-obs
  embeddenator-io
  embeddenator-workspace

Tier 2 (depends on vsa):
  embeddenator-retrieval

Tier 3 (depends on vsa, retrieval):
  embeddenator-fs

Tier 4 (depends on vsa, fs):
  embeddenator-interop

Tier 5 (depends on vsa, retrieval, fs, io):
  embeddenator-cli

Tier 6 (depends on all):
  embeddenator
  embeddenator-testkit
  embeddenator-contract-bench (PENDING)
```

## Usage

To use in your project:

```toml
[dependencies]
embeddenator = "0.20.0-alpha.1"

# Or individual components:
embeddenator-vsa = "0.20.0-alpha.1"
embeddenator-retrieval = "0.20.0-alpha.1"
embeddenator-fs = "0.20.0-alpha.1"
```

## Next Steps

1. **Create PRs** for each `release/alpha-publish` branch → `main`
2. **Fix contract-bench** API references
3. **Merge dev → main** in main embeddenator repo (71 commits behind)
4. **Tag releases** after PRs are merged
