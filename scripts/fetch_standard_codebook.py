#!/usr/bin/env python3
"""Fetch tiered Standard Baseline Codebook (SBCB) artifacts.

- Reads: codebooks/standard/manifest.json
- Downloads: the requested tier artifact (if URLs are provided)
- Verifies: SHA-256
- Stores: XDG cache dir by default

This is intentionally stdlib-only so it works in CI without extra deps.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import sys
import tempfile
import urllib.request


def _xdg_cache_dir() -> pathlib.Path:
    xdg = os.environ.get("XDG_CACHE_HOME")
    if xdg:
        return pathlib.Path(xdg)
    return pathlib.Path.home() / ".cache"


def _default_cache_root() -> pathlib.Path:
    return _xdg_cache_dir() / "embeddenator" / "codebooks"


def _sha256_file(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _download(url: str, dest_tmp: pathlib.Path) -> None:
    with urllib.request.urlopen(url) as resp:
        with dest_tmp.open("wb") as f:
            while True:
                chunk = resp.read(1024 * 1024)
                if not chunk:
                    break
                f.write(chunk)


def main() -> int:
    parser = argparse.ArgumentParser(description="Fetch Embeddenator standard baseline codebook tier")
    parser.add_argument("--manifest", default="codebooks/standard/manifest.json")
    parser.add_argument("--tier", required=True, choices=["tiny", "small", "medium", "large"])
    parser.add_argument("--cache-root", default=None)
    parser.add_argument("--output", default=None, help="If set, copy the artifact to this exact path")
    parser.add_argument("--require", action="store_true", help="Fail if the artifact is missing/unfetchable")

    args = parser.parse_args()

    manifest_path = pathlib.Path(args.manifest)
    if not manifest_path.exists():
        print(f"Manifest not found: {manifest_path}", file=sys.stderr)
        return 2

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    family = manifest.get("family")
    content_version = manifest.get("content_version")

    tier_entry = None
    for t in manifest.get("tiers", []):
        if t.get("tier") == args.tier:
            tier_entry = t
            break

    if not tier_entry:
        msg = f"Tier {args.tier} not present in manifest"
        if args.require:
            print(msg, file=sys.stderr)
            return 2
        print(msg)
        return 0

    artifact = tier_entry.get("artifact", {})
    filename = artifact.get("filename")
    sha256_hex = (artifact.get("sha256_hex") or "").strip().lower()
    urls = artifact.get("urls") or []

    if not filename:
        msg = f"Tier {args.tier} missing artifact filename"
        if args.require:
            print(msg, file=sys.stderr)
            return 2
        print(msg)
        return 0

    cache_root = pathlib.Path(args.cache_root) if args.cache_root else _default_cache_root()
    cache_dir = cache_root / family / content_version
    cache_dir.mkdir(parents=True, exist_ok=True)
    cached_path = cache_dir / filename

    if cached_path.exists() and sha256_hex:
        got = _sha256_file(cached_path)
        if got == sha256_hex:
            print(str(cached_path))
            return 0
        cached_path.unlink(missing_ok=True)

    if not urls:
        msg = f"No URLs in manifest for tier {args.tier}; cannot fetch"
        if args.require:
            print(msg, file=sys.stderr)
            return 2
        print(msg)
        return 0

    with tempfile.TemporaryDirectory() as td:
        tmp_path = pathlib.Path(td) / (filename + ".tmp")
        last_err = None

        for url in urls:
            try:
                _download(url, tmp_path)
                if sha256_hex:
                    got = _sha256_file(tmp_path)
                    if got != sha256_hex:
                        raise RuntimeError(f"SHA256 mismatch: expected {sha256_hex}, got {got}")
                tmp_path.replace(cached_path)
                break
            except Exception as e:
                last_err = e
                try:
                    tmp_path.unlink(missing_ok=True)
                except Exception:
                    pass
        else:
            msg = f"Failed to fetch tier {args.tier}: {last_err}"
            if args.require:
                print(msg, file=sys.stderr)
                return 2
            print(msg)
            return 0

    if args.output:
        out = pathlib.Path(args.output)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(cached_path.read_bytes())
        print(str(out))
        return 0

    print(str(cached_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
