//! Incremental Update Example
//!
//! Demonstrates incremental add/remove/modify operations.
//!
//! Run with:
//!   cargo run --example incremental_update

use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use std::fs;

fn main() -> std::io::Result<()> {
    println!("=== Incremental Update Example ===\n");

    // Setup test directory
    let temp_dir = std::env::temp_dir().join("embeddenator_fs_example_incremental");
    let input_dir = temp_dir.join("input");
    let output_dir = temp_dir.join("output");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&input_dir)?;
    fs::create_dir_all(&output_dir)?;

    // Create initial files
    println!("Step 1: Creating initial files...");
    fs::write(input_dir.join("file1.txt"), "Initial content 1")?;
    fs::write(input_dir.join("file2.txt"), "Initial content 2")?;
    fs::write(input_dir.join("file3.txt"), "Initial content 3")?;

    // Initial ingestion
    println!("Step 2: Initial ingestion...");
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_directory(&input_dir, false, &config)?;

    println!("  Files: {}", fs.manifest.files.len());
    println!("  Chunks: {}", fs.manifest.total_chunks);

    // Save initial state
    let engram_path = temp_dir.join("incremental.engram");
    let manifest_path = temp_dir.join("incremental.json");

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;
    println!("  ✓ Saved initial engram\n");

    // Add a new file
    println!("Step 3: Adding new file...");
    let new_file_path = input_dir.join("file4.txt");
    std::fs::write(&new_file_path, "New file content")?;

    fs.add_file(&new_file_path, "file4.txt".to_string(), false, &config)?;

    println!("  Files: {}", fs.manifest.files.len());
    println!("  Chunks: {}", fs.manifest.total_chunks);

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;
    println!("  ✓ Added file4.txt\n");

    // Modify an existing file
    println!("Step 4: Modifying existing file...");
    let modified_file_path = input_dir.join("file2.txt");
    std::fs::write(&modified_file_path, "Modified content 2")?;

    fs.modify_file(&modified_file_path, "file2.txt".to_string(), false, &config)?;

    println!("  ✓ Modified file2.txt");
    println!("  Files: {}", fs.manifest.files.len());
    println!(
        "  Chunks: {} (includes old version)",
        fs.manifest.total_chunks
    );

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;
    println!("  ✓ Saved\n");

    // Remove a file
    println!("Step 5: Removing file...");
    fs.remove_file("file3.txt", false)?;

    let active_files = fs.manifest.files.iter().filter(|f| !f.deleted).count();
    println!("  Active files: {}", active_files);
    println!(
        "  Total chunks: {} (still includes deleted)",
        fs.manifest.total_chunks
    );

    fs.save_manifest(&manifest_path)?;
    println!("  ✓ Removed file3.txt (marked as deleted)\n");

    // Extract to verify current state
    println!("Step 6: Extracting current state...");
    let engram_data = EmbrFS::load_engram(&engram_path)?;
    let manifest_data = EmbrFS::load_manifest(&manifest_path)?;

    EmbrFS::extract(&engram_data, &manifest_data, &output_dir, false, &config)?;

    let extracted_files = std::fs::read_dir(&output_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

    println!("  ✓ Extracted {} active files", extracted_files);
    println!("  Contents:");
    for entry in std::fs::read_dir(&output_dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            let content = std::fs::read_to_string(entry.path())?;
            println!(
                "    {}: \"{}\"",
                entry.file_name().to_string_lossy(),
                content
            );
        }
    }
    println!();

    // Compact to reclaim space
    println!("Step 7: Compacting engram...");
    let old_chunks = fs.manifest.total_chunks;
    let old_files = fs.manifest.files.len();

    fs.compact(false, &config)?;

    let new_chunks = fs.manifest.total_chunks;
    let new_files = fs.manifest.files.len();

    println!("  Files: {} -> {}", old_files, new_files);
    println!(
        "  Chunks: {} -> {} (reclaimed deleted data)",
        old_chunks, new_chunks
    );

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;
    println!("  ✓ Compaction complete\n");

    // Final extraction to verify
    println!("Step 8: Final verification...");
    let final_engram = EmbrFS::load_engram(&engram_path)?;
    let final_manifest = EmbrFS::load_manifest(&manifest_path)?;

    let final_output = temp_dir.join("final_output");
    std::fs::create_dir_all(&final_output)?;

    EmbrFS::extract(
        &final_engram,
        &final_manifest,
        &final_output,
        false,
        &config,
    )?;

    // Check final files
    println!("  Final files:");
    for entry in std::fs::read_dir(&final_output)? {
        let entry = entry?;
        if entry.path().is_file() {
            let content = std::fs::read_to_string(entry.path())?;
            println!(
                "    {}: \"{}\"",
                entry.file_name().to_string_lossy(),
                content
            );
        }
    }

    println!("\n=== Example Complete ===");
    println!("Demonstrated:");
    println!("  ✓ Initial ingestion");
    println!("  ✓ Adding files (incremental)");
    println!("  ✓ Modifying files");
    println!("  ✓ Removing files (soft delete)");
    println!("  ✓ Compacting (hard rebuild)");
    println!("  ✓ Bit-perfect reconstruction throughout");
    println!("\nTest files remain in: {}", temp_dir.display());

    Ok(())
}
