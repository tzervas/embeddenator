//! CLI interface for Embeddenator
//!
//! Provides command-line interface for:
//! - Ingesting files/directories into engrams
//! - Extracting files from engrams
//! - Querying similarity
//! - Mounting engrams as FUSE filesystems (requires `fuse` feature)

use crate::embrfs::EmbrFS;
use crate::vsa::{SparseVec, ReversibleVSAConfig};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "embeddenator")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Holographic computing substrate using sparse ternary VSA")]
#[command(
    long_about = "Embeddenator - A production-grade holographic computing substrate using Vector Symbolic Architecture (VSA)\n\n\
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
#[command(author = "Embeddenator Contributors")]
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
        /// Input directory to ingest (will recursively process all files)
        #[arg(short, long, value_name = "DIR", help_heading = "Required")]
        input: PathBuf,

        /// Output engram file containing holographic encoding
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

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

        /// Query file or pattern to search for
        #[arg(short, long, value_name = "FILE", help_heading = "Required")]
        query: PathBuf,

        /// Enable verbose output showing similarity scores and details
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
            fs.ingest_directory(&input, verbose, &config)?;

            fs.save_engram(&engram)?;
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

            let query_vec = SparseVec::encode_data(&query_data, &ReversibleVSAConfig::default(), None);
            let similarity = query_vec.cosine(&engram_data.root);

            println!("Query file: {}", query.display());
            println!("Similarity to engram: {:.4}", similarity);

            let top_matches = engram_data.query_codebook(&query_vec, 10);
            if !top_matches.is_empty() {
                println!("Top codebook matches:");
                for m in top_matches {
                    println!(
                        "  chunk {}  cosine {:.4}  approx_dot {}",
                        m.id, m.cosine, m.approx_score
                    );
                }
            } else if verbose {
                println!("Top codebook matches: (none)");
            }

            if similarity > 0.75 {
                println!("Status: STRONG MATCH");
            } else if similarity > 0.3 {
                println!("Status: Partial match");
            } else {
                println!("Status: No significant match");
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
            use crate::embrfs::DEFAULT_CHUNK_SIZE;
            
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

            // Create FUSE filesystem and populate with decoded files
            let fuse_fs = EngramFS::new(true);
            
            for file_entry in &manifest_data.files {
                // Decode file data using the same approach as EmbrFS::extract
                let mut reconstructed = Vec::new();
                
                for &chunk_id in &file_entry.chunks {
                    if let Some(chunk_vec) = engram_data.codebook.get(&chunk_id) {
                        // Decode the sparse vector to bytes
                        // IMPORTANT: Use the same path as during encoding for correct shift calculation
                        let decoded = chunk_vec.decode_data(&config, Some(&file_entry.path), DEFAULT_CHUNK_SIZE);
                        
                        // Apply correction to guarantee bit-perfect reconstruction
                        let chunk_data = if let Some(corrected) = engram_data.corrections.apply(chunk_id as u64, &decoded) {
                            corrected
                        } else {
                            // No correction found - use decoded directly
                            decoded
                        };
                        
                        reconstructed.extend_from_slice(&chunk_data);
                    }
                }

                // Truncate to exact file size
                reconstructed.truncate(file_entry.size);
                
                // Add to FUSE filesystem
                if let Err(e) = fuse_fs.add_file(&file_entry.path, reconstructed) {
                    if verbose {
                        eprintln!("Warning: Failed to add {}: {}", file_entry.path, e);
                    }
                }
            }

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
