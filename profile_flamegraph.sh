#!/bin/bash
# Flamegraph Generation - Visual profiling with call stacks
# Usage: ./profile_flamegraph.sh <operation> <size_mb>
# Example: ./profile_flamegraph.sh ingest 500

set -e

OPERATION=${1:-ingest}
SIZE=${2:-100}
OUTPUT_DIR="profile_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$OUTPUT_DIR"

echo "================================"
echo "Flamegraph Profiling"
echo "================================"
echo "Operation: $OPERATION"
echo "Data Size: ${SIZE}MB"
echo "Output: $OUTPUT_DIR/$OPERATION-$TIMESTAMP.svg"
echo ""

# Generate test data if needed
if [ ! -f "test_data_${SIZE}mb.bin" ]; then
    echo "Generating ${SIZE}MB test dataset..."
    head -c $((SIZE * 1024 * 1024)) </dev/urandom > test_data_${SIZE}mb.bin
fi

# Build optimized binary with debug symbols
echo "Building optimized binary with debug symbols..."
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release --quiet 2>/dev/null || \
    CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release

echo "Recording flamegraph data..."
cargo flamegraph \
    --bin embeddenator \
    -o "$OUTPUT_DIR/$OPERATION-$TIMESTAMP.svg" \
    --freq 99 \
    -- $OPERATION \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.fg \
    -m out.json.fg

echo ""
echo "✓ Flamegraph generated!"
echo "View with: open $OUTPUT_DIR/$OPERATION-$TIMESTAMP.svg"
echo "           # or: firefox $OUTPUT_DIR/$OPERATION-$TIMESTAMP.svg"
echo ""
echo "Flamegraph shows:"
echo "  - Call stack depth (height)"
echo "  - Time spent (width)"
echo "  - Click on blocks to zoom in/out"
