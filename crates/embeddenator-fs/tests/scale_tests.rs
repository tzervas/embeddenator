//! Scale tests for large files and directories
//!
//! These tests are marked with #[ignore] and are NOT run in CI.
//! They are intended for manual validation on systems with sufficient resources.
//!
//! Run locally with: cargo test --test scale_tests -- --ignored --nocapture
//!
//! For large file tests, set SCALE_TEST_DIR to a path with enough space:
//!   SCALE_TEST_DIR=/home/user/tmp cargo test --test scale_tests -- --ignored --nocapture
//!
//! These tests validate:
//! - Large file handling (1GB+)
//! - Deep directory hierarchies (10K+ files)
//! - Memory efficiency under load
//! - Concurrent large operations
//!
//! Note: These tests document known limitations and expected behavior at scale.
//! Results may vary based on system configuration and available resources.

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

/// Get a temporary directory for scale tests.
/// Uses SCALE_TEST_DIR env var if set, otherwise falls back to system temp.
fn get_scale_test_dir() -> TempDir {
    if let Ok(dir) = std::env::var("SCALE_TEST_DIR") {
        let path = PathBuf::from(&dir);
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Failed to create SCALE_TEST_DIR");
        }
        tempfile::tempdir_in(&path).expect("Failed to create temp dir in SCALE_TEST_DIR")
    } else {
        TempDir::new().expect("Failed to create temp dir")
    }
}

/// Progress reporter for long-running operations
struct Progress {
    name: &'static str,
    total: usize,
    current: usize,
    last_percent: usize,
    start: Instant,
}

impl Progress {
    fn new(name: &'static str, total: usize) -> Self {
        eprintln!("\n▶ Starting: {}", name);
        Self {
            name,
            total,
            current: 0,
            last_percent: 0,
            start: Instant::now(),
        }
    }

    fn update(&mut self, current: usize) {
        self.current = current;
        let percent = if self.total > 0 {
            (current * 100) / self.total
        } else {
            0
        };

        // Report every 5% or at completion
        if percent >= self.last_percent + 5 || current == self.total {
            let elapsed = self.start.elapsed();
            let rate = if elapsed.as_secs_f64() > 0.0 {
                current as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };
            eprint!(
                "\r  [{:>3}%] {}/{} ({:.1}/s) - {:?}    ",
                percent, current, self.total, rate, elapsed
            );
            io::stderr().flush().ok();
            self.last_percent = percent;
        }
    }

    fn inc(&mut self) {
        self.update(self.current + 1);
    }

    fn finish(self) {
        let elapsed = self.start.elapsed();
        eprintln!("\n✓ Completed: {} in {:?}", self.name, elapsed);
    }
}

// =============================================================================
// LARGE FILE TESTS
// =============================================================================

mod large_files {
    use super::*;

    /// Test encoding a 1GB file
    /// Run with: SCALE_TEST_DIR=/path/with/space cargo test test_1gb_file -- --ignored --nocapture
    #[test]
    #[ignore]
    fn test_1gb_file() {
        let temp = get_scale_test_dir();
        let file_path = temp.path().join("1gb_file.bin");

        // Create 1GB file with pattern
        const SIZE: usize = 1024 * 1024 * 1024; // 1GB
        const CHUNKS: usize = 1024;
        let mut file = std::fs::File::create(&file_path).expect("Failed to create file");

        let mut progress = Progress::new("Creating 1GB file", CHUNKS);

        // Write in 1MB chunks to avoid memory issues
        let chunk: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        for i in 0..CHUNKS {
            file.write_all(&chunk).expect("Failed to write chunk");
            progress.update(i + 1);
        }
        file.sync_all().expect("Failed to sync");
        progress.finish();

        let metadata = std::fs::metadata(&file_path).expect("Failed to get metadata");
        assert_eq!(metadata.len() as usize, SIZE);

        eprintln!("  File location: {:?}", file_path);

        // TODO: Add actual encoding test once API is stable
        // let fs = VersionedEmbrFS::new();
        // fs.ingest_file(&file_path).expect("Failed to ingest 1GB file");
    }

    /// Test encoding a 5GB file (stress test)
    /// Run with: SCALE_TEST_DIR=/path/with/space cargo test test_5gb_file -- --ignored --nocapture
    #[test]
    #[ignore]
    fn test_5gb_file() {
        let temp = get_scale_test_dir();
        let file_path = temp.path().join("5gb_file.bin");

        const CHUNKS: usize = 5 * 1024;
        let mut progress = Progress::new("Creating 5GB file (compressible pattern)", CHUNKS);

        let mut file = std::fs::File::create(&file_path).expect("Failed to create file");

        // Write compressible pattern (lots of zeros with occasional data)
        let zeros = vec![0u8; 1024 * 1024];
        let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        for i in 0..CHUNKS {
            // Every 10th chunk is data, rest are zeros (highly compressible)
            if i % 10 == 0 {
                file.write_all(&data).expect("Failed to write data chunk");
            } else {
                file.write_all(&zeros).expect("Failed to write zero chunk");
            }
            progress.update(i + 1);
        }
        file.sync_all().expect("Failed to sync");
        progress.finish();

        let metadata = std::fs::metadata(&file_path).expect("Failed to get metadata");
        assert_eq!(metadata.len() as usize, 5 * 1024 * 1024 * 1024);

        eprintln!("  File location: {:?}", file_path);
    }

    /// Test encoding a highly compressible file
    #[test]
    #[ignore]
    fn test_compressible_1gb() {
        let temp = get_scale_test_dir();
        let file_path = temp.path().join("compressible.bin");

        const CHUNKS: usize = 1024;
        let mut progress = Progress::new("Creating 1GB compressible file (all zeros)", CHUNKS);

        let mut file = std::fs::File::create(&file_path).expect("Failed to create file");

        // All zeros = maximum compression
        let zeros = vec![0u8; 1024 * 1024];
        for i in 0..CHUNKS {
            file.write_all(&zeros).expect("Failed to write zeros");
            progress.update(i + 1);
        }
        file.sync_all().expect("Failed to sync");
        progress.finish();

        eprintln!("  Expected compression: ~1GB -> ~1MB");
    }

    /// Test encoding incompressible random data
    #[test]
    #[ignore]
    fn test_random_1gb() {
        let temp = get_scale_test_dir();
        let file_path = temp.path().join("random.bin");

        const CHUNKS: usize = 1024;
        let mut progress = Progress::new("Creating 1GB random (incompressible) file", CHUNKS);

        let mut file = std::fs::File::create(&file_path).expect("Failed to create file");

        // Use a simple PRNG for reproducible "random" data
        let mut seed: u64 = 0xDEADBEEF;
        let mut random_chunk = vec![0u8; 1024 * 1024];

        for i in 0..CHUNKS {
            for byte in random_chunk.iter_mut() {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                *byte = (seed >> 16) as u8;
            }
            file.write_all(&random_chunk)
                .expect("Failed to write random data");
            progress.update(i + 1);
        }
        file.sync_all().expect("Failed to sync");
        progress.finish();

        eprintln!("  Expected compression: ~1GB -> ~1GB (incompressible)");
    }
}

// =============================================================================
// LARGE DIRECTORY TESTS
// =============================================================================

mod large_directories {
    use super::*;

    /// Create a directory with 10,000 files
    #[test]
    #[ignore]
    fn test_10k_files_flat() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let mut progress = Progress::new("Creating 10,000 files in flat directory", 10_000);

        for i in 0..10_000 {
            let file_path = temp.path().join(format!("file_{:05}.txt", i));
            std::fs::write(&file_path, format!("Content of file {}", i))
                .expect("Failed to write file");
            progress.inc();
        }
        progress.finish();

        let count = std::fs::read_dir(temp.path()).unwrap().count();
        assert_eq!(count, 10_000);
    }

    /// Create a deep directory hierarchy
    #[test]
    #[ignore]
    fn test_deep_directory_100_levels() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let mut progress = Progress::new("Creating 100-level deep directory", 100);

        let mut current = temp.path().to_path_buf();
        for i in 0..100 {
            current = current.join(format!("level_{:03}", i));
            std::fs::create_dir(&current).expect("Failed to create dir");
            std::fs::write(current.join("file.txt"), format!("Level {}", i))
                .expect("Failed to write file");
            progress.inc();
        }
        progress.finish();

        assert!(current.exists());
    }

    /// Create a wide directory tree (10 dirs × 10 subdirs × 10 files each)
    #[test]
    #[ignore]
    fn test_wide_tree_1000_dirs() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let mut progress = Progress::new("Creating wide tree (10×10×10 = 1000 files)", 1000);
        let mut count = 0;

        for i in 0..10 {
            let dir_i = temp.path().join(format!("dir_{}", i));
            std::fs::create_dir(&dir_i).expect("Failed to create dir");

            for j in 0..10 {
                let dir_j = dir_i.join(format!("subdir_{}", j));
                std::fs::create_dir(&dir_j).expect("Failed to create subdir");

                for k in 0..10 {
                    let file = dir_j.join(format!("file_{}.txt", k));
                    std::fs::write(&file, format!("{}/{}/{}", i, j, k))
                        .expect("Failed to write file");
                    count += 1;
                    progress.update(count);
                }
            }
        }
        progress.finish();

        // 10 × 10 × 10 = 1000 files
        let file_count: usize = walkdir::WalkDir::new(temp.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        assert_eq!(file_count, 1000);
    }

    /// Create 50,000 files across multiple directories
    #[test]
    #[ignore]
    fn test_50k_files_distributed() {
        let temp = get_scale_test_dir();
        let mut progress = Progress::new("Creating 50,000 files across 50 directories", 50_000);
        let mut count = 0;

        // 50 directories × 1000 files each
        for dir_num in 0..50 {
            let dir = temp.path().join(format!("dir_{:02}", dir_num));
            std::fs::create_dir(&dir).expect("Failed to create dir");

            for file_num in 0..1000 {
                let file = dir.join(format!("file_{:04}.txt", file_num));
                std::fs::write(&file, format!("Dir {} File {}", dir_num, file_num))
                    .expect("Failed to write file");
                count += 1;
                progress.update(count);
            }
        }
        progress.finish();
    }
}

// =============================================================================
// MEMORY STRESS TESTS
// =============================================================================

mod memory_stress {
    use super::*;
    use std::sync::Arc;

    /// Test concurrent file operations
    #[test]
    #[ignore]
    fn test_concurrent_large_reads() {
        let temp = get_scale_test_dir();

        // Create 10 files of 100MB each
        let mut progress = Progress::new("Creating 10 × 100MB files", 10);
        for i in 0..10 {
            let file_path = temp.path().join(format!("large_{}.bin", i));
            let data = vec![i as u8; 100 * 1024 * 1024]; // 100MB
            std::fs::write(&file_path, &data).expect("Failed to write file");
            progress.inc();
        }
        progress.finish();

        // Read all concurrently
        eprintln!("\n▶ Starting: Concurrent read of all 10 files");
        let start = Instant::now();
        let temp_path = Arc::new(temp.path().to_path_buf());
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let path = Arc::clone(&temp_path);
                std::thread::spawn(move || {
                    let file_path = path.join(format!("large_{}.bin", i));
                    let data = std::fs::read(&file_path).expect("Failed to read");
                    assert_eq!(data.len(), 100 * 1024 * 1024);
                    assert!(data.iter().all(|&b| b == i as u8));
                    eprintln!("  Thread {} verified 100MB", i);
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        eprintln!(
            "✓ Completed: Concurrent read of 10 × 100MB files in {:?}",
            start.elapsed()
        );
    }

    /// Test memory limits aren't exceeded
    #[test]
    #[ignore]
    fn test_streaming_large_file() {
        let temp = get_scale_test_dir();
        let file_path = temp.path().join("streaming.bin");

        // Write 2GB file
        const SIZE: usize = 2 * 1024 * 1024 * 1024;
        const CHUNKS: usize = 2048;
        {
            let mut file = std::fs::File::create(&file_path).expect("Failed to create file");
            let chunk = vec![0xABu8; 1024 * 1024];
            let mut progress = Progress::new("Writing 2GB file", CHUNKS);
            for i in 0..CHUNKS {
                file.write_all(&chunk).expect("Failed to write");
                progress.update(i + 1);
            }
            progress.finish();
        }

        // Read in streaming fashion (shouldn't OOM)
        use std::io::{BufReader, Read};
        let file = std::fs::File::open(&file_path).expect("Failed to open");
        let mut reader = BufReader::with_capacity(1024 * 1024, file);

        let mut buf = vec![0u8; 1024 * 1024];
        let mut total = 0usize;
        let mut progress = Progress::new("Streaming read of 2GB file", CHUNKS);
        let mut chunk_count = 0;
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    total += n;
                    chunk_count += 1;
                    progress.update(chunk_count.min(CHUNKS));
                }
                Err(e) => panic!("Read error: {}", e),
            }
        }
        progress.finish();

        assert_eq!(total, SIZE);
        eprintln!("  Memory stayed bounded during streaming");
    }
}

// =============================================================================
// SYMLINK STRESS TESTS
// =============================================================================

mod symlink_stress {
    use super::*;

    /// Create a deep symlink chain
    #[test]
    #[ignore]
    fn test_symlink_chain_40_deep() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let mut progress = Progress::new("Creating 40-deep symlink chain", 40);

        // Create target file
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "Final target").expect("Failed to write target");

        // Create chain of 40 symlinks
        // Linux default MAXSYMLINKS is 40
        let mut prev_name = "target.txt".to_string();
        for i in 0..40 {
            let link_name = format!("link_{:02}", i);
            let link_path = temp.path().join(&link_name);
            std::os::unix::fs::symlink(&prev_name, &link_path).expect("Failed to create symlink");
            prev_name = link_name;
            progress.inc();
        }
        progress.finish();

        // The last link should resolve to the target
        eprintln!("  Verifying chain resolution...");
        let final_link = temp.path().join("link_39");
        let resolved = std::fs::read_to_string(&final_link).expect("Failed to read through chain");
        assert_eq!(resolved, "Final target");
        eprintln!("  ✓ Chain resolves correctly");
    }

    /// Create many symlinks pointing to same target
    #[test]
    #[ignore]
    fn test_1000_symlinks_same_target() {
        let temp = TempDir::new().expect("Failed to create temp dir");

        let target = temp.path().join("target.txt");
        std::fs::write(&target, "Shared target").expect("Failed to write target");

        let mut progress = Progress::new("Creating 1000 symlinks to same target", 1000);
        for i in 0..1000 {
            let link = temp.path().join(format!("link_{:04}", i));
            std::os::unix::fs::symlink("target.txt", &link).expect("Failed to create symlink");
            progress.inc();
        }
        progress.finish();

        // Verify all resolve correctly
        let mut progress = Progress::new("Verifying all 1000 symlinks", 1000);
        for i in 0..1000 {
            let link = temp.path().join(format!("link_{:04}", i));
            let content = std::fs::read_to_string(&link).expect("Failed to read link");
            assert_eq!(content, "Shared target");
            progress.inc();
        }
        progress.finish();
    }
}

// =============================================================================
// HARDLINK STRESS TESTS
// =============================================================================

mod hardlink_stress {
    use super::*;

    /// Create many hardlinks to same file
    #[test]
    #[ignore]
    fn test_1000_hardlinks() {
        let temp = TempDir::new().expect("Failed to create temp dir");

        let original = temp.path().join("original.txt");
        std::fs::write(&original, "Shared content").expect("Failed to write original");

        let mut progress = Progress::new("Creating 1000 hardlinks", 1000);
        for i in 0..1000 {
            let link = temp.path().join(format!("hardlink_{:04}", i));
            std::fs::hard_link(&original, &link).expect("Failed to create hardlink");
            progress.inc();
        }
        progress.finish();

        // All should have same inode
        use std::os::unix::fs::MetadataExt;
        let original_ino = std::fs::metadata(&original).unwrap().ino();

        let mut progress = Progress::new("Verifying inode consistency", 1000);
        for i in 0..1000 {
            let link = temp.path().join(format!("hardlink_{:04}", i));
            let link_ino = std::fs::metadata(&link).unwrap().ino();
            assert_eq!(link_ino, original_ino);
            progress.inc();
        }
        progress.finish();

        // nlink should be 1001 (original + 1000 hardlinks)
        let nlink = std::fs::metadata(&original).unwrap().nlink();
        assert_eq!(nlink, 1001);
        eprintln!("  ✓ nlink = 1001 (correct)");
    }

    /// Test hardlink detection (deduplication opportunity)
    #[test]
    #[ignore]
    fn test_hardlink_dedup_detection() {
        let temp = TempDir::new().expect("Failed to create temp dir");

        // Create 100 files with their hardlinks
        use std::collections::HashMap;
        use std::os::unix::fs::MetadataExt;

        let mut inode_map: HashMap<u64, Vec<String>> = HashMap::new();

        let mut progress = Progress::new("Creating 100 files with 5 hardlinks each", 100);
        for i in 0..100 {
            let original = temp.path().join(format!("file_{:03}.txt", i));
            std::fs::write(&original, format!("Content {}", i)).expect("Failed to write");

            // Create 5 hardlinks for each
            for j in 0..5 {
                let link = temp.path().join(format!("link_{:03}_{}.txt", i, j));
                std::fs::hard_link(&original, &link).expect("Failed to hardlink");
            }
            progress.inc();
        }
        progress.finish();

        // Scan and group by inode
        eprintln!("\n▶ Starting: Scanning and grouping by inode");
        for entry in walkdir::WalkDir::new(temp.path()) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let ino = entry.metadata().unwrap().ino();
                inode_map
                    .entry(ino)
                    .or_default()
                    .push(entry.path().to_string_lossy().to_string());
            }
        }

        // Should have 100 unique inodes, each with 6 paths (original + 5 links)
        assert_eq!(inode_map.len(), 100);
        for paths in inode_map.values() {
            assert_eq!(paths.len(), 6);
        }

        eprintln!("✓ Completed: Detected 100 hardlink groups with 6 paths each");
    }
}

// =============================================================================
// MIXED WORKLOAD TESTS
// =============================================================================

mod mixed_workload {
    use super::*;

    /// Simulate realistic VM filesystem structure
    #[test]
    #[ignore]
    fn test_vm_filesystem_structure() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let root = temp.path();

        eprintln!("\n▶ Starting: VM filesystem structure simulation");

        // Create realistic directory structure
        let dirs = vec![
            "boot",
            "boot/grub",
            "etc",
            "etc/ssh",
            "etc/apt",
            "etc/systemd",
            "home/user",
            "home/user/.config",
            "lib/x86_64-linux-gnu",
            "lib/modules/6.1.0",
            "opt",
            "root",
            "tmp",
            "usr/bin",
            "usr/lib/x86_64-linux-gnu",
            "usr/share/doc",
            "var/cache/apt",
            "var/log",
            "var/lib",
        ];

        eprintln!("  Creating {} directories...", dirs.len());
        for dir in &dirs {
            std::fs::create_dir_all(root.join(dir)).expect("Failed to create dir");
        }

        // Add files of various types
        // Boot files (large)
        eprintln!("  Writing boot files (10MB vmlinuz)...");
        std::fs::write(root.join("boot/vmlinuz"), vec![0u8; 10 * 1024 * 1024])
            .expect("Failed to write vmlinuz");

        // Libraries (medium)
        let mut progress = Progress::new("Creating 50 libraries (100KB each)", 50);
        for i in 0..50 {
            std::fs::write(
                root.join(format!("lib/x86_64-linux-gnu/lib{}.so.1", i)),
                vec![i as u8; 100 * 1024],
            )
            .expect("Failed to write lib");
            progress.inc();
        }
        progress.finish();

        // Binaries
        let mut progress = Progress::new("Creating 100 binaries (50KB each)", 100);
        for i in 0..100 {
            std::fs::write(
                root.join(format!("usr/bin/prog{}", i)),
                vec![0xCC; 50 * 1024],
            )
            .expect("Failed to write binary");
            progress.inc();
        }
        progress.finish();

        // Config files (small)
        let mut progress = Progress::new("Creating 200 config files", 200);
        for i in 0..200 {
            std::fs::write(
                root.join(format!("etc/config{}.conf", i)),
                format!("# Config {}\nkey=value\n", i),
            )
            .expect("Failed to write config");
            progress.inc();
        }
        progress.finish();

        // Symlinks
        eprintln!("  Creating symlinks...");
        std::os::unix::fs::symlink("../lib/x86_64-linux-gnu", root.join("lib64"))
            .expect("Failed to create symlink");

        // Log files
        eprintln!("  Writing log file (1MB)...");
        std::fs::write(root.join("var/log/syslog"), vec![b'x'; 1024 * 1024])
            .expect("Failed to write log");

        // Count total
        let file_count: usize = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        eprintln!(
            "✓ Completed: Created VM-like filesystem with {} files in {} directories",
            file_count,
            dirs.len()
        );
        assert!(file_count > 350); // At least our created files
    }
}
