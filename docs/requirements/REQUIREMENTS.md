# Embeddenator Requirements Specification

**Version**: 1.0
**Date**: 2026-01-26
**Status**: Active

## 1. Problem Statement

### 1.1 Background

Traditional computing separates storage (disk), memory (RAM), and compute (CPU/GPU) into distinct layers with incompatible representations. Data must be converted between formats at each layer boundary, creating:

- **I/O bottlenecks**: Serialization/deserialization overhead
- **Memory waste**: Duplicate representations in different formats
- **Lost semantics**: Raw bytes lose meaning during storage
- **No content addressing**: Must know exact location to retrieve data

### 1.2 Problem Definition

We need a **unified data representation** that:
1. Works across storage, memory, and compute layers
2. Preserves semantic meaning (similarity, relationships)
3. Enables content-addressable retrieval
4. Achieves competitive storage efficiency
5. Supports computation directly in the representation

### 1.3 Scope

**In Scope**:
- Holographic filesystem (engram-based storage)
- System memory model (VSA-based RAM)
- Compute in latent space (engram arithmetic)
- GPU/SIMD acceleration
- Integration with Rust AI ecosystem

**Out of Scope** (for v1.0):
- Hardware implementation (FPGA/ASIC)
- Kernel-level integration
- Distributed storage

## 2. User Stories

### 2.1 Storage User Stories

| ID | As a... | I want to... | So that... | Priority |
|----|---------|--------------|------------|----------|
| US-S01 | Developer | Store files as engrams | I can search by content similarity | Must |
| US-S02 | Developer | Retrieve files with >94% accuracy without corrections | Storage is truly holographic | Must |
| US-S03 | Developer | Have <10% storage overhead | System is practical for real use | Must |
| US-S04 | Developer | Mount engrams as FUSE filesystem | I can use standard file tools | Should |
| US-S05 | Developer | Modify files in mounted engrams | System supports read/write workflows | Should |
| US-S06 | Researcher | Search across all stored files by similarity | I can find related content | Must |
| US-S07 | Researcher | Bundle multiple files into one engram | I can create holographic archives | Should |

### 2.2 Memory User Stories

| ID | As a... | I want to... | So that... | Priority |
|----|---------|--------------|------------|----------|
| US-M01 | Systems Developer | Use VSA engrams as RAM representation | Memory is content-addressable | Must |
| US-M02 | Systems Developer | Access memory with <1μs latency | System is practical for runtime use | Must |
| US-M03 | Systems Developer | Have working set fit in L2 cache (~50KB) | CPU access is fast | Should |
| US-M04 | ML Engineer | Compute directly on engrams without decode | I avoid encode/decode overhead | Should |

### 2.3 Compute User Stories

| ID | As a... | I want to... | So that... | Priority |
|----|---------|--------------|------------|----------|
| US-C01 | ML Engineer | Perform arithmetic on engrams | I can compute in latent space | Should |
| US-C02 | ML Engineer | Use GPU for batch VSA operations | Training is fast | Must |
| US-C03 | Developer | Use SIMD for CPU VSA operations | Single-core performance is good | Must |
| US-C04 | Researcher | Define custom VSA operations | I can extend the algebra | Could |

### 2.4 Integration User Stories

| ID | As a... | I want to... | So that... | Priority |
|----|---------|--------------|------------|----------|
| US-I01 | Developer | Use rust-ai-core for device management | GPU/CPU selection is automatic | Must |
| US-I02 | Developer | Use trit-vsa for SIMD operations | I get optimized primitives | Must |
| US-I03 | ML Engineer | Use DeterministicPhaseTrainer for codebook | Training is reproducible and fast | Should |
| US-I04 | Developer | Have clean Rust API with documentation | Integration is straightforward | Must |

## 3. Functional Requirements

### 3.1 Core VSA Operations

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-V01 | Implement bind operation (⊙) for ternary vectors | Must | Done |
| FR-V02 | Implement bundle operation (⊕) for superposition | Must | Done |
| FR-V03 | Implement cosine similarity for retrieval | Must | Done |
| FR-V04 | Support dimensions from 64 to 100,000 | Must | Done |
| FR-V05 | Provide deterministic seeding for reproducibility | Must | Done |
| FR-V06 | Implement block-sparse representation | Must | Planned |

### 3.2 Encoding/Decoding

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-E01 | Position-aware byte encoding: `bind(pos_vec, byte_vec)` | Must | Done |
| FR-E02 | Chunked encoding for large files | Must | Done |
| FR-E03 | Achieve >94% reconstruction accuracy | Must | 92% (Gap) |
| FR-E04 | Support correction layer for 100% accuracy | Must | Done |
| FR-E05 | Superposition of chunks into root engram | Must | Done |

### 3.3 Filesystem

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-F01 | Ingest directories into engram files | Must | Done |
| FR-F02 | FUSE mount with read support | Must | Done |
| FR-F03 | FUSE mount with write support | Must | Done |
| FR-F04 | Hierarchical manifest for O(log n) lookup | Should | Done |
| FR-F05 | Version control with atomic operations | Should | Done |
| FR-F06 | Signal handlers for graceful unmount | Must | Planned |

### 3.4 Performance

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-P01 | Encode throughput >100 MB/s with GPU | Must | 0.06 MB/s (Gap) |
| FR-P02 | Decode throughput >200 MB/s with GPU | Must | 0.02 MB/s (Gap) |
| FR-P03 | AVX-512 SIMD for CPU operations | Should | Planned |
| FR-P04 | CUDA kernels for GPU batch operations | Must | Partial |
| FR-P05 | Memory-mapped engram files for large datasets | Should | Done |

### 3.5 Training

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-T01 | Codebook training from representative data | Should | Partial |
| FR-T02 | DeterministicPhaseTrainer integration | Should | Planned |
| FR-T03 | N-gram frequency analysis for basis vectors | Should | Planned |
| FR-T04 | GPU-accelerated training | Should | Planned |

## 4. Non-Functional Requirements

### 4.1 Performance

| ID | Requirement | Target | Current |
|----|-------------|--------|---------|
| NFR-P01 | Encode latency per MB | <10ms | ~17,000ms |
| NFR-P02 | Decode latency per MB | <5ms | ~50,000ms |
| NFR-P03 | Memory access latency | <1μs | N/A |
| NFR-P04 | Storage overhead | <10% | 5.3% (Met!) |

### 4.2 Scalability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-S01 | Single engram size | Up to 1TB |
| NFR-S02 | Files per engram | Up to 10M |
| NFR-S03 | Concurrent operations | Thread-safe |

### 4.3 Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-R01 | Data integrity | 100% with corrections |
| NFR-R02 | Crash recovery | WAL journaling |
| NFR-R03 | Graceful degradation | Fallback to CPU |

### 4.4 Usability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-U01 | API documentation | 100% coverage |
| NFR-U02 | Examples | All major use cases |
| NFR-U03 | Error messages | Actionable |

## 5. Constraints

### 5.1 Technical Constraints

| ID | Constraint | Rationale |
|----|------------|-----------|
| TC-01 | Rust language only | Performance, safety |
| TC-02 | Must support CPU-only fallback | Not all systems have GPU |
| TC-03 | Ternary values {-1, 0, +1} | VSA theory requirement |
| TC-04 | Deterministic operations | Reproducibility |

### 5.2 Resource Constraints

| ID | Constraint | Limit |
|----|------------|-------|
| RC-01 | Working set memory | <100MB |
| RC-02 | GPU memory per batch | <8GB |
| RC-03 | Disk I/O | Standard SSD speeds |

### 5.3 Integration Constraints

| ID | Constraint | Rationale |
|----|------------|-----------|
| IC-01 | Compatible with rust-ai-core | Unified AI infrastructure |
| IC-02 | Use trit-vsa for VSA primitives | Optimized implementation |
| IC-03 | Support candle tensor backend | ML interoperability |

## 6. Acceptance Criteria

### 6.1 Storage Acceptance

| Criterion | Metric | Target | Validation |
|-----------|--------|--------|------------|
| AC-S01 | Reconstruction accuracy | >94% uncorrected | Benchmark suite |
| AC-S02 | Storage overhead | <10% for >100KB | Benchmark suite |
| AC-S03 | FUSE mount stability | No crashes in 24h | Soak test |
| AC-S04 | Write consistency | All writes durable | Crash recovery test |

### 6.2 Performance Acceptance

| Criterion | Metric | Target | Validation |
|-----------|--------|--------|------------|
| AC-P01 | GPU encode throughput | >100 MB/s | Benchmark on RTX 5080 |
| AC-P02 | GPU decode throughput | >200 MB/s | Benchmark on RTX 5080 |
| AC-P03 | CPU encode throughput | >10 MB/s with AVX-512 | Benchmark on 14700K |
| AC-P04 | Similarity search | <100ms for 1M vectors | Benchmark |

### 6.3 Integration Acceptance

| Criterion | Metric | Target | Validation |
|-----------|--------|--------|------------|
| AC-I01 | rust-ai-core compatibility | All APIs work | Integration tests |
| AC-I02 | trit-vsa compatibility | Seamless conversion | Unit tests |
| AC-I03 | CLI usability | All commands documented | Manual review |

## 7. Deliverables

### 7.1 Software Deliverables

| ID | Deliverable | Description | Status |
|----|-------------|-------------|--------|
| D-01 | embeddenator-vsa | Core VSA library | Released |
| D-02 | embeddenator-fs | Filesystem implementation | Released |
| D-03 | embeddenator-cli | Command-line interface | Released |
| D-04 | ReversibleVSAEncoder | Position-aware encoding | Done |
| D-05 | Block-sparse module | Memory-efficient storage | Planned |
| D-06 | GPU kernels | CUDA acceleration | Partial |

### 7.2 Documentation Deliverables

| ID | Deliverable | Description | Status |
|----|-------------|-------------|--------|
| D-D01 | API documentation | rustdoc for all crates | Done |
| D-D02 | Architecture docs | ADRs and design docs | In Progress |
| D-D03 | User guide | Getting started, CLI usage | Partial |
| D-D04 | Benchmark reports | Performance validation | In Progress |

### 7.3 Validation Deliverables

| ID | Deliverable | Description | Status |
|----|-------------|-------------|--------|
| D-V01 | Unit test suite | >80% coverage | Done |
| D-V02 | Integration tests | End-to-end workflows | Partial |
| D-V03 | Benchmark suite | Performance measurement | In Progress |
| D-V04 | Accuracy test suite | Reconstruction validation | Done |

## 8. Glossary

| Term | Definition |
|------|------------|
| Engram | A VSA vector representing stored data holographically |
| VSA | Vector Symbolic Architecture - high-dimensional computing framework |
| Bind | VSA operation creating composite representation |
| Bundle | VSA operation for superposition/overlay |
| Codebook | Set of basis vectors for encoding |
| Holographic | Data distributed across all dimensions (not localized) |
| Superposition | Multiple items encoded in one vector via bundling |
| Trit | Ternary digit {-1, 0, +1} |
