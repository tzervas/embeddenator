# Embeddenator Version Roadmap

## Current State: v0.19.3 (2026-01-02)

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

### Progress to v0.19.3

**Current Version: 0.19.3** (stable pre-1.0 milestone)

**Completed for v0.19.3:**

#### P0: Critical Path (Required for 1.0.0)
- ✅ **Incremental update support** (TASK-007)
  - Add/remove/modify files without full re-ingestion
  - Hybrid VSA bundle + soft-delete approach
  - Periodic compaction support
  - CLI update subcommands
  - 18 comprehensive tests
- ✅ **SIMD optimization** (TASK-009)
  - AVX2 (x86_64) and NEON (aarch64) implementations
  - Feature-gated with scalar fallback
  - Stable Rust (no nightly required)
  - 16 dedicated tests
  - Accuracy within 1e-10 of scalar
- ✅ **Expanded property-based testing** (TASK-010)
  - 28 property tests covering VSA algebraic properties
  - 23,000+ property checks
  - Bundling, binding, permutation properties validated
  - Sparsity and stress testing
  - Documented limitations for production use

#### Backlog: Infrastructure-dependent (Deprioritized)
- ⏸️ **ARM64 CI validation** (TASK-004, TASK-005)
  - Backlog: infrastructure-dependent (self-hosted ARM64 runner availability)
  - ARM64 runtime support exists; local validation is possible
  - CI workflow remains configured/manual-only until runners exist
  - Not scheduled for near-term delivery

#### P2: Nice-to-Have (Post-v0.19.3)
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

<<<<<<< Updated upstream
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

=======
#### v0.19.3 (Released: January 2, 2026) ✅
- Stable pre-1.0 milestone: core feature set, extensive QA, and deterministic artifacts
- Incremental update support (TASK-007)
- SIMD optimizations (TASK-009)
- Performance benchmarks validated (TASK-006)
- Expanded property-based testing (TASK-010)
- Production stability audit + error recovery suite

#### v0.20.0 (Planned: Q1 2026)
- GPU acceleration research (infrastructure-dependent)
- Additional performance optimizations

#### v0.21.0 (Planned: Q2 2026)
- Optional compression (zstd/lz4) (TASK-008)
- FUSE mount production hardening
- Enhanced monitoring and observability

>>>>>>> Stashed changes
### Feature Comparison

| Feature | v0.1.0 | v0.2.0 | v0.3.0 | v0.19.3 |
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

- **Pre-1.0.0 (current)**: Minor versions (0.x.0) may include breaking API changes
- **Patch versions** (0.x.y) are reserved for fixes and small improvements

### Ownership and Copyright

**Author:** Tyler Zervas <tz-dev@vectorweight.com>  
**Copyright:** (c) 2025-2026 Tyler Zervas  
**License:** MIT  
**Repository:** https://github.com/tzervas/embeddenator

All code, documentation, and project artifacts are owned by Tyler Zervas.
