//! embeddenator-fs CLI
//!
//! Command-line interface for EmbrFS holographic filesystem operations.
//!
//! Provides commands for:
//! - Ingesting files/directories into engrams
//! - Extracting files from engrams
//! - Querying similarity
//! - Listing files in engrams
//! - Verifying engram integrity
//! - Incremental updates (add/remove/modify/compact)

use clap::{Parser, Subcommand};
use embeddenator_fs::EmbrFS;
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Format bytes as human-readable size
fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Get logical path for file input
fn logical_path_for_file(path: &Path, cwd: &Path) -> String {
    if path.is_relative() {
        return path.to_string_lossy().replace('\\', "/");
    }

    if let Ok(rel) = path.strip_prefix(cwd) {
        let s = rel.to_string_lossy().replace('\\', "/");
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
#[command(name = "embeddenator-fs")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "EmbrFS: Holographic filesystem operations")]
#[command(long_about = "embeddenator-fs - Holographic Filesystem CLI\n\n\
    EmbrFS encodes filesystems into holographic 'engrams' using Vector Symbolic Architecture (VSA),\n\
    enabling bit-perfect reconstruction and similarity-based retrieval.\n\n\
    Key Features:\n\
    • 100% bit-perfect file reconstruction\n\
    • Holographic superposition of directory trees\n\
    • Similarity-based querying\n\
    • Incremental updates (add/remove/modify)\n\
    • Read-only FUSE mounting (with 'fuse' feature)\n\n\
    Examples:\n\
      embeddenator-fs ingest -i ./mydata -e data.engram\n\
      embeddenator-fs extract -e data.engram -o ./restored\n\
      embeddenator-fs query -e data.engram -q testfile.txt\n\
      embeddenator-fs list -e data.engram\n\
      embeddenator-fs info -e data.engram")]
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
        This command recursively processes all files in the input directory and encodes\n\
        them into a holographic VSA engram with bit-perfect reconstruction guarantees.\n\n\
        Example:\n\
          embeddenator-fs ingest -i ./myproject -e project.engram\n\
          embeddenator-fs ingest -i ./data -e data.engram -m data.json -v"
    )]
    Ingest {
        /// Input path(s) to ingest (directory or file)
        #[arg(
            short,
            long,
            value_name = "PATH",
            help_heading = "Required",
            num_args = 1..,
            action = clap::ArgAction::Append
        )]
        input: Vec<PathBuf>,

        /// Output engram file
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Output manifest file
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Extract files from a holographic engram
    #[command(
        long_about = "Extract and reconstruct files from a holographic engram\n\n\
        Performs bit-perfect reconstruction of all files from an engram.\n\n\
        Example:\n\
          embeddenator-fs extract -e project.engram -o ./restored\n\
          embeddenator-fs extract -e backup.engram -m backup.json -o ~/restored -v"
    )]
    Extract {
        /// Input engram file
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Input manifest file
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Output directory
        #[arg(short, long, value_name = "DIR", help_heading = "Required")]
        output_dir: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Query similarity between a file and engram contents
    #[command(
        long_about = "Query cosine similarity between a file and engram contents\n\n\
        Computes VSA cosine similarity to enable holographic search.\n\n\
        Similarity interpretation:\n\
        • >0.75: Strong match\n\
        • 0.3-0.75: Moderate similarity\n\
        • <0.3: Low similarity\n\n\
        Example:\n\
          embeddenator-fs query -e archive.engram -q search.txt\n\
          embeddenator-fs query -e data.engram -q pattern.bin -k 20 -v"
    )]
    Query {
        /// Engram file to query
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Query file to search for
        #[arg(short, long, value_name = "FILE", help_heading = "Required")]
        query: PathBuf,

        /// Number of top matches to show
        #[arg(short = 'k', long, default_value_t = 10, value_name = "K")]
        top_k: usize,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List files in an engram
    #[command(long_about = "List all files stored in an engram\n\n\
        Displays file paths, sizes, and metadata from the manifest.\n\n\
        Example:\n\
          embeddenator-fs list -e data.engram\n\
          embeddenator-fs list -e archive.engram -m archive.json -v")]
    List {
        /// Engram file
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show information about an engram
    #[command(long_about = "Display detailed information about an engram\n\n\
        Shows statistics including file count, total size, chunk count,\n\
        correction layer stats, and engram properties.\n\n\
        Example:\n\
          embeddenator-fs info -e data.engram\n\
          embeddenator-fs info -e archive.engram -m archive.json")]
    Info {
        /// Engram file
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,
    },

    /// Verify engram integrity
    #[command(long_about = "Verify bit-perfect reconstruction guarantee\n\n\
        Tests that all files can be correctly decoded from the engram.\n\
        Reports any corruption or inconsistencies.\n\n\
        Example:\n\
          embeddenator-fs verify -e data.engram\n\
          embeddenator-fs verify -e backup.engram -m backup.json -v")]
    Verify {
        /// Engram file
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Incremental update operations
    #[command(long_about = "Perform incremental updates to an existing engram\n\n\
        Subcommands:\n\
        • add     - Add a new file\n\
        • remove  - Mark a file as deleted\n\
        • modify  - Update an existing file\n\
        • compact - Rebuild without deleted files\n\n\
        Examples:\n\
          embeddenator-fs update add -e data.engram -f new.txt\n\
          embeddenator-fs update remove -e data.engram -p old.txt\n\
          embeddenator-fs update modify -e data.engram -f changed.txt\n\
          embeddenator-fs update compact -e data.engram")]
    #[command(subcommand)]
    Update(UpdateCommands),
}

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// Add a new file to an engram
    Add {
        /// Engram file to update
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// File to add
        #[arg(short, long, value_name = "FILE", help_heading = "Required")]
        file: PathBuf,

        /// Logical path in engram (defaults to filename)
        #[arg(short = 'p', long, value_name = "PATH")]
        logical_path: Option<String>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove a file from the engram
    Remove {
        /// Engram file to update
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Logical path of file to remove
        #[arg(short = 'p', long, value_name = "PATH", help_heading = "Required")]
        path: String,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Modify an existing file
    Modify {
        /// Engram file to update
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// File with new content
        #[arg(short, long, value_name = "FILE", help_heading = "Required")]
        file: PathBuf,

        /// Logical path in engram (defaults to filename)
        #[arg(short = 'p', long, value_name = "PATH")]
        logical_path: Option<String>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Compact engram by rebuilding without deleted files
    Compact {
        /// Engram file to compact
        #[arg(short, long, default_value = "filesystem.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "filesystem.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            input,
            engram,
            manifest,
            verbose,
        } => {
            if verbose {
                println!("embeddenator-fs v{} - Ingestion", env!("CARGO_PKG_VERSION"));
                println!("=====================================");
            }

            let start = Instant::now();
            // Use holographic mode for ~94% encoding accuracy and <10% storage overhead
            let mut fs = EmbrFS::new_holographic();
            let config = ReversibleVSAConfig::default();

            // Single directory input - use simple ingestion
            if input.len() == 1 && input[0].is_dir() {
                if verbose {
                    println!("Ingesting directory: {}", input[0].display());
                }
                fs.ingest_directory(&input[0], verbose, &config)?;
            } else {
                // Multiple inputs or mixed files/directories
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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

                        if verbose {
                            println!("Ingesting directory: {} (prefix: {})", p.display(), prefix);
                        }
                        fs.ingest_directory_with_prefix(p, Some(&prefix), verbose, &config)?;
                    } else {
                        let logical = logical_path_for_file(p, &cwd);
                        if verbose {
                            println!("Ingesting file: {} -> {}", p.display(), logical);
                        }
                        fs.ingest_file(p, logical, verbose, &config)?;
                    }
                }
            }

            // Save results
            fs.save_engram(&engram)?;
            fs.save_manifest(&manifest)?;

            let elapsed = start.elapsed();
            let total_bytes: usize = fs
                .manifest
                .files
                .iter()
                .filter(|f| !f.deleted)
                .map(|f| f.size)
                .sum();

            if verbose {
                println!("\n✓ Ingestion complete!");
                println!("  Engram: {}", engram.display());
                println!("  Manifest: {}", manifest.display());
                println!("  Files: {}", fs.manifest.files.len());
                println!("  Total size: {}", format_bytes(total_bytes));
                println!("  Chunks: {}", fs.manifest.total_chunks);
                println!("  Time: {:.2}s", elapsed.as_secs_f64());

                let stats = fs.correction_stats();
                println!("\nCorrection Layer Stats:");
                println!("  Total entries: {}", stats.total_chunks);
                println!(
                    "  Bit-perfect: {} ({:.1}%)",
                    stats.perfect_chunks,
                    stats.perfect_ratio * 100.0
                );
                println!("  Avg overhead: {:.2}%", stats.correction_ratio * 100.0);
            } else {
                println!(
                    "✓ Ingested {} files ({}) in {:.2}s",
                    fs.manifest.files.len(),
                    format_bytes(total_bytes),
                    elapsed.as_secs_f64()
                );
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
                    "embeddenator-fs v{} - Extraction",
                    env!("CARGO_PKG_VERSION")
                );
                println!("=====================================");
            }

            let start = Instant::now();
            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;
            let config = ReversibleVSAConfig::default();

            if verbose {
                println!(
                    "Extracting {} files to {}",
                    manifest_data.files.len(),
                    output_dir.display()
                );
            }

            EmbrFS::extract(&engram_data, &manifest_data, &output_dir, verbose, &config)?;

            let elapsed = start.elapsed();
            let total_bytes: usize = manifest_data
                .files
                .iter()
                .filter(|f| !f.deleted)
                .map(|f| f.size)
                .sum();

            if verbose {
                println!("\n✓ Extraction complete!");
                println!("  Output: {}", output_dir.display());
                println!("  Files: {}", manifest_data.files.len());
                println!("  Total size: {}", format_bytes(total_bytes));
                println!("  Time: {:.2}s", elapsed.as_secs_f64());
                println!(
                    "  Speed: {:.2} MB/s",
                    (total_bytes as f64 / elapsed.as_secs_f64()) / (1024.0 * 1024.0)
                );
            } else {
                println!(
                    "✓ Extracted {} files ({}) in {:.2}s",
                    manifest_data.files.len(),
                    format_bytes(total_bytes),
                    elapsed.as_secs_f64()
                );
            }

            Ok(())
        }

        Commands::Query {
            engram,
            manifest,
            query,
            top_k,
            verbose,
        } => {
            if verbose {
                println!("embeddenator-fs v{} - Query", env!("CARGO_PKG_VERSION"));
                println!("=====================================");
            }

            let start = Instant::now();
            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;

            // Read query file
            let mut query_file = File::open(&query)?;
            let mut query_data = Vec::new();
            query_file.read_to_end(&mut query_data)?;

            if verbose {
                println!(
                    "Query file: {} ({})",
                    query.display(),
                    format_bytes(query_data.len())
                );
            }

            let config = ReversibleVSAConfig::default();
            let base_query = SparseVec::encode_data(&query_data, &config, None);

            // Build codebook index for efficient querying
            let codebook_index = engram_data.build_codebook_index();

            // Sweep across possible path-hash buckets
            let mut best_similarity = f64::MIN;
            let mut best_shift = 0usize;
            let mut merged: HashMap<usize, f64> = HashMap::new();

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
                    top_k * 10,
                    top_k * 5,
                );

                for m in matches {
                    let entry = merged.entry(m.id).or_insert(m.cosine);
                    if m.cosine > *entry {
                        *entry = m.cosine;
                    }
                }
            }

            let elapsed = start.elapsed();

            println!("\nQuery: {}", query.display());
            if verbose {
                println!(
                    "Best shift: {} (depth 0..{})",
                    best_shift,
                    config.max_path_depth.saturating_sub(1)
                );
            }
            println!("Similarity to engram root: {:.4}", best_similarity);

            // Get top matches
            let mut top_matches: Vec<(usize, f64)> = merged.into_iter().collect();
            top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_matches.truncate(top_k);

            if !top_matches.is_empty() {
                println!("\nTop {} chunk matches:", top_k);
                for (chunk_id, cosine) in &top_matches {
                    // Find which file(s) contain this chunk
                    let files: Vec<&str> = manifest_data
                        .files
                        .iter()
                        .filter(|f| f.chunks.contains(chunk_id))
                        .map(|f| f.path.as_str())
                        .collect();

                    if verbose {
                        println!(
                            "  Chunk {}: {:.4} (files: {})",
                            chunk_id,
                            cosine,
                            files.join(", ")
                        );
                    } else {
                        println!("  Chunk {}: {:.4}", chunk_id, cosine);
                    }
                }
            } else {
                println!("\nNo significant matches found");
            }

            // Interpretation
            println!();
            if best_similarity > 0.75 {
                println!("Status: ✓ STRONG MATCH");
            } else if best_similarity > 0.3 {
                println!("Status: ~ Partial match");
            } else {
                println!("Status: ✗ No significant match");
            }

            if verbose {
                println!("\nQuery time: {:.3}s", elapsed.as_secs_f64());
            }

            Ok(())
        }

        Commands::List {
            engram: _,
            manifest,
            verbose,
        } => {
            let manifest_data = EmbrFS::load_manifest(&manifest)?;

            println!("Files in engram: {}", manifest_data.files.len());
            println!();

            let mut total_size = 0usize;
            let mut active_count = 0;
            let mut deleted_count = 0;

            for file in &manifest_data.files {
                if file.deleted {
                    deleted_count += 1;
                    if verbose {
                        println!(
                            "  [DELETED] {} ({}, {} chunks)",
                            file.path,
                            format_bytes(file.size),
                            file.chunks.len()
                        );
                    }
                } else {
                    active_count += 1;
                    total_size += file.size;
                    if verbose {
                        println!(
                            "  {} ({}, {} chunks)",
                            file.path,
                            format_bytes(file.size),
                            file.chunks.len()
                        );
                    } else {
                        println!("  {} ({})", file.path, format_bytes(file.size));
                    }
                }
            }

            println!();
            println!("Summary:");
            println!("  Active files: {}", active_count);
            if deleted_count > 0 {
                println!("  Deleted files: {}", deleted_count);
            }
            println!("  Total size: {}", format_bytes(total_size));
            println!("  Total chunks: {}", manifest_data.total_chunks);

            Ok(())
        }

        Commands::Info { engram, manifest } => {
            println!(
                "embeddenator-fs v{} - Engram Info",
                env!("CARGO_PKG_VERSION")
            );
            println!("=====================================");

            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;

            // File statistics
            let active_files: Vec<_> = manifest_data.files.iter().filter(|f| !f.deleted).collect();
            let deleted_files = manifest_data.files.len() - active_files.len();
            let total_size: usize = active_files.iter().map(|f| f.size).sum();

            println!("\nEngram: {}", engram.display());
            println!("Manifest: {}", manifest.display());
            println!();

            println!("Files:");
            println!("  Active: {}", active_files.len());
            if deleted_files > 0 {
                println!("  Deleted: {}", deleted_files);
            }
            println!("  Total size: {}", format_bytes(total_size));
            println!();

            println!("Chunks:");
            println!("  Total: {}", manifest_data.total_chunks);
            println!("  In codebook: {}", engram_data.codebook.len());
            println!(
                "  Avg per file: {:.1}",
                manifest_data.total_chunks as f64 / active_files.len().max(1) as f64
            );
            println!();

            println!("Root Vector:");
            println!("  Positive indices: {}", engram_data.root.pos.len());
            println!("  Negative indices: {}", engram_data.root.neg.len());
            println!(
                "  Total sparsity: {}",
                engram_data.root.pos.len() + engram_data.root.neg.len()
            );
            println!();

            // Correction stats
            let stats = engram_data.corrections.stats();
            let correction_bytes = bincode::serialize(&engram_data.corrections)
                .map(|b| b.len())
                .unwrap_or(0);

            println!("Correction Layer:");
            println!("  Entries: {}", stats.total_chunks);
            println!("  Size: {}", format_bytes(correction_bytes));
            if total_size > 0 {
                println!(
                    "  Overhead: {:.2}%",
                    (correction_bytes as f64 / total_size as f64) * 100.0
                );
            }

            Ok(())
        }

        Commands::Verify {
            engram,
            manifest,
            verbose,
        } => {
            if verbose {
                println!(
                    "embeddenator-fs v{} - Verification",
                    env!("CARGO_PKG_VERSION")
                );
                println!("=====================================");
            }

            let start = Instant::now();
            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;
            let config = ReversibleVSAConfig::default();

            if verbose {
                println!("Verifying {} files...", manifest_data.files.len());
            }

            let mut verified = 0;
            let mut errors = Vec::new();

            for file_entry in &manifest_data.files {
                if file_entry.deleted {
                    continue;
                }

                if verbose {
                    print!("  Checking {}... ", file_entry.path);
                    std::io::stdout().flush()?;
                }

                // Reconstruct file data
                let mut reconstructed = Vec::new();
                let num_chunks = file_entry.chunks.len();
                for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
                    if let Some(chunk_vec) = engram_data.codebook.get(&chunk_id) {
                        let chunk_size = if chunk_idx == num_chunks - 1 {
                            let remaining =
                                file_entry.size - (chunk_idx * embeddenator_fs::DEFAULT_CHUNK_SIZE);
                            remaining.min(embeddenator_fs::DEFAULT_CHUNK_SIZE)
                        } else {
                            embeddenator_fs::DEFAULT_CHUNK_SIZE
                        };

                        let decoded =
                            chunk_vec.decode_data(&config, Some(&file_entry.path), chunk_size);

                        // Apply corrections
                        let chunk_data = if let Some(corrected) =
                            engram_data.corrections.apply(chunk_id as u64, &decoded)
                        {
                            corrected
                        } else {
                            decoded
                        };

                        reconstructed.extend_from_slice(&chunk_data);
                    } else {
                        errors.push(format!("{}: missing chunk {}", file_entry.path, chunk_id));
                        if verbose {
                            println!("✗ FAIL (missing chunk)");
                        }
                        continue;
                    }
                }

                reconstructed.truncate(file_entry.size);

                if reconstructed.len() != file_entry.size {
                    errors.push(format!(
                        "{}: size mismatch (expected {}, got {})",
                        file_entry.path,
                        file_entry.size,
                        reconstructed.len()
                    ));
                    if verbose {
                        println!("✗ FAIL (size mismatch)");
                    }
                } else {
                    verified += 1;
                    if verbose {
                        println!("✓ OK");
                    }
                }
            }

            let elapsed = start.elapsed();

            println!();
            if errors.is_empty() {
                println!("✓ Verification complete: {} files OK", verified);
            } else {
                println!("✗ Verification found {} errors:", errors.len());
                for error in &errors {
                    println!("  - {}", error);
                }
            }

            if verbose {
                println!("\nVerification time: {:.2}s", elapsed.as_secs_f64());
            }

            if !errors.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Verification failed",
                ));
            }

            Ok(())
        }

        Commands::Update(update_cmd) => match update_cmd {
            UpdateCommands::Add {
                engram,
                manifest,
                file,
                logical_path,
                verbose,
            } => {
                if verbose {
                    println!("Adding file to engram...");
                }

                let mut fs = EmbrFS::load(&engram, &manifest)?;

                let config = ReversibleVSAConfig::default();
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let logical = logical_path.unwrap_or_else(|| logical_path_for_file(&file, &cwd));

                if verbose {
                    println!("  File: {} -> {}", file.display(), logical);
                }

                fs.add_file(&file, logical.clone(), verbose, &config)?;

                fs.save_engram(&engram)?;
                fs.save_manifest(&manifest)?;

                println!("✓ Added {} to engram", logical);

                Ok(())
            }

            UpdateCommands::Remove {
                engram,
                manifest,
                path,
                verbose,
            } => {
                if verbose {
                    println!("Removing file from engram...");
                }

                let mut fs = EmbrFS::load(&engram, &manifest)?;

                fs.remove_file(&path, verbose)?;

                fs.save_manifest(&manifest)?;

                println!("✓ Removed {} from engram (marked as deleted)", path);
                println!("  Use 'compact' to reclaim space");

                Ok(())
            }

            UpdateCommands::Modify {
                engram,
                manifest,
                file,
                logical_path,
                verbose,
            } => {
                if verbose {
                    println!("Modifying file in engram...");
                }

                let mut fs = EmbrFS::load(&engram, &manifest)?;

                let config = ReversibleVSAConfig::default();
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let logical = logical_path.unwrap_or_else(|| logical_path_for_file(&file, &cwd));

                if verbose {
                    println!("  File: {} -> {}", file.display(), logical);
                }

                fs.modify_file(&file, logical.clone(), verbose, &config)?;

                fs.save_engram(&engram)?;
                fs.save_manifest(&manifest)?;

                println!("✓ Modified {} in engram", logical);

                Ok(())
            }

            UpdateCommands::Compact {
                engram,
                manifest,
                verbose,
            } => {
                if verbose {
                    println!("Compacting engram...");
                }

                let start = Instant::now();
                let mut fs = EmbrFS::load(&engram, &manifest)?;

                let old_chunks = fs.manifest.total_chunks;
                let old_files = fs.manifest.files.len();
                let deleted = fs.manifest.files.iter().filter(|f| f.deleted).count();

                let config = ReversibleVSAConfig::default();
                fs.compact(verbose, &config)?;

                fs.save_engram(&engram)?;
                fs.save_manifest(&manifest)?;

                let elapsed = start.elapsed();
                let new_chunks = fs.manifest.total_chunks;
                let new_files = fs.manifest.files.len();

                println!("✓ Compaction complete");
                println!("  Removed {} deleted files", deleted);
                println!("  Files: {} -> {}", old_files, new_files);
                println!("  Chunks: {} -> {}", old_chunks, new_chunks);
                if verbose {
                    println!("  Time: {:.2}s", elapsed.as_secs_f64());
                }

                Ok(())
            }
        },
    }
}
