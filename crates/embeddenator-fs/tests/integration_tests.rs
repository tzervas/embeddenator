// Copyright (c) 2024-2026 Tyler Zervas
// SPDX-License-Identifier: MIT

//! Integration tests for embeddenator-fs
//!
//! These tests verify end-to-end workflows including:
//! - Directory ingestion
//! - File extraction
//! - Bit-perfect reconstruction
//! - Incremental operations
//! - Hierarchical sub-engrams

use embeddenator_fs::EmbrFS;
use embeddenator_vsa::ReversibleVSAConfig;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper to create a test directory with various file types
fn create_test_directory(base: &Path) -> std::io::Result<()> {
    // Create directory structure
    fs::create_dir_all(base.join("dir1"))?;
    fs::create_dir_all(base.join("dir2/nested"))?;
    fs::create_dir_all(base.join("empty_dir"))?;

    // Create test files
    fs::write(base.join("file1.txt"), b"Hello, world!")?;
    fs::write(base.join("file2.log"), b"Log entry 1\nLog entry 2\n")?;
    fs::write(
        base.join("dir1/file3.dat"),
        b"Binary data: \x00\x01\x02\xFF",
    )?;
    fs::write(
        base.join("dir2/file4.md"),
        b"# Markdown\n\n## Section\n\nContent here.",
    )?;
    fs::write(
        base.join("dir2/nested/file5.json"),
        br#"{"key": "value", "number": 42}"#,
    )?;

    // Create a larger file (multiple chunks)
    let mut large_file = fs::File::create(base.join("large.bin"))?;
    for i in 0..1000 {
        writeln!(large_file, "Line {}: {}", i, "X".repeat(100))?;
    }

    Ok(())
}

/// Helper to compare directory contents recursively
fn compare_directories(original: &Path, extracted: &Path) -> std::io::Result<()> {
    let original_files = collect_files(original)?;
    let extracted_files = collect_files(extracted)?;

    // Check that all files exist
    assert_eq!(
        original_files.len(),
        extracted_files.len(),
        "File count mismatch"
    );

    for original_file in &original_files {
        let rel_path = original_file.strip_prefix(original).unwrap();
        let extracted_file = extracted.join(rel_path);

        assert!(
            extracted_file.exists(),
            "File missing: {:?}",
            extracted_file
        );

        // Compare file contents
        let original_content = fs::read(original_file)?;
        let extracted_content = fs::read(&extracted_file)?;

        assert_eq!(
            original_content, extracted_content,
            "Content mismatch for {:?}",
            rel_path
        );
    }

    Ok(())
}

/// Recursively collect all files in a directory
fn collect_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

#[test]
fn test_basic_ingest_extract() {
    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let extract_dir = temp_dir.path().join("extracted");
    let engram_path = temp_dir.path().join("test.engram");

    // Create test files
    create_test_directory(&source_dir).unwrap();

    // Ingest directory
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();

    // Save engram and manifest
    fs.save_engram(&engram_path).unwrap();
    let manifest_path = engram_path.with_extension("manifest.json");
    fs.save_manifest(&manifest_path).unwrap();

    // Verify engram file exists
    assert!(engram_path.exists(), "Engram file not created");

    // Load engram
    let loaded_engram = EmbrFS::load_engram(&engram_path).unwrap();
    let mut loaded_fs = EmbrFS::new();
    loaded_fs.engram = loaded_engram;

    // Load manifest
    let manifest_path = engram_path.with_extension("manifest.json");
    if manifest_path.exists() {
        loaded_fs.manifest = EmbrFS::load_manifest(&manifest_path).unwrap();
    }

    // Create output directory and extract all files
    fs::create_dir_all(&extract_dir).unwrap();
    let config = ReversibleVSAConfig::default();
    EmbrFS::extract(
        &loaded_fs.engram,
        &loaded_fs.manifest,
        &extract_dir,
        false,
        &config,
    )
    .unwrap();

    // Verify bit-perfect reconstruction
    compare_directories(&source_dir, &extract_dir).unwrap();
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("empty");
    let extract_dir = temp_dir.path().join("extracted");

    // Create empty directory
    fs::create_dir(&source_dir).unwrap();

    // Ingest
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();

    // Create output directory and extract
    fs::create_dir_all(&extract_dir).unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, &extract_dir, false, &config).unwrap();

    // Verify directory exists but is empty
    assert!(extract_dir.exists());
    let files = collect_files(&extract_dir).unwrap();
    assert_eq!(files.len(), 0, "Empty directory should have no files");
}

#[test]
fn test_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("single");
    let extract_dir = temp_dir.path().join("extracted");

    // Create single file
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("single.txt"), b"Single file content").unwrap();

    // Ingest and extract
    let mut fs_instance = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs_instance
        .ingest_directory(&source_dir, false, &config)
        .unwrap();
    EmbrFS::extract(
        &fs_instance.engram,
        &fs_instance.manifest,
        &extract_dir,
        false,
        &config,
    )
    .unwrap();

    // Verify
    compare_directories(&source_dir, &extract_dir).unwrap();
}

#[test]
fn test_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("large");
    let extract_dir = temp_dir.path().join("extracted");

    // Create large file (multiple chunks)
    fs::create_dir(&source_dir).unwrap();
    let large_data = vec![0xABu8; 100_000]; // 100KB
    fs::write(source_dir.join("large.bin"), &large_data).unwrap();

    // Ingest and extract
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, &extract_dir, false, &config).unwrap();

    // Verify bit-perfect reconstruction
    let extracted_data = fs::read(extract_dir.join("large.bin")).unwrap();
    assert_eq!(
        large_data, extracted_data,
        "Large file reconstruction failed"
    );
}

#[test]
fn test_special_characters_in_filenames() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("special");
    let extract_dir = temp_dir.path().join("extracted");

    // Create files with special characters
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("file with spaces.txt"), b"content1").unwrap();
    fs::write(source_dir.join("file-dash.txt"), b"content2").unwrap();
    fs::write(source_dir.join("file_underscore.txt"), b"content3").unwrap();
    fs::write(source_dir.join("file.multiple.dots.txt"), b"content4").unwrap();

    // Ingest and extract
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, &extract_dir, false, &config).unwrap();

    // Verify
    compare_directories(&source_dir, &extract_dir).unwrap();
}

#[test]
fn test_binary_data() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("binary");
    let extract_dir = temp_dir.path().join("extracted");

    // Create file with various binary patterns
    fs::create_dir(&source_dir).unwrap();
    let mut binary_data = Vec::new();
    for i in 0..256 {
        binary_data.push(i as u8);
    }
    fs::write(source_dir.join("binary.bin"), &binary_data).unwrap();

    // Ingest and extract
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, &extract_dir, false, &config).unwrap();

    // Verify bit-perfect reconstruction
    let extracted_data = fs::read(extract_dir.join("binary.bin")).unwrap();
    assert_eq!(
        binary_data, extracted_data,
        "Binary data reconstruction failed"
    );
}

#[test]
fn test_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("nested");
    let extract_dir = temp_dir.path().join("extracted");

    // Create deeply nested structure
    fs::create_dir_all(source_dir.join("a/b/c/d/e")).unwrap();
    fs::write(source_dir.join("a/file1.txt"), b"level1").unwrap();
    fs::write(source_dir.join("a/b/file2.txt"), b"level2").unwrap();
    fs::write(source_dir.join("a/b/c/file3.txt"), b"level3").unwrap();
    fs::write(source_dir.join("a/b/c/d/file4.txt"), b"level4").unwrap();
    fs::write(source_dir.join("a/b/c/d/e/file5.txt"), b"level5").unwrap();

    // Ingest and extract
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, &extract_dir, false, &config).unwrap();

    // Verify
    compare_directories(&source_dir, &extract_dir).unwrap();
}

#[test]
fn test_correction_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");

    // Create test files
    create_test_directory(&source_dir).unwrap();

    // Ingest
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();

    // Get correction statistics
    let stats = fs.correction_stats();

    // Verify statistics are reasonable
    assert!(stats.total_chunks > 0, "Should have at least one chunk");
    assert!(
        stats.perfect_chunks + stats.corrected_chunks == stats.total_chunks,
        "Chunks should be either perfect or corrected"
    );

    // Print statistics for informational purposes (overhead varies by VSA config)
    println!(
        "Correction stats: {:.1}% perfect, {:.2}% overhead",
        stats.perfect_ratio * 100.0,
        stats.correction_ratio * 100.0
    );

    // Just verify stats are computed (actual overhead depends on VSA dimensionality)
    assert!(
        stats.correction_ratio >= 0.0,
        "Correction ratio should be non-negative"
    );
    assert!(
        stats.correction_ratio <= 1.5,
        "Correction ratio should be reasonable (<=150%)"
    );
}

#[test]
fn test_roundtrip_preserves_content() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let extract_dir1 = temp_dir.path().join("extracted1");
    let extract_dir2 = temp_dir.path().join("extracted2");

    // Create test files
    create_test_directory(&source_dir).unwrap();

    // First roundtrip
    let mut fs1 = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs1.ingest_directory(&source_dir, false, &config).unwrap();
    EmbrFS::extract(&fs1.engram, &fs1.manifest, &extract_dir1, false, &config).unwrap();

    // Second roundtrip (ingest extracted files)
    let mut fs2 = EmbrFS::new();
    fs2.ingest_directory(&extract_dir1, false, &config).unwrap();
    EmbrFS::extract(&fs2.engram, &fs2.manifest, &extract_dir2, false, &config).unwrap();

    // Both extractions should match original
    compare_directories(&source_dir, &extract_dir1).unwrap();
    compare_directories(&source_dir, &extract_dir2).unwrap();
}

#[test]
fn test_manifest_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let manifest_path = temp_dir.path().join("test.manifest.json");

    // Create test files
    create_test_directory(&source_dir).unwrap();

    // Ingest and save manifest
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fs.ingest_directory(&source_dir, false, &config).unwrap();
    fs.save_manifest(&manifest_path).unwrap();

    // Load manifest
    let loaded_manifest = EmbrFS::load_manifest(&manifest_path).unwrap();

    // Verify manifest contents
    assert_eq!(
        fs.manifest.files.len(),
        loaded_manifest.files.len(),
        "Manifest file count mismatch"
    );

    // Verify file entries match
    for (original_entry, loaded_entry) in fs.manifest.files.iter().zip(loaded_manifest.files.iter())
    {
        assert_eq!(original_entry.path, loaded_entry.path);
        assert_eq!(original_entry.size, loaded_entry.size);
        assert_eq!(original_entry.is_text, loaded_entry.is_text);
    }
}
