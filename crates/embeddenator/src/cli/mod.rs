//! CLI interface for Embeddenator
//!
//! Provides command-line interface for:
//! - Ingesting files/directories into engrams
//! - Extracting files from engrams
//! - Querying similarity
//! - Mounting engrams as FUSE filesystems (requires `fuse` feature)

use crate::fs::fs::embrfs::{
    DirectorySubEngramStore, EmbrFS, HierarchicalQueryBounds, load_hierarchical_manifest,
    query_hierarchical_codebook_with_store,
    save_hierarchical_manifest, save_sub_engrams_dir_with_options,
};
use crate::io::envelope::{BinaryWriteOptions, CompressionCodec};
use crate::vsa::vsa::{SparseVec, ReversibleVSAConfig};
use clap::{Parser, Subcommand};
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum CompressionArg {
    None,
    Zstd,
    Lz4,
}

impl From<CompressionArg> for CompressionCodec {
    fn from(v: CompressionArg) -> Self {
        match v {
            CompressionArg::None => CompressionCodec::None,
            CompressionArg::Zstd => CompressionCodec::Zstd,
            CompressionArg::Lz4 => CompressionCodec::Lz4,
        }
    }
}

fn path_to_forward_slash_string(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(|v| v.to_string()),
            _ => None,
        })
        .collect::<Vec<String>>()
        .join("/")
}

fn logical_path_for_file_input(path: &Path, cwd: &Path) -> String {
    if path.is_relative() {
        return path_to_forward_slash_string(path);
    }

    if let Ok(rel) = path.strip_prefix(cwd) {
        let s = path_to_forward_slash_string(rel);
        if !s.is_empty() {
            return s;
        }
    }

    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.bin")
        .to_string()
}

#[derive(Parser)]
#[command(name = "embeddenator")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Holographic computing substrate using sparse ternary VSA")]
#[command(
    long_about = "Embeddenator - A pre-1.0 holographic computing substrate using Vector Symbolic Architecture (VSA)\n\n\
    Embeddenator encodes entire filesystems into holographic 'engrams' using sparse ternary vectors,\n\
    enabling bit-perfect reconstruction and algebraic operations on data.\n\n\
    Key Features:\n\
    • 100% bit-perfect reconstruction of all files\n\
    • Holographic superposition of multiple data sources\n\
    • Algebraic operations (bundle, bind) on engrams\n\
    • Hierarchical chunked encoding for TB-scale data\n\
    • Multi-architecture support (amd64/arm64)\n\n\
    Examples:\n\
      embeddenator ingest -i ./mydata -e data.engram -m data.json -v\n\
      embeddenator extract -e data.engram -m data.json -o ./restored -v\n\
      embeddenator query -e data.engram -q ./testfile.txt -v"
)]
#[command(author = "Tyler Zervas <tz-dev@vectorweight.com>")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Ingest files/directories into a holographic engram
    #[command(
        long_about = "Ingest files and directories into a holographic engram\n\n\
        This command recursively processes all files in the input directory, chunks them,\n\
        and encodes them into a holographic VSA engram. The result is a single .engram file\n\
        containing the superposition of all data, plus a manifest tracking file metadata.\n\n\
        The engram uses sparse ternary vectors to create a holographic representation where:\n\
        • All files are superimposed in a single root vector\n\
        • Each chunk is bound to a unique position vector\n\
        • Reconstruction is bit-perfect for all file types\n\n\
        Example:\n\
          embeddenator ingest -i ./myproject -e project.engram -m project.json -v\n\
          embeddenator ingest --input ~/Documents --engram docs.engram --verbose"
    )]
    Ingest {
        /// Input path(s) to ingest (directory or file). Can be provided multiple times.
        #[arg(
            short,
            long,
            value_name = "PATH",
            help_heading = "Required",
            num_args = 1..,
            action = clap::ArgAction::Append
        )]
        input: Vec<PathBuf>,

        /// Output engram file containing holographic encoding
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Optional compression for the output engram (default: none)
        #[arg(long, default_value = "none", value_enum)]
        engram_compression: CompressionArg,

        /// Optional compression level (codec-dependent; used for zstd)
        #[arg(long, value_name = "LEVEL")]
        engram_compression_level: Option<i32>,

        /// Output manifest file containing file metadata and chunk mappings
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Enable verbose output showing ingestion progress and statistics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Extract and reconstruct files from a holographic engram
    #[command(
        long_about = "Extract and reconstruct files from a holographic engram\n\n\
        This command performs bit-perfect reconstruction of all files from an engram.\n\
        It uses the manifest to locate chunks in the codebook and algebraically unbinds\n\
        them from the holographic root vector to recover the original data.\n\n\
        The extraction process:\n\
        • Loads the engram and manifest files\n\
        • Reconstructs the directory structure\n\
        • Unbinds and decodes each chunk using VSA operations\n\
        • Writes bit-perfect copies of all original files\n\n\
        Example:\n\
          embeddenator extract -e project.engram -m project.json -o ./restored -v\n\
          embeddenator extract --engram backup.engram --output-dir ~/restored"
    )]
    Extract {
        /// Input engram file to extract from
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Input manifest file with metadata and chunk mappings
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Output directory where files will be reconstructed
        #[arg(short, long, value_name = "DIR", help_heading = "Required")]
        output_dir: PathBuf,

        /// Enable verbose output showing extraction progress
        #[arg(short, long)]
        verbose: bool,
    },

    /// Query similarity between a file and engram contents
    #[command(
        long_about = "Query cosine similarity between a file and engram contents\n\n\
        This command computes the similarity between a query file and the data encoded\n\
        in an engram using VSA cosine similarity. This enables holographic search and\n\
        content-based retrieval without full extraction.\n\n\
        Similarity interpretation:\n\
        • >0.75: Strong match, likely contains similar content\n\
        • 0.3-0.75: Moderate similarity, some shared patterns\n\
        • <0.3: Low similarity, likely unrelated content\n\n\
        Example:\n\
          embeddenator query -e archive.engram -q search.txt -v\n\
          embeddenator query --engram data.engram --query pattern.bin"
    )]
    Query {
        /// Engram file to query
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Query file to search for
        #[arg(short, long, value_name = "FILE", help_heading = "Required")]
        query: PathBuf,

        /// Optional hierarchical manifest (enables selective unfolding search)
        #[arg(long, value_name = "FILE")]
        hierarchical_manifest: Option<PathBuf>,

        /// Directory containing bincode-serialized sub-engrams (used with --hierarchical-manifest)
        #[arg(long, value_name = "DIR")]
        sub_engrams_dir: Option<PathBuf>,

        /// Top-k results to print for codebook/hierarchical search
        #[arg(long, default_value_t = 10, value_name = "K")]
        k: usize,

        /// Enable verbose output showing similarity scores and details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Query similarity using a literal text string (basic inference-to-vector)
    #[command(
        long_about = "Query cosine similarity using a literal text string\n\n\
        This is a convenience wrapper that encodes the provided text as bytes into a VSA query vector\n\
        and runs the same retrieval path as `query`."
    )]
    QueryText {
        /// Engram file to query
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Text to encode and search for
        #[arg(long, value_name = "TEXT", help_heading = "Required")]
        text: String,

        /// Optional hierarchical manifest (enables selective unfolding search)
        #[arg(long, value_name = "FILE")]
        hierarchical_manifest: Option<PathBuf>,

        /// Directory containing bincode-serialized sub-engrams (used with --hierarchical-manifest)
        #[arg(long, value_name = "DIR")]
        sub_engrams_dir: Option<PathBuf>,

        /// Top-k results to print for codebook/hierarchical search
        #[arg(long, default_value_t = 10, value_name = "K")]
        k: usize,

        /// Enable verbose output showing similarity scores and details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Build hierarchical retrieval artifacts (manifest + sub-engrams store)
    #[command(
        long_about = "Build hierarchical retrieval artifacts from an existing engram+manifest\n\n\
        This command produces a hierarchical manifest JSON and a directory of sub-engrams\n\
        suitable for store-backed selective unfolding (DirectorySubEngramStore)."
    )]
    BundleHier {
        /// Input engram file
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Input manifest file
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Output hierarchical manifest JSON
        #[arg(long, default_value = "hier.json", value_name = "FILE")]
        out_hierarchical_manifest: PathBuf,

        /// Output directory to write bincode sub-engrams
        #[arg(long, default_value = "sub_engrams", value_name = "DIR")]
        out_sub_engrams_dir: PathBuf,

        /// Optional compression for the output `.subengram` blobs (default: none)
        #[arg(long, default_value = "none", value_enum)]
        sub_engram_compression: CompressionArg,

        /// Optional compression level (codec-dependent; used for zstd)
        #[arg(long, value_name = "LEVEL")]
        sub_engram_compression_level: Option<i32>,

        /// Maximum sparsity per level bundle
        #[arg(long, default_value_t = 500, value_name = "N")]
        max_level_sparsity: usize,

        /// Optional cap on chunk IDs per node (enables deterministic sharding when exceeded)
        #[arg(long, value_name = "N")]
        max_chunks_per_node: Option<usize>,

        /// Embed sub-engrams in the manifest JSON (in addition to writing the directory)
        #[arg(long, default_value_t = false)]
        embed_sub_engrams: bool,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Mount an engram as a FUSE filesystem (requires --features fuse)
    #[cfg(feature = "fuse")]
    #[command(
        long_about = "Mount an engram as a FUSE filesystem\n\n\
        This command mounts an engram at the specified mountpoint, making all files\n\
        accessible through the standard filesystem interface. Files are decoded\n\
        on-demand from the holographic representation.\n\n\
        Requirements:\n\
        • FUSE kernel module must be loaded (modprobe fuse)\n\
        • libfuse3-dev installed on the system\n\
        • Build with: cargo build --features fuse\n\n\
        To unmount:\n\
          fusermount -u /path/to/mountpoint\n\n\
        Example:\n\
          embeddenator mount -e project.engram -m project.json /mnt/engram\n\
          embeddenator mount --engram backup.engram --mountpoint ~/mnt --allow-other"
    )]
    Mount {
        /// Engram file to mount
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file with metadata and chunk mappings
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Mountpoint directory (must exist and be empty)
        #[arg(value_name = "MOUNTPOINT", help_heading = "Required")]
        mountpoint: PathBuf,

        /// Allow other users to access the mount
        #[arg(long)]
        allow_other: bool,

        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

pub fn run() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            input,
            engram,
            manifest,
            engram_compression,
            engram_compression_level,
            verbose,
        } => {
            if verbose {
                println!(
                    "Embeddenator v{} - Holographic Ingestion",
                    env!("CARGO_PKG_VERSION")
                );
                println!("=====================================");
            }

            let mut fs = EmbrFS::new();
            let config = ReversibleVSAConfig::default();

            // Backward-compatible behavior: a single directory input ingests with paths
            // relative to that directory (no namespacing).
            if input.len() == 1 && input[0].is_dir() {
                fs.ingest_directory(&input[0], verbose, &config)?;
            } else {
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

                // Ensure deterministic and collision-resistant namespacing for multiple directory roots.
                let mut dir_prefix_counts: HashMap<String, usize> = HashMap::new();

                for p in &input {
                    if !p.exists() {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Input path does not exist: {}", p.display()),
                        ));
                    }

                    if p.is_dir() {
                        let base = p
                            .file_name()
                            .and_then(|s| s.to_str())
                            .filter(|s| !s.is_empty())
                            .unwrap_or("input")
                            .to_string();
                        let count = dir_prefix_counts.entry(base.clone()).or_insert(0);
                        *count += 1;
                        let prefix = if *count == 1 {
                            base
                        } else {
                            format!("{}_{}", base, count)
                        };

                        fs.ingest_directory_with_prefix(p, Some(&prefix), verbose, &config)?;
                    } else {
                        let logical = logical_path_for_file_input(p, &cwd);
                        fs.ingest_file(p, logical, verbose, &config)?;
                    }
                }
            }

            fs.save_engram_with_options(
                &engram,
                BinaryWriteOptions {
                    codec: engram_compression.into(),
                    level: engram_compression_level,
                },
            )?;
            fs.save_manifest(&manifest)?;

            if verbose {
                println!("\nIngestion complete!");
                println!("  Engram: {}", engram.display());
                println!("  Manifest: {}", manifest.display());
                println!("  Files: {}", fs.manifest.files.len());
                println!("  Total chunks: {}", fs.manifest.total_chunks);
            }

            Ok(())
        }

        Commands::Extract {
            engram,
            manifest,
            output_dir,
            verbose,
        } => {
            if verbose {
                println!(
                    "Embeddenator v{} - Holographic Extraction",
                    env!("CARGO_PKG_VERSION")
                );
                println!("======================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;
            let config = ReversibleVSAConfig::default();

            EmbrFS::extract(&engram_data, &manifest_data, &output_dir, verbose, &config)?;

            if verbose {
                println!("\nExtraction complete!");
                println!("  Output: {}", output_dir.display());
            }

            Ok(())
        }

        Commands::Query {
            engram,
            query,
            hierarchical_manifest,
            sub_engrams_dir,
            k,
            verbose,
        } => {
            if verbose {
                println!(
                    "Embeddenator v{} - Holographic Query",
                    env!("CARGO_PKG_VERSION")
                );
                println!("=================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;

            let mut query_file = File::open(&query)?;
            let mut query_data = Vec::new();
            query_file.read_to_end(&mut query_data)?;

            // Chunks are encoded with a path-hash bucket shift; when querying we don't know the
            // original path, so sweep possible buckets (bounded by config.max_path_depth).
            let config = ReversibleVSAConfig::default();
            let base_query = SparseVec::encode_data(&query_data, &config, None);

            // Build the codebook index once and reuse it across the sweep.
            let codebook_index = engram_data.build_codebook_index();

            let mut best_similarity = f64::MIN;
            let mut best_shift = 0usize;
            let mut best_top_cosine = f64::MIN;

            // Merge matches across shifts; keep the best score per chunk.
            let mut merged: HashMap<usize, (f64, i32)> = HashMap::new();

            // Optionally merge hierarchical hits too.
            let mut merged_hier: HashMap<(String, usize), (f64, i32)> = HashMap::new();

            let hierarchical_loaded = if let (Some(hier_path), Some(_)) = (hierarchical_manifest.as_ref(), sub_engrams_dir.as_ref()) {
                Some(load_hierarchical_manifest(hier_path)?)
            } else {
                None
            };

            // Increase per-bucket cutoff so global top-k merge is less likely to miss true winners.
            let k_sweep = (k.saturating_mul(10)).max(100);
            let candidate_k = (k_sweep.saturating_mul(10)).max(200);

            for depth in 0..config.max_path_depth.max(1) {
                let shift = depth * config.base_shift;
                let query_vec = base_query.permute(shift);

                let similarity = query_vec.cosine(&engram_data.root);
                if similarity > best_similarity {
                    best_similarity = similarity;
                    best_shift = shift;
                }

                let matches = engram_data.query_codebook_with_index(
                    &codebook_index,
                    &query_vec,
                    candidate_k,
                    k_sweep,
                );

                if let Some(top) = matches.first() {
                    if top.cosine > best_top_cosine {
                        best_top_cosine = top.cosine;
                        best_shift = shift;
                        best_similarity = similarity;
                    }
                }

                for m in matches {
                    let entry = merged.entry(m.id).or_insert((m.cosine, m.approx_score));
                    if m.cosine > entry.0 {
                        *entry = (m.cosine, m.approx_score);
                    }
                }
            }

            // Hierarchical query can be expensive (sub-engram loads + per-node indexing).
            // Run it once using the best shift from the sweep.
            if let (Some(hierarchical), Some(sub_dir)) = (hierarchical_loaded.as_ref(), sub_engrams_dir.as_ref()) {
                let store = DirectorySubEngramStore::new(sub_dir);
                let bounds = HierarchicalQueryBounds {
                    k,
                    ..HierarchicalQueryBounds::default()
                };
                let query_vec = base_query.permute(best_shift);
                let hier_hits = query_hierarchical_codebook_with_store(
                    hierarchical,
                    &store,
                    &engram_data.codebook,
                    &query_vec,
                    &bounds,
                );
                for h in hier_hits {
                    let key = (h.sub_engram_id, h.chunk_id);
                    let entry = merged_hier.entry(key).or_insert((h.cosine, h.approx_score));
                    if h.cosine > entry.0 {
                        *entry = (h.cosine, h.approx_score);
                    }
                }
            }

            println!("Query file: {}", query.display());
            if verbose {
                println!(
                    "Best bucket-shift: {} (buckets 0..{})",
                    best_shift,
                    config.max_path_depth.saturating_sub(1)
                );
            }
            println!("Similarity to engram: {:.4}", best_similarity);

            let mut top_matches: Vec<(usize, f64, i32)> = merged
                .into_iter()
                .map(|(id, (cosine, approx))| (id, cosine, approx))
                .collect();
            top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_matches.truncate(k);

            if !top_matches.is_empty() {
                println!("Top codebook matches:");
                for (id, cosine, approx) in top_matches {
                    println!("  chunk {}  cosine {:.4}  approx_dot {}", id, cosine, approx);
                }
            } else if verbose {
                println!("Top codebook matches: (none)");
            }

            let mut top_hier: Vec<(String, usize, f64, i32)> = merged_hier
                .into_iter()
                .map(|((sub_id, chunk_id), (cosine, approx))| (sub_id, chunk_id, cosine, approx))
                .collect();
            top_hier.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            top_hier.truncate(k);

            if !top_hier.is_empty() {
                println!("Top hierarchical matches:");
                for (sub_id, chunk_id, cosine, approx) in top_hier {
                    println!("  sub {}  chunk {}  cosine {:.4}  approx_dot {}", sub_id, chunk_id, cosine, approx);
                }
            } else if verbose && hierarchical_manifest.is_some() {
                println!("Top hierarchical matches: (none)");
            }

            if best_similarity > 0.75 {
                println!("Status: STRONG MATCH");
            } else if best_similarity > 0.3 {
                println!("Status: Partial match");
            } else {
                println!("Status: No significant match");
            }

            Ok(())
        }

        Commands::QueryText {
            engram,
            text,
            hierarchical_manifest,
            sub_engrams_dir,
            k,
            verbose,
        } => {
            if verbose {
                println!(
                    "Embeddenator v{} - Holographic Query (Text)",
                    env!("CARGO_PKG_VERSION")
                );
                println!("========================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;

            let config = ReversibleVSAConfig::default();
            let base_query = SparseVec::encode_data(text.as_bytes(), &config, None);

            let codebook_index = engram_data.build_codebook_index();

            let mut best_similarity = f64::MIN;
            let mut best_shift = 0usize;
            let mut best_top_cosine = f64::MIN;

            let mut merged: HashMap<usize, (f64, i32)> = HashMap::new();
            let mut merged_hier: HashMap<(String, usize), (f64, i32)> = HashMap::new();

            let hierarchical_loaded = if let (Some(hier_path), Some(_)) = (hierarchical_manifest.as_ref(), sub_engrams_dir.as_ref()) {
                Some(load_hierarchical_manifest(hier_path)?)
            } else {
                None
            };

            let k_sweep = (k.saturating_mul(10)).max(100);
            let candidate_k = (k_sweep.saturating_mul(10)).max(200);

            for depth in 0..config.max_path_depth.max(1) {
                let shift = depth * config.base_shift;
                let query_vec = base_query.permute(shift);

                let similarity = query_vec.cosine(&engram_data.root);
                if similarity > best_similarity {
                    best_similarity = similarity;
                    best_shift = shift;
                }

                let matches = engram_data.query_codebook_with_index(
                    &codebook_index,
                    &query_vec,
                    candidate_k,
                    k_sweep,
                );

                if let Some(top) = matches.first() {
                    if top.cosine > best_top_cosine {
                        best_top_cosine = top.cosine;
                        best_shift = shift;
                        best_similarity = similarity;
                    }
                }

                for m in matches {
                    let entry = merged.entry(m.id).or_insert((m.cosine, m.approx_score));
                    if m.cosine > entry.0 {
                        *entry = (m.cosine, m.approx_score);
                    }
                }
            }

            if let (Some(hierarchical), Some(sub_dir)) = (hierarchical_loaded.as_ref(), sub_engrams_dir.as_ref()) {
                let store = DirectorySubEngramStore::new(sub_dir);
                let bounds = HierarchicalQueryBounds {
                    k,
                    ..HierarchicalQueryBounds::default()
                };
                let query_vec = base_query.permute(best_shift);
                let hier_hits = query_hierarchical_codebook_with_store(
                    hierarchical,
                    &store,
                    &engram_data.codebook,
                    &query_vec,
                    &bounds,
                );
                for h in hier_hits {
                    let key = (h.sub_engram_id, h.chunk_id);
                    let entry = merged_hier.entry(key).or_insert((h.cosine, h.approx_score));
                    if h.cosine > entry.0 {
                        *entry = (h.cosine, h.approx_score);
                    }
                }
            }

            println!("Query text: {}", text);
            if verbose {
                println!(
                    "Best bucket-shift: {} (buckets 0..{})",
                    best_shift,
                    config.max_path_depth.saturating_sub(1)
                );
            }
            println!("Similarity to engram: {:.4}", best_similarity);

            let mut top_matches: Vec<(usize, f64, i32)> = merged
                .into_iter()
                .map(|(id, (cosine, approx))| (id, cosine, approx))
                .collect();
            top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_matches.truncate(k);

            if !top_matches.is_empty() {
                println!("Top codebook matches:");
                for (id, cosine, approx) in top_matches {
                    println!("  chunk {}  cosine {:.4}  approx_dot {}", id, cosine, approx);
                }
            } else if verbose {
                println!("Top codebook matches: (none)");
            }

            let mut top_hier: Vec<(String, usize, f64, i32)> = merged_hier
                .into_iter()
                .map(|((sub_id, chunk_id), (cosine, approx))| (sub_id, chunk_id, cosine, approx))
                .collect();
            top_hier.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            top_hier.truncate(k);

            if !top_hier.is_empty() {
                println!("Top hierarchical matches:");
                for (sub_id, chunk_id, cosine, approx) in top_hier {
                    println!("  sub {}  chunk {}  cosine {:.4}  approx_dot {}", sub_id, chunk_id, cosine, approx);
                }
            } else if verbose && hierarchical_manifest.is_some() {
                println!("Top hierarchical matches: (none)");
            }

            Ok(())
        }

        Commands::BundleHier {
            engram,
            manifest,
            out_hierarchical_manifest,
            out_sub_engrams_dir,
            max_level_sparsity,
            max_chunks_per_node,
            embed_sub_engrams,
            sub_engram_compression,
            sub_engram_compression_level,
            verbose,
        } => {
            if verbose {
                println!(
                    "Embeddenator v{} - Build Hierarchical Artifacts",
                    env!("CARGO_PKG_VERSION")
                );
                println!("=============================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;

            let mut fs = EmbrFS::new();
            fs.engram = engram_data;
            fs.manifest = manifest_data;

            let config = ReversibleVSAConfig::default();
            let mut hierarchical = fs.bundle_hierarchically_with_options(
                max_level_sparsity,
                max_chunks_per_node,
                verbose,
                &config,
            )?;

            // Always write the sub-engrams directory for store-backed retrieval.
            save_sub_engrams_dir_with_options(
                &hierarchical.sub_engrams,
                &out_sub_engrams_dir,
                BinaryWriteOptions {
                    codec: sub_engram_compression.into(),
                    level: sub_engram_compression_level,
                },
            )?;

            if !embed_sub_engrams {
                hierarchical.sub_engrams.clear();
            }

            save_hierarchical_manifest(&hierarchical, &out_hierarchical_manifest)?;

            if verbose {
                println!("Wrote hierarchical manifest: {}", out_hierarchical_manifest.display());
                println!("Wrote sub-engrams dir: {}", out_sub_engrams_dir.display());
            }

            Ok(())
        }

        #[cfg(feature = "fuse")]
        Commands::Mount {
            engram,
            manifest,
            mountpoint,
            allow_other,
            foreground: _foreground,
            verbose,
        } => {
            use crate::fuse_shim::{EngramFS, MountOptions, mount};
            use crate::fs::fs::embrfs::DEFAULT_CHUNK_SIZE;
            
            if verbose {
                println!(
                    "Embeddenator v{} - FUSE Mount",
                    env!("CARGO_PKG_VERSION")
                );
                println!("============================");
            }

            // Load engram and manifest
            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;
            let config = ReversibleVSAConfig::default();

            if verbose {
                println!("Loaded engram: {}", engram.display());
                println!("Loaded manifest: {} files", manifest_data.files.len());
            }

            // Production-hardening: build a metadata-only filesystem and decode chunks on-demand
            // during reads. This avoids preloading all file bytes into memory at mount time.
            let fuse_fs = EngramFS::from_engram(
                engram_data,
                manifest_data,
                config,
                DEFAULT_CHUNK_SIZE,
                true,
            );

            if verbose {
                println!("Populated {} files into FUSE filesystem", fuse_fs.file_count());
                println!("Total size: {} bytes", fuse_fs.total_size());
                println!("Mounting at: {}", mountpoint.display());
                println!();
            }

            // Verify mountpoint exists
            if !mountpoint.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Mountpoint does not exist: {}", mountpoint.display())
                ));
            }

            // Configure mount options
            let options = MountOptions {
                read_only: true,
                allow_other,
                allow_root: !allow_other,
                fsname: format!("engram:{}", engram.display()),
            };

            // Mount the filesystem (blocks until unmounted)
            println!("EngramFS mounted at {}", mountpoint.display());
            println!("Use 'fusermount -u {}' to unmount", mountpoint.display());
            
            mount(fuse_fs, &mountpoint, options)?;

            if verbose {
                println!("\nUnmounted.");
            }

            Ok(())
        }
    }
}
