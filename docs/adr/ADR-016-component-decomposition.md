# ADR-016: Component Decomposition Strategy

## Status

Accepted

## Date

2025-12-31

## Context

The Embeddenator project began as a monolithic repository containing all components: core VSA operations, retrieval systems, filesystem integration, I/O handling, observability, CLI tools, and MCP servers. While this approach facilitated rapid initial development, it has created several challenges as the project scales:

### Problems with Monolithic Architecture

1. **Tight Coupling**: All subsystems are interdependent, making it difficult to modify one component without affecting others
2. **Slow Build Times**: Changes to any component require rebuilding the entire codebase (~15,000 LOC)
3. **Versioning Challenges**: Cannot version VSA core separately from filesystem or CLI tools
4. **Dependency Bloat**: Users who only need VSA operations must pull in filesystem and FUSE dependencies
5. **Testing Complexity**: Integration tests span multiple concerns, making failures hard to isolate
6. **Parallel Development**: Multiple developers cannot work independently on different subsystems
7. **Deployment Flexibility**: Cannot deploy VSA library separately from filesystem components

### Current Structure Issues

```
embeddenator/
├── src/
│   ├── vsa/          # Core VSA (should be independent)
│   ├── retrieval.rs  # Depends on VSA
│   ├── embrfs.rs     # Depends on VSA + retrieval
│   ├── kernel_interop.rs  # Depends on VSA + fs
│   ├── io/           # Could be independent
│   ├── cli.rs        # Depends on everything
│   └── main.rs       # CLI entrypoint
├── crates/           # Sister projects
│   ├── embeddenator-screen-mcp/
│   └── ...
```

**Problem:** Everything is in `src/`, creating a single compilation unit with complex internal dependencies.

### Industry Best Practices

Projects like Rust's `tokio`, `serde`, and `async-std` demonstrate successful component decomposition:

- **tokio**: Split into `tokio-core`, `tokio-io`, `tokio-timer`, `tokio-executor`
- **serde**: Separated into `serde` (core), `serde_json`, `serde_derive`
- **async-std**: Modular with optional features for filesystem, networking, etc.

**Pattern:** Core primitives in one crate, domain-specific functionality in separate crates with optional features.

## Decision

We will decompose the Embeddenator monolith into **focused, independently versioned component libraries** following a three-phase approach:

### Phase 1: Repository Setup and Sister Project Stabilization ✅

**Goal:** Establish infrastructure for multi-crate workspace

**Timeline:** December 2025

**Actions:**
1. Create separate repositories for each component (14 repos total)
2. Set up CI/CD for individual components
3. Document architectural decisions (ADRs)
4. Stabilize existing sister projects (embeddenator-bench, embeddenator-contract-bench, etc.)

**Status:** Complete

### Phase 2: Core Component Extraction

**Goal:** Extract core embeddenator functionality into modular libraries

**Timeline:** January-February 2026

#### Phase 2A: Foundation Components (4 weeks)

Extract components with clear boundaries and minimal dependencies:

1. **embeddenator-vsa** (Week 1)
   - Core VSA operations: `SparseVec`, binding, bundling
   - Ternary primitives: `Trit`, `PackedTritVec`
   - Codebook management
   - Dimensional operations
   - **No dependencies** except standard library

2. **embeddenator-retrieval** (Week 1-2)
   - Inverted index for sparse ternary vectors
   - Resonator networks for pattern completion
   - Signature-based retrieval
   - **Depends on:** embeddenator-vsa

3. **embeddenator-fs** (Week 2)
   - EmbrFS filesystem implementation
   - FUSE integration (optional feature)
   - Algebraic correction layer
   - **Depends on:** embeddenator-vsa, embeddenator-retrieval

4. **embeddenator-interop** (Week 2-3)
   - Kernel VSA integration
   - Backend abstractions
   - **Depends on:** embeddenator-vsa, embeddenator-fs

5. **embeddenator-io** (Week 3)
   - Binary envelope format
   - Compression codecs (zstd, lz4)
   - **Independent** (no Embeddenator dependencies)

6. **embeddenator-obs** (Week 3)
   - Logging infrastructure
   - Metrics collection
   - High-resolution timing
   - **Independent** (no Embeddenator dependencies)

#### Phase 2B: Application Components (2 weeks)

Extract user-facing applications:

1. **embeddenator-cli** (Week 4)
   - Command-line interface
   - **Depends on:** All Phase 2A components

2. **MCP Servers** (Week 4)
   - embeddenator-context-mcp
   - embeddenator-security-mcp
   - embeddenator-screen-mcp
   - **Depends on:** embeddenator-vsa, embeddenator-obs

### Phase 3: Integration and Cleanup (2 weeks)

**Goal:** Finalize migration and remove monolith

**Actions:**
1. Merge all extraction branches
2. Update monorepo to be a thin integration layer
3. Publish all components to crates.io
4. Update documentation and examples
5. Performance regression testing
6. Remove obsolete code

### Component Dependency Graph

```
Level 0 (No dependencies):
  ├─ embeddenator-vsa
  ├─ embeddenator-io
  └─ embeddenator-obs

Level 1 (Depends on vsa):
  ├─ embeddenator-retrieval
  └─ embeddenator-bench

Level 2 (Depends on retrieval):
  └─ embeddenator-fs

Level 3 (Depends on fs):
  └─ embeddenator-interop

Level 4 (Depends on multiple):
  ├─ embeddenator-cli (depends on all)
  └─ embeddenator (main crate, re-exports)
```

### Versioning Strategy

Each component will follow semantic versioning independently:

- **embeddenator-vsa**: Core stability, likely 1.0.0 soon
- **embeddenator-retrieval**: Experimental features, 0.x.x for now
- **embeddenator-fs**: Beta quality, 0.x.x
- **embeddenator-io**: Stable format, could reach 1.0.0
- **embeddenator-obs**: Utility crate, 0.x.x

**Main crate:** `embeddenator` becomes a facade that re-exports all components with version pinning for compatibility.

## Consequences

### Positive

1. **Independent Development**: Teams can work on VSA, filesystem, and CLI simultaneously
2. **Faster Compilation**: Changes to VSA don't require rebuilding filesystem code
3. **Flexible Deployment**: Users can depend on only `embeddenator-vsa` for core operations
4. **Better Testing**: Component-specific tests run in isolation
5. **Clearer Ownership**: Each component has a dedicated maintainer
6. **Versioning Flexibility**: Can release VSA 1.1.0 without changing filesystem version
7. **Dependency Hygiene**: Filesystem users don't need to install VSA benchmark dependencies

### Negative

1. **Initial Overhead**: Significant upfront work to extract and test components
2. **Coordination Cost**: Must manage dependencies between 14+ repositories
3. **Documentation Burden**: Each component needs standalone documentation
4. **Breaking Changes**: API changes in one component may require updates in others
5. **Version Hell Risk**: Incompatible component versions could cause integration issues

### Mitigation Strategies

1. **Path Dependencies During Development**: Use `path = "../embeddenator-vsa"` during Phase 2, publish to crates.io in Phase 3
2. **Feature Flags**: Make expensive dependencies optional (e.g., `fuse` feature in embeddenator-fs)
3. **Comprehensive Testing**: Maintain integration tests in main crate to catch cross-component issues
4. **Version Pinning**: Main crate pins exact versions to ensure compatibility
5. **CI/CD Automation**: Automated testing across all components on every commit
6. **Documentation Generation**: Use `cargo doc` with inter-crate links

## References

- [CRATE_STRUCTURE_AND_CONCURRENCY.md](../CRATE_STRUCTURE_AND_CONCURRENCY.md) - Detailed component architecture
- [ADR-017: Phase 2A Component Extraction Strategy](ADR-017-phase2a-component-extraction.md) - Tactical extraction plan
- [SPLIT_TRACKER.md](../../SPLIT_TRACKER.md) - Progress tracking
- [Cargo Workspaces Documentation](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Tokio Architecture](https://tokio.rs) - Multi-crate async runtime (inspiration)
- [Serde Architecture](https://serde.rs) - Core + ecosystem pattern (inspiration)

## Related ADRs

- **ADR-001**: Sparse Ternary VSA (foundation for embeddenator-vsa)
- **ADR-009**: Deterministic Hierarchical Artifacts (affects embeddenator-fs)
- **ADR-013**: Hierarchical Manifest Format (part of embeddenator-fs)
- **ADR-017**: Phase 2A Component Extraction Strategy (tactical implementation)

## Notes

**Why not a Cargo workspace?** We chose separate repositories over a single workspace to:
- Enable independent CI/CD pipelines
- Allow different release cadences
- Facilitate open-source contributions to specific components
- Permit future language diversity (e.g., Python bindings for embeddenator-vsa)

**Future Considerations:**
- May consolidate some components post-1.0 if boundaries prove incorrect
- Could create additional components (e.g., `embeddenator-network` for distributed VSA)
- Might extract platform-specific code (e.g., `embeddenator-linux`, `embeddenator-macos`)

---

**Status:** Accepted  
**Author:** Tyler Zervas  
**Reviewers:** Workflow Orchestrator, QA Tester  
**Implementation:** Phase 1 complete, Phase 2A in progress (50% complete as of 2026-01-04)
