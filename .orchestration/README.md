# Embeddenator Orchestration Documentation

> **Generated**: January 9, 2026  
> **Status**: Initial Analysis Complete

This directory contains meta-documentation for orchestrating the Embeddenator multi-repository project.

## Contents

| Document | Description |
|----------|-------------|
| [PROJECT_INDEX.md](PROJECT_INDEX.md) | Component catalog and repository overview |
| [DEPENDENCY_GRAPH.md](DEPENDENCY_GRAPH.md) | Inter-component dependency mapping |
| [BRANCH_STATUS.md](BRANCH_STATUS.md) | Git branch analysis and merge recommendations |
| [ACTION_PLAN.md](ACTION_PLAN.md) | Prioritized cleanup and merge action plan |
| [STATUS_REPORT.md](STATUS_REPORT.md) | Executive summary and current state |

## Quick Status

| Metric | Value |
|--------|-------|
| Total Repositories | 11 |
| Critical PRs/Merges Needed | 3 |
| Dependabot PRs Pending | 14+ |
| Phase | 2A Component Decomposition (in progress) |
| Main Blocker | dev branch 71 commits ahead of main |

## Repository List

1. `embeddenator` - Main meta-repo
2. `embeddenator-vsa` - Vector Symbolic Architecture core
3. `embeddenator-retrieval` - Retrieval & resonator network
4. `embeddenator-fs` - FUSE filesystem (EmbrFS)
5. `embeddenator-obs` - Observability
6. `embeddenator-io` - Envelope format & serialization
7. `embeddenator-interop` - Kernel interop
8. `embeddenator-cli` - Command-line interface
9. `embeddenator-contract-bench` - Deterministic benchmarks
10. `embeddenator-testkit` - Test utilities
11. `embeddenator-workspace` - Workspace management
