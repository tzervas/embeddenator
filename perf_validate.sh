#!/bin/bash
# Performance tuning validation script
# Measures ingestion and extraction throughput before/after optimizations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"

echo "═══════════════════════════════════════════════════════════════"
echo "  🚀 PERFORMANCE TUNING VALIDATION"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Create test data sizes
declare -a SIZES=(10 50 100)  # MB - reduced for faster testing
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "Creating test datasets..."
for SIZE in "${SIZES[@]}"; do
    echo "  Generating ${SIZE}MB test file..."
    dd if=/dev/urandom of="$TEMP_DIR/test_${SIZE}mb.bin" bs=1M count=$SIZE 2>/dev/null
done
echo "✓ Test data ready"
echo ""

# Build optimized binary
echo "Building optimized binary..."
cd "$PROJECT_DIR"
cargo build --release --features "bt-phase-2,simd" 2>&1 | grep -E "Finished|Compiling|error|warning" || true
BINARY="$PROJECT_DIR/target/release/embeddenator"
if [ ! -f "$BINARY" ]; then
    echo "ERROR: Build failed"
    exit 1
fi
echo "✓ Binary ready at $BINARY"
echo ""

# Performance test
echo "═══════════════════════════════════════════════════════════════"
echo "  📊 PERFORMANCE MEASUREMENTS"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Function to run ingest benchmark
benchmark_ingest() {
    local file="$1"
    local size_mb="$2"

    local engram="$TEMP_DIR/test_${size_mb}.engram"
    local manifest="$TEMP_DIR/test_${size_mb}.json"

    echo -n "  Ingesting ${size_mb}MB... " >&2

    if [ ! -f "$file" ]; then
        echo -e "${YELLOW}FAILED (input file not found)${NC}" >&2
        return 1
    fi

    local start=$(date +%s%N)
    $BINARY ingest -i "$file" -e "$engram" -m "$manifest" > /dev/null 2>&1
    local result=$?
    local end=$(date +%s%N)

    if [ $result -ne 0 ]; then
        echo -e "${YELLOW}FAILED (exit code $result)${NC}" >&2
        return 1
    fi
    
    local duration_ns=$((end - start))
    local duration_ms=$((duration_ns / 1000000))
    local duration_sec=$(echo "scale=3; $duration_ms / 1000" | bc)
    
    local throughput=$(echo "scale=1; $size_mb / $duration_sec" | bc)
    
    echo -e "${GREEN}${duration_sec}s (${throughput} MB/s)${NC}" >&2
    echo "$throughput"
}

# Function to run extract benchmark
benchmark_extract() {
    local engram="$1"
    local manifest="$2"
    local size_mb="$3"
    local output_dir="$TEMP_DIR/extracted_${size_mb}"

    mkdir -p "$output_dir"

    echo -n "  Extracting ${size_mb}MB... " >&2

    if [ ! -f "$engram" ] || [ ! -f "$manifest" ]; then
        echo -e "${YELLOW}FAILED (engram/manifest files not found)${NC}" >&2
        return 1
    fi

    local start=$(date +%s%N)
    $BINARY extract -e "$engram" -m "$manifest" -o "$output_dir" > /dev/null 2>&1
    local result=$?
    local end=$(date +%s%N)

    if [ $result -ne 0 ]; then
        echo -e "${YELLOW}FAILED (exit code $result)${NC}" >&2
        return 1
    fi
    
    local duration_ns=$((end - start))
    local duration_ms=$((duration_ns / 1000000))
    local duration_sec=$(echo "scale=3; $duration_ms / 1000" | bc)
    
    local throughput=$(echo "scale=1; $size_mb / $duration_sec" | bc)
    
    echo -e "${GREEN}${duration_sec}s (${throughput} MB/s)${NC}" >&2
    echo "$throughput"
}

# Run benchmarks
echo "INGESTION PERFORMANCE:"
declare -a INGEST_RATES
for i in "${!SIZES[@]}"; do
    SIZE=${SIZES[$i]}
    FILE="$TEMP_DIR/test_${SIZE}mb.bin"
    echo "  Processing ${SIZE}MB file: $FILE"
    RATE=$(benchmark_ingest "$FILE" "$SIZE")
    if [ $? -ne 0 ]; then
        echo "  Skipping extraction for ${SIZE}MB due to ingest failure"
        continue
    fi
    INGEST_RATES+=("$RATE")
done

echo ""
echo "EXTRACTION PERFORMANCE:"
declare -a EXTRACT_RATES
for i in "${!SIZES[@]}"; do
    SIZE=${SIZES[$i]}
    ENGRAM="$TEMP_DIR/test_${SIZE}.engram"
    MANIFEST="$TEMP_DIR/test_${SIZE}.json"
    echo "  Processing ${SIZE}MB engram: $ENGRAM"
    RATE=$(benchmark_extract "$ENGRAM" "$MANIFEST" "$SIZE")
    if [ $? -ne 0 ]; then
        echo "  Skipping extraction result for ${SIZE}MB due to extract failure"
        continue
    fi
    EXTRACT_RATES+=("$RATE")
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  📈 RESULTS SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Calculate averages
INGEST_AVG=$(echo "scale=1; (${INGEST_RATES[0]} + ${INGEST_RATES[1]} + ${INGEST_RATES[2]}) / 3" | bc)
EXTRACT_AVG=$(echo "scale=1; (${EXTRACT_RATES[0]} + ${EXTRACT_RATES[1]} + ${EXTRACT_RATES[2]}) / 3" | bc)

echo "Average Ingestion Rate: ${INGEST_AVG} MB/s"
echo "  Target: 30+ MB/s"
INGEST_OK=$(echo "$INGEST_AVG >= 30" | bc)
if [ "$INGEST_OK" -eq 1 ]; then
    echo -e "  Status: ${GREEN}✓ TARGET MET${NC}"
else
    echo -e "  Status: ${YELLOW}⚠ Below target${NC}"
fi

echo ""
echo "Average Extraction Rate: ${EXTRACT_AVG} MB/s"
echo "  Target: 65+ MB/s"
EXTRACT_OK=$(echo "$EXTRACT_AVG >= 65" | bc)
if [ "$EXTRACT_OK" -eq 1 ]; then
    echo -e "  Status: ${GREEN}✓ TARGET MET${NC}"
else
    echo -e "  Status: ${YELLOW}⚠ Below target${NC}"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"

# Comparison with baseline
BASELINE_INGEST=16.6
BASELINE_EXTRACT=40.8

INGEST_IMPROVEMENT=$(echo "scale=1; (($INGEST_AVG / $BASELINE_INGEST) - 1) * 100" | bc)
EXTRACT_IMPROVEMENT=$(echo "scale=1; (($EXTRACT_AVG / $BASELINE_EXTRACT) - 1) * 100" | bc)

echo "IMPROVEMENT vs BASELINE:"
echo "  Ingestion: ${INGEST_IMPROVEMENT}% faster (baseline: ${BASELINE_INGEST} MB/s)"
echo "  Extraction: ${EXTRACT_IMPROVEMENT}% faster (baseline: ${BASELINE_EXTRACT} MB/s)"
echo ""

if [ "$INGEST_OK" -eq 1 ] && [ "$EXTRACT_OK" -eq 1 ]; then
    echo -e "${GREEN}🎉 ALL TARGETS MET!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some targets not yet met. Further optimization needed.${NC}"
    exit 1
fi
