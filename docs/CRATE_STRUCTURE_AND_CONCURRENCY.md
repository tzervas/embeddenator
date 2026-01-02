# Crate structure + concurrency strategy (future inference + balanced ternary compute)

This document proposes a workspace layout and concurrency strategy that keeps “sister” subcrates isolated so they can be extracted later with minimal dependency entanglement.

Goals
- Keep core math/representation crates dependency-light and extraction-ready.
- Keep IO/CLI/FUSE concerns out of core compute.
- Make CPU-bound compute scale via data parallelism without forcing an async runtime.
- Preserve today’s public API surface via a facade crate to reduce churn.

Non-goals
- This does not redesign algorithms; it only restructures code boundaries and runtime patterns.
- This does not require adopting nightly or pervasive `unsafe`.

---

## (1) Recommended workspace / crates layout

Proposed top-level layout:

```
embeddenator/
  Cargo.toml                # workspace + facade crate (optional)
  crates/
    embeddenator-ternary/
    embeddenator-vsa/
    embeddenator-codebook/
    embeddenator-correction/
    embeddenator-embrfs/
    embeddenator-fuse/      # optional, Linux/macOS only
    embeddenator-inference/ # future
    embeddenator-runtime/   # optional: shared concurrency config
  src/                      # facade crate (keeps current `embeddenator` API stable)
  tests/
  benches/
  docs/
```

Members and dependency direction (acyclic):

- `embeddenator-ternary` (leaf)
- `embeddenator-vsa` (leaf-ish; may depend on `embeddenator-ternary` if you unify “trit” types)
- `embeddenator-codebook` → (`ternary`, `vsa`)
- `embeddenator-correction` → (`ternary`, `vsa`, `codebook`)
- `embeddenator-embrfs` → (`ternary`, `vsa`, `codebook`, `correction`)
- `embeddenator-fuse` → (`embrfs`) + `fuser`/`libc` (platform-specific)
- `embeddenator-inference` → (`vsa`, `codebook`, maybe `ternary`) + (optional async/network deps later)
- `embeddenator-runtime` → (no functional deps; only config/traits/utilities)
- Facade crate `embeddenator` → re-exports from the above, plus current `src/cli.rs` and the `embeddenator` bin

This structure keeps extractable “math crates” independent of filesystem/CLI and keeps platform-specific `fuse` isolated.

---

## (2) What belongs in each crate

### `crates/embeddenator-ternary`
Purpose: balanced ternary primitives and fast, deterministic arithmetic.

Belongs here
- `Trit`, `Tryte*`, `Word*` types, conversion, formatting.
- Saturating / wrapping ternary arithmetic rules.
- Bit/tryte packing, compact encodings (if any).
- SIMD-optional kernels (feature-gated) for ternary ops.

Does not belong here
- VSA vector operations, hashing, random generation.
- File formats, IO, serde formats (unless the type’s canonical encoding is intrinsic).

Public API guidance
- Prefer small, `#[repr(transparent)]` newtypes around integers when possible.
- Make invariants explicit (e.g., `Tryte` always in -13..=13). Enforce on construction.

### `crates/embeddenator-vsa`
Purpose: VSA representations (sparse ternary vectors) + algebraic operations.

Belongs here
- `SparseVec`, `HyperVec` (if it’s a core representation) and operations: bundle/bind/permutation/similarity.
- Deterministic PRNG strategy for vector generation (seeded / reproducible).
- “Kernel” layer traits if you want multiple backends later (dense, sparse, GPU).

Does not belong here
- Codebook semantics, manifests, correction stores.
- Filesystem chunking policy (except generic block encode/decode primitives).

API guidance
- Keep pure functions where feasible: `fn bundle(a, b) -> c` style or `impl` methods returning new values.
- If you need in-place ops for performance, expose them as explicit `*_in_place` methods.

### `crates/embeddenator-codebook`
Purpose: mapping symbols/chunks/words to vectors and metadata.

Belongs here
- `Codebook`, `BalancedTernaryWord`, `ProjectionResult`, `WordMetadata`, scoring utilities.
- Projection/search utilities that don’t require filesystem concepts.

Does not belong here
- On-disk manifest layout of a filesystem.
- Correction layer persistence.

### `crates/embeddenator-correction`
Purpose: correctness and reconstruction guarantees.

Belongs here
- `CorrectionStore`, `ReconstructionVerifier`, `ChunkCorrection` and algorithms that restore 100% fidelity.
- Statistics + validation utilities.

Does not belong here
- CLI rendering.
- FUSE plumbing.

### `crates/embeddenator-embrfs`
Purpose: “holographic filesystem” domain model and ingest/export logic.

Belongs here
- `EmbrFS`, `Engram`, `Manifest`, `FileEntry`, chunking policy.
- Directory ingestion (sync by default; async optionally via features).
- Serialization/deserialization glue for engrams/manifests.

Does not belong here
- Platform-specific `fuser` integration.

### `crates/embeddenator-fuse` (feature / platform gated)
Purpose: isolate all OS/FFI boundary code.

Belongs here
- `EngramFS` and `fuser`-related types.
- Any `unsafe` required for FUSE/FFI should be concentrated here.

### `crates/embeddenator-inference` (future)
Purpose: inference/retrieval logic that consumes VSA + codebook to answer queries.

Belongs here
- Query model: “retrieve top-k”, “explain match”, “compose hypothesis”, etc.
- Index structures (sharded similarity search, caches).
- Pipeline orchestration (batch queries, streaming results) without binding to CLI.

Design constraint
- Avoid taking a dependency on filesystem/EmbrFS unless truly necessary; prefer passing in traits like “vector store” and “document source”.

### `crates/embeddenator-runtime` (optional)
Purpose: shared concurrency configuration and execution traits.

Belongs here
- A small abstraction layer so core crates don’t hard-depend on `rayon`/`tokio`.
- Example: `trait Executor { fn scope(...); fn parallel_for(...); }`

Pragmatic note
- If you want maximum simplicity, skip this crate and let `embrfs`/`inference` choose concurrency directly.

### Facade crate `embeddenator` (root)
Purpose: keep the existing public API and binary stable while you refactor.

Belongs here
- `pub use ...` re-exports to preserve `use embeddenator::SparseVec` etc.
- CLI entrypoint and current `src/cli.rs`.

---

## (3) Concurrency primitives and patterns

### Rule of thumb: CPU compute != async runtime
- Balanced ternary kernels and VSA operations are CPU-bound → prefer **Rayon** or explicit threads.
- File IO (ingest directories, read/write engrams) can be sync or async, but should not force an async dependency on core compute crates.

### Recommended: Rayon for data-parallel compute
Use Rayon where work is “embarrassingly parallel”:
- Encoding blocks/chunks in parallel (per-file or per-block).
- Bundling many vectors via map-reduce.
- Similarity scoring across a candidate set.

Patterns
- Map-reduce: `par_iter().map(f).reduce(|| id, combine)`
- Chunked parallelism: avoid tiny tasks; operate on blocks (e.g., 64–1024 items per job).
- Deterministic results: ensure your reduction is associative/commutative or define a deterministic reduction order (e.g., stable chunk boundaries).

Thread pool strategy
- Build a dedicated pool for heavy workloads (especially in a library context) rather than relying on the global pool, so embedding applications can control threads.
- Expose a “configure threads” entrypoint at the `embrfs`/`inference` boundary.

### Inference-specific execution model (recommended)
For “future inference logic” (retrieval + scoring), the most reliable scaling pattern is *read-mostly sharded state + per-request parallel scoring*.

State layout
- Store codebooks/indices as immutable snapshots: `Arc<IndexState>`.
- For hot reload / incremental updates, swap the snapshot atomically (copy-on-write): readers never lock; writers build a new snapshot then publish.
  - If you want a crate for this later, consider `arc-swap` (optional dependency) at the `embeddenator-inference` boundary.

Query parallelism
- **Across queries**: bound concurrency with a work queue (Crossbeam bounded channel) or a Tokio semaphore (service mode).
- **Within a query**: use Rayon for scoring candidates in parallel.

Top-k pattern (deterministic + low contention)
- Each worker computes a thread-local top-k (small binary heap or fixed-size array).
- Reduce thread-local top-k sets into a single top-k at the end (single-thread or tree-reduction).
- Avoid pushing into a shared heap from multiple threads.

Sharding pattern
- Partition candidates into shards (by id range or hash) so each worker touches mostly local memory.
- Keep shard data contiguous (arrays/slices) where possible to improve cache locality.

### Crossbeam for pipelines and scoped threads
Use Crossbeam where you need explicit backpressure or a producer/consumer pipeline:
- Ingest pipeline: filesystem walker → reader → encoder → bundler.
- Streaming inference: query stream → candidate generation → scoring → top-k aggregation.

Primitives
- `crossbeam::channel` for bounded queues (backpressure).
- `crossbeam::scope` for spawning threads borrowing stack references (avoids `Arc` cloning when safe).

### Tokio only when you truly need async
Tokio makes sense if inference becomes network-facing or you need high-concurrency IO:
- Serving requests (HTTP/gRPC) in a future service.
- Async file IO on platforms where it helps (often limited on Linux; still can help when multiplexing many operations).

Recommended boundary
- Keep Tokio confined to `embeddenator-inference` (service mode) or CLI.
- Run CPU-heavy work via `tokio::task::spawn_blocking` and then inside Rayon/threads.

### When to use plain threads
Plain `std::thread` is fine for:
- A small, fixed number of long-lived worker threads.
- Pinning specific responsibilities (e.g., one writer thread).

If you go this route, prefer channels + owned messages; avoid sharing mutable state.

---

## (4) Memory safety considerations (and performance-friendly safety)

### Minimize shared mutable state
- Prefer immutable data + functional transforms.
- For shared read-mostly structures (codebooks, indices): store as `Arc<T>` and update by swapping an `Arc` (copy-on-write model).

### Use lock types intentionally
- `RwLock` (or `parking_lot::RwLock` if you choose) for read-heavy maps.
- `Mutex` for infrequent mutation.
- Atomics for counters/metrics only.

Avoid
- Fine-grained locking inside tight inner loops (similarity scoring, ternary kernels).

### Ensure thread-safe RNG
Vector generation often uses randomness:
- Don’t share a single RNG across threads.
- Use per-thread RNG seeded deterministically from a master seed + thread index (or hash of task id) to get reproducibility.

### Avoid aliasing in parallel reductions
For parallel bundling/scoring:
- Compute into thread-local accumulators, then merge.
- Do not write into a shared `Vec` from multiple threads without chunk partitioning.

### Keep `unsafe` isolated and justified
- If you introduce `unsafe` for SIMD or unchecked indexing, keep it behind a small module boundary (ideally inside `embeddenator-ternary` or `embeddenator-vsa` kernel modules) with extensive tests.
- Keep all FFI `unsafe` inside `embeddenator-fuse`.

### Data representation invariants
- For sparse vectors: maintain sorted/unique `pos`/`neg` indices; enforce on construction or normalize after ops.
- Consider smaller index types (`u32`) if `DIM` fits, to reduce memory bandwidth.

### Avoid accidental quadratic behavior in hot paths
- Prefer set-like operations using two-pointer merge on sorted `pos`/`neg` lists over `HashSet` in inner loops.
- If an operation needs a scratch buffer, prefer reusing it per-thread (thread-local) rather than allocating per vector.

### Prefer “owned-message passing” across threads
- When crossing thread boundaries, prefer `Arc<[T]>`, `Arc<Vec<T>>`, or `Vec<T>` moved through a channel.
- Avoid sharing `&mut` across threads; let each worker own its chunk of output.

---

## (5) Migration steps (incremental, low-risk)

1) Convert to a workspace without breaking users
- Update root `Cargo.toml` to include `[workspace]` and add members under `crates/`.
- Keep the existing `embeddenator` crate as a facade (same name) to preserve public API and binary entrypoints.

2) Extract leaf crates first
- Move `src/ternary.rs` → `crates/embeddenator-ternary/src/lib.rs`.
- Move `src/vsa.rs` → `crates/embeddenator-vsa/src/lib.rs`.
- Add minimal `Cargo.toml`s with only needed deps.

3) Re-export through the facade
- In the root crate, replace `pub mod ternary; pub mod vsa;` with `pub use embeddenator_ternary::*; pub use embeddenator_vsa::*;` (or re-export specific items to preserve names).

4) Extract dependent crates in order
- `codebook` → `correction` → `embrfs` → `fuse`.
- Keep tests compiling by pointing them at the facade crate until the dust settles.

5) Introduce feature flags for concurrency
- Add `parallel` feature (enables Rayon) in `vsa`/`inference` (or only at higher layers).
- Add `async` feature (Tokio) only in `inference`/CLI if needed.

6) Add a single concurrency “entrypoint”
- Provide one place to configure thread pools and batch sizing (ideally `embrfs` and `inference`).
- Avoid having core crates create global pools implicitly.

7) Validate correctness + performance
- Keep existing tests; add targeted tests for parallel determinism (same input → same output).
- Add Criterion benches at the crate boundary (e.g., bundle N vectors, score M candidates).

---

## Suggested defaults (practical)
- Use Rayon for CPU-heavy: bundling, scoring, encoding blocks.
- Use Crossbeam channels for multi-stage ingest/inference pipelines.
- Keep Tokio out of core unless you’re building a service.
- Keep unsafe minimal and boxed into kernel/FFI crates.
