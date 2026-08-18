//! VM Boot Test Harness Entry Point
//!
//! This file serves as the entry point for the VM boot test suite.
//! It re-exports the test modules so they can be run with:
//!
//! ```bash
//! cargo test --test vm_boot_tests -- --ignored --nocapture
//! ```
//!
//! All tests in this harness are marked `#[ignore]` because they:
//! - Require QEMU to be installed
//! - Download large VM images (50-500MB)
//! - Take 30-120 seconds to run (emulated architectures)
//! - Should only run locally, never in CI

// Re-export the vm_boot module
mod vm_boot;

// Re-export tests so cargo test can find them
pub use vm_boot::*;
