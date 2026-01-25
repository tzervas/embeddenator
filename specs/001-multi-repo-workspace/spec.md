# Spec: Multi-Repo Workspace Architecture

**Status**: Draft  
**Created**: 2026-01-16  
**Authors**: GitHub Copilot (spec-kit SDD methodology)  
**Related**: [WORKSPACE_TRACKER.md](../../WORKSPACE_TRACKER.md), [SPEC_KIT_INTEGRATION.md](../../SPEC_KIT_INTEGRATION.md)

## Overview

Define the architectural structure, coordination mechanisms, and development workflows for the Embeddenator multi-repository workspace consisting of 11 Rust crates.

## Problem Statement

The Embeddenator project has evolved from a monolithic repository into a multi-component architecture with 11 specialized crates:
- **Core**: `embeddenator` (main CLI + coordinator)
- **VSA Primitives**: `embeddenator-vsa` (sparse ternary vectors, bundling, binding)
- **Filesystem**: `embeddenator-fs` (FUSE-based engram mounting)
- **Components**: `embeddenator-cli`, `embeddenator-io`, `embeddenator-retrieval`, `embeddenator-interop`, `embeddenator-obs`
- **Tooling**: `embeddenator-testkit`, `embeddenator-workspace`, `embeddenator-contract-bench`

### Challenges
1. **Version Synchronization**: Ensuring version alignment across 11 crates (currently 0.20.0-alpha.1 for most, 0.20.0-alpha.2 for fs)
2. **Dependency Management**: Managing cross-repo dependencies with Cargo patches for local development
3. **Git Coordination**: Coordinating branches, remotes, and sync status across repos
4. **Development Context**: Maintaining shared knowledge via agent configs and .copilot files
5. **Specification-Driven Development**: Adopting SDD methodology via GitHub spec-kit

## Goals

### Primary Goals
1. **Unified Development Experience**: Developers should seamlessly work across repos with consistent tooling
2. **Automated Coordination**: CI/CD pipelines automate version bumps, dependency updates, and cross-repo testing
3. **Specification-First**: All features follow SDD methodology (specify → plan → tasks → implement → validate)
4. **Performance Visibility**: Continuous benchmarking via contract-bench with baseline comparisons
5. **Component Independence**: Each crate can be developed, tested, and released independently

### Secondary Goals
1. **Documentation Consistency**: Shared architecture docs, ADRs, and contribution guides
2. **Agent Orchestration**: Multi-agent workflows coordinate specialized tasks (math → impl → optimize → test → doc)
3. **Workspace Utilities**: Helper tools for bulk operations (sync all, version bump, dependency check)

## Non-Goals
- **Monorepo Consolidation**: Not reverting to single-repo structure; multi-repo is intentional
- **External Publishing**: Some crates (workspace, contract-bench) remain internal-only
- **Non-Rust Components**: Embeddenator remains a pure-Rust project

## Proposed Solution

### Repository Structure
```
/home/kang/Documents/projects/embdntr/
├── embeddenator/              # Main coordinator + CLI
│   ├── .github/agents/        # Specialized agent configs (8 agents)
│   ├── .copilot               # Development context
│   ├── specs/                 # Feature specifications (SDD methodology)
│   ├── src/                   # Core implementation
│   └── Cargo.toml             # v0.20.0-alpha
├── embeddenator-vsa/          # VSA primitives (bundle, bind, cosine)
│   ├── .copilot
│   ├── specs/
│   ├── docs/ARCHITECTURE.md   # VSA design documentation
│   └── Cargo.toml
├── embeddenator-fs/           # FUSE filesystem
│   ├── .copilot
│   ├── specs/
│   └── Cargo.toml
├── embeddenator-cli/          # CLI interface
│   ├── .copilot
│   ├── specs/
│   └── Cargo.toml
├── [7 more component repos]   # io, retrieval, interop, obs, testkit, workspace, contract-bench
├── specs/                     # Workspace-level specifications
│   └── 001-multi-repo-workspace/
│       ├── spec.md            # This file
│       ├── plan.md            # Implementation plan
│       └── tasks.md           # Executable tasks
├── WORKSPACE_TRACKER.md       # Central repo status tracker
├── SPEC_KIT_INTEGRATION.md    # SDD workflow guide
└── update_all.sh              # Git sync helper script
```

### Coordination Mechanisms

#### 1. Version Management
- **Cargo Workspaces**: Use virtual manifests for local development
- **Cargo Patches**: Override git dependencies with local paths during development
- **Automated Bumping**: `embeddenator-workspace` provides CLI for synchronized version bumps

#### 2. Git Synchronization
- **update_all.sh**: Batch fetch/pull across all repos with timeout protection
- **Branch Strategy**: 
  - `main`: stable releases
  - `dev`: active development
  - `docs/*`: documentation updates
  - `fix/*`, `feat/*`: feature branches
- **Remote Tracking**: All repos → `https://github.com/tzervas/embeddenator-*`

#### 3. Development Context (`.copilot` Files)
Each repo includes `.copilot` with:
- **Active Technologies**: Rust version, key dependencies, features
- **Project Structure**: Directory layout, module organization
- **Commands**: Build, test, benchmark shortcuts
- **Code Conventions**: Style guide, architectural patterns
- **Current Focus**: Active work streams, recent changes
- **Agents**: References to specialized agent configs

#### 4. Specification-Driven Development (`specs/` Directories)
- **Workspace-Level**: `specs/001-multi-repo-workspace/` (this spec)
- **Repo-Level**: Each repo has `specs/` for feature specifications
- **SDD Workflow**: `/speckit.specify` → `/speckit.plan` → `/speckit.tasks`
- **Agent Integration**: All agent configs reference spec-kit commands

#### 5. Agent Orchestration (`.github/agents/`)
Specialized agents coordinate development:
- **embeddenator.agent.md**: Main VSA/engram/Rust expert (v2.0.0 with SDD)
- **performance-tuner.agent.md**: Optimization and profiling
- **rust-implementer.agent.md**: Code generation
- **qa-tester.agent.md**: Testing and evaluation
- **documentation-writer.agent.md**: Docs and guides
- **workflow-orchestrator.agent.md**: Multi-agent coordination
- **integration-specialist.agent.md**: Filesystem/kernel integration
- **vsa-mathematician.agent.md**: Mathematical foundations

### Dependency Graph
```
embeddenator (main)
├── embeddenator-vsa (core VSA primitives)
├── embeddenator-fs
│   └── embeddenator-vsa
├── embeddenator-cli
│   ├── embeddenator-vsa
│   ├── embeddenator-retrieval
│   │   └── embeddenator-vsa
│   ├── embeddenator-fs
│   └── embeddenator-io
├── embeddenator-interop
│   ├── embeddenator-vsa
│   └── embeddenator-fs
└── embeddenator-obs (optional tracing)

embeddenator-testkit (independent evaluation framework)
embeddenator-workspace (internal tooling)
embeddenator-contract-bench (benchmarking)
```

### Workflow Example: Adding a New Feature

1. **Specify** (`/speckit.specify multi-level-compression`):
   ```bash
   cd embeddenator
   # Create specs/008-multi-level-compression/spec.md
   # Define: Goal, problem statement, proposed solution, dependencies
   ```

2. **Plan** (`/speckit.plan specs/008-multi-level-compression/spec.md`):
   ```bash
   # Generate plan.md with:
   # - Mathematical derivation (VSA mathematician)
   # - Implementation steps (Rust implementer)
   # - Integration points (Integration specialist)
   # - Performance targets (Performance tuner)
   # - Test strategy (QA tester)
   # - Documentation (Documentation writer)
   ```

3. **Tasks** (`/speckit.tasks specs/008-multi-level-compression/plan.md`):
   ```bash
   # Generate tasks.md with executable checklist:
   # [ ] Math: Derive compression formula
   # [ ] Impl: Add MultiLevelEngram struct to embeddenator-vsa
   # [ ] Test: Property-based tests for reconstruction
   # [ ] Bench: Contract benchmark baseline
   # [ ] Docs: Update ARCHITECTURE.md
   ```

4. **Implement**:
   - Execute tasks across repos with agent handoffs
   - Use Cargo patches for local cross-repo development
   - Run tests and benchmarks continuously

5. **Validate**:
   - QA agent runs full evaluation loop
   - Contract-bench compares against baselines
   - Documentation writer finalizes guides
   - Workflow orchestrator ensures all steps complete

## Success Criteria

### Functional Requirements
- ✅ All 11 repos have synchronized `.copilot` files
- ✅ All agent configs updated to v2.0.0 with spec-kit integration
- ✅ `specs/` directories created in all repos
- ✅ Workspace-level spec (this document) exists
- ⏳ `update_all.sh` successfully syncs all repos (ahead/behind 0/0)
- ⏳ CI pipeline validates cross-repo compatibility
- ⏳ Contract-bench baselines established for all components

### Performance Requirements
- Version bump across all repos: <1 minute
- Full workspace sync (fetch + pull): <3 minutes
- Cross-repo integration tests: <10 minutes
- Contract-bench execution: <5 minutes

### Quality Requirements
- All repos follow consistent code conventions (Rust 2021, clippy, rustfmt)
- Agent configs reference .copilot and specs/ directories
- SDD methodology documented and enforced
- Performance baselines tracked in git

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| **Version Drift** | High - incompatible versions break integration | Automated dependency checks in CI; `embeddenator-workspace` version validator |
| **Git Sync Conflicts** | Medium - merge conflicts block development | Protected branches; rebase-first workflow; `update_all.sh` with conflict detection |
| **Orphaned Branches** | Low - tracking remote-deleted branches | Automated branch cleanup; alert on orphaned upstream |
| **Spec-Kit Adoption** | Medium - team needs training | Comprehensive guide (SPEC_KIT_INTEGRATION.md); agent configs enforce workflow |
| **Agent Config Divergence** | Low - inconsistent development experience | Template-based agent config updates; version tracking (v2.0.0) |

## Future Considerations

### Phase 2 (Q1 2026)
- Automated release pipeline (git tags → cargo publish)
- Cross-repo change impact analysis
- Workspace-level benchmarking dashboard
- Spec visualization tool (dependency graphs, completion status)

### Phase 3 (Q2 2026)
- Per-repo CI caching optimization
- Distributed contract-bench execution
- Agent config A/B testing framework
- Workspace health metrics (test coverage, doc coverage, spec coverage)

## References

- [GitHub spec-kit](https://github.com/github/spec-kit): Specification-Driven Development methodology
- [ADR-016](../embeddenator/docs/adr/ADR-016-component-decomposition.md): Component decomposition rationale
- [WORKSPACE_TRACKER.md](../../WORKSPACE_TRACKER.md): Real-time repo status
- [SPEC_KIT_INTEGRATION.md](../../SPEC_KIT_INTEGRATION.md): SDD workflow guide
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html): Official documentation
