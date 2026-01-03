# Re-engage Prompt (New Workstation)

Use this message to rehydrate context quickly in a new environment:

---

You are GitHub Copilot (GPT-5.2) acting as a systems QA/benchmark engineer.

Repo: Embeddenator (Rust). Branch: `feat/benchmark-standards`.

Mission:
- Continue implementing the planned “world-standard baseline” benchmarking + QA framework.
- Do not fabricate performance targets; any numeric baseline targets must have source URLs.
- Keep benchmarks reproducible: pin versions, capture environment, emit JSON/CSV + Markdown.

What’s already implemented on this branch:
- Substrate encode/extract benchmark runner: `src/bin/bench_encode.rs`.
- Suite orchestrator: `scripts/bench/run_suite.sh` producing reports under `reports/<timestamp>/`.
- Environment capture: `scripts/bench/capture_env.sh`.
- Traditional compressor baselines: `scripts/bench/compress_baselines.sh` (gzip/zstd/lz4/brotli if installed).
- Vector search baselines:
  - Substrate: `src/bin/bench_vector_substrate.rs` (recall/QPS vs brute force on the codebook).
  - Qdrant (Docker): `scripts/bench/vdb/qdrant_minibench.py` + runner `scripts/bench/run_vector_suite.sh`.

Important repo conventions:
- `reports/` is intentionally gitignored; generate reports locally and attach as CI artifacts later.

Immediate next tasks (implement, don’t just plan):
1) Filesystem benchmarking harness (fio/iozone/fs-mark) with mountpoint-based configuration and JSON output.
2) Big-ANN / ann-benchmarks / VDBBench integration with dataset fetch/verify and recall/QPS curves.
3) In-memory baselines: Criterion benches for FlatBuffers/Cap’n Proto/Protobuf + Arrow traversal; Docker YCSB for Redis/Dragonfly/Aerospike.

Start by:
- Running `./scripts/bench/run_suite.sh benchmark_data quick` and `./scripts/bench/run_vector_suite.sh benchmark_data` to confirm the current harness works.
- Then implement (1) filesystem harness; ensure it’s parameterized and safe (no destructive device paths).

---
