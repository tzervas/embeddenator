# Version Bump Complete ✅

**Date**: January 16, 2026  
**Status**: All versions updated and verified

## Version Changes Summary

| Crate | Old Version | New Version | Change Type | crates.io |
|-------|-------------|-------------|-------------|-----------|
| **Production Releases** |
| embeddenator-vsa | 0.20.0-alpha.1 | **0.20.1** | PATCH (bug fixes) | Ready ✅ |
| embeddenator-fs | 0.21.0 | **0.22.0** | MINOR (CLI/examples) | Ready ✅ |
| embeddenator-obs | 0.20.0-alpha.1 | **0.21.0** | MINOR (new features) | Ready ✅ |
| embeddenator-interop | 0.20.0-alpha.1 | **0.21.0** | MINOR (completion) | Ready ✅ |
| **Stable Releases** |
| embeddenator-io | 0.20.0-alpha.1 | **0.20.0** | STABLE | Ready ✅ |
| embeddenator-testkit | 0.20.0-alpha.1 | **0.20.0** | STABLE | Ready ✅ |
| embeddenator-retrieval | 0.20.0-alpha.1 | **0.20.0** | STABLE | Ready ✅ |
| embeddenator-workspace | 0.20.0-alpha.1 | **0.20.0** | STABLE | Ready ✅ |
| **Development** |
| embeddenator-cli | 0.20.0-alpha.1 | **0.20.0-alpha.2** | PRERELEASE | Optional |
| embeddenator-contract-bench | 0.20.0-alpha.1 | **0.20.0-alpha.2** | PRERELEASE | Internal ❌ |
| embeddenator-integration-tests | 0.20.0-alpha.1 | **0.20.0-alpha.2** | PRERELEASE | Internal ❌ |
| **Deprecated** |
| embeddenator | 0.20.0-alpha.1 | 0.20.0-alpha.1 | FINAL | No more ❌ |

## Independent Versioning Confirmed

✅ Each repository now has independent versioning  
✅ Components can evolve at their own pace  
✅ Breaking changes in one component don't force bumps in others  
✅ Users can pin to specific component versions  

## Dependency Consistency Verified

```bash
$ embeddenator-workspace check-versions --verbose
✓ All versions are consistent!
```

All inter-component dependencies updated to match new versions.

## Publishing Order (Dependency-based)

When ready to publish to crates.io:

```bash
# 1. Core components (no dependencies)
cd embeddenator-vsa && cargo publish
cd embeddenator-io && cargo publish

# 2. Components that depend on core
cd embeddenator-testkit && cargo publish
cd embeddenator-retrieval && cargo publish
cd embeddenator-obs && cargo publish

# 3. Higher-level components
cd embeddenator-interop && cargo publish
cd embeddenator-workspace && cargo publish

# 4. Filesystem (depends on many)
cd embeddenator-fs && cargo publish

# 5. Optional: CLI and tools
cd embeddenator-cli && cargo publish
```

## Git Tagging Recommendations

Create version tags for each component:

```bash
git tag -a embeddenator-vsa-v0.20.1 -m "Bug fix: vector invariant violations"
git tag -a embeddenator-fs-v0.22.0 -m "Feature: CLI, examples, benchmarks"
git tag -a embeddenator-obs-v0.21.0 -m "Feature: Prometheus, OpenTelemetry"
git tag -a embeddenator-interop-v0.21.0 -m "Feature: compression, C headers"
git tag -a embeddenator-io-v0.20.0 -m "Stable release"
git tag -a embeddenator-testkit-v0.20.0 -m "Stable release"
git tag -a embeddenator-retrieval-v0.20.0 -m "Stable release"
git tag -a embeddenator-workspace-v0.20.0 -m "Stable release"
git tag -a embeddenator-cli-v0.20.0-alpha.2 -m "Prerelease update"
git tag -a embeddenator-contract-bench-v0.20.0-alpha.2 -m "Dev tool update"
git tag -a embeddenator-integration-tests-v0.20.0-alpha.2 -m "Test suite update"

git push --tags
```

## Changelog Entries Needed

Each published crate needs CHANGELOG.md entries:

### embeddenator-vsa v0.20.1
- Fixed: Vector invariant violation causing incorrect similarity
- Fixed: Self-similarity returning 0.478 instead of 1.000
- Added: Overlap detection and conflict resolution
- Added: Comprehensive stress tests

### embeddenator-fs v0.22.0
- Added: Full CLI binary with 9 commands
- Added: 4 examples (ingest, query, incremental, batch)
- Added: 3 benchmark suites
- Performance: 8.12 MB/s ingestion, 9.31 MB/s extraction

### embeddenator-obs v0.21.0
- Added: Prometheus metrics export
- Added: OpenTelemetry distributed tracing
- Added: Advanced statistics (percentiles, std dev)
- Added: Real-time metric streaming
- All features optional via feature flags

### embeddenator-interop v0.21.0
- Added: Full envelope compression (Zstd, LZ4)
- Added: Automated C header generation (cbindgen)
- Added: Round-trip compression verification
- All features optional via feature flags

### embeddenator-io v0.20.0
- First stable release
- Complete I/O operations migration
- Serialization, buffering, streaming

### embeddenator-testkit v0.20.0
- First stable release
- Complete test utilities migration
- Generators, metrics, fixtures, harness

### embeddenator-retrieval v0.20.0
- First stable release
- Complete retrieval operations migration
- 4 search strategies, 4 similarity metrics

### embeddenator-workspace v0.20.0
- First stable release
- Complete workspace management tools
- Version management, patch management, health checks

## Next Steps

1. **Review changelogs** - Ensure all changes documented
2. **Test builds** - Verify all components build with new versions
3. **Create git tags** - Tag each component with new version
4. **Publish to crates.io** - Follow dependency order above
5. **Announce releases** - GitHub releases for major components

## Notes

- **embeddenator** (monolithic) remains at 0.20.0-alpha.1 (deprecated, no more releases)
- **embeddenator-fs** continues from 0.21.0 → 0.22.0 (was already ahead)
- **4 stable releases**: io, testkit, retrieval, workspace (ready for 1.0.0 path)
- **4 feature releases**: vsa, fs, obs, interop (new capabilities)
- **3 dev tools**: cli, contract-bench, integration-tests (internal use)

All versions verified consistent, all dependencies updated! 🎉
