# Embeddenator Workspace Status

**Last Updated:** 2026-01-26
**Workspace Version:** 0.22.0

---

## Component Matrix

| Component | Version | crates.io | Branch | Tests | Status |
|-----------|---------|-----------|--------|-------|--------|
| [embeddenator-core](./embeddenator-core/) | 0.22.0 | [published](https://crates.io/crates/embeddenator-core) | main | - | Production |
| [embeddenator-vsa](./embeddenator-vsa/) | 0.21.0 | [published](https://crates.io/crates/embeddenator-vsa) | main | 35 | Production |
| [embeddenator-io](./embeddenator-io/) | 0.21.0 | [published](https://crates.io/crates/embeddenator-io) | main | 21 | Production |
| [embeddenator-obs](./embeddenator-obs/) | 0.21.0 | [published](https://crates.io/crates/embeddenator-obs) | main | 58 | Production |
| [embeddenator-retrieval](./embeddenator-retrieval/) | 0.21.0 | [published](https://crates.io/crates/embeddenator-retrieval) | main | 23 | Production |
| [embeddenator-fs](./embeddenator-fs/) | 0.23.0 | [published](https://crates.io/crates/embeddenator-fs) | main | 74 | Alpha |
| [embeddenator-interop](./embeddenator-interop/) | 0.22.0 | [published](https://crates.io/crates/embeddenator-interop) | main | 27 | Production |
| [embeddenator-cli](./embeddenator-cli/) | 0.21.1 | [published](https://crates.io/crates/embeddenator-cli) | main | - | Production |
| [embeddenator-testkit](./embeddenator-testkit/) | 0.20.0 | internal | main | 37 | Internal |
| [embeddenator-contract-bench](./embeddenator-contract-bench/) | 0.20.0-alpha.2 | internal | main | 5 | Alpha |
| [embeddenator-workspace](./embeddenator-workspace/) | 0.20.0 | internal | main | 19 | Internal |

**Total Tests:** 299 unit tests across all components

---

## Container Images

| Image | Tag | Architecture | Registry |
|-------|-----|--------------|----------|
| embeddenator-core | 0.22.0 | amd64 | [ghcr.io/tzervas/embeddenator-core](https://ghcr.io/tzervas/embeddenator-core) |
| embeddenator-core | latest | amd64 | [ghcr.io/tzervas/embeddenator-core](https://ghcr.io/tzervas/embeddenator-core) |

```bash
# Pull latest
docker pull ghcr.io/tzervas/embeddenator-core:latest

# Pull specific version
docker pull ghcr.io/tzervas/embeddenator-core:0.22.0
```

**Note:** ARM64 images require native ARM64 runners (QEMU emulation is unreliable for Rust compilation).

---

## Maintained Dependencies

This workspace maintains forks of unmaintained crates in the Rust ML ecosystem:

| Fork | Original | Version | Purpose |
|------|----------|---------|---------|
| [qlora-paste](https://crates.io/crates/qlora-paste) | paste | 1.0.20 | Token pasting macros |
| [qlora-gemm](https://crates.io/crates/qlora-gemm) | gemm | 0.20.0 | Matrix multiplication |
| [qlora-candle-core](https://crates.io/crates/qlora-candle-core) | candle-core | 0.8.4 | ML framework |

**Upstream PR:** [huggingface/candle#3335](https://github.com/huggingface/candle/pull/3335) (open)

See [MAINTAINED_DEPENDENCIES.md](./MAINTAINED_DEPENDENCIES.md) for integration guide.

---

## Recent Changes

### 2026-01-26
- **embeddenator-core v0.22.0**: Package renamed from `embeddenator` to `embeddenator-core`
- **Container Images**: Published to ghcr.io/tzervas/embeddenator-core
- **Dependencies**: Updated rand 0.9, criterion 0.8
- **qlora-candle**: Now published to crates.io (v0.8.4)
- **CI/CD**: Removed nightly builds, updated to use `main` branch

---

## Remaining Tasks

### High Priority
- [ ] Fix default branches to `main` for all component repos
- [ ] ARM64 container images (requires native runners)
- [ ] Update component submodule references

### Medium Priority
- [ ] Publish embeddenator-testkit to crates.io (if needed externally)
- [ ] Add integration test coverage for CLI commands
- [ ] Document FUSE mount usage in production

### Low Priority
- [ ] Benchmark regression dashboard
- [ ] Multi-arch manifest for container images
- [ ] Performance optimization for large-scale datasets

---

## Vision Gaps

**Full Roadmap:** [ROADMAP.md](./ROADMAP.md)

The following critical gaps block the holographic vision of unified storage/RAM/VRAM with latent space computing:

| Gap | Component | Priority | Phase |
|-----|-----------|----------|-------|
| Latent space semantics | vsa | CRITICAL | 1 |
| VRAM persistence layer | vsa | CRITICAL | 1 |
| Streaming decode API | fs | HIGH | 1 |
| Parallel batch search | retrieval | CRITICAL | 1 |
| Virtual memory abstraction | vsa | CRITICAL | 3 |
| Distributed search | retrieval | CRITICAL | 3 |

**Timeline:**
- **v0.25** (6 weeks): Production hardening - streaming, parallel ops, FUSE improvements
- **v0.30** (3 months): Semantic intelligence - learned codebooks, latent operations
- **v1.0** (6 months): Unified memory vision - RAM/VRAM/storage coherency, distributed scale

See [Gap Analysis](./docs/benchmarks/GAP_ANALYSIS.md) for detailed component gaps.

---

## CI/CD Status

| Workflow | Trigger | Status |
|----------|---------|--------|
| Workspace CI | push/PR to main | Active |
| Benchmark Regression | push/PR to main | Active |
| Health Check | daily | Active |
| Documentation | push to main | Active |
| Security Scan | weekly | Active |
| Release | manual | Active |

**Removed:** Nightly builds (unnecessary overhead)

---

## Links

- **Documentation:** https://docs.rs/embeddenator-core
- **Repository:** https://github.com/tzervas/embeddenator
- **Issues:** https://github.com/tzervas/embeddenator/issues
- **Container Registry:** https://ghcr.io/tzervas/embeddenator-core

---

**Maintained by:** @tzervas
