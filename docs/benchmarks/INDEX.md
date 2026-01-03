# Benchmarks Index

This repo contains both **Criterion micro/meso benches** and **reproducible, report-producing benchmark harnesses** designed to compare Embeddenator against conventional approaches (compression tools + vector DB).

## What’s Implemented (this branch)

### Substrate encode/extract + effective ratio report
- Runner: `src/bin/bench_encode.rs`
- Runs: ingest for one or more input paths and reports:
  - ingest time
  - artifact size breakdown (root/codebook/corrections/manifest)
  - `effective_ratio_including_corrections = raw_bytes / (root+codebook+corrections+manifest)`
  - optional extract+SHA256 verification

### Traditional compression baselines (tar stream)
- Script: `scripts/bench/compress_baselines.sh`
- Baselines (auto-detected if on PATH): `gzip`, `zstd`, `lz4`, `brotli`
- Measures size + wall/user/sys time on a deterministic tar stream of the dataset directory.

### Environment capture
- Script: `scripts/bench/capture_env.sh`
- Captures: kernel, CPU model, RAM, lsblk/findmnt JSON, tool versions.

### Report generation (JSON + Markdown)
- Script: `scripts/bench/merge_reports.py`
- Outputs:
  - `merged.json` (all results + env)
  - `REPORT.md` (human summary)

### Run-all entrypoint
- Script: `scripts/bench/run_suite.sh`
- Produces a timestamped report directory under `reports/<UTC_TIMESTAMP>/`.

### Vector search: substrate vs Qdrant
- Substrate runner: `src/bin/bench_vector_substrate.rs`
  - Measures QPS/latency for codebook queries using the inverted index + rerank path
  - Computes Recall@k against brute-force cosine on the same codebook
- Qdrant runner: `scripts/bench/vdb/qdrant_minibench.py`
  - Starts Qdrant in Docker, ingests synthetic vectors, measures QPS/latency
  - Computes Recall@k against brute-force cosine over the same synthetic vectors
- Entry point: `scripts/bench/run_vector_suite.sh`

## How To Run

### Quick local report (small dataset)
- Generate dataset (already in repo as a script):
  - `./scripts/fetch_benchmark_data.sh --small`
- Run suite:
  - `./scripts/bench/run_suite.sh benchmark_data quick`

### With extract+SHA256 verification
- `./scripts/bench/run_suite.sh benchmark_data verify`

### With substrate engram compression (zstd)
- `./scripts/bench/run_suite.sh benchmark_data zstd`

### Vector suite (substrate vs Dockerized Qdrant)
- `./scripts/bench/run_vector_suite.sh benchmark_data`

## Notes / Known Gaps
- Filesystem benchmarks (`fio`, `iozone`, `fs_mark`) are not wired in yet; tool availability and mountpoint setup must be supplied.
- Big-ANN / VDBBench / ann-benchmarks integration is not wired in yet; the current Qdrant harness is a reproducible small-scale proxy.
- In-memory serialization/Arrow/YCSB baselines are not implemented in this branch yet.
