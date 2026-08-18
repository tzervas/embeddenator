//! aarch64 (ARM64) VM Boot Tests
//! ==============================
//!
//! This module tests booting ARM64 VMs from engram-encoded disk images.
//!
//! ## Why ARM64 Testing Matters
//!
//! ARM64 is increasingly important because:
//! 1. **AWS Graviton**: Cost-effective ARM instances (40% cheaper)
//! 2. **Apple Silicon**: M1/M2/M3 Macs are ARM64
//! 3. **Edge/IoT**: Raspberry Pi, NVIDIA Jetson, etc.
//! 4. **Mobile**: Android devices for containerized apps
//!
//! Many users will encode x86_64 VMs for deployment on ARM64 (cross-arch),
//! or encode ARM64 VMs for testing on x86_64 hosts.
//!
//! ## Emulation Overhead
//!
//! When running ARM64 on x86_64 hosts (most common case):
//! - QEMU uses TCG (Tiny Code Generator) for translation
//! - Expect 10-20x slowdown compared to native
//! - Boot tests take 30-60 seconds instead of 3 seconds
//! - This is acceptable for testing; production runs native
//!
//! ## Test Image Selection
//!
//! We use Alpine Linux ARM64 because:
//! - Same minimal footprint as x86_64 (~50MB)
//! - Identical boot sequence to validate consistency
//! - Official ARM64 builds available
//! - Well-tested on QEMU virt machine
//!
//! ## QEMU ARM64 Machine
//!
//! We use the "virt" machine type which provides:
//! - Generic ARM64 platform (not tied to specific hardware)
//! - virtio devices for storage, network, etc.
//! - GICv2/v3 interrupt controller
//! - UEFI boot via AAVMF firmware

use super::common::{run_boot_test, BootError, BootTestConfig};
use super::{Architecture, TestImage};

/// Alpine Linux 3.19 "virt" image for aarch64
///
/// # Why Alpine aarch64
///
/// Same rationale as x86_64: minimal, fast, reliable.
/// The aarch64 variant uses the same build system and packages,
/// ensuring consistent behavior across architectures.
///
/// # UEFI Boot
///
/// ARM64 systems typically use UEFI boot. QEMU requires:
/// - AAVMF firmware (from edk2-aarch64)
/// - EFI partition in the disk image
/// - GRUB EFI or systemd-boot
///
/// Alpine virt images include EFI boot support.
pub const ALPINE_AARCH64: TestImage = TestImage {
    name: "Alpine Linux 3.19 virt aarch64",
    url: "https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/aarch64/alpine-virt-3.19.0-aarch64.iso",
    sha256: "placeholder_verify_actual_aarch64_checksum",
    arch: Architecture::Aarch64,
    size_mb: 55,
    success_marker: "login:",
};

/// Debian ARM64 cloud image for compatibility testing
///
/// # Why Debian as Alternative
///
/// While Alpine is minimal, Debian tests a more "realistic" scenario:
/// - systemd init (most enterprise distros use this)
/// - APT package manager
/// - More services at boot
/// - Larger filesystem to encode
///
/// This catches issues that only appear with complex boot sequences.
pub const DEBIAN_AARCH64: TestImage = TestImage {
    name: "Debian 12 cloud aarch64",
    url: "https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-arm64.qcow2",
    sha256: "placeholder_verify_debian_aarch64_checksum",
    arch: Architecture::Aarch64,
    size_mb: 350,
    success_marker: "login:",
};

/// Test booting Alpine Linux aarch64 (emulated on x86_64)
///
/// # Test Requirements
///
/// - QEMU installed: `qemu-system-aarch64`
/// - EFI firmware: `/usr/share/qemu/edk2-aarch64-code.fd` or similar
/// - Network access: For initial image download
/// - Patience: 30-60 seconds per boot (emulated)
///
/// # Why No KVM
///
/// KVM only works for the host's native architecture. On x86_64 hosts,
/// ARM64 VMs must use TCG emulation. On ARM64 hosts (M1 Mac, Graviton),
/// we could use KVM/HVF, but we don't detect this automatically yet.
#[test]
#[ignore = "Local-only: requires QEMU aarch64, slow emulation"]
fn test_aarch64_alpine() {
    // Check for QEMU
    if !super::qemu_available(Architecture::Aarch64) {
        println!("SKIP: qemu-system-aarch64 not installed");
        println!("Install with: sudo apt install qemu-system-arm");
        return;
    }

    let config = BootTestConfig {
        image: ALPINE_AARCH64,
        use_kvm: false, // ARM64 can't use KVM on x86_64 host
        timeout: Some(std::time::Duration::from_secs(90)), // 90s for emulation
        extra_args: vec![
            // ARM64 requires UEFI firmware
            "-bios".to_string(),
            find_aarch64_firmware()
                .unwrap_or_else(|| "/usr/share/qemu/edk2-aarch64-code.fd".to_string()),
        ],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Boot failed: {:?}", result.error);
            println!("\n=== aarch64 Boot Test Results ===");
            println!("Image prep time: {:?}", result.image_prep_time);
            println!("Encode time: {:?}", result.encode_time);
            println!("Boot time: {:?} (emulated)", result.boot_time);

            // Emulation is slow, allow up to 60 seconds
            assert!(
                result.boot_time.as_secs() <= 90,
                "Boot time {:?} exceeds 90 second threshold",
                result.boot_time
            );
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed for aarch64");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Test booting Debian aarch64 for systemd compatibility
#[test]
#[ignore = "Local-only: larger image, longer boot"]
fn test_aarch64_debian() {
    if !super::qemu_available(Architecture::Aarch64) {
        println!("SKIP: qemu-system-aarch64 not installed");
        return;
    }

    let config = BootTestConfig {
        image: DEBIAN_AARCH64,
        use_kvm: false,
        timeout: Some(std::time::Duration::from_secs(180)), // Debian boots slower
        extra_args: vec![
            "-bios".to_string(),
            find_aarch64_firmware()
                .unwrap_or_else(|| "/usr/share/qemu/edk2-aarch64-code.fd".to_string()),
            // Debian needs more RAM
            "-m".to_string(),
            "1024".to_string(),
        ],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Debian boot failed: {:?}", result.error);
            println!("Debian aarch64 boot time: {:?}", result.boot_time);
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Find AARCH64 UEFI firmware on the system
///
/// # Why This Function
///
/// UEFI firmware location varies by distro:
/// - Debian/Ubuntu: `/usr/share/qemu/edk2-aarch64-code.fd`
/// - Fedora: `/usr/share/edk2/aarch64/QEMU_EFI.fd`
/// - Arch: `/usr/share/ovmf/aarch64/QEMU_EFI.fd`
/// - macOS (Homebrew): `/opt/homebrew/share/qemu/edk2-aarch64-code.fd`
///
/// We search common paths and return the first that exists.
fn find_aarch64_firmware() -> Option<String> {
    let paths = [
        // Debian/Ubuntu
        "/usr/share/qemu/edk2-aarch64-code.fd",
        "/usr/share/AAVMF/AAVMF_CODE.fd",
        // Fedora
        "/usr/share/edk2/aarch64/QEMU_EFI.fd",
        // Arch
        "/usr/share/ovmf/aarch64/QEMU_EFI.fd",
        "/usr/share/edk2-ovmf/aarch64/QEMU_EFI.fd",
        // macOS Homebrew
        "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
        "/usr/local/share/qemu/edk2-aarch64-code.fd",
    ];

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}

/// Compare encoding performance between architectures
///
/// # Why Compare Architectures
///
/// The same source image might have different encoding characteristics
/// based on:
/// - Compiler differences (GCC x86 vs GCC ARM)
/// - Library implementations
/// - Alignment/padding in binaries
/// - Kernel image format differences
///
/// This test tracks whether encoding performance varies by target arch.
#[test]
#[ignore = "Local-only: cross-architecture benchmark"]
fn bench_aarch64_vs_x86_64_encoding() {
    use super::x86_64::ALPINE_X86_64;

    // This would compare encoding times for both architectures
    // Currently just a placeholder for the benchmark infrastructure

    println!("\n=== Cross-Architecture Encoding Comparison ===");
    println!(
        "x86_64 image: {} ({} MB)",
        ALPINE_X86_64.name, ALPINE_X86_64.size_mb
    );
    println!(
        "aarch64 image: {} ({} MB)",
        ALPINE_AARCH64.name, ALPINE_AARCH64.size_mb
    );
    println!("\nTODO: Implement actual encoding comparison");
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_image_constants() {
        assert!(!ALPINE_AARCH64.url.is_empty());
        assert_eq!(ALPINE_AARCH64.arch, Architecture::Aarch64);
    }

    #[test]
    fn test_firmware_search() {
        // Just verify the function doesn't panic
        let firmware = find_aarch64_firmware();
        if let Some(path) = firmware {
            println!("Found aarch64 UEFI firmware: {}", path);
        } else {
            println!("No aarch64 UEFI firmware found (install qemu-efi-aarch64)");
        }
    }

    #[test]
    fn test_qemu_detection() {
        let available = super::super::qemu_available(Architecture::Aarch64);
        println!("QEMU aarch64 available: {}", available);
        if !available {
            println!("Install with: sudo apt install qemu-system-arm");
        }
    }
}
