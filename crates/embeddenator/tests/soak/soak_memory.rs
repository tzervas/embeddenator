//! Soak / multi-GB scale memory tests (opt-in, ignored).
//!
//! This test is intentionally hard-gated.
//! Enable with:
//!   EMBEDDENATOR_RUN_SOAK=1 \
//!   EMBEDDENATOR_SOAK_TOTAL_MB=4096 \
//!   EMBEDDENATOR_SOAK_FILE_MB=64 \
//!   cargo test --release --features soak-memory --test soak_memory -- --ignored --nocapture

#![cfg(feature = "soak-memory")]

use embeddenator::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn enabled() -> bool {
    matches!(std::env::var("EMBEDDENATOR_RUN_SOAK").as_deref(), Ok("1") | Ok("true") | Ok("TRUE"))
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse::<u64>().ok())
}

fn read_mem_available_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            return rest
                .split_whitespace()
                .next()
                .and_then(|n| n.parse::<u64>().ok());
        }
    }
    None
}

fn read_proc_status_kb(field: &str) -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(field) {
            return rest
                .split_whitespace()
                .next()
                .and_then(|n| n.parse::<u64>().ok());
        }
    }
    None
}

fn write_file_of_size(path: &Path, size_bytes: u64) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut f = fs::File::create(path)?;
    let mut buf = [0u8; 64 * 1024];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i % 251) as u8; // deterministic, slightly non-trivial
    }

    let mut remaining = size_bytes;
    while remaining > 0 {
        let n = (remaining as usize).min(buf.len());
        f.write_all(&buf[..n])?;
        remaining -= n as u64;
    }

    Ok(())
}

fn make_dataset(dir: &Path, total_mb: u64, file_mb: u64) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;

    let total_bytes = total_mb * 1024 * 1024;
    let file_bytes = file_mb.max(1) * 1024 * 1024;

    let mut written = 0u64;
    let mut idx = 0u64;
    while written < total_bytes {
        let this_size = (total_bytes - written).min(file_bytes);
        let path = dir.join(format!("blob_{idx:05}.bin"));
        write_file_of_size(&path, this_size)?;
        written += this_size;
        idx += 1;
    }

    Ok(dir.to_path_buf())
}

#[test]
#[ignore]
fn soak_memory_ingest_extract() {
    if !enabled() {
        eprintln!("skipping soak test; set EMBEDDENATOR_RUN_SOAK=1 to enable");
        return;
    }

    let total_mb = env_u64("EMBEDDENATOR_SOAK_TOTAL_MB").unwrap_or(1024);
    let file_mb = env_u64("EMBEDDENATOR_SOAK_FILE_MB").unwrap_or(64);
    let max_seconds = env_u64("EMBEDDENATOR_SOAK_MAX_SECONDS").unwrap_or(60 * 60);
    let max_duration = Duration::from_secs(max_seconds);

    // Safety: refuse to start if the machine looks too small, unless forced.
    let force = matches!(std::env::var("EMBEDDENATOR_FORCE").as_deref(), Ok("1") | Ok("true") | Ok("TRUE"));
    if !force {
        if let Some(avail_kb) = read_mem_available_kb() {
            // Heuristic: require at least ~2x dataset size in available memory.
            let required_kb = total_mb.saturating_mul(2) * 1024;
            if avail_kb < required_kb {
                eprintln!(
                    "skipping soak test; MemAvailable={}kB < required~{}kB (set EMBEDDENATOR_FORCE=1 to override)",
                    avail_kb, required_kb
                );
                return;
            }
        }
    }

    let tmp = TempDir::new().expect("tempdir");
    let dataset_dir = tmp.path().join("dataset");
    make_dataset(&dataset_dir, total_mb, file_mb).expect("create dataset");

    let config = ReversibleVSAConfig::default();

    let rss_before = read_proc_status_kb("VmRSS:");
    let hwm_before = read_proc_status_kb("VmHWM:");

    let mut fsys = EmbrFS::new();

    let start_all = Instant::now();

    let start = Instant::now();
    fsys.ingest_directory(&dataset_dir, false, &config)
        .expect("ingest_directory");
    let ingest_dur = start.elapsed();

    if start_all.elapsed() > max_duration {
        panic!("soak test exceeded max duration during ingest: {:?}", max_duration);
    }

    let extract_dir = tmp.path().join("extract");
    fs::create_dir_all(&extract_dir).unwrap();

    let start = Instant::now();
    EmbrFS::extract(&fsys.engram, &fsys.manifest, &extract_dir, false, &config)
        .expect("extract");
    let extract_dur = start.elapsed();

    if start_all.elapsed() > max_duration {
        panic!("soak test exceeded max duration during extract: {:?}", max_duration);
    }

    let rss_after = read_proc_status_kb("VmRSS:");
    let hwm_after = read_proc_status_kb("VmHWM:");

    println!("soak_memory_ingest_extract: total={}MB file={}MB", total_mb, file_mb);
    println!("  ingest:  {:?} ({:.3} MB/s)", ingest_dur, (total_mb as f64) / ingest_dur.as_secs_f64());
    println!("  extract: {:?} ({:.3} MB/s)", extract_dur, (total_mb as f64) / extract_dur.as_secs_f64());
    println!("  rss_kb:  before={:?} after={:?}", rss_before, rss_after);
    println!("  hwm_kb:  before={:?} after={:?}", hwm_before, hwm_after);
}
