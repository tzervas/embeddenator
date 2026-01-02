# Embeddenator Version Roadmap

## Current State: v0.3.0 (2026-01-01)

### Completed Features
- ✅ Core VSA implementation with sparse ternary vectors
- ✅ EmbrFS holographic filesystem with 100% bit-perfect reconstruction
- ✅ CLI toolchain (ingest, extract, query, query-text, bundle-hier)
- ✅ Hierarchical selective unfolding with store-backed retrieval
- ✅ Deterministic artifact generation with optional node sharding
- ✅ Multi-input ingest with automatic namespacing
- ✅ Comprehensive test suite (unit, integration, E2E, property-based)
- ✅ Zero clippy warnings
- ✅ Multi-architecture Docker support (amd64/arm64)
- ✅ Correction store for guaranteed reconstruction
- ✅ Resonator networks for pattern completion

### Progress to 1.0.0

**Current Version: 0.3.0** (60% toward 1.0.0)

**Remaining for 1.0.0:**

#### P0: Critical Path (Required for 1.0.0)
- [ ] **ARM64 CI validation** (TASK-004, TASK-005)
  - Self-hosted runner deployment and testing
  - Enable auto-trigger on main branch
  - Est: 2-3 days
- [ ] **Performance benchmarks** (TASK-006 final item)
  - TB-scale dataset validation
  - Hierarchical query performance metrics
  - Throughput benchmarks for encode/decode
  - Est: 1-2 days
- [ ] **Production stability**
  - Error handling audit
  - Edge case validation
  - Memory usage profiling
  - Est: 2-3 days

#### P1: High Priority (Strongly recommended for 1.0.0)
- [ ] **Incremental update support** (TASK-007)
  - Add/remove/modify files without full re-ingestion
  - Manifest consistency maintenance
  - Est: 3-4 days
- [ ] **SIMD optimization** (TASK-009)
  - Accelerated cosine similarity computation
  - Feature-gated with fallback
  - Est: 3-4 days
- [ ] **Expanded property-based testing** (TASK-010)
  - VSA algebraic invariants
  - Reconstruction guarantees
  - Est: 1-2 days

#### P2: Nice-to-Have (Post-1.0.0)
- [ ] **Compression options** (TASK-008)
  - Optional zstd/lz4 compression
  - Backward compatibility
  - Est: 1-2 days
- [ ] **GPU runner support** (TASK-CI-001 extension)
  - VSA acceleration research
  - Est: 5-7 days
- [ ] **FUSE mount production hardening**
  - Stability improvements
  - Performance optimization
  - Est: 3-5 days

### Version Milestones

#### v0.4.0 (Target: Q1 2026)
- ARM64 CI fully operational
- Performance benchmarks validated
- Incremental update support
- SIMD optimizations

#### v0.5.0 (Target: Q2 2026)
- Production stability validated
- Comprehensive property-based test coverage
- GPU acceleration prototype
- Compression options

#### v1.0.0 (Target: Q2-Q3 2026)
- All P0 and P1 tasks completed
- Full documentation and examples
- Production deployment validation
- API stability guarantee

### Feature Comparison

| Feature | v0.1.0 | v0.2.0 | v0.3.0 | v1.0.0 |
|---------|--------|--------|--------|--------|
| Core VSA | ✅ | ✅ | ✅ | ✅ |
| Basic ingest/extract | ✅ | ✅ | ✅ | ✅ |
| Query/similarity | ✅ | ✅ | ✅ | ✅ |
| Test coverage | Basic | Good | Comprehensive | Complete |
| Hierarchical encoding | ❌ | Partial | ✅ | ✅ |
| Deterministic artifacts | ❌ | ❌ | ✅ | ✅ |
| Multi-input ingest | ❌ | ❌ | ✅ | ✅ |
| Node sharding caps | ❌ | ❌ | ✅ | ✅ |
| ARM64 CI | ❌ | Ready | Ready | ✅ |
| Performance benchmarks | ❌ | ❌ | ❌ | ✅ |
| Incremental updates | ❌ | ❌ | ❌ | ✅ |
| SIMD optimization | ❌ | ❌ | ❌ | ✅ |
| GPU support | ❌ | ❌ | ❌ | Planned |
| Compression | ❌ | ❌ | ❌ | Optional |

### Breaking Changes Policy

- **Pre-1.0.0**: Minor versions (0.x.0) may include breaking API changes
- **Post-1.0.0**: Semantic versioning strictly followed
  - Major version for breaking changes
  - Minor version for backward-compatible features
  - Patch version for bug fixes

### Ownership and Copyright

**Author:** Tyler Zervas <tz-dev@vectorweight.com>  
**Copyright:** (c) 2025-2026 Tyler Zervas  
**License:** MIT  
**Repository:** https://github.com/tzervas/embeddenator

All code, documentation, and project artifacts are owned by Tyler Zervas.
