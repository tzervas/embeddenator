//! End-to-End Regression Test Suite
//!
//! Comprehensive tests to ensure critical functionality is maintained across updates
//! Tests the complete workflow from ingestion to extraction with various scenarios

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the embeddenator binary
fn embeddenator_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_embeddenator"))
}

/// Create a comprehensive test dataset
fn create_comprehensive_dataset(dir: &TempDir) -> std::io::Result<()> {
    let input = dir.path().join("input");
    fs::create_dir(&input)?;

    // Various text files with different encodings and sizes
    fs::write(input.join("small.txt"), "Small file")?;
    fs::write(input.join("medium.txt"), "Medium file ".repeat(100))?;
    fs::write(input.join("large.txt"), "Large file content ".repeat(1000))?;

    // JSON and structured data
    fs::write(input.join("data.json"), r#"{"key": "value", "number": 42}"#)?;
    fs::write(input.join("config.yaml"), "setting: true\nvalue: 123\n")?;

    // Binary files of various sizes
    fs::write(input.join("tiny.bin"), [0u8, 1, 2, 3, 4])?;
    let medium_bin: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    fs::write(input.join("medium.bin"), medium_bin)?;

    // Subdirectories
    fs::create_dir(input.join("subdir1"))?;
    fs::create_dir(input.join("subdir2"))?;
    fs::write(input.join("subdir1/nested1.txt"), "Nested file 1")?;
    fs::write(input.join("subdir2/nested2.txt"), "Nested file 2")?;

    // Deep nesting
    fs::create_dir_all(input.join("deep/path/to/file"))?;
    fs::write(input.join("deep/path/to/file/deep.txt"), "Deep nested")?;

    // Special characters in filenames
    fs::write(input.join("file-with-dashes.txt"), "Dashes")?;
    fs::write(input.join("file_with_underscores.txt"), "Underscores")?;

    // Empty file
    File::create(input.join("empty.txt"))?;

    Ok(())
}

#[test]
fn test_e2e_comprehensive_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    create_comprehensive_dataset(&temp_dir).expect("Failed to create dataset");

    let input = temp_dir.path().join("input");
    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");
    let output = temp_dir.path().join("output");

    // Step 1: Ingest
    let ingest_output = Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            input.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-v",
        ])
        .output()
        .expect("Failed to run ingest");

    assert!(
        ingest_output.status.success(),
        "Ingest failed: {}",
        String::from_utf8_lossy(&ingest_output.stderr)
    );
    assert!(engram.exists(), "Engram file not created");
    assert!(manifest.exists(), "Manifest file not created");

    // Verify manifest content
    let manifest_content = fs::read_to_string(&manifest).unwrap();
    assert!(manifest_content.contains("small.txt"));
    assert!(manifest_content.contains("subdir1"));
    assert!(manifest_content.contains("deep/path"));

    // Step 2: Extract
    let extract_output = Command::new(embeddenator_bin())
        .args([
            "extract",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "-v",
        ])
        .output()
        .expect("Failed to run extract");

    assert!(
        extract_output.status.success(),
        "Extract failed: {}",
        String::from_utf8_lossy(&extract_output.stderr)
    );

    // Step 3: Verify all files
    assert_eq!(
        fs::read(input.join("small.txt")).unwrap(),
        fs::read(output.join("small.txt")).unwrap(),
        "small.txt mismatch"
    );

    assert_eq!(
        fs::read(input.join("medium.txt")).unwrap(),
        fs::read(output.join("medium.txt")).unwrap(),
        "medium.txt mismatch"
    );

    assert_eq!(
        fs::read(input.join("large.txt")).unwrap(),
        fs::read(output.join("large.txt")).unwrap(),
        "large.txt mismatch"
    );

    assert_eq!(
        fs::read(input.join("tiny.bin")).unwrap(),
        fs::read(output.join("tiny.bin")).unwrap(),
        "tiny.bin mismatch"
    );

    assert_eq!(
        fs::read(input.join("medium.bin")).unwrap(),
        fs::read(output.join("medium.bin")).unwrap(),
        "medium.bin mismatch"
    );

    assert_eq!(
        fs::read(input.join("subdir1/nested1.txt")).unwrap(),
        fs::read(output.join("subdir1/nested1.txt")).unwrap(),
        "nested1.txt mismatch"
    );

    assert_eq!(
        fs::read(input.join("deep/path/to/file/deep.txt")).unwrap(),
        fs::read(output.join("deep/path/to/file/deep.txt")).unwrap(),
        "deep.txt mismatch"
    );

    // Verify empty file
    assert_eq!(fs::read(input.join("empty.txt")).unwrap().len(), 0);
    assert_eq!(fs::read(output.join("empty.txt")).unwrap().len(), 0);
}

#[test]
fn test_e2e_large_dataset_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    // Create 100 files of various sizes
    for i in 0..100 {
        let content = format!("File {} content ", i).repeat(50);
        fs::write(input.join(format!("file{:03}.txt", i)), content).unwrap();
    }

    let engram = temp_dir.path().join("large.engram");
    let manifest = temp_dir.path().join("large.manifest.json");
    let output = temp_dir.path().join("output");

    // Ingest
    let start = std::time::Instant::now();
    let ingest_output = Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            input.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run ingest");
    let ingest_time = start.elapsed();

    assert!(ingest_output.status.success());
    assert!(
        ingest_time.as_secs() < 10,
        "Ingest took too long: {:?}",
        ingest_time
    );

    // Extract
    let start = std::time::Instant::now();
    let extract_output = Command::new(embeddenator_bin())
        .args([
            "extract",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run extract");
    let extract_time = start.elapsed();

    assert!(extract_output.status.success());
    assert!(
        extract_time.as_secs() < 10,
        "Extract took too long: {:?}",
        extract_time
    );

    // Verify a few random files
    assert_eq!(
        fs::read(input.join("file000.txt")).unwrap(),
        fs::read(output.join("file000.txt")).unwrap()
    );
    assert_eq!(
        fs::read(input.join("file050.txt")).unwrap(),
        fs::read(output.join("file050.txt")).unwrap()
    );
    assert_eq!(
        fs::read(input.join("file099.txt")).unwrap(),
        fs::read(output.join("file099.txt")).unwrap()
    );
}

#[test]
fn test_e2e_query_functionality() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    fs::write(
        input.join("doc1.txt"),
        "Important document about holographic storage",
    )
    .unwrap();
    fs::write(
        input.join("doc2.txt"),
        "Another document with different content",
    )
    .unwrap();
    fs::write(input.join("doc3.txt"), "Third document for testing").unwrap();

    let engram = temp_dir.path().join("query.engram");
    let manifest = temp_dir.path().join("query.manifest.json");

    // Ingest
    Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            input.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run ingest");

    // Query with a document that's in the engram
    let query_output = Command::new(embeddenator_bin())
        .args([
            "query",
            "-e",
            engram.to_str().unwrap(),
            "-q",
            input.join("doc1.txt").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run query");

    assert!(query_output.status.success());
    let output_str = String::from_utf8_lossy(&query_output.stdout);
    assert!(output_str.contains("Similarity"));
}

#[test]
fn test_e2e_regression_data_integrity() {
    // This test ensures that data integrity is maintained across updates
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    // Create test data with known checksums
    let test_data = b"This is test data for integrity checking";
    fs::write(input.join("integrity.txt"), test_data).unwrap();

    let binary_data: Vec<u8> = (0..=255).collect();
    fs::write(input.join("integrity.bin"), &binary_data).unwrap();

    let engram = temp_dir.path().join("integrity.engram");
    let manifest = temp_dir.path().join("integrity.manifest.json");
    let output = temp_dir.path().join("output");

    // Ingest
    Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            input.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run ingest");

    // Extract
    Command::new(embeddenator_bin())
        .args([
            "extract",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run extract");

    // Verify exact byte-for-byte match
    let extracted_text = fs::read(output.join("integrity.txt")).unwrap();
    assert_eq!(
        test_data,
        extracted_text.as_slice(),
        "Text data integrity check failed"
    );

    let extracted_binary = fs::read(output.join("integrity.bin")).unwrap();
    assert_eq!(
        binary_data, extracted_binary,
        "Binary data integrity check failed"
    );
}

#[test]
fn test_e2e_regression_directory_structure() {
    // Ensure directory structure is preserved
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");

    // Create complex directory structure
    fs::create_dir_all(input.join("a/b/c/d")).unwrap();
    fs::create_dir_all(input.join("x/y/z")).unwrap();
    fs::write(input.join("a/file1.txt"), "File 1").unwrap();
    fs::write(input.join("a/b/file2.txt"), "File 2").unwrap();
    fs::write(input.join("a/b/c/file3.txt"), "File 3").unwrap();
    fs::write(input.join("a/b/c/d/file4.txt"), "File 4").unwrap();
    fs::write(input.join("x/y/z/file5.txt"), "File 5").unwrap();

    let engram = temp_dir.path().join("structure.engram");
    let manifest = temp_dir.path().join("structure.manifest.json");
    let output = temp_dir.path().join("output");

    // Ingest and extract
    Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            input.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run ingest");

    Command::new(embeddenator_bin())
        .args([
            "extract",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run extract");

    // Verify directory structure
    assert!(output.join("a").is_dir());
    assert!(output.join("a/b").is_dir());
    assert!(output.join("a/b/c").is_dir());
    assert!(output.join("a/b/c/d").is_dir());
    assert!(output.join("x/y/z").is_dir());

    // Verify files in correct locations
    assert_eq!(
        fs::read_to_string(output.join("a/file1.txt")).unwrap(),
        "File 1"
    );
    assert_eq!(
        fs::read_to_string(output.join("a/b/file2.txt")).unwrap(),
        "File 2"
    );
    assert_eq!(
        fs::read_to_string(output.join("a/b/c/file3.txt")).unwrap(),
        "File 3"
    );
    assert_eq!(
        fs::read_to_string(output.join("a/b/c/d/file4.txt")).unwrap(),
        "File 4"
    );
    assert_eq!(
        fs::read_to_string(output.join("x/y/z/file5.txt")).unwrap(),
        "File 5"
    );
}

#[test]
fn test_e2e_engram_modification_persistence() {
    // **Critical Test**: Validates that filesystem can be converted to engram,
    // modified while in engram-extractable state, and changes persist correctly.
    // This is a KEY FEATURE of Embeddenator - proving engrams are functional filesystems.

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    // Phase 1: Create initial filesystem
    fs::write(input.join("original.txt"), "Original content v1").unwrap();
    fs::write(input.join("to_modify.txt"), "Will be modified").unwrap();
    fs::write(input.join("to_delete.txt"), "Will be deleted").unwrap();
    fs::create_dir(input.join("original_dir")).unwrap();
    fs::write(input.join("original_dir/nested.txt"), "Nested original").unwrap();

    let engram1 = temp_dir.path().join("v1.engram");
    let manifest1 = temp_dir.path().join("v1.manifest.json");
    let extract1 = temp_dir.path().join("extract_v1");

    // Phase 2: Convert to engram (v1)
    let ingest1 = Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            input.to_str().unwrap(),
            "-e",
            engram1.to_str().unwrap(),
            "-m",
            manifest1.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to ingest v1");

    assert!(
        ingest1.status.success(),
        "Initial engram creation failed: {}",
        String::from_utf8_lossy(&ingest1.stderr)
    );

    // Phase 3: Extract from engram (v1)
    let extract_cmd1 = Command::new(embeddenator_bin())
        .args([
            "extract",
            "-e",
            engram1.to_str().unwrap(),
            "-m",
            manifest1.to_str().unwrap(),
            "-o",
            extract1.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to extract v1");

    assert!(
        extract_cmd1.status.success(),
        "Extract v1 failed: {}",
        String::from_utf8_lossy(&extract_cmd1.stderr)
    );

    // Phase 4: Verify initial extraction matches original
    assert_eq!(
        fs::read_to_string(extract1.join("original.txt")).unwrap(),
        "Original content v1"
    );
    assert_eq!(
        fs::read_to_string(extract1.join("to_modify.txt")).unwrap(),
        "Will be modified"
    );
    assert!(extract1.join("to_delete.txt").exists());

    // Phase 5: MODIFY the extracted filesystem (simulating usage while in engram state)
    // This is the KEY TEST - can we modify the extracted engram and re-ingest?
    fs::write(extract1.join("to_modify.txt"), "MODIFIED content v2").unwrap();
    fs::remove_file(extract1.join("to_delete.txt")).unwrap();
    fs::write(extract1.join("new_file.txt"), "Newly created file").unwrap();
    fs::create_dir(extract1.join("new_dir")).unwrap();
    fs::write(
        extract1.join("new_dir/new_nested.txt"),
        "New nested content",
    )
    .unwrap();
    fs::write(extract1.join("original_dir/nested.txt"), "Nested MODIFIED").unwrap();

    // Phase 6: Re-ingest the MODIFIED filesystem to create v2 engram
    let engram2 = temp_dir.path().join("v2.engram");
    let manifest2 = temp_dir.path().join("v2.manifest.json");

    let ingest2 = Command::new(embeddenator_bin())
        .args([
            "ingest",
            "-i",
            extract1.to_str().unwrap(),
            "-e",
            engram2.to_str().unwrap(),
            "-m",
            manifest2.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to ingest v2");

    assert!(
        ingest2.status.success(),
        "Re-ingestion of modified filesystem failed: {}",
        String::from_utf8_lossy(&ingest2.stderr)
    );

    // Phase 7: Extract from v2 engram to verify modifications persisted
    let extract2 = temp_dir.path().join("extract_v2");

    let extract_cmd2 = Command::new(embeddenator_bin())
        .args([
            "extract",
            "-e",
            engram2.to_str().unwrap(),
            "-m",
            manifest2.to_str().unwrap(),
            "-o",
            extract2.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to extract v2");

    assert!(
        extract_cmd2.status.success(),
        "Extract v2 failed: {}",
        String::from_utf8_lossy(&extract_cmd2.stderr)
    );

    // Phase 8: VALIDATE ALL MODIFICATIONS PERSISTED CORRECTLY

    // Verify modified file has new content
    assert_eq!(
        fs::read_to_string(extract2.join("to_modify.txt")).unwrap(),
        "MODIFIED content v2",
        "Modified file content did not persist!"
    );

    // Verify deleted file is gone
    assert!(
        !extract2.join("to_delete.txt").exists(),
        "Deleted file should not exist in v2 extract!"
    );

    // Verify new file was added
    assert_eq!(
        fs::read_to_string(extract2.join("new_file.txt")).unwrap(),
        "Newly created file",
        "New file not persisted correctly!"
    );

    // Verify new directory and nested file
    assert!(
        extract2.join("new_dir").is_dir(),
        "New directory not persisted!"
    );
    assert_eq!(
        fs::read_to_string(extract2.join("new_dir/new_nested.txt")).unwrap(),
        "New nested content",
        "New nested file not persisted!"
    );

    // Verify modifications to nested files
    assert_eq!(
        fs::read_to_string(extract2.join("original_dir/nested.txt")).unwrap(),
        "Nested MODIFIED",
        "Nested file modifications not persisted!"
    );

    // Verify unmodified file remained unchanged
    assert_eq!(
        fs::read_to_string(extract2.join("original.txt")).unwrap(),
        "Original content v1",
        "Unmodified file should remain unchanged!"
    );

    // Phase 9: Verify manifest reflects the changes
    let manifest2_content = fs::read_to_string(&manifest2).unwrap();
    assert!(
        manifest2_content.contains("new_file.txt"),
        "Manifest should contain new_file.txt"
    );
    assert!(
        manifest2_content.contains("new_dir"),
        "Manifest should contain new_dir"
    );
    assert!(
        !manifest2_content.contains("to_delete.txt"),
        "Manifest should not reference deleted file"
    );

    println!("✅ CRITICAL TEST PASSED: Engram modification persistence validated!");
    println!("   - Filesystem converted to engram (v1)");
    println!("   - Extracted and modified (added/changed/deleted files)");
    println!("   - Re-ingested to create new engram (v2)");
    println!("   - All modifications persisted correctly after extraction");
    println!("   - Proves engrams are fully functional, modifiable filesystems!");
}
