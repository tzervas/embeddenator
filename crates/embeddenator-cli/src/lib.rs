//! Embeddenator CLI - Command-line interface for holographic computing substrate
//!
//! This library provides a modular CLI for Embeddenator operations including:
//! - Ingesting files/directories into engrams
//! - Extracting files from engrams
//! - Querying similarity
//! - Mounting engrams as FUSE filesystems (requires `fuse` feature)
//! - Incremental update operations

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod commands;
pub mod utils;

/// Embeddenator CLI main structure
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

    /// Incremental update operations (add/remove/modify files)
    #[command(
        long_about = "Perform incremental updates to an existing engram\n\n\
        This command enables efficient updates to engrams without full re-ingestion.\n\
        Use subcommands to add, remove, or modify files, or to compact the engram.\n\n\
        Subcommands:\n\
        • add     - Add a new file to the engram\n\
        • remove  - Mark a file as deleted\n\
        • modify  - Update an existing file\n\
        • compact - Rebuild engram without deleted files\n\n\
        Examples:\n\
          embeddenator update add -e data.engram -m data.json -f new.txt\n\
          embeddenator update remove -e data.engram -m data.json -p old.txt\n\
          embeddenator update modify -e data.engram -m data.json -f changed.txt\n\
          embeddenator update compact -e data.engram -m data.json"
    )]
    #[command(subcommand)]
    Update(UpdateCommands),
}

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// Add a new file to an existing engram
    #[command(
        long_about = "Add a new file to an existing engram without full re-ingestion\n\n\
        This operation bundles the new file's chunks with the existing root vector\n\
        using VSA's associative bundle operation. Much faster than full re-ingestion.\n\n\
        Example:\n\
          embeddenator update add -e data.engram -m data.json -f new_file.txt"
    )]
    Add {
        /// Engram file to update
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// File to add to the engram
        #[arg(short, long, value_name = "FILE", help_heading = "Required")]
        file: PathBuf,

        /// Logical path in engram (defaults to filename)
        #[arg(short = 'p', long, value_name = "PATH")]
        logical_path: Option<String>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove a file from the engram (mark as deleted)
    #[command(
        long_about = "Mark a file as deleted in the engram manifest\n\n\
        This operation marks the file as deleted without modifying the root vector,\n\
        since VSA bundling has no clean inverse. Use 'compact' to truly remove chunks.\n\n\
        Example:\n\
          embeddenator update remove -e data.engram -m data.json -p old_file.txt"
    )]
    Remove {
        /// Engram file to update
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Logical path of file to remove
        #[arg(short = 'p', long, value_name = "PATH", help_heading = "Required")]
        path: String,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Modify an existing file in the engram
    #[command(
        long_about = "Update an existing file's content in the engram\n\n\
        This operation marks the old version as deleted and adds the new version.\n\
        Use 'compact' periodically to clean up old chunks.\n\n\
        Example:\n\
          embeddenator update modify -e data.engram -m data.json -f updated.txt"
    )]
    Modify {
        /// Engram file to update
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
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
    #[command(
        long_about = "Rebuild engram from scratch, excluding deleted files\n\n\
        This operation recreates the engram with only active files, reclaiming space\n\
        from deleted chunks. Expensive but necessary after many updates.\n\n\
        Example:\n\
          embeddenator update compact -e data.engram -m data.json -v"
    )]
    Compact {
        /// Engram file to compact
        #[arg(short, long, default_value = "root.engram", value_name = "FILE")]
        engram: PathBuf,

        /// Manifest file to update
        #[arg(short, long, default_value = "manifest.json", value_name = "FILE")]
        manifest: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Main entry point for the CLI
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            input,
            engram,
            manifest,
            verbose,
        } => commands::handle_ingest(input, engram, manifest, verbose),

        Commands::Extract {
            engram,
            manifest,
            output_dir,
            verbose,
        } => commands::handle_extract(engram, manifest, output_dir, verbose),

        Commands::Query {
            engram,
            query,
            hierarchical_manifest,
            sub_engrams_dir,
            k,
            verbose,
        } => commands::handle_query(
            engram,
            query,
            hierarchical_manifest,
            sub_engrams_dir,
            k,
            verbose,
        ),

        Commands::QueryText {
            engram,
            text,
            hierarchical_manifest,
            sub_engrams_dir,
            k,
            verbose,
        } => commands::handle_query_text(
            engram,
            text,
            hierarchical_manifest,
            sub_engrams_dir,
            k,
            verbose,
        ),

        Commands::BundleHier {
            engram,
            manifest,
            out_hierarchical_manifest,
            out_sub_engrams_dir,
            max_level_sparsity,
            max_chunks_per_node,
            embed_sub_engrams,
            verbose,
        } => commands::handle_bundle_hier(
            engram,
            manifest,
            out_hierarchical_manifest,
            out_sub_engrams_dir,
            max_level_sparsity,
            max_chunks_per_node,
            embed_sub_engrams,
            verbose,
        ),

        #[cfg(feature = "fuse")]
        Commands::Mount {
            engram,
            manifest,
            mountpoint,
            allow_other,
            foreground,
            verbose,
        } => commands::handle_mount(
            engram,
            manifest,
            mountpoint,
            allow_other,
            foreground,
            verbose,
        ),

        Commands::Update(update_cmd) => match update_cmd {
            UpdateCommands::Add {
                engram,
                manifest,
                file,
                logical_path,
                verbose,
            } => commands::handle_update_add(engram, manifest, file, logical_path, verbose),

            UpdateCommands::Remove {
                engram,
                manifest,
                path,
                verbose,
            } => commands::handle_update_remove(engram, manifest, path, verbose),

            UpdateCommands::Modify {
                engram,
                manifest,
                file,
                logical_path,
                verbose,
            } => commands::handle_update_modify(engram, manifest, file, logical_path, verbose),

            UpdateCommands::Compact {
                engram,
                manifest,
                verbose,
            } => commands::handle_update_compact(engram, manifest, verbose),
        },
    }
}
