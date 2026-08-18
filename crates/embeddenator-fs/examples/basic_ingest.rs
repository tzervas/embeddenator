//! Basic Ingestion Example
//!
//! Demonstrates simple file ingestion into an engram.
//!
//! Run with:
//!   cargo run --example basic_ingest

use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use std::fs;

fn main() -> std::io::Result<()> {
    println!("=== Basic Ingestion Example ===\n");

    // Setup test directory
    let temp_dir = std::env::temp_dir().join("embeddenator_fs_example_basic");
    let input_dir = temp_dir.join("input");
    let output_dir = temp_dir.join("output");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&input_dir)?;
    fs::create_dir_all(&output_dir)?;

    println!("Creating test files in: {}", input_dir.display());

    // Create some test files
    fs::write(input_dir.join("hello.txt"), "Hello, EmbrFS!")?;
    fs::write(
        input_dir.join("data.bin"),
        vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    fs::write(
        input_dir.join("readme.md"),
        "# EmbrFS Example\n\nThis is a test file for holographic filesystem.",
    )?;

    // Create a subdirectory
    fs::create_dir_all(input_dir.join("subdir"))?;
    fs::write(input_dir.join("subdir/nested.txt"), "Nested file content")?;

    println!("✓ Created 4 test files\n");

    // Ingest directory into engram
    println!("Ingesting files into engram...");
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_directory(&input_dir, true, &config)?;

    println!();
    println!("Engram statistics:");
    println!("  Files: {}", fs.manifest.files.len());
    println!("  Total chunks: {}", fs.manifest.total_chunks);
    println!(
        "  Root vector sparsity: {}",
        fs.engram.root.pos.len() + fs.engram.root.neg.len()
    );

    // Save engram and manifest
    let engram_path = temp_dir.join("basic.engram");
    let manifest_path = temp_dir.join("basic.json");

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;

    println!("\n✓ Saved:");
    println!("  Engram: {}", engram_path.display());
    println!("  Manifest: {}", manifest_path.display());

    // Extract to verify
    println!("\nExtracting files to verify bit-perfect reconstruction...");
    let engram_data = EmbrFS::load_engram(&engram_path)?;
    let manifest_data = EmbrFS::load_manifest(&manifest_path)?;

    EmbrFS::extract(&engram_data, &manifest_data, &output_dir, true, &config)?;

    // Verify files match
    println!("\nVerifying reconstruction:");
    let original = fs::read_to_string(input_dir.join("hello.txt"))?;
    let reconstructed = fs::read_to_string(output_dir.join("hello.txt"))?;

    if original == reconstructed {
        println!("✓ Bit-perfect reconstruction verified!");
    } else {
        println!("✗ Reconstruction failed!");
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Reconstruction mismatch",
        ));
    }

    println!("\n=== Example Complete ===");
    println!("Test files remain in: {}", temp_dir.display());

    Ok(())
}
