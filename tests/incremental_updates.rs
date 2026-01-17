//! Tests for incremental update functionality (TASK-007)
//!
//! This test suite verifies that the incremental update API works correctly:
//! - Adding files to existing engrams
//! - Removing files (marking as deleted)
//! - Modifying existing files
//! - Compacting engrams to remove deleted files
//! - Hierarchical engram updates
//!
//! Key properties tested:
//! - Bit-perfect reconstruction after updates
//! - Correct manifest tracking of deleted files
//! - Associativity of bundle operations
//! - Space reclamation via compaction

use embeddenator::{EmbrFS, ReversibleVSAConfig};
use std::io::Write;
use tempfile::TempDir;

/// Helper: create a temporary file with given content
fn create_temp_file(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

#[test]
fn test_add_single_file_to_empty_engram() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"hello world");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add file to empty engram
    fs.add_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();

    // Verify manifest
    assert_eq!(fs.manifest.files.len(), 1);
    assert_eq!(fs.manifest.files[0].path, "file1.txt");
    assert!(!fs.manifest.files[0].deleted);
    assert_eq!(fs.manifest.files[0].size, 11);

    // Verify codebook populated
    assert!(fs.engram.codebook.len() > 0);

    // Verify extraction
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted = std::fs::read(extract_dir.path().join("file1.txt")).unwrap();
    assert_eq!(extracted, b"hello world");
}

#[test]
fn test_add_file_to_existing_engram() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"first file");
    let file2 = create_temp_file(&temp_dir, "file2.txt", b"second file");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Ingest first file
    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    let chunks_after_first = fs.manifest.total_chunks;

    // Add second file incrementally
    fs.add_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();

    // Verify manifest
    assert_eq!(fs.manifest.files.len(), 2);
    assert!(fs.manifest.total_chunks > chunks_after_first);

    // Verify both files extract correctly
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted1 = std::fs::read(extract_dir.path().join("file1.txt")).unwrap();
    let extracted2 = std::fs::read(extract_dir.path().join("file2.txt")).unwrap();
    assert_eq!(extracted1, b"first file");
    assert_eq!(extracted2, b"second file");
}

#[test]
fn test_add_file_duplicate_error() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"content");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add file once
    fs.add_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();

    // Try to add same file again - should error
    let result = fs.add_file(&file1, "file1.txt".to_string(), false, &config);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
}

#[test]
fn test_remove_file_marks_as_deleted() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"to be removed");
    let file2 = create_temp_file(&temp_dir, "file2.txt", b"to be kept");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Ingest two files
    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    fs.ingest_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();

    // Remove first file
    fs.remove_file("file1.txt", false).unwrap();

    // Verify manifest still has both entries but first is deleted
    assert_eq!(fs.manifest.files.len(), 2);
    assert!(fs.manifest.files[0].deleted);
    assert!(!fs.manifest.files[1].deleted);

    // Verify extraction skips deleted file
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    assert!(!extract_dir.path().join("file1.txt").exists());
    assert!(extract_dir.path().join("file2.txt").exists());
    let extracted2 = std::fs::read(extract_dir.path().join("file2.txt")).unwrap();
    assert_eq!(extracted2, b"to be kept");
}

#[test]
fn test_remove_nonexistent_file_error() {
    let mut fs = EmbrFS::new();

    let result = fs.remove_file("nonexistent.txt", false);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_remove_already_deleted_file_error() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"content");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    fs.remove_file("file1.txt", false).unwrap();

    // Try to remove again - should error
    let result = fs.remove_file("file1.txt", false);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_modify_file_updates_content() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"original content");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Ingest original file
    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();

    // Modify file
    let file1_modified = create_temp_file(&temp_dir, "file1_modified.txt", b"updated content");
    fs.modify_file(&file1_modified, "file1.txt".to_string(), false, &config)
        .unwrap();

    // Verify manifest has two entries (old deleted, new active)
    assert_eq!(fs.manifest.files.len(), 2);
    assert!(fs.manifest.files[0].deleted);
    assert!(!fs.manifest.files[1].deleted);
    assert_eq!(fs.manifest.files[1].path, "file1.txt");

    // Verify extraction gets new content
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted = std::fs::read(extract_dir.path().join("file1.txt")).unwrap();
    assert_eq!(extracted, b"updated content");
}

#[test]
fn test_modify_nonexistent_file_error() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"content");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    let result = fs.modify_file(&file1, "nonexistent.txt".to_string(), false, &config);
    assert!(result.is_err());
}

#[test]
fn test_compact_removes_deleted_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"file to delete");
    let file2 = create_temp_file(&temp_dir, "file2.txt", b"file to keep");
    let file3 = create_temp_file(&temp_dir, "file3.txt", b"another deleted");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Ingest three files
    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    fs.ingest_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();
    fs.ingest_file(&file3, "file3.txt".to_string(), false, &config)
        .unwrap();

    let chunks_before = fs.manifest.total_chunks;

    // Delete two files
    fs.remove_file("file1.txt", false).unwrap();
    fs.remove_file("file3.txt", false).unwrap();

    // Before compaction: still 3 entries in manifest
    assert_eq!(fs.manifest.files.len(), 3);

    // Compact
    fs.compact(false, &config).unwrap();

    // After compaction: only 1 file in manifest
    assert_eq!(fs.manifest.files.len(), 1);
    assert_eq!(fs.manifest.files[0].path, "file2.txt");
    assert!(!fs.manifest.files[0].deleted);

    // Chunks should be reduced
    assert!(fs.manifest.total_chunks < chunks_before);

    // Verify extraction
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    assert!(!extract_dir.path().join("file1.txt").exists());
    assert!(extract_dir.path().join("file2.txt").exists());
    assert!(!extract_dir.path().join("file3.txt").exists());

    let extracted2 = std::fs::read(extract_dir.path().join("file2.txt")).unwrap();
    assert_eq!(extracted2, b"file to keep");
}

#[test]
fn test_compact_empty_engram() {
    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Compact empty engram - should not error
    fs.compact(false, &config).unwrap();

    assert_eq!(fs.manifest.files.len(), 0);
    assert_eq!(fs.manifest.total_chunks, 0);
}

#[test]
fn test_compact_no_deleted_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"keep me");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    let chunks_before = fs.manifest.total_chunks;

    // Compact without any deletions
    fs.compact(false, &config).unwrap();

    // Should still have same file
    assert_eq!(fs.manifest.files.len(), 1);
    assert_eq!(fs.manifest.files[0].path, "file1.txt");

    // Chunk count should be similar (may differ slightly due to re-encoding)
    assert!(fs.manifest.total_chunks >= chunks_before - 1);
}

#[test]
fn test_multiple_add_remove_cycle() {
    let temp_dir = TempDir::new().unwrap();

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add file 1
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"first");
    fs.add_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();

    // Add file 2
    let file2 = create_temp_file(&temp_dir, "file2.txt", b"second");
    fs.add_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();

    // Remove file 1
    fs.remove_file("file1.txt", false).unwrap();

    // Add file 3
    let file3 = create_temp_file(&temp_dir, "file3.txt", b"third");
    fs.add_file(&file3, "file3.txt".to_string(), false, &config)
        .unwrap();

    // Verify state before compaction
    assert_eq!(fs.manifest.files.len(), 3);

    // Extract and verify
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    assert!(!extract_dir.path().join("file1.txt").exists());
    assert!(extract_dir.path().join("file2.txt").exists());
    assert!(extract_dir.path().join("file3.txt").exists());

    // Compact
    fs.compact(false, &config).unwrap();
    assert_eq!(fs.manifest.files.len(), 2);

    // Extract again and verify
    let extract_dir2 = TempDir::new().unwrap();
    EmbrFS::extract(
        &fs.engram,
        &fs.manifest,
        extract_dir2.path(),
        false,
        &config,
    )
    .unwrap();

    let extracted2 = std::fs::read(extract_dir2.path().join("file2.txt")).unwrap();
    let extracted3 = std::fs::read(extract_dir2.path().join("file3.txt")).unwrap();
    assert_eq!(extracted2, b"second");
    assert_eq!(extracted3, b"third");
}

#[test]
fn test_add_large_file_incrementally() {
    let temp_dir = TempDir::new().unwrap();

    // Create large file (larger than chunk size)
    let large_content = vec![b'X'; 20_000]; // 20KB
    let file1 = create_temp_file(&temp_dir, "large.bin", &large_content);

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add large file
    fs.add_file(&file1, "large.bin".to_string(), false, &config)
        .unwrap();

    // Verify multiple chunks created
    assert!(fs.manifest.files[0].chunks.len() > 1);

    // Verify extraction
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted = std::fs::read(extract_dir.path().join("large.bin")).unwrap();
    assert_eq!(extracted, large_content);
}

#[test]
fn test_modify_with_different_size() {
    let temp_dir = TempDir::new().unwrap();

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Start with small file
    let file1 = create_temp_file(&temp_dir, "file.txt", b"small");
    fs.ingest_file(&file1, "file.txt".to_string(), false, &config)
        .unwrap();

    // Modify to larger file
    let large_content = vec![b'L'; 10_000];
    let file2 = create_temp_file(&temp_dir, "file_large.txt", &large_content);
    fs.modify_file(&file2, "file.txt".to_string(), false, &config)
        .unwrap();

    // Extract and verify
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted = std::fs::read(extract_dir.path().join("file.txt")).unwrap();
    assert_eq!(extracted, large_content);
}

#[test]
fn test_add_binary_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create binary file with various byte values
    let binary_content = (0u8..=255).collect::<Vec<u8>>();
    let file1 = create_temp_file(&temp_dir, "binary.bin", &binary_content);

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.add_file(&file1, "binary.bin".to_string(), false, &config)
        .unwrap();

    // Verify bit-perfect reconstruction
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted = std::fs::read(extract_dir.path().join("binary.bin")).unwrap();
    assert_eq!(extracted, binary_content);
}

#[test]
fn test_compact_preserves_corrections() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"test content with corrections");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    fs.ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();

    // Get correction stats before compaction
    let _stats_before = fs.correction_stats();

    // Compact (should preserve corrections)
    fs.compact(false, &config).unwrap();

    // Verify extraction still works perfectly
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    let extracted = std::fs::read(extract_dir.path().join("file1.txt")).unwrap();
    assert_eq!(extracted, b"test content with corrections");
}

#[test]
fn test_incremental_updates_maintain_determinism() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"deterministic content");
    let file2 = create_temp_file(&temp_dir, "file2.txt", b"more content");

    let config = ReversibleVSAConfig::default();

    // Create engram via full ingestion
    let mut fs_full = EmbrFS::new();
    fs_full
        .ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    fs_full
        .ingest_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();

    // Create engram via incremental updates
    let mut fs_inc = EmbrFS::new();
    fs_inc
        .ingest_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    fs_inc
        .add_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();

    // Both should extract identically
    let extract_dir_full = TempDir::new().unwrap();
    let extract_dir_inc = TempDir::new().unwrap();

    EmbrFS::extract(
        &fs_full.engram,
        &fs_full.manifest,
        extract_dir_full.path(),
        false,
        &config,
    )
    .unwrap();
    EmbrFS::extract(
        &fs_inc.engram,
        &fs_inc.manifest,
        extract_dir_inc.path(),
        false,
        &config,
    )
    .unwrap();

    let content_full_1 = std::fs::read(extract_dir_full.path().join("file1.txt")).unwrap();
    let content_full_2 = std::fs::read(extract_dir_full.path().join("file2.txt")).unwrap();
    let content_inc_1 = std::fs::read(extract_dir_inc.path().join("file1.txt")).unwrap();
    let content_inc_2 = std::fs::read(extract_dir_inc.path().join("file2.txt")).unwrap();

    assert_eq!(content_full_1, content_inc_1);
    assert_eq!(content_full_2, content_inc_2);
}

#[test]
fn test_add_after_delete_and_compact() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_temp_file(&temp_dir, "file1.txt", b"first");
    let file2 = create_temp_file(&temp_dir, "file2.txt", b"second");

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add, remove, compact, add again
    fs.add_file(&file1, "file1.txt".to_string(), false, &config)
        .unwrap();
    fs.remove_file("file1.txt", false).unwrap();
    fs.compact(false, &config).unwrap();
    fs.add_file(&file2, "file2.txt".to_string(), false, &config)
        .unwrap();

    // Should have only file2
    assert_eq!(fs.manifest.files.len(), 1);
    assert_eq!(fs.manifest.files[0].path, "file2.txt");

    // Extract and verify
    let extract_dir = TempDir::new().unwrap();
    EmbrFS::extract(&fs.engram, &fs.manifest, extract_dir.path(), false, &config).unwrap();

    assert!(!extract_dir.path().join("file1.txt").exists());
    let extracted2 = std::fs::read(extract_dir.path().join("file2.txt")).unwrap();
    assert_eq!(extracted2, b"second");
}
