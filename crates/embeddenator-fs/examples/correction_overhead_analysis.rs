//! Correction Overhead Analysis - Is This True Holographic Storage?
//!
//! This test measures how much data is actually stored in the VSA vectors
//! versus the correction layer. If corrections are >50% of original,
//! this is effectively indexed storage, not holographic.

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

fn analyze_correction_overhead(name: &str, data: &[u8], config: &ReversibleVSAConfig) {
    let chunk_size = config.block_size;
    let mut total_original = 0usize;
    let mut total_correction = 0usize;
    let mut verbatim_chunks = 0usize;
    let mut bitflip_chunks = 0usize;
    let mut perfect_chunks = 0usize;
    let mut total_chunks = 0usize;

    for chunk_data in data.chunks(chunk_size) {
        total_chunks += 1;
        total_original += chunk_data.len();

        // Encode and decode
        let encoded = SparseVec::encode_data(chunk_data, config, None);
        let decoded = encoded.decode_data(config, None, chunk_data.len());

        // Count differences
        let diff_count = chunk_data
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a != b)
            .count();

        if diff_count == 0 {
            perfect_chunks += 1;
            // No correction needed
        } else if diff_count > chunk_data.len() / 2 {
            verbatim_chunks += 1;
            // Would store verbatim = full chunk
            total_correction += chunk_data.len();
        } else {
            bitflip_chunks += 1;
            // BitFlips: 9 bytes per difference (8 pos + 1 mask)
            total_correction += diff_count * 9;
        }
    }

    let correction_ratio = total_correction as f64 / total_original as f64 * 100.0;
    let perfect_ratio = perfect_chunks as f64 / total_chunks as f64 * 100.0;
    let verbatim_ratio = verbatim_chunks as f64 / total_chunks as f64 * 100.0;

    println!("\n=== {} ===", name);
    println!(
        "  Original size: {} bytes ({} chunks @ {}B)",
        total_original, total_chunks, chunk_size
    );
    println!(
        "  Correction overhead: {} bytes ({:.1}% of original)",
        total_correction, correction_ratio
    );
    println!("  Chunk breakdown:");
    println!(
        "    - Perfect (0% correction): {} ({:.1}%)",
        perfect_chunks, perfect_ratio
    );
    println!(
        "    - BitFlips (sparse):       {} ({:.1}%)",
        bitflip_chunks,
        bitflip_chunks as f64 / total_chunks as f64 * 100.0
    );
    println!(
        "    - VERBATIM (full copy):    {} ({:.1}%)",
        verbatim_chunks, verbatim_ratio
    );

    if verbatim_ratio > 50.0 {
        println!(
            "\n  !!! WARNING: >50% chunks stored VERBATIM - this is NOT holographic storage !!!"
        );
    } else if correction_ratio > 50.0 {
        println!(
            "\n  !!! WARNING: Correction overhead >50% - this is borderline indexed storage !!!"
        );
    } else if perfect_ratio > 80.0 {
        println!("\n  SUCCESS: >80% chunks perfectly encoded in VSA - TRUE holographic storage!");
    } else {
        println!("\n  MIXED: Some data is holographic, some needs correction.");
    }
}

fn main() {
    println!("=== CORRECTION OVERHEAD ANALYSIS ===");
    println!("Measuring how much data lives in VSA vs correction layer\n");

    let config = ReversibleVSAConfig::default();

    // Test 1: ASCII text (should encode well)
    let ascii_text = b"The quick brown fox jumps over the lazy dog. \
                       This is a test of ASCII text encoding which should \
                       map reasonably well to VSA vectors since the byte \
                       values are in a predictable range (32-127).";
    analyze_correction_overhead("ASCII Text", ascii_text, &config);

    // Test 2: High entropy (random bytes - worst case)
    let mut random_data = vec![0u8; 1024];
    for (i, byte) in random_data.iter_mut().enumerate() {
        *byte = (i * 17 + 31) as u8; // Pseudo-random
    }
    analyze_correction_overhead("Pseudo-Random Bytes", &random_data, &config);

    // Test 3: Repetitive data (should compress well in any system)
    let repetitive = vec![0x42u8; 1024]; // All 'B's
    analyze_correction_overhead("Repetitive Data", &repetitive, &config);

    // Test 4: Real text file (Stoic philosophy)
    if let Ok(ebook) = std::fs::read("/tmp/ebook-test/meditations_marcus_aurelius.txt") {
        analyze_correction_overhead("Marcus Aurelius - Meditations", &ebook, &config);
    }

    // Test 5: Binary file (EPUB)
    if let Ok(epub) = std::fs::read("/tmp/ebook-test/meditations_marcus_aurelius.epub") {
        analyze_correction_overhead("EPUB (binary container)", &epub, &config);
    }

    // Test 6: Mixed content
    if let Ok(image) = std::fs::read("/tmp/embdntr-test-images/test_png.png") {
        analyze_correction_overhead("PNG Image", &image, &config);
    }

    println!("\n\n=== CONCLUSION ===\n");
    println!("If most chunks need VERBATIM correction, the system is essentially:");
    println!("  VSA = content-addressable index");
    println!("  Correction layer = actual data storage");
    println!("\nFor TRUE holographic storage, we need the VSA decode to be more accurate");
    println!("so corrections are sparse (BitFlips), not full data copies (Verbatim).");
}
