//! Unix permissions handling tests
//!
//! Tests for uid/gid preservation, mode bits, and special permission bits
//! (setuid, setgid, sticky).

use embeddenator_fs::fs::versioned::manifest::{FilePermissions, FileType, VersionedFileEntry};

// =============================================================================
// BASIC PERMISSION TESTS
// =============================================================================

mod basic_permissions {
    use super::*;

    #[test]
    fn test_default_file_permissions() {
        let perms = FilePermissions::default_file();

        assert_eq!(perms.uid, 0);
        assert_eq!(perms.gid, 0);
        assert_eq!(perms.mode, 0o644);

        // Verify no special bits
        assert!(!perms.is_setuid());
        assert!(!perms.is_setgid());
        assert!(!perms.is_sticky());
    }

    #[test]
    fn test_default_dir_permissions() {
        let perms = FilePermissions::default_dir();

        assert_eq!(perms.uid, 0);
        assert_eq!(perms.gid, 0);
        assert_eq!(perms.mode, 0o755);
    }

    #[test]
    fn test_custom_permissions() {
        let perms = FilePermissions::new(1000, 1000, 0o600);

        assert_eq!(perms.uid, 1000);
        assert_eq!(perms.gid, 1000);
        assert_eq!(perms.mode, 0o600);
    }

    #[test]
    fn test_permission_preservation_in_entry() {
        let perms = FilePermissions::new(500, 100, 0o755);
        let entry = VersionedFileEntry::new_with_metadata(
            "/test.sh".to_string(),
            FileType::Regular,
            perms,
            false,
            0,
            Vec::new(),
        );

        assert_eq!(entry.permissions.uid, 500);
        assert_eq!(entry.permissions.gid, 100);
        assert_eq!(entry.permissions.mode, 0o755);
    }
}

// =============================================================================
// UID/GID TESTS
// =============================================================================

mod ownership_tests {
    use super::*;

    #[test]
    fn test_root_ownership() {
        let perms = FilePermissions::new(0, 0, 0o644);

        assert_eq!(perms.uid, 0);
        assert_eq!(perms.gid, 0);
    }

    #[test]
    fn test_regular_user_ownership() {
        let perms = FilePermissions::new(1000, 1000, 0o644);

        assert_eq!(perms.uid, 1000);
        assert_eq!(perms.gid, 1000);
    }

    #[test]
    fn test_system_user_ownership() {
        // www-data is typically uid/gid 33 on Debian
        let perms = FilePermissions::new(33, 33, 0o644);

        assert_eq!(perms.uid, 33);
        assert_eq!(perms.gid, 33);
    }

    #[test]
    fn test_mixed_ownership() {
        // File owned by user but in a different group
        let perms = FilePermissions::new(1000, 100, 0o640);

        assert_eq!(perms.uid, 1000);
        assert_eq!(perms.gid, 100);
    }

    #[test]
    fn test_high_uid_gid() {
        // Test with high UID/GID values (LDAP, NIS users)
        let perms = FilePermissions::new(65534, 65534, 0o644); // nobody:nogroup

        assert_eq!(perms.uid, 65534);
        assert_eq!(perms.gid, 65534);
    }

    #[test]
    fn test_max_uid_gid() {
        // Maximum 32-bit values
        let perms = FilePermissions::new(u32::MAX, u32::MAX, 0o644);

        assert_eq!(perms.uid, u32::MAX);
        assert_eq!(perms.gid, u32::MAX);
    }
}

// =============================================================================
// MODE BITS TESTS
// =============================================================================

mod mode_bits {
    use super::*;

    #[test]
    fn test_mode_rwx_user() {
        let perms = FilePermissions::new(0, 0, 0o700);
        assert_eq!(perms.mode & 0o700, 0o700); // User rwx
        assert_eq!(perms.mode & 0o070, 0o000); // Group none
        assert_eq!(perms.mode & 0o007, 0o000); // Other none
    }

    #[test]
    fn test_mode_rwx_group() {
        let perms = FilePermissions::new(0, 0, 0o070);
        assert_eq!(perms.mode & 0o700, 0o000); // User none
        assert_eq!(perms.mode & 0o070, 0o070); // Group rwx
        assert_eq!(perms.mode & 0o007, 0o000); // Other none
    }

    #[test]
    fn test_mode_rwx_other() {
        let perms = FilePermissions::new(0, 0, 0o007);
        assert_eq!(perms.mode & 0o700, 0o000); // User none
        assert_eq!(perms.mode & 0o070, 0o000); // Group none
        assert_eq!(perms.mode & 0o007, 0o007); // Other rwx
    }

    #[test]
    fn test_common_file_modes() {
        let modes = vec![
            (0o644, "rw-r--r--"), // Regular file
            (0o755, "rwxr-xr-x"), // Executable
            (0o600, "rw-------"), // Private file
            (0o400, "r--------"), // Read-only
            (0o666, "rw-rw-rw-"), // World writable
            (0o777, "rwxrwxrwx"), // Full access
        ];

        for (mode, _description) in modes {
            let perms = FilePermissions::new(0, 0, mode);
            assert_eq!(perms.mode, mode, "Mode {:o} not preserved", mode);
        }
    }
}

// =============================================================================
// SETUID/SETGID/STICKY BIT TESTS
// =============================================================================

mod special_bits {
    use super::*;

    #[test]
    fn test_setuid_bit() {
        let perms = FilePermissions::new(0, 0, 0o4755);

        assert!(perms.is_setuid());
        assert!(!perms.is_setgid());
        assert!(!perms.is_sticky());
    }

    #[test]
    fn test_setgid_bit() {
        let perms = FilePermissions::new(0, 0, 0o2755);

        assert!(!perms.is_setuid());
        assert!(perms.is_setgid());
        assert!(!perms.is_sticky());
    }

    #[test]
    fn test_sticky_bit() {
        let perms = FilePermissions::new(0, 0, 0o1755);

        assert!(!perms.is_setuid());
        assert!(!perms.is_setgid());
        assert!(perms.is_sticky());
    }

    #[test]
    fn test_setuid_and_setgid() {
        let perms = FilePermissions::new(0, 0, 0o6755);

        assert!(perms.is_setuid());
        assert!(perms.is_setgid());
        assert!(!perms.is_sticky());
    }

    #[test]
    fn test_all_special_bits() {
        let perms = FilePermissions::new(0, 0, 0o7755);

        assert!(perms.is_setuid());
        assert!(perms.is_setgid());
        assert!(perms.is_sticky());
    }

    #[test]
    fn test_no_special_bits() {
        let perms = FilePermissions::new(0, 0, 0o755);

        assert!(!perms.is_setuid());
        assert!(!perms.is_setgid());
        assert!(!perms.is_sticky());
    }

    #[test]
    fn test_setuid_binary() {
        // Simulate /usr/bin/sudo
        let entry = VersionedFileEntry::new_with_metadata(
            "/usr/bin/sudo".to_string(),
            FileType::Regular,
            FilePermissions::new(0, 0, 0o4755), // setuid root
            false,
            100000,
            vec![1, 2, 3],
        );

        assert!(entry.permissions.is_setuid());
        assert_eq!(entry.permissions.uid, 0); // Owned by root
    }

    #[test]
    fn test_setgid_directory() {
        // setgid directories inherit group
        let entry = VersionedFileEntry::new_with_metadata(
            "/var/shared".to_string(),
            FileType::Directory,
            FilePermissions::new(0, 100, 0o2775), // setgid, group users
            false,
            0,
            Vec::new(),
        );

        assert!(entry.permissions.is_setgid());
        assert_eq!(entry.permissions.gid, 100);
    }

    #[test]
    fn test_sticky_tmp() {
        // /tmp has sticky bit
        let entry = VersionedFileEntry::new_with_metadata(
            "/tmp".to_string(),
            FileType::Directory,
            FilePermissions::new(0, 0, 0o1777), // sticky, world writable
            false,
            0,
            Vec::new(),
        );

        assert!(entry.permissions.is_sticky());
        assert_eq!(entry.permissions.mode, 0o1777);
    }
}

// =============================================================================
// PERMISSION PRESERVATION TESTS
// =============================================================================

mod preservation {
    use super::*;

    #[test]
    fn test_permission_preserved_in_symlink() {
        // Symlinks typically have 0777 but we should preserve what we set
        let entry = VersionedFileEntry::new_symlink(
            "/lib/libc.so.6".to_string(),
            "libc-2.33.so".to_string(),
            FilePermissions::new(0, 0, 0o777),
        );

        assert_eq!(entry.permissions.mode, 0o777);
    }

    #[test]
    fn test_permission_preserved_in_device() {
        let entry = VersionedFileEntry::new_device(
            "/dev/tty".to_string(),
            true,
            5,
            0,
            FilePermissions::new(0, 5, 0o666), // root:tty
        );

        assert_eq!(entry.permissions.uid, 0);
        assert_eq!(entry.permissions.gid, 5); // tty group
        assert_eq!(entry.permissions.mode, 0o666);
    }

    #[test]
    fn test_permission_preserved_in_compressed() {
        let entry = VersionedFileEntry::new_compressed_with_metadata(
            "/data/archive.bin".to_string(),
            FilePermissions::new(1000, 1000, 0o640),
            false,
            1024,
            2048,
            1, // zstd
            vec![1, 2, 3],
        );

        assert_eq!(entry.permissions.uid, 1000);
        assert_eq!(entry.permissions.gid, 1000);
        assert_eq!(entry.permissions.mode, 0o640);
    }

    #[test]
    fn test_permission_preserved_after_update() {
        let original = VersionedFileEntry::new_with_metadata(
            "/test.txt".to_string(),
            FileType::Regular,
            FilePermissions::new(500, 100, 0o4755), // setuid
            true,
            100,
            vec![1],
        );

        let updated = original.update(vec![1, 2], 200);

        // Permissions should be preserved
        assert_eq!(updated.permissions.uid, 500);
        assert_eq!(updated.permissions.gid, 100);
        assert_eq!(updated.permissions.mode, 0o4755);
        assert!(updated.permissions.is_setuid());
    }

    #[test]
    fn test_permission_preserved_after_compressed_update() {
        let original = VersionedFileEntry::new_with_metadata(
            "/test.bin".to_string(),
            FileType::Regular,
            FilePermissions::new(0, 0, 0o2755), // setgid
            false,
            100,
            vec![1],
        );

        let updated = original.update_compressed(vec![1, 2], 150, 200, 1);

        assert_eq!(updated.permissions.mode, 0o2755);
        assert!(updated.permissions.is_setgid());
    }
}

// =============================================================================
// REAL-WORLD PERMISSION PATTERNS
// =============================================================================

mod real_world {
    use super::*;

    #[test]
    fn test_etc_passwd_permissions() {
        let entry = VersionedFileEntry::new_with_metadata(
            "/etc/passwd".to_string(),
            FileType::Regular,
            FilePermissions::new(0, 0, 0o644),
            true,
            2048,
            vec![1],
        );

        assert_eq!(entry.permissions.mode, 0o644);
        assert!(!entry.permissions.is_setuid());
    }

    #[test]
    fn test_etc_shadow_permissions() {
        let entry = VersionedFileEntry::new_with_metadata(
            "/etc/shadow".to_string(),
            FileType::Regular,
            FilePermissions::new(0, 42, 0o640), // root:shadow
            true,
            1024,
            vec![1],
        );

        assert_eq!(entry.permissions.mode, 0o640);
        assert_eq!(entry.permissions.gid, 42); // shadow group
    }

    #[test]
    fn test_usr_bin_permissions() {
        // Typical system binaries
        let binaries = vec![
            ("/usr/bin/ls", 0, 0, 0o755),
            ("/usr/bin/sudo", 0, 0, 0o4755),    // setuid
            ("/usr/bin/crontab", 0, 0, 0o4755), // setuid
            ("/usr/bin/chsh", 0, 0, 0o4755),    // setuid
        ];

        for (path, uid, gid, mode) in binaries {
            let entry = VersionedFileEntry::new_with_metadata(
                path.to_string(),
                FileType::Regular,
                FilePermissions::new(uid, gid, mode),
                false,
                100000,
                vec![1, 2, 3],
            );

            assert_eq!(entry.permissions.mode, mode, "Mode mismatch for {}", path);
        }
    }

    #[test]
    fn test_var_permissions() {
        // Various /var directories
        let dirs = vec![
            ("/var/log", 0, 4, 0o2775),      // setgid, adm group
            ("/var/tmp", 0, 0, 0o1777),      // sticky
            ("/var/mail", 0, 8, 0o2775),     // setgid, mail group
            ("/var/cache/apt", 0, 0, 0o755), // normal
        ];

        for (path, uid, gid, mode) in dirs {
            let entry = VersionedFileEntry::new_with_metadata(
                path.to_string(),
                FileType::Directory,
                FilePermissions::new(uid, gid, mode),
                false,
                0,
                Vec::new(),
            );

            assert_eq!(entry.permissions.mode, mode, "Mode mismatch for {}", path);
        }
    }

    #[test]
    fn test_home_directory_permissions() {
        // Home directory patterns
        let entry = VersionedFileEntry::new_with_metadata(
            "/home/user".to_string(),
            FileType::Directory,
            FilePermissions::new(1000, 1000, 0o700), // Private home
            false,
            0,
            Vec::new(),
        );

        assert_eq!(entry.permissions.mode, 0o700);
        assert_eq!(entry.permissions.uid, 1000);
    }

    #[test]
    fn test_ssh_key_permissions() {
        // SSH private key must be 600
        let entry = VersionedFileEntry::new_with_metadata(
            "/home/user/.ssh/id_rsa".to_string(),
            FileType::Regular,
            FilePermissions::new(1000, 1000, 0o600),
            true,
            1024,
            vec![1],
        );

        assert_eq!(entry.permissions.mode, 0o600);
    }
}
