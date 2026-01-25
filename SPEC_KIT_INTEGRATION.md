# Embeddenator Workspace Spec-Kit Integration

**Date**: 2026-01-16  
**Status**: Active  
**Framework**: [GitHub spec-kit](https://github.com/github/spec-kit)

---

## Overview

This workspace adopts **Specification-Driven Development (SDD)** methodology where specifications drive implementation rather than documentation following code. Each repo maintains its own specs/ directory with feature specifications, implementation plans, and tasks.

## Workspace Structure

```
embdntr/
├── specs/                           # Workspace-level specifications
│   └── 001-multi-repo-workspace/    # Feature: Multi-repo workspace
│       ├── spec.md                  # Feature specification
│       ├── plan.md                  # Implementation plan
│       └── tasks.md                 # Executable tasks
├── .copilot                         # Workspace-level development context
├── embeddenator/
│   ├── specs/                       # Repo-specific specifications
│   ├── .copilot                     # Repo-level development context
│   └── .github/agents/              # Agent configurations
├── embeddenator-fs/
│   ├── specs/
│   ├── .copilot
│   └── .github/agents/
└── [other repos...]

```

---

## Spec-Kit Commands

### `/speckit.specify`
Creates a new feature specification with automatic branch and directory management.

```bash
/speckit.specify Real-time performance monitoring with metrics collection
```

**Generates**:
- Branch: `00N-performance-monitoring`
- Directory: `specs/00N-performance-monitoring/`
- File: `spec.md` with structured requirements

### `/speckit.plan`
Converts specification into detailed implementation plan.

```bash
/speckit.plan Use Prometheus for metrics, Grafana for visualization
```

**Generates**:
- `plan.md` with technical architecture
- `data-model.md` with entities/schemas
- `contracts/` with API definitions
- `research.md` with technology evaluations

### `/speckit.tasks`
Derives executable tasks from implementation plan.

```bash
/speckit.tasks
```

**Generates**:
- `tasks.md` with prioritized, parallelizable task list

---

## Development Workflow

1. **Specify**: Describe feature in natural language → `/speckit.specify`
2. **Plan**: Define technical approach → `/speckit.plan`
3. **Task**: Generate work breakdown → `/speckit.tasks`
4. **Implement**: Execute tasks with AI assistance
5. **Validate**: Run tests derived from acceptance criteria
6. **Evolve**: Update specs as requirements change

---

## Agent Configuration

Each repo includes:
- `.copilot`: Project context (technologies, structure, conventions)
- `.github/agents/*.agent.md`: Specialized agents (VSA mathematician, performance tuner, etc.)

Agents reference specs/ for current feature context and align implementations with documented plans.

---

## Key Principles

1. **Specifications as Source of Truth**: Code serves specs, not vice versa
2. **Executable Specifications**: Precise enough to generate working systems
3. **Continuous Refinement**: Specs evolve with production learnings
4. **Research-Driven**: Agents gather context (library compatibility, benchmarks, constraints)
5. **Branching for Exploration**: Multiple implementations from same spec

---

## Per-Repo Integration

### embeddenator (Main)
- **Focus**: Core VSA operations, CLI, hierarchical encoding
- **Specs**: Performance tuning, SIMD optimizations, incremental updates
- **Agents**: embeddenator.agent, performance-tuner.agent, rust-implementer.agent

### embeddenator-fs
- **Focus**: FUSE integration, read-write engrams, transactional updates
- **Specs**: Read-write pivot, CLI app, hierarchical queries
- **Agents**: fs-integration.agent, vsa-mathematician.agent

### embeddenator-vsa
- **Focus**: Sparse ternary VSA primitives, SIMD, balanced ternary
- **Specs**: bundle_hybrid_many completion, AVX2/NEON, error handling
- **Agents**: vsa-mathematician.agent, simd-optimizer.agent

### embeddenator-testkit
- **Focus**: Evaluation framework, benchmarking, performance validation
- **Specs**: Large-scale testing, CI integration, regression detection
- **Agents**: qa-tester.agent, benchmark-engineer.agent

### embeddenator-retrieval
- **Focus**: Query engine, shift-sweep search, hierarchical retrieval
- **Specs**: Performance characterization, query optimization
- **Agents**: retrieval-optimizer.agent

### embeddenator-io
- **Focus**: Codebook, manifest, engram I/O
- **Specs**: Compression, streaming, format versioning
- **Agents**: io-engineer.agent

### embeddenator-interop
- **Focus**: Python/FFI bindings, cross-language integrations
- **Specs**: API surface stabilization, wheel builds
- **Agents**: interop-specialist.agent

### embeddenator-obs
- **Focus**: Observability, metrics, tracing
- **Specs**: Structured logging, dashboards, alerting
- **Agents**: observability-engineer.agent

### embeddenator-workspace
- **Focus**: CI/CD, orchestration, shared utilities
- **Specs**: Build automation, release coordination
- **Agents**: workflow-orchestrator.agent

---

## References

- [GitHub spec-kit](https://github.com/github/spec-kit)
- [spec-driven.md](/tmp/spec-kit/spec-driven.md) - SDD methodology guide
- [WORKSPACE_TRACKER.md](WORKSPACE_TRACKER.md) - Current repo states
- [CODE_CHANGES_ANALYSIS.md](CODE_CHANGES_ANALYSIS.md) - Code alignment review
