# Ternary Representation Quick Reference

**Date**: 2026-01-14  
**Version**: 0.20.0-alpha.1

---

## Overview

embeddenator-vsa uses two complementary representations for ternary vectors:

1. **SparseVec** - For sparse vectors (< 25% density)
2. **PackedTritVec** - For dense vectors (≥ 25% density)

---

## Comparison Table

| Feature | SparseVec | PackedTritVec |
|---------|-----------|---------------|
| **Storage** | Index lists | Bitsliced words |
| **Encoding** | pos/neg indices | 2 bits per trit |
| **Memory** | 8 bytes per non-zero | 2 bits per dimension |
| **Best for** | < 25% density | ≥ 25% density |
| **Operations** | Merge algorithms | Word-level bitwise |
| **SIMD** | Limited | Excellent |
| **GPU** | Difficult | Natural |
| **Random access** | O(log k) | O(1) |
| **Dot product** | O(k log k) | O(n/32) |

Where:
- k = number of non-zero trits
- n = total dimension

---

## SparseVec: Sparse Representation

### Structure

```rust
pub struct SparseVec {
    pub pos: Vec<usize>,  // Indices where trit = +1
    pub neg: Vec<usize>,  // Indices where trit = -1
}
```

### Encoding

- Store only non-zero indices
- Sorted lists for efficient merge operations
- Zero trits are implicit (not stored)

### Example

Vector: `[0, +1, -1, 0, +1, 0, 0, -1]`

```rust
SparseVec {
    pos: vec![1, 4],      // Positions 1 and 4 are +1
    neg: vec![2, 7],      // Positions 2 and 7 are -1
}
```

### Memory Usage

- **Header**: ~48 bytes (3 × Vec overhead)
- **Data**: 8 bytes × (number of non-zeros)
- **Example**: 10,000 dimensions, 10% density = 48 + 8×1000 = ~8 KB

### Use Cases

 Random data encoding (typically < 5% density)  
 Feature vectors with few active dimensions  
 Incremental vector construction  
 Memory-constrained environments  

 Dense bundled vectors (> 25% density)  
 SIMD/GPU-accelerated operations  

---

## PackedTritVec: Bitsliced Dense Representation

### Structure

```rust
pub struct PackedTritVec {
    len: usize,
    data: Vec<u64>,  // 32 trits per word
}
```

### Encoding: Bitsliced 2-bits-per-trit

Each trit uses 2 bits:
- `00` = Z (zero, 0)
- `01` = P (positive, +1)
- `10` = N (negative, -1)
- `11` = unused (reserved, treated as Z)

### Bitplane Layout

```
Each u64 word holds 32 trits:
[t31_hi|t31_lo|...|t1_hi|t1_lo|t0_hi|t0_lo]

Even bits (mask 0x5555...): P bitplane
Odd bits  (shifted right 1): N bitplane
```

### Example

Vector: `[Z, P, N, Z, P, Z, Z, N]`

```
Binary encoding:
  Trit:  Z    P    N    Z    P    Z    Z    N
  Bits: 00 | 01 | 10 | 00 | 01 | 00 | 00 | 10
  
Stored as u64 (least significant bits first):
  0b...00_00_00_01_00_10_01_00 = 0x...0124

Bitplanes:
  P plane: 0x...0010 (bits 1, 4)
  N plane: 0x...0082 (bits 2, 7)
```

### Memory Usage

- **Header**: ~32 bytes (usize + Vec overhead)
- **Data**: 2 bits × dimension / 8 = dimension / 4 bytes
- **Example**: 10,000 dimensions = 32 + 10000/4 = ~2.5 KB

### Use Cases

 Dense vectors (> 25% density)  
 Bundle operations (accumulation)  
 Batch processing  
 SIMD/GPU acceleration  
 Throughput-critical paths  

 Very sparse data (< 10% density)  
 Random sparse access patterns  

---

## Operations Comparison

### Dot Product (Cosine Similarity)

#### SparseVec Approach

```rust
// Sorted merge + intersection counting
let pp = intersection_count(&a.pos, &b.pos);  // O(k log k)
let nn = intersection_count(&a.neg, &b.neg);
let pn = intersection_count(&a.pos, &b.neg);
let np = intersection_count(&a.neg, &b.pos);

dot = (pp + nn) - (pn + np);
```

**Complexity**: O(k log k) where k = non-zeros  
**Performance**: Excellent for sparse (< 25%)

#### PackedTritVec Approach

```rust
// Word-level bitwise operations + POPCNT
for word in 0..words {
    let a_pos = a.data[word] & MASK_EVEN;
    let a_neg = (a.data[word] >> 1) & MASK_EVEN;
    let b_pos = b.data[word] & MASK_EVEN;
    let b_neg = (b.data[word] >> 1) & MASK_EVEN;
    
    let pp = (a_pos & b_pos).count_ones();  // Hardware POPCNT
    let nn = (a_neg & b_neg).count_ones();
    let pn = (a_pos & b_neg).count_ones();
    let np = (a_neg & b_pos).count_ones();
    
    dot += (pp + nn) - (pn + np);
}
```

**Complexity**: O(n/32) where n = dimension  
**Performance**: Excellent for dense (≥ 25%)  
**SIMD**: Auto-vectorizes, 32 trits per operation

---

### Bind (Element-wise Multiplication)

#### SparseVec Approach

```rust
// Combine sorted lists
let pos = merge_with_rule(a.pos, a.neg, b.pos, b.neg);
let neg = merge_with_rule(a.pos, a.neg, b.pos, b.neg);
```

**Complexity**: O(k) where k = non-zeros

#### PackedTritVec Approach

```rust
// Word-level bitwise operations
for word in 0..words {
    let a_pos = a.data[word] & MASK_EVEN;
    let a_neg = (a.data[word] >> 1) & MASK_EVEN;
    let b_pos = b.data[word] & MASK_EVEN;
    let b_neg = (b.data[word] >> 1) & MASK_EVEN;
    
    // Same sign → P, opposite → N
    let same = (a_pos & b_pos) | (a_neg & b_neg);
    let opp = (a_pos & b_neg) | (a_neg & b_pos);
    
    out.data[word] = same | (opp << 1);
}
```

**Complexity**: O(n/32) where n = dimension  
**SIMD**: Pure bitwise, no branches

---

### Bundle (Element-wise Addition)

#### SparseVec Approach

```rust
// Complex merge with thinning
let combined = bundle_sum_many(&[vec1, vec2, ...]);
if density > threshold {
    combined.thin(target_count);  // Maintain sparsity
}
```

**Complexity**: O(k log k) + thinning overhead

#### PackedTritVec Approach

```rust
// Word-level saturating addition
for word in 0..words {
    let a_pos = a.data[word] & MASK_EVEN;
    let a_neg = (a.data[word] >> 1) & MASK_EVEN;
    let b_pos = b.data[word] & MASK_EVEN;
    let b_neg = (b.data[word] >> 1) & MASK_EVEN;
    
    // Saturating ternary addition via bitwise logic
    let pos = (a_pos & !b_neg) | (b_pos & !a_neg);
    let neg = (a_neg & !b_pos) | (b_neg & !a_pos);
    
    out.data[word] = pos | (neg << 1);
}
```

**Complexity**: O(n/32) where n = dimension  
**SIMD**: Branchless, vectorizes perfectly

---

## When to Use Which

### Decision Tree

```
Is vector density < 25%?
├─ YES: Use SparseVec
│   ├─ Random access needed? → SparseVec
│   ├─ Incremental construction? → SparseVec
│   └─ Memory constrained? → SparseVec
│
└─ NO: Use PackedTritVec
    ├─ Bulk operations? → PackedTritVec
    ├─ SIMD acceleration? → PackedTritVec
    └─ GPU acceleration? → PackedTritVec
```

### Conversion Guidelines

**SparseVec → PackedTritVec**:
- After bundling many vectors (density increases)
- Before batch processing
- When enabling SIMD/GPU

**PackedTritVec → SparseVec**:
- After strong thinning (density decreases)
- When memory is critical
- For sparse output storage

---

## Performance Tips

### For SparseVec

 Keep indices sorted (enables O(k log k) operations)  
 Use `thin()` after bundling to maintain sparsity  
 Batch operations when possible  
 Consider converting to PackedTritVec for repeated operations  

 Don't use for dense vectors (> 25%)  
 Don't modify without re-sorting  

### For PackedTritVec

 Use for batch operations on multiple vectors  
 Leverage auto-vectorization (compiler does it)  
 Process large dimensions (> 10K) for best speedup  
 Use `bind_into()` / `bundle_into()` to reuse allocations  

 Don't use for very sparse data (< 10%)  
 Don't create for one-time operations  

---

## Future Enhancements

### Explicit SIMD (Phase 2)

**PackedTritVec** will get hand-written SIMD:
- AVX2: Process 128 trits per instruction (4 × u64)
- AVX-512: Process 256 trits per instruction (8 × u64)
- NEON: Process 64 trits per instruction (2 × u64)

Expected speedup: 2-4x over current auto-vectorization

### GPU Acceleration (Phase 3)

**PackedTritVec** will get GPU kernels:
- CUDA for NVIDIA GPUs
- OpenCL for cross-platform
- Process 1024+ trits per GPU warp

Expected speedup: 10-100x for large vectors (> 1M dimensions)

**SparseVec** likely stays CPU-only:
- Sparse operations don't parallelize well on GPU
- Irregular memory access patterns
- Better suited for CPU cache

---

## Code Examples

### Creating Vectors

```rust
use embeddenator_vsa::{SparseVec, PackedTritVec};

// Sparse: from indices
let sparse = SparseVec {
    pos: vec![1, 4, 7],
    neg: vec![2, 5],
};

// Packed: from sparse
let packed = PackedTritVec::from_sparsevec(&sparse, 10);

// Sparse: from encoding
let config = ReversibleVSAConfig::default();
let sparse2 = SparseVec::encode_data(b"hello", &config, None);
```

### Converting Between Representations

```rust
// Sparse → Packed
let packed = PackedTritVec::from_sparsevec(&sparse, dimension);

// Packed → Sparse
let sparse = packed.to_sparsevec();
```

### Operations

```rust
// Dot product (both types)
let similarity_sparse = sparse1.cosine(&sparse2);
let similarity_packed = packed1.dot(&packed2);

// Bind (both types)
let bound_sparse = sparse1.bind(&sparse2);
let bound_packed = packed1.bind(&packed2);

// Bundle (both types)
let bundled_sparse = sparse1.bundle(&sparse2);
let bundled_packed = packed1.bundle(&packed2);
```

---

## References

- **Bitsliced Design**: `docs/BITSLICED_TERNARY_DESIGN.md`
- **SIMD Strategy**: `docs/SIMD_DESIGN.md`
- **Implementation Plan**: `IMPLEMENTATION_PLAN.md`
- **Gap Analysis**: `GAP_ANALYSIS.md`

---

**Document Version**: 1.0  
**Last Updated**: 2026-01-14  
**Maintained By**: embeddenator-vsa team
