# Tasks: Multi-Repo Workspace Implementation

**Based on**: [plan.md](plan.md)  
**Status**: In Progress  
**Created**: 2026-01-16  
**Last Updated**: 2026-01-16

## Phase 1: Foundation  COMPLETED

### Development Context Files 
- [x] Create `.copilot` for workspace root
- [x] Create `.copilot` for embeddenator
- [x] Create `.copilot` for embeddenator-fs
- [x] Create `.copilot` for embeddenator-vsa
- [x] Create `.copilot` for embeddenator-testkit
- [x] Create `.copilot` for embeddenator-cli
- [x] Create `.copilot` for embeddenator-retrieval
- [x] Create `.copilot` for embeddenator-io
- [x] Create `.copilot` for embeddenator-interop
- [x] Create `.copilot` for embeddenator-obs
- [x] Create `.copilot` for embeddenator-workspace
- [x] Create `.copilot` for embeddenator-contract-bench

### Specification Directories 
- [x] Create `specs/` in all 11 repos + workspace root
- [x] Create `specs/001-multi-repo-workspace/spec.md`
- [x] Create `specs/001-multi-repo-workspace/plan.md`
- [x] Create `specs/001-multi-repo-workspace/tasks.md` (this file)

### Agent Config Modernization 
- [x] Update embeddenator.agent.md to v2.0.0
- [x] Update performance-tuner.agent.md to v2.0.0
- [x] Update rust-implementer.agent.md to v2.0.0
- [x] Update qa-tester.agent.md to v2.0.0
- [x] Update documentation-writer.agent.md to v2.0.0
- [x] Update workflow-orchestrator.agent.md to v2.0.0
- [x] Update integration-specialist.agent.md to v2.0.0
- [x] Update vas-mathematician.agent.md to v2.0.0

## Phase 2: Tooling Automation ⏳ NEXT

### Git Synchronization 
- [x] Create `update_all.sh` with fetch/pull and timeout protection
- [ ] **[P1]** Add conflict detection to `update_all.sh`
  - Detect merge conflicts during pull
  - Report conflicted files
  - Suggest resolution strategy (manual review, abort, force)
  - **Owner**: Rust Implementer
  - **Estimate**: 2 hours
- [ ] **[P1]** Add branch cleanup for orphaned upstreams
  - Detect branches with deleted upstream
  - Prompt for local branch deletion
  - Log cleanup actions
  - **Owner**: Rust Implementer
  - **Estimate**: 2 hours
- [ ] **[P2]** Enhance ahead/behind reporting
  - Show commit count ahead/behind for each repo
  - Highlight repos requiring push/pull
  - Add `--summary` flag for concise output
  - **Owner**: Rust Implementer
  - **Estimate**: 1 hour
- [ ] **[P2]** Integrate with CI pipeline
  - Run `update_all.sh` in GitHub Actions
  - Fail if any repo is out of sync
  - Auto-create PR with sync updates
  - **Owner**: Integration Specialist
  - **Estimate**: 3 hours

### Version Management  COMPLETED
- [x] **[P1]** Design `embeddenator-workspace bump-version` CLI
  - Parse all Cargo.toml files across repos
  - Update `package.version` field
  - Update dependency versions (e.g., `embeddenator-vsa = "0.20.0-alpha.2"`)
  - Generate git commit: `chore: bump version to 0.21.0-alpha`
  - Support flags: `--major`, `--minor`, `--patch`, `--prerelease`
  - **Owner**: Rust Implementer
  - **Estimate**: 1 day
  - **Status**: Implemented with full test coverage
- [x] **[P1]** Implement version validator
  - Check for version drift across repos
  - Report incompatible dependency versions
  - Suggest fixes with `cargo tree` analysis
  - Exit non-zero if drift detected
  - **Owner**: Rust Implementer + QA Tester
  - **Estimate**: 1 day
  - **Status**: Implemented as `check-versions` subcommand
- [ ] **[P2]** Add CI check for version consistency
  - Run `embeddenator-workspace check-versions` in CI
  - Fail PR if versions out of sync
  - Auto-create issue with version alignment recommendations
  - **Owner**: Integration Specialist
  - **Estimate**: 2 hours

### Dependency Management  COMPLETED
- [x] **[P1]** Create Cargo patch templates
  - Generate `[patch.crates-io]` or `[patch."https://github.com/..."]`
  - Automate path resolution for workspace repos
  - Support both `.cargo/config.toml` and root `Cargo.toml`
  - **Owner**: Rust Implementer
  - **Estimate**: 4 hours
  - **Status**: Completed - patches written to .cargo/config.toml
- [x] **[P1]** Implement `embeddenator-workspace patch-local` CLI
  - Scan workspace for repos
  - Generate patch configuration
  - Write to `.cargo/config.toml` (preferred) or `Cargo.toml`
  - Verify patches applied with `cargo metadata`
  - **Owner**: Rust Implementer
  - **Estimate**: 1 day
  - **Status**: Completed with --verify flag and comprehensive tests
- [x] **[P2]** Implement `embeddenator-workspace patch-reset` CLI
  - Remove local patches
  - Restore git dependency references
  - Clean cargo cache if needed
  - **Owner**: Rust Implementer
  - **Estimate**: 2 hours
  - **Status**: Completed with --clean flag and config preservation

### Workspace Health Checks  COMPLETED
- [x] **[P1]** Design health check dashboard
  - Git status: dirty files, ahead/behind, branch, orphaned upstreams
  - Version alignment: consistent across repos, dependency drift
  - Test coverage: cargo test integration (tarpaulin deferred)
  - Doc coverage: `cargo rustdoc -- -D warnings`
  - Spec coverage: % of features with `specs/` documentation
  - **Owner**: QA Tester + Workflow Orchestrator
  - **Estimate**: 1 day
  - **Status**: Implemented with 5 check categories
- [x] **[P1]** Implement `embeddenator-workspace health` CLI
  - Run all health checks in parallel (tokio async)
  - Generate Markdown report (stdout + file)
  - Exit non-zero if critical failures (version drift, failing tests)
  - Support `--json` for CI parsing
  - Support `--verbose` for detailed output
  - Support `--check` for selective checks
  - **Owner**: Rust Implementer + QA Tester
  - **Estimate**: 2 days
  - **Status**: Fully implemented with parallel execution, 26 tests passing
  - **Docs**: See HEALTH_CHECK_IMPLEMENTATION.md
- [ ] **[P2]** Integrate with CI pipeline
  - Run `embeddenator-workspace health` on every PR
  - Block merge if health degraded
  - Upload health report as GitHub Actions artifact
  - **Owner**: Integration Specialist
  - **Estimate**: 2 hours

## Phase 3: Performance Baseline Establishment 🔜

### Contract Benchmarking
- [ ] **[P1]** Define baseline benchmark suite
  - Ingest: 100MB directory → engram (measure throughput MB/s)
  - Extract: engram → directory (measure throughput MB/s)
  - Query: similarity search (measure latency ms)
  - Bundle: 1000 vectors (measure latency ms)
  - SIMD ops: AVX2/NEON performance comparison
  - **Owner**: Performance Tuner + QA Tester
  - **Estimate**: 2 days
- [ ] **[P1]** Implement benchmark harness in `embeddenator-contract-bench`
  - Deterministic input generation (fixed seed: `0x12345678`)
  - CPU affinity pinning (isolate from other processes)
  - JSON output format for comparison
  - Statistical analysis (mean, median, p95, p99)
  - **Owner**: Rust Implementer + Performance Tuner
  - **Estimate**: 2 days
- [ ] **[P1]** Establish baselines for all components
  - Run benchmarks on reference hardware (document specs)
  - Store snapshots in `baselines/snapshots/baseline-v0.20.0-alpha.json`
  - Git tag: `baseline-v0.20.0-alpha`
  - **Owner**: Performance Tuner
  - **Estimate**: 1 day
- [ ] **[P2]** CI integration for contract-bench
  - Run benchmarks on every PR
  - Compare against baselines with tolerance threshold (10%)
  - Fail if regression detected
  - Generate flamegraph with `cargo flamegraph` on regression
  - **Owner**: Integration Specialist + Performance Tuner
  - **Estimate**: 1 day

### Evaluation Framework Integration
- [ ] **[P2]** Document embeddenator-testkit usage
  - Fast evaluation loop (14 phases explained)
  - Baseline metrics (16.6 MB/s ingest, 40.8 MB/s extract documented)
  - Integration with main workflow (when to use testkit vs contract-bench)
  - **Owner**: Documentation Writer
  - **Estimate**: 1 day
- [ ] **[P2]** Create evaluation automation
  - `embeddenator-workspace evaluate` CLI
  - Run evaluation across all repos with testkit
  - Compare against historical results
  - Generate regression report (Markdown + JSON)
  - **Owner**: QA Tester + Rust Implementer
  - **Estimate**: 2 days
- [ ] **[P3]** CI integration for evaluation
  - Run evaluation on nightly builds (not every PR - too slow)
  - Track metrics over time (store in git or database)
  - Alert on significant deviations (>20% change)
  - **Owner**: Integration Specialist
  - **Estimate**: 1 day

## Phase 4: CI/CD Pipeline 🔜

### GitHub Actions Workflows
- [ ] **[P1]** Create `ci.yml` workflow
  - Trigger: on pull_request
  - Checkout all repos with `actions/checkout@v4`
  - Rust toolchain setup: `actions-rs/toolchain@v1` (stable + clippy + rustfmt)
  - `cargo build --workspace --all-features`
  - `cargo test --workspace --all-features`
  - `cargo clippy --workspace --all-features -- -D warnings`
  - `cargo fmt --check --workspace`
  - `embeddenator-workspace health`
  - `embeddenator-workspace check-versions`
  - **Owner**: Integration Specialist
  - **Estimate**: 1 day
- [ ] **[P1]** Create `bench.yml` workflow
  - Trigger: on pull_request + schedule (nightly)
  - Run contract benchmarks
  - Compare against baselines from git tag
  - Fail if regression >10%
  - Upload benchmark results as artifacts
  - Comment on PR with performance summary
  - **Owner**: Integration Specialist + Performance Tuner
  - **Estimate**: 1 day
- [ ] **[P2]** Create `docs.yml` workflow
  - Trigger: on push to main
  - `cargo doc --no-deps --workspace`
  - Deploy to GitHub Pages via `peaceiris/actions-gh-pages@v3`
  - Update documentation index (redirect from root to embeddenator crate docs)
  - **Owner**: Integration Specialist + Documentation Writer
  - **Estimate**: 4 hours
- [ ] **[P2]** Create `release.yml` workflow
  - Trigger: on tag push matching `v*.*.*`
  - Validate version consistency (`embeddenator-workspace check-versions`)
  - Run full test suite + benchmarks
  - `cargo publish` for public crates (embeddenator, embeddenator-vsa, embeddenator-fs, embeddenator-cli, etc.)
  - Create GitHub release with auto-generated changelog
  - Update WORKSPACE_TRACKER.md with release status
  - **Owner**: Integration Specialist + Workflow Orchestrator
  - **Estimate**: 2 days

### Branch Protection Rules
- [ ] **[P1]** Configure protected branches
  - `main`: Require 1 PR review, passing CI, no force-push, require linear history
  - `dev`: Require passing CI, allow fast-forward merge only
  - Apply to all 11 repos + workspace root
  - **Owner**: Integration Specialist
  - **Estimate**: 1 hour
- [ ] **[P2]** Configure CODEOWNERS
  - Assign component ownership (e.g., embeddenator-vsa → @vsa-team)
  - Require owner approval for critical paths (Cargo.toml, .github/, src/lib.rs)
  - **Owner**: Workflow Orchestrator
  - **Estimate**: 2 hours
- [ ] **[P3]** Configure merge strategies
  - Squash commits for feature branches (keeps history clean)
  - Rebase for hotfixes (maintains linear history)
  - Merge commits for releases (preserves full history)
  - **Owner**: Integration Specialist
  - **Estimate**: 1 hour

## Phase 5: Documentation and Training 🔜

### Developer Guides
- [ ] **[P1]** Create `CONTRIBUTING.md` (workspace-level)
  - Multi-repo workflow explanation
  - SDD methodology (specify → plan → tasks)
  - Agent usage patterns with examples
  - Local development setup (Cargo patches, testing)
  - Code review guidelines
  - **Owner**: Documentation Writer + Workflow Orchestrator
  - **Estimate**: 1 day
- [ ] **[P1]** Create `ARCHITECTURE.md` (workspace-level)
  - Dependency graph visualization (mermaid diagram)
  - Component responsibilities matrix
  - Cross-cutting concerns (observability, error handling, SIMD)
  - Performance characteristics (latency, throughput, memory)
  - **Owner**: Documentation Writer + VSA Mathematician
  - **Estimate**: 2 days
- [ ] **[P2]** Create `SDD_TUTORIAL.md`
  - Step-by-step spec creation walkthrough
  - Example feature implementation (e.g., multi-level compression)
  - Agent handoff patterns demonstrated
  - QA loop best practices
  - **Owner**: Documentation Writer + Workflow Orchestrator
  - **Estimate**: 1 day

### Agent Training Materials
- [ ] **[P2]** Create `AGENT_USAGE_GUIDE.md`
  - When to use each specialized agent (decision tree)
  - Example prompts and expected responses (20+ examples)
  - Handoff patterns and QA loops (flowchart)
  - Troubleshooting common issues (FAQ)
  - **Owner**: Documentation Writer
  - **Estimate**: 2 days
- [ ] **[P3]** Update agent config examples
  - Add 5 more diverse examples per agent
  - Document edge cases (e.g., large hierarchies, sparse data)
  - Provide template responses for common queries
  - **Owner**: Documentation Writer + All Specialized Agents
  - **Estimate**: 1 day

### Workspace README Updates
- [ ] **[P1]** Update workspace root `README.md`
  - Link to WORKSPACE_TRACKER.md
  - Link to SPEC_KIT_INTEGRATION.md
  - Link to developer guides (CONTRIBUTING, ARCHITECTURE, SDD_TUTORIAL)
  - Quick start instructions (clone, build, test, run)
  - **Owner**: Documentation Writer
  - **Estimate**: 2 hours
- [ ] **[P2]** Update per-repo `README.md` files
  - Link to `.copilot` for development context
  - Link to `specs/` for feature specifications
  - Reference specialized agents (e.g., "Use @vsa-mathematician for math questions")
  - Add badges (CI status, docs coverage, version)
  - **Owner**: Documentation Writer
  - **Estimate**: 4 hours

## Phase 6: Validation and Rollout 🔜

### Internal Testing
- [ ] **[P1]** Functional testing
  - Test all `embeddenator-workspace` CLI commands
  - Validate git sync across scenarios (clean, dirty, conflicts, orphaned branches)
  - Test version bump automation (major, minor, patch, prerelease)
  - Validate Cargo patch generation (local paths, git dependencies)
  - **Owner**: QA Tester
  - **Estimate**: 2 days
- [ ] **[P1]** Integration testing
  - Full SDD workflow end-to-end (specify → plan → tasks → implement → validate)
  - Agent handoff patterns with real feature implementation
  - CI pipeline execution on test PR
  - Contract-bench regression detection with artificial slowdown
  - **Owner**: QA Tester + All Specialized Agents
  - **Estimate**: 3 days
- [ ] **[P2]** Performance testing
  - Measure overhead of workspace tooling (health checks, version validation)
  - Validate baseline accuracy (run benchmarks 10x, check variance <5%)
  - Test CI pipeline latency (target: <10 minutes for full suite)
  - **Owner**: Performance Tuner + QA Tester
  - **Estimate**: 1 day

### Team Onboarding
- [ ] **[P1]** Training Session 1: Multi-Repo Workflow
  - Git synchronization demo (`update_all.sh`)
  - Cargo patches for local development demo
  - Cross-repo dependency management walkthrough
  - **Owner**: Workflow Orchestrator
  - **Estimate**: 2 hours
- [ ] **[P1]** Training Session 2: SDD Methodology
  - Spec creation with `/speckit.specify` demo
  - Plan generation with `/speckit.plan` walkthrough
  - Task execution with `/speckit.tasks` example
  - **Owner**: Workflow Orchestrator + Documentation Writer
  - **Estimate**: 2 hours
- [ ] **[P1]** Training Session 3: Agent Orchestration
  - When to use each specialized agent (examples)
  - Handoff patterns and QA loops (live demo)
  - Troubleshooting and best practices (Q&A)
  - **Owner**: Workflow Orchestrator + All Specialized Agents
  - **Estimate**: 2 hours

### Rollout
- [ ] **[P1]** Merge spec-kit integration to `main`
  - Create PR with all Phase 1-6 changes
  - Require review from all specialized agents
  - Merge to `main` across all 11 repos + workspace root
  - **Owner**: Workflow Orchestrator
  - **Estimate**: 1 day
- [ ] **[P1]** Tag workspace architecture
  - Git tag: `v1.0.0-workspace` on all repos
  - Create GitHub release with summary of changes
  - **Owner**: Workflow Orchestrator
  - **Estimate**: 1 hour
- [ ] **[P1]** Update WORKSPACE_TRACKER.md
  - Mark all Phase 1-6 tasks as complete
  - Update status badges (CI passing, docs deployed)
  - Document rollout date and version
  - **Owner**: Documentation Writer
  - **Estimate**: 1 hour
- [ ] **[P1]** Announce to team
  - Send announcement email with links to guides
  - Schedule follow-up Q&A session
  - Create #embeddenator-workspace Slack channel
  - **Owner**: Workflow Orchestrator
  - **Estimate**: 30 minutes

## Priority Legend
- **[P1]**: Critical - blocks further progress
- **[P2]**: Important - enhances workflow
- **[P3]**: Nice-to-have - quality-of-life improvements

## Status Legend
-  Completed
-  In Progress
- ⏳ Next
- 🔜 Planned
-  Blocked
