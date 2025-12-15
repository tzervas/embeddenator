//! CLI interface for Embeddenator
//! 
//! Provides command-line interface for:
//! - Ingesting files/directories into engrams
//! - Extracting files from engrams
//! - Querying similarity

use crate::embrfs::EmbrFS;
use crate::vsa::SparseVec;
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "embeddenator")]
#[command(version = "0.2.0")]
#[command(about = "Holographic computing substrate using sparse ternary VSA")]
#[command(author = "Embeddenator Contributors")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Ingest files/directories into engram
    Ingest {
        /// Input directory to ingest
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output engram file (default: root.engram)
        #[arg(short, long, default_value = "root.engram")]
        engram: PathBuf,
        
        /// Output manifest file (default: manifest.json)
        #[arg(short, long, default_value = "manifest.json")]
        manifest: PathBuf,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Extract files from engram
    Extract {
        /// Input engram file (default: root.engram)
        #[arg(short, long, default_value = "root.engram")]
        engram: PathBuf,
        
        /// Input manifest file (default: manifest.json)
        #[arg(short, long, default_value = "manifest.json")]
        manifest: PathBuf,
        
        /// Output directory
        #[arg(short, long)]
        output_dir: PathBuf,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Query engram similarity
    Query {
        /// Engram file to query (default: root.engram)
        #[arg(short, long, default_value = "root.engram")]
        engram: PathBuf,
        
        /// Query file or pattern
        #[arg(short, long)]
        query: PathBuf,
        
        /// Verbose output
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
                println!("Embeddenator v0.1.0 - Holographic Ingestion");
                println!("=====================================");
            }

            let mut fs = EmbrFS::new();
            fs.ingest_directory(&input, verbose)?;

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
                println!("Embeddenator v0.1.0 - Holographic Extraction");
                println!("======================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;

            EmbrFS::extract(&engram_data, &manifest_data, &output_dir, verbose)?;

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
                println!("Embeddenator v0.1.0 - Holographic Query");
                println!("=================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;

            let mut query_file = File::open(&query)?;
            let mut query_data = Vec::new();
            query_file.read_to_end(&mut query_data)?;

            let query_vec = SparseVec::from_data(&query_data);
            let similarity = query_vec.cosine(&engram_data.root);

            println!("Query file: {}", query.display());
            println!("Similarity to engram: {:.4}", similarity);

            if similarity > 0.75 {
                println!("Status: STRONG MATCH");
            } else if similarity > 0.3 {
                println!("Status: Partial match");
            } else {
                println!("Status: No significant match");
            }

            Ok(())
        }
    }
}
