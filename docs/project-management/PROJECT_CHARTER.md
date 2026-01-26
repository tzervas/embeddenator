# Embeddenator Project Charter

**Project Name**: Embeddenator - Holographic Computing Substrate
**Version**: 1.0
**Date**: 2026-01-26
**Sponsor**: @tzervas

## 1. Executive Summary

Embeddenator is a holographic computing substrate that unifies storage, memory, and compute using Vector Symbolic Architecture (VSA). By representing data as high-dimensional sparse ternary vectors ("engrams"), we enable:

- **Content-addressable storage** with similarity search
- **Holographic superposition** where multiple items share one vector
- **Compute in latent space** without decode/re-encode overhead
- **System memory integration** for VSA-native RAM

## 2. Problem Statement

### Current State
Traditional computing maintains strict separation between:
- **Storage** (files on disk - raw bytes)
- **Memory** (RAM - addresses and values)
- **Compute** (CPU/GPU - arithmetic operations)

Data must be converted at each boundary, losing semantic information and creating overhead.

### Desired State
A unified representation where:
- Data is stored holographically (meaning preserved)
- Memory is content-addressable (find by similarity)
- Compute operates on representations directly (no conversion)
- Storage efficiency is comparable to compression

### Gap
No existing system provides all of these properties together with practical performance.

## 3. Objectives

### Primary Objectives

| Objective | Measure | Target | Timeline |
|-----------|---------|--------|----------|
| O1: Storage efficiency | Overhead vs raw | <10% for >100KB | Q1 2026 |
| O2: Reconstruction accuracy | % bytes correct | >94% uncorrected | Q1 2026 |
| O3: Encode throughput | MB/s | >100 MB/s (GPU) | Q2 2026 |
| O4: Decode throughput | MB/s | >200 MB/s (GPU) | Q2 2026 |

### Secondary Objectives

| Objective | Measure | Target | Timeline |
|-----------|---------|--------|----------|
| O5: FUSE stability | Uptime | 24h+ no crashes | Q1 2026 |
| O6: Similarity search | Latency | <100ms for 1M | Q2 2026 |
| O7: System memory PoC | Working demo | Functional | Q3 2026 |
| O8: Latent compute PoC | Working demo | Functional | Q4 2026 |

## 4. Scope

### In Scope

| Category | Items |
|----------|-------|
| Core VSA | Bind, bundle, similarity, sparse/dense representations |
| Encoding | Position-aware, chunked, reversible, trainable codebook |
| Storage | FUSE filesystem, versioning, journaling, corrections |
| Performance | GPU acceleration (CUDA), SIMD (AVX-512, NEON) |
| Integration | rust-ai-core, trit-vsa, tritter-accel, vsa-optim-rs |
| CLI | Ingest, mount, search, update commands |

### Out of Scope (v1.0)

| Category | Items | Rationale |
|----------|-------|-----------|
| Hardware | FPGA/ASIC implementation | Requires hardware expertise |
| Kernel | Kernel module for RAM | Risk, complexity |
| Distributed | Multi-node storage | Future phase |
| Languages | Non-Rust bindings | Focus on core first |

## 5. Success Criteria

### Must Have (Release Blockers)

- [ ] >94% reconstruction accuracy for text data
- [ ] <10% storage overhead for files >100KB
- [ ] FUSE mount with read/write support
- [ ] CLI for ingest, mount, search operations
- [ ] GPU batch encoding functional
- [ ] Documentation: API docs, user guide, architecture

### Should Have (High Priority)

- [ ] AVX-512 SIMD acceleration
- [ ] Block-sparse representation
- [ ] DeterministicPhaseTrainer integration
- [ ] Signal handlers for graceful unmount
- [ ] Extended attributes (xattr) support

### Could Have (Nice to Have)

- [ ] Online codebook learning
- [ ] Streaming encode/decode
- [ ] Compression on top of VSA
- [ ] WebAssembly bindings

## 6. Constraints

### Technical Constraints

| Constraint | Impact | Mitigation |
|------------|--------|------------|
| Rust only | Learning curve | Good documentation |
| Ternary VSA | Limited operations | Leverage existing theory |
| CUDA dependency | Hardware requirement | CPU fallback |

### Resource Constraints

| Resource | Limit | Impact |
|----------|-------|--------|
| Development time | Solo developer | Prioritize ruthlessly |
| Hardware | 1x RTX 5080, 1x 14700K | Target these specifically |
| Memory | 64GB RAM | Batch processing needed |

### External Dependencies

| Dependency | Risk | Mitigation |
|------------|------|------------|
| rust-ai-core | API changes | Pin versions |
| CUDA toolkit | Installation complexity | Docker images |
| FUSE | Platform support | Linux primary |

## 7. Stakeholders

| Role | Responsibility | Communication |
|------|----------------|---------------|
| Sponsor/Owner | Direction, decisions | GitHub issues |
| Contributors | Implementation | PRs, reviews |
| Users | Feedback, testing | Issues, discussions |

## 8. Milestones

### Phase 1: Foundation (Current - Q1 2026)
- [x] ReversibleVSAEncoder with 94% accuracy
- [x] Superposition storage model validated
- [ ] Block-sparse implementation
- [ ] VersionedEmbrFS integration

### Phase 2: Performance (Q2 2026)
- [ ] GPU batch encoding >100 MB/s
- [ ] AVX-512 SIMD >10 MB/s
- [ ] Memory-efficient representations
- [ ] Benchmark suite

### Phase 3: Polish (Q3 2026)
- [ ] Signal handlers
- [ ] Extended attributes
- [ ] Comprehensive testing
- [ ] Documentation complete

### Phase 4: R&D (Q4 2026+)
- [ ] System memory PoC
- [ ] Latent compute PoC
- [ ] Hardware acceleration exploration

## 9. Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Performance targets not met | Medium | High | Incremental optimization |
| Accuracy degradation at scale | Low | High | Extensive testing |
| GPU driver issues | Medium | Medium | CPU fallback |
| Scope creep | High | Medium | Strict prioritization |

## 10. Communication Plan

| Channel | Purpose | Frequency |
|---------|---------|-----------|
| GitHub Issues | Task tracking | Daily |
| GitHub Discussions | Design decisions | As needed |
| Commit messages | Change documentation | Per commit |
| CHANGELOG | Release notes | Per release |

## 11. Approval

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Project Sponsor | @tzervas | 2026-01-26 | ✓ |
