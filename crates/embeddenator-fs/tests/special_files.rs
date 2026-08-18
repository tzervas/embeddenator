//! Special file handling tests
//!
//! Tests for symlinks, hardlinks, device nodes, and other special files.
//! These tests verify that embeddenator-fs correctly handles Unix special files
//! during encoding and FUSE operations.

use embeddenator_fs::fs::versioned::manifest::{FilePermissions, FileType, VersionedFileEntry};
use embeddenator_fs::fuse_shim::{EngramFS, FileKind};

// =============================================================================
// SYMLINK TESTS
// =============================================================================

mod symlink_tests {
    use super::*;

    #[test]
    fn test_symlink_entry_creation() {
        let entry = VersionedFileEntry::new_symlink(
            "/lib/libc.so.6".to_string(),
            "libc-2.33.so".to_string(),
            FilePermissions::new(0, 0, 0o777),
        );

        assert_eq!(entry.file_type, FileType::Symlink);
        assert_eq!(entry.symlink_target, Some("libc-2.33.so".to_string()));
        assert_eq!(entry.permissions.mode, 0o777);
        assert!(entry.chunks.is_empty()); // Symlinks don't have content chunks
        assert_eq!(entry.size, 0);
    }

    #[test]
    fn test_symlink_target_preservation() {
        // Test various symlink target types
        let targets = vec![
            "relative/path/to/file",
            "/absolute/path/to/file",
            "../parent/relative",
            "../../grandparent/path",
            "./same-dir-file",
            "simple-name",
        ];

        for target in targets {
            let entry = VersionedFileEntry::new_symlink(
                "/test/link".to_string(),
                target.to_string(),
                FilePermissions::default_file(),
            );
            assert_eq!(
                entry.symlink_target,
                Some(target.to_string()),
                "Symlink target '{}' not preserved",
                target
            );
        }
    }

    #[test]
    fn test_dangling_symlink() {
        // Symlinks to non-existent targets are valid
        let entry = VersionedFileEntry::new_symlink(
            "/test/dangling".to_string(),
            "/nonexistent/target".to_string(),
            FilePermissions::default_file(),
        );

        assert_eq!(entry.file_type, FileType::Symlink);
        assert_eq!(
            entry.symlink_target,
            Some("/nonexistent/target".to_string())
        );
    }

    #[test]
    fn test_symlink_chain() {
        // Create a chain: link1 -> link2 -> link3 -> actual_file
        let links = [
            ("/chain/link1", "link2"),
            ("/chain/link2", "link3"),
            ("/chain/link3", "actual_file"),
        ];

        let entries: Vec<_> = links
            .iter()
            .map(|(path, target)| {
                VersionedFileEntry::new_symlink(
                    path.to_string(),
                    target.to_string(),
                    FilePermissions::default_file(),
                )
            })
            .collect();

        // Verify all are symlinks with correct targets
        for (i, ((_path, target), entry)) in links.iter().zip(entries.iter()).enumerate() {
            assert_eq!(
                entry.file_type,
                FileType::Symlink,
                "Entry {} not a symlink",
                i
            );
            assert_eq!(
                entry.symlink_target.as_deref(),
                Some(*target),
                "Entry {} has wrong target",
                i
            );
        }
    }

    #[test]
    fn test_fuse_symlink_operations() {
        let fs = EngramFS::new(false); // Writable

        // Add a symlink
        let ino = fs
            .add_symlink("/lib/libtest.so.1", "libtest.so.1.2.3".to_string())
            .expect("Failed to add symlink");

        // Verify it was created
        let attr = fs.get_attr(ino).expect("Symlink not found");
        assert_eq!(attr.kind, FileKind::Symlink);

        // Verify target is readable
        let target = fs.read_symlink(ino).expect("Failed to read symlink target");
        assert_eq!(target, "libtest.so.1.2.3");
    }

    #[test]
    fn test_fuse_symlink_count() {
        let fs = EngramFS::new(false);

        assert_eq!(fs.symlink_count(), 0);

        fs.add_symlink("/link1", "target1".to_string()).unwrap();
        assert_eq!(fs.symlink_count(), 1);

        fs.add_symlink("/link2", "target2".to_string()).unwrap();
        assert_eq!(fs.symlink_count(), 2);

        fs.add_symlink("/link3", "target3".to_string()).unwrap();
        assert_eq!(fs.symlink_count(), 3);
    }
}

// =============================================================================
// HARDLINK TESTS
// =============================================================================

mod hardlink_tests {
    use super::*;

    #[test]
    fn test_hardlink_entry_creation() {
        let entry = VersionedFileEntry::new_hardlink(
            "/data/hardlink".to_string(),
            "/data/original".to_string(),
            FilePermissions::new(1000, 1000, 0o644),
        );

        assert_eq!(entry.file_type, FileType::Hardlink);
        assert_eq!(entry.hardlink_target, Some("/data/original".to_string()));
        assert!(entry.symlink_target.is_none()); // Not a symlink
    }

    #[test]
    fn test_hardlink_target_resolution() {
        // Multiple hardlinks to the same file
        let original = "/data/original.txt";
        let hardlinks = [
            "/data/link1.txt",
            "/data/link2.txt",
            "/data/subdir/link3.txt",
        ];

        let entries: Vec<_> = hardlinks
            .iter()
            .map(|path| {
                VersionedFileEntry::new_hardlink(
                    path.to_string(),
                    original.to_string(),
                    FilePermissions::default_file(),
                )
            })
            .collect();

        // All should point to the same original
        for entry in entries {
            assert_eq!(entry.hardlink_target, Some(original.to_string()));
        }
    }

    #[test]
    fn test_hardlink_vs_symlink() {
        let hardlink = VersionedFileEntry::new_hardlink(
            "/test/hardlink".to_string(),
            "/test/target".to_string(),
            FilePermissions::default_file(),
        );

        let symlink = VersionedFileEntry::new_symlink(
            "/test/symlink".to_string(),
            "/test/target".to_string(),
            FilePermissions::default_file(),
        );

        // Verify types are distinct
        assert_eq!(hardlink.file_type, FileType::Hardlink);
        assert_eq!(symlink.file_type, FileType::Symlink);

        // Verify targets are in correct fields
        assert!(hardlink.hardlink_target.is_some());
        assert!(hardlink.symlink_target.is_none());

        assert!(symlink.symlink_target.is_some());
        assert!(symlink.hardlink_target.is_none());
    }
}

// =============================================================================
// DEVICE NODE TESTS
// =============================================================================

mod device_tests {
    use super::*;

    #[test]
    fn test_char_device_metadata() {
        // /dev/null is major 1, minor 3
        let entry = VersionedFileEntry::new_device(
            "/dev/null".to_string(),
            true, // char device
            1,
            3,
            FilePermissions::new(0, 0, 0o666),
        );

        assert_eq!(entry.file_type, FileType::CharDevice);
        assert_eq!(entry.device_id, Some((1, 3)));
        assert!(entry.chunks.is_empty()); // Metadata only
        assert_eq!(entry.size, 0);
    }

    #[test]
    fn test_block_device_metadata() {
        // /dev/sda is typically major 8, minor 0
        let entry = VersionedFileEntry::new_device(
            "/dev/sda".to_string(),
            false, // block device
            8,
            0,
            FilePermissions::new(0, 6, 0o660), // root:disk
        );

        assert_eq!(entry.file_type, FileType::BlockDevice);
        assert_eq!(entry.device_id, Some((8, 0)));
    }

    #[test]
    fn test_device_with_data_option_c() {
        // Option C: Block device with actual data (e.g., loop device)
        let chunks = vec![1, 2, 3, 4, 5]; // Example chunk IDs
        let entry = VersionedFileEntry::new_device_with_data(
            "/dev/loop0".to_string(),
            false, // block device
            7,     // loop device major
            0,     // minor 0
            FilePermissions::new(0, 6, 0o660),
            1024 * 1024, // 1MB of data
            chunks.clone(),
        );

        assert_eq!(entry.file_type, FileType::BlockDevice);
        assert_eq!(entry.device_id, Some((7, 0)));
        assert_eq!(entry.size, 1024 * 1024);
        assert_eq!(entry.chunks, chunks);
    }

    #[test]
    fn test_device_compressed_option_c() {
        // Option C with compression
        let chunks = vec![10, 20, 30];
        let entry = VersionedFileEntry::new_device_compressed(
            "/dev/loop1".to_string(),
            false,
            7,
            1,
            FilePermissions::new(0, 6, 0o660),
            512 * 1024,  // compressed size
            1024 * 1024, // uncompressed size
            1,           // zstd
            chunks.clone(),
        );

        assert_eq!(entry.file_type, FileType::BlockDevice);
        assert_eq!(entry.size, 512 * 1024);
        assert_eq!(entry.uncompressed_size, Some(1024 * 1024));
        assert_eq!(entry.compression_codec, Some(1));
    }

    #[test]
    fn test_fuse_device_node() {
        let fs = EngramFS::new(false);

        // Add a character device
        let ino = fs
            .add_device("/dev/test_null", true, 1, 3, Vec::new())
            .expect("Failed to add device");

        let attr = fs.get_attr(ino).expect("Device not found");
        assert_eq!(attr.kind, FileKind::CharDevice);
        assert_eq!(attr.rdev, (1 << 8) | 3); // Encoded major/minor
    }

    #[test]
    fn test_device_count() {
        let fs = EngramFS::new(false);

        assert_eq!(fs.device_count(), 0);

        fs.add_device("/dev/null", true, 1, 3, Vec::new()).unwrap();
        fs.add_device("/dev/zero", true, 1, 5, Vec::new()).unwrap();
        fs.add_device("/dev/sda", false, 8, 0, Vec::new()).unwrap();

        assert_eq!(fs.device_count(), 3);
    }
}

// =============================================================================
// FIFO AND SOCKET TESTS
// =============================================================================

mod special_file_tests {
    use super::*;

    #[test]
    fn test_fifo_creation() {
        let entry = VersionedFileEntry::new_special(
            "/tmp/test.pipe".to_string(),
            FileType::Fifo,
            FilePermissions::new(1000, 1000, 0o644),
        );

        assert_eq!(entry.file_type, FileType::Fifo);
        assert!(entry.chunks.is_empty());
        assert_eq!(entry.size, 0);
    }

    #[test]
    fn test_socket_creation() {
        let entry = VersionedFileEntry::new_special(
            "/var/run/test.sock".to_string(),
            FileType::Socket,
            FilePermissions::new(0, 0, 0o755),
        );

        assert_eq!(entry.file_type, FileType::Socket);
        assert!(entry.chunks.is_empty());
    }

    #[test]
    fn test_fuse_fifo() {
        let fs = EngramFS::new(false);

        let ino = fs.add_fifo("/tmp/test.fifo").expect("Failed to add FIFO");
        let attr = fs.get_attr(ino).expect("FIFO not found");
        assert_eq!(attr.kind, FileKind::Fifo);
    }

    #[test]
    fn test_fuse_socket() {
        let fs = EngramFS::new(false);

        let ino = fs
            .add_socket("/run/test.socket")
            .expect("Failed to add socket");
        let attr = fs.get_attr(ino).expect("Socket not found");
        assert_eq!(attr.kind, FileKind::Socket);
    }

    #[test]
    fn test_file_type_predicates() {
        assert!(FileType::Regular.has_content());
        assert!(!FileType::Symlink.has_content());
        assert!(!FileType::CharDevice.has_content());

        assert!(FileType::Regular.can_have_content());
        assert!(FileType::BlockDevice.can_have_content());
        assert!(!FileType::Fifo.can_have_content());

        assert!(FileType::Fifo.is_metadata_only());
        assert!(FileType::Socket.is_metadata_only());
        assert!(!FileType::Regular.is_metadata_only());
        assert!(!FileType::BlockDevice.is_metadata_only()); // Option C

        assert!(FileType::Symlink.is_link());
        assert!(FileType::Hardlink.is_link());
        assert!(!FileType::Regular.is_link());

        assert!(FileType::CharDevice.is_device());
        assert!(FileType::BlockDevice.is_device());
        assert!(!FileType::Regular.is_device());
    }
}

// =============================================================================
// SPARSE FILE AND ZERO-BYTE FILE TESTS
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_byte_file() {
        let entry = VersionedFileEntry::new("/empty.txt".to_string(), true, 0, Vec::new());

        assert_eq!(entry.size, 0);
        assert!(entry.chunks.is_empty());
        assert_eq!(entry.file_type, FileType::Regular);
    }

    #[test]
    fn test_fuse_zero_byte_file() {
        let fs = EngramFS::new(false);

        let ino = fs
            .add_file("/empty.txt", Vec::new())
            .expect("Failed to add empty file");

        let attr = fs.get_attr(ino).expect("Empty file not found");
        assert_eq!(attr.size, 0);

        // Reading should return empty
        let data = fs.read_data(ino, 0, 1024);
        assert_eq!(data, Some(Vec::new()));
    }

    #[test]
    fn test_deeply_nested_symlink() {
        let fs = EngramFS::new(false);

        // Create deeply nested directory structure
        for i in 0..10 {
            let path = format!("/deep{}/link", "/nested".repeat(i));
            let target = format!("../{}target", "nested/".repeat(i));
            fs.add_symlink(&path, target).ok();
        }

        // Just verify no panics and count is correct
        assert!(fs.symlink_count() > 0);
    }

    #[test]
    fn test_special_characters_in_symlink_target() {
        let targets = vec![
            "file with spaces.txt",
            "file\twith\ttabs",
            "file'with'quotes",
            "file\"with\"doublequotes",
            "file$with$dollars",
            "file*with*asterisks", // But not actual glob expansion
            "файл_юникод",         // Unicode
            "文件名",              // CJK
        ];

        for target in targets {
            let entry = VersionedFileEntry::new_symlink(
                "/test/link".to_string(),
                target.to_string(),
                FilePermissions::default_file(),
            );

            assert_eq!(
                entry.symlink_target,
                Some(target.to_string()),
                "Target '{}' not preserved",
                target
            );
        }
    }

    #[test]
    fn test_mixed_special_files() {
        let fs = EngramFS::new(false);

        // Add various special files
        fs.add_file("/regular.txt", b"hello".to_vec()).unwrap();
        fs.add_symlink("/link", "regular.txt".to_string()).unwrap();
        fs.add_device("/dev/test", true, 1, 1, Vec::new()).unwrap();
        fs.add_fifo("/pipe").unwrap();
        fs.add_socket("/sock").unwrap();

        // Verify counts
        // file_count counts items in file_cache (regular files + device data for Option C)
        assert_eq!(
            fs.file_count(),
            2,
            "Should have 2 cached files (regular + device)"
        );
        assert_eq!(fs.symlink_count(), 1, "Should have 1 symlink");
        assert_eq!(fs.device_count(), 1, "Should have 1 device");
        // FIFOs and sockets don't have separate counts yet
    }
}

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_filesystem_with_symlink_chain() {
        let fs = EngramFS::new(false);

        // Create: /bin/sh -> /bin/bash -> /usr/bin/bash
        fs.add_file("/usr/bin/bash", b"#!/bin/bash\necho hello".to_vec())
            .unwrap();
        fs.add_symlink("/bin/bash", "/usr/bin/bash".to_string())
            .unwrap();
        fs.add_symlink("/bin/sh", "bash".to_string()).unwrap();

        // Verify structure
        assert_eq!(fs.file_count(), 1);
        assert_eq!(fs.symlink_count(), 2);
    }

    #[test]
    fn test_dev_directory_structure() {
        let fs = EngramFS::new(false);

        // Simulate minimal /dev structure
        let devices = vec![
            ("/dev/null", true, 1, 3),
            ("/dev/zero", true, 1, 5),
            ("/dev/random", true, 1, 8),
            ("/dev/urandom", true, 1, 9),
            ("/dev/tty", true, 5, 0),
            ("/dev/console", true, 5, 1),
        ];

        for (path, is_char, major, minor) in devices {
            fs.add_device(path, is_char, major, minor, Vec::new())
                .unwrap_or_else(|_| panic!("Failed to add {}", path));
        }

        assert_eq!(fs.device_count(), 6);

        // Verify specific device
        let null_ino = fs.lookup_path("/dev/null").expect("/dev/null not found");
        let attr = fs.get_attr(null_ino).expect("attr not found");
        assert_eq!(attr.kind, FileKind::CharDevice);
    }
}
