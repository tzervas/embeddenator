# Embeddenator Version Roadmap

## Version Context

**Internal Development Version:** v1.0.0 (achieved January 2, 2026)
**First Public Release (crates.io):** v0.20.0-alpha.1 (January 9, 2026)

### Why 0.20.0-alpha.1 instead of 1.0.0?

This is the **first publication to crates.io**. Per Rust ecosystem conventions:
- Internal v1.0.0 = internal milestone achievement (full feature set, production-ready)
- Public v0.20.0-alpha.1 = first alpha release to gather community feedback
- crates.io versions follow independent numbering from internal milestones
- Alpha designation allows API refinement based on real-world usage
- Path to public v1.0.0 will incorporate community feedback and battle-testing

This approach follows the pattern of many major Rust projects that use conservative public versioning while maintaining internal milestones.

## Current State: v1.0.0 Internal / v0.20.0-alpha.1 Public (2026-01-02)

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

**Current Version: 1.0.0** (100% complete - Ready for release!)

**Completed for 1.0.0:**

#### P0: Critical Path ✅ COMPLETE
- ✅ **Performance benchmarks** (TASK-006)
  - Hierarchical scaling benchmarks (10MB in 6.18ms, linear O(n))
  - Query performance benchmarks (O(log n) hierarchical advantage)
  - SIMD infrastructure validation
  - Throughput documentation updated
- ✅ **Production stability**
  - Comprehensive error handling audit (QA_AUDIT_1.0.0_READINESS.md)
  - Critical unwrap fixes in production code
  - RwLock safety improvements
  - Error recovery test suite (19 tests)
  - Memory safety validation
  - Edge case coverage (unicode, deep hierarchies, large files)

#### P1: High Priority ✅ COMPLETE
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

#### P0: Deferred (Infrastructure-dependent)
- ⏸️ **ARM64 CI validation** (TASK-004, TASK-005)
  - Blocked by infrastructure availability
  - ARM64 code ready and tested locally
  - Will be enabled when self-hosted runners available
  - Not blocking 1.0.0 release (non-critical path)

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

#### v1.0.0 (Released: January 2, 2026) ✅
- All P0 and P1 tasks completed
- Incremental update support
- SIMD optimizations (AVX2/NEON)
- Performance benchmarks validated
- Production stability audit complete
- Comprehensive property-based testing
- Full documentation and 14 ADRs
- 231 tests (100% passing)
- Zero critical bugs
- API stability guarantee

#### v1.1.0 (Planned: Q1 2026)
- ARM64 CI fully operational (when infrastructure available)
- GPU acceleration research
- Additional performance optimizations

#### v1.2.0 (Planned: Q2 2026)
- Optional compression (zstd/lz4)
- FUSE mount production hardening
- Enhanced monitoring and observability

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
| ARM64 CI | ❌ | Ready | Ready | ⏸️ Deferred |
| Performance benchmarks | ❌ | ❌ | ❌ | ✅ |
| Incremental updates | ❌ | ❌ | ❌ | ✅ |
| SIMD optimization | ❌ | ❌ | ❌ | ✅ |
| Property testing | Basic | Good | Comprehensive | ✅ Complete |
| Production stability | ❌ | ❌ | ❌ | ✅ |
| Error recovery | ❌ | ❌ | Partial | ✅ |
| GPU support | ❌ | ❌ | ❌ | Planned |
| Compression | ❌ | ❌ | ❌ | Planned |

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
