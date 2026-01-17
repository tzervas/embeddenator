#!/bin/bash
# Complete Profiling Toolkit - All tools in one pass
# Usage: ./profile_comprehensive.sh <size_mb>
# Example: ./profile_comprehensive.sh 500

set -e

SIZE=${1:-100}
OUTPUT_DIR="profile_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$OUTPUT_DIR"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║        COMPREHENSIVE PERFORMANCE PROFILING SUITE          ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Dataset Size: ${SIZE}MB"
echo "Output Directory: $OUTPUT_DIR"
echo "Timestamp: $TIMESTAMP"
echo ""

# Generate test data
if [ ! -f "test_data_${SIZE}mb.bin" ]; then
    echo "[1/5] Generating ${SIZE}MB test dataset..."
    head -c $((SIZE * 1024 * 1024)) </dev/urandom > test_data_${SIZE}mb.bin
    echo "      ✓ Test data ready"
else
    echo "[1/5] Using existing ${SIZE}MB test dataset"
fi

# Build optimized binary
echo "[2/5] Building optimized binary..."
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release --quiet 2>/dev/null || \
    CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release
echo "      ✓ Binary built"

# CPU Profiling
echo "[3/5] Running CPU profiler (perf)..."
PERF_FILE="$OUTPUT_DIR/ingest-cpu-$TIMESTAMP.perf"
timeout 30 perf record -F 99 \
    -o "$PERF_FILE" \
    ./target/release/embeddenator ingest \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.perf \
    -m out.json.perf \
    2>&1 || true
echo "      ✓ CPU profile saved: $PERF_FILE"

# Memory Profiling
echo "[4/5] Running memory profiler (Valgrind)..."
MEM_FILE="$OUTPUT_DIR/ingest-mem-$TIMESTAMP.massif"
valgrind --tool=massif \
    --massif-out-file="$MEM_FILE" \
    --max-snapshots=100 \
    ./target/release/embeddenator ingest \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.mem \
    -m out.json.mem \
    2>&1 | grep -E "(Command|Massif)" | head -5 || true
echo "      ✓ Memory profile saved: $MEM_FILE"

# Flamegraph
echo "[5/5] Generating flamegraph..."
FG_FILE="$OUTPUT_DIR/ingest-$TIMESTAMP.svg"
cargo flamegraph \
    --bin embeddenator \
    -o "$FG_FILE" \
    --freq 99 \
    -- ingest \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.fg \
    -m out.json.fg \
    2>&1 | tail -3 || true
echo "      ✓ Flamegraph saved: $FG_FILE"

# Cache Statistics
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "CACHE & INSTRUCTION STATISTICS"
echo "═══════════════════════════════════════════════════════════"
perf stat -e cycles,instructions,cache-references,cache-misses,branches,branch-misses,LLC-loads,LLC-load-misses \
    ./target/release/embeddenator ingest \
    -i test_data_${SIZE}mb.bin \
    -e out.engram.stat \
    -m out.json.stat \
    2>&1 | tail -20

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "PROFILING COMPLETE"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Generated Files:"
echo "  1. CPU Profile (perf): $PERF_FILE"
echo "     → View: perf report -i $PERF_FILE"
echo ""
echo "  2. Memory Profile (Valgrind): $MEM_FILE"
echo "     → View: ms_print $MEM_FILE | less"
echo ""
echo "  3. Flamegraph (SVG): $FG_FILE"
echo "     → View: firefox $FG_FILE (or any web browser)"
echo ""

# Generate summary report
REPORT_FILE="$OUTPUT_DIR/PROFILING_SUMMARY_$TIMESTAMP.txt"
cat > "$REPORT_FILE" << 'EOF'
═══════════════════════════════════════════════════════════
PROFILING RESULTS INTERPRETATION GUIDE
═══════════════════════════════════════════════════════════

1. CPU PROFILE (perf)
   ─────────────────────────────────────────────────────
   Shows which functions consume the most CPU time.
   
   Top Priority Optimizations:
   - Functions with >10% CPU usage are high-value targets
   - Look for:
     * Memory allocation patterns
     * Algorithm complexity (O(n²) vs O(n))
     * Tight inner loops
   
   Action: Profile before/after optimization to measure impact

2. MEMORY PROFILE (Valgrind Massif)
   ─────────────────────────────────────────────────────
   Shows peak memory usage and allocation patterns.
   
   Key Metrics:
   - Peak heap: Total memory used at peak
   - Allocations: Number of malloc/new calls
   - Leaked memory: Not freed before exit
   
   Optimization Targets:
   - Peak heap > 1GB = compression/mmap candidates
   - Many small allocs = use object pools or stack allocation
   - Leaks = find and fix (review component boundaries)

3. FLAMEGRAPH
   ─────────────────────────────────────────────────────
   Interactive visualization of call stack depth and duration.
   
   How to Read:
   - Horizontal axis = time spent (wider = slower)
   - Vertical axis = call stack depth
   - Click to zoom in/out
   - Search for function names (Ctrl+F in browser)
   
   Look for:
   - Long horizontal bars = hot functions
   - Wide stacks = deep call chains (potential for inlining)
   - Repeated patterns = loop optimization candidates

═══════════════════════════════════════════════════════════
NEXT STEPS
═══════════════════════════════════════════════════════════

1. Analyze CPU profile for hotspots
2. Identify top 3 functions consuming >80% CPU
3. Review component architecture for optimization opportunities
4. Profile external crates (embeddenator-fs, embeddenator-vsa)
5. Implement targeted optimizations
6. Re-profile to verify improvements

═══════════════════════════════════════════════════════════
EOF

echo "  4. Summary Guide: $REPORT_FILE"
echo ""
echo "Quick Start:"
echo "  # View CPU hotspots"
echo "  perf report -i $PERF_FILE"
echo ""
echo "  # View memory usage"
echo "  ms_print $MEM_FILE | less"
echo ""
echo "  # View flamegraph (click to explore)"
echo "  firefox $FG_FILE &"
