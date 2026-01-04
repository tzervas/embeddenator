# ADR-017: Phase 2A Component Extraction Strategy

## Status

Accepted

## Date

2026-01-04

## Context

Phase 1 (monorepo split) is complete with 9 crates successfully extracted to individual repositories. All sister projects build and are stable. Phase 2A focuses on **decomposing the core embeddenator library** into focused component libraries to:

1. Improve modularity and maintainability
2. Enable independent versioning of subsystems
3. Reduce compilation times for dependent projects
4. Facilitate specialized optimizations per component
5. Support the vision outlined in [CRATE_STRUCTURE_AND_CONCURRENCY.md](../CRATE_STRUCTURE_AND_CONCURRENCY.md)

Current embeddenator (v0.19.4) is monolithic, combining VSA operations, retrieval, filesystem, I/O, and observability in a single crate. This creates tight coupling and makes it difficult to evolve components independently.

### Component Candidates

Based on code structure analysis and dependency graphs:

| Component | Source Modules | Status | Rationale |
|-----------|---------------|--------|-----------|
| embeddenator-vsa | `vsa/*`, `ternary_vec.rs`, `ternary.rs`, `codebook.rs`, `dimensional.rs` | ☐ Ready | Core VSA math, isolated from I/O |
| embeddenator-retrieval | `retrieval.rs`, `resonator.rs`, `signature.rs` | ☐ Ready | Search/query operations |
| embeddenator-fs | `embrfs.rs`, `fuse_shim.rs` | ☐ Ready | Filesystem logic |
| embeddenator-interop | `kernel_interop.rs` | ☐ Ready | Kernel integration |
| embeddenator-io | `envelope.rs` (from io/envelope.rs) | ☐ Ready | Serialization/compression |
| embeddenator-obs | `logging.rs`, `metrics.rs`, `hires_timing.rs` | ☐ Ready | Observability primitives |

### Extraction Challenges

1. **Circular Dependencies**: Some modules import from others that should become separate crates
2. **Unsafe Code**: 54 unsafe blocks across 6 files require security audit before extraction
3. **Test Coverage**: Must migrate tests to extracted crates while maintaining coverage
4. **API Surface**: Public API must remain stable or provide migration path
5. **Documentation**: Each extracted crate needs comprehensive docs

## Decision

We will extract components using a **staged, bottom-up approach**:

### Extraction Order (Dependency-Driven)

```
1. embeddenator-vsa (no component deps, foundational)
   └─> 2. embeddenator-retrieval (depends on vsa)
        └─> 3. embeddenator-fs (depends on vsa, retrieval)
             └─> 4. embeddenator-interop (depends on vsa, fs)
5. embeddenator-io (parallel, no component deps)
6. embeddenator-obs (parallel, no component deps)
```

### Extraction Workflow (Per Component)

**Branch Strategy:**
- Each extraction gets a feature branch: `feat/extract-<component>`
- Branch from current `feat/component-architecture-clean`
- Merge back to `feat/component-architecture-clean` after validation

**Steps:**

1. **Prepare Extraction** (in embeddenator monorepo)
   - [ ] Create ADR addendum documenting module boundaries
   - [ ] Audit unsafe code in modules to be extracted
   - [ ] Create tracking issue: `Extract embeddenator-<component>`
   - [ ] Branch: `feat/extract-<component>`

2. **Create Target Repo** (in ~/Documents/projects/embeddenator/)
   - [ ] Initialize new repo with proper structure
   - [ ] Copy source modules from embeddenator
   - [ ] Copy relevant tests, benches, examples
   - [ ] Create Cargo.toml with appropriate dependencies
   - [ ] Write README.md with component purpose and API

3. **Update Dependencies**
   - [ ] In extracted crate: use `path = "../embeddenator-vsa"` for sister crates
   - [ ] In embeddenator: add path dependency to extracted crate
   - [ ] Update re-exports in embeddenator/src/lib.rs

4. **Validate Extraction**
   - [ ] Build extracted crate: `cargo build`
   - [ ] Run tests: `cargo test`
   - [ ] Run benchmarks: `cargo bench` (if applicable)
   - [ ] Build embeddenator with new dependency
   - [ ] Run embeddenator test suite

5. **Document and Release**
   - [ ] Update embeddenator CHANGELOG.md
   - [ ] Update SPLIT_TRACKER.md Phase 2A status
   - [ ] Commit to feature branch
   - [ ] Create GitHub repo (if not exists)
   - [ ] Push to GitHub
   - [ ] Tag with v0.1.0 (initial component release)

6. **Cleanup** (in embeddenator)
   - [ ] Remove extracted modules from src/
   - [ ] Update lib.rs to re-export from component crate
   - [ ] Verify all tests still pass
   - [ ] Commit cleanup

### Security Requirements

For any module containing `unsafe` code:
1. **Document SAFETY invariants** before extraction
2. **Security audit handoff** to review unsafe blocks
3. **Add safety tests** verifying invariants
4. **Update unsafe inventory** in security audit docs

### Testing Strategy

**Preserve Coverage:**
- Move unit tests to component crates
- Keep integration tests in embeddenator
- Add cross-component integration tests where needed

**Regression Prevention:**
- Run full embeddenator test suite after each extraction
- Benchmark performance to detect regressions
- Use contract tests to verify API compatibility

### Documentation Requirements

Each extracted component must have:
- **README.md** - Purpose, features, quick start
- **CHANGELOG.md** - Version history
- **Cargo.toml** - Proper metadata, keywords, categories
- **src/lib.rs** - Comprehensive rustdoc with examples
- **examples/** - Working code samples

## Consequences

### Positive

1. **Modularity**: Clear boundaries between VSA math, retrieval, filesystem, I/O
2. **Independent Evolution**: Components can version and release independently
3. **Reduced Compilation**: Dependent projects only compile needed components
4. **Specialized Testing**: Component-specific test harnesses
5. **Better Documentation**: Focused docs per component
6. **Security Isolation**: Unsafe code confined to specific crates
7. **Reusability**: Components usable outside embeddenator ecosystem

### Negative

1. **Initial Overhead**: Extraction requires significant effort
2. **Dependency Management**: More complex with 6 additional crates
3. **Breaking Changes**: Potential API breaks during extraction
4. **Version Coordination**: Must coordinate releases across components
5. **Testing Complexity**: Integration testing across repos
6. **CI/CD Burden**: Each component needs CI pipeline (mitigated by Phase 2B)

### Neutral

1. **Path Dependencies**: Development uses local paths, production uses git tags
2. **Workspace Structure**: Developers need multi-repo setup
3. **Release Cadence**: May slow down initially as process stabilizes

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Circular dependencies | Bottom-up extraction order, refactor if needed |
| Test coverage loss | Migrate tests to components, track coverage metrics |
| Performance regression | Benchmark suite runs on each extraction |
| API breakage | Maintain re-exports in embeddenator for compatibility |
| Unsafe code issues | Security audit before extraction, document invariants |

## Success Criteria

Phase 2A is complete when:
- [x] Phase 1 sister projects are stable (verified)
- [ ] All 6 components extracted to separate repos
- [ ] Each component builds independently
- [ ] Embeddenator builds with component dependencies
- [ ] Full test suite passes (embeddenator + components)
- [ ] Benchmarks show no regression >5%
- [ ] All components have v0.1.0 tags
- [ ] SPLIT_TRACKER.md Phase 2A marked complete

## Related Documents

- [CRATE_STRUCTURE_AND_CONCURRENCY.md](../CRATE_STRUCTURE_AND_CONCURRENCY.md) - Original component vision
- [SPLIT_TRACKER.md](../handoff/SPLIT_TRACKER.md) - Phase tracking
- [LOCAL_DEVELOPMENT.md](../LOCAL_DEVELOPMENT.md) - Multi-repo development guide
- [VERSIONING.md](../VERSIONING.md) - Semantic versioning strategy
- ADR-001 through ADR-014 - Prior architectural decisions

## Timeline

- **Week 1**: Extract vsa, retrieval (foundational components)
- **Week 2**: Extract fs, interop (dependent components)
- **Week 3**: Extract io, obs (independent components)
- **Week 4**: Integration testing, documentation, release coordination

**Estimated Completion:** January 28, 2026

---

**Next Steps:**
1. Create GitHub issues for each component extraction
2. Begin with embeddenator-vsa extraction
3. Document unsafe code audit findings
4. Coordinate with Phase 2B (shared workflows) for CI setup
