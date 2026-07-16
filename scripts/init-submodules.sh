#!/usr/bin/env bash
# Initialize component submodules (requires .gitmodules in repo root).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
if [[ ! -f .gitmodules ]]; then
  echo "error: .gitmodules missing in $root" >&2
  exit 1
fi
git submodule sync --recursive
git submodule update --init --recursive
echo "Submodules ready under $root"