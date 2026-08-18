#!/usr/bin/env bash
set -euo pipefail
tree=$(cargo tree -e features --prefix none -p embeddenator-vsa)
if echo "$tree" | grep -q '^cudarc ' && echo "$tree" | grep -q '^cubecl '; then
  echo 'refusing build: cudarc and cubecl both present'
  exit 1
fi
echo 'cuda exclusivity ok (at most one of cudarc, cubecl)'
