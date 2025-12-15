#!/usr/bin/env bash
# Documentation Generation Script for Embeddenator
# Generates comprehensive documentation using rustdoc

set -e

echo "🔨 Building documentation..."

# Clean previous docs
echo "Cleaning old documentation..."
cargo clean --doc

# Generate documentation with all features
echo "Generating rustdoc documentation..."
RUSTDOCFLAGS="--cfg docsrs" cargo doc \
    --no-deps \
    --document-private-items \
    --all-features

# Run doc tests
echo ""
echo "🧪 Running documentation tests..."
cargo test --doc

# Generate coverage information
echo ""
echo "📊 Documentation coverage:"
cargo doc --no-deps 2>&1 | grep -E "Documenting|warning" || true

echo ""
echo "✅ Documentation generated successfully!"
echo "📖 Open docs at: target/doc/embeddenator/index.html"
echo ""
echo "To view documentation locally:"
echo "  cargo doc --open"
echo ""
echo "To deploy to docs.rs:"
echo "  cargo publish (will automatically build docs)"
