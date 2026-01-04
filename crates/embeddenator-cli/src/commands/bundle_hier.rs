//! Bundle hierarchical artifacts command implementation

use anyhow::Result;
use embeddenator_fs::embrfs::{EmbrFS, save_hierarchical_manifest, save_sub_engrams_dir};
use embeddenator_vsa::ReversibleVSAConfig;
use std::path::PathBuf;

pub fn handle_bundle_hier(
    engram: PathBuf,
    manifest: PathBuf,
    out_hierarchical_manifest: PathBuf,
    out_sub_engrams_dir: PathBuf,
    max_level_sparsity: usize,
    max_chunks_per_node: Option<usize>,
    embed_sub_engrams: bool,
    verbose: bool,
) -> Result<()> {
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
    save_sub_engrams_dir(&hierarchical.sub_engrams, &out_sub_engrams_dir)?;

    if !embed_sub_engrams {
        hierarchical.sub_engrams.clear();
    }

    save_hierarchical_manifest(&hierarchical, &out_hierarchical_manifest)?;

    if verbose {
        println!(
            "Wrote hierarchical manifest: {}",
            out_hierarchical_manifest.display()
        );
        println!("Wrote sub-engrams dir: {}", out_sub_engrams_dir.display());
    }

    Ok(())
}
