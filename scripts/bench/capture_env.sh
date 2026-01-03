#!/usr/bin/env bash
set -euo pipefail

OUT_JSON="${1:-}"
if [[ -z "$OUT_JSON" ]]; then
  echo "usage: $0 /path/to/env.json" >&2
  exit 2
fi

mkdir -p "$(dirname "$OUT_JSON")"

have() { command -v "$1" >/dev/null 2>&1; }

kernel="$(uname -a || true)"

cpu_model=""
if have lscpu; then
  cpu_model="$(lscpu | awk -F: '/Model name/ {sub(/^ +/,"",$2); print $2; exit}' || true)"
fi

mem_total_bytes=""
if have free; then
  mem_total_bytes="$(free -b | awk '/Mem:/ {print $2}' || true)"
fi

LSBLK_TMP="$(mktemp --tmpdir env_lsblk.XXXXXXXX.json)"
FINDMNT_TMP="$(mktemp --tmpdir env_findmnt.XXXXXXXX.json)"
TOOLS_TMP="$(mktemp --tmpdir env_tools.XXXXXXXX.json)"
trap 'rm -f "$LSBLK_TMP" "$FINDMNT_TMP" "$TOOLS_TMP"' EXIT

if have lsblk; then
  lsblk -o NAME,MODEL,SIZE,ROTA,TYPE,MOUNTPOINT -J 2>/dev/null >"$LSBLK_TMP" || true
else
  echo "{}" >"$LSBLK_TMP"
fi

if have findmnt; then
  findmnt -J 2>/dev/null >"$FINDMNT_TMP" || true
else
  echo "{}" >"$FINDMNT_TMP"
fi

if have python3; then
  python3 - <<'PY' >"$TOOLS_TMP"
import json, shutil, subprocess

def ver(cmd):
    try:
        out = subprocess.check_output(cmd, stderr=subprocess.STDOUT, text=True)
        return out.strip().splitlines()[0][:400]
    except Exception as e:
        return f"ERR: {e}"

candidates = {
    "fio": ["fio", "--version"],
    "iozone": ["iozone", "-V"],
    "fs_mark": ["fs_mark", "-V"],
    "zstd": ["zstd", "--version"],
    "lz4": ["lz4", "--version"],
    "gzip": ["gzip", "--version"],
    "brotli": ["brotli", "--version"],
    "docker": ["docker", "--version"],
    "rustc": ["rustc", "--version"],
    "cargo": ["cargo", "--version"],
}

out = {}
for k, cmd in candidates.items():
    if shutil.which(cmd[0]) is None:
        continue
    out[k] = ver(cmd)

print(json.dumps(out, indent=2))
PY
else
  echo "{}" >"$TOOLS_TMP"
fi

python3 - <<PY >"$OUT_JSON"
import json
from pathlib import Path

def load_json(path):
    try:
        return json.loads(Path(path).read_text())
    except Exception:
        return {}

env = {
  "kernel": ${kernel@Q},
  "cpu_model": ${cpu_model@Q},
  "mem_total_bytes": ${mem_total_bytes@Q},
  "lsblk": load_json(${LSBLK_TMP@Q}),
  "findmnt": load_json(${FINDMNT_TMP@Q}),
  "tool_versions": load_json(${TOOLS_TMP@Q}),
}

print(json.dumps(env, indent=2))
PY

echo "Wrote $OUT_JSON" >&2
