#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-benchmark_data}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT_DIR="$(dirname "$ROOT_DIR")"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$PROJECT_DIR/reports/$TS"
mkdir -p "$OUT_DIR"

if [[ -d "$PROJECT_DIR/$DATA_DIR" ]]; then
  DATA_DIR="$PROJECT_DIR/$DATA_DIR"
fi

echo "[vector] dataset=$DATA_DIR out=$OUT_DIR" >&2

chmod +x "$PROJECT_DIR/scripts/bench/vdb/qdrant_minibench.py" || true

# Substrate vector bench
cargo build --release --bin bench_vector_substrate
"$PROJECT_DIR/target/release/bench_vector_substrate" --input "$DATA_DIR" --out "$OUT_DIR/substrate_vector.json"

# Qdrant microbench (synthetic)
python3 "$PROJECT_DIR/scripts/bench/vdb/qdrant_minibench.py" --out "$OUT_DIR/qdrant_vector.json" --vectors 20000 --dim 128 --queries 200 --k 10

echo "[vector] wrote: $OUT_DIR/substrate_vector.json and qdrant_vector.json" >&2
