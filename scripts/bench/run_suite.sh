#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-benchmark_data}"
PROFILE="${2:-quick}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$ROOT_DIR/reports/$TS"

mkdir -p "$OUT_DIR"

if [[ ! -d "$ROOT_DIR/$DATA_DIR" && ! -d "$DATA_DIR" ]]; then
  echo "dataset dir not found: $DATA_DIR (relative to repo root or absolute)" >&2
  exit 2
fi

if [[ -d "$ROOT_DIR/$DATA_DIR" ]]; then
  DATA_DIR="$ROOT_DIR/$DATA_DIR"
fi

# Choose settings.
ENGRAM_CODEC="none"
VERIFY="false"
FEATURES=""

case "$PROFILE" in
  quick)
    ENGRAM_CODEC="none"
    VERIFY="false"
    FEATURES=""
    ;;
  verify)
    ENGRAM_CODEC="none"
    VERIFY="true"
    FEATURES=""
    ;;
  zstd)
    ENGRAM_CODEC="zstd"
    VERIFY="false"
    FEATURES="compression"
    ;;
  zstd-verify)
    ENGRAM_CODEC="zstd"
    VERIFY="true"
    FEATURES="compression"
    ;;
  *)
    echo "unknown profile: $PROFILE (quick|verify|zstd|zstd-verify)" >&2
    exit 2
    ;;
esac

echo "[suite] dataset=$DATA_DIR profile=$PROFILE out=$OUT_DIR" >&2

chmod +x "$ROOT_DIR/scripts/bench/capture_env.sh" || true
chmod +x "$ROOT_DIR/scripts/bench/compress_baselines.sh" || true
chmod +x "$ROOT_DIR/scripts/bench/merge_reports.py" || true

"$ROOT_DIR/scripts/bench/capture_env.sh" "$OUT_DIR/env.json"

# Build + run embeddenator bench runner.
if [[ -n "$FEATURES" ]]; then
  cargo build --release --features "$FEATURES" --bin bench_encode
else
  cargo build --release --bin bench_encode
fi

EMBED_JSON="$OUT_DIR/embeddenator.json"
COMP_JSON="$OUT_DIR/compress.json"
MERGED_JSON="$OUT_DIR/merged.json"
REPORT_MD="$OUT_DIR/REPORT.md"

CMD=("$ROOT_DIR/target/release/bench_encode" --input "$DATA_DIR" --engram-codec "$ENGRAM_CODEC" --out "$EMBED_JSON")
if [[ "$VERIFY" == "true" ]]; then
  CMD+=(--verify)
fi

"${CMD[@]}"

"$ROOT_DIR/scripts/bench/compress_baselines.sh" "$DATA_DIR" "$COMP_JSON"

python3 "$ROOT_DIR/scripts/bench/merge_reports.py" --env "$OUT_DIR/env.json" --embed "$EMBED_JSON" --compress "$COMP_JSON" --out-json "$MERGED_JSON" --out-md "$REPORT_MD"

echo "[suite] done: $REPORT_MD" >&2
