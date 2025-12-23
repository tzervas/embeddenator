# ADR-005: Hologram-Based Package Isolation and Factoralization

## Status

Proposed

## Date

2025-12-23

## Context

The Embeddenator holographic computing substrate requires a mechanism to isolate and manipulate individual packages or package sets within a holographic container without requiring full reconstruction. Traditional container and packaging systems require unpacking, modifying, and repacking the entire container to update a single package, which is inefficient for large holographic OS containers.

### Problem Statement

In holographic OS containers (e.g., Debian/Ubuntu distributions encoded as engrams), we need to:
- Isolate a single package or package set from the holographic superposition
- Bundle all packages *except* the target package(s) into a complementary hologram
- Represent the isolated package hologram in a compact, two-way encoded format
- Enable efficient factorization and reconstruction without full extraction
- Optimize for contemporary 64-bit hardware without requiring AVX or other SIMD extensions (though able to leverage them when available)

### Requirements

1. **Package Isolation**: Ability to algebraically separate one package from a holographic engram containing multiple packages
2. **Complementary Bundling**: Create a "negative" hologram representing everything except the target package
3. **Compact Encoding**: Store hologram representations efficiently using balanced ternary mathematics
4. **Hardware Efficiency**: Optimize for 64-bit registers without requiring specialized CPU extensions
5. **Two-Way Encoding**: Hash-like representation that decodes back to the superposed vectors
6. **Factoralization**: Mathematical decomposition of holograms into constituent packages

## Decision

We implement a **Hologram Package Isolation** system with the following design:

### 1. Algebraic Package Isolation

Using VSA (Vector Symbolic Architecture) operations, we can isolate packages through algebraic manipulation:

```
Given: root_engram = pkg_A ⊕ pkg_B ⊕ pkg_C ⊕ ... ⊕ pkg_N

To isolate pkg_X:
1. Bundle all other packages: complementary = pkg_A ⊕ ... ⊕ pkg_(X-1) ⊕ pkg_(X+1) ⊕ ... ⊕ pkg_N
2. Extract target: isolated_pkg_X = root_engram ⊖ complementary
   where ⊖ is the inverse bundle operation (element-wise subtraction)
```

### 2. Factorialized Hologram Representation

The hologram is encoded as a **flat-rendered vector set** with two-way encoding:

**Forward Encoding** (vector → compact representation):
- Input: Sparse ternary vector with positive and negative indices
- Output: Compact hash-like representation that preserves all information
- Process: Encode the superposition state as a balanced ternary value set

**Reverse Encoding** (compact representation → vector):
- Input: Hash-like compact representation
- Output: Full sparse ternary vector with reconstructed superposition
- Process: Decode the balanced ternary representation back to vector indices

This representation acts as a **holographic fingerprint** that:
- Looks like a hash value for compact storage
- Contains complete information to reconstruct the full vector
- Preserves the superposed state of multiple packages
- Enables algebraic operations without full expansion

### 3. Balanced Ternary Implementation

We leverage **balanced ternary mathematics** for optimal hardware utilization:

#### Ternary Basics

- **Trits** (ternary digits): {-1, 0, +1} representing the three states
- **Trytes** (ternary bytes): Groups of trits that efficiently map to binary hardware
- Balanced ternary: Uses -1, 0, +1 instead of 0, 1, 2 for mathematical elegance

#### Optimal Trit-to-Bit Mapping

**40-trit encoding in 64-bit registers**:
```
3^40 = 12,157,665,459,056,928,801 ≈ 2^63.4

Mapping for unsigned 64-bit:
- 40 trits can represent any value from 0 to 3^40-1
- Fits in an unsigned 64-bit integer (2^64 = 18,446,744,073,709,551,616)
- Each trit encodes log₂(3) ≈ 1.585 bits of information
- 40 trits × 1.585 = 63.4 bits (optimal for 64-bit registers)

For signed 64-bit with balanced ternary:
- Use 39 trits: 3^39 = 4,052,555,153,018,976,267 < 2^63
- Range: -(3^39-1)/2 to +(3^39-1)/2
- Still optimal: 39 trits × 1.585 = 61.8 bits
```

**Tryte Structure**:
- A **tryte** is a group of trits optimized for register operations
- Standard tryte: 5 trits (3^5 = 243 values, fits in 8 bits)
- Extended tryte: 8 trits (3^8 = 6,561 values, fits in 13 bits)
- Register tryte: 40 trits (optimal for 64-bit operations)

#### Hardware Optimization Strategy

**64-bit Register Operations** (no AVX required):
```rust
// Encode sparse vector indices into balanced ternary
// 39 trits per 64-bit register (for signed balanced ternary)
struct BalancedTernary64 {
    value: i64,  // Stores up to 39 trits in balanced representation
}

impl BalancedTernary64 {
    // Convert trit vector to 64-bit value
    fn from_trits(trits: &[i8]) -> Self {
        let mut value: i64 = 0;
        let mut multiplier: i64 = 1;
        
        for &trit in trits {
            value += (trit as i64) * multiplier;
            multiplier *= 3;
        }
        
        BalancedTernary64 { value }
    }
    
    // Extract trits from 64-bit value
    fn to_trits(&self, num_trits: usize) -> Vec<i8> {
        let mut trits = Vec::with_capacity(num_trits);
        let mut remaining = self.value;
        
        for _ in 0..num_trits {
            // Properly handle balanced ternary {-1, 0, +1}
            // Use Euclidean modulo for consistent behavior with negative numbers
            let mut trit = ((remaining % 3) + 3) % 3;
            remaining = (remaining - trit) / 3;
            
            // Convert to balanced representation: {0, 1, 2} -> {0, 1, -1}
            if trit == 2 {
                trit = -1;
                remaining += 1; // Carry adjustment
            }
            
            trits.push(trit as i8);
        }
        
        trits
    }
}
```

**SIMD Extensions (optional optimization)**:
- When AVX/AVX2/AVX-512 available: Process multiple 64-bit ternary values in parallel
- Automatic runtime detection: Falls back to scalar operations if unavailable
- Benefit: 2-8× throughput increase with vector extensions
- Core functionality: Never *requires* SIMD, only accelerates when present

### 4. Compact Hologram Encoding Format

**Storage Format**:
```
HologramEncoding {
    dimensions: usize,        // Vector dimensionality (e.g., 10000)
    density: f32,            // Sparsity ratio (e.g., 0.01 for 1%)
    ternary_blocks: Vec<i64>, // Array of 64-bit balanced ternary values
    metadata: PackageMetadata,
}
```

For a 10,000-dimensional vector with 1% density (100 positive + 100 negative = 200 non-zero elements):
- Traditional storage: ~200 × 8 bytes (indices) = 1,600 bytes
- Balanced ternary: ~200/39 = 5.1 blocks × 8 bytes ≈ 41 bytes
- Compression ratio: ~39× improvement

**Two-Way Encoding Properties**:
1. **Forward**: Vector indices → balanced ternary blocks → hash-like representation
2. **Reverse**: Hash-like representation → balanced ternary blocks → vector indices
3. **Preservation**: No information loss; perfect reconstruction guaranteed
4. **Algebraic**: Operations can be performed on encoded form

### 5. Package Factoralization Algorithm

**Factorialize Package**:
```
Given: holographic_container (engram with N packages)
       target_package (package to isolate)

Steps:
1. Identify target package chunks in manifest
2. Bundle all non-target chunks: complementary_hologram
3. Compute isolated hologram: target_hologram = root ⊖ complementary
4. Encode both holograms in balanced ternary format
5. Store as factorialized pair:
   - target_hologram: The isolated package
   - complementary_hologram: Everything else
6. Reconstruct when needed:
   - Full reconstruction: target ⊕ complementary = original
   - Partial reconstruction: Extract just target or just complementary
```

**Use Cases**:
1. **Package Updates**: Replace target_hologram without touching complementary
2. **Differential Distribution**: Send only updated package hologram
3. **Selective Extraction**: Extract single package without full container reconstruction
4. **Package Removal**: Use complementary_hologram as new base
5. **A/B Testing**: Multiple target_holograms with same complementary base

## Consequences

### Positive

- **Efficient Isolation**: Algebraically extract packages without full reconstruction
  - O(1) algebraic operations vs O(N) full extraction
  - Memory usage: ~40 bytes per package hologram vs MB for full extraction

- **Hardware Optimized**: Leverages 64-bit registers optimally
  - No SIMD required for basic functionality
  - 40 trits per register maximizes information density
  - Scalar operations sufficient for real-time performance

- **Compact Storage**: Balanced ternary encoding dramatically reduces storage
  - 40× compression for sparse vectors
  - Hash-like representation for transfer
  - Full reconstruction capability preserved

- **Algebraic Flexibility**: Enables operations on encoded packages
  - Bundle multiple package holograms
  - Compute package differences algebraically
  - Version management through hologram composition

- **SIMD Ready**: Optional acceleration when hardware supports it
  - 2-8× speedup with AVX/AVX2/AVX-512
  - Graceful fallback to scalar operations
  - No code duplication (runtime detection)

### Negative

- **Implementation Complexity**: Balanced ternary math adds complexity
  - Requires careful integer arithmetic
  - Overflow handling for 40-trit values
  - Testing across different architectures

- **Metadata Management**: Must track package boundaries
  - Manifest must identify package chunk ranges
  - Complementary hologram computation requires full package list
  - Metadata consistency critical for correctness

- **Precision Considerations**: Algebraic operations accumulate noise
  - Each bundle/unbundle operation adds small error
  - Cosine similarity threshold must account for factoralization depth
  - Deep factoralization may require higher dimensions

- **Learning Curve**: Balanced ternary unfamiliar to most developers
  - Documentation must explain trit/tryte concepts clearly
  - Examples essential for adoption
  - Tooling needed to visualize ternary representations

### Neutral

- **Platform Specific Optimizations**: Different performance characteristics
  - 64-bit registers: baseline performance
  - AVX2 (256-bit): 4× parallelism
  - AVX-512 (512-bit): 8× parallelism
  - Trade-off: Complexity vs performance

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)
- [ ] Implement BalancedTernary64 type with trit encoding/decoding
- [ ] Add balanced ternary arithmetic operations (+, -, ×)
- [ ] Create comprehensive unit tests for ternary operations
- [ ] Benchmark 64-bit register operations

### Phase 2: Vector Encoding (Weeks 3-4)
- [ ] Implement sparse vector → balanced ternary conversion
- [ ] Implement balanced ternary → sparse vector reconstruction
- [ ] Add two-way encoding validation tests
- [ ] Measure compression ratios

### Phase 3: Package Isolation (Weeks 5-6)
- [ ] Extend manifest format to track package boundaries
- [ ] Implement complementary hologram computation
- [ ] Add package isolation API
- [ ] Create factoralization CLI commands

### Phase 4: Optimization (Weeks 7-8)
- [ ] Add SIMD detection and dispatch
- [ ] Implement AVX2 parallel ternary operations
- [ ] Optimize hot paths identified by profiling
- [ ] Add performance benchmarks

### Phase 5: Integration (Weeks 9-10)
- [ ] Integrate with holographic OS builder
- [ ] Add package update workflows
- [ ] Create end-to-end tests
- [ ] Documentation and examples

## Use Cases

### 1. Selective Package Updates
```bash
# Isolate a package from holographic OS
embeddenator factorialize \
  --engram debian-12.engram \
  --package vim \
  --output vim-isolated.hologram \
  --complementary debian-12-no-vim.hologram

# Update just the vim package
embeddenator bundle \
  --base debian-12-no-vim.hologram \
  --package vim-new.hologram \
  --output debian-12-updated.engram
```

### 2. Package Removal
```bash
# Remove a package by using complementary hologram
embeddenator factorialize \
  --engram system.engram \
  --package bloatware \
  --output bloatware.hologram \
  --complementary system-clean.hologram

# system-clean.hologram now represents system without bloatware
```

### 3. A/B Testing Multiple Package Versions
```bash
# Create base system without test package
embeddenator factorialize \
  --engram base-system.engram \
  --package nginx \
  --complementary base-no-nginx.hologram

# Test version A
embeddenator bundle \
  --base base-no-nginx.hologram \
  --package nginx-v1.20.hologram \
  --output system-nginx-v1.20.engram

# Test version B
embeddenator bundle \
  --base base-no-nginx.hologram \
  --package nginx-v1.21.hologram \
  --output system-nginx-v1.21.engram
```

### 4. Differential Distribution
```bash
# Distribute only updated packages as compact holograms
embeddenator factorialize \
  --engram old-system.engram \
  --package updated-kernel \
  --output kernel-update.hologram

# Ship only kernel-update.hologram (~41 bytes vs 100MB)
# Receivers bundle with their existing complementary hologram
```

## References

- ADR-001: Sparse Ternary VSA (foundation for this work)
- ADR-004: Holographic OS Container Design (use case)
- [Balanced Ternary](https://en.wikipedia.org/wiki/Balanced_ternary)
- [Ternary Computing](https://homepage.divms.uiowa.edu/~jones/ternary/)
- Vector Symbolic Architectures (Kanerva, 2009)

## Notes

### Optimal Trit Count for 64-bit Registers

The choice between 39 and 40 trits depends on whether signed or unsigned integers are used:

**For Unsigned 64-bit** (40 trits):
```
3^40 = 12,157,665,459,056,928,801
2^64 = 18,446,744,073,709,551,616 (max unsigned 64-bit)

3^40 ≈ 2^63.4

This means:
- 40 trits use 63.4 bits of information
- Fits in 64-bit unsigned integer
- 41 trits would overflow (3^41 > 2^64)
- Range: 0 to 3^40-1
```

**For Signed 64-bit with Balanced Ternary** (39 trits):
```
3^39 = 4,052,555,153,018,976,267
2^63 = 9,223,372,036,854,775,808 (max signed 64-bit)

3^39 ≈ 2^61.8

This means:
- 39 trits use 61.8 bits of information
- Fits comfortably in signed 64-bit integer
- Balanced ternary range: -(3^39-1)/2 to +(3^39-1)/2
- 40 trits would overflow signed (3^40 > 2^63)
- Only wastes ~1.2 bits vs theoretical maximum
```

**Recommendation**: Use 39 trits for balanced ternary (signed) representation, which aligns naturally with the {-1, 0, +1} sparse vector format.
```

### Balanced vs Unbalanced Ternary

**Balanced ternary** {-1, 0, +1} has advantages over unbalanced {0, 1, 2}:

1. **Symmetric**: Equal positive and negative representation
2. **Sign-magnitude**: Natural representation for signed values
3. **Efficient operations**: Addition/subtraction without bias correction
4. **VSA Compatible**: Matches our {-1, 0, +1} sparse vector representation

### Future Extensions

- **Hierarchical Factoralization**: Multi-level package isolation
- **Lazy Evaluation**: Defer hologram reconstruction until needed
- **Quantum-Ready**: Ternary representation compatible with qutrit computing
- **Neural Integration**: Direct embedding in ternary neural networks
