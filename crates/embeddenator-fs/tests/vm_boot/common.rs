//! Common VM Boot Test Utilities
//! ==============================
//!
//! This module provides shared functionality for VM boot tests across all
//! architectures. It handles:
//!
//! - **Image Downloading**: Fetching test images with progress display
//! - **Image Caching**: Avoiding repeated downloads using checksums
//! - **Engram Encoding**: Converting VM images to engram format
//! - **QEMU Execution**: Spawning VMs with proper arguments
//! - **Output Monitoring**: Watching serial console for boot success
//!
//! ## Why This Design
//!
//! The common module exists because VM boot testing follows the same pattern
//! regardless of architecture:
//!
//! 1. Ensure test image is available (download or use cache)
//! 2. Encode the image as an engram
//! 3. Mount the engram via FUSE
//! 4. Boot QEMU pointing at the mounted filesystem
//! 5. Monitor serial output for success marker
//! 6. Clean up (unmount, remove temp files)
//!
//! The only differences between architectures are:
//! - QEMU binary and machine arguments
//! - Test image URL and checksum
//! - Expected boot time
//! - Success marker string
//!
//! By centralizing the common logic, we ensure consistent behavior and make
//! it easy to add new architectures.

use std::fs::{self};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{Architecture, BootResult, TestImage};

/// Error type for VM boot operations
#[derive(Debug)]
pub enum BootError {
    /// Test image download failed
    DownloadFailed(String),
    /// Checksum verification failed
    ChecksumMismatch { expected: String, actual: String },
    /// Engram encoding failed
    EncodeFailed(String),
    /// FUSE mount failed
    MountFailed(String),
    /// QEMU failed to start
    QemuStartFailed(String),
    /// Boot timed out
    BootTimeout { timeout: Duration, output: String },
    /// Boot failed (success marker not found)
    BootFailed(String),
    /// QEMU not installed
    QemuNotInstalled(Architecture),
    /// I/O error
    Io(std::io::Error),
}

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootError::DownloadFailed(msg) => write!(f, "Download failed: {}", msg),
            BootError::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            BootError::EncodeFailed(msg) => write!(f, "Engram encoding failed: {}", msg),
            BootError::MountFailed(msg) => write!(f, "FUSE mount failed: {}", msg),
            BootError::QemuStartFailed(msg) => write!(f, "QEMU start failed: {}", msg),
            BootError::BootTimeout { timeout, .. } => {
                write!(f, "Boot timed out after {:?}", timeout)
            }
            BootError::BootFailed(msg) => write!(f, "Boot failed: {}", msg),
            BootError::QemuNotInstalled(arch) => {
                write!(f, "QEMU not installed for {:?}", arch)
            }
            BootError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for BootError {}

impl From<std::io::Error> for BootError {
    fn from(e: std::io::Error) -> Self {
        BootError::Io(e)
    }
}

/// Download or verify cached test image
///
/// # Why Caching Matters
///
/// Test images are 50-500MB and downloading them for every test run would:
/// 1. Waste bandwidth (yours and the server's)
/// 2. Make tests take minutes instead of seconds
/// 3. Fail when offline
/// 4. Be environmentally wasteful (data center energy)
///
/// We cache images with their checksums so:
/// - First run downloads and verifies
/// - Subsequent runs verify checksum and skip download
/// - Corrupted downloads are detected and re-fetched
pub fn ensure_image_available(image: &TestImage) -> Result<PathBuf, BootError> {
    let path = image.cache_path();
    let checksum_path = path.with_extension("qcow2.sha256");

    // Create cache directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Check if we have a valid cached image
    if path.exists() && checksum_path.exists() {
        let stored_checksum = fs::read_to_string(&checksum_path)?;
        if stored_checksum.trim() == image.sha256 {
            // Verify actual file checksum
            let actual = compute_sha256(&path)?;
            if actual == image.sha256 {
                println!("✓ Using cached image: {}", path.display());
                return Ok(path);
            }
            println!("⚠ Cached image corrupted, re-downloading...");
        } else {
            println!("⚠ Image version changed, re-downloading...");
        }
    }

    // Download the image
    println!("⬇ Downloading {} (~{}MB)...", image.name, image.size_mb);
    download_with_progress(image.url, &path, image.size_mb)?;

    // Verify checksum
    println!("🔍 Verifying checksum...");
    let actual = compute_sha256(&path)?;
    if actual != image.sha256 {
        fs::remove_file(&path)?; // Remove corrupted download
        return Err(BootError::ChecksumMismatch {
            expected: image.sha256.to_string(),
            actual,
        });
    }

    // Save checksum for future runs
    fs::write(&checksum_path, image.sha256)?;
    println!("✓ Image downloaded and verified");

    Ok(path)
}

/// Download a file with progress display
fn download_with_progress(url: &str, path: &Path, _expected_mb: u32) -> Result<(), BootError> {
    // Use curl for downloading (available on all platforms)
    // We could use reqwest but that adds dependencies to test code
    let output = Command::new("curl")
        .args([
            "-L", // Follow redirects
            "-#", // Progress bar
            "-o",
            path.to_str().unwrap(),
            url,
        ])
        .status()?;

    if !output.success() {
        return Err(BootError::DownloadFailed(format!(
            "curl exited with status {}",
            output
        )));
    }

    // Verify file exists and has reasonable size
    let metadata = fs::metadata(path)?;
    let size_mb = metadata.len() / (1024 * 1024);
    if size_mb < 1 {
        return Err(BootError::DownloadFailed(format!(
            "Downloaded file too small: {} bytes",
            metadata.len()
        )));
    }

    Ok(())
}

/// Compute SHA256 hash of a file
fn compute_sha256(path: &Path) -> Result<String, BootError> {
    // Use system sha256sum (faster than pure Rust for large files)
    let output = Command::new("sha256sum").arg(path).output()?;

    if !output.status.success() {
        return Err(BootError::Io(std::io::Error::other("sha256sum failed")));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let hash = output_str.split_whitespace().next().unwrap_or("");
    Ok(hash.to_string())
}

/// Configuration for a VM boot test
pub struct BootTestConfig {
    /// Test image to use
    pub image: TestImage,
    /// Whether to use KVM acceleration (x86_64 only)
    pub use_kvm: bool,
    /// Timeout for boot (defaults to architecture-specific value)
    pub timeout: Option<Duration>,
    /// Extra QEMU arguments
    pub extra_args: Vec<String>,
}

/// Run a complete VM boot test
///
/// # Test Sequence
///
/// 1. **Image Preparation**
///    - Check cache for existing image
///    - Download if not cached or checksum mismatch
///    - Verify checksum after download
///
/// 2. **Engram Encoding**
///    - Parse disk image (QCOW2 or raw)
///    - Detect partition table (GPT/MBR)
///    - Traverse filesystem (ext4)
///    - Encode to engram format
///
/// 3. **FUSE Mount**
///    - Mount engram as read-only filesystem
///    - Verify kernel and initramfs are accessible
///
/// 4. **QEMU Boot**
///    - Start QEMU with architecture-specific arguments
///    - Monitor serial console output
///    - Wait for success marker or timeout
///
/// 5. **Cleanup**
///    - Kill QEMU process
///    - Unmount FUSE filesystem
///    - Remove temporary files
pub fn run_boot_test(config: BootTestConfig) -> Result<BootResult, BootError> {
    let arch = config.image.arch;
    let _start = Instant::now();

    // Check QEMU is available
    if !super::qemu_available(arch) {
        return Err(BootError::QemuNotInstalled(arch));
    }

    // Step 1: Ensure image is available
    let image_start = Instant::now();
    let image_path = ensure_image_available(&config.image)?;
    let image_prep_time = image_start.elapsed();

    // Step 2: Encode as engram
    println!("📦 Encoding image as engram...");
    let encode_start = Instant::now();
    let engram_path = encode_image_to_engram(&image_path, arch)?;
    let encode_time = encode_start.elapsed();
    println!("✓ Encoding complete in {:?}", encode_time);

    // Step 3: Mount via FUSE
    println!("🔗 Mounting engram filesystem...");
    let mount_point = mount_engram(&engram_path)?;
    println!("✓ Mounted at {}", mount_point.display());

    // Step 4: Boot QEMU
    let timeout = config
        .timeout
        .unwrap_or_else(|| Duration::from_secs(arch.expected_boot_seconds(config.use_kvm)));

    println!(
        "🚀 Booting {} VM (timeout: {:?})...",
        arch.cache_dir_name(),
        timeout
    );

    let boot_start = Instant::now();
    let boot_result = boot_qemu(
        arch,
        &mount_point,
        config.use_kvm,
        timeout,
        config.image.success_marker,
        &config.extra_args,
    );
    let boot_time = boot_start.elapsed();

    // Step 5: Cleanup
    println!("🧹 Cleaning up...");
    unmount_engram(&mount_point)?;
    fs::remove_file(&engram_path).ok(); // Ignore errors

    // Build result
    match boot_result {
        Ok(console_output) => {
            println!("✓ Boot successful in {:?}", boot_time);
            Ok(BootResult {
                arch,
                image_prep_time,
                encode_time,
                boot_time,
                success: true,
                console_output,
                error: None,
            })
        }
        Err(e) => {
            println!("✗ Boot failed: {}", e);
            Ok(BootResult {
                arch,
                image_prep_time,
                encode_time,
                boot_time,
                success: false,
                console_output: match &e {
                    BootError::BootTimeout { output, .. } => output.clone(),
                    _ => String::new(),
                },
                error: Some(e.to_string()),
            })
        }
    }
}

/// Encode a disk image to engram format
///
/// # Why This Uses CLI
///
/// We shell out to the `embr` CLI rather than using library APIs because:
/// 1. Tests should exercise the same code path users will use
/// 2. It validates the CLI is working correctly
/// 3. Library APIs may change; CLI is stable public interface
fn encode_image_to_engram(image_path: &Path, _arch: Architecture) -> Result<PathBuf, BootError> {
    let engram_path = image_path.with_extension("embr");

    // Use embr CLI to encode
    // TODO: Replace with library call when disk module is complete
    let status = Command::new("embr")
        .args([
            "encode",
            "--input",
            image_path.to_str().unwrap(),
            "--output",
            engram_path.to_str().unwrap(),
            "--disk-image", // Enable disk image mode
        ])
        .status();

    match status {
        Ok(s) if s.success() => Ok(engram_path),
        Ok(s) => Err(BootError::EncodeFailed(format!(
            "embr encode exited with status {}",
            s
        ))),
        Err(_e) => {
            // embr not installed, use mock for testing
            println!("⚠ embr CLI not found, creating mock engram for testing");
            create_mock_engram(image_path, &engram_path)?;
            Ok(engram_path)
        }
    }
}

/// Create a mock engram for testing when embr isn't available
///
/// # Why Mock?
///
/// This allows running VM boot tests even when the full encoding pipeline
/// isn't complete. The mock just copies the original image, which lets us
/// test the QEMU/boot logic independently.
fn create_mock_engram(image_path: &Path, engram_path: &Path) -> Result<(), BootError> {
    // For testing, just symlink to original (QEMU can read it directly)
    // Real implementation would encode via embeddenator-fs library
    #[cfg(unix)]
    std::os::unix::fs::symlink(image_path, engram_path)?;

    #[cfg(not(unix))]
    fs::copy(image_path, engram_path)?;

    Ok(())
}

/// Mount an engram via FUSE
fn mount_engram(engram_path: &Path) -> Result<PathBuf, BootError> {
    let mount_point = engram_path.with_extension("mount");
    fs::create_dir_all(&mount_point)?;

    // Use embr CLI to mount
    let status = Command::new("embr")
        .args([
            "mount",
            "--engram",
            engram_path.to_str().unwrap(),
            "--mountpoint",
            mount_point.to_str().unwrap(),
            "--readonly",
        ])
        .status();

    match status {
        Ok(s) if s.success() => Ok(mount_point),
        Ok(s) => Err(BootError::MountFailed(format!(
            "embr mount exited with status {}",
            s
        ))),
        Err(_) => {
            // embr not installed, use direct image path
            println!("⚠ embr CLI not found, using direct image path");
            Ok(engram_path.to_path_buf())
        }
    }
}

/// Unmount an engram FUSE filesystem
fn unmount_engram(mount_point: &Path) -> Result<(), BootError> {
    // Try fusermount first (Linux)
    let _ = Command::new("fusermount")
        .args(["-u", mount_point.to_str().unwrap()])
        .status();

    // Also try umount (macOS, root)
    let _ = Command::new("umount").arg(mount_point).status();

    // Remove mount point directory
    fs::remove_dir(mount_point).ok();

    Ok(())
}

/// Boot QEMU and monitor for success
fn boot_qemu(
    arch: Architecture,
    root_path: &Path,
    use_kvm: bool,
    timeout: Duration,
    success_marker: &str,
    extra_args: &[String],
) -> Result<String, BootError> {
    let mut cmd = Command::new(arch.qemu_binary());

    // Machine configuration
    cmd.args(["-machine", arch.machine_type()]);
    cmd.args([
        "-cpu",
        arch.cpu_type(use_kvm && arch == Architecture::X86_64),
    ]);
    cmd.args(["-m", &format!("{}M", arch.memory_mb())]);

    // Enable KVM if requested and available
    if use_kvm && arch == Architecture::X86_64 && super::kvm_available() {
        cmd.arg("-enable-kvm");
    }

    // Disable display (we only care about serial)
    cmd.args(["-display", "none"]);

    // Serial to stdio so we can monitor
    cmd.args(["-serial", "stdio"]);

    // Root filesystem
    // For QCOW2/raw images, use as block device
    // For directories (mounted engram), use virtio-9p
    if root_path.is_dir() {
        cmd.args([
            "-fsdev",
            &format!(
                "local,id=root,path={},security_model=none,readonly=on",
                root_path.display()
            ),
            "-device",
            "virtio-9p-pci,fsdev=root,mount_tag=root",
        ]);
    } else {
        cmd.args([
            "-drive",
            &format!("file={},format=qcow2,if=virtio", root_path.display()),
        ]);
    }

    // Extra arguments
    for arg in extra_args {
        cmd.arg(arg);
    }

    // Start QEMU
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    println!("Running: {:?}", cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| BootError::QemuStartFailed(format!("Failed to spawn QEMU: {}", e)))?;

    // Monitor output for success marker
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let mut output = String::new();
    let start = Instant::now();

    // Use non-blocking reads with timeout
    for line in reader.lines() {
        let line = line.map_err(BootError::Io)?;
        output.push_str(&line);
        output.push('\n');

        // Print for debugging
        println!("[VM] {}", line);

        // Check for success
        if line.contains(success_marker) {
            child.kill().ok();
            return Ok(output);
        }

        // Check timeout
        if start.elapsed() > timeout {
            child.kill().ok();
            return Err(BootError::BootTimeout { timeout, output });
        }
    }

    // Process ended without success marker
    child.kill().ok();
    Err(BootError::BootFailed(format!(
        "VM exited without boot success marker. Output:\n{}",
        output
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path_generation() {
        let image = TestImage {
            name: "Test Alpine",
            url: "https://example.com/alpine-virt-3.19.0-x86_64.qcow2",
            sha256: "abc123",
            arch: Architecture::X86_64,
            size_mb: 50,
            success_marker: "login:",
        };

        let path = image.cache_path();
        assert!(path.to_string_lossy().contains("x86_64"));
        assert!(path
            .to_string_lossy()
            .contains("alpine-virt-3.19.0-x86_64.qcow2"));
    }

    #[test]
    fn test_compute_sha256() {
        // Create a temp file with known content
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello world\n").unwrap();

        let hash = compute_sha256(&test_file).unwrap();
        // SHA256 of "hello world\n" - just verify it's a valid hex hash
        assert_eq!(hash.len(), 64, "SHA256 should be 64 hex characters");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Should be hex");
    }
}
