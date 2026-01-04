//! Integration tests for Embeddenator CLI
//!
//! These tests validate the end-to-end functionality of the CLI:
//! - Ingest directories to engrams
//! - Extract engrams to directories
//! - Query engrams for similarity
//! - Bit-perfect reconstruction

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the embeddenator binary
fn embeddenator_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_embeddenator"))
}

/// Create a test directory with sample files
fn create_test_input(dir: &TempDir) -> std::io::Result<()> {
    let input = dir.path().join("input");
    fs::create_dir(&input)?;

    // Text file
    let mut text_file = File::create(input.join("test.txt"))?;
    text_file.write_all(b"Hello, holographic world!\n")?;

    // JSON file
    let mut json_file = File::create(input.join("data.json"))?;
    json_file.write_all(b"{\"test\": true, \"value\": 42}\n")?;

    // Binary file
    let mut bin_file = File::create(input.join("binary.bin"))?;
    bin_file.write_all(&(0..=255).collect::<Vec<u8>>())?;

    // Subdirectory with file
    fs::create_dir(input.join("subdir"))?;
    let mut sub_file = File::create(input.join("subdir/nested.txt"))?;
    sub_file.write_all(b"Nested file content\n")?;

    Ok(())
}

#[test]
fn test_cli_ingest_and_extract() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    create_test_input(&temp_dir).expect("Failed to create test input");

    let input = temp_dir.path().join("input");
    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");
    let output = temp_dir.path().join("output");

    // Test ingest
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

    assert!(
        ingest_output.status.success(),
        "Ingest failed: {}",
        String::from_utf8_lossy(&ingest_output.stderr)
    );
    assert!(engram.exists(), "Engram file not created");
    assert!(manifest.exists(), "Manifest file not created");

    // Test extract
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

    assert!(
        extract_output.status.success(),
        "Extract failed: {}",
        String::from_utf8_lossy(&extract_output.stderr)
    );
    assert!(output.exists(), "Output directory not created");

    // Verify reconstructed files
    assert!(output.join("test.txt").exists(), "test.txt not extracted");
    assert!(output.join("data.json").exists(), "data.json not extracted");
    assert!(
        output.join("binary.bin").exists(),
        "binary.bin not extracted"
    );
    assert!(
        output.join("subdir/nested.txt").exists(),
        "nested.txt not extracted"
    );

    // Verify content matches
    let original_text = fs::read(input.join("test.txt")).unwrap();
    let extracted_text = fs::read(output.join("test.txt")).unwrap();
    assert_eq!(original_text, extracted_text, "Text file content mismatch");

    let original_bin = fs::read(input.join("binary.bin")).unwrap();
    let extracted_bin = fs::read(output.join("binary.bin")).unwrap();
    assert_eq!(original_bin, extracted_bin, "Binary file content mismatch");
}

#[test]
fn test_cli_query() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    create_test_input(&temp_dir).expect("Failed to create test input");

    let input = temp_dir.path().join("input");
    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");

    // Ingest first
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

    assert!(ingest_output.status.success());

    // Query with a file from the input
    let query_file = input.join("test.txt");
    let query_output = Command::new(embeddenator_bin())
        .args([
            "query",
            "-e",
            engram.to_str().unwrap(),
            "-q",
            query_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run query");

    assert!(
        query_output.status.success(),
        "Query failed: {}",
        String::from_utf8_lossy(&query_output.stderr)
    );

    let output_str = String::from_utf8_lossy(&query_output.stdout);
    assert!(
        output_str.contains("Similarity"),
        "Query output missing similarity"
    );
}

#[test]
fn test_cli_bundle_hier_produces_artifacts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    create_test_input(&temp_dir).expect("Failed to create test input");

    let input = temp_dir.path().join("input");
    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");

    // Ingest first
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
    assert!(ingest_output.status.success());

    let hier_manifest = temp_dir.path().join("hier.json");
    let sub_dir = temp_dir.path().join("sub_engrams");

    // Build hierarchical artifacts
    let bundle_output = Command::new(embeddenator_bin())
        .args([
            "bundle-hier",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "--out-hierarchical-manifest",
            hier_manifest.to_str().unwrap(),
            "--out-sub-engrams-dir",
            sub_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run bundle-hier");

    assert!(
        bundle_output.status.success(),
        "bundle-hier failed: {}",
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(hier_manifest.exists(), "hierarchical manifest not created");
    assert!(sub_dir.exists(), "sub-engrams dir not created");

    // Ensure query-text can run with hierarchical args.
    let query_output = Command::new(embeddenator_bin())
        .args([
            "query-text",
            "-e",
            engram.to_str().unwrap(),
            "--text",
            "Hello",
            "--hierarchical-manifest",
            hier_manifest.to_str().unwrap(),
            "--sub-engrams-dir",
            sub_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run query-text");

    assert!(
        query_output.status.success(),
        "query-text failed: {}",
        String::from_utf8_lossy(&query_output.stderr)
    );
}

#[test]
fn test_cli_version() {
    let output = Command::new(embeddenator_bin())
        .arg("--version")
        .output()
        .expect("Failed to run --version");

    assert!(output.status.success());
    let version_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        version_str.contains(env!("CARGO_PKG_VERSION")),
        "Version should be {}, got: {}",
        env!("CARGO_PKG_VERSION"),
        version_str
    );
}

#[test]
fn test_cli_help() {
    let output = Command::new(embeddenator_bin())
        .arg("--help")
        .output()
        .expect("Failed to run --help");

    assert!(output.status.success());
    let help_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        help_str.contains("ingest"),
        "Help should mention ingest command"
    );
    assert!(
        help_str.contains("extract"),
        "Help should mention extract command"
    );
    assert!(
        help_str.contains("query"),
        "Help should mention query command"
    );
}

#[test]
fn test_bit_perfect_reconstruction() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    // Create a larger binary file with specific pattern
    let mut test_data = Vec::new();
    for i in 0..10000 {
        test_data.push((i % 256) as u8);
    }

    let original_file = input.join("test_data.bin");
    fs::write(&original_file, &test_data).unwrap();

    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");
    let output = temp_dir.path().join("output");

    // Ingest
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

    assert!(ingest_output.status.success());

    // Extract
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

    assert!(extract_output.status.success());

    // Verify bit-perfect reconstruction
    let extracted_data = fs::read(output.join("test_data.bin")).unwrap();
    assert_eq!(
        test_data, extracted_data,
        "Data not reconstructed bit-perfectly"
    );
    assert_eq!(test_data.len(), extracted_data.len(), "Length mismatch");
}

#[test]
fn test_empty_directory_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");

    // Ingest empty directory
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

    assert!(
        ingest_output.status.success(),
        "Should handle empty directory: {}",
        String::from_utf8_lossy(&ingest_output.stderr)
    );
    assert!(engram.exists());
    assert!(manifest.exists());
}

#[test]
fn test_large_file_chunking() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input");
    fs::create_dir(&input).unwrap();

    // Create a file larger than the chunk size (4KB)
    let large_data: Vec<u8> = (0..20000).map(|i| (i % 256) as u8).collect();
    let large_file = input.join("large.bin");
    fs::write(&large_file, &large_data).unwrap();

    let engram = temp_dir.path().join("test.engram");
    let manifest = temp_dir.path().join("test.manifest.json");
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

    // Check manifest for multiple chunks
    let manifest_content = fs::read_to_string(&manifest).unwrap();
    assert!(
        manifest_content.contains("total_chunks"),
        "Manifest should contain chunk info"
    );

    // Extract and verify
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

    let extracted_data = fs::read(output.join("large.bin")).unwrap();
    assert_eq!(
        large_data, extracted_data,
        "Large file not reconstructed correctly"
    );
}
