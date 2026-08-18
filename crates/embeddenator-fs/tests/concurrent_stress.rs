//! Concurrent stress tests for VersionedEmbrFS
//!
//! Tests concurrent read/write operations to verify optimistic locking,
//! version tracking, and data integrity under high contention.

use embeddenator_fs::{EmbrFSError, VersionedEmbrFS};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_concurrent_writes_different_files() {
    let fs = Arc::new(VersionedEmbrFS::new());
    let mut handles = vec![];

    // Spawn 10 threads, each writing to a different file
    for i in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            let path = format!("file{}.txt", i);
            let data = format!("Thread {} data", i).into_bytes();

            // Write file
            let result = fs_clone.write_file(&path, &data, None);
            assert!(result.is_ok(), "Write failed: {:?}", result);

            // Read it back
            let (read_data, _) = fs_clone.read_file(&path).unwrap();
            assert_eq!(read_data, data);
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all files exist
    let files = fs.list_files();
    assert_eq!(files.len(), 10);
}

#[test]
fn test_concurrent_writes_same_file() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create initial file
    fs.write_file("shared.txt", b"initial", None).unwrap();

    let mut handles = vec![];
    let success_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Spawn 20 threads trying to update the same file
    for i in 0..20 {
        let fs_clone = Arc::clone(&fs);
        let success_count_clone = Arc::clone(&success_count);

        let handle = thread::spawn(move || {
            // Read current version
            let (data, version) = fs_clone.read_file("shared.txt").unwrap();

            // Modify data
            let mut new_data = data;
            new_data.extend_from_slice(format!(" update{}", i).as_bytes());

            // Try to write with version check
            // Some will succeed, some will get version mismatch
            match fs_clone.write_file("shared.txt", &new_data, Some(version)) {
                Ok(_) => {
                    success_count_clone.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                }
                Err(EmbrFSError::VersionMismatch { .. }) => {
                    // Expected - someone else updated first
                }
                Err(e) => {
                    panic!("Unexpected error: {:?}", e);
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // At least one write should succeed
    let success_count = success_count.load(std::sync::atomic::Ordering::Acquire);
    assert!(success_count > 0, "No writes succeeded!");

    // File should still be valid
    let (final_data, _) = fs.read_file("shared.txt").unwrap();
    assert!(final_data.starts_with(b"initial"));
}

#[test]
fn test_concurrent_read_write() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create initial file
    fs.write_file("test.txt", b"initial data", None).unwrap();

    let mut handles = vec![];

    // Spawn 10 reader threads
    for _ in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                // Read should always succeed
                let result = fs_clone.read_file("test.txt");
                assert!(result.is_ok());
                thread::sleep(Duration::from_micros(10));
            }
        });
        handles.push(handle);
    }

    // Spawn 2 writer threads
    for i in 0..2 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            for _ in 0..50 {
                let data = format!("updated by thread {}", i).into_bytes();
                // Writer might fail with version mismatch, retry until success
                loop {
                    let (_, version) = fs_clone.read_file("test.txt").unwrap();
                    match fs_clone.write_file("test.txt", &data, Some(version)) {
                        Ok(_) => break,
                        Err(EmbrFSError::VersionMismatch { .. }) => {
                            // Retry
                            continue;
                        }
                        Err(e) => panic!("Unexpected error: {:?}", e),
                    }
                }
                thread::sleep(Duration::from_micros(100));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_create_delete() {
    let fs = Arc::new(VersionedEmbrFS::new());
    let mut handles = vec![];

    // Spawn threads that create and delete files
    for i in 0..5 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            for j in 0..20 {
                let path = format!("temp_{}_{}.txt", i, j);
                let data = format!("data {}", j).into_bytes();

                // Create
                let result = fs_clone.write_file(&path, &data, None);
                assert!(
                    result.is_ok(),
                    "Failed to create file {}: {:?}",
                    path,
                    result
                );

                // Read
                let (read_data, version) = fs_clone
                    .read_file(&path)
                    .unwrap_or_else(|_| panic!("Failed to read {}", path));
                assert_eq!(read_data, data);

                // Delete
                let result = fs_clone.delete_file(&path, version);
                assert!(
                    result.is_ok(),
                    "Failed to delete file {}: {:?}",
                    path,
                    result
                );

                // Verify deleted
                assert!(!fs_clone.exists(&path));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // All temporary files should be gone
    let files = fs.list_files();
    assert_eq!(files.len(), 0);
}

#[test]
fn test_large_file_concurrent() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create large file (multiple chunks)
    let large_data = vec![42u8; 50 * 1024]; // 50KB
    fs.write_file("large.bin", &large_data, None).unwrap();

    let mut handles = vec![];

    // Spawn 10 threads reading the same large file
    for _ in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let expected_data = large_data.clone();
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let (data, _) = fs_clone.read_file("large.bin").unwrap();
                assert_eq!(data, expected_data);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_stress_many_files() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create 100 files sequentially first (to avoid version conflicts on initial creates)
    for i in 0..100 {
        let path = format!("stress_{}.txt", i);
        let data = format!("Stress test file {}", i).into_bytes();
        fs.write_file(&path, &data, None).unwrap();
    }

    // Verify all files exist
    let files = fs.list_files();
    assert_eq!(files.len(), 100);

    // Now update them all concurrently
    let mut handles = vec![];
    for i in 0..100 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            let path = format!("stress_{}.txt", i);
            loop {
                // Read current version
                let (_, version) = fs_clone.read_file(&path).unwrap();

                // Update with new data
                let new_data = format!("Updated stress test file {}", i).into_bytes();
                match fs_clone.write_file(&path, &new_data, Some(version)) {
                    Ok(_) => break,
                    Err(EmbrFSError::VersionMismatch { .. }) => continue,
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify stats
    let stats = fs.stats();
    assert_eq!(stats.active_files, 100);
    assert_eq!(stats.deleted_files, 0);
}

#[test]
fn test_version_monotonicity() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create file
    let v1 = fs.write_file("test.txt", b"v1", None).unwrap();
    assert_eq!(v1, 0);

    // Update with correct version
    let v2 = fs.write_file("test.txt", b"v2", Some(v1)).unwrap();
    assert_eq!(v2, 1);

    // Update with correct version again
    let v3 = fs.write_file("test.txt", b"v3", Some(v2)).unwrap();
    assert_eq!(v3, 2);

    // Try to update with old version (should fail)
    let result = fs.write_file("test.txt", b"v4", Some(v1));
    assert!(matches!(result, Err(EmbrFSError::VersionMismatch { .. })));

    // Current data should still be v3
    let (data, version) = fs.read_file("test.txt").unwrap();
    assert_eq!(data, b"v3");
    assert_eq!(version, 2);
}

#[test]
fn test_retry_on_conflict() {
    let fs = Arc::new(VersionedEmbrFS::new());

    // Create initial file
    fs.write_file("counter.txt", b"0", None).unwrap();

    let mut handles = vec![];

    // Spawn 10 threads that increment a counter with retry logic
    for _ in 0..10 {
        let fs_clone = Arc::clone(&fs);
        let handle = thread::spawn(move || {
            loop {
                // Read current value
                let (data, version) = fs_clone.read_file("counter.txt").unwrap();
                let current: i32 = String::from_utf8(data).unwrap().parse().unwrap();

                // Increment
                let new_value = current + 1;
                let new_data = new_value.to_string().into_bytes();

                // Try to write with version check
                match fs_clone.write_file("counter.txt", &new_data, Some(version)) {
                    Ok(_) => break, // Success!
                    Err(EmbrFSError::VersionMismatch { .. }) => {
                        // Retry
                        continue;
                    }
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Final value should be 10
    let (data, _) = fs.read_file("counter.txt").unwrap();
    let final_value: i32 = String::from_utf8(data).unwrap().parse().unwrap();
    assert_eq!(final_value, 10);
}
