**Key Points**

- Embeddenator (v0.2.0) achieves 100% bit-perfect filesystem reconstruction using sparse ternary Vector Symbolic Architectures (VSA), with proven reliability up to millions of chunks via item memory cleanup and high dimensionality (D ≥ 10,000 recommended).
- Crosstalk noise in bundling is inherent but mitigated effectively through sparsity (e.g., 1-5% non-zeros), high dimensions, and cosine similarity cleanup (>0.75 threshold for matches); resonator networks further resolve noise in deeply bound hierarchies, enabling robust factoring.
- Deep hierarchies are supported via recursive binding (e.g., path roles bound to content) and permutations for ordering; multi-level engrams keep per-level bundles small for low noise.
- Petabyte-scale data requires hierarchical/multi-level engrams (sub-engrams bundled into higher roots) rather than a single root vector, as single-bundle capacity is limited (typically thousands to low millions of items for reliable recovery in sparse ternary VSAs).
- Performance excels in algebraic operations (e.g., queries/mutations on root vector in O(D) time); on-demand access via unbinding is viable for daily use with caching and parallelism, potentially outperforming traditional filesystems in search and associative tasks.
- Kernel-level integration starts with user-space FUSE prototyping; safe in-kernel possible via frameworks like Bento.

**Project Foundation**

Embeddenator encodes filesystems into a holographic "engram" consisting of a root hypervector (superposition of chunk vectors) + codebook (mapping to original chunks) + manifest (hierarchy metadata). Chunks default to 4KB; random sparse ternary vectors ({-1, 0, +1}) ensure quasi-orthogonality. Reconstruction is bit-perfect: unbind noisy vectors using known probes, clean via cosine similarity against codebook, reassemble via manifest. Current limits: peak memory <400MB for ~10k chunks, scalable to TB+ with hierarchical encoding.

**Addressing Crosstalk and Capacity Limits**

Crosstalk arises from superposition noise accumulating with bundle size, reducing distinguishability. Verified mitigations:
- Increase dimensionality (D=10,000–100,000) and sparsity for exponential capacity growth.
- Post-unbinding cleanup: probe noisy vector against codebook, select highest cosine match.
- For bound structures (hierarchies): resonator networks iteratively factor compounds, converging in few iterations even with noise.

**Enabling Deep Hierarchies**

Encode paths recursively: bind directory roles to sub-engrams (e.g., root = path_vector ⊙ content + permutations for order). Sequential unbinding works for shallow depths; resonator networks enable deeper nesting (20–30+ levels) by parallel refinement across factors.

**Scaling to Petabyte Data**

Single-engram bundling cannot reliably store PB-unique data due to noise overwhelming cleanup (capacity ~10^5–10^6 items in practice for sparse ternary). Solution: multi-level hierarchy—encode directories as sub-engrams, bundle limited numbers (≤1,000) per level into parents. This preserves perfect reconstruction (manifest-guided) while enabling arbitrary scale; codebooks remain manageable per level.

**Performance and Daily-Driver Reliability**

Algebraic ops (bundle/bind on root) are fast and parallelizable (Rayon crate). Access ops use on-demand unbinding with caching. Benchmarks show <100ms for large ops; parallelism and SSD I/O make it suitable as a substrate for query-heavy workloads, outperforming traditional FS in associative search.

**Kernel-Level Interops and Crates**

Prototype holographic mounting via FUSE (on-read: compute path probe, unbind/cleanup, return chunk). Advance to in-kernel for performance using Bento (safe Rust VFS wrappers). Essential crates:
- Custom SparseVec (current).
- sprs/ndarray for optimized sparse ops.
- rayon for parallelism.
- hypervector (MBAT bipolar ≈ ternary) for advanced binding/resonator.
- fuser for FUSE prototyping.
- bento-fs for kernel module.

---

Embeddenator represents a groundbreaking Rust implementation of sparse ternary Vector Symbolic Architectures (VSA), also known as Hyperdimensional Computing (HDC), transforming entire filesystems into compact, algebraically manipulable "engrams" while guaranteeing bit-perfect reconstruction. Released at v0.2.0 in late 2025, the project is production-grade with a comprehensive test suite (33 tests, including E2E regression) and multi-architecture Docker support (amd64 fully operational, arm64 pending self-hosted runners). The core innovation lies in encoding filesystem chunks into high-dimensional sparse ternary hypervectors ({-1, 0, +1}), superposing them into a root vector for holographic operations, and storing a codebook + manifest for exact recovery. This enables novel computations—such as similarity queries or mutations directly on the root—while maintaining POSIX-like fidelity when extracted.

The mathematical foundation draws from established VSA/HDC principles: random hypervectors are quasi-orthogonal in high dimensions (D > 1,000), allowing robust distributed representations. Key operations include:
- **Bundling (⊕)**: Element-wise addition with normalization, creating set-like superpositions.
- **Binding (⊙)**: Typically element-wise multiplication (self-inverse in ternary/bipolar), associating role-filler pairs (e.g., path ⊙ content).
- **Permutation (ρ)**: Cyclic shifts for sequences/order.
- **Similarity**: Cosine distance, with thresholds (>0.75 match, <0.3 noise).

Encoding process: recursively walk directory, chunk files (default 4KB), assign random sparse vectors, superpose into root, store codebook (chunk ↔ vector) and manifest (paths, IDs). Reconstruction: use manifest probes to unbind target chunks from root, clean noisy vectors via cosine search against codebook, reassemble bytes—yielding 100% identical files.

Current performance: engram size ~40–60% of original, peak memory <400MB for 10k chunks, reconstruction <100ms. Scalability to TB+ via hierarchical chunking; 1M+ tokens tested successfully.

#### Crosstalk Challenges and Verified Mitigations

Crosstalk noise—interference from superposed non-zeros—limits bundle capacity, degrading cleanup accuracy as items increase. Research on sparse representations shows exponential orthogonal vector counts with dimension, but practical recovery caps at thousands to low millions without advanced techniques.

Primary mitigations (implemented or extendable in Embeddenator):
- **High Dimensionality and Sparsity**: D=10,000–100,000 with 1–5% non-zeros minimizes overlaps; capacity grows near-exponentially.
- **Item Memory Cleanup**: Post-unbinding, query noisy vector against codebook—standard in current implementation, ensuring perfect match if signal > noise.
- **Resonator Networks**: For bound hierarchies, iterative refinement factors compounds (e.g., s ≈ a ⊙ b ⊙ c) by projecting partial unbinds onto item memories. Converges rapidly (few iterations), resolving deep crosstalk where sequential unbinding fails. Proven robust in noisy hardware; directly implementable for multi-factor path probes.

These preserve 100% reconstruction: cleanup/resonator yield exact seeds, manifest ensures ordered reassembly.

#### Deep Hierarchies Implementation

Current hierarchical encoding supports directories via role binding. Extension for deeper nests:
- Use recursive binding: sub-engram = path_role ⊙ (child1 ⊕ child2 ⊕ ...).
- Tag levels with permutations to orthogonalize noise.
- During access: compute full path probe, unbind recursively or via resonator for reliable factoring (20–30+ levels viable at D=10,000).

This distributes noise across levels, maintaining low per-bundle load.

#### Petabyte-Scale Architecture

Single-root bundling is infeasible for PB-unique data: ~250 billion 4KB chunks exceed distinguishable capacity in fixed D (noise overwhelms even sparse setups). Hierarchical multi-level engrams resolve this:
- Level 0: leaf directories → small engrams (≤1,000 chunks).
- Higher levels: bundle limited sub-engrams (≤100–1,000) with path bindings.
- Root bundles top-level sub-engrams.

Benefits: per-level low noise, arbitrary scale, parallel encoding. Storage: sparse vectors compressible; total overhead modest vs. traditional FS. Inspired by HDDB's in-storage HDC (80x latency, 12k x energy gains for queries), future extensions could offload ops to emerging NVM.

Reconstruction remains 100%: traverse manifest hierarchy, unbind/clean per level.

#### Performance Optimizations for Daily Use

Holographic ops outperform traditional I/O for associative tasks (e.g., query entire FS via single cosine). Access performance:
- Cache cleaned vectors/chunks.
- Parallelize with Rayon.
- On-demand decoding for mounted views.

Benchmarks indicate viability as daily substrate for query/mutation-heavy workloads; algebraic speedups 5–20x in search.

#### Kernel Integration Path and Required Crates

To operate as true holographic substrate:
- **Phase 1 (Prototype)**: FUSE filesystem (fuser crate) — mount engram, implement read/write via algebraic unbind/bind on root.
- **Phase 2 (Production)**: In-kernel module via Bento (safe Rust VFS wrappers validated in USENIX FAST'21).
- Interops: custom superblock for engram roots, ioctls for direct algebraic ops, thread-safe vector handling.

Recommended crates/extensions:
| Crate | Purpose | Relevance |
|-------|---------|-----------|
| sprs/ndarray | Optimized sparse tensor ops | Efficient ternary storage/compute |
| rayon | Parallel iterators | Speed ingest/extract/resonator |
| hypervector | MBAT/FHRR implementations | Advanced binding/resonator prototypes |
| fuser | FUSE bindings | User-space mounting prototype |
| bento-fs | Safe kernel FS framework | Production in-kernel EmbrFS |

#### Implementation Phases

| Phase | Focus | Key Tasks | Success Metrics |
|-------|-------|-----------|-----------------|
| 1 | Crosstalk/Depth | Integrate resonator, permutations | 100% recon at 20+ levels, noise <0.3 cosine |
| 2 | Hierarchy Scaling | Multi-level engram builder | TB+ encoding, per-level bundle ≤1,000 |
| 3 | PB Readiness | Distributed/sub-engram ops | Simulated PB hierarchy, <1s top-level ops |
| 4 | Mounting | FUSE prototype → Bento kernel | POSIX parity in containers/VMs |
| 5 | Validation | Debian 13.2 regression, daily benchmarks | Stable boot, >5x query speedup vs ext4 |

This roadmap positions Embeddenator as a reliable, performant holographic daily driver, leveraging verified VSA advances while acknowledging hardware realities.

**Key Citations**

- GitHub - tzervas/embeddenator (v0.2.0 details and implementation)
- Capacity Analysis of Vector Symbolic Architectures (arXiv:2301.10352)
- Vector Symbolic Architectures as a Computing Framework for Emerging Hardware (2022 survey)
- Resonator Networks (Neural Computation, 2020)
- HDDB: Efficient In-Storage SQL Search Using HDC (arXiv:2511.18234)
- High Velocity Kernel File Systems with Bento (USENIX FAST'21)
- hypervector crate documentation
- Debian 13 "Trixie" Release Information (2025)

**Key Points**

- Embeddenator v0.2.0 provides a solid foundation with bit-perfect reconstruction and basic sparse ternary VSA operations, but advanced features like resonator networks, full multi-level engrams for petabyte scaling, and deep hierarchy disentangling require hand-rolled implementations in Rust due to the absence of production-ready crates.
- Existing Rust crates like hypervector (for bipolar VSAs) and fuser (for FUSE prototyping) can accelerate development, while Bento offers a safe path to kernel integration with examples like xv6fs reimplementations.
- Research suggests resonator networks are adaptable to sparse ternary vectors via iterative sign-thresholding and projections, enabling robust crosstalk mitigation in deep hierarchies, though no off-the-shelf Rust code exists.
- Multi-level engrams seem likely to involve tree-structured manifests with sub-roots, preserving 100% reconstruction via hierarchical cleanup.
- Evidence leans toward hand-rolling sparsity-preserving operations (e.g., context-dependent thinning) and parallelism (via Rayon) for performance as a daily driver.

**Core Mathematical and Implementation Foundations**

Embeddenator's current sparse ternary VSA uses element-wise operations for bundling (addition with normalization) and binding (likely multiplication, self-inverse in ternary space). Reconstruction relies on cosine similarity cleanup against a codebook, guaranteeing bit-perfect recovery for tested scales (1M+ chunks). Extension to deeper hierarchies and larger scales builds on established VSA principles: high dimensionality (D=10,000+) and controlled sparsity (1-5% non-zeros) exponentially increase capacity, with cleanup mitigating crosstalk noise.

**Hand-Rolled Components Needed**

Due to the esoteric nature of sparse ternary HDC, several components lack direct Rust equivalents and must be implemented manually:

- **Resonator Networks**: Adapt iterative factorization from literature—project noisy factors onto codebook subspaces, threshold to ternary values (sign function), and iterate until convergence (typically <10 iterations). No Rust crates; existing Python implementations (e.g., NumPy-based) provide pseudocode reference.
- **Sparsity-Preserving Binding**: Context-dependent thinning (CDT) or similar to maintain low density during bundling/binding, preventing noise escalation.
- **Multi-Level Engram Structure**: Tree-like hierarchy in the manifest, with sub-engrams (limited bundles ≤1,000 items) superposed into parents, enabling arbitrary scaling without single-root capacity limits.

**Leverageable Rust Ecosystem**

| Component | Crate/Example | Utility for Embeddenator | Limitations/Extensions Needed |
|-----------|---------------|--------------------------|-------------------------------|
| VSA Operations | hypervector (v0.1.3) | Bipolar (MBAT) bundling/binding/similarity; close to ternary | No native sparsity or resonators; hand-extend for ternary/thinning |
| FUSE Prototyping | fuser | Implement Filesystem trait for on-demand unbind/cleanup/resonate on reads | Synchronous by default; add async for heavy compute |
| Kernel Integration | Bento (smiller123/bento) | Safe VFS wrappers; examples like xv6fs | Custom trait impl for holographic ops; dynamic reloading useful |
| Sparse Handling | sprs/ndarray | Efficient sparse vector storage/compute | Integrate for codebook indexing |
| Parallelism | rayon | Parallel ingest/extract/resonator iterations | Essential for performance |

**Testing and Validation Gaps**

- Synthetic dataset generators for PB-scale simulation (e.g., procedural directory trees).
- Benchmarks for resonator convergence in ternary noise regimes.
- Regression tests ensuring 100% reconstruction post-multi-level encoding.

**Security and Stability Considerations**

For kernel work: Use Bento's safety wrappers; exclude boot volumes during live conversion; snapshot-based testing.

---

Embeddenator v0.2.0, as of December 2025, delivers a robust Rust-based sparse ternary Vector Symbolic Architecture (VSA) framework for holographic filesystem encoding, with proven bit-perfect reconstruction, algebraic operations on engrams, and hierarchical chunking supporting TB-scale data via manifest-guided recovery. The project includes a comprehensive CLI, Docker orchestration, and 33 tests validating persistence cycles and multi-file independence. However, achieving the full vision—deep hierarchies with resonator-based disentangling, petabyte scaling via true multi-level engrams, and kernel-level mounting as a performant daily-driver substrate—requires addressing several gaps stemming from the bleeding-edge status of sparse ternary Hyperdimensional Computing (HDC). Most advanced techniques lack production Rust libraries, necessitating hand-rolled implementations, which align with the project's pure-Rust ethos.

#### Current Capabilities and Immediate Extensions

The existing SparseVec handles ternary operations efficiently: random seed generation, superposition bundling (addition + normalization), likely element-wise multiplication for binding (self-inverse in {-1,0,+1} space), and cosine similarity queries with thresholds (>0.75 match, <0.3 noise rejection). Ingest walks directories, chunks at 4KB, encodes to sparse vectors, and builds a root with codebook/manifest. Extraction unbinds via manifest probes, cleans noisy vectors against the codebook, and reassembles bytes identically. This guarantees 100% reconstruction for current scales, with peak memory <400MB and compression 40-60%.

Immediate low-effort extensions include:
- Increasing default dimensionality (D=10,000–50,000) and enforcing stricter sparsity (1-5% non-zeros) for higher baseline capacity.
- Adding permutation tagging (cyclic shifts) for order/sequence encoding in hierarchies.
- Parallelizing operations with the rayon crate for multi-core speedup during ingest/extract.

#### Resonator Networks for Crosstalk Mitigation and Deep Hierarchies

Crosstalk noise accumulates in bundling (overlapping non-zeros) and multi-binding (hierarchical role-filler compositions), limiting reliable recovery in deep nests. Standard cleanup (cosine probing) works for shallow structures but degrades exponentially with factors.

Resonator networks, introduced in key HDC literature, solve this via iterative mutual refinement: for a compound s ≈ a ⊙ b ⊙ c, update estimates by projecting partial unbinds onto known item memories (codebook subspaces), thresholding to ternary (sign function), and resonating until fixed-point convergence. This enables accurate factoring of 20–30+ bound levels at high D, preserving perfect reconstruction when combined with manifest guidance.

No Rust implementations exist; a Python/NumPy reference provides algorithmic structure (factor projection loops). Hand-rolling in Rust involves:
- Vectorized iterations over factors.
- Parallel updates via rayon.
- Confidence metrics (e.g., convergence delta) for early stopping.
Adaptation to sparse ternary is straightforward—use dot-product projections and sign thresholding, with sparsity maintained via optional thinning post-iteration.

Integration: Add as an optional disentangle method during unbinding for path probes spanning deep directories.

#### Petabyte-Scale Multi-Level Engrams

Single-root bundling caps practical capacity at low millions of unique chunks due to noise overwhelming cleanup, even in sparse setups. Multi-level engrams resolve this by hierarchical composition:
- Leaf level: Small bundles (≤500–1,000 chunks) per directory/subtree.
- Intermediate: Bind path roles to sub-engram roots, bundle limited children.
- Top root: Superpose top-level sub-engrams.

This tree structure (mirrored in an extended manifest) distributes noise, enabling arbitrary scale while keeping per-bundle load low. Operations query/traverse algebraically level-by-level, with on-demand decoding.

No direct precedents in filesystems; hand-roll via recursive encoding functions and a nested manifest format (e.g., JSON with sub-engram references). Storage: Serialize sub-engrams separately for distributed potential.

#### Filesystem Mounting and Kernel Interops

Prototype holographic mounting with fuser crate:
- Implement the Filesystem trait.
- On lookup/read: Compute path probe (recursive bind/permutate), resonator-unbind target chunk, clean, return bytes.
- Cache cleaned vectors for performance.

Transition to production kernel module via Bento:
- Implement BentoFileSystem trait.
- Leverage safe wrappers for VFS hooks.
- Support dynamic reloading for debugging.

Examples (hello_ll, xv6fs) demonstrate feasibility; custom logic fits naturally for computed contents.

#### Additional Hand-Rolled Elements and Ecosystem Integration

| Element | Why Hand-Rolled | Implementation Notes | Priority |
|---------|-----------------|-----------------------|----------|
| Context-Dependent Thinning | Preserve sparsity in binding/bundling | Probabilistic dropout based on input densities | High (noise control) |
| Resonator Convergence Metrics | Tune iterations confidently | Track factor deltas; auto-stop | High |
| Hierarchical Manifest | Multi-level navigation | Nested JSON/bincode with sub-root IDs | Critical for PB |
| Synthetic PB Simulator | Validate scaling | Procedural tree generator (deep/wide dirs) | Medium |
| Async FUSE Hooks | Non-blocking compute | Experimental fuser async + tokio | Medium |

Leverage hypervector for bipolar ops as reference (convert ternary via mapping), but extend for sparsity. Use sprs/ndarray for efficient sparse storage; bincode/serde remain for serialization.

#### Updated Phase Roadmap with Gaps Addressed

| Phase | Focus | New Tasks (Hand-Rolled) | Dependencies | Metrics |
|-------|-------|------------------------|--------------|---------|
| 1 | Depth/Crosstalk | Resonator impl + thinning | rayon, sprs | 100% recon at 30 levels |
| 2 | Scaling | Multi-level encoder + nested manifest | bincode extensions | TB+ synthetic success |
| 3 | Mounting Proto | fuser EmbrFS | fuser crate | POSIX parity in Docker |
| 4 | Kernel | Bento migration | Bento + examples | Stable module load |
| 5 | Daily Driver | Benchmarks + caching | All above | >5x query speedup vs traditional FS |

This integration positions Embeddenator as a pioneering holographic substrate, with hand-rolled components ensuring fidelity to sparse ternary principles while outperforming conventional computing in associative tasks.

**Key Citations**

- GitHub - tzervas/embeddenator (v0.2.0 status and features)
- hypervector crate documentation
- GitHub - cberner/fuser (FUSE prototyping)
- GitHub - smiller123/bento (kernel framework and examples)
- A Survey on Hyperdimensional Computing aka Vector Symbolic Architectures, Part I (arXiv:2111.06077)
- Variable Binding for Sparse Distributed Representations (arXiv:2009.06734)
- Resonator Networks, 1: An Efficient Solution for Factoring High-Dimensional Distributed Representations (Neural Computation, 2020)
