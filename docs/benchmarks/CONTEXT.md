# Benchmarking + QA Context (Work Snapshot)

## Goal
Provide reproducible, apples-to-apples evidence for Embeddenator’s tradeoffs vs:
- conventional compression tools (gzip/zstd/lz4/brotli)
- conventional vector DB/search (Qdrant baseline in Docker)

…and lay the groundwork for expanding to:
- filesystem benchmarks (fio/iozone/fs-mark) across ext4/xfs/btrfs/zfs
- Big-ANN / VDBBench / ann-benchmarks
- in-memory serialization + Arrow + YCSB

## What This Branch Adds

### New binaries
- `bench_encode`: produces JSON report for ingest sizes/timing + effective ratio.
- `bench_vector_substrate`: measures recall/QPS/latency for the substrate retrieval path.

### New scripts
- `scripts/bench/run_suite.sh`: orchestrates env capture + substrate run + compressor baselines + merged report.
- `scripts/bench/capture_env.sh`: writes `env.json`.
- `scripts/bench/compress_baselines.sh`: writes `compress.json`.
- `scripts/bench/merge_reports.py`: writes `merged.json` + `REPORT.md`.
- `scripts/bench/run_vector_suite.sh`: runs substrate retrieval bench + Qdrant microbench.
- `scripts/bench/vdb/qdrant_minibench.py`: dockerized Qdrant harness.
- `scripts/datasets/make_synthetic_dataset.sh`: optional helper for multi-GB synthetic files.

## Reproducibility Decisions
- Reports are written to `reports/<UTC_TIMESTAMP>/` and ignored by git.
- Baseline compression uses a deterministic tar stream (best-effort) so size/speed comparisons are stable.
- Vector DB baseline uses Docker and captures container image id.
- Environment capture stores kernel/CPU/RAM + lsblk/findmnt + tool versions.

## Generated Artifacts
Each run produces:
- `env.json`
- `embeddenator.json`
- `compress.json`
- `merged.json`
- `REPORT.md`

## Interpreting Current Metrics
- `effective_ratio_including_corrections` uses uncompressed serialization sizes for root/codebook/corrections + manifest JSON.
  - This intentionally accounts for “metadata-driven access overhead” rather than only on-disk compressed size.
- Compression baselines are computed on the tar stream to approximate traditional archival compression.
- Vector recall is computed vs brute-force cosine to avoid claiming external benchmark parity.

## Next Implementation Targets
1) Filesystem suite:
- Add `scripts/bench/fs/run_fio.sh` etc. parameterized by mountpoints + block sizes + queue depths.
- Emit JSON with MB/s, IOPS, p99 latency.

2) Big-ANN / ann-benchmarks integration:
- Add dataset fetch/verify scripts (SHA256) + runner that emits recall/QPS curves.

3) In-memory format/store baselines:
- Add Criterion benches for protobuf/flatbuffers/capnp + Arrow traversal.
- Add dockerized YCSB runners for Redis/Dragonfly/Aerospike.
