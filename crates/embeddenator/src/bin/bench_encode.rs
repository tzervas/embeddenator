use embeddenator::EmbrFS;
use embeddenator_io::envelope::{BinaryWriteOptions, CompressionCodec, PayloadKind, wrap_or_legacy};
use embeddenator::vsa::ReversibleVSAConfig;
use clap::Parser;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CodecArg {
	None,
	Zstd,
	Lz4,
}

impl From<CodecArg> for CompressionCodec {
	fn from(v: CodecArg) -> Self {
		match v {
			CodecArg::None => CompressionCodec::None,
			CodecArg::Zstd => CompressionCodec::Zstd,
			CodecArg::Lz4 => CompressionCodec::Lz4,
		}
	}
}

#[derive(Parser, Debug)]
#[command(name = "bench_encode")]
#[command(about = "Encode/extract benchmark runner (JSON output)")]
struct Args {
	/// Input directory or file. Can be provided multiple times.
	#[arg(short, long, value_name = "PATH", num_args = 1.., action = clap::ArgAction::Append)]
	input: Vec<PathBuf>,

	/// Logical prefix for each input path; defaults to basename.
	#[arg(long, value_name = "PREFIX")]
	prefix: Option<String>,

	/// Engram compression codec.
	#[arg(long, value_enum, default_value = "none")]
	engram_codec: CodecArg,

	/// Engram compression level (codec-dependent; used for zstd).
	#[arg(long)]
	engram_level: Option<i32>,

	/// Perform an extract + SHA256 verify pass.
	#[arg(long, default_value_t = false)]
	verify: bool,

	/// Where to write the JSON report. If omitted, prints to stdout.
	#[arg(long, value_name = "FILE")]
	out: Option<PathBuf>,
}

#[derive(Serialize)]
struct SizeBreakdown {
	raw_bytes: u64,

	// Serialized component sizes (uncompressed serialization).
	root_bincode_bytes: usize,
	codebook_bincode_bytes: usize,
	corrections_bincode_bytes: usize,
	manifest_json_bytes: usize,

	// On-disk artifact size.
	engram_wrapped_bytes: usize,

	// Derived.
	effective_ratio_including_corrections: f64,
}

#[derive(Serialize)]
struct TimingBreakdown {
	ingest_ms: u128,
	extract_ms: Option<u128>,
}

#[derive(Serialize)]
struct CorrectionSummary {
	total_chunks: u64,
	perfect_ratio: f64,
	correction_ratio: f64,
}

#[derive(Serialize)]
struct Report {
	version: String,
	inputs: Vec<String>,
	codec: String,
	codec_level: Option<i32>,

	timing: TimingBreakdown,
	sizes: SizeBreakdown,
	corrections: CorrectionSummary,

	verify_ok: Option<bool>,
	verify_mismatches: Option<u64>,
}

fn sha256_file(path: &Path) -> io::Result<[u8; 32]> {
	let bytes = fs::read(path)?;
	Ok(Sha256::digest(&bytes).into())
}

fn hex32(d: [u8; 32]) -> String {
	let mut s = String::with_capacity(64);
	for b in d {
		s.push_str(&format!("{:02x}", b));
	}
	s
}

fn collect_files(root: &Path) -> io::Result<Vec<PathBuf>> {
	let mut out = Vec::new();
	if root.is_file() {
		out.push(root.to_path_buf());
		return Ok(out);
	}

	for entry in walkdir::WalkDir::new(root).follow_links(false) {
		let entry = entry?;
		if entry.file_type().is_file() {
			out.push(entry.path().to_path_buf());
		}
	}
	out.sort();
	Ok(out)
}

fn logical_prefix_for_input(input: &Path, explicit: Option<&str>) -> String {
	if let Some(p) = explicit {
		return p.to_string();
	}
	input
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or("input")
		.to_string()
}

fn main() -> io::Result<()> {
	let args = Args::parse();

	if args.input.is_empty() {
		return Err(io::Error::other("at least one --input is required"));
	}

	let config = ReversibleVSAConfig::default();

	// Build an EmbrFS by ingesting each input path under a prefix.
	let mut fsys = EmbrFS::new();

	// Compute raw bytes + per-file hashes (for optional verification).
	let mut raw_bytes: u64 = 0;
	let mut original_hashes: BTreeMap<String, String> = BTreeMap::new();

	for input in &args.input {
		let files = collect_files(input)?;
		for f in files {
			raw_bytes += fs::metadata(&f)?.len();
			if args.verify {
				let rel = if input.is_dir() {
					match f.strip_prefix(input).ok().and_then(|p| p.to_str()) {
						Some(s) => s.replace('\\', "/"),
						None => f.to_string_lossy().replace('\\', "/"),
					}
				} else {
					f.file_name()
						.and_then(|s| s.to_str())
						.unwrap_or("input.bin")
						.to_string()
				};

				let key = format!(
					"{}/{}",
					logical_prefix_for_input(input, args.prefix.as_deref()),
					rel
				)
				.trim_end_matches('/')
				.to_string();
				original_hashes.insert(key, hex32(sha256_file(&f)?));
			}
		}
	}

	let ingest_start = Instant::now();
	for input in &args.input {
		if input.is_dir() {
			let prefix = logical_prefix_for_input(input, args.prefix.as_deref());
			fsys.ingest_directory_with_prefix(input, Some(&prefix), false, &config)?;
		} else {
			let prefix = logical_prefix_for_input(input, args.prefix.as_deref());
			let logical_path = format!(
				"{}/{}",
				prefix,
				input
					.file_name()
					.and_then(|s| s.to_str())
					.unwrap_or("input.bin")
			);
			fsys.ingest_file(input, logical_path, false, &config)?;
		}
	}
	let ingest_dur = ingest_start.elapsed();

	// Component sizes.
	let root_bincode = bincode::serialize(&fsys.engram.root).map_err(io::Error::other)?;
	let codebook_bincode = bincode::serialize(&fsys.engram.codebook).map_err(io::Error::other)?;
	let corrections_bincode =
		bincode::serialize(&fsys.engram.corrections).map_err(io::Error::other)?;
	let manifest_json = serde_json::to_vec(&fsys.manifest).map_err(io::Error::other)?;

	// On-disk engram size (wrapped if codec != none).
	let engram_bincode = bincode::serialize(&fsys.engram).map_err(io::Error::other)?;
	let opts = BinaryWriteOptions {
		codec: args.engram_codec.into(),
		level: args.engram_level,
	};
	let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, &engram_bincode)?;

	let denom = (root_bincode.len()
		+ codebook_bincode.len()
		+ corrections_bincode.len()
		+ manifest_json.len()) as f64;
	let effective_ratio = if denom <= 0.0 { 0.0 } else { raw_bytes as f64 / denom };

	let mut extract_ms = None;
	let mut verify_ok = None;
	let mut verify_mismatches = None;

	if args.verify {
		let temp = TempDir::new()?;
		let engram_path = temp.path().join("root.engram");
		let manifest_path = temp.path().join("manifest.json");
		let out_dir = temp.path().join("out");

		fsys.save_engram_with_options(&engram_path, opts)?;
		fsys.save_manifest(&manifest_path)?;

		let e = EmbrFS::load_engram(&engram_path)?;
		let m = EmbrFS::load_manifest(&manifest_path)?;

		let extract_start = Instant::now();
		EmbrFS::extract(&e, &m, &out_dir, false, &config)?;
		extract_ms = Some(extract_start.elapsed().as_millis());

		// Verify SHA256 per file.
		let mut mismatches: u64 = 0;
		for (logical_path, expected_hash) in &original_hashes {
			let extracted_path = out_dir.join(logical_path);
			let got_hash = hex32(sha256_file(&extracted_path)?);
			if &got_hash != expected_hash {
				mismatches += 1;
			}
		}
		verify_ok = Some(mismatches == 0);
		verify_mismatches = Some(mismatches);
	}

	let stats = fsys.correction_stats();

	let report = Report {
		version: env!("CARGO_PKG_VERSION").to_string(),
		inputs: args
			.input
			.iter()
			.map(|p| p.to_string_lossy().to_string())
			.collect(),
		codec: format!("{:?}", args.engram_codec),
		codec_level: args.engram_level,
		timing: TimingBreakdown {
			ingest_ms: ingest_dur.as_millis(),
			extract_ms,
		},
		sizes: SizeBreakdown {
			raw_bytes,
			root_bincode_bytes: root_bincode.len(),
			codebook_bincode_bytes: codebook_bincode.len(),
			corrections_bincode_bytes: corrections_bincode.len(),
			manifest_json_bytes: manifest_json.len(),
			engram_wrapped_bytes: wrapped.len(),
			effective_ratio_including_corrections: effective_ratio,
		},
		corrections: CorrectionSummary {
			total_chunks: stats.total_chunks,
			perfect_ratio: stats.perfect_ratio,
			correction_ratio: stats.correction_ratio,
		},
		verify_ok,
		verify_mismatches,
	};

	let json = serde_json::to_string_pretty(&report).map_err(io::Error::other)?;
	if let Some(out) = args.out {
		fs::write(out, json)?;
	} else {
		println!("{}", json);
	}

	Ok(())
}