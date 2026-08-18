//! Holographic Reconstruction Accuracy Test
//!
//! Tests the new holographic mode to measure:
//! 1. Reconstruction accuracy without corrections
//! 2. Correction overhead percentage
//! 3. Whether we can achieve >94% accuracy uncorrected

use embeddenator_fs::VersionedEmbrFS;

fn test_reconstruction_accuracy(name: &str, data: &[u8]) {
    println!("\n=== {} ({} bytes) ===", name, data.len());

    // Create holographic FS
    let fs = VersionedEmbrFS::new_holographic();

    // Write file
    let path = format!("test_{}.bin", name.to_lowercase().replace(" ", "_"));
    fs.write_file_holographic(&path, data, None)
        .expect("Failed to write file");

    // Read back
    let (reconstructed, _) = fs
        .read_file_holographic(&path)
        .expect("Failed to read file");

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

fn main() {
    println!("=== HOLOGRAPHIC RECONSTRUCTION ACCURACY TEST ===");
    println!("Goal: >94% uncorrected OR <=10% correction overhead\n");

    // Test 1: Simple ASCII text
    let ascii_text = b"The quick brown fox jumps over the lazy dog. \
                       This is a test of the holographic encoding system.";
    test_reconstruction_accuracy("ASCII Text", ascii_text);

    // Test 2: Repetitive data
    let repetitive = vec![0x42u8; 1024]; // All 'B's
    test_reconstruction_accuracy("Repetitive", &repetitive);

    // Test 3: Pseudo-random
    let pseudo_random: Vec<u8> = (0..1024).map(|i| ((i * 17 + 31) % 256) as u8).collect();
    test_reconstruction_accuracy("Pseudo-Random", &pseudo_random);

    // Test 4: Real text (if available)
    if let Ok(ebook) = std::fs::read("/tmp/ebook-test/meditations_marcus_aurelius.txt") {
        test_reconstruction_accuracy("Marcus Aurelius", &ebook);
    }

    // Test 5: Binary (EPUB)
    if let Ok(epub) = std::fs::read("/tmp/ebook-test/meditations_marcus_aurelius.epub") {
        test_reconstruction_accuracy("EPUB Binary", &epub);
    }

    // Test 6: PNG image
    if let Ok(png) = std::fs::read("/tmp/embdntr-test-images/test_png.png") {
        test_reconstruction_accuracy("PNG Image", &png);
    }

    println!("\n\n=== SUMMARY ===");
    println!("If accuracy is low, the Codebook basis vectors need expansion.");
    println!("The goal is to have basis vectors that capture common data patterns,");
    println!("so residuals are sparse (only storing unique/unusual data).");
}
