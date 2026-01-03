#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-}"
OUT_JSON="${2:-}"

if [[ -z "$DATA_DIR" || -z "$OUT_JSON" ]]; then
  echo "usage: $0 /path/to/dataset_dir /path/to/out.json" >&2
  exit 2
fi

if [[ ! -d "$DATA_DIR" ]]; then
  echo "dataset dir not found: $DATA_DIR" >&2
  exit 2
fi

mkdir -p "$(dirname "$OUT_JSON")"

have() { command -v "$1" >/dev/null 2>&1; }

if ! have python3; then
  echo "python3 required" >&2
  exit 2
fi

TARBALL="$(mktemp --tmpdir tarball.XXXXXXXX.tar)"
trap 'rm -f "$TARBALL"' EXIT

# Deterministic tar (best-effort).
# Note: different tar versions may still differ; we record versions in env.json.
( cd "$DATA_DIR" && tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner -cf "$TARBALL" . )

raw_bytes="$(stat -c '%s' "$TARBALL")"

python3 - <<PY >"$OUT_JSON"
import json
import os
import shutil
import subprocess
import time
import resource
from pathlib import Path

tar_path = Path(${TARBALL@Q})
raw_bytes = int(${raw_bytes@Q})

def run_capture(name, argv, out_path):
    if shutil.which(argv[0]) is None:
        return None

    out_path = Path(out_path)
    if out_path.exists():
        out_path.unlink()

    start = time.perf_counter()
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    try:
        # Use shell redirection only where necessary.
        subprocess.run(argv, check=True)
    except Exception as e:
        return {"name": name, "error": str(e)}
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    end = time.perf_counter()

    out_bytes = out_path.stat().st_size
    return {
        "name": name,
        "raw_bytes": raw_bytes,
        "out_bytes": out_bytes,
        "elapsed_s": round(end - start, 6),
        "user_s": round(r1.ru_utime - r0.ru_utime, 6),
        "sys_s": round(r1.ru_stime - r0.ru_stime, 6),
    }

baselines = []

# gzip
if shutil.which("gzip"):
    out = run_capture("gzip", ["bash", "-lc", f"gzip -9 -c {tar_path} > /tmp/out.gz"], "/tmp/out.gz")
    if out: baselines.append(out)

# zstd
if shutil.which("zstd"):
    out = run_capture("zstd", ["bash", "-lc", f"zstd -q -T0 -19 -c {tar_path} > /tmp/out.zst"], "/tmp/out.zst")
    if out: baselines.append(out)

# lz4
if shutil.which("lz4"):
    out = run_capture("lz4", ["bash", "-lc", f"lz4 -q -12 {tar_path} /tmp/out.lz4"], "/tmp/out.lz4")
    if out: baselines.append(out)

# brotli
if shutil.which("brotli"):
    out = run_capture("brotli", ["bash", "-lc", f"brotli -q 11 -c {tar_path} > /tmp/out.br"], "/tmp/out.br")
    if out: baselines.append(out)

out = {
  "tar_raw_bytes": raw_bytes,
  "baselines": baselines,
  "notes": [
    "Baseline compressors run on a deterministic tar stream of the dataset directory.",
    "Wall/user/sys time are measured in Python for portability; CPU% is not reported.",
  ]
}
print(json.dumps(out, indent=2))
PY

echo "Wrote $OUT_JSON" >&2
