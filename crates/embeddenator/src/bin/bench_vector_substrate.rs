use clap::Parser;
use embeddenator::EmbrFS;
use embeddenator::retrieval::RerankedResult;
use embeddenator::vsa::ReversibleVSAConfig;
use serde::Serialize;
use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "bench_vector_substrate")]
#[command(about = "Vector search bench for substrate (recall/QPS vs brute force)")]
struct Args {
    #[arg(short, long, value_name = "DIR")]
    input: PathBuf,

    /// Number of queries to run (defaults to min(chunks, 200)).
    #[arg(long)]
    queries: Option<usize>,

    /// Top-k to evaluate.
    #[arg(long, default_value_t = 10)]
    k: usize,

    /// Candidate set size factor (candidate_k = k * factor).
    #[arg(long, default_value_t = 10)]
    candidate_factor: usize,

    /// Where to write JSON report. If omitted, prints to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Serialize)]
struct LatencyStats {
    count: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    mean_ms: f64,
}

#[derive(Serialize)]
struct Report {
    version: String,
    chunks: usize,
    queries: usize,
    k: usize,
    candidate_k: usize,
    qps: f64,
    latency_ms: LatencyStats,
    recall_at_k: f64,
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    if !args.input.is_dir() {
        return Err(io::Error::other("--input must be a directory"));
    }

    let config = ReversibleVSAConfig::default();
    let mut fsys = EmbrFS::new();
    fsys.ingest_directory(&args.input, false, &config)?;

    let engram = &fsys.engram;
    let index = engram.build_codebook_index();

    let mut codebook: Vec<(usize, embeddenator::SparseVec)> = engram
        .codebook
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    codebook.sort_by_key(|(k, _)| *k);

    let chunks = codebook.len();
    if chunks == 0 {
        return Err(io::Error::other("no chunks in codebook"));
    }

    let k = args.k.max(1).min(chunks);
    let candidate_k = (k.saturating_mul(args.candidate_factor)).max(50).min(chunks);

    let queries = args.queries.unwrap_or_else(|| chunks.min(200));
    let queries = queries.max(1).min(chunks);

    // Use the first N chunk vectors as queries for determinism.
    let query_vecs: Vec<(usize, embeddenator::SparseVec)> = codebook.iter().take(queries).cloned().collect();

    let mut latencies_ms: Vec<f64> = Vec::with_capacity(queries);
    let mut total_recall_hits: usize = 0;

    for (qid, qv) in &query_vecs {
        // Approx path (index -> rerank)
        let start = Instant::now();
        let approx: Vec<RerankedResult> = engram.query_codebook_with_index(&index, qv, candidate_k, k);
        let elapsed = start.elapsed();
        latencies_ms.push(elapsed.as_secs_f64() * 1000.0);

        // Brute force exact top-k
        let mut exact: Vec<(usize, f64)> = Vec::with_capacity(chunks);
        for (cid, cv) in &codebook {
            let sim = qv.cosine(cv);
            exact.push((*cid, sim));
        }
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        exact.truncate(k);

        let exact_ids: std::collections::HashSet<usize> = exact.into_iter().map(|(id, _)| id).collect();
        let approx_ids: std::collections::HashSet<usize> = approx.into_iter().map(|r| r.id).collect();

        let hits = approx_ids.intersection(&exact_ids).count();
        total_recall_hits += hits;

        // Sanity: for self-query we expect itself in exact top-k.
        if !exact_ids.contains(qid) {
            // Not fatal, but indicates cosine ties/ordering edge; still continue.
        }
    }

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mean_ms = latencies_ms.iter().sum::<f64>() / (latencies_ms.len().max(1) as f64);

    let total_time_s = latencies_ms.iter().sum::<f64>() / 1000.0;
    let qps = if total_time_s <= 0.0 { 0.0 } else { (queries as f64) / total_time_s };

    let recall = (total_recall_hits as f64) / ((queries * k) as f64);

    let report = Report {
        version: env!("CARGO_PKG_VERSION").to_string(),
        chunks,
        queries,
        k,
        candidate_k,
        qps,
        latency_ms: LatencyStats {
            count: queries,
            p50_ms: quantile(&latencies_ms, 0.50),
            p95_ms: quantile(&latencies_ms, 0.95),
            p99_ms: quantile(&latencies_ms, 0.99),
            mean_ms,
        },
        recall_at_k: recall,
    };

    let json = serde_json::to_string_pretty(&report).map_err(io::Error::other)?;
    if let Some(out) = args.out {
        fs::write(out, json)?;
    } else {
        println!("{}", json);
    }

    Ok(())
}
