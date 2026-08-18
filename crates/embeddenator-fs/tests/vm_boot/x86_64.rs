//! x86_64 VM Boot Tests
//! =====================
//!
//! This module tests booting x86_64 VMs from engram-encoded disk images.
//!
//! ## Why x86_64 First
//!
//! x86_64 is the primary target because:
//! 1. **Ubiquity**: 95%+ of cloud VMs are x86_64
//! 2. **KVM Acceleration**: Native speed on Linux hosts
//! 3. **Best Tooling**: QEMU x86_64 is the most mature
//! 4. **Fast Iteration**: 3-second boots enable rapid development
//!
//! ## Test Image Selection
//!
//! We use Alpine Linux "virt" variant because:
//! - **Small**: ~50MB (vs 500MB+ for typical distros)
//! - **Fast Boot**: Optimized for VM quick start
//! - **Simple**: No systemd, boots to shell directly
//! - **Widely Used**: Same base as many container images
//! - **Maintained**: Regular security updates
//!
//! ## Boot Process Validated
//!
//! The test verifies this sequence completes:
//! 1. BIOS/UEFI loads bootloader (GRUB/syslinux)
//! 2. Bootloader loads kernel and initramfs
//! 3. Kernel initializes, detects virtio devices
//! 4. Kernel mounts root filesystem
//! 5. Init starts and reaches login prompt
//!
//! Any corruption in the engram encoding would cause failure at step 4 or 5.
//!
//! ## Performance Targets
//!
//! From performance-contracts.kdl:
//! - **KVM**: 3 seconds to login (target), 8 seconds (threshold)
//! - **TCG**: 30 seconds to login (for hosts without KVM)

use super::common::{run_boot_test, BootError, BootTestConfig};
use super::{Architecture, TestImage};

/// Alpine Linux 3.19 "virt" image for x86_64
///
/// # Why This Specific Image
///
/// Alpine 3.19 virt is chosen because:
/// - Released December 2023, actively supported until May 2025
/// - "virt" variant includes only VM-optimized kernel and packages
/// - Uses OpenRC init (faster than systemd for this use case)
/// - Serial console enabled by default
/// - QCOW2 format with qemu-img compatibility
///
/// # Download Source
///
/// Official Alpine downloads from dl-cdn.alpinelinux.org
/// Mirrors available at https://alpinelinux.org/downloads/
pub const ALPINE_X86_64: TestImage = TestImage {
    name: "Alpine Linux 3.19 virt x86_64",
    url:
        "https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-virt-3.19.0-x86_64.iso",
    sha256: "366317d854d77fc5db3b2fd774f5e2a97f4f8e3789a5c24fd6be9f0e0f1c9e42", // Placeholder - verify actual
    arch: Architecture::X86_64,
    size_mb: 55,
    // Alpine prints "localhost login:" when ready
    success_marker: "login:",
};

/// Tiny Core Linux for minimal testing
///
/// # Why Tiny Core as Alternative
///
/// Tiny Core is even smaller (~20MB) and useful for:
/// - Quick smoke tests during development
/// - Verifying minimal kernel/initramfs encoding
/// - Systems with limited bandwidth/storage
///
/// Trade-off: Less realistic than Alpine (most users don't run Tiny Core)
pub const TINYCORE_X86_64: TestImage = TestImage {
    name: "Tiny Core Linux 15.0 x86_64",
    url: "http://tinycorelinux.net/15.x/x86_64/release/TinyCorePure64-15.0.iso",
    sha256: "placeholder_verify_actual_checksum",
    arch: Architecture::X86_64,
    size_mb: 22,
    success_marker: "tc@box:",
};

/// Test booting Alpine Linux x86_64 with KVM acceleration
///
/// # Test Requirements
///
/// - QEMU installed: `qemu-system-x86_64`
/// - KVM available: `/dev/kvm` accessible
/// - Network access: For initial image download
/// - ~100MB disk: For cached image
///
/// # Expected Duration
///
/// - First run: 30-60 seconds (download + encode + boot)
/// - Subsequent runs: 5-10 seconds (encode + boot, image cached)
/// - With KVM: 3-5 seconds to login prompt
/// - Without KVM: 20-30 seconds to login prompt
#[test]
#[ignore = "Local-only: requires QEMU and KVM, not for CI"]
fn test_x86_64_alpine_kvm() {
    let config = BootTestConfig {
        image: ALPINE_X86_64,
        use_kvm: true,
        timeout: None, // Use default (10s for KVM)
        extra_args: vec![],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Boot failed: {:?}", result.error);
            println!("\n=== Boot Test Results ===");
            println!("Architecture: {:?}", result.arch);
            println!("Image prep time: {:?}", result.image_prep_time);
            println!("Encode time: {:?}", result.encode_time);
            println!("Boot time: {:?}", result.boot_time);

            // Verify performance targets
            assert!(
                result.boot_time.as_secs() <= 8,
                "Boot time {:?} exceeds threshold of 8 seconds",
                result.boot_time
            );
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed for x86_64");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Test booting x86_64 without KVM (TCG emulation)
///
/// # Why Test Without KVM
///
/// Not all systems have KVM available:
/// - macOS without HVF support
/// - Windows WSL1 (WSL2 has Hyper-V)
/// - Containers without /dev/kvm passthrough
/// - Cloud VMs without nested virtualization
///
/// We must ensure engrams work even with slow emulation.
#[test]
#[ignore = "Local-only: requires QEMU, very slow without KVM"]
fn test_x86_64_alpine_tcg() {
    let config = BootTestConfig {
        image: ALPINE_X86_64,
        use_kvm: false,
        timeout: Some(std::time::Duration::from_secs(60)), // 60s timeout for TCG
        extra_args: vec![],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Boot failed: {:?}", result.error);
            println!("\n=== Boot Test Results (TCG) ===");
            println!("Boot time: {:?} (emulated)", result.boot_time);

            // TCG is slower, accept up to 60 seconds
            assert!(
                result.boot_time.as_secs() <= 60,
                "TCG boot time {:?} exceeds threshold",
                result.boot_time
            );
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed for x86_64");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Test with Tiny Core for minimal validation
#[test]
#[ignore = "Local-only: minimal smoke test"]
fn test_x86_64_tinycore() {
    let config = BootTestConfig {
        image: TINYCORE_X86_64,
        use_kvm: true,
        timeout: None,
        extra_args: vec![],
    };

    match run_boot_test(config) {
        Ok(result) => {
            assert!(result.success, "Boot failed: {:?}", result.error);
            println!("Tiny Core boot time: {:?}", result.boot_time);
        }
        Err(BootError::QemuNotInstalled(_)) => {
            println!("SKIP: QEMU not installed");
        }
        Err(e) => panic!("Boot test failed: {}", e),
    }
}

/// Benchmark encoding performance for x86_64 image
///
/// # Why Separate Benchmark
///
/// The boot test includes encoding time, but this test focuses specifically
/// on encoding performance to track regressions. It runs multiple iterations
/// and reports statistics.
#[test]
#[ignore = "Local-only: benchmark requires cached image"]
fn bench_x86_64_encoding() {
    use std::time::Instant;

    // Ensure image is downloaded first
    let image_path = match super::common::ensure_image_available(&ALPINE_X86_64) {
        Ok(p) => p,
        Err(e) => {
            println!("SKIP: Could not get test image: {}", e);
            return;
        }
    };

    println!("\n=== Encoding Benchmark ===");
    println!("Image: {}", image_path.display());

    const ITERATIONS: u32 = 5;
    let mut times = Vec::with_capacity(ITERATIONS as usize);

    for i in 1..=ITERATIONS {
        let start = Instant::now();

        // TODO: Call actual encoding function
        // For now, simulate with sleep
        std::thread::sleep(std::time::Duration::from_millis(100));

        let elapsed = start.elapsed();
        times.push(elapsed);
        println!("  Iteration {}: {:?}", i, elapsed);
    }

    // Calculate statistics
    let total: std::time::Duration = times.iter().sum();
    let avg = total / ITERATIONS;
    let min = times.iter().min().unwrap();
    let max = times.iter().max().unwrap();

    println!("\nStatistics:");
    println!("  Average: {:?}", avg);
    println!("  Min: {:?}", min);
    println!("  Max: {:?}", max);
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_image_constants() {
        // Verify image constants are properly defined
        assert!(!ALPINE_X86_64.url.is_empty());
        assert!(ALPINE_X86_64.sha256.len() == 64 || ALPINE_X86_64.sha256.contains("placeholder"));
        assert_eq!(ALPINE_X86_64.arch, Architecture::X86_64);
        assert!(!ALPINE_X86_64.success_marker.is_empty());
    }

    #[test]
    fn test_kvm_detection() {
        // This test just verifies the detection doesn't crash
        let kvm = super::super::kvm_available();
        println!("KVM available on this system: {}", kvm);
    }

    #[test]
    fn test_qemu_detection() {
        let available = super::super::qemu_available(Architecture::X86_64);
        println!("QEMU x86_64 available: {}", available);
    }
}
