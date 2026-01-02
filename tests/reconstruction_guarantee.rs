//! Integration test for 100% reconstruction guarantee
//!
//! This test verifies that the CorrectionStore integration with EmbrFS
//! guarantees bit-perfect reconstruction for all types of data.

use embeddenator::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use tempfile::TempDir;

/// Test helper to verify exact byte equality
fn verify_exact_reconstruction(original: &[u8], reconstructed: &[u8], description: &str) {
    assert_eq!(
        original.len(),
        reconstructed.len(),
        "{}: Length mismatch - original {} bytes, reconstructed {} bytes",
        description,
        original.len(),
        reconstructed.len()
    );
    
    let mismatches: Vec<_> = original
        .iter()
        .zip(reconstructed.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| format!("byte {}: {} != {}", i, a, b))
        .take(10)
        .collect();
    
    assert!(
        mismatches.is_empty(),
        "{}: {} byte mismatches (showing first 10): {:?}",
        description,
        original.iter().zip(reconstructed).filter(|(a, b)| a != b).count(),
        mismatches
    );
}

#[test]
fn test_reconstruction_guarantee_text() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    
    // Create test text file
    let test_content = "Hello, World!\nThis is a test of the holographic filesystem.\n\
        The quick brown fox jumps over the lazy dog.\n\
        Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n";
    let test_path = input_dir.join("test.txt");
    fs::write(&test_path, test_content).unwrap();
    
    // Ingest
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    embrfs.ingest_file(&test_path, "test.txt".to_string(), false, &config).unwrap();
    
    // Extract
    fs::create_dir_all(&output_dir).unwrap();
    EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, false, &config).unwrap();
    
    // Verify exact reconstruction
    let reconstructed = fs::read(output_dir.join("test.txt")).unwrap();
    verify_exact_reconstruction(test_content.as_bytes(), &reconstructed, "text file");
    
    // Print stats
    let stats = embrfs.correction_stats();
    println!("Text file stats: {:?}", stats);
}

#[test]
fn test_reconstruction_guarantee_binary() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    
    // Create binary data with all possible byte values
    let mut binary_data: Vec<u8> = (0..=255).collect();
    // Repeat to make it larger
    for i in 0..10 {
        binary_data.extend((0..=255u8).map(|b| b.wrapping_add(i)));
    }
    
    let test_path = input_dir.join("binary.bin");
    fs::write(&test_path, &binary_data).unwrap();
    
    // Ingest
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    embrfs.ingest_file(&test_path, "binary.bin".to_string(), false, &config).unwrap();
    
    // Extract
    fs::create_dir_all(&output_dir).unwrap();
    EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, false, &config).unwrap();
    
    // Verify exact reconstruction
    let reconstructed = fs::read(output_dir.join("binary.bin")).unwrap();
    verify_exact_reconstruction(&binary_data, &reconstructed, "binary file");
    
    // Print stats
    let stats = embrfs.correction_stats();
    println!("Binary file stats: {} chunks, {} perfect ({:.1}%)", 
        stats.total_chunks, 
        stats.perfect_chunks,
        stats.perfect_ratio * 100.0);
}

#[test]
fn test_reconstruction_guarantee_random() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    
    // Create pseudo-random data (deterministic for reproducibility)
    let mut random_data = vec![0u8; 8192];
    let mut state: u64 = 0xDEADBEEF;
    for byte in random_data.iter_mut() {
        // LCG random number generator
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *byte = (state >> 56) as u8;
    }
    
    let test_path = input_dir.join("random.bin");
    fs::write(&test_path, &random_data).unwrap();
    
    // Ingest
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    embrfs.ingest_file(&test_path, "random.bin".to_string(), false, &config).unwrap();
    
    // Extract
    fs::create_dir_all(&output_dir).unwrap();
    EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, false, &config).unwrap();
    
    // Verify exact reconstruction
    let reconstructed = fs::read(output_dir.join("random.bin")).unwrap();
    verify_exact_reconstruction(&random_data, &reconstructed, "random data");
    
    let stats = embrfs.correction_stats();
    println!("Random data stats: correction overhead {:.2}%", stats.correction_ratio * 100.0);
}

#[test]
fn test_reconstruction_guarantee_empty() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    
    // Empty file
    let test_path = input_dir.join("empty.txt");
    fs::write(&test_path, &[]).unwrap();
    
    // Ingest
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    embrfs.ingest_file(&test_path, "empty.txt".to_string(), false, &config).unwrap();
    
    // Extract
    fs::create_dir_all(&output_dir).unwrap();
    EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, false, &config).unwrap();
    
    // Verify exact reconstruction
    let reconstructed = fs::read(output_dir.join("empty.txt")).unwrap();
    assert!(reconstructed.is_empty(), "Empty file should remain empty");
}

#[test]
fn test_reconstruction_guarantee_single_byte() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    
    // Test all possible single-byte values
    for byte_val in 0..=255u8 {
        let test_path = input_dir.join(format!("byte_{}.bin", byte_val));
        fs::write(&test_path, &[byte_val]).unwrap();
        
        let mut embrfs = EmbrFS::new();
        let config = ReversibleVSAConfig::default();
        embrfs.ingest_file(&test_path, format!("byte_{}.bin", byte_val), false, &config).unwrap();
        
        fs::create_dir_all(&output_dir).unwrap();
        EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, false, &config).unwrap();
        
        let reconstructed = fs::read(output_dir.join(format!("byte_{}.bin", byte_val))).unwrap();
        assert_eq!(
            vec![byte_val], reconstructed,
            "Single byte {} reconstruction failed", byte_val
        );
        
        // Clean up for next iteration
        fs::remove_dir_all(&output_dir).ok();
    }
}

#[test]
fn test_reconstruction_guarantee_multi_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(input_dir.join("subdir")).unwrap();
    
    // Create multiple files of different types
    let files = vec![
        ("file1.txt", b"Hello, World!".to_vec()),
        ("file2.txt", b"Another test file with more content.".to_vec()),
        ("binary.bin", (0..=255).collect::<Vec<u8>>()),
        ("subdir/nested.txt", b"Nested file content".to_vec()),
    ];
    
    for (name, content) in &files {
        let path = input_dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&path, content).unwrap();
    }
    
    // Ingest all files
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    
    for (name, _) in &files {
        let path = input_dir.join(name);
        embrfs.ingest_file(&path, name.to_string(), false, &config).unwrap();
    }
    
    // Extract all files
    fs::create_dir_all(&output_dir).unwrap();
    EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, false, &config).unwrap();
    
    // Verify all files
    for (name, original) in &files {
        let reconstructed = fs::read(output_dir.join(name)).unwrap();
        verify_exact_reconstruction(original, &reconstructed, name);
    }
    
    let stats = embrfs.correction_stats();
    println!("Multi-file stats: {} total chunks, {:.1}% perfect", 
        stats.total_chunks, 
        stats.perfect_ratio * 100.0);
}

#[test]
fn test_correction_stats_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    
    fs::create_dir_all(&input_dir).unwrap();
    
    // Create a file that spans multiple chunks (4KB each)
    let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let test_path = input_dir.join("large.bin");
    fs::write(&test_path, &large_data).unwrap();
    
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    embrfs.ingest_file(&test_path, "large.bin".to_string(), false, &config).unwrap();
    
    let stats = embrfs.correction_stats();
    
    // Verify stats make sense
    assert!(stats.total_chunks > 0, "Should have chunks");
    assert!(stats.perfect_chunks + stats.corrected_chunks == stats.total_chunks,
        "Perfect + corrected should equal total");
    assert!(stats.original_bytes >= large_data.len() as u64,
        "Original bytes should be at least file size");
    
    println!("Large file stats:");
    println!("  Total chunks: {}", stats.total_chunks);
    println!("  Perfect: {} ({:.1}%)", stats.perfect_chunks, stats.perfect_ratio * 100.0);
    println!("  Corrected: {}", stats.corrected_chunks);
    println!("  Correction overhead: {:.2}%", stats.correction_ratio * 100.0);
}
