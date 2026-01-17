#!/bin/bash
# CPU Profiling with perf - Hotspot identification
# Usage: ./profile_cpu_perf.sh <operation> <size_mb>
# Example: ./profile_cpu_perf.sh ingest 500

set -e

OPERATION=${1:-ingest}
SIZE=${2:-100}
OUTPUT_DIR="profile_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$OUTPUT_DIR"

echo "================================"
echo "CPU Performance Profiling"
echo "================================"
echo "Operation: $OPERATION"
echo "Data Size: ${SIZE}MB"
echo "Output: $OUTPUT_DIR/$OPERATION-$TIMESTAMP"
echo ""

# Generate test data if needed
if [ ! -f "test_data_${SIZE}mb.bin" ]; then
    echo "Generating ${SIZE}MB test dataset..."
    head -c $((SIZE * 1024 * 1024)) </dev/urandom > test_data_${SIZE}mb.bin
fi

# Build optimized binary
echo "Building optimized binary..."
cargo build --release --quiet 2>/dev/null || cargo build --release

echo "Recording perf data (30 second sample)..."
timeout 30 perf record -F 99 \
    -o "$OUTPUT_DIR/$OPERATION-$TIMESTAMP.perf" \
    ./target/release/embeddenator $OPERATION \
    -i test_data_${SIZE}mb.bin \
    -e out.engram \
    -m out.json \
    2>&1 || true

echo ""
echo "Generating reports..."

# Report 1: Top functions
echo "=== TOP 20 HOTSPOTS ===" | tee "$OUTPUT_DIR/$OPERATION-$TIMESTAMP-report.txt"
perf report -i "$OUTPUT_DIR/$OPERATION-$TIMESTAMP.perf" --stdio 2>/dev/null | \
    grep -A 30 "^#" | tail -25 | tee -a "$OUTPUT_DIR/$OPERATION-$TIMESTAMP-report.txt"

# Report 2: Annotated source (if symbols available)
echo "" | tee -a "$OUTPUT_DIR/$OPERATION-$TIMESTAMP-report.txt"
echo "=== CACHE STATS ===" | tee -a "$OUTPUT_DIR/$OPERATION-$TIMESTAMP-report.txt"
perf stat -e cycles,instructions,cache-references,cache-misses,branches,branch-misses \
    ./target/release/embeddenator $OPERATION \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.1 \
    -m out.json.1 \
    2>&1 | tee -a "$OUTPUT_DIR/$OPERATION-$TIMESTAMP-report.txt"

echo ""
echo "✓ Profiling complete!"
echo "Results saved to: $OUTPUT_DIR/$OPERATION-$TIMESTAMP*"
echo ""
echo "To view detailed report:"
echo "  perf report -i $OUTPUT_DIR/$OPERATION-$TIMESTAMP.perf"
echo ""
echo "To generate flamegraph:"
echo "  perf script -i $OUTPUT_DIR/$OPERATION-$TIMESTAMP.perf | stackcollapse-perf.pl | flamegraph.pl > $OUTPUT_DIR/$OPERATION-$TIMESTAMP.svg"
