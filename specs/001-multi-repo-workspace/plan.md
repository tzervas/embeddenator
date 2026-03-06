# Plan: Multi-Repo Workspace Implementation

**Based on**: [spec.md](spec.md)  
**Status**: Draft  
**Created**: 2026-01-16  
**Estimated Effort**: 2-3 weeks

## Phase 1: Foundation (Completed )

### 1.1 Development Context Files 
**Effort**: 2 days  
**Owner**: Documentation Writer + Workflow Orchestrator

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

**Deliverables**: 12 `.copilot` files with Active Technologies, Project Structure, Commands, Conventions, Current Focus

### 1.2 Specification Directories 
**Effort**: 1 day  
**Owner**: Workflow Orchestrator

- [x] Create `specs/` in embeddenator
- [x] Create `specs/` in embeddenator-vsa
- [x] Create `specs/` in embeddenator-fs
- [x] Create `specs/` in embeddenator-cli
- [x] Create `specs/` in embeddenator-retrieval
- [x] Create `specs/` in embeddenator-io
- [x] Create `specs/` in embeddenator-interop
- [x] Create `specs/` in embeddenator-obs
- [x] Create `specs/` in embeddenator-testkit
- [x] Create `specs/` in embeddenator-workspace
- [x] Create `specs/` in embeddenator-contract-bench
- [x] Create `specs/001-multi-repo-workspace/` at workspace root

**Deliverables**: 12 empty `specs/` directories ready for SDD workflow

### 1.3 Agent Config Modernization 
**Effort**: 3 days  
**Owner**: Workflow Orchestrator + All Specialized Agents

- [x] Update embeddenator.agent.md to v2.0.0 with spec-kit integration
- [x] Update performance-tuner.agent.md to v2.0.0
- [x] Update rust-implementer.agent.md to v2.0.0
- [x] Update qa-tester.agent.md to v2.0.0
- [x] Update documentation-writer.agent.md to v2.0.0
- [x] Update workflow-orchestrator.agent.md to v2.0.0
- [x] Update integration-specialist.agent.md to v2.0.0
- [x] Update vas-mathematician.agent.md to v2.0.0

**Deliverables**: 8 agent configs with SDD workflow, .copilot references, Project Context sections

### 1.4 Workspace Specification 
**Effort**: 1 day  
**Owner**: Workflow Orchestrator + Documentation Writer

- [x] Create `specs/001-multi-repo-workspace/spec.md`
- [ ] Create `specs/001-multi-repo-workspace/plan.md` (this file)
- [ ] Create `specs/001-multi-repo-workspace/tasks.md`

**Deliverables**: Complete workspace specification with problem statement, proposed solution, success criteria

## Phase 2: Tooling Automation (In Progress ⏳)

### 2.1 Git Synchronization  Partially Complete
**Effort**: 2 days  
**Owner**: Workflow Orchestrator + Rust Implementer

- [x] `update_all.sh`: Fetch/pull with timeout protection 
- [ ] Add conflict detection and resolution strategies
- [ ] Add branch cleanup for orphaned upstream branches
- [ ] Add ahead/behind summary report
- [ ] Integrate with CI pipeline for automated sync checks

**Deliverables**: Robust git sync script with error handling and reporting

### 2.2 Version Management
**Effort**: 3 days  
**Owner**: Rust Implementer + Workflow Orchestrator

- [ ] Design `embeddenator-workspace bump-version` CLI
  - Parse all Cargo.toml files
  - Update version fields (package.version, dependencies.*.version)
  - Generate git commit with version bump message
- [ ] Implement version validator
  - Check for version drift across repos
  - Report incompatible dependency versions
  - Suggest fixes (e.g., "embeddenator-cli depends on embeddenator-vsa 0.20.0-alpha.1 but 0.20.0-alpha.2 exists")
- [ ] Add CI check for version consistency
  - Fail if versions are out of sync
  - Auto-create PR with version alignment fixes

**Deliverables**: `embeddenator-workspace` CLI with `bump-version` and `check-versions` commands

### 2.3 Dependency Management
**Effort**: 2 days  
**Owner**: Rust Implementer + Integration Specialist

- [ ] Create Cargo patch templates for local development
  - Generate `[patch.crates-io]` entries
  - Automate path resolution for multi-repo workspace
- [ ] Implement `embeddenator-workspace patch-local` CLI
  - Scan workspace for repos
  - Generate patch configuration
  - Write to Cargo.toml or .cargo/config.toml
- [ ] Implement `embeddenator-workspace patch-reset` CLI
  - Remove local patches
  - Restore git dependency references

**Deliverables**: Cargo patch automation for seamless local cross-repo development

### 2.4 Workspace Health Checks
**Effort**: 3 days  
**Owner**: QA Tester + Workflow Orchestrator

- [ ] Design health check dashboard
  - Git status (dirty files, ahead/behind, branch)
  - Version alignment (consistent across repos)
  - Test coverage (cargo tarpaulin integration)
  - Doc coverage (rustdoc --show-missing-docs)
  - Spec coverage (% of features with specs/)
- [ ] Implement `embeddenator-workspace health` CLI
  - Run all health checks
  - Generate Markdown report
  - Exit non-zero if critical failures
- [ ] Integrate with CI pipeline
  - Run health checks on every PR
  - Block merge if health degraded

**Deliverables**: Comprehensive workspace health monitoring

## Phase 3: Performance Baseline Establishment

### 3.1 Contract Benchmarking
**Effort**: 4 days  
**Owner**: Performance Tuner + QA Tester

- [ ] Define baseline benchmark suite
  - Ingest: 100MB directory → engram creation time
  - Extract: engram → directory restoration time
  - Query: similarity search latency
  - Bundle: 1000 vectors → bundled vector time
  - SIMD ops: AVX2/NEON performance
- [ ] Implement benchmark harness in `embeddenator-contract-bench`
  - Deterministic input generation (fixed seeds)
  - Stable execution environment (CPU affinity, no throttling)
  - JSON output for comparison
- [ ] Establish baselines for all components
  - Run benchmarks on reference hardware
  - Store snapshots in `baselines/snapshots/`
  - Document hardware specs (CPU, RAM, OS)
- [ ] CI integration
  - Run contract-bench on every PR
  - Compare against baselines
  - Fail if regression >10%
  - Generate flamegraphs for performance analysis

**Deliverables**: Contract benchmark suite with baselines and CI integration

### 3.2 Evaluation Framework Integration
**Effort**: 2 days  
**Owner**: QA Tester + Performance Tuner

- [ ] Document embeddenator-testkit usage
  - Fast evaluation loop (14 phases)
  - Baseline metrics (16.6 MB/s ingest, 40.8 MB/s extract)
  - Integration with main workflow
- [ ] Create evaluation automation
  - `embeddenator-workspace evaluate` CLI
  - Run evaluation across all repos
  - Compare against historical results
  - Generate regression report
- [ ] CI integration
  - Run evaluation on every PR
  - Track metrics over time
  - Alert on significant deviations

**Deliverables**: Automated evaluation framework with historical tracking

## Phase 4: CI/CD Pipeline

### 4.1 GitHub Actions Workflows
**Effort**: 5 days  
**Owner**: Integration Specialist + Workflow Orchestrator

- [ ] **Workflow: `ci.yml`** (runs on every PR)
  - Checkout all repos
  - Rust toolchain setup (stable + nightly)
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
  - `cargo fmt --check --workspace`
  - Health checks (`embeddenator-workspace health`)
  - Version validation (`embeddenator-workspace check-versions`)
- [ ] **Workflow: `bench.yml`** (runs on every PR + nightly)
  - Run contract benchmarks
  - Compare against baselines
  - Fail if regression >10%
  - Upload benchmark results as artifacts
  - Generate performance report
- [ ] **Workflow: `docs.yml`** (runs on push to main)
  - `cargo doc --no-deps --workspace`
  - Deploy to GitHub Pages
  - Update documentation index
- [ ] **Workflow: `release.yml`** (runs on git tag)
  - Validate version consistency
  - Run full test suite + benchmarks
  - `cargo publish` for public crates
  - Create GitHub release with changelog
  - Update WORKSPACE_TRACKER.md

**Deliverables**: Complete CI/CD pipeline with testing, benchmarking, docs, and releases

### 4.2 Branch Protection Rules
**Effort**: 1 day  
**Owner**: Integration Specialist

- [ ] Configure protected branches
  - `main`: Require PR reviews, passing CI, no force-push
  - `dev`: Require passing CI, allow fast-forward merge
- [ ] Configure CODEOWNERS
  - Assign component ownership
  - Require owner approval for critical paths
- [ ] Configure merge strategies
  - Squash commits for feature branches
  - Rebase for hotfixes
  - Merge commits for releases

**Deliverables**: Branch protection configured across all repos

## Phase 5: Documentation and Training

### 5.1 Developer Guides
**Effort**: 3 days  
**Owner**: Documentation Writer + Workflow Orchestrator

- [ ] **CONTRIBUTING.md** (workspace-level)
  - Multi-repo workflow explanation
  - SDD methodology (specify → plan → tasks)
  - Agent usage patterns
  - Local development setup (Cargo patches)
  - Testing and benchmarking guidelines
- [ ] **ARCHITECTURE.md** (workspace-level)
  - Dependency graph visualization
  - Component responsibilities
  - Cross-cutting concerns (observability, error handling)
  - Performance characteristics
- [ ] **SDD_TUTORIAL.md**
  - Step-by-step spec creation
  - Example feature implementation
  - Agent handoff patterns
  - QA loop best practices

**Deliverables**: Comprehensive developer documentation

### 5.2 Agent Training Materials
**Effort**: 2 days  
**Owner**: Documentation Writer

- [ ] **AGENT_USAGE_GUIDE.md**
  - When to use each specialized agent
  - Example prompts and expected responses
  - Handoff patterns and QA loops
  - Troubleshooting common issues
- [ ] Update agent config examples
  - Add more diverse examples
  - Document edge cases
  - Provide template responses

**Deliverables**: Agent usage guide and improved agent configs

### 5.3 Workspace README Updates
**Effort**: 1 day  
**Owner**: Documentation Writer

- [ ] Update workspace root README.md
  - Link to WORKSPACE_TRACKER.md
  - Link to SPEC_KIT_INTEGRATION.md
  - Link to developer guides
  - Quick start instructions
- [ ] Update per-repo READMEs
  - Link to .copilot for development context
  - Link to specs/ for feature specifications
  - Reference specialized agents

**Deliverables**: Updated READMEs across all repos

## Phase 6: Validation and Rollout

### 6.1 Internal Testing
**Effort**: 3 days  
**Owner**: QA Tester + All Agents

- [ ] **Functional Testing**
  - Test all CLI commands (`embeddenator-workspace` subcommands)
  - Validate git sync across all scenarios (clean, dirty, conflicts)
  - Test version bump automation
  - Validate Cargo patch generation
- [ ] **Integration Testing**
  - Full SDD workflow (specify → plan → tasks → implement → validate)
  - Agent handoff patterns
  - CI pipeline execution
  - Contract-bench regression detection
- [ ] **Performance Testing**
  - Measure overhead of workspace tooling
  - Validate baseline accuracy
  - Test CI pipeline latency

**Deliverables**: Comprehensive test report with pass/fail criteria

### 6.2 Team Onboarding
**Effort**: 2 days  
**Owner**: Workflow Orchestrator + Documentation Writer

- [ ] **Training Session 1: Multi-Repo Workflow**
  - Git synchronization (`update_all.sh`)
  - Cargo patches for local development
  - Cross-repo dependency management
- [ ] **Training Session 2: SDD Methodology**
  - Spec creation with `/speckit.specify`
  - Plan generation with `/speckit.plan`
  - Task execution with `/speckit.tasks`
- [ ] **Training Session 3: Agent Orchestration**
  - When to use each specialized agent
  - Handoff patterns and QA loops
  - Troubleshooting and best practices

**Deliverables**: Training materials and team onboarding completion

### 6.3 Rollout
**Effort**: 1 day  
**Owner**: Workflow Orchestrator

- [ ] Merge spec-kit integration to `main` across all repos
- [ ] Tag workspace architecture as `v1.0.0-workspace`
- [ ] Update WORKSPACE_TRACKER.md with rollout status
- [ ] Announce to team with documentation links

**Deliverables**: Multi-repo workspace architecture live in production

## Timeline Summary

| Phase | Duration | Dependencies | Owner |
|-------|----------|--------------|-------|
| **Phase 1: Foundation** |  1 week | None | Workflow Orchestrator |
| **Phase 2: Tooling** | 2 weeks | Phase 1 | Rust Implementer |
| **Phase 3: Performance** | 1 week | Phase 2 | Performance Tuner |
| **Phase 4: CI/CD** | 1.5 weeks | Phase 3 | Integration Specialist |
| **Phase 5: Documentation** | 1 week | Phase 4 | Documentation Writer |
| **Phase 6: Validation** | 1 week | Phase 5 | QA Tester |
| **Total** | **7.5 weeks** | Sequential | All Agents |

## Success Metrics

### Quantitative
-  12 `.copilot` files created (100% completion)
-  8 agent configs updated to v2.0.0 (100% completion)
- ⏳ CI pipeline success rate >95%
- ⏳ Contract-bench regression detection accuracy >99%
- ⏳ Version drift incidents: 0 per month
- ⏳ Developer onboarding time: <1 day

### Qualitative
- Developers report seamless cross-repo workflow
- SDD methodology adopted for all new features
- Agent orchestration reduces context switching
- Performance regressions caught before merge

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| **CI pipeline complexity** | Start with minimal workflow, iterate based on feedback |
| **Agent config learning curve** | Comprehensive training + examples in AGENT_USAGE_GUIDE.md |
| **Version drift during transition** | Manual validation during Phase 6 rollout |
| **Contract-bench false positives** | Tunable regression thresholds + manual review process |

## Next Steps

After this plan is approved:
1.  Complete Phase 1 (Foundation) - **DONE**
2. ⏳ Create `tasks.md` with executable checklist for Phase 2
3. ⏳ Begin Phase 2 implementation (Tooling Automation)
4. ⏳ Schedule weekly sync meetings to track progress
