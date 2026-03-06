# Embeddenator Baseline Benchmark Report

**Date:** 2026-01-29
**Host:** akula-prime
**CPU:** Intel Core i7-14700K (AVX2/AVX-512)
**RAM:** 46GB DDR5
**Storage:** NVMe SSD

---

## Executive Summary

Baseline benchmarks for embeddenator holographic storage system using mixed public datasets.

### Key Metrics

| Metric | Structured | Unstructured | Binary | Combined |
|--------|------------|--------------|--------|----------|
| Input Size | 52 MB | 4.2 MB | 1.3 MB | 265 MB |
| **Ingest Throughput** | 11.15 MB/s | 15.95 MB/s | 13.18 MB/s | **13.72 MB/s** |
| **Extract Throughput** | 32.67 MB/s | 51.28 MB/s | 28.05 MB/s | **39.55 MB/s** |
| Storage Overhead | 260% | 320% | 264% | 287% |
| **Accuracy** | 100% | 100% | 100% | **100%** |

---

## Dataset Description

### Structured (52 MB)
- UCI Adult Census (3.8 MB CSV)
- UCI Iris (4.5 KB CSV)
- UCI Wine (11 KB CSV)
- GitHub API JSON (5.8 KB)
- NYC Taxi 2024-01 (48 MB Parquet)

### Unstructured (4.2 MB)
- Pride and Prejudice (735 KB text)
- Moby Dick (1.2 MB text)
- Apache logs (2.3 MB synthetic)
- Enron email sample (CSV)

### Binary (1.3 MB)
- Sample JPEG images (3x, ~200 KB total)
- RFC 791 IP specification (93 KB text)
- Big Buck Bunny video clip (1 MB MP4)

---

## Analysis

### Throughput

| Operation | Achieved | v0.25 Target | v1.0 Target | Status |
|-----------|----------|--------------|-------------|--------|
| Encode | 13.72 MB/s | >10 MB/s | >100 MB/s | **EXCEEDS v0.25** |
| Decode | 39.55 MB/s | >20 MB/s | >200 MB/s | **EXCEEDS v0.25** |

**Notes:**
- CPU-only (no GPU acceleration active)
- AVX2 SIMD optimizations enabled
- Rayon parallel processing active

### Storage Efficiency

| Metric | Current | v0.25 Target | v1.0 Target |
|--------|---------|--------------|-------------|
| Overhead | 287% | <150% | <106% |

**Notes:**
- High overhead due to verbatim correction layer (not using ReversibleVSAEncoder path)
- Expected to improve significantly when holographic encoding is fully activated
- Current implementation prioritizes 100% accuracy over compression

### Accuracy

**100% bit-perfect reconstruction** across all data types:
- CSV structured data
- JSON documents
- Plain text (novels, logs)
- Binary images (JPEG)
- Video files (MP4)
- Parquet columnar format

---

## Performance by Data Type

### Structured Data (Best for: Tabular, JSON)
```
Ingest: 11.15 MB/s | Extract: 32.67 MB/s | Overhead: 260%
```
- Highly compressible columnar data
- Good pattern recognition in repetitive structures

### Unstructured Text (Best for: Documents, Logs)
```
Ingest: 15.95 MB/s | Extract: 51.28 MB/s | Overhead: 320%
```
- Highest throughput category
- Natural language has predictable patterns

### Binary Data (Images, Video)
```
Ingest: 13.18 MB/s | Extract: 28.05 MB/s | Overhead: 264%
```
- Compressed media (JPEG, MP4) harder to encode
- Still achieves 100% accuracy

---

## Bottleneck Analysis

1. **Storage Overhead (287%)** - Primary gap
   - Root cause: Verbatim correction layer storing full data
   - Solution: Wire ReversibleVSAEncoder for holographic encoding
   - Expected improvement: 287% → <150%

2. **Encode Throughput (13.72 MB/s)** - Meets v0.25 target
   - Current: CPU parallel encoding with Rayon
   - Future: GPU batch encoding for >100 MB/s

3. **Decode Throughput (39.55 MB/s)** - Exceeds v0.25 target
   - Good performance from SIMD optimizations
   - Room for GPU acceleration

---

## Recommendations

### Immediate Actions
1. Wire ReversibleVSAEncoder to reduce storage overhead
2. Enable GPU acceleration for larger datasets
3. Add streaming I/O for TB-scale testing

### Future Testing
1. Scale to 1GB+ datasets
2. Multi-GPU benchmarking (RTX 5080)
3. Concurrent access stress testing
4. Long-running soak tests (24-48 hours)

---

## Raw Logs

See detailed logs in:
- `structured_ingest.log`
- `structured_extract.log`
- `unstructured_ingest.log`
- `unstructured_extract.log`
- `binary_ingest.log`
- `binary_extract.log`
- `combined_ingest.log`
- `combined_extract.log`

---

*Benchmark suite: embeddenator v0.21.0 (CLI), vsa v0.23.0*
