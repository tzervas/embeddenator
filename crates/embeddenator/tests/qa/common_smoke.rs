// This file intentionally left minimal.
//
// Historical note: integration tests sometimes use a `common/` module directory.
// If this file exists, declaring `mod common;` would cause an ambiguity between
// `tests/common.rs` and `tests/common/mod.rs`.

#[test]
fn common_smoke() {
	// No-op: ensures this test crate compiles.
}
