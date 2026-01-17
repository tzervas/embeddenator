//! Scaled memory/throughput tests (opt-in).
//!
//! These tests are intentionally isolated from default `cargo test` runs.
//! Enable with:
//!   EMBEDDENATOR_RUN_QA_MEMORY=1 cargo test --features qa --test memory_scaled -- --ignored --nocapture

#![cfg(feature = "qa")]

use embeddenator::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

fn is_enabled() -> bool {
    matches!(
        std::env::var("EMBEDDENATOR_RUN_QA_MEMORY").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

fn write_file_of_size(path: &Path, size_bytes: usize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut f = fs::File::create(path)?;
    // Deterministic pattern: repeating 0..=255
    let mut buf = [0u8; 64 * 1024];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i % 256) as u8;
    }

    let mut remaining = size_bytes;
    while remaining > 0 {
        let n = remaining.min(buf.len());
        f.write_all(&buf[..n])?;
        remaining -= n;
    }

    Ok(())
}

fn make_dataset(dir: &Path, total_mb: usize, file_mb: usize) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;

    let total_bytes = total_mb * 1024 * 1024;
    let file_bytes = file_mb.max(1) * 1024 * 1024;

    let mut written = 0usize;
    let mut idx = 0usize;
    while written < total_bytes {
        let this_size = (total_bytes - written).min(file_bytes);
        let path = dir.join(format!("blob_{idx:04}.bin"));
        write_file_of_size(&path, this_size)?;
        written += this_size;
        idx += 1;
    }

    Ok(dir.to_path_buf())
}

fn read_proc_status_kb(field: &str) -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(field) {
            // e.g. "VmRSS:\t  12345 kB"
            let kb = rest
                .split_whitespace()
                .next()
                .and_then(|n| n.parse::<u64>().ok())?;
            return Some(kb);
        }
    }
    None
}

#[test]
#[ignore]
fn scaled_memory_ingest_extract() {
    if !is_enabled() {
        eprintln!("skipping scaled memory test; set EMBEDDENATOR_RUN_QA_MEMORY=1 to enable");
        return;
    }

    let total_mb = env_usize("EMBEDDENATOR_QA_MEMORY_TOTAL_MB").unwrap_or(256);
    let file_mb = env_usize("EMBEDDENATOR_QA_MEMORY_FILE_MB").unwrap_or(16);

    let tmp = TempDir::new().expect("tempdir");
    let dataset_dir = tmp.path().join("dataset");
    make_dataset(&dataset_dir, total_mb, file_mb).expect("create dataset");

    let config = ReversibleVSAConfig::default();

    let rss_before = read_proc_status_kb("VmRSS:");
    let hwm_before = read_proc_status_kb("VmHWM:");

    let start = Instant::now();
    let mut fsys = EmbrFS::new();
    fsys.ingest_directory(&dataset_dir, false, &config)
        .expect("ingest_directory");
    let ingest_dur = start.elapsed();

    let extract_dir = tmp.path().join("extract");
    fs::create_dir_all(&extract_dir).unwrap();

    let start = Instant::now();
    EmbrFS::extract(&fsys.engram, &fsys.manifest, &extract_dir, false, &config).expect("extract");
    let extract_dur = start.elapsed();

    let rss_after = read_proc_status_kb("VmRSS:");
    let hwm_after = read_proc_status_kb("VmHWM:");

    println!(
        "scaled_memory_ingest_extract: total={}MB file={}MB",
        total_mb, file_mb
    );
    println!(
        "  ingest:  {:?} ({:.3} MB/s)",
        ingest_dur,
        (total_mb as f64) / ingest_dur.as_secs_f64()
    );
    println!(
        "  extract: {:?} ({:.3} MB/s)",
        extract_dur,
        (total_mb as f64) / extract_dur.as_secs_f64()
    );
    println!("  rss_kb:  before={:?} after={:?}", rss_before, rss_after);
    println!("  hwm_kb:  before={:?} after={:?}", hwm_before, hwm_after);
}
