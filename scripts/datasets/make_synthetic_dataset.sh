#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-}"
SIZE_GB="${2:-1}"
MODE="${3:-random}"

if [[ -z "$OUT_DIR" ]]; then
  echo "usage: $0 /path/to/out_dir [size_gb=1] [mode=random|text]" >&2
  exit 2
fi

mkdir -p "$OUT_DIR"

# Create a multi-GB file with deterministic(ish) content based on mode.
# - random: uses /dev/urandom (fast, non-compressible)
# - text: repeats a fixed paragraph (compressible)

bytes=$((SIZE_GB * 1024 * 1024 * 1024))

case "$MODE" in
  random)
    dd if=/dev/urandom of="$OUT_DIR/blob_${SIZE_GB}gb_random.bin" bs=8M count=$((bytes / (8*1024*1024))) status=progress
    ;;
  text)
    python3 - <<PY
from pathlib import Path
out = Path(${OUT_DIR@Q}) / f"blob_{int(${SIZE_GB@Q})}gb_text.txt"
chunk = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
         "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n").encode()
want = int(${bytes@Q})
with out.open('wb') as f:
    written = 0
    while written < want:
        n = min(len(chunk), want - written)
        f.write(chunk[:n])
        written += n
print(out)
PY
    ;;
  *)
    echo "unknown mode: $MODE" >&2
    exit 2
    ;;
esac

echo "Created synthetic dataset in $OUT_DIR" >&2
