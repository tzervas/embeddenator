#!/bin/bash
# Memory Profiling with Valgrind/Massif - Memory usage and leaks
# Usage: ./profile_memory.sh <operation> <size_mb>
# Example: ./profile_memory.sh ingest 100

set -e

OPERATION=${1:-ingest}
SIZE=${2:-100}
OUTPUT_DIR="profile_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$OUTPUT_DIR"

echo "================================"
echo "Memory Profiling (Valgrind)"
echo "================================"
echo "Operation: $OPERATION"
echo "Data Size: ${SIZE}MB"
echo "Output: $OUTPUT_DIR/$OPERATION-mem-$TIMESTAMP"
echo ""

# Generate test data if needed
if [ ! -f "test_data_${SIZE}mb.bin" ]; then
    echo "Generating ${SIZE}MB test dataset..."
    head -c $((SIZE * 1024 * 1024)) </dev/urandom > test_data_${SIZE}mb.bin
fi

# Build optimized binary
echo "Building optimized binary..."
cargo build --release --quiet 2>/dev/null || cargo build --release

echo "Running memory profiler (Valgrind Massif)..."
valgrind --tool=massif \
    --massif-out-file="$OUTPUT_DIR/$OPERATION-mem-$TIMESTAMP.massif" \
    --max-snapshots=100 \
    ./target/release/embeddenator $OPERATION \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.mem \
    -m out.json.mem \
    2>&1 | grep -E "(Massif|max RSS|elapsed)" || true

echo ""
echo "✓ Memory profiling complete!"
echo "Results saved to: $OUTPUT_DIR/$OPERATION-mem-$TIMESTAMP.massif"
echo ""
echo "To visualize:"
echo "  ms_print $OUTPUT_DIR/$OPERATION-mem-$TIMESTAMP.massif | less"
echo ""
echo "Summary:"
valgrind --tool=massif --massif-out-file=/tmp/tmp.massif \
    ./target/release/embeddenator $OPERATION \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.tmp \
    -m out.json.tmp \
    2>&1 | tail -20 && \
    ms_print /tmp/tmp.massif 2>/dev/null | tail -40 || true
