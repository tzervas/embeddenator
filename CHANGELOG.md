# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Gated against `embeddenator-vsa` wrap of `trit-vsa` 0.3. Holographic `SparseVec::bind` is unchanged (trit multiplication: `P*P=P`, self-inverse unbind-by-rebind); not trit-vsa subtraction-mod-3 (`P.bind(P)=Z`). No direct `trit-vsa` dependency; default features only (no `python` / `c-bindings`).

## [0.22.1] - 2026-01-26

### Changed
- **Supply Chain Security**: Documented maintained dependency ecosystem for unmaintained crates
  - Projects using candle for ML can use `qlora-candle` fork with maintained dependencies
  - `qlora-paste` (v1.0.20) replaces unmaintained `paste`
  - `qlora-gemm` (v0.20.0) replaces unmaintained `gemm`
  - See [MAINTAINED_DEPENDENCIES.md](../MAINTAINED_DEPENDENCIES.md) for integration guide
  - Upstream PR: https://github.com/huggingface/candle/pull/3335

## [0.22.0] - 2026-01-25

### Added
- Full envelope compression support (Zstd, LZ4)
- Automated cbindgen C header generation
- Round-trip compression verification
- Feature flags for optional compression algorithms

### Changed
- API stabilized for FFI and language interoperability
- Version bump to align with workspace releases

## [0.20.0-alpha.1] - 2026-01-16

### Added
- Initial alpha release
- Python FFI bindings
- C bindings foundation
- Integration with embeddenator-vsa, embeddenator-fs, embeddenator-io
