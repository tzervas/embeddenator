//! Ingest command implementation

use anyhow::Result;
use embeddenator_fs::embrfs::EmbrFS;
use embeddenator_vsa::ReversibleVSAConfig;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use crate::utils::logical_path_for_file_input;

pub fn handle_ingest(
    input: Vec<PathBuf>,
    engram: PathBuf,
    manifest: PathBuf,
    verbose: bool,
) -> Result<()> {
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
                anyhow::bail!("Input path does not exist: {}", p.display());
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
