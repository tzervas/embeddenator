//! Query Files Example
//!
//! Demonstrates querying for similar files in an engram.
//!
//! Run with:
//!   cargo run --example query_files

use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use embeddenator_vsa::SparseVec;
use std::fs;

fn main() -> std::io::Result<()> {
    println!("=== Query Files Example ===\n");

    // Setup test directory
    let temp_dir = std::env::temp_dir().join("embeddenator_fs_example_query");
    let input_dir = temp_dir.join("input");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&input_dir)?;

    println!("Creating test corpus...");

    // Create different types of files
    fs::write(
        input_dir.join("rust_code.rs"),
        "fn main() {\n    println!(\"Hello, Rust!\");\n}\n",
    )?;

    fs::write(
        input_dir.join("python_code.py"),
        "def main():\n    print('Hello, Python!')\n\nif __name__ == '__main__':\n    main()\n",
    )?;

    fs::write(
        input_dir.join("text_doc.txt"),
        "This is a text document about Rust programming.\nRust is a systems programming language.",
    )?;

    fs::write(
        input_dir.join("binary_data.bin"),
        vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE],
    )?;

    fs::write(
        input_dir.join("similar_rust.rs"),
        "fn hello() {\n    println!(\"Hello, World!\");\n}\n",
    )?;

    println!("✓ Created 5 test files\n");

    // Ingest into engram
    println!("Ingesting files...");
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_directory(&input_dir, false, &config)?;

    let engram_path = temp_dir.join("query.engram");
    let manifest_path = temp_dir.join("query.json");

    fs.save_engram(&engram_path)?;
    fs.save_manifest(&manifest_path)?;

    println!("✓ Engram created with {} files\n", fs.manifest.files.len());

    // Load engram for querying
    let engram_data = EmbrFS::load_engram(&engram_path)?;
    let manifest_data = EmbrFS::load_manifest(&manifest_path)?;

    // Build codebook index for efficient querying
    let codebook_index = engram_data.build_codebook_index();

    // Query 1: Search for Rust code
    println!("=== Query 1: Similar to rust_code.rs ===");
    let query_data = fs::read(input_dir.join("rust_code.rs"))?;
    query_and_display(
        &engram_data,
        &manifest_data,
        &codebook_index,
        &query_data,
        &config,
    );

    // Query 2: Search for Python code (should have different results)
    println!("\n=== Query 2: Similar to python_code.py ===");
    let query_data = fs::read(input_dir.join("python_code.py"))?;
    query_and_display(
        &engram_data,
        &manifest_data,
        &codebook_index,
        &query_data,
        &config,
    );

    // Query 3: Search for binary data
    println!("\n=== Query 3: Similar to binary_data.bin ===");
    let query_data = fs::read(input_dir.join("binary_data.bin"))?;
    query_and_display(
        &engram_data,
        &manifest_data,
        &codebook_index,
        &query_data,
        &config,
    );

    println!("\n=== Example Complete ===");
    println!("Test files remain in: {}", temp_dir.display());

    Ok(())
}

fn query_and_display(
    engram_data: &embeddenator_fs::Engram,
    manifest_data: &embeddenator_fs::Manifest,
    codebook_index: &embeddenator_retrieval::TernaryInvertedIndex,
    query_data: &[u8],
    config: &ReversibleVSAConfig,
) {
    let base_query = SparseVec::encode_data(query_data, config, None);

    // Sweep across path-hash buckets
    let mut merged = std::collections::HashMap::new();
    let mut best_similarity = f64::MIN;

    for depth in 0..config.max_path_depth.max(1) {
        let shift = depth * config.base_shift;
        let query_vec = base_query.permute(shift);

        let similarity = query_vec.cosine(&engram_data.root);
        if similarity > best_similarity {
            best_similarity = similarity;
        }

        let matches = engram_data.query_codebook_with_index(codebook_index, &query_vec, 50, 20);

        for m in matches {
            let entry = merged.entry(m.id).or_insert(m.cosine);
            if m.cosine > *entry {
                *entry = m.cosine;
            }
        }
    }

    println!("Root similarity: {:.4}", best_similarity);

    // Get top 5 chunk matches
    let mut top_matches: Vec<_> = merged.into_iter().collect();
    top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    top_matches.truncate(5);

    println!("Top chunk matches:");
    for (chunk_id, cosine) in &top_matches {
        // Find which file(s) contain this chunk
        let files: Vec<&str> = manifest_data
            .files
            .iter()
            .filter(|f| f.chunks.contains(chunk_id))
            .map(|f| f.path.as_str())
            .collect();

        println!(
            "  {:.4} - chunk {} in: {}",
            cosine,
            chunk_id,
            files.join(", ")
        );
    }
}
