# Embeddenator Workspace Roadmap

**Last Updated:** 2026-01-27
**Version:** 1.0.0
**Target Vision:** Unified holographic storage/RAM/VRAM with latent space computing

---

## Executive Summary

This roadmap tracks implementation gaps toward realizing the holographic computation vision for unified storage, RAM, and VRAM with latent space computing capabilities.

---

## Current State vs. Vision

| Component | Version | Core Readiness | Vision Alignment | Key Gaps |
|-----------|---------|----------------|------------------|----------|
| embeddenator-vsa | 0.21.1 | 95% | 40% | Latent semantics, VRAM persistence |
| embeddenator-fs | 0.24.0 | 70% | Partial | Streaming decode, update wiring |
| embeddenator-retrieval | 0.21.0 | 75% | Partial | Parallel/distributed search |
| embeddenator-io | 0.21.0 | 85% | Good | Streaming compression |
| embeddenator-cli | 0.21.0 | 86% | Good | Update commands blocked |
| embeddenator-obs | 0.21.0 | Ready | Ready | None |
| embeddenator-interop | 0.22.0 | Ready | Ready | None |
| embeddenator-testkit | 0.20.0 | Ready | Ready | Version alignment |
| embeddenator-core | 0.22.0 | Ready | Ready | None |

---

## Critical Blockers for Holographic Vision

These gaps block the core vision of unified storage/RAM/VRAM with latent space computation:

### 1. No Latent Space Semantics
- **Impact:** Cannot learn from data or perform reasoning
- **Missing:** Resonator networks, semantic variable inference, learned codebooks
- **Component:** embeddenator-vsa
- **Issue:** TBD

### 2. No VRAM Persistence Layer
- **Impact:** Cannot treat GPU memory as unified with RAM/storage
- **Missing:** GPU memory pool, host-device coherency protocol
- **Component:** embeddenator-vsa
- **Issue:** TBD

### 3. No Streaming Decode
- **Impact:** Large file extraction loads everything into RAM
- **Missing:** `stream_decode()` API for incremental reconstruction
- **Component:** embeddenator-fs
- **Issue:** TBD

### 4. No Parallel/Distributed Search
- **Impact:** Single-threaded, single-machine only
- **Missing:** rayon parallelization, distributed index protocol
- **Component:** embeddenator-retrieval
- **Issue:** TBD

### 5. Update Operations Blocked
- **Impact:** Cannot incrementally modify engrams via CLI
- **Blocked by:** embeddenator-fs `add_file`/`remove_file`/`modify_file`/`compact` wiring
- **Component:** embeddenator-cli
- **Issue:** TBD

---

## Phase 1: Critical Foundation (4-6 weeks)

**Focus:** Enable the holographic vision core capabilities

| Task | Component | Priority | Complexity | Issue | Status |
|------|-----------|----------|------------|-------|--------|
| Implement learned codebooks with resonator networks | vsa | CRITICAL | XL | [#37](https://github.com/tzervas/embeddenator/issues/37) | Open |
| Implement GPU VRAM persistence layer | vsa | CRITICAL | L | [#38](https://github.com/tzervas/embeddenator/issues/38) | Open |
| Add streaming decode API for incremental reconstruction | fs | HIGH | M | [#39](https://github.com/tzervas/embeddenator/issues/39) | Open |
| Implement parallel batch search with rayon | retrieval | CRITICAL | M | [#40](https://github.com/tzervas/embeddenator/issues/40) | Open |
| Wire update commands to fs operations | cli, fs | HIGH | M | [#41](https://github.com/tzervas/embeddenator/issues/41) | Open |

---

## Phase 2: Performance Optimization (3-4 weeks)

**Focus:** Production-ready throughput

| Task | Component | Priority | Complexity | Issue | Status |
|------|-----------|----------|------------|-------|--------|
| True SIMD intrinsics for intersection operations | vsa | HIGH | M | [#42](https://github.com/tzervas/embeddenator/issues/42) | Open |
| SIMD operations for PackedTritVec | vsa | HIGH | M | [#43](https://github.com/tzervas/embeddenator/issues/43) | Open |
| Matrix CUDA kernels for GPU acceleration | vsa | HIGH | L | [#44](https://github.com/tzervas/embeddenator/issues/44) | Open |
| Activate parallel encoding configuration | fs | MEDIUM | S | [#45](https://github.com/tzervas/embeddenator/issues/45) | Open |
| Streaming compression support | io | HIGH | M | [#46](https://github.com/tzervas/embeddenator/issues/46) | Open |

---

## Phase 3: Integration & Scale (6-8 weeks)

**Focus:** Unified memory system and distributed capabilities

| Task | Component | Priority | Complexity | Issue | Status |
|------|-----------|----------|------------|-------|--------|
| Virtual memory abstraction for large engrams | vsa | CRITICAL | XL | [#47](https://github.com/tzervas/embeddenator/issues/47) | Open |
| CPU/GPU/storage coherency protocol | vsa | CRITICAL | XL | [#48](https://github.com/tzervas/embeddenator/issues/48) | Open |
| Hierarchical HNSW indexing | retrieval | HIGH | M | [#49](https://github.com/tzervas/embeddenator/issues/49) | Open |
| Complete xattr FUSE integration | fs | MEDIUM | S | [#50](https://github.com/tzervas/embeddenator/issues/50) | Open |
| Distributed search protocol | retrieval | CRITICAL | XL | [#51](https://github.com/tzervas/embeddenator/issues/51) | Open |

---

## Quick Wins (4/5 Complete)

| Task | Component | Complexity | Issue | Status |
|------|-----------|------------|-------|--------|
| Enable signature module | retrieval | S | [#52](https://github.com/tzervas/embeddenator/issues/52) | **Done** |
| Activate parallel config in default settings | fs | S | [#53](https://github.com/tzervas/embeddenator/issues/53) | **Done** |
| Fix chunk recovery stub implementation | retrieval | S | [#54](https://github.com/tzervas/embeddenator/issues/54) | Open |
| Add remaining xattr FUSE method stubs | fs | S | [#55](https://github.com/tzervas/embeddenator/issues/55) | **Done** (already implemented) |
| Version bump testkit from 0.20 to 0.21 | testkit | S | [#56](https://github.com/tzervas/embeddenator/issues/56) | **Done** |

---

## Version Milestones

### v0.25 (Target: 6 weeks)
**Production Hardening**
- Streaming decode for large files
- Parallel search and encoding
- xattr/symlink FUSE support
- Update command implementation
- Streaming compression

### v0.30 (Target: 3 months)
**Semantic Intelligence**
- Learned codebooks (gradient optimization)
- Latent space operations (resonator networks)
- Query expansion/refinement
- Attention-weighted retrieval
- Symbol grounding framework

### v1.0 (Target: 6 months)
**Unified Memory Vision**
- Virtual memory abstraction (RAM/VRAM/storage)
- Coherency protocol (CPU <-> GPU <-> disk)
- Distributed search (petabyte scale)
- Multi-GPU support
- Edge deployment optimization

---

## Dependency Graph

```
QUICK WINS (parallel, immediate)
├── Enable signature module (retrieval)
├── Activate parallel config (fs)
├── Fix chunk recovery stub (retrieval)
├── Add xattr FUSE methods (fs)
└── Version bump testkit

PHASE 1 (Foundation - parallel tracks)
├── Learned codebooks (vsa) ──────────┐
├── VRAM persistence (vsa) ───────────┤── Start Week 1
├── Streaming decode (fs) ────────────┤
├── Parallel search (retrieval) ──────┤
└── Wire update commands (cli) ───────┘ ← After streaming decode

PHASE 2 (Performance - after Phase 1)
├── SIMD intrinsics (vsa) ← After codebooks
├── PackedTritVec SIMD (vsa) ← After codebooks
├── CUDA kernels (vsa) ← After VRAM
├── Parallel encoding (fs) ← After streaming
└── Streaming compression (io) ← Independent

PHASE 3 (Integration - after Phase 2)
├── Virtual memory (vsa) ← After SIMD + VRAM
├── Coherency protocol (vsa) ← After virtual memory
├── HNSW indexing (retrieval) ← After parallel search
├── xattr FUSE integration (fs) ← Independent
└── Distributed search (retrieval) ← After HNSW
```

---

## Success Metrics

| Metric | Current | v0.25 Target | v1.0 Target |
|--------|---------|--------------|-------------|
| Encode throughput | 0.06 MB/s | >10 MB/s | >100 MB/s |
| Decode throughput | 0.02 MB/s | >20 MB/s | >200 MB/s |
| Accuracy | 92.27% | >94% | >98% |
| Storage overhead | 3400% | <150% | <106% |
| Search latency (1M items) | N/A | <500ms | <100ms |
| Max file size | Limited | >1GB | Unlimited |
| Concurrent writers | 1 | >10 | >100 |

---

## Related Documentation

- **[Gap Analysis](./docs/benchmarks/GAP_ANALYSIS.md)** - Detailed performance gaps
- **[Execution Plan](./docs/architecture/EXECUTION_PLAN.md)** - Holographic FS maturity plan
- **[Refactor Plan](./docs/architecture/HOLOGRAPHIC_REFACTOR_PLAN.md)** - Architecture redesign
- **[Project Charter](./docs/project-management/PROJECT_CHARTER.md)** - Vision and objectives
- **[Component Roadmaps](#component-roadmaps)** - Per-component planning

### Component Roadmaps

- [embeddenator-fs Roadmap](./embeddenator-fs/ROADMAP.md)
- [embeddenator-core Version Roadmap](./embeddenator-core/VERSION_ROADMAP.md)

---

## Tracking

All gaps are tracked as GitHub issues with labels:
- `phase: 1`, `phase: 2`, `phase: 3` - Phase assignment
- `priority: critical`, `priority: high`, `priority: medium` - Priority level
- `complexity: S/M/L/XL` - Effort estimation
- `area: vsa`, `area: fs`, `area: retrieval`, etc. - Component assignment
- `quick-win` - Low-effort improvements
- `blocked` - Blocked by another issue

**Epic Issue:** [#36](https://github.com/tzervas/embeddenator/issues/36) - Gap Assessment Implementation

---

*Roadmap created: 2026-01-27 | Owner: @tzervas*
