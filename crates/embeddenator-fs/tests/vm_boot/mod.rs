//! VM Boot Validation Test Suite
//! ==============================
//!
//! This module provides LOCAL-ONLY validation tests that verify engram-encoded
//! VM images can actually boot on their target architectures.
//!
//! ## Why This Exists
//!
//! The ultimate validation of a VM filesystem encoding is whether the VM boots.
//! Unit tests can verify individual components, but only a real boot test proves
//! the entire pipeline works:
//!
//! 1. Disk image parsing (QCOW2/raw)
//! 2. Partition table detection (GPT/MBR)
//! 3. Filesystem traversal (ext4)
//! 4. VSA encoding with correct metadata
//! 5. FUSE mount providing valid filesystem
//! 6. Kernel finding and loading initramfs
//! 7. Init system mounting root and reaching login
//!
//! Any bug in any layer will cause boot failure, making this an effective
//! integration test.
//!
//! ## Architecture Support
//!
//! We test three architectures to ensure platform independence:
//!
//! | Architecture | Execution Mode | Expected Speed | Real Use Case |
//! |--------------|----------------|----------------|---------------|
//! | x86_64       | KVM (native)   | ~3 seconds     | Cloud VMs, desktops |
//! | aarch64      | TCG (emulated) | ~30 seconds    | ARM servers, mobile |
//! | riscv64      | TCG (emulated) | ~45 seconds    | Embedded, research |
//!
//! ## Why Local-Only (No CI)
//!
//! These tests are explicitly excluded from CI because:
//!
//! 1. **QEMU Installation**: CI runners may not have QEMU with all targets
//! 2. **KVM Access**: Nested virtualization is unreliable in cloud CI
//! 3. **Test Duration**: Emulated boots take 30-90 seconds each
//! 4. **Disk Space**: VM images are 50-500MB even when minimized
//! 5. **Flakiness**: VM boot timing is inherently variable
//!
//! For CI, we use component-level tests that verify each layer in isolation.
//!
//! ## Test Image Philosophy
//!
//! We use the SMALLEST POSSIBLE images that still prove the encoding works:
//!
//! - **Minimal kernel**: Just enough to boot and print to serial
//! - **Tiny initramfs**: Busybox or static init that echoes success
//! - **No GUI**: Serial console output only
//! - **No network**: Reduces attack surface and boot time
//! - **No extra packages**: Base system only
//!
//! Image sources (in order of preference):
//! 1. Pre-built test images from buildroot/alpine
//! 2. Minimal cloud images (Alpine, Tiny Core Linux)
//! 3. Custom-built minimal rootfs
//!
//! ## Caching Strategy
//!
//! Test images are cached to avoid repeated downloads:
//!
//! ```text
//! ~/.cache/embeddenator/vm-images/
//! ├── x86_64/
//! │   ├── alpine-virt-3.19.0-x86_64.qcow2  (50MB)
//! │   └── alpine-virt-3.19.0-x86_64.qcow2.sha256
//! ├── aarch64/
//! │   ├── alpine-virt-3.19.0-aarch64.qcow2 (50MB)
//! │   └── alpine-virt-3.19.0-aarch64.qcow2.sha256
//! └── riscv64/
//!     ├── fedora-minimal-riscv64.qcow2 (200MB)
//!     └── fedora-minimal-riscv64.qcow2.sha256
//! ```
//!
//! Images are only re-downloaded if:
//! - The checksum file is missing
//! - The image file doesn't match its checksum
//! - The image version is updated in this code
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all VM boot tests (requires QEMU)
//! cargo test --test vm_boot -- --ignored --nocapture
//!
//! # Run specific architecture
//! cargo test --test vm_boot test_x86_64 -- --ignored --nocapture
//!
//! # Override cache directory
//! VM_IMAGE_CACHE=/tmp/vm-images cargo test --test vm_boot -- --ignored
//! ```
//!
//! ## Test Output
//!
//! Each test outputs:
//! - Image download/cache status
//! - Engram encoding time
//! - FUSE mount status
//! - QEMU command being executed
//! - Serial console output (for debugging failures)
//! - Boot success/failure and timing
//!
//! ## Debugging Boot Failures
//!
//! If a VM fails to boot:
//!
//! 1. Check serial output for kernel panic or init failure
//! 2. Verify engram contains all expected files: `embr ls --recursive`
//! 3. Mount engram manually and compare to source image
//! 4. Try booting source image directly to rule out image corruption
//! 5. Check QEMU arguments (memory, CPU, devices)

pub mod aarch64;
pub mod cache;
pub mod common;
pub mod riscv64;
pub mod x86_64;

use std::path::PathBuf;
use std::time::Duration;

/// Result of a VM boot test
#[derive(Debug)]
pub struct BootResult {
    /// Architecture that was tested
    pub arch: Architecture,
    /// Time to download/prepare image (cached = 0)
    pub image_prep_time: Duration,
    /// Time to encode image as engram
    pub encode_time: Duration,
    /// Time from QEMU start to login prompt
    pub boot_time: Duration,
    /// Whether boot succeeded
    pub success: bool,
    /// Serial console output (for debugging)
    pub console_output: String,
    /// Any error message
    pub error: Option<String>,
}

/// Supported VM architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Riscv64,
}

impl Architecture {
    /// QEMU system emulator binary name
    pub fn qemu_binary(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "qemu-system-x86_64",
            Architecture::Aarch64 => "qemu-system-aarch64",
            Architecture::Riscv64 => "qemu-system-riscv64",
        }
    }

    /// Default machine type for this architecture
    pub fn machine_type(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "q35",
            Architecture::Aarch64 => "virt",
            Architecture::Riscv64 => "virt",
        }
    }

    /// CPU type (use KVM on x86_64 if available)
    pub fn cpu_type(&self, kvm_available: bool) -> &'static str {
        match self {
            Architecture::X86_64 if kvm_available => "host",
            Architecture::X86_64 => "qemu64",
            Architecture::Aarch64 => "cortex-a72",
            Architecture::Riscv64 => "rv64",
        }
    }

    /// Recommended memory in MB
    pub fn memory_mb(&self) -> u32 {
        match self {
            // Minimal Linux needs ~128MB, 256 gives headroom
            Architecture::X86_64 => 256,
            Architecture::Aarch64 => 256,
            // RISC-V images sometimes need more
            Architecture::Riscv64 => 512,
        }
    }

    /// Expected boot time in seconds (for timeout)
    pub fn expected_boot_seconds(&self, kvm_available: bool) -> u64 {
        match self {
            // KVM is near-native speed
            Architecture::X86_64 if kvm_available => 10,
            // TCG emulation is ~10-20x slower
            Architecture::X86_64 => 30,
            Architecture::Aarch64 => 60,
            Architecture::Riscv64 => 90,
        }
    }

    /// Directory name in cache
    pub fn cache_dir_name(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
            Architecture::Riscv64 => "riscv64",
        }
    }
}

/// Test image metadata
#[derive(Debug, Clone)]
pub struct TestImage {
    /// Human-readable name
    pub name: &'static str,
    /// Download URL
    pub url: &'static str,
    /// SHA256 checksum of downloaded file
    pub sha256: &'static str,
    /// Target architecture
    pub arch: Architecture,
    /// Approximate size in MB (for progress display)
    pub size_mb: u32,
    /// String to search for in serial output indicating successful boot
    pub success_marker: &'static str,
}

impl TestImage {
    /// Get path where this image should be cached
    pub fn cache_path(&self) -> PathBuf {
        let cache_dir = std::env::var("VM_IMAGE_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("embeddenator/vm-images")
            });

        cache_dir
            .join(self.arch.cache_dir_name())
            .join(self.filename())
    }

    /// Extract filename from URL
    fn filename(&self) -> &str {
        self.url.rsplit('/').next().unwrap_or("image.qcow2")
    }
}

/// Check if KVM is available on this system
pub fn kvm_available() -> bool {
    std::path::Path::new("/dev/kvm").exists()
        && std::fs::metadata("/dev/kvm")
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
}

/// Check if a QEMU binary is available
pub fn qemu_available(arch: Architecture) -> bool {
    std::process::Command::new("which")
        .arg(arch.qemu_binary())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_properties() {
        // Verify all architectures have valid configurations
        for arch in [
            Architecture::X86_64,
            Architecture::Aarch64,
            Architecture::Riscv64,
        ] {
            assert!(!arch.qemu_binary().is_empty());
            assert!(!arch.machine_type().is_empty());
            assert!(arch.memory_mb() >= 128);
            assert!(arch.expected_boot_seconds(false) > 0);
        }
    }

    #[test]
    fn test_kvm_detection() {
        // This will pass on Linux with KVM, fail elsewhere
        let kvm = kvm_available();
        println!("KVM available: {}", kvm);
        // Not asserting because this depends on the system
    }
}
