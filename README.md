# embeddenator-vsa

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/embeddenator-vsa/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-vsa/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/embeddenator-vsa/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-vsa/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

Vector Symbolic Architecture (VSA) operations for sparse and dense balanced ternary representations.

**Independent component** extracted from the Embeddenator monolithic repository. Part of the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

## Status

**Version**: 0.21.0 (published on crates.io)

### Implementation Status

- Core VSA Operations: Bundle, bind, permute
- Bitsliced Ternary: Optimized 2-bits-per-trit representation
- Sparse and Dense: Adaptive representation (SparseVec / PackedTritVec)
- SIMD-Ready: Word-level parallelism with auto-vectorization
- Tested: 53+ tests passing (unit + integration + doc tests)  

## Features

- **Balanced Ternary Arithmetic**: {-1, 0, +1} operations with mathematical guarantees
- **Bitsliced Representation**: 32 trits per u64 word for SIMD efficiency
- **Adaptive Storage**: Automatic selection between sparse (< 25% density) and dense (≥ 25% density)
- **High Performance**: Word-level operations with compiler auto-vectorization
- **GPU-Ready Design**: Coalesced memory access, branchless operations

## Documentation

### Core Documentation
- **[Bitsliced Ternary Design](docs/BITSLICED_TERNARY_DESIGN.md)** - Comprehensive guide to the bitslicing technique
- **[SIMD Design](docs/SIMD_DESIGN.md)** - SIMD strategy and implementation roadmap
- **[Ternary Representation Guide](docs/TERNARY_REPRESENTATION_GUIDE.md)** - Quick reference for choosing representations

## Quick Start

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

// Encode data to hypervectors
let config = ReversibleVSAConfig::default();
let vec1 = SparseVec::encode_data(b"hello", &config, None);
let vec2 = SparseVec::encode_data(b"world", &config, None);

// Bundle (superposition)
let bundled = vec1.bundle(&vec2);

// Bind (association)
let bound = vec1.bind(&vec2);

// Compute similarity
let similarity = vec1.cosine(&vec2);
println!("Similarity: {:.3}", similarity);
```

## Usage

```toml
[dependencies]
embeddenator-vsa = "0.21"
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run tests with all features
cargo test --all-features

# Check code quality
cargo clippy --all-features -- -D warnings

# For cross-repo work, use Cargo patches:
# Add to Cargo.toml:
# [patch."https://github.com/tzervas/embeddenator-vsa"]
# embeddenator-vsa = { path = "../embeddenator-vsa" }
```

## Architecture

### Representation Types

**SparseVec**: For sparse vectors (< 25% density)
- Memory: 8 bytes per non-zero trit
- Best for: Random data encoding, feature vectors
- Operations: Sorted merge algorithms

**PackedTritVec**: For dense vectors (≥ 25% density)  
- Memory: 2 bits per trit
- Best for: Bundle operations, SIMD/GPU acceleration
- Operations: Word-level bitwise operations (32 trits per u64)

See [Ternary Representation Guide](docs/TERNARY_REPRESENTATION_GUIDE.md) for detailed comparison.

### Performance

- **Dot Product**: O(n/32) for PackedTritVec, O(k log k) for SparseVec
- **Bind/Bundle**: O(n/32) for PackedTritVec, O(k) for SparseVec
- **SIMD**: Designed for compiler auto-vectorization; actual speedup varies by platform
- **Future**: Explicit SIMD (Phase 2) and GPU (Phase 5) planned

> **Note**: Performance characteristics are theoretical and depend on hardware, data patterns, and compiler optimizations. Run `cargo bench` to measure on your specific system.

## Roadmap

See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for complete roadmap.

### Near Term
- **v0.21.0** (Phase 2): Explicit SIMD (AVX2, AVX-512, NEON)
- **v0.22.0** (Phase 3): Documentation & usability improvements
- **v1.0.0** (Phase 4): Production optimization

### Future
- **v1.1.0** (Phase 5): GPU acceleration (CUDA, OpenCL, Vulkan)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## References

- **Component Decomposition**: [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md)
- **VSA Theory**: Kanerva, P. (2009). "Hyperdimensional Computing"
- **Balanced Ternary**: Knuth, D. (1998). "The Art of Computer Programming, Vol 2"

## License

MIT
