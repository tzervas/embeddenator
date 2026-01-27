# Embeddenator Documentation

## Quick Links

| Document | Description |
|----------|-------------|
| [Roadmap](../ROADMAP.md) | Vision gaps and implementation timeline |
| [Project Charter](project-management/PROJECT_CHARTER.md) | Vision, objectives, scope |
| [Requirements](requirements/REQUIREMENTS.md) | User stories, functional requirements |
| [Gap Analysis](benchmarks/GAP_ANALYSIS.md) | Current state vs targets |
| [Refactor Plan](architecture/HOLOGRAPHIC_REFACTOR_PLAN.md) | Implementation roadmap |

## Documentation Structure

```
embeddenator/
├── ROADMAP.md                # Vision gaps and implementation timeline
└── docs/
    ├── README.md             # This file
    ├── adr/                  # Architecture Decision Records
    │   └── ADR-001-*.md     # VSA superposition storage
    ├── architecture/         # Design documents
    │   ├── HOLOGRAPHIC_REFACTOR_PLAN.md
    │   └── EXECUTION_PLAN.md
    ├── benchmarks/           # Performance analysis
    │   └── GAP_ANALYSIS.md
    ├── guides/               # How-to guides
    │   └── CI_CD_GUIDE.md
    ├── project-management/   # Project tracking
    │   └── PROJECT_CHARTER.md
    └── requirements/         # Specifications
        └── REQUIREMENTS.md
```

## Architecture Decision Records (ADRs)

ADRs document significant architectural decisions:

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-001](adr/ADR-001-vsa-superposition-storage.md) | VSA Superposition Storage | Accepted |

## Key Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Storage overhead | 5.3% (1MB) | <10% | ✅ Met |
| Accuracy | 92.27% | >94% | ⚠️ Gap |
| Encode throughput | 0.06 MB/s | >100 MB/s | ❌ Gap |
| Decode throughput | 0.02 MB/s | >200 MB/s | ❌ Gap |

## Contributing

See the main [README](../README.md) for contribution guidelines.
