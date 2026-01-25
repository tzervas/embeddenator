# Versioning Strategy - Independent Component Repos

**Date**: January 16, 2026  
**Status**: Active

## Current State on crates.io

| Crate | crates.io Version | Local Version | Status |
|-------|------------------|---------------|--------|
| embeddenator | 0.20.0-alpha.1 | 0.20.0-alpha.1 | **DEPRECATED** |
| embeddenator-vsa | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Bug fixes ready |
| embeddenator-fs | **0.21.0** | **0.21.0** | CLI added, ready for 0.22.0 |
| embeddenator-io | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Migration complete |
| embeddenator-cli | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Depends on fs |
| embeddenator-obs | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Major features added |
| embeddenator-retrieval | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Migration complete |
| embeddenator-interop | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Full completion |
| embeddenator-testkit | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Migration complete |
| embeddenator-workspace | 0.20.0-alpha.1 | 0.20.0-alpha.1 | Tooling complete |
| embeddenator-contract-bench | Not published | 0.20.0-alpha.1 | Benchmarks added |

## Versioning Philosophy

### Independent Versioning
Each component repository now has **independent versioning**. This means:
- Components can evolve at their own pace
- Breaking changes in one component don't force version bumps in others
- Users can choose specific versions for specific components

### Semantic Versioning
Following [SemVer 2.0.0](https://semver.org/):
- **MAJOR** (x.0.0): Breaking API changes
- **MINOR** (0.x.0): New features, backwards compatible
- **PATCH** (0.0.x): Bug fixes, backwards compatible
- **PRERELEASE** (0.x.0-alpha.y): Development versions

## Recommended Version Bumps

### Immediate (Bug Fixes & Completions)

#### 1. embeddenator-vsa: 0.20.0-alpha.1 → **0.20.1**
**Reason**: Critical bug fixes (similarity calculation)
**Changes**:
- Fixed vector invariant violations
- Fixed self-similarity calculation (0.478 → 1.000)
- Added validation and stress tests
**Type**: PATCH (bug fixes, no API changes)

#### 2. embeddenator-fs: 0.21.0 → **0.22.0**
**Reason**: Major new features (CLI, examples, benchmarks)
**Changes**:
- Added full CLI binary (9 commands)
- Added 4 examples
- Added 3 benchmark suites
**Type**: MINOR (new features, backwards compatible)

#### 3. embeddenator-obs: 0.20.0-alpha.1 → **0.21.0**
**Reason**: Major new features (Prometheus, OpenTelemetry, streaming)
**Changes**:
- Added Prometheus metrics export
- Added OpenTelemetry tracing
- Added advanced statistics
- Added real-time streaming
**Type**: MINOR (new features, optional via feature flags)

#### 4. embeddenator-interop: 0.20.0-alpha.1 → **0.21.0**
**Reason**: Completion to 100% (compression, C headers)
**Changes**:
- Full envelope compression
- Automated C header generation
- Round-trip verification
**Type**: MINOR (new features, backwards compatible)

### Graduated to Stable

#### 5. embeddenator-io: 0.20.0-alpha.1 → **0.20.0**
**Reason**: Migration complete, production-ready, no more changes expected
**Changes**: Complete migration from monolithic repo
**Type**: Remove -alpha suffix (stable release)

#### 6. embeddenator-testkit: 0.20.0-alpha.1 → **0.20.0**
**Reason**: Migration complete, production-ready
**Changes**: Complete migration from monolithic repo
**Type**: Remove -alpha suffix (stable release)

#### 7. embeddenator-retrieval: 0.20.0-alpha.1 → **0.20.0**
**Reason**: Migration complete, production-ready
**Changes**: Complete migration from monolithic repo
**Type**: Remove -alpha suffix (stable release)

#### 8. embeddenator-workspace: 0.20.0-alpha.1 → **0.20.0**
**Reason**: Tooling complete, production-ready
**Changes**: All workspace management tools complete
**Type**: Remove -alpha suffix (stable release)

### Remain Alpha (Development Tools)

#### 9. embeddenator-cli: 0.20.0-alpha.1 → **0.20.0-alpha.2**
**Reason**: Depends on embeddenator-fs, minor updates
**Changes**: Updated dependencies
**Type**: PRERELEASE bump (still in development)

#### 10. embeddenator-contract-bench: 0.20.0-alpha.1 → **0.20.0-alpha.2**
**Reason**: Development/testing tool, not production
**Changes**: Benchmark suites added
**Type**: PRERELEASE bump (internal tool)

#### 11. embeddenator-integration-tests: 0.20.0-alpha.1 → **0.20.0-alpha.2**
**Reason**: Testing crate, not published
**Changes**: Integration tests added
**Type**: PRERELEASE bump (internal tool)

### Deprecated (No Version Bump)

#### 12. embeddenator: 0.20.0-alpha.1 → **0.20.0-alpha.1**
**Reason**: DEPRECATED - no further releases
**Changes**: Formal deprecation added
**Type**: FINAL VERSION (archive in v0.21.0, remove in v1.0.0)

## Version Bump Summary Table

| Crate | Current | New Version | Type | Publish? |
|-------|---------|-------------|------|----------|
| embeddenator-vsa | 0.20.0-alpha.1 | **0.20.1** | PATCH | ✅ Yes |
| embeddenator-fs | 0.21.0 | **0.22.0** | MINOR | ✅ Yes |
| embeddenator-obs | 0.20.0-alpha.1 | **0.21.0** | MINOR | ✅ Yes |
| embeddenator-interop | 0.20.0-alpha.1 | **0.21.0** | MINOR | ✅ Yes |
| embeddenator-io | 0.20.0-alpha.1 | **0.20.0** | STABLE | ✅ Yes |
| embeddenator-testkit | 0.20.0-alpha.1 | **0.20.0** | STABLE | ✅ Yes |
| embeddenator-retrieval | 0.20.0-alpha.1 | **0.20.0** | STABLE | ✅ Yes |
| embeddenator-workspace | 0.20.0-alpha.1 | **0.20.0** | STABLE | ✅ Yes |
| embeddenator-cli | 0.20.0-alpha.1 | 0.20.0-alpha.2 | PRERELEASE | ⚠️ Optional |
| embeddenator-contract-bench | 0.20.0-alpha.1 | 0.20.0-alpha.2 | PRERELEASE | ❌ No (dev tool) |
| embeddenator-integration-tests | 0.20.0-alpha.1 | 0.20.0-alpha.2 | PRERELEASE | ❌ No (dev tool) |
| embeddenator | 0.20.0-alpha.1 | 0.20.0-alpha.1 | DEPRECATED | ❌ No (final) |

## Dependency Management

### Critical: Update Dependency Versions

After bumping versions, update all inter-component dependencies:

```toml
# Example: embeddenator-fs/Cargo.toml
[dependencies]
embeddenator-vsa = "0.20.1"      # Updated from 0.20.0-alpha.1
embeddenator-io = "0.20.0"        # Updated from 0.20.0-alpha.1
embeddenator-obs = "0.21.0"       # Updated from 0.20.0-alpha.1
```

### Compatibility Matrix

Components with stable APIs (0.x.0 versions):
- embeddenator-io 0.20.0
- embeddenator-testkit 0.20.0
- embeddenator-retrieval 0.20.0
- embeddenator-workspace 0.20.0

These should use **exact** versions in Cargo.toml for stability:
```toml
embeddenator-io = { version = "0.20.0", path = "../embeddenator-io" }
```

Components still evolving:
- embeddenator-vsa, embeddenator-fs, embeddenator-obs, embeddenator-interop

These can use **compatible** version requirements:
```toml
embeddenator-vsa = { version = "0.20", path = "../embeddenator-vsa" }
```

## Implementation Steps

### Phase 1: Version Bump (Manual)
```bash
# For each component, update Cargo.toml version field
vim embeddenator-vsa/Cargo.toml       # 0.20.0-alpha.1 → 0.20.1
vim embeddenator-fs/Cargo.toml        # 0.21.0 → 0.22.0
vim embeddenator-obs/Cargo.toml       # 0.20.0-alpha.1 → 0.21.0
vim embeddenator-interop/Cargo.toml   # 0.20.0-alpha.1 → 0.21.0
vim embeddenator-io/Cargo.toml        # 0.20.0-alpha.1 → 0.20.0
vim embeddenator-testkit/Cargo.toml   # 0.20.0-alpha.1 → 0.20.0
vim embeddenator-retrieval/Cargo.toml # 0.20.0-alpha.1 → 0.20.0
vim embeddenator-workspace/Cargo.toml # 0.20.0-alpha.1 → 0.20.0
```

### Phase 2: Update Dependencies (Automated)
```bash
# Use workspace tool to update all inter-component dependencies
embeddenator-workspace update-deps --sync
```

### Phase 3: Verify Consistency
```bash
embeddenator-workspace check-versions --verbose
```

### Phase 4: Commit and Tag
```bash
# Create version tags for each component
git tag -a embeddenator-vsa-v0.20.1 -m "Bug fix release: vector invariant violations"
git tag -a embeddenator-fs-v0.22.0 -m "Feature release: CLI, examples, benchmarks"
git tag -a embeddenator-obs-v0.21.0 -m "Feature release: Prometheus, OpenTelemetry"
# ... etc for each component
```

### Phase 5: Publish to crates.io
```bash
# Publish in dependency order
cd embeddenator-vsa && cargo publish
cd embeddenator-io && cargo publish
cd embeddenator-testkit && cargo publish
cd embeddenator-obs && cargo publish
cd embeddenator-retrieval && cargo publish
cd embeddenator-interop && cargo publish
cd embeddenator-workspace && cargo publish
cd embeddenator-fs && cargo publish
```

## Changelog Requirements

Each component with a version bump needs a CHANGELOG entry:

### embeddenator-vsa v0.20.1
```markdown
## [0.20.1] - 2026-01-16
### Fixed
- Fixed vector invariant violation causing incorrect similarity calculations
- Fixed self-similarity returning 0.478 instead of 1.000
- Added overlap detection in encode() function
- Added comprehensive stress tests
```

### embeddenator-fs v0.22.0
```markdown
## [0.22.0] - 2026-01-16
### Added
- Full CLI binary with 9 commands (ingest, extract, query, list, info, verify, update)
- 4 examples: basic_ingest, query_files, incremental_update, batch_processing
- 3 benchmark suites: ingest, query, incremental operations
### Performance
- Ingestion: 8.12 MB/s
- Extraction: 9.31 MB/s
```

### embeddenator-obs v0.21.0
```markdown
## [0.21.0] - 2026-01-16
### Added
- Prometheus metrics export (text format)
- OpenTelemetry distributed tracing (W3C Trace Context)
- Advanced statistics (percentiles, std dev, histograms)
- Real-time metric streaming with callbacks
- All features optional via feature flags
```

## Future Versioning

### Path to 1.0.0

Components ready for 1.0.0 consideration (stable API):
- embeddenator-io
- embeddenator-testkit
- embeddenator-workspace

Requirements for 1.0.0:
- ✅ API stability (no breaking changes expected)
- ✅ Comprehensive documentation
- ✅ Full test coverage
- ✅ Production usage validation
- ⏳ 6+ months in production (0.x.0 stable)

### Long-term Timeline

- **Q1 2026** (Now): Bump to appropriate versions per table above
- **Q2 2026**: Monitor stability, gather user feedback
- **Q3 2026**: Promote stable components to 1.0.0
- **Q4 2026**: Deprecation of monolithic embeddenator complete

## Notes

- **embeddenator-fs** is ahead at 0.21.0 - we continue from there
- **embeddenator** (monolithic) remains at 0.20.0-alpha.1 as FINAL VERSION
- Development tools (contract-bench, integration-tests) stay in alpha
- All production components get clean versions (no alpha suffix where appropriate)

## Version Consistency Checks

After bumping, verify no conflicts:
```bash
embeddenator-workspace check-versions
```

Expected result: All dependencies consistent, no drift detected.
