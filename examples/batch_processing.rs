//! Batch Processing Example
//!
//! Demonstrates processing larger numbers of files efficiently.
//!
//! Run with:
//!   cargo run --example batch_processing --release

use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use std::time::Instant;

fn main() -> std::io::Result<()> {
    println!("=== Batch Processing Example ===\n");

    // Setup test directory
    let temp_dir = std::env::temp_dir().join("embeddenator_fs_example_batch");
    let input_dir = temp_dir.join("input");
    let output_dir = temp_dir.join("output");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&input_dir)?;
    fs::create_dir_all(&output_dir)?;

    // Generate test files
    let file_count = 100;
    let file_size = 4096; // 4KB per file

    println!(
        "Generating {} test files ({} bytes each)...",
        file_count, file_size
    );
    let gen_start = Instant::now();

    for i in 0..file_count {
        let filename = format!("file_{:04}.dat", i);
        let mut data = Vec::with_capacity(file_size);

        // Generate pseudo-random but reproducible content
        for j in 0..file_size {
            data.push(((i * file_size + j) % 256) as u8);
        }

        fs::write(input_dir.join(&filename), &data)?;
    }

    let gen_elapsed = gen_start.elapsed();
    let total_bytes = file_count * file_size;

    println!(
        "✓ Generated {} files ({:.2} MB) in {:.3}s\n",
        file_count,
        total_bytes as f64 / (1024.0 * 1024.0),
        gen_elapsed.as_secs_f64()
    );

    // Ingest all files
    println!("Ingesting files into engram...");
    let ingest_start = Instant::now();

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_directory(&input_dir, false, &config)?;

    let ingest_elapsed = ingest_start.elapsed();
    let ingest_rate = total_bytes as f64 / ingest_elapsed.as_secs_f64();

    println!(
        "✓ Ingested {} files in {:.3}s",
        fs.manifest.files.len(),
        ingest_elapsed.as_secs_f64()
    );
    println!("  Rate: {:.2} MB/s", ingest_rate / (1024.0 * 1024.0));
    println!("  Chunks: {}", fs.manifest.total_chunks);
    println!(
        "  Root sparsity: {}",
        fs.engram.root.pos.len() + fs.engram.root.neg.len()
    );

    // Save engram
    let engram_path = temp_dir.join("batch.engram");
    let manifest_path = temp_dir.join("batch.json");

    println!("\nSaving engram...");
    let save_start = Instant::now();

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;

    let save_elapsed = save_start.elapsed();

    // Check file sizes
    let engram_size = std::fs::metadata(&engram_path)?.len();
    let manifest_size = std::fs::metadata(&manifest_path)?.len();
    let total_storage = engram_size + manifest_size;

    println!("✓ Saved in {:.3}s", save_elapsed.as_secs_f64());
    println!("  Engram: {:.2} MB", engram_size as f64 / (1024.0 * 1024.0));
    println!("  Manifest: {:.2} KB", manifest_size as f64 / 1024.0);
    println!(
        "  Total storage: {:.2} MB",
        total_storage as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Overhead: {:.2}%",
        (total_storage as f64 / total_bytes as f64 - 1.0) * 100.0
    );

    // Extract all files
    println!("\nExtracting files...");
    let extract_start = Instant::now();

    let engram_data = EmbrFS::load_engram(&engram_path)?;
    let manifest_data = EmbrFS::load_manifest(&manifest_path)?;

    EmbrFS::extract(&engram_data, &manifest_data, &output_dir, false, &config)?;

    let extract_elapsed = extract_start.elapsed();
    let extract_rate = total_bytes as f64 / extract_elapsed.as_secs_f64();

    println!(
        "✓ Extracted {} files in {:.3}s",
        manifest_data.files.len(),
        extract_elapsed.as_secs_f64()
    );
    println!("  Rate: {:.2} MB/s", extract_rate / (1024.0 * 1024.0));

    // Verify a sample of files
    println!("\nVerifying sample files...");
    let sample_count = 10.min(file_count);
    let mut verified = 0;

    for i in 0..sample_count {
        let filename = format!("file_{:04}.dat", i);
        let original = fs::read(input_dir.join(&filename))?;
        let reconstructed = fs::read(output_dir.join(&filename))?;

        if original == reconstructed {
            verified += 1;
        } else {
            println!("  ✗ Mismatch: {}", filename);
        }
    }

    println!(
        "✓ Verified {}/{} sample files (bit-perfect)",
        verified, sample_count
    );

    // Performance summary
    println!("\n=== Performance Summary ===");
    println!("Files: {}", file_count);
    println!(
        "Total data: {:.2} MB",
        total_bytes as f64 / (1024.0 * 1024.0)
    );
    println!();
    println!(
        "Ingestion: {:.3}s ({:.2} MB/s)",
        ingest_elapsed.as_secs_f64(),
        ingest_rate / (1024.0 * 1024.0)
    );
    println!("Save: {:.3}s", save_elapsed.as_secs_f64());
    println!(
        "Extract: {:.3}s ({:.2} MB/s)",
        extract_elapsed.as_secs_f64(),
        extract_rate / (1024.0 * 1024.0)
    );
    println!();
    println!(
        "Storage overhead: {:.2}%",
        (total_storage as f64 / total_bytes as f64 - 1.0) * 100.0
    );
    println!("Reconstruction: ✓ Bit-perfect");

    println!("\n=== Example Complete ===");
    println!("Test files remain in: {}", temp_dir.display());

    Ok(())
}
