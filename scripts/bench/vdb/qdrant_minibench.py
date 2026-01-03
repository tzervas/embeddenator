#!/usr/bin/env python3
import argparse
import json
import math
import os
import random
import subprocess
import time
import urllib.request
import urllib.error
from pathlib import Path

def http_json(method, url, body=None, timeout=30):
    data = None
    headers = {"Content-Type": "application/json"}
    if body is not None:
        data = json.dumps(body).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))

def wait_ready(base_url, deadline_s=60):
    t0 = time.time()
    while time.time() - t0 < deadline_s:
        try:
            http_json("GET", f"{base_url}/")
            return True
        except Exception:
            time.sleep(0.5)
    return False

def cosine(a, b):
    dot = 0.0
    na = 0.0
    nb = 0.0
    for x, y in zip(a, b):
        dot += x * y
        na += x * x
        nb += y * y
    if na <= 0.0 or nb <= 0.0:
        return 0.0
    return dot / (math.sqrt(na) * math.sqrt(nb))

def quantile(sorted_vals, q):
    if not sorted_vals:
        return 0.0
    idx = round((len(sorted_vals) - 1) * q)
    idx = min(max(idx, 0), len(sorted_vals) - 1)
    return sorted_vals[idx]

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--image", default="qdrant/qdrant:latest")
    ap.add_argument("--port", type=int, default=6333)
    ap.add_argument("--vectors", type=int, default=20000)
    ap.add_argument("--dim", type=int, default=128)
    ap.add_argument("--queries", type=int, default=200)
    ap.add_argument("--k", type=int, default=10)
    ap.add_argument("--seed", type=int, default=123)
    ap.add_argument("--batch", type=int, default=512)
    args = ap.parse_args()

    random.seed(args.seed)
    base_url = f"http://127.0.0.1:{args.port}"
    cname = f"embeddenator-qdrant-bench-{int(time.time())}"
    collection = f"c{args.seed}"

    # Start container
    subprocess.run([
        "docker", "run", "-d", "--rm",
        "--name", cname,
        "-p", f"{args.port}:6333",
        args.image,
    ], check=True)

    try:
        if not wait_ready(base_url, deadline_s=90):
            raise RuntimeError("qdrant did not become ready")

        # Capture image id for reproducibility
        insp = subprocess.check_output(["docker", "inspect", cname], text=True)
        image_id = None
        try:
            image_id = json.loads(insp)[0].get("Image")
        except Exception:
            pass

        # Create collection
        http_json(
            "PUT",
            f"{base_url}/collections/{collection}",
            {
                "vectors": {
                    "size": args.dim,
                    "distance": "Cosine",
                }
            },
        )

        # Generate vectors
        vectors = []
        for i in range(args.vectors):
            # Deterministic pseudo-random floats in [-1, 1]
            v = [(random.random() * 2.0 - 1.0) for _ in range(args.dim)]
            vectors.append(v)

        # Upload points in batches
        t_ingest0 = time.perf_counter()
        for start in range(0, args.vectors, args.batch):
            end = min(args.vectors, start + args.batch)
            pts = []
            for i in range(start, end):
                pts.append({"id": i, "vector": vectors[i], "payload": {}})
            http_json(
                "PUT",
                f"{base_url}/collections/{collection}/points?wait=true",
                {"points": pts},
                timeout=120,
            )
        ingest_s = time.perf_counter() - t_ingest0

        # Query
        q = min(args.queries, args.vectors)
        lat_ms = []
        recall_hits = 0
        total = q * args.k

        for qi in range(q):
            query_vec = vectors[qi]

            t0 = time.perf_counter()
            res = http_json(
                "POST",
                f"{base_url}/collections/{collection}/points/search",
                {"vector": query_vec, "limit": args.k},
                timeout=60,
            )
            lat_ms.append((time.perf_counter() - t0) * 1000.0)

            got_ids = [int(p["id"]) for p in res.get("result", [])]

            # Brute force exact
            scored = [(i, cosine(query_vec, vectors[i])) for i in range(args.vectors)]
            scored.sort(key=lambda x: x[1], reverse=True)
            exact_ids = set([i for i, _ in scored[: args.k]])

            recall_hits += sum(1 for i in got_ids if i in exact_ids)

        lat_ms.sort()
        mean_ms = sum(lat_ms) / max(1, len(lat_ms))
        qps = (q / (sum(lat_ms) / 1000.0)) if sum(lat_ms) > 0 else 0.0
        recall = recall_hits / total if total > 0 else 0.0

        out = {
            "image": args.image,
            "image_id": image_id,
            "vectors": args.vectors,
            "dim": args.dim,
            "queries": q,
            "k": args.k,
            "seed": args.seed,
            "ingest_s": ingest_s,
            "qps": qps,
            "latency_ms": {
                "count": q,
                "p50": quantile(lat_ms, 0.50),
                "p95": quantile(lat_ms, 0.95),
                "p99": quantile(lat_ms, 0.99),
                "mean": mean_ms,
            },
            "recall_at_k": recall,
            "notes": [
                "This is a small-scale, reproducible micro-to-meso harness (not Big-ANN).",
                "Recall@k is computed against brute-force cosine over the same synthetic vectors.",
            ],
        }
        Path(args.out).write_text(json.dumps(out, indent=2))

    finally:
        subprocess.run(["docker", "stop", cname], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

if __name__ == "__main__":
    main()
