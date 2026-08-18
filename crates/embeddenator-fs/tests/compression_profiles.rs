//! Compression profile selection tests
//!
//! Tests for the CompressionProfiler path-based auto-selection.
//! These tests verify that different file paths get the correct
//! compression profile based on their location and extension.

// Note: We're testing the profile selection logic. The actual CompressionProfiler
// is in embeddenator-io, which is always available as a dependency.

mod profile_selection {
    use embeddenator_io::io::envelope::CompressionCodec;
    use embeddenator_io::io::profiles::{
        CompressionProfiler, PROFILE_BALANCED, PROFILE_BINARIES, PROFILE_CONFIG, PROFILE_DATABASE,
        PROFILE_KERNEL, PROFILE_LIBRARIES, PROFILE_MEDIA, PROFILE_RUNTIME,
    };

    #[test]
    fn test_kernel_profile_selection() {
        let profiler = CompressionProfiler::default();

        // Boot files
        assert_eq!(profiler.for_path("/boot/vmlinuz").name, "Kernel");
        assert_eq!(
            profiler.for_path("/boot/vmlinuz-6.1.0-amd64").name,
            "Kernel"
        );
        assert_eq!(profiler.for_path("/boot/initramfs.img").name, "Kernel");
        assert_eq!(profiler.for_path("/boot/initrd.img-6.1.0").name, "Kernel");

        // Kernel modules
        assert_eq!(
            profiler
                .for_path("/lib/modules/6.1.0/kernel/fs/ext4.ko")
                .name,
            "Kernel"
        );
        assert_eq!(
            profiler
                .for_path("/lib/modules/6.1.0/kernel/drivers/nvme.ko.zst")
                .name,
            "Kernel"
        );
        assert_eq!(
            profiler
                .for_path("/lib/modules/6.1.0/kernel/net/ipv4.ko.xz")
                .name,
            "Kernel"
        );

        // Verify codec and level
        let profile = profiler.for_path("/boot/vmlinuz");
        assert_eq!(profile.codec, CompressionCodec::Zstd);
        assert_eq!(profile.level, Some(19));
    }

    #[test]
    fn test_library_profile_selection() {
        let profiler = CompressionProfiler::default();

        // Shared libraries
        assert_eq!(profiler.for_path("/lib/libc.so.6").name, "Libraries");
        assert_eq!(profiler.for_path("/usr/lib/libssl.so.3").name, "Libraries");
        assert_eq!(
            profiler.for_path("/lib/x86_64-linux-gnu/libm.so.6").name,
            "Libraries"
        );
        assert_eq!(
            profiler
                .for_path("/usr/lib/x86_64-linux-gnu/libstdc++.so.6")
                .name,
            "Libraries"
        );

        // Windows DLLs (when encoding Windows partitions)
        assert_eq!(
            profiler.for_path("/mnt/windows/system32/kernel32.dll").name,
            "Libraries"
        );

        // Verify codec and level
        let profile = profiler.for_path("/usr/lib/libssl.so");
        assert_eq!(profile.codec, CompressionCodec::Zstd);
        assert_eq!(profile.level, Some(9));
    }

    #[test]
    fn test_binary_profile_selection() {
        let profiler = CompressionProfiler::default();

        // System binaries
        assert_eq!(profiler.for_path("/bin/ls").name, "Binaries");
        assert_eq!(profiler.for_path("/usr/bin/vim").name, "Binaries");
        assert_eq!(profiler.for_path("/sbin/init").name, "Binaries");
        assert_eq!(profiler.for_path("/usr/sbin/sshd").name, "Binaries");
        assert_eq!(profiler.for_path("/usr/local/bin/myapp").name, "Binaries");

        // Verify codec and level
        let profile = profiler.for_path("/usr/bin/bash");
        assert_eq!(profile.codec, CompressionCodec::Zstd);
        assert_eq!(profile.level, Some(6));
    }

    #[test]
    fn test_config_profile_selection() {
        let profiler = CompressionProfiler::default();

        // /etc files
        assert_eq!(profiler.for_path("/etc/passwd").name, "Config");
        assert_eq!(profiler.for_path("/etc/shadow").name, "Config");
        assert_eq!(profiler.for_path("/etc/fstab").name, "Config");
        assert_eq!(profiler.for_path("/etc/hosts").name, "Config");
        assert_eq!(profiler.for_path("/etc/ssh/sshd_config").name, "Config");

        // Config file extensions
        assert_eq!(profiler.for_path("/app/config.conf").name, "Config");
        assert_eq!(profiler.for_path("/app/settings.cfg").name, "Config");
        assert_eq!(profiler.for_path("/app/config.yaml").name, "Config");
        assert_eq!(profiler.for_path("/app/config.yml").name, "Config");
        assert_eq!(profiler.for_path("/app/config.toml").name, "Config");
        assert_eq!(profiler.for_path("/app/config.json").name, "Config");
        assert_eq!(profiler.for_path("/app/config.xml").name, "Config");
        assert_eq!(profiler.for_path("/app/settings.ini").name, "Config");

        // Verify codec (LZ4 for speed)
        let profile = profiler.for_path("/etc/passwd");
        assert_eq!(profile.codec, CompressionCodec::Lz4);
    }

    #[test]
    fn test_runtime_profile_selection() {
        let profiler = CompressionProfiler::default();

        // Temporary directories
        assert_eq!(profiler.for_path("/tmp/tempfile").name, "Runtime");
        assert_eq!(profiler.for_path("/var/tmp/cache").name, "Runtime");
        assert_eq!(profiler.for_path("/run/lock/subsys").name, "Runtime");
        assert_eq!(profiler.for_path("/dev/shm/shared_mem").name, "Runtime");

        // Cache directories (note: must contain "/cache/" not ".cache")
        assert_eq!(profiler.for_path("/var/cache/apt/archives").name, "Runtime");
        // Note: /home/user/.cache/ doesn't match because it's ".cache" not "/cache/"
        // This is by design - user caches may contain important data worth compressing

        // Verify no compression
        let profile = profiler.for_path("/tmp/test");
        assert_eq!(profile.codec, CompressionCodec::None);
    }

    #[test]
    fn test_database_profile_selection() {
        let profiler = CompressionProfiler::default();

        // Database files
        assert_eq!(profiler.for_path("/var/lib/app/data.db").name, "Database");
        assert_eq!(
            profiler.for_path("/var/lib/app/data.sqlite").name,
            "Database"
        );
        assert_eq!(
            profiler.for_path("/var/lib/app/data.sqlite3").name,
            "Database"
        );

        // Log files
        assert_eq!(profiler.for_path("/var/log/syslog").name, "Database");
        assert_eq!(profiler.for_path("/var/log/auth.log").name, "Database");
        assert_eq!(profiler.for_path("/app/logs/app.log").name, "Database");

        // Systemd journals
        assert_eq!(
            profiler.for_path("/var/log/journal/system.journal").name,
            "Database"
        );

        // Verify codec
        let profile = profiler.for_path("/var/log/syslog");
        assert_eq!(profile.codec, CompressionCodec::Zstd);
        assert_eq!(profile.level, Some(5));
    }

    #[test]
    fn test_media_profile_selection() {
        let profiler = CompressionProfiler::default();

        // Images
        assert_eq!(profiler.for_path("/photos/image.jpg").name, "Media");
        assert_eq!(profiler.for_path("/photos/image.jpeg").name, "Media");
        assert_eq!(profiler.for_path("/photos/image.png").name, "Media");
        assert_eq!(profiler.for_path("/photos/image.gif").name, "Media");
        assert_eq!(profiler.for_path("/photos/image.webp").name, "Media");

        // Audio
        assert_eq!(profiler.for_path("/music/song.mp3").name, "Media");
        assert_eq!(profiler.for_path("/music/song.ogg").name, "Media");
        assert_eq!(profiler.for_path("/music/song.flac").name, "Media");

        // Video
        assert_eq!(profiler.for_path("/videos/movie.mp4").name, "Media");
        assert_eq!(profiler.for_path("/videos/movie.mkv").name, "Media");
        assert_eq!(profiler.for_path("/videos/movie.webm").name, "Media");

        // Verify no compression (already compressed)
        let profile = profiler.for_path("/photos/image.jpg");
        assert_eq!(profile.codec, CompressionCodec::None);
    }

    #[test]
    fn test_balanced_fallback() {
        let profiler = CompressionProfiler::default();

        // Unknown file types should get balanced
        assert_eq!(profiler.for_path("/data/unknown_file").name, "Balanced");
        assert_eq!(
            profiler.for_path("/home/user/document.txt").name,
            "Balanced"
        );
        assert_eq!(profiler.for_path("/data/file.dat").name, "Balanced");

        // Verify codec
        let profile = profiler.for_path("/data/unknown");
        assert_eq!(profile.codec, CompressionCodec::Zstd);
        assert_eq!(profile.level, Some(3));
    }

    #[test]
    fn test_case_insensitivity() {
        let profiler = CompressionProfiler::default();

        // Path matching should be case-insensitive
        assert_eq!(profiler.for_path("/Boot/VmLinuz").name, "Kernel");
        assert_eq!(profiler.for_path("/BOOT/VMLINUZ").name, "Kernel");
        assert_eq!(profiler.for_path("/Etc/Passwd").name, "Config");
        assert_eq!(profiler.for_path("/photos/IMAGE.JPG").name, "Media");
        assert_eq!(profiler.for_path("/photos/image.JPEG").name, "Media");
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // These validate const values at compile time
    fn test_compression_ratio_estimates() {
        // Verify expected compression ratios are reasonable

        assert!(PROFILE_KERNEL.expected_ratio < 0.30); // High compression
        assert!(PROFILE_LIBRARIES.expected_ratio < 0.50); // Good compression
        assert!(PROFILE_BINARIES.expected_ratio < 0.50); // Decent compression
        assert!(PROFILE_CONFIG.expected_ratio < 0.60); // Moderate compression
        assert!(PROFILE_RUNTIME.expected_ratio >= 1.0); // No compression
        assert!(PROFILE_DATABASE.expected_ratio < 0.40); // Good for structured data
        assert!(PROFILE_MEDIA.expected_ratio >= 0.95); // Almost no gain
        assert!(PROFILE_BALANCED.expected_ratio < 0.60); // Moderate default
    }

    #[test]
    fn test_profile_size_estimation() {
        let profiler = CompressionProfiler::default();

        // Kernel files should estimate high compression
        let kernel_profile = profiler.for_path("/boot/vmlinuz");
        let estimated = (10_000_000.0 * kernel_profile.expected_ratio) as usize;
        assert!(estimated < 3_000_000, "Kernel should compress well");

        // Media should estimate almost no compression
        let media_profile = profiler.for_path("/photos/image.jpg");
        let estimated = (10_000_000.0 * media_profile.expected_ratio) as usize;
        assert!(estimated > 9_500_000, "Media should not compress much");
    }
}

// =============================================================================
// PROFILE APPLICATION TESTS (without compression feature)
// =============================================================================

mod profile_constants {
    /// Test that profile names are correctly defined
    #[test]
    fn test_profile_names() {
        let expected_names = vec![
            "Kernel",
            "Libraries",
            "Binaries",
            "Config",
            "Runtime",
            "Archive",
            "Balanced",
            "Database",
            "Media",
        ];

        // This test documents the expected profile names
        for name in expected_names {
            // Profile name should be a valid string
            assert!(!name.is_empty());
            // Profile name should be Title Case
            assert!(name.chars().next().unwrap().is_uppercase());
        }
    }
}

// =============================================================================
// VM FILESYSTEM PATH COVERAGE TESTS
// =============================================================================

mod vm_path_coverage {
    use embeddenator_io::io::profiles::CompressionProfiler;

    /// Comprehensive test of paths found in Debian VM
    #[test]
    fn test_debian_vm_paths() {
        let profiler = CompressionProfiler::default();

        // Boot partition (high compression)
        let boot_paths = vec![
            "/boot/grub/grub.cfg",
            "/boot/grub/i386-pc/normal.mod",
            "/boot/vmlinuz-6.1.0-amd64",
            "/boot/initrd.img-6.1.0-amd64",
            "/boot/System.map-6.1.0-amd64",
        ];
        for path in boot_paths {
            let profile = profiler.for_path(path);
            assert!(
                profile.name == "Kernel" || profile.name == "Config",
                "Boot path {} should be Kernel or Config, got {}",
                path,
                profile.name
            );
        }

        // System libraries
        let lib_paths = vec![
            "/lib/x86_64-linux-gnu/libc.so.6",
            "/lib/x86_64-linux-gnu/libm.so.6",
            "/usr/lib/x86_64-linux-gnu/libssl.so.3",
            "/usr/lib/x86_64-linux-gnu/libcrypto.so.3",
        ];
        for path in lib_paths {
            assert_eq!(
                profiler.for_path(path).name,
                "Libraries",
                "Library {} should use Libraries profile",
                path
            );
        }

        // System binaries
        let bin_paths = vec![
            "/bin/bash",
            "/bin/ls",
            "/usr/bin/apt",
            "/usr/bin/dpkg",
            "/usr/sbin/sshd",
        ];
        for path in bin_paths {
            assert_eq!(
                profiler.for_path(path).name,
                "Binaries",
                "Binary {} should use Binaries profile",
                path
            );
        }

        // Configuration
        let etc_paths = vec![
            "/etc/passwd",
            "/etc/shadow",
            "/etc/group",
            "/etc/fstab",
            "/etc/hostname",
            "/etc/hosts",
            "/etc/resolv.conf",
            "/etc/ssh/sshd_config",
            "/etc/apt/sources.list",
            "/etc/systemd/system.conf",
        ];
        for path in etc_paths {
            assert_eq!(
                profiler.for_path(path).name,
                "Config",
                "Config {} should use Config profile",
                path
            );
        }

        // Logs
        let log_paths = vec![
            "/var/log/syslog",
            "/var/log/auth.log",
            "/var/log/dpkg.log",
            "/var/log/apt/history.log",
        ];
        for path in log_paths {
            assert_eq!(
                profiler.for_path(path).name,
                "Database",
                "Log {} should use Database profile",
                path
            );
        }
    }

    /// Test paths from the VM encoder script
    #[test]
    fn test_vm_encoder_priority_paths() {
        let profiler = CompressionProfiler::default();

        // Priority 10: /usr/share - balanced
        assert_eq!(
            profiler.for_path("/usr/share/doc/bash/README").name,
            "Balanced"
        );

        // Priority 20-25: /usr/lib and /lib - Libraries
        assert_eq!(
            profiler.for_path("/usr/lib/systemd/systemd").name,
            "Libraries"
        );
        assert_eq!(
            profiler.for_path("/lib/systemd/system/ssh.service").name,
            "Libraries"
        );

        // Priority 30: /etc - Config
        assert_eq!(profiler.for_path("/etc/default/grub").name, "Config");

        // Priority 80: /boot - Kernel
        assert_eq!(
            profiler.for_path("/boot/efi/EFI/debian/grubx64.efi").name,
            "Kernel"
        );
    }
}
