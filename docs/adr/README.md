# Architecture Decision Records (ADR)

This directory contains Architecture Decision Records (ADRs) documenting significant architectural decisions made in the Embeddenator project.

## What is an ADR?

An Architecture Decision Record captures an important architectural decision made along with its context and consequences. ADRs help:
- Understand the rationale behind design choices
- Onboard new team members
- Prevent repeating past discussions
- Document architectural evolution over time

## ADR Format

Each ADR follows this structure:

- **Status**: Proposed | Accepted | Deprecated | Superseded
- **Date**: When the decision was made
- **Context**: The issue motivating this decision
- **Decision**: The change being proposed or made
- **Consequences**: Positive, negative, and neutral impacts

## Current ADRs

| ID | Title | Status | Date |
|----|-------|--------|------|
| [ADR-001](ADR-001-sparse-ternary-vsa.md) | Choice of Sparse Ternary VSA | Accepted | 2025-12-15 |
| [ADR-002](ADR-002-multi-agent-workflow-system.md) | Multi-Agent Workflow System | Accepted | 2025-12-15 |
| [ADR-003](ADR-003-self-hosted-runner-architecture.md) | Self-Hosted Runner Architecture | Accepted | 2025-12-22 |
| [ADR-004](ADR-004-holographic-os-container-design.md) | Holographic OS Container Design | Accepted | 2025-12-15 |
| [ADR-005](ADR-005-hologram-package-isolation.md) | Hologram-Based Package Isolation and Factoralization | Proposed | 2025-12-23 |
| [ADR-006](ADR-006-dimensionality-sparsity-scaling.md) | Dimensionality and Sparsity Scaling in Holographic Space | Proposed | 2025-12-23 |

## Creating a New ADR

When making a significant architectural decision:

1. Copy the template below
2. Number it sequentially (ADR-00X)
3. Fill in all sections
4. Submit for review via pull request
5. Update this README's table

### Template

```markdown
# ADR-XXX: [Title]

## Status

[Proposed | Accepted | Deprecated | Superseded by ADR-YYY]

## Date

YYYY-MM-DD

## Context

[Describe the issue or situation requiring a decision]

## Decision

[Describe the decision and its rationale]

## Consequences

### Positive
- [List positive outcomes]

### Negative
- [List negative outcomes and trade-offs]

### Neutral
- [List neutral impacts or considerations]

## References

- [Links to related documentation, issues, or PRs]
```

## References

- [Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) - Michael Nygard
- [ADR GitHub Organization](https://adr.github.io/) - Tools and resources
