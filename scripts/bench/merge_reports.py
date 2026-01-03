#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

def fmt_bytes(n: int) -> str:
    for unit in ["B", "KB", "MB", "GB", "TB"]:
        if n < 1024 or unit == "TB":
            return f"{n:.2f} {unit}" if unit != "B" else f"{n} B"
        n /= 1024
    return str(n)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--env", required=True)
    ap.add_argument("--embed", required=True)
    ap.add_argument("--compress", required=True)
    ap.add_argument("--out-json", required=True)
    ap.add_argument("--out-md", required=True)
    args = ap.parse_args()

    env = json.loads(Path(args.env).read_text())
    embed = json.loads(Path(args.embed).read_text())
    comp = json.loads(Path(args.compress).read_text())

    out = {
        "env": env,
        "embeddenator": embed,
        "compression_baselines": comp,
    }
    Path(args.out_json).write_text(json.dumps(out, indent=2))

    # Markdown summary (no invented numbers; only computed from reports)
    raw = int(embed["sizes"]["raw_bytes"])
    denom = (
        int(embed["sizes"]["root_bincode_bytes"]) +
        int(embed["sizes"]["codebook_bincode_bytes"]) +
        int(embed["sizes"]["corrections_bincode_bytes"]) +
        int(embed["sizes"]["manifest_json_bytes"])
    )

    lines = []
    lines.append("# Benchmark Report")
    lines.append("")
    lines.append("## Embeddenator (Substrate)")
    lines.append("")
    lines.append(f"- Version: {embed.get('version')}")
    lines.append(f"- Inputs: {', '.join(embed.get('inputs', []))}")
    lines.append(f"- Engram codec: {embed.get('codec')} level={embed.get('codec_level')}")
    lines.append(f"- Raw bytes: {fmt_bytes(raw)}")
    lines.append(f"- Effective denom (root+codebook+corrections+manifest): {fmt_bytes(denom)}")
    lines.append(f"- Effective ratio (raw/denom): {embed['sizes']['effective_ratio_including_corrections']:.4f}x")
    lines.append(f"- Engram on-disk bytes: {fmt_bytes(int(embed['sizes']['engram_wrapped_bytes']))}")
    lines.append(f"- Ingest time: {embed['timing']['ingest_ms']} ms")
    if embed['timing'].get('extract_ms') is not None:
        lines.append(f"- Extract time: {embed['timing']['extract_ms']} ms")
        lines.append(f"- Verify: ok={embed.get('verify_ok')} mismatches={embed.get('verify_mismatches')}")
    lines.append("")

    lines.append("## Traditional Compression Baselines (tar stream)")
    lines.append("")
    tar_raw = int(comp.get("tar_raw_bytes", 0))
    lines.append(f"- Tar stream bytes: {fmt_bytes(tar_raw)}")
    baselines = comp.get("baselines", [])
    if not baselines:
        lines.append("- No baseline compressors detected on PATH.")
    else:
        lines.append("")
        lines.append("| Codec | Output Size | Ratio (tar/out) | Elapsed (s) | CPU % |")
        lines.append("|---|---:|---:|---:|---:|")
        for b in baselines:
            out_bytes = int(b.get("out_bytes", 0))
            ratio = (tar_raw / out_bytes) if out_bytes else 0.0
            elapsed = b.get("elapsed_s", "")
            cpu = b.get("cpu_pct", "")
            lines.append(f"| {b.get('name')} | {fmt_bytes(out_bytes)} | {ratio:.4f}x | {elapsed} | {cpu} |")

    lines.append("")
    lines.append("## Environment (captured)")
    lines.append("")
    lines.append(f"- Kernel: {env.get('kernel')}")
    lines.append(f"- CPU: {env.get('cpu_model')}")
    lines.append(f"- RAM bytes: {env.get('mem_total_bytes')}")

    Path(args.out_md).write_text("\n".join(lines) + "\n")

if __name__ == "__main__":
    main()
