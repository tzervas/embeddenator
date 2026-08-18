# embeddenator-fs Roadmap

**Current Version**: v0.20.0-alpha.3
**Target Version**: v1.0.0
**Estimated Timeline**: 6-8 weeks

---

## Milestone 1: Alpha → Beta (v0.20.0-beta.1)
**Timeline**: 2-3 weeks
**Goal**: Production-ready tooling and documentation

### Week 1: Critical Path

#### 🔴 P0: CLI Application
- [ ] Create `src/bin/embrfs.rs` main entry point
- [ ] Implement `ingest` command (directory → engram)
- [ ] Implement `extract` command (engram → directory)
- [ ] Implement `stats` command (show engram info)
- [ ] Implement `compact` command (remove deleted files)
- [ ] Add `clap` dependency for argument parsing
- [ ] Write CLI usage documentation
- [ ] Test CLI with sample datasets

**Deliverable**: Working `embrfs` binary

#### 🔴 P0: Examples
- [ ] `examples/basic_usage.rs` - Simple ingest/extract workflow
- [ ] `examples/incremental_updates.rs` - Add/remove/modify operations
- [ ] `examples/hierarchical.rs` - Hierarchical bundling demo
- [ ] Add example documentation in README
- [ ] Create sample datasets in `examples/data/`

**Deliverable**: 3+ working examples

### Week 2: Quality & Polish

#### 🟡 P1: Integration Tests
- [ ] Create `tests/integration/` directory
- [ ] `tests/integration/large_files.rs` - Test GB-scale ingestion
- [ ] `tests/integration/corruption.rs` - Verify correction layer
- [ ] `tests/integration/hierarchical.rs` - Multi-level queries
- [ ] `tests/integration/fuse_mount.rs` - FUSE operations (with feature)
- [ ] Add test fixtures in `tests/fixtures/`

**Deliverable**: Comprehensive integration test suite

#### 🟡 P1: Fix Bugs
- [ ] Fix `level_bundle` unused assignment warning (line 1765)
- [ ] Integrate `query_hierarchical_codebook()` into `extract_hierarchically()`
- [ ] Verify hierarchical bundling produces correct results
- [ ] Run clippy and fix all warnings

**Deliverable**: Clean build with zero warnings

#### 🟡 P1: Benchmarks
- [ ] Create `benches/` directory
- [ ] `benches/encoding.rs` - Encode/decode throughput
- [ ] `benches/query.rs` - Query latency (flat + hierarchical)
- [ ] `benches/fuse.rs` - FUSE operation latency
- [ ] Add criterion dependency
- [ ] Document baseline performance in README

**Deliverable**: Performance benchmarks with baselines

### Week 3: Documentation

#### 🟡 P1: Architecture Documentation
- [ ] Create `docs/ARCHITECTURE.md` - System design, VSA explanation
- [ ] Create `docs/PERFORMANCE.md` - Benchmarks, tuning guide
- [ ] Create `docs/DEPLOYMENT.md` - Production deployment guide
- [ ] Update README with CLI examples
- [ ] Update README with library API examples
- [ ] Add performance metrics to README

**Deliverable**: Comprehensive documentation

#### 🟢 P2: Observability
- [ ] Add `tracing` dependency
- [ ] Replace `println!` with structured logging
- [ ] Add `#[instrument]` to key methods
- [ ] Add trace/debug/info levels appropriately
- [ ] Document logging configuration

**Deliverable**: Structured logging throughout

### Release: v0.20.0-beta.1
- [ ] Update CHANGELOG.md
- [ ] Tag release in git
- [ ] Publish to crates.io
- [ ] Announce in embeddenator project

---

## Milestone 2: Beta → RC (v0.20.0-rc.1)
**Timeline**: 2-3 weeks
**Goal**: Advanced features and optimization

### Advanced Features

#### 🟡 P1: Streaming Ingestion
- [ ] Design `StreamingIngester` API
- [ ] Implement chunked streaming (avoid full file load)
- [ ] Add streaming example
- [ ] Benchmark memory usage improvement
- [ ] Document streaming API

**Deliverable**: Memory-efficient ingestion

#### 🟡 P1: Query Command in CLI
- [ ] Implement `query` command for content search
- [ ] Add query result formatting (JSON, table, tree)
- [ ] Support hierarchical vs flat query modes
- [ ] Add query examples
- [ ] Document query syntax

**Deliverable**: Content search via CLI

#### 🟢 P2: Compression Support
- [ ] Add `CompressionCodec` enum
- [ ] Integrate zstd compression (optional feature)
- [ ] Update `FileEntry` with compression metadata
- [ ] Add compression to CLI (`--compress` flag)
- [ ] Benchmark compression ratios

**Deliverable**: Optional compression (feature-gated)

#### 🟢 P2: Async Support
- [ ] Add `async` feature flag
- [ ] Create `async_ops` module
- [ ] Implement `ingest_directory_async()`
- [ ] Implement `extract_async()`
- [ ] Add async example
- [ ] Benchmark async vs sync performance

**Deliverable**: Async API (feature-gated)

### Optimization

#### 🟡 P1: Parameter Tuning
- [ ] Benchmark different `DEFAULT_CHUNK_SIZE` values
- [ ] Tune `HierarchicalQueryBounds` defaults
- [ ] Optimize sparsity thresholds
- [ ] Document tuning recommendations
- [ ] Create tuning guide

**Deliverable**: Optimized default parameters

#### 🟡 P1: Memory Optimization
- [ ] Implement memory-mapped engram reading
- [ ] Add LRU eviction for codebook entries
- [ ] Stream manifest parsing for large trees
- [ ] Benchmark memory footprint reduction

**Deliverable**: Reduced memory usage

### Release: v0.20.0-rc.1
- [ ] Update CHANGELOG.md
- [ ] Tag release in git
- [ ] Publish to crates.io
- [ ] Solicit feedback from early adopters

---

## Milestone 3: RC → Stable (v0.20.0 / v1.0.0)
**Timeline**: 1-2 weeks
**Goal**: Production stability and ecosystem integration

### Final Polish

#### 🟡 P1: Production Hardening
- [ ] Audit error handling (all error paths)
- [ ] Add retry logic for transient failures
- [ ] Implement graceful degradation strategies
- [ ] Add panic recovery in FUSE operations
- [ ] Security audit (no unsafe code issues)

**Deliverable**: Robust error handling

#### 🟡 P1: FUSE Mount Command
- [ ] Implement `mount` command in CLI
- [ ] Add unmount handling (signal handling)
- [ ] Support mount options (read-only, allow-other, etc.)
- [ ] Add mount example
- [ ] Document FUSE usage

**Deliverable**: Full FUSE integration in CLI

#### 🟢 P2: Ecosystem Integration
- [ ] Ensure compatibility with embeddenator-vsa v0.20.x
- [ ] Ensure compatibility with embeddenator-retrieval v0.20.x
- [ ] Test cross-crate integration
- [ ] Document version compatibility matrix

**Deliverable**: Verified cross-crate compatibility

### Release Decision

**Option A: v0.20.0 (Conservative)**
- Signals component extraction complete
- Aligns with embeddenator versioning
- Allows for breaking changes before v1.0.0

**Option B: v1.0.0 (Aggressive)**
- Signals production-ready stability
- Commits to API stability
- Requires comprehensive stability testing

**Recommendation**: Start with v0.20.0, move to v1.0.0 after 1-2 production deployments

---

## Post-1.0 Roadmap

### Future Enhancements (v1.1+)

#### Distributed Support
- [ ] Design replication protocol
- [ ] Implement sharding strategies
- [ ] Add remote engram access
- [ ] Network protocol (gRPC/HTTP)

#### Advanced Query
- [ ] Content-based search (beyond chunks)
- [ ] Path-based filtering
- [ ] Fuzzy matching
- [ ] Query result ranking

#### Observability
- [ ] Prometheus metrics integration
- [ ] OpenTelemetry tracing
- [ ] FUSE operation instrumentation
- [ ] Performance dashboards

#### Enterprise Features
- [ ] Encryption at rest
- [ ] Access control (ACLs)
- [ ] Audit logging
- [ ] Multi-tenancy support

---

## Success Criteria

### Beta Release (v0.20.0-beta.1)
- [ ] CLI can ingest 1GB dataset in <5 minutes
- [ ] All unit + integration tests pass
- [ ] 5+ working examples
- [ ] Documentation covers 80% of use cases
- [ ] Zero build warnings

### Stable Release (v0.20.0 or v1.0.0)
- [ ] Benchmarks meet defined baselines
- [ ] Zero critical bugs in issue tracker
- [ ] Used in at least 1 production deployment
- [ ] API stability guaranteed (semver)
- [ ] Published to crates.io

---

## Risk Mitigation

### Technical Risks
1. **Performance issues at scale** → Early benchmarking (Milestone 1)
2. **Memory exhaustion** → Streaming + memory optimization (Milestone 2)
3. **FUSE stability** → Integration tests (Milestone 1)
4. **Cross-crate compatibility** → Version alignment checks (Milestone 3)

### Timeline Risks
1. **Scope creep** → Strict prioritization (P0/P1/P2)
2. **Dependency issues** → Lock versions in Cargo.toml
3. **Testing bottleneck** → Parallelize test writing

---

## Open Questions

1. **Should FUSE be default or optional?**
   - Current: Optional feature
   - Consideration: Makes binary larger, Linux-specific

2. **Async-first or sync-first?**
   - Current: Sync API with optional async
   - Consideration: Async adds complexity

3. **Target Rust MSRV?**
   - Current: 1.84 (per CI config)
   - Consideration: Balance features vs compatibility

4. **Compression default codec?**
   - Options: zstd, lz4, brotli
   - Recommendation: zstd (best ratio/speed trade-off)

---

## Contributing

See each milestone's task list for contribution opportunities. Priority labels:
- 🔴 P0: Critical path items
- 🟡 P1: Important, not blocking
- 🟢 P2: Nice-to-have enhancements

**Current Focus**: Milestone 1, Week 1 (CLI + Examples)

---

**Last Updated**: 2026-01-14
**Maintained By**: embeddenator-fs core team
