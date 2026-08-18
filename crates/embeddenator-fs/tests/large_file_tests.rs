//! Large file tests for VersionedEmbrFS
//!
//! Tests multi-MB to GB file handling, various content types, and production-grade scenarios.
//! These tests use realistic memory patterns rather than allocating huge arrays.

use embeddenator_fs::VersionedEmbrFS;
use std::sync::Arc;
use std::thread;

// Helper to create test data of various types
fn create_test_data(size_mb: usize, pattern: TestDataPattern) -> Vec<u8> {
    let size_bytes = size_mb * 1024 * 1024;

    match pattern {
        TestDataPattern::Zeros => vec![0u8; size_bytes],
        TestDataPattern::Ones => vec![0xFF; size_bytes],
        TestDataPattern::Sequential => (0..size_bytes).map(|i| (i % 256) as u8).collect(),
        TestDataPattern::Random => {
            // Simple deterministic "random" pattern
            (0..size_bytes)
                .map(|i| ((i.wrapping_mul(2654435761)) % 256) as u8)
                .collect()
        }
        TestDataPattern::Compressible => {
            // Repeating pattern that compresses well
            let pattern = b"The quick brown fox jumps over the lazy dog. ";
            (0..size_bytes)
                .map(|i| pattern[i % pattern.len()])
                .collect()
        }
        TestDataPattern::Text => {
            // ASCII text pattern
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 \n";
            (0..size_bytes).map(|i| chars[i % chars.len()]).collect()
        }
    }
}

// Verify data with sampling for large files
fn verify_data_sampled(data: &[u8], expected_pattern: TestDataPattern, sample_points: usize) {
    let len = data.len();
    let stride = len / sample_points;

    for i in 0..sample_points {
        let pos = i * stride;
        if pos >= len {
            break;
        }
        let expected = match expected_pattern {
            TestDataPattern::Zeros => 0u8,
            TestDataPattern::Ones => 0xFF,
            TestDataPattern::Sequential => (pos % 256) as u8,
            TestDataPattern::Random => ((pos.wrapping_mul(2654435761)) % 256) as u8,
            TestDataPattern::Compressible => {
                let pattern = b"The quick brown fox jumps over the lazy dog. ";
                pattern[pos % pattern.len()]
            }
            TestDataPattern::Text => {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 \n";
                chars[pos % chars.len()]
            }
        };
        assert_eq!(
            data[pos], expected,
            "Mismatch at position {} (sample {})",
            pos, i
        );
    }
}

#[derive(Clone, Copy)]
enum TestDataPattern {
    Zeros,
    Ones,
    Sequential,
    Random,
    Compressible,
    Text,
}

#[test]
fn test_10mb_file() {
    let fs = VersionedEmbrFS::new();

    // Create 10MB file
    let data = create_test_data(10, TestDataPattern::Random);
    let version = fs.write_file("large_10mb.bin", &data, None).unwrap();

    // Read it back
    let (read_data, read_version) = fs.read_file("large_10mb.bin").unwrap();
    assert_eq!(read_version, version);
    assert_eq!(read_data.len(), data.len());
    assert_eq!(read_data, data);
}

#[test]
#[ignore] // Slow: >15 minutes in CI
fn test_50mb_file() {
    let fs = VersionedEmbrFS::new();

    // Create 50MB file
    let data = create_test_data(50, TestDataPattern::Sequential);
    let version = fs.write_file("large_50mb.bin", &data, None).unwrap();

    // Read it back and verify with sampling
    let (read_data, read_version) = fs.read_file("large_50mb.bin").unwrap();
    assert_eq!(read_version, version);
    assert_eq!(read_data.len(), data.len());
    verify_data_sampled(&read_data, TestDataPattern::Sequential, 1000);

    // Verify stats
    let stats = fs.stats();
    assert_eq!(stats.active_files, 1);
}

#[test]
#[ignore] // Slow: >15 minutes in CI
fn test_100mb_file() {
    let fs = VersionedEmbrFS::new();

    // Create 100MB file
    let data = create_test_data(100, TestDataPattern::Compressible);
    let version = fs.write_file("large_100mb.bin", &data, None).unwrap();

    // Read it back and verify with sampling
    let (read_data, _) = fs.read_file("large_100mb.bin").unwrap();
    assert_eq!(read_data.len(), data.len());
    verify_data_sampled(&read_data, TestDataPattern::Compressible, 1000);

    // Update with version check
    let new_data = create_test_data(100, TestDataPattern::Text);
    let new_version = fs
        .write_file("large_100mb.bin", &new_data, Some(version))
        .unwrap();
    assert!(new_version > version);

    // Verify update
    let (updated_data, _) = fs.read_file("large_100mb.bin").unwrap();
    verify_data_sampled(&updated_data, TestDataPattern::Text, 1000);
}

#[test]
#[ignore] // Memory intensive: 200MB x2 in memory
fn test_200mb_file() {
    let fs = VersionedEmbrFS::new();

    // Create 200MB file
    let data = create_test_data(200, TestDataPattern::Random);
    fs.write_file("large_200mb.bin", &data, None).unwrap();

    // Read it back and verify with sampling
    let (read_data, _) = fs.read_file("large_200mb.bin").unwrap();
    assert_eq!(read_data.len(), data.len());
    verify_data_sampled(&read_data, TestDataPattern::Random, 2000);
}

#[test]
fn test_multiple_medium_files_concurrent() {
    let fs = Arc::new(VersionedEmbrFS::new());
    let mut handles = vec![];

    // Create 10 x 10MB files concurrently
    for i in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            let data = create_test_data(10, TestDataPattern::Sequential);
            let path = format!("concurrent_medium_{}.bin", i);
            fs_clone.write_file(&path, &data, None).unwrap();

            // Verify with sampling
            let (read_data, _) = fs_clone.read_file(&path).unwrap();
            assert_eq!(read_data.len(), data.len());
            verify_data_sampled(&read_data, TestDataPattern::Sequential, 100);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all files exist
    let files = fs.list_files();
    assert_eq!(files.len(), 10);
}

#[test]
fn test_various_content_types() {
    let fs = VersionedEmbrFS::new();

    let test_cases = vec![
        ("zeros_10mb.bin", 10, TestDataPattern::Zeros),
        ("ones_10mb.bin", 10, TestDataPattern::Ones),
        ("sequential_10mb.bin", 10, TestDataPattern::Sequential),
        ("random_10mb.bin", 10, TestDataPattern::Random),
        ("compressible_10mb.txt", 10, TestDataPattern::Compressible),
        ("text_10mb.txt", 10, TestDataPattern::Text),
    ];

    for (path, size_mb, pattern) in test_cases {
        let data = create_test_data(size_mb, pattern);
        fs.write_file(path, &data, None).unwrap();

        let (read_data, _) = fs.read_file(path).unwrap();
        assert_eq!(read_data.len(), data.len());
        verify_data_sampled(&read_data, pattern, 100);
    }

    // Verify all files
    let files = fs.list_files();
    assert_eq!(files.len(), 6);
}

#[test]
fn test_update_large_file() {
    let fs = VersionedEmbrFS::new();

    // Create initial 20MB file
    let data1 = create_test_data(20, TestDataPattern::Sequential);
    let v1 = fs.write_file("update_test.bin", &data1, None).unwrap();

    // Update to 30MB
    let data2 = create_test_data(30, TestDataPattern::Random);
    let v2 = fs.write_file("update_test.bin", &data2, Some(v1)).unwrap();
    assert!(v2 > v1);

    // Update to 15MB (shrink)
    let data3 = create_test_data(15, TestDataPattern::Compressible);
    let v3 = fs.write_file("update_test.bin", &data3, Some(v2)).unwrap();
    assert!(v3 > v2);

    // Verify final content
    let (read_data, _) = fs.read_file("update_test.bin").unwrap();
    assert_eq!(read_data.len(), data3.len());
    verify_data_sampled(&read_data, TestDataPattern::Compressible, 100);
}

#[test]
#[ignore] // Slow: >15 minutes in CI
fn test_concurrent_read_large_file() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create 50MB file
    let data = create_test_data(50, TestDataPattern::Sequential);
    fs.write_file("shared_large.bin", &data, None).unwrap();

    let mut handles = vec![];

    // 20 threads reading concurrently
    for _ in 0..20 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            let (read_data, _) = fs_clone.read_file("shared_large.bin").unwrap();
            assert_eq!(read_data.len(), 50 * 1024 * 1024);
            verify_data_sampled(&read_data, TestDataPattern::Sequential, 100);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
#[ignore] // Slow: >5 minutes in CI
fn test_mixed_size_files() {
    let fs = VersionedEmbrFS::new();

    // Mix of small, medium, and large files
    let files = vec![
        ("tiny_1kb.bin", 0, TestDataPattern::Random), // Special case
        ("small_5mb.bin", 5, TestDataPattern::Text),
        ("medium_20mb.bin", 20, TestDataPattern::Sequential),
        ("large_50mb.bin", 50, TestDataPattern::Compressible),
        ("huge_100mb.bin", 100, TestDataPattern::Zeros),
    ];

    for (path, size_mb, pattern) in files {
        let data = if size_mb == 0 {
            vec![42u8; 1024] // 1KB for tiny file
        } else {
            create_test_data(size_mb, pattern)
        };

        fs.write_file(path, &data, None).unwrap();
    }

    // Verify all files
    let stats = fs.stats();
    assert_eq!(stats.active_files, 5);
}

// Simulated binary format tests
mod binary_formats {
    use super::*;

    #[test]
    fn test_image_like_data() {
        let fs = VersionedEmbrFS::new();

        // Simulate a large image file (e.g., 10MB uncompressed RGBA)
        // Images typically have local coherence
        let mut data = Vec::with_capacity(10 * 1024 * 1024);
        for _ in 0..(10 * 1024 * 1024 / 4) {
            // RGBA pixel pattern
            data.extend_from_slice(&[128, 128, 255, 255]);
        }

        fs.write_file("image_10mb.rgba", &data, None).unwrap();
        let (read_data, _) = fs.read_file("image_10mb.rgba").unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_video_like_data() {
        let fs = VersionedEmbrFS::new();

        // Simulate video frames (already compressed, so more random)
        let data = create_test_data(20, TestDataPattern::Random);
        fs.write_file("video_20mb.h264", &data, None).unwrap();

        let (read_data, _) = fs.read_file("video_20mb.h264").unwrap();
        verify_data_sampled(&read_data, TestDataPattern::Random, 200);
    }

    #[test]
    fn test_compressed_archive() {
        let fs = VersionedEmbrFS::new();

        // Simulate compressed data (high entropy)
        let data = create_test_data(15, TestDataPattern::Random);
        fs.write_file("archive_15mb.zip", &data, None).unwrap();

        let (read_data, _) = fs.read_file("archive_15mb.zip").unwrap();
        verify_data_sampled(&read_data, TestDataPattern::Random, 150);
    }

    #[test]
    fn test_encrypted_data() {
        let fs = VersionedEmbrFS::new();

        // Encrypted data appears random
        let data = create_test_data(10, TestDataPattern::Random);
        fs.write_file("encrypted_10mb.enc", &data, None).unwrap();

        let (read_data, _) = fs.read_file("encrypted_10mb.enc").unwrap();
        verify_data_sampled(&read_data, TestDataPattern::Random, 100);
    }

    #[test]
    fn test_database_file() {
        let fs = VersionedEmbrFS::new();

        // Simulate database file with mixed patterns
        // Header + records + index
        let mut data = Vec::new();

        // Header (1KB)
        data.extend_from_slice(&vec![0xFF; 1024]);

        // Records (sequential IDs)
        for i in 0..10000 {
            let record = format!("RECORD_{:08}|DATA_FIELD_VALUE_{}", i, i);
            data.extend_from_slice(record.as_bytes());
            data.push(b'\n');
        }

        fs.write_file("database_file.db", &data, None).unwrap();
        let (read_data, _) = fs.read_file("database_file.db").unwrap();
        assert_eq!(read_data.len(), data.len());

        // Verify header
        assert_eq!(&read_data[0..1024], &vec![0xFF; 1024][..]);
    }
}

// Log file simulation
#[test]
fn test_append_like_operations() {
    let fs = VersionedEmbrFS::new();

    // Simulate log file growth
    let mut accumulated = Vec::new();

    for i in 0..50 {
        let log_entry = format!(
            "[2024-01-{:02}T10:{}:00Z] INFO: Log entry number {} with detailed data about operation status and metrics\n",
            (i % 28) + 1,
            i % 60,
            i
        );
        accumulated.extend_from_slice(log_entry.as_bytes());

        let _current_version = if i == 0 {
            fs.write_file("logfile.log", &accumulated, None).unwrap()
        } else {
            let (_, version) = fs.read_file("logfile.log").unwrap();
            fs.write_file("logfile.log", &accumulated, Some(version))
                .unwrap()
        };

        // Verify content after each "append"
        let (read_data, _) = fs.read_file("logfile.log").unwrap();
        assert_eq!(read_data, accumulated);
    }

    // Final log should have 50 entries
    let (final_data, _) = fs.read_file("logfile.log").unwrap();
    let lines = final_data.iter().filter(|&&b| b == b'\n').count();
    assert_eq!(lines, 50);
}

// Stress test with many chunks
#[test]
#[ignore] // Slow: >15 minutes in CI
fn test_high_chunk_count_file() {
    let fs = VersionedEmbrFS::new();

    // 50MB file will create ~12,800 chunks at 4KB per chunk
    let data = create_test_data(50, TestDataPattern::Sequential);
    fs.write_file("many_chunks.bin", &data, None).unwrap();

    let (read_data, _) = fs.read_file("many_chunks.bin").unwrap();
    assert_eq!(read_data.len(), data.len());
    verify_data_sampled(&read_data, TestDataPattern::Sequential, 1000);

    // Verify stats show many chunks
    let stats = fs.stats();
    assert!(stats.total_chunks > 10000);
}
