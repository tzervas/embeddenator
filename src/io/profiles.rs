//! Compression profiles for different filesystem use cases
//!
//! This module provides pre-configured compression profiles optimized for
//! different types of data found in operating systems and filesystems.
//!
//! ## Available Profiles
//!
//! | Profile     | Codec | Level | Use Case                           |
//! |-------------|-------|-------|-----------------------------------|
//! | `Kernel`    | zstd  | 19    | Boot files, vmlinuz, initramfs    |
//! | `Libraries` | zstd  | 9     | Shared libraries (.so, .dll)      |
//! | `Binaries`  | zstd  | 6     | Executables (/bin, /usr/bin)      |
//! | `Config`    | lz4   | -     | Configuration files (/etc)         |
//! | `Runtime`   | none  | -     | tmpfs, frequently mutating data    |
//! | `Archive`   | zstd  | 22    | Cold storage, backups              |
//! | `Balanced`  | zstd  | 3     | General purpose default            |
//!
//! ## Path-Based Auto-Selection
//!
//! The `CompressionProfiler` can automatically select appropriate profiles
//! based on file paths and extensions:
//!
//! ```rust,no_run
//! use embeddenator_io::CompressionProfiler;
//!
//! let profiler = CompressionProfiler::default();
//!
//! // Kernel files get maximum compression
//! let profile = profiler.for_path("/boot/vmlinuz");
//! assert_eq!(profile.name, "Kernel");
//!
//! // Config files get fast LZ4
//! let profile = profiler.for_path("/etc/passwd");
//! assert_eq!(profile.name, "Config");
//! ```

use super::envelope::{BinaryWriteOptions, CompressionCodec};

/// A compression profile with codec, level, and metadata
#[derive(Clone, Debug)]
pub struct CompressionProfile {
    /// Profile name for identification
    pub name: &'static str,
    /// Compression codec to use
    pub codec: CompressionCodec,
    /// Compression level (codec-specific, None = default)
    pub level: Option<i32>,
    /// Expected compression ratio (for planning)
    pub expected_ratio: f32,
    /// Brief description
    pub description: &'static str,
}

impl CompressionProfile {
    /// Create a new compression profile
    pub const fn new(
        name: &'static str,
        codec: CompressionCodec,
        level: Option<i32>,
        expected_ratio: f32,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            codec,
            level,
            expected_ratio,
            description,
        }
    }

    /// Convert to BinaryWriteOptions for the envelope module
    pub fn to_write_options(&self) -> BinaryWriteOptions {
        BinaryWriteOptions {
            codec: self.codec,
            level: self.level,
        }
    }
}

// Predefined profiles

/// Maximum compression for kernel and boot components
/// Use for: vmlinuz, initramfs, kernel modules
/// Trade-off: Slow compression, fast decompression, best ratio
pub const PROFILE_KERNEL: CompressionProfile = CompressionProfile::new(
    "Kernel",
    CompressionCodec::Zstd,
    Some(19),
    0.25, // ~4:1 compression typical for kernel images
    "Maximum compression for kernel/boot files",
);

/// Balanced compression for shared libraries
/// Use for: .so files, dynamically linked libraries
/// Trade-off: Good compression, reasonable speed
pub const PROFILE_LIBRARIES: CompressionProfile = CompressionProfile::new(
    "Libraries",
    CompressionCodec::Zstd,
    Some(9),
    0.40, // ~2.5:1 compression for compiled code
    "Balanced compression for shared libraries",
);

/// Moderate compression for executables
/// Use for: /bin, /sbin, /usr/bin binaries
/// Trade-off: Faster compression, decent ratio
pub const PROFILE_BINARIES: CompressionProfile = CompressionProfile::new(
    "Binaries",
    CompressionCodec::Zstd,
    Some(6),
    0.45, // ~2:1 compression for executables
    "Moderate compression for executables",
);

/// Fast compression for configuration files
/// Use for: /etc, small text configs, JSON, YAML
/// Trade-off: Very fast, lower ratio
pub const PROFILE_CONFIG: CompressionProfile = CompressionProfile::new(
    "Config",
    CompressionCodec::Lz4,
    None,
    0.50, // ~2:1 for text configs
    "Fast LZ4 compression for config files",
);

/// No compression for runtime/temporary data
/// Use for: tmpfs, frequently mutating files, memory-mapped
/// Trade-off: No CPU overhead, no size reduction
pub const PROFILE_RUNTIME: CompressionProfile = CompressionProfile::new(
    "Runtime",
    CompressionCodec::None,
    None,
    1.0, // No compression
    "No compression for runtime/temp data",
);

/// Maximum compression for cold storage/archives
/// Use for: Backups, infrequently accessed data
/// Trade-off: Very slow compression, best ratio
pub const PROFILE_ARCHIVE: CompressionProfile = CompressionProfile::new(
    "Archive",
    CompressionCodec::Zstd,
    Some(22), // Near-max zstd level
    0.20,     // ~5:1 compression
    "Maximum compression for archives/backups",
);

/// General-purpose balanced profile
/// Use for: Default when no specific profile applies
/// Trade-off: Fast compression, decent ratio
pub const PROFILE_BALANCED: CompressionProfile = CompressionProfile::new(
    "Balanced",
    CompressionCodec::Zstd,
    Some(3),
    0.55, // ~1.8:1 compression
    "General-purpose balanced compression",
);

/// Database and log files
/// Use for: SQLite, logs, journals
/// Trade-off: Good compression for structured data
pub const PROFILE_DATABASE: CompressionProfile = CompressionProfile::new(
    "Database",
    CompressionCodec::Zstd,
    Some(5),
    0.35, // ~3:1 for repetitive structured data
    "Compression for databases and logs",
);

/// Media files (usually pre-compressed)
/// Use for: Images, audio, video that are already compressed
/// Trade-off: Skip compression to avoid wasting CPU
pub const PROFILE_MEDIA: CompressionProfile = CompressionProfile::new(
    "Media",
    CompressionCodec::None,
    None,
    0.98, // Minimal gain on pre-compressed data
    "Skip compression for pre-compressed media",
);

/// All predefined profiles
pub const ALL_PROFILES: &[&CompressionProfile] = &[
    &PROFILE_KERNEL,
    &PROFILE_LIBRARIES,
    &PROFILE_BINARIES,
    &PROFILE_CONFIG,
    &PROFILE_RUNTIME,
    &PROFILE_ARCHIVE,
    &PROFILE_BALANCED,
    &PROFILE_DATABASE,
    &PROFILE_MEDIA,
];

/// Auto-select compression profiles based on file paths
#[derive(Clone, Debug)]
pub struct CompressionProfiler {
    /// Default profile when no pattern matches
    pub default_profile: CompressionProfile,
}

impl Default for CompressionProfiler {
    fn default() -> Self {
        Self {
            default_profile: PROFILE_BALANCED,
        }
    }
}

impl CompressionProfiler {
    /// Create a profiler with a custom default
    pub fn with_default(default: CompressionProfile) -> Self {
        Self {
            default_profile: default,
        }
    }

    /// Select compression profile based on file path
    pub fn for_path(&self, path: &str) -> CompressionProfile {
        // Normalize path for matching
        let path_lower = path.to_lowercase();

        // Boot/kernel paths
        if path_lower.starts_with("/boot")
            || path_lower.contains("vmlinuz")
            || path_lower.contains("initr")
            || path_lower.ends_with(".ko")
            || path_lower.ends_with(".ko.zst")
            || path_lower.ends_with(".ko.xz")
        {
            return PROFILE_KERNEL;
        }

        // Shared libraries
        if path_lower.ends_with(".so")
            || path_lower.contains(".so.")
            || path_lower.ends_with(".dll")
            || path_lower.starts_with("/lib")
            || path_lower.starts_with("/usr/lib")
        {
            return PROFILE_LIBRARIES;
        }

        // Executables
        if path_lower.starts_with("/bin")
            || path_lower.starts_with("/sbin")
            || path_lower.starts_with("/usr/bin")
            || path_lower.starts_with("/usr/sbin")
            || path_lower.starts_with("/usr/local/bin")
        {
            return PROFILE_BINARIES;
        }

        // Configuration files
        if path_lower.starts_with("/etc")
            || path_lower.ends_with(".conf")
            || path_lower.ends_with(".cfg")
            || path_lower.ends_with(".ini")
            || path_lower.ends_with(".yaml")
            || path_lower.ends_with(".yml")
            || path_lower.ends_with(".toml")
            || path_lower.ends_with(".json")
            || path_lower.ends_with(".xml")
        {
            return PROFILE_CONFIG;
        }

        // Runtime/temporary
        if path_lower.starts_with("/tmp")
            || path_lower.starts_with("/var/tmp")
            || path_lower.starts_with("/run")
            || path_lower.starts_with("/dev/shm")
            || path_lower.contains("/cache/")
        {
            return PROFILE_RUNTIME;
        }

        // Database and logs
        if path_lower.ends_with(".db")
            || path_lower.ends_with(".sqlite")
            || path_lower.ends_with(".sqlite3")
            || path_lower.ends_with(".log")
            || path_lower.starts_with("/var/log")
            || path_lower.ends_with(".journal")
        {
            return PROFILE_DATABASE;
        }

        // Media files (pre-compressed, skip)
        if path_lower.ends_with(".jpg")
            || path_lower.ends_with(".jpeg")
            || path_lower.ends_with(".png")
            || path_lower.ends_with(".gif")
            || path_lower.ends_with(".webp")
            || path_lower.ends_with(".mp3")
            || path_lower.ends_with(".mp4")
            || path_lower.ends_with(".mkv")
            || path_lower.ends_with(".webm")
            || path_lower.ends_with(".ogg")
            || path_lower.ends_with(".flac")
            || path_lower.ends_with(".zip")
            || path_lower.ends_with(".gz")
            || path_lower.ends_with(".xz")
            || path_lower.ends_with(".zst")
            || path_lower.ends_with(".bz2")
            || path_lower.ends_with(".7z")
            || path_lower.ends_with(".rar")
        {
            return PROFILE_MEDIA;
        }

        // Archive paths
        if path_lower.starts_with("/var/backups")
            || path_lower.starts_with("/backup")
            || path_lower.contains("/archive/")
        {
            return PROFILE_ARCHIVE;
        }

        // Default
        self.default_profile.clone()
    }

    /// Get profile by name
    pub fn by_name(&self, name: &str) -> Option<CompressionProfile> {
        ALL_PROFILES
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .map(|p| (*p).clone())
    }

    /// Estimate compressed size for planning
    pub fn estimate_compressed_size(&self, path: &str, original_size: usize) -> usize {
        let profile = self.for_path(path);
        (original_size as f32 * profile.expected_ratio) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_selection_kernel() {
        let profiler = CompressionProfiler::default();

        assert_eq!(profiler.for_path("/boot/vmlinuz").name, "Kernel");
        assert_eq!(profiler.for_path("/boot/initrd.img").name, "Kernel");
        assert_eq!(
            profiler.for_path("/lib/modules/5.4.0/ext4.ko").name,
            "Kernel"
        );
    }

    #[test]
    fn test_profile_selection_libraries() {
        let profiler = CompressionProfiler::default();

        assert_eq!(
            profiler.for_path("/lib/x86_64-linux-gnu/libc.so.6").name,
            "Libraries"
        );
        assert_eq!(profiler.for_path("/usr/lib/libssl.so.3").name, "Libraries");
    }

    #[test]
    fn test_profile_selection_binaries() {
        let profiler = CompressionProfiler::default();

        assert_eq!(profiler.for_path("/bin/bash").name, "Binaries");
        assert_eq!(profiler.for_path("/usr/bin/python3").name, "Binaries");
        assert_eq!(profiler.for_path("/sbin/init").name, "Binaries");
    }

    #[test]
    fn test_profile_selection_config() {
        let profiler = CompressionProfiler::default();

        assert_eq!(profiler.for_path("/etc/passwd").name, "Config");
        assert_eq!(profiler.for_path("/etc/nginx/nginx.conf").name, "Config");
        assert_eq!(profiler.for_path("/app/config.yaml").name, "Config");
    }

    #[test]
    fn test_profile_selection_runtime() {
        let profiler = CompressionProfiler::default();

        assert_eq!(profiler.for_path("/tmp/session.sock").name, "Runtime");
        assert_eq!(profiler.for_path("/run/systemd/notify").name, "Runtime");
    }

    #[test]
    fn test_profile_selection_media() {
        let profiler = CompressionProfiler::default();

        assert_eq!(profiler.for_path("/home/user/photo.jpg").name, "Media");
        assert_eq!(profiler.for_path("/var/data/video.mp4").name, "Media");
        assert_eq!(profiler.for_path("/archive/backup.tar.gz").name, "Media");
    }

    #[test]
    fn test_profile_to_write_options() {
        let profile = PROFILE_KERNEL;
        let opts = profile.to_write_options();

        assert_eq!(opts.codec, CompressionCodec::Zstd);
        assert_eq!(opts.level, Some(19));
    }

    #[test]
    fn test_estimate_compressed_size() {
        let profiler = CompressionProfiler::default();

        // Kernel: 25% of original
        let est = profiler.estimate_compressed_size("/boot/vmlinuz", 10_000_000);
        assert_eq!(est, 2_500_000);

        // Runtime: 100% (no compression)
        let est = profiler.estimate_compressed_size("/tmp/data", 10_000_000);
        assert_eq!(est, 10_000_000);
    }

    #[test]
    fn test_by_name() {
        let profiler = CompressionProfiler::default();

        assert!(profiler.by_name("Kernel").is_some());
        assert!(profiler.by_name("kernel").is_some()); // Case insensitive
        assert!(profiler.by_name("NonExistent").is_none());
    }
}
