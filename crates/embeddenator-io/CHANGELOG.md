# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.21.0] - 2026-01-25

### Added
- **Compression Profiles Module**: Path-based automatic compression profile selection
- `CompressionProfile` struct with algorithm and level configuration
- `CompressionProfiler` for automatic profile selection based on file paths
- 9 predefined profiles optimized for VM filesystem encoding:
  - `kernel`: zstd-19 for /boot, /lib/modules (maximum compression)
  - `libraries`: zstd-9 for /usr/lib, /lib (balanced)
  - `binaries`: zstd-6 for /usr/bin, /bin (good compression, fast decode)
  - `config`: lz4 for /etc (fast access, frequent reads)
  - `logs`: zstd-3 for /var/log (compressible text)
  - `docs`: zstd-6 for /usr/share/doc
  - `media`: zstd-3 for media files (already compressed)
  - `runtime`: none for /tmp, /var/cache
  - `default`: zstd-6 fallback

## [0.20.0] - 2026-01-25

### Changed
- Graduated from alpha to stable release
- Migration from monolithic repository complete
- API stable for production use

### Fixed
- Code style improvements (removed unnecessary return statements)

## [0.20.0-alpha.1] - 2026-01-16

### Added
- Initial alpha release
- Envelope format for holographic engrams
- Serialization/deserialization support
- Optional compression (zstd, lz4) feature flags
