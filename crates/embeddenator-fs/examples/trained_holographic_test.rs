//! Trained Holographic Storage Test
//!
//! This test:
//! 1. Trains the codebook on representative data (ebooks)
//! 2. Tests reconstruction accuracy with the trained codebook
//! 3. Measures correction overhead

use embeddenator_fs::VersionedEmbrFS;
use embeddenator_vsa::{Codebook, CodebookTrainingConfig, DIM};

fn test_with_trained_codebook(name: &str, data: &[u8], fs: &VersionedEmbrFS) {
    println!("\n=== {} ({} bytes) ===", name, data.len());

    // Write file using holographic encoding
    let path = format!(
        "test_{}.bin",
        name.to_lowercase().replace(" ", "_").replace("/", "_")
    );

    if let Err(e) = fs.write_file_holographic(&path, data, None) {
        println!("  Error writing: {:?}", e);
        return;
    }

    // Read back
    match fs.read_file_holographic(&path) {
        Ok((reconstructed, _)) => {
            // Compare
            let matching_bytes = data
                .iter()
                .zip(reconstructed.iter())
                .filter(|(a, b)| a == b)
                .count();

            let accuracy = matching_bytes as f64 / data.len() as f64 * 100.0;

            // Get correction stats
            let stats = fs.stats();
            let correction_ratio = if stats.total_size_bytes > 0 {
                stats.correction_overhead_bytes as f64 / stats.total_size_bytes as f64 * 100.0
            } else {
                0.0
            };

            println!(
                "  Reconstruction: {}/{} bytes match",
                matching_bytes,
                data.len()
            );
            println!("  Accuracy (uncorrected): {:.2}%", accuracy);
            println!("  Correction overhead: {:.2}%", correction_ratio);

            if accuracy >= 94.0 {
                println!("  STATUS: PASS (>94% uncorrected accuracy)");
            } else if correction_ratio <= 10.0 {
                println!("  STATUS: PASS (<=10% correction overhead)");
            } else {
                println!("  STATUS: NEEDS IMPROVEMENT");
            }
        }
        Err(e) => {
            println!("  Error reading: {:?}", e);
        }
    }
}

fn main() {
    println!("=== TRAINED HOLOGRAPHIC STORAGE TEST ===\n");

    // 1. Load training data (ebooks)
    println!("1. Loading training data...");
    let mut training_files: Vec<Vec<u8>> = Vec::new();

    let training_paths = [
        "/tmp/ebook-test/meditations_marcus_aurelius.txt",
        "/tmp/ebook-test/enchiridion_epictetus.txt",
        "/tmp/ebook-test/discourses_epictetus.txt",
        "/tmp/ebook-test/shortness_of_life_seneca.txt",
        "/tmp/ebook-test/moral_letters_seneca.txt",
    ];

    for path in &training_paths {
        if let Ok(data) = std::fs::read(path) {
            println!("   Loaded {} ({} bytes)", path, data.len());
            training_files.push(data);
        }
    }

    if training_files.is_empty() {
        println!("   No training files found, using synthetic data");
        // Generate synthetic training data
        for i in 0..5 {
            let mut data = Vec::new();
            for j in 0..10000 {
                data.push((j * (i + 1) % 256) as u8);
            }
            training_files.push(data);
        }
    }

    // 2. Train the codebook
    println!("\n2. Training codebook...");
    let mut codebook = Codebook::new(DIM);

    let config = CodebookTrainingConfig {
        max_basis_vectors: 512,
        min_frequency: 3,
        include_byte_basis: true,
        include_position_basis: true,
    };

    let refs: Vec<&[u8]> = training_files.iter().map(|v| v.as_slice()).collect();
    let learned = codebook.train(&refs, &config);
    println!("   Learned {} basis vectors", learned);
    println!("   Total basis vectors: {}", codebook.basis_vectors.len());

    // 3. Create filesystem with trained codebook
    println!("\n3. Creating holographic filesystem...");
    let fs = VersionedEmbrFS::new_holographic();

    // Replace the codebook with our trained one
    *fs.codebook().write().unwrap() = codebook;

    // 4. Test reconstruction accuracy
    println!("\n4. Testing reconstruction accuracy...\n");
    println!("Goal: >94% uncorrected OR <=10% correction overhead");

    // Test with ASCII text
    let ascii = b"The quick brown fox jumps over the lazy dog. Testing holographic storage.";
    test_with_trained_codebook("ASCII Text", ascii, &fs);

    // Test with training data (should have best accuracy)
    if !training_files.is_empty() {
        test_with_trained_codebook(
            "Training Sample",
            &training_files[0][..1000.min(training_files[0].len())],
            &fs,
        );
    }

    // Test with unseen text
    let unseen = b"This is completely new text that was not in the training set. \
                   The holographic storage should still handle it via byte-level basis.";
    test_with_trained_codebook("Unseen Text", unseen, &fs);

    // Test with binary data
    let binary: Vec<u8> = (0..256).map(|i| i as u8).collect();
    test_with_trained_codebook("Binary (0-255)", &binary, &fs);

    // Test with EPUB if available
    if let Ok(epub) = std::fs::read("/tmp/ebook-test/meditations_marcus_aurelius.epub") {
        test_with_trained_codebook("EPUB Binary", &epub[..1000.min(epub.len())], &fs);
    }

    println!("\n\n=== SUMMARY ===");
    println!("The trained codebook should capture common patterns from training data.");
    println!("Byte-level basis provides fallback for any data not matching patterns.");
}
