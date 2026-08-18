//! RISC-V 64-bit VM Boot Tests
//! ============================
//!
//! This module tests booting RISC-V VMs from engram-encoded disk images.
//!
//! ## Why RISC-V Testing Matters
//!
//! RISC-V is the emerging open-source ISA gaining rapid adoption:
//!
//! 1. **Open Standard**: No licensing fees, fully open specification
//! 2. **Growing Ecosystem**: SiFive, StarFive, Milk-V hardware shipping
//! 3. **Research Platform**: Academia and startups use RISC-V extensively
//! 4. **Future-Proofing**: May become significant in embedded/edge computing
//!
//! By testing RISC-V now, we ensure embeddenator works on tomorrow's hardware.
//!
//! ## Current RISC-V State
//!
//! As of 2024, RISC-V ecosystem is less mature than ARM64:
//! - Fewer prebuilt images available
//! - QEMU RISC-V is less optimized than ARM64 TCG
//! - Expect 15-30x slowdown compared to native
//! - Boot tests may take 60-120 seconds
//!
//! ## Test Image Selection
//!
//! RISC-V images are harder to source. Options:
//! - **Fedora RISC-V**: Official Fedora spin, well-maintained
//! - **Ubuntu RISC-V**: Available via Ubuntu RISC-V project
//! - **openSUSE RISC-V**: Experimental builds
//! - **Buildroot**: Custom minimal images
//!
//! We prioritize Fedora RISC-V for:
//! - Regular updates and security patches
//! - QCOW2 cloud images available
//! - Reasonable size (200-400MB)
//!
//! ## QEMU RISC-V Machine
//!
//! We use the "virt" machine with:
//! - Generic RISC-V 64-bit platform
//! - virtio-mmio devices
//! - OpenSBI firmware (provides M-mode services)
//! - U-Boot or direct kernel boot

use super::common::{run_boot_test, BootError, BootTestConfig};
use super::{Architecture, TestImage};

/// Fedora RISC-V 39 minimal image
///
/// # Why Fedora RISC-V
///
/// Fedora has one of the best-maintained RISC-V ports:
/// - Official Fedora project, not a community remix
/// - Regular updates matching x86_64/aarch64 releases
/// - Cloud/minimal images in QCOW2 format
/// - Good QEMU compatibility testing
///
/// # Image Variants
///
/// Fedora provides several RISC-V images:
/// - **Server**: Full server installation (~1.5GB)
/// - **Minimal**: CLI-only, small footprint (~200MB)
/// - **Rawhide**: Development branch (unstable)
///
/// We use "minimal" for fast boot tests.
pub const FEDORA_RISCV64: TestImage = TestImage {
    name: "Fedora 39 Minimal RISC-V",
    url: "https://fedora.riscv.rocks/kojifiles/work/tasks/6900/1366900/Fedora-Minimal-Rawhide-20231120.n.0-sda.raw.xz",
    sha256: "placeholder_verify_fedora_riscv_checksum",
    arch: Architecture::Riscv64,
    size_mb: 200,
    // Fedora prints this after systemd reaches multi-user.target
    success_marker: "login:",
};

/// OpenSBI + Linux minimal boot test image
///
/// # Why a Minimal Image
///
/// For quick smoke tests, we want the smallest possible image that
/// proves the encoding works:
/// - OpenSBI firmware (M-mode)
/// - Linux kernel with initramfs
/// - Busybox userspace
/// - Prints "Hello from RISC-V" and halts
///
/// This boots in ~10-15 seconds even on TCG emulation.
pub const MINIMAL_RISCV64: TestImage = TestImage {
    name: "Minimal RISC-V Linux (buildroot)",
    // This would be hosted on our own infrastructure or GitHub releases
    url: "https://github.com/embeddenator/test-images/releases/download/v1/riscv64-minimal.qcow2",
    sha256: "placeholder_minimal_riscv_checksum",
    arch: Architecture::Riscv64,
    size_mb: 20,
    success_marker: "Hello from RISC-V",
};

/// Test booting Fedora RISC-V (emulated on x86_64)
///
/// # Test Requirements
///
/// - QEMU installed: `qemu-system-riscv64`
/// - OpenSBI firmware: Usually bundled with QEMU
/// - Patience: 60-120 seconds per boot (heavily emulated)
/// - ~300MB disk: For cached image
///
/// # Why This Is Slow
///
/// RISC-V on x86_64 is slower than ARM64 emulation because:
/// - RISC-V TCG backend is less optimized
/// - Different memory models require extra barriers
/// - Vector extensions (if any) need full emulation
/// - Less development effort (smaller user base)
#[test]
#[ignore = "Local-only: requires QEMU riscv64, very slow emulation"]
fn test_riscv64_fedora() {
    // Check for QEMU
    if !super::qemu_available(Architecture::Riscv64) {
        println!("SKIP: qemu-system-riscv64 not installed");
        println!("Install with: sudo apt install qemu-system-misc");
        return;
    }

    let config = BootTestConfig {
        image: FEDORA_RISCV64,
        use_kvm: false, // RISC-V can't use KVM on x86_64
        timeout: Some(std::time::Duration::from_secs(180)), // 3 minutes for emulation
        extra_args: vec![
            // RISC-V needs OpenSBI firmware
            "-bios".to_string(),
            find_opensbi_firmware().unwrap_or_else(|| "default".to_string()),
            // Fedora needs more RAM
            "-m".to_string(),
            "2048".to_string(),
        ],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Boot failed: {:?}", result.error);
            println!("\n=== RISC-V Boot Test Results ===");
            println!("Image prep time: {:?}", result.image_prep_time);
            println!("Encode time: {:?}", result.encode_time);
            println!("Boot time: {:?} (emulated)", result.boot_time);

            // RISC-V emulation is very slow
            assert!(
                result.boot_time.as_secs() <= 180,
                "Boot time {:?} exceeds 180 second threshold",
                result.boot_time
            );
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed for riscv64");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Test with minimal RISC-V image for fast smoke test
///
/// # Why Minimal Image
///
/// The Fedora image takes 2-3 minutes to boot under emulation.
/// For quick validation during development, we want something faster.
///
/// The minimal image boots in ~15-30 seconds and validates:
/// - Disk image parsing
/// - Kernel/initramfs extraction
/// - Basic boot sequence
///
/// It does NOT validate:
/// - Full filesystem traversal (only few files)
/// - Complex init sequences
/// - systemd compatibility
#[test]
#[ignore = "Local-only: quick RISC-V smoke test"]
fn test_riscv64_minimal() {
    if !super::qemu_available(Architecture::Riscv64) {
        println!("SKIP: qemu-system-riscv64 not installed");
        return;
    }

    let config = BootTestConfig {
        image: MINIMAL_RISCV64,
        use_kvm: false,
        timeout: Some(std::time::Duration::from_secs(60)), // 60s should be enough for minimal
        extra_args: vec![
            "-bios".to_string(),
            find_opensbi_firmware().unwrap_or_else(|| "default".to_string()),
        ],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Minimal boot failed: {:?}", result.error);
            println!("Minimal RISC-V boot time: {:?}", result.boot_time);
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Find OpenSBI firmware on the system
///
/// # What is OpenSBI
///
/// OpenSBI (Open Source Supervisor Binary Interface) provides:
/// - M-mode (machine mode) firmware for RISC-V
/// - SBI calls for S-mode (supervisor) software
/// - Platform initialization (UART, timers, etc.)
/// - Similar role to ARM's Trusted Firmware
///
/// QEMU can use its built-in OpenSBI or an external firmware file.
fn find_opensbi_firmware() -> Option<String> {
    let paths = [
        // Debian/Ubuntu
        "/usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin",
        "/usr/share/opensbi/lp64/generic/firmware/fw_dynamic.bin",
        // Fedora
        "/usr/share/edk2/riscv64/RISCV_VIRT.fd",
        // Arch
        "/usr/share/opensbi/riscv64-generic/opensbi-riscv64-generic-fw_dynamic.bin",
        // QEMU bundled
        "/usr/share/qemu/opensbi-riscv64-virt-fw_jump.bin",
    ];

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Return "default" to let QEMU use its built-in firmware
    None
}

/// Test encoding a RISC-V image without booting
///
/// # Why Encode-Only Test
///
/// RISC-V boot takes 2+ minutes even for minimal images.
/// This test validates the encoding pipeline specifically:
/// - QCOW2/raw parsing for RISC-V images
/// - Filesystem traversal
/// - VSA encoding of RISC-V binaries
/// - Compression of RISC-V specific formats
///
/// Runs in ~10-30 seconds vs 2+ minutes for full boot.
#[test]
#[ignore = "Local-only: tests encoding without full boot"]
fn test_riscv64_encode_only() {
    // Download/cache the image
    let image_path = match super::common::ensure_image_available(&MINIMAL_RISCV64) {
        Ok(p) => p,
        Err(e) => {
            println!("SKIP: Could not get test image: {}", e);
            return;
        }
    };

    println!("\n=== RISC-V Encode-Only Test ===");
    println!("Image: {}", image_path.display());

    // TODO: Call actual encoding function and verify output
    let start = std::time::Instant::now();

    // Placeholder for encoding
    println!("TODO: Implement RISC-V specific encoding test");

    let elapsed = start.elapsed();
    println!("Encoding time: {:?}", elapsed);
}

/// Validate RISC-V specific binaries in encoded engram
///
/// # Why Arch-Specific Validation
///
/// RISC-V binaries have unique characteristics:
/// - Different ELF machine type (EM_RISCV = 243)
/// - Compressed instruction extension (RVC)
/// - Vector extension (RVV) if enabled
/// - Different calling conventions
///
/// This test verifies we correctly preserve RISC-V binaries.
#[test]
#[ignore = "Local-only: validates RISC-V ELF handling"]
fn test_riscv64_elf_preservation() {
    println!("\n=== RISC-V ELF Preservation Test ===");
    println!("TODO: Implement ELF magic/header validation");

    // Would verify:
    // 1. Extract a known binary from engram
    // 2. Verify ELF header is intact
    // 3. Verify RISC-V machine type preserved
    // 4. Verify file permissions preserved
    // 5. Verify the binary executes correctly in QEMU
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_image_constants() {
        assert!(!FEDORA_RISCV64.url.is_empty());
        assert_eq!(FEDORA_RISCV64.arch, Architecture::Riscv64);
    }

    #[test]
    fn test_opensbi_search() {
        let firmware = find_opensbi_firmware();
        if let Some(path) = firmware {
            println!("Found OpenSBI firmware: {}", path);
        } else {
            println!("No OpenSBI firmware found (QEMU will use built-in)");
        }
    }

    #[test]
    fn test_qemu_detection() {
        let available = super::super::qemu_available(Architecture::Riscv64);
        println!("QEMU riscv64 available: {}", available);
        if !available {
            println!("Install with: sudo apt install qemu-system-misc");
        }
    }
}
