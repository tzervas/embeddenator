# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Consume `trit-vsa` 0.3.0 (`default-features = false`) for `Trit` and `PackedTritVec` storage.
- Holographic bind remains element-wise trit **multiplication** (`P*P=P`, self-inverse); not `trit-vsa` subtraction-mod-3 bind.
- `cuda` feature stays `["dep:cudarc"]` only — no trit-vsa/cuda (cubecl) dual-backend.

### Added
- Optional `trit-vsa-simd` feature forwarding `trit-vsa/simd` (not folded into `simd`).
- Algebra guard tests in `tests/bt_phase1_algebra_guards.rs`.

### Fixed
- Retract the 0.21.1 claim that `qlora-candle-*` v0.8.4 is on crates.io.
  Those crate names do not exist. Candle is git-only:
  [tzervas/qlora-candle@use-qlora-gemm](https://github.com/tzervas/qlora-candle/tree/use-qlora-gemm)
  (package names stay `candle-*` 0.9.2). `qlora-paste` is 1.0.21.
  Guide: [embeddenator/MAINTAINED_DEPENDENCIES.md](https://github.com/tzervas/embeddenator/blob/main/MAINTAINED_DEPENDENCIES.md).
  This crate does not depend on candle (GPU path is `cudarc`).

## [0.21.1] - 2026-01-26

### Changed
- **Supply Chain Security**: Documented maintained dependency ecosystem
  - Projects using candle can now use `qlora-candle-*` crates from crates.io (v0.8.4)
  - `qlora-paste` (v1.0.20) replaces unmaintained `paste`
  - `qlora-gemm` (v0.20.0) replaces unmaintained `gemm`
- See [MAINTAINED_DEPENDENCIES.md](../MAINTAINED_DEPENDENCIES.md) for integration guide
- Upstream PR: https://github.com/huggingface/candle/pull/3335

## [0.21.0] - 2026-01-25

### Added
- **GPU Acceleration**: Dynamic GPU memory scaling with nvidia-smi detection
- Runtime compute capability querying for architecture detection
- SM120 Blackwell (RTX 50 series) optimization as default target
- `GpuMemoryConfig` struct with automatic memory limit configuration
- Configurable memory headroom (default 10%)
- `gpu_benchmark.rs` and `gpu_test.rs` examples for validation
- Support for SM75 (Turing), SM80/SM86 (Ampere), SM89 (Ada), SM120 (Blackwell)

### Changed
- `GpuArch` enum now uses derive(Default) with #[default] on Sm120

### Performance
- Verified 5-10x GPU acceleration on RTX 5080 (SM120) hardware
- Dynamic batch sizing based on available VRAM

## [0.20.1] - 2026-01-25

### Fixed
- Critical: Fixed vector invariant violation causing incorrect similarity calculations
- Self-similarity now correctly returns 1.000 (was incorrectly returning 0.478)
- Fixed PackedTritVec corruption propagation issues

### Added
- Comprehensive validation and stress tests

## [0.20.0-alpha.1] - 2026-01-16

### Added
- Initial alpha release of embeddenator-vsa
- Sparse ternary VSA primitives
- SIMD-optimized operations (optional feature)
