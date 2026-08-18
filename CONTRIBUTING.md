# Contributing to embeddenator-fs

Thank you for your interest in contributing to EmbrFS! This document provides guidelines for contributing to the project.

## Code of Conduct

**Be Respectful:** Treat all contributors with respect. Harassment, discrimination, or unprofessional behavior will not be tolerated.

**Be Constructive:** Provide helpful feedback. Focus on the code, not the person.

**Be Professional:** Maintain professionalism in all interactions (issues, PRs, discussions).

## How to Contribute

### Reporting Bugs

**Before submitting:**
1. Search existing issues to avoid duplicates
2. Verify the bug exists in the latest version (main branch)
3. Collect relevant information (OS, Rust version, error messages)

**Bug Report Template:**
```markdown
**Description:**
Clear description of the bug.

**Steps to Reproduce:**
1. Step one
2. Step two
3. Step three

**Expected Behavior:**
What you expected to happen.

**Actual Behavior:**
What actually happened.

**Environment:**
- OS: [e.g., Ubuntu 24.04]
- Rust version: [e.g., 1.75.0]
- embeddenator-fs version: [e.g., 0.20.0-alpha.3]
- Features enabled: [e.g., fuse]

**Additional Context:**
Logs, screenshots, or other relevant information.
```

### Suggesting Features

**Feature Request Template:**
```markdown
**Problem Statement:**
What problem does this feature solve?

**Proposed Solution:**
How should the feature work?

**Alternatives Considered:**
What other approaches did you consider?

**Impact:**
Who benefits from this feature?

**Implementation Notes:**
Any technical considerations or constraints?
```

**Note:** EmbrFS is focused on holographic filesystem research. Features should align with this goal. General-purpose filesystem features may be out of scope.

### Pull Requests

**Before submitting:**
1. Discuss significant changes in an issue first
2. Ensure tests pass (`cargo test`)
3. Run linters (`cargo clippy`, `cargo fmt`)
4. Update documentation if needed
5. Add tests for new functionality

**PR Checklist:**
- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation updated (rustdoc, README, docs/)
- [ ] CHANGELOG.md updated (if user-facing change)
- [ ] Commit messages are clear and descriptive

**PR Template:**
```markdown
**Description:**
What does this PR do?

**Related Issue:**
Fixes #123 (if applicable)

**Changes:**
- Change 1
- Change 2
- Change 3

**Testing:**
How was this tested?

**Breaking Changes:**
Does this break existing APIs? If so, how?

**Documentation:**
What documentation was updated?
```

## Development Setup

### Prerequisites

**Required:**
- Rust 1.70+ (stable)
- Git
- Linux (for FUSE development)

**Optional:**
- libfuse3 (for FUSE feature)
- cargo-watch (for auto-rebuild)
- cargo-criterion (for benchmarking)

### Clone and Build

```bash
# Clone repository
git clone https://github.com/tzervas/embeddenator-fs
cd embeddenator-fs

# Build
cargo build

# Run tests
cargo test

# Build with FUSE support
cargo build --features fuse

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Development Workflow

**1. Create a Branch:**
```bash
git checkout -b feature/my-feature
# or
git checkout -b fix/bug-description
```

**2. Make Changes:**
- Write code
- Add tests
- Update documentation

**3. Test Locally:**
```bash
# Run all tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check documentation
cargo doc --open
```

**4. Commit:**
```bash
git add .
git commit -m "feat: add new feature"
# or
git commit -m "fix: resolve bug in correction layer"
```

**Commit Message Format:**
- `feat: description` - New feature
- `fix: description` - Bug fix
- `docs: description` - Documentation only
- `test: description` - Test changes
- `refactor: description` - Code refactoring
- `perf: description` - Performance improvement
- `chore: description` - Build/maintenance tasks

**5. Push and Create PR:**
```bash
git push origin feature/my-feature
# Open PR on GitHub
```

### Running FUSE Tests (Linux Only)

```bash
# Install FUSE dependencies
sudo apt-get install libfuse3-3 libfuse3-dev  # Debian/Ubuntu

# Run FUSE tests
cargo test --features fuse

# Test FUSE mounting (requires root or fuse group)
cargo run --features fuse --example fuse_mount
```

## Code Style

### Rust Style

**Follow standard Rust conventions:**
- Use `cargo fmt` for formatting (enforced in CI)
- Use `cargo clippy` for linting (enforced in CI)
- Prefer explicit types over `auto` where clarity helps
- Use meaningful variable names
- Avoid abbreviations (except standard ones like `fs`, `io`)

**Documentation:**
- Every public item must have rustdoc comments
- Include examples in rustdoc where helpful
- Explain "why" not just "what" in comments
- Keep comments up-to-date with code changes

**Example:**
```rust
/// Encodes a file into a holographic engram.
///
/// This function splits the file into chunks, encodes each chunk as a
/// SparseVec, and bundles them into the engram. A correction layer is
/// generated to ensure bit-perfect reconstruction.
///
/// # Arguments
///
/// * `path` - Path to the file to encode
/// * `options` - Encoding options (chunk size, dimensionality, etc.)
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if encoding fails.
///
/// # Examples
///
/// ```
/// use embeddenator_fs::{EmbrFS, EncodeOptions};
/// use std::path::Path;
///
/// let mut fs = EmbrFS::new();
/// let options = EncodeOptions::default();
/// fs.encode_file(Path::new("file.txt"), &options)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be read
/// - Encoding fails due to insufficient dimensionality
/// - Disk is full when writing corrections
pub fn encode_file(&mut self, path: &Path, options: &EncodeOptions) -> Result<()> {
    // Implementation
}
```

### Error Handling

**Use `Result` types:**
```rust
// Good
pub fn load(path: &Path) -> Result<EmbrFS, std::io::Error> { ... }

// Bad (avoid unwrap in library code)
pub fn load(path: &Path) -> EmbrFS {
    let data = std::fs::read(path).unwrap();  // DON'T DO THIS
    // ...
}
```

**Provide context in errors:**
```rust
// Good
std::fs::read(path)
    .map_err(|e| format!("Failed to read engram from {:?}: {}", path, e))?;

// Acceptable
std::fs::read(path)?;
```

**Use custom error types for complex errors:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum EngramError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    
    #[error("Hash mismatch for {path}: expected {expected:x}, got {actual:x}")]
    HashMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Testing

**Test Coverage Goals:**
- All public APIs should have tests
- Critical paths (encoding, decoding, correction) should have extensive tests
- Edge cases should be tested (empty files, large files, special characters)

**Test Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = "test data";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    #[should_panic(expected = "error message")]
    fn test_error_handling() {
        // Test that errors are properly raised
    }
}
```

**Property-Based Tests:**
```rust
#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            // Encode and decode should be lossless
            let encoded = encode(&data)?;
            let decoded = decode(&encoded)?;
            prop_assert_eq!(data, decoded);
        }
    }
}
```

**Integration Tests:**
```rust
// tests/integration_test.rs
use embeddenator_fs::{EmbrFS, IngestOptions};
use tempfile::tempdir;

#[test]
fn test_full_workflow() {
    let dir = tempdir().unwrap();
    // Create test files
    // Ingest
    // Extract
    // Verify
}
```

## Architecture Guidelines

### Module Organization

**Current Structure:**
```
src/
├── lib.rs           # Public API exports
├── fs/
│   ├── mod.rs       # Module exports
│   ├── embrfs.rs    # Core filesystem logic
│   ├── fuse_shim.rs # FUSE integration (feature-gated)
│   └── correction.rs# Bit-perfect reconstruction
```

**Guidelines:**
- Keep modules focused (single responsibility)
- Limit file size (target <2000 lines, refactor if larger)
- Use `mod.rs` for module exports only
- Feature-gate platform-specific code (`#[cfg(feature = "fuse")]`)

### Performance Considerations

**Optimization Strategy:**
1. Correctness first
2. Measure before optimizing (use `cargo bench`)
3. Optimize hot paths (encoding, decoding)
4. Document performance characteristics in rustdoc

**Common Patterns:**
- Use `Vec::with_capacity` when size is known
- Prefer `&[u8]` over `Vec<u8>` for read-only data
- Use `Arc` for shared read-only data
- Use `Cow` for copy-on-write semantics
- Cache expensive computations (LRU cache)

**Avoid:**
- Premature optimization
- Unsafe code (unless absolutely necessary and well-documented)
- Allocations in hot loops
- Excessive cloning

### Concurrency

**Current Model:**
- `Arc<RwLock<EmbrFS>>` for thread-safe access
- Read lock for queries (concurrent)
- Write lock for modifications (exclusive)

**Guidelines:**
- Minimize lock scope (acquire late, release early)
- Avoid holding locks across I/O operations
- Use lock-free structures where appropriate (atomic counters)
- Document thread-safety guarantees in rustdoc

## Documentation

### Rustdoc

**Required for:**
- All public modules
- All public types (structs, enums, traits)
- All public functions and methods
- All public constants and statics

**Optional but recommended:**
- Private functions (for maintainers)
- Complex algorithms (explain approach)
- Performance characteristics

**Include:**
- Summary (first line, <80 chars)
- Detailed description (if needed)
- Examples (where helpful)
- Error conditions
- Performance characteristics (if notable)

### External Documentation

**docs/ Directory:**
- `ARCHITECTURE.md` - System design and architecture
- `FUSE.md` - FUSE implementation guide
- `CORRECTION.md` - Bit-perfect reconstruction details (future)
- `BENCHMARKS.md` - Performance benchmarks (future)

**Update when:**
- Architecture changes significantly
- New features are added
- Performance characteristics change
- APIs change

### README.md

**Keep Updated:**
- Installation instructions
- Basic usage examples
- Feature list (accurate scope)
- Limitations (honest assessment)

**Avoid:**
- Overly ambitious claims
- Outdated examples
- Links to non-existent documentation

## Release Process

**Versioning:** Semantic Versioning (semver)
- `MAJOR.MINOR.PATCH[-PRERELEASE]`
- MAJOR: Breaking changes
- MINOR: New features (backwards compatible)
- PATCH: Bug fixes
- PRERELEASE: alpha.1, beta.1, etc.

**Release Checklist:**
1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with release notes
3. Run full test suite (`cargo test --all-features`)
4. Run clippy (`cargo clippy -- -D warnings`)
5. Build documentation (`cargo doc --all-features`)
6. Create git tag (`git tag -a v0.x.y -m "Release v0.x.y"`)
7. Push tag (`git push origin v0.x.y`)
8. Publish to crates.io (`cargo publish`)
9. Create GitHub release with changelog

**Pre-release Testing:**
- Test on clean machine
- Test with minimum supported Rust version (MSRV)
- Test on different platforms (Linux variants)
- Verify documentation renders correctly

## Communication

**Preferred Channels:**
- GitHub Issues - Bug reports, feature requests
- GitHub Discussions - General questions, ideas (if enabled)
- Pull Requests - Code contributions

**Response Times:**
- We aim to respond to issues within 1-3 days
- PRs reviewed within 1 week
- Urgent security issues within 24 hours

**Be Patient:**
- This is an open-source project maintained by volunteers
- Complex PRs may require multiple review iterations
- Breaking changes may be delayed to coordinate with other components

## License

By contributing to embeddenator-fs, you agree that your contributions will be licensed under the MIT License.

All new files should include:
```rust
// Copyright (c) 2024-2026 Tyler Zervas
// SPDX-License-Identifier: MIT
```

## Getting Help

**Stuck?**
- Check existing documentation (README, docs/, rustdoc)
- Search GitHub issues
- Ask in GitHub Discussions (if enabled)
- Review related projects (embeddenator-vsa, embeddenator-retrieval)

**Contact:**
- GitHub: [@tzervas](https://github.com/tzervas)
- Email: See GitHub profile

---

Thank you for contributing to EmbrFS! Your efforts help advance holographic filesystem research.
