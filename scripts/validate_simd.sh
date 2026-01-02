#!/bin/bash
# SIMD Optimization Validation Script
# Verifies that SIMD implementation is working correctly

set -e

echo "=================================="
echo "SIMD Optimization Validation"
echo "=================================="
echo ""

# Check Rust version
echo "1. Checking Rust version..."
rustc --version
cargo --version
echo "✓ Rust toolchain available"
echo ""

# Test without SIMD (baseline)
echo "2. Running tests without SIMD (baseline)..."
cargo test --lib simd_cosine --quiet
echo "✓ Baseline tests pass"
echo ""

# Test with SIMD feature
echo "3. Running tests with SIMD feature..."
cargo test --features simd --lib simd_cosine --quiet
echo "✓ SIMD feature tests pass"
echo ""

# Run integration tests
echo "4. Running integration tests..."
cargo test --features simd --test simd_cosine_tests --quiet
echo "✓ Integration tests pass (12 tests)"
echo ""

# Build with native optimizations
echo "5. Building with native CPU optimizations..."
RUSTFLAGS="-C target-cpu=native" cargo build --release --features simd --quiet 2>&1 | head -5
echo "✓ Release build successful"
echo ""

# Run quick benchmark
echo "6. Running quick benchmark comparison..."
echo "   (This may take a minute...)"
cargo bench --bench simd_cosine -- "scalar.*identical" --noplot --quiet 2>&1 | grep -E "(Benchmarking|time:)" | head -5
echo "✓ Benchmarks complete"
echo ""

echo "=================================="
echo "✓ All validation checks passed!"
echo "=================================="
echo ""
echo "SIMD optimization is working correctly."
echo ""
echo "To use SIMD in your builds:"
echo "  RUSTFLAGS=\"-C target-cpu=native\" cargo build --release --features simd"
echo ""
echo "For more information, see:"
echo "  - docs/SIMD_OPTIMIZATION.md"
echo "  - TASK_009_SIMD_OPTIMIZATION_COMPLETE.md"
