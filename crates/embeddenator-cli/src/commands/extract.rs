//! Extract command implementation

use anyhow::Result;
use embeddenator_fs::embrfs::EmbrFS;
use embeddenator_vsa::ReversibleVSAConfig;
use std::path::PathBuf;

pub fn handle_extract(
    engram: PathBuf,
    manifest: PathBuf,
    output_dir: PathBuf,
    verbose: bool,
) -> Result<()> {
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
