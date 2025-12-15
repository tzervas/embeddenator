#!/usr/bin/env bash
# CI Build Monitor and Optimizer
# Helps test and optimize CI builds locally with hang detection

set -e

PLATFORM="${1:-linux/amd64}"
MODE="${2:-build}"
TIMEOUT="${3:-300}"

echo "================================================"
echo "  Embeddenator CI Build Monitor"
echo "================================================"
echo "Platform: $PLATFORM"
echo "Mode: $MODE"
echo "Hang timeout: ${TIMEOUT}s"
echo "Cores available: $(nproc)"
echo "================================================"
echo ""

# Export parallelism settings
export CARGO_BUILD_JOBS=$(nproc)
export MAKEFLAGS=-j$(nproc)
export RUSTFLAGS="-C target-cpu=native"

echo "🔧 Optimizations enabled:"
echo "  CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"
echo "  MAKEFLAGS=$MAKEFLAGS"
echo ""

# Create monitoring function
monitor_process() {
    local PID=$1
    local TIMEOUT=$2
    local CHECK_INTERVAL=10
    local LAST_ACTIVITY=$(date +%s)
    local LOG_FILE="/tmp/ci_monitor_$$.log"
    
    echo "Starting hang monitor (PID: $PID, timeout: ${TIMEOUT}s)" | tee -a "$LOG_FILE"
    
    while kill -0 $PID 2>/dev/null; do
        sleep $CHECK_INTERVAL
        CURRENT_TIME=$(date +%s)
        ELAPSED=$((CURRENT_TIME - LAST_ACTIVITY))
        
        # Check CPU usage to detect activity
        CPU_USAGE=$(ps -p $PID -o %cpu --no-headers 2>/dev/null || echo "0")
        
        if awk "BEGIN {exit !($CPU_USAGE > 1)}"; then
            # Process is active
            LAST_ACTIVITY=$CURRENT_TIME
            echo "[$(date '+%H:%M:%S')] Active (CPU: ${CPU_USAGE}%)" | tee -a "$LOG_FILE"
        else
            # Process appears idle
            echo "[$(date '+%H:%M:%S')] Idle for ${ELAPSED}s (CPU: ${CPU_USAGE}%)" | tee -a "$LOG_FILE"
            
            if [ $ELAPSED -gt $TIMEOUT ]; then
                echo "❌ HANG DETECTED: Process idle for ${ELAPSED}s (threshold: ${TIMEOUT}s)" | tee -a "$LOG_FILE"
                echo "Terminating hung process..."
                kill -9 $PID 2>/dev/null || true
                
                # Dump diagnostic info
                echo ""
                echo "=== Diagnostic Information ==="
                echo "Process tree at time of hang:"
                pstree -p $PID 2>/dev/null || echo "pstree not available"
                
                exit 124
            fi
        fi
    done
    
    echo "Process completed normally" | tee -a "$LOG_FILE"
    cat "$LOG_FILE"
}

# Start the build
echo "🚀 Starting build..."
START_TIME=$(date +%s)

(
    timeout 1800 python3 orchestrator.py \
        --mode "$MODE" \
        --platform "$PLATFORM" \
        --verbose 2>&1 | tee /tmp/ci_build_$$.log
) &
BUILD_PID=$!

# Start monitor in background
monitor_process $BUILD_PID $TIMEOUT &
MONITOR_PID=$!

# Wait for build
wait $BUILD_PID
BUILD_EXIT=$?

# Clean up monitor
kill $MONITOR_PID 2>/dev/null || true

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo "================================================"
if [ $BUILD_EXIT -eq 0 ]; then
    echo "✅ Build completed successfully"
elif [ $BUILD_EXIT -eq 124 ]; then
    echo "❌ Build timed out or hung"
    echo "Last 100 lines of output:"
    tail -100 /tmp/ci_build_$$.log
    exit 1
else
    echo "❌ Build failed with exit code $BUILD_EXIT"
    exit $BUILD_EXIT
fi

echo "Duration: ${DURATION}s ($(($DURATION / 60))m $(($DURATION % 60))s)"
echo "================================================"

# Performance report
echo ""
echo "📊 Performance Metrics:"
echo "  Build time: ${DURATION}s"
echo "  Average CPU: $(awk '{s+=$1} END {print s/NR}' /proc/loadavg 2>/dev/null || echo 'N/A')"
echo "  Peak memory: $(cat /proc/meminfo | grep MemAvailable | awk '{print $2/1024/1024}' | xargs printf "%.1f GB" 2>/dev/null || echo 'N/A')"
echo ""
echo "💡 Tips for optimization:"
echo "  - For arm64 emulation, expect 5-10x slower than native"
echo "  - Use 'build' mode instead of 'full' for faster feedback"
echo "  - Check /tmp/ci_build_$$.log for detailed output"
echo ""
