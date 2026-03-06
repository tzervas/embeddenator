# Embeddenator Benchmark Results

**Date:** 2026-01-29T01:23:33-05:00
**Host:** akula-prime
**CPU:** Intel(R) Core(TM) i7-14700K
**RAM:** 46Gi

## Summary Table

| Dataset | Size | Ingest (s) | Ingest MB/s | Engram | Overhead% | Extract (s) | Extract MB/s | Accuracy |
|---------|------|------------|-------------|--------|-----------|-------------|--------------|----------|
=== Benchmarking: structured ===

Input size: 52M (53957153 bytes)
### Ingestion
- Time: 4.610373721s
- Throughput: 11.15 MB/s
- Engram size: 135M (260.73% of input)

### Extraction
- Time: 1.574832722s
- Throughput: 32.67 MB/s

### Integrity Verification
- Original files: 5
- Extracted files: 5
- Sample accuracy: 100.00% (5/5 files)

---

| === Benchmarking: structured === |  |  |  |  |  |  |  | % |
=== Benchmarking: unstructured ===

Input size: 4.2M (4379931 bytes)
### Ingestion
- Time: .261415072s
- Throughput: 15.95 MB/s
- Engram size: 14M (319.92% of input)

### Extraction
- Time: .081316735s
- Throughput: 51.28 MB/s

### Integrity Verification
- Original files: 4
- Extracted files: 4
- Sample accuracy: 100.00% (4/4 files)

---

| === Benchmarking: unstructured === |  |  |  |  |  |  |  | % |
=== Benchmarking: binary ===

Input size: 1.3M (1303391 bytes)
### Ingestion
- Time: .094063080s
- Throughput: 13.18 MB/s
- Engram size: 3.3M (264.05% of input)

### Extraction
- Time: .044197102s
- Throughput: 28.05 MB/s

### Integrity Verification
- Original files: 6
- Extracted files: 6
- Sample accuracy: 100.00% (6/6 files)

---

| === Benchmarking: binary === |  |  |  |  |  |  |  | % |
=== Benchmarking: combined ===

Input size: 265M (277638097 bytes)
### Ingestion
- Time: 19.289747364s
- Throughput: 13.72 MB/s
- Engram size: 760M (287.00% of input)

### Extraction
- Time: 6.693537314s
- Throughput: 39.55 MB/s

### Integrity Verification
- Original files: 93
- Extracted files: 45
- Sample accuracy: 100.00% (10/10 files)

---

| **combined** |  |  |  |  |  |  |  | % |

## Detailed Logs

Logs saved in: `/home/kang/Documents/projects/embdntr/test-datasets/benchmarks/`
