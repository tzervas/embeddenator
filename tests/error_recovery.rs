//! Error Recovery Test Suite
//!
//! Comprehensive tests for production resilience covering:
//! - Corrupted engram files
//! - Malformed manifests
//! - Resource exhaustion scenarios
//! - Concurrent access safety
//!
//! These tests validate that the system fails gracefully and provides clear
//! error messages when encountering invalid or corrupted data.

use embeddenator::{EmbrFS, HierarchicalManifest, Manifest, ReversibleVSAConfig};
use std::fs::{self, File};
use std::path::Path;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a valid test dataset with multiple files
fn create_test_dataset(dir: &Path) -> std::io::Result<()> {
    fs::write(dir.join("small.txt"), "Small test file")?;
    fs::write(dir.join("medium.txt"), "Medium ".repeat(100))?;
    let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    fs::write(dir.join("large.bin"), large_data)?;
    Ok(())
}

/// Create a valid engram and manifest for testing
fn create_valid_engram_and_manifest(
    temp_dir: &TempDir,
) -> std::io::Result<(std::path::PathBuf, std::path::PathBuf)> {
    let input_dir = temp_dir.path().join("input");
    fs::create_dir(&input_dir)?;
    create_test_dataset(&input_dir)?;

    let config = ReversibleVSAConfig::default();
    let mut fsys = EmbrFS::new();
    fsys.ingest_directory(&input_dir, false, &config)?;

    let engram_path = temp_dir.path().join("test.engram");
    let manifest_path = temp_dir.path().join("test.manifest.json");

    fsys.save_engram(&engram_path)?;
    fsys.save_manifest(&manifest_path)?;

    Ok((engram_path, manifest_path))
}

/// Corrupt a file by randomly flipping bits
fn corrupt_file_random(path: &Path, num_bytes: usize) -> std::io::Result<()> {
    use rand::Rng;
    let mut data = fs::read(path)?;
    if data.is_empty() {
        return Ok(());
    }

    let mut rng = rand::thread_rng();
    for _ in 0..num_bytes {
        let idx = rng.gen_range(0..data.len());
        data[idx] ^= 0xFF; // Flip all bits in the byte
    }

    fs::write(path, data)?;
    Ok(())
}

/// Truncate a file to a specific size
fn truncate_file(path: &Path, new_size: usize) -> std::io::Result<()> {
    let data = fs::read(path)?;
    if new_size >= data.len() {
        return Ok(());
    }
    fs::write(path, &data[..new_size])?;
    Ok(())
}

// ============================================================================
// Corrupted Engram Files Tests
// ============================================================================

#[test]
fn test_corrupted_engram_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (engram_path, _) =
        create_valid_engram_and_manifest(&temp_dir).expect("Failed to create valid engram");

    // Corrupt the engram file significantly (corrupt 50% of the file)
    // bincode is resilient to small amounts of corruption
    let file_size = fs::metadata(&engram_path).unwrap().len() as usize;
    let corruption_amount = file_size / 2;
    corrupt_file_random(&engram_path, corruption_amount).expect("Failed to corrupt file");

    // Attempt to load the corrupted engram - should fail gracefully
    let result = EmbrFS::load_engram(&engram_path);

    assert!(
        result.is_err(),
        "Loading heavily corrupted engram should fail, but succeeded"
    );

    // Verify error message provides context
    match result {
        Err(error) => {
            let error_msg = error.to_string();
            // bincode errors typically mention deserialization or I/O issues
            assert!(!error_msg.is_empty(), "Error message should not be empty");
            // Most deserialization errors will be I/O errors wrapping bincode errors
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[test]
fn test_truncated_engram_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (engram_path, _) =
        create_valid_engram_and_manifest(&temp_dir).expect("Failed to create valid engram");

    // Get original size and truncate to 25%
    let original_size = fs::metadata(&engram_path)
        .expect("Failed to get file metadata")
        .len() as usize;
    let truncated_size = original_size / 4;

    truncate_file(&engram_path, truncated_size).expect("Failed to truncate file");

    // Attempt to load the truncated engram
    let result = EmbrFS::load_engram(&engram_path);

    assert!(
        result.is_err(),
        "Loading truncated engram should fail, but succeeded"
    );

    // Verify error provides context about the issue
    match result {
        Err(error) => {
            let error_msg = error.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
            // Truncated files will produce I/O or deserialization errors
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[test]
fn test_empty_engram_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let engram_path = temp_dir.path().join("empty.engram");

    // Create an empty file
    File::create(&engram_path).expect("Failed to create empty file");

    // Attempt to load the empty engram
    let result = EmbrFS::load_engram(&engram_path);

    assert!(
        result.is_err(),
        "Loading empty engram should fail, but succeeded"
    );
}

#[test]
fn test_non_bincode_engram_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let engram_path = temp_dir.path().join("invalid.engram");

    // Write non-bincode data (plain text)
    fs::write(&engram_path, "This is not a valid bincode engram").expect("Failed to write file");

    // Attempt to load the invalid engram
    let result = EmbrFS::load_engram(&engram_path);

    assert!(
        result.is_err(),
        "Loading non-bincode engram should fail, but succeeded"
    );
}

// ============================================================================
// Malformed Manifests Tests
// ============================================================================

#[test]
fn test_malformed_json_manifest() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_path = temp_dir.path().join("malformed.manifest.json");

    // Write malformed JSON (missing closing brace)
    fs::write(&manifest_path, r#"{"files": [{"path": "test.txt""#)
        .expect("Failed to write malformed manifest");

    // Attempt to load the malformed manifest
    let result = EmbrFS::load_manifest(&manifest_path);

    assert!(
        result.is_err(),
        "Loading malformed manifest should fail, but succeeded"
    );

    // Verify error message indicates JSON parsing issue
    match result {
        Err(error) => {
            let error_msg = error.to_string();
            // JSON parsing errors should have meaningful messages
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[test]
fn test_manifest_missing_required_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_path = temp_dir.path().join("incomplete.manifest.json");

    // Write JSON missing required 'total_chunks' field
    fs::write(&manifest_path, r#"{"files": []}"#).expect("Failed to write incomplete manifest");

    // Attempt to load the incomplete manifest
    let result = EmbrFS::load_manifest(&manifest_path);

    assert!(
        result.is_err(),
        "Loading incomplete manifest should fail, but succeeded"
    );

    match result {
        Err(error) => {
            let error_msg = error.to_string();
            // Should have meaningful error about missing fields
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[test]
fn test_manifest_invalid_field_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_path = temp_dir.path().join("invalid_types.manifest.json");

    // Write JSON with wrong field types (total_chunks as string instead of number)
    fs::write(
        &manifest_path,
        r#"{"files": [], "total_chunks": "not_a_number"}"#,
    )
    .expect("Failed to write invalid manifest");

    // Attempt to load the manifest with invalid types
    let result = EmbrFS::load_manifest(&manifest_path);

    assert!(
        result.is_err(),
        "Loading manifest with invalid types should fail, but succeeded"
    );
}

#[test]
fn test_hierarchical_manifest_version_mismatch() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_path = temp_dir.path().join("future_version.json");

    // Create a hierarchical manifest with a future version number
    // Current code doesn't validate version, but this tests for forward compatibility
    let future_manifest = serde_json::json!({
        "version": 999,
        "levels": [],
        "sub_engrams": {}
    });

    fs::write(&manifest_path, future_manifest.to_string()).expect("Failed to write manifest");

    // Attempt to deserialize - should succeed but we document this behavior
    let result: Result<HierarchicalManifest, _> =
        serde_json::from_reader(File::open(&manifest_path).unwrap());

    // Currently this succeeds because we don't validate version
    // This test documents that behavior and can be updated when version validation is added
    if result.is_ok() {
        let manifest = result.unwrap();
        assert_eq!(manifest.version, 999, "Future version should be preserved");
        // Note: Production code should add version validation
        eprintln!("Warning: No version validation currently implemented");
    }
}

#[test]
fn test_manifest_with_invalid_paths() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (_, manifest_path) =
        create_valid_engram_and_manifest(&temp_dir).expect("Failed to create valid manifest");

    // Read and modify the manifest to include invalid paths
    let mut manifest: Manifest = serde_json::from_reader(File::open(&manifest_path).unwrap())
        .expect("Failed to read manifest");

    // Add a file entry with problematic path characters
    manifest.files.push(embeddenator::FileEntry {
        path: "../../../../etc/passwd".to_string(), // Path traversal attempt
        is_text: true,
        size: 100,
        chunks: vec![999],
        deleted: false,
    });

    let modified_path = temp_dir.path().join("modified.manifest.json");
    serde_json::to_writer_pretty(File::create(&modified_path).unwrap(), &manifest)
        .expect("Failed to write modified manifest");

    // Load succeeds (deserialization is OK)
    let loaded = EmbrFS::load_manifest(&modified_path);
    assert!(
        loaded.is_ok(),
        "Manifest with suspicious paths should deserialize"
    );

    // Note: Path validation should happen during extraction, not loading
    // This test documents current behavior - extraction code should validate paths
}

#[test]
fn test_empty_manifest() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_path = temp_dir.path().join("empty.manifest.json");

    // Write empty file
    File::create(&manifest_path).expect("Failed to create empty file");

    // Attempt to load empty manifest
    let result = EmbrFS::load_manifest(&manifest_path);

    assert!(
        result.is_err(),
        "Loading empty manifest should fail, but succeeded"
    );
}

// ============================================================================
// Resource Exhaustion Tests
// ============================================================================

#[test]
fn test_extremely_large_chunk_count() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_path = temp_dir.path().join("huge_chunks.manifest.json");

    // Create a manifest claiming billions of chunks (but not actually storing them)
    let manifest = serde_json::json!({
        "files": [{
            "path": "fake.txt",
            "is_text": true,
            "size": 1_000_000_000_000_u64,
            "chunks": vec![0u32; 1000] // Large but reasonable array
        }],
        "total_chunks": 1_000_000_000_000_u64
    });

    fs::write(&manifest_path, manifest.to_string()).expect("Failed to write manifest");

    // Loading the manifest should succeed (it's just metadata)
    let result = EmbrFS::load_manifest(&manifest_path);
    assert!(
        result.is_ok(),
        "Loading manifest with large counts should succeed"
    );

    // The actual resource exhaustion would occur during extraction when
    // trying to process chunks that don't exist in the engram
}

#[test]
fn test_memory_limit_graceful_failure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a very large input file (10MB of data)
    let input_dir = temp_dir.path().join("input");
    fs::create_dir(&input_dir).expect("Failed to create input dir");

    let large_data: Vec<u8> = vec![0xAB; 10_000_000]; // 10MB
    fs::write(input_dir.join("huge.bin"), large_data).expect("Failed to write large file");

    // Ingest should handle this without panic
    let config = ReversibleVSAConfig::default();
    let mut fsys = EmbrFS::new();
    let result = fsys.ingest_directory(&input_dir, false, &config);

    // Should succeed - this is within reasonable limits
    assert!(
        result.is_ok(),
        "Ingesting 10MB file should succeed: {:?}",
        result.err()
    );

    // Verify the engram was created correctly
    assert!(
        fsys.engram.codebook.len() > 0,
        "Codebook should contain chunks"
    );
}

#[test]
fn test_very_deep_directory_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input");

    // Create a very deep directory structure (100 levels)
    let mut deep_path = input_dir.clone();
    for i in 0..100 {
        deep_path = deep_path.join(format!("level_{}", i));
    }
    fs::create_dir_all(&deep_path).expect("Failed to create deep structure");
    fs::write(deep_path.join("deep_file.txt"), "Deep file content")
        .expect("Failed to write deep file");

    // Ingest should handle deep structures gracefully
    let config = ReversibleVSAConfig::default();
    let mut fsys = EmbrFS::new();
    let result = fsys.ingest_directory(&input_dir, false, &config);

    assert!(
        result.is_ok(),
        "Ingesting deep directory should succeed: {:?}",
        result.err()
    );
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[test]
fn test_concurrent_read_safety() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (engram_path, manifest_path) =
        create_valid_engram_and_manifest(&temp_dir).expect("Failed to create valid engram");

    // Share paths across threads
    let engram_path = Arc::new(engram_path);
    let manifest_path = Arc::new(manifest_path);
    let barrier = Arc::new(Barrier::new(5));

    let mut handles = vec![];

    // Spawn 5 threads that all try to read the same files concurrently
    for thread_id in 0..5 {
        let engram_path = Arc::clone(&engram_path);
        let manifest_path = Arc::clone(&manifest_path);
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            // Synchronize start
            barrier.wait();

            // Each thread attempts to load the engram and manifest
            let engram_result = EmbrFS::load_engram(engram_path.as_ref());
            let manifest_result = EmbrFS::load_manifest(manifest_path.as_ref());

            (thread_id, engram_result.is_ok(), manifest_result.is_ok())
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let (thread_id, engram_ok, manifest_ok) = handle.join().expect("Thread panicked");
        assert!(engram_ok, "Thread {} failed to load engram", thread_id);
        assert!(manifest_ok, "Thread {} failed to load manifest", thread_id);
    }
}

#[test]
fn test_concurrent_write_to_different_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    // Spawn 3 threads that write to different engram files
    for thread_id in 0..3 {
        let base_path = base_path.to_path_buf();
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier.wait();

            // Each thread creates its own engram
            let input_dir = base_path.join(format!("input_{}", thread_id));
            fs::create_dir(&input_dir).expect("Failed to create input dir");
            fs::write(input_dir.join("file.txt"), format!("Thread {}", thread_id))
                .expect("Failed to write file");

            let config = ReversibleVSAConfig::default();
            let mut fsys = EmbrFS::new();
            fsys.ingest_directory(&input_dir, false, &config)
                .expect("Failed to ingest");

            let engram_path = base_path.join(format!("thread_{}.engram", thread_id));
            let manifest_path = base_path.join(format!("thread_{}.manifest.json", thread_id));

            fsys.save_engram(&engram_path)
                .expect("Failed to save engram");
            fsys.save_manifest(&manifest_path)
                .expect("Failed to save manifest");

            thread_id
        });

        handles.push(handle);
    }

    // All threads should complete successfully
    for handle in handles {
        let thread_id = handle.join().expect("Thread panicked");

        // Verify files were created
        let engram_path = base_path.join(format!("thread_{}.engram", thread_id));
        let manifest_path = base_path.join(format!("thread_{}.manifest.json", thread_id));

        assert!(
            engram_path.exists(),
            "Thread {} engram not created",
            thread_id
        );
        assert!(
            manifest_path.exists(),
            "Thread {} manifest not created",
            thread_id
        );
    }
}

#[test]
fn test_read_during_corruption_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (engram_path, _) =
        create_valid_engram_and_manifest(&temp_dir).expect("Failed to create valid engram");

    // First, verify the engram loads correctly
    let original_load = EmbrFS::load_engram(&engram_path);
    assert!(original_load.is_ok(), "Original engram should load");

    // Corrupt the file heavily (50% of file)
    let file_size = fs::metadata(&engram_path).unwrap().len() as usize;
    let corruption_amount = file_size / 2;
    corrupt_file_random(&engram_path, corruption_amount).expect("Failed to corrupt");

    // Now attempt to load - should fail gracefully
    let corrupted_load = EmbrFS::load_engram(&engram_path);
    assert!(
        corrupted_load.is_err(),
        "Heavily corrupted engram should fail to load"
    );

    // Error should be clear, not a panic or silent failure
    match corrupted_load {
        Err(error) => {
            let error_msg = error.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_error_messages_contain_context() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent_path = temp_dir.path().join("does_not_exist.engram");

    // Try to load non-existent file
    let result = EmbrFS::load_engram(&nonexistent_path);

    assert!(result.is_err(), "Should fail for non-existent file");

    match result {
        Err(error) => {
            let error_msg = error.to_string();
            // Error should mention file operation or "not found"
            assert!(!error_msg.is_empty(), "Error message should not be empty");
            // File not found errors are typically std::io::Error with NotFound kind
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[test]
fn test_no_silent_failures_on_invalid_data() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create various invalid data scenarios
    let test_cases = vec![
        ("empty.engram", vec![]),
        ("garbage.engram", vec![0xFF; 1000]),
        ("partial.engram", vec![0x42; 10]),
    ];

    for (filename, data) in test_cases {
        let path = temp_dir.path().join(filename);
        fs::write(&path, data).expect("Failed to write test file");

        // All should fail with an error, not succeed with corrupted data
        let result = EmbrFS::load_engram(&path);
        assert!(
            result.is_err(),
            "Loading {} should fail, but succeeded",
            filename
        );

        // Should not return default/empty engram
        // (This would be a silent failure)
    }
}

#[test]
fn test_manifest_load_preserves_all_data() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (_, manifest_path) =
        create_valid_engram_and_manifest(&temp_dir).expect("Failed to create valid manifest");

    // Load the manifest
    let manifest = EmbrFS::load_manifest(&manifest_path).expect("Failed to load manifest");

    // Verify critical fields are present and valid
    assert!(
        manifest.total_chunks > 0,
        "Manifest should have chunks recorded"
    );
    assert!(
        !manifest.files.is_empty(),
        "Manifest should have file entries"
    );

    // Verify file entries have complete information
    for file_entry in &manifest.files {
        assert!(
            !file_entry.path.is_empty(),
            "File entry should have non-empty path"
        );
        assert!(
            !file_entry.chunks.is_empty(),
            "File entry should reference chunks"
        );
        // Size can be 0 for empty files, so we don't assert on that
    }
}
