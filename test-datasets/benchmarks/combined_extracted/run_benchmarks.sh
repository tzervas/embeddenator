#!/bin/bash
# Embeddenator Benchmark Suite
# Tests mixed data types: structured, unstructured, binary

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI="$SCRIPT_DIR/../embeddenator-cli/target/release/embeddenator-cli"
OUTPUT_DIR="$SCRIPT_DIR/benchmarks"
RESULTS_FILE="$OUTPUT_DIR/benchmark_results.md"

mkdir -p "$OUTPUT_DIR"

echo "# Embeddenator Benchmark Results" > "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "**Date:** $(date -Iseconds)" >> "$RESULTS_FILE"
echo "**Host:** $(hostname)" >> "$RESULTS_FILE"
echo "**CPU:** $(lscpu | grep 'Model name' | cut -d: -f2 | xargs)" >> "$RESULTS_FILE"
echo "**RAM:** $(free -h | grep Mem | awk '{print $2}')" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Function to run benchmark and capture metrics
benchmark() {
    local name="$1"
    local input_dir="$2"
    local engram_file="$OUTPUT_DIR/${name}.engram"
    local manifest_file="$OUTPUT_DIR/${name}.json"
    local extract_dir="$OUTPUT_DIR/${name}_extracted"

    echo "=== Benchmarking: $name ===" | tee -a "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"

    # Get input size
    local input_size=$(du -sb "$input_dir" | cut -f1)
    local input_size_human=$(du -sh "$input_dir" | cut -f1)
    echo "Input size: $input_size_human ($input_size bytes)" | tee -a "$RESULTS_FILE"

    # Ingest benchmark
    echo "### Ingestion" >> "$RESULTS_FILE"
    local start_time=$(date +%s.%N)
    $CLI ingest -i "$input_dir" -e "$engram_file" -m "$manifest_file" -v 2>&1 | tee "$OUTPUT_DIR/${name}_ingest.log"
    local end_time=$(date +%s.%N)
    local ingest_time=$(echo "$end_time - $start_time" | bc)
    local ingest_throughput=$(echo "scale=2; $input_size / 1048576 / $ingest_time" | bc)

    echo "- Time: ${ingest_time}s" >> "$RESULTS_FILE"
    echo "- Throughput: ${ingest_throughput} MB/s" >> "$RESULTS_FILE"

    # Engram size
    local engram_size=$(stat -c%s "$engram_file" 2>/dev/null || echo "0")
    local engram_size_human=$(du -sh "$engram_file" | cut -f1)
    local overhead=$(echo "scale=2; $engram_size * 100 / $input_size" | bc)
    echo "- Engram size: $engram_size_human (${overhead}% of input)" >> "$RESULTS_FILE"

    # Extract benchmark
    echo "" >> "$RESULTS_FILE"
    echo "### Extraction" >> "$RESULTS_FILE"
    rm -rf "$extract_dir"
    mkdir -p "$extract_dir"
    start_time=$(date +%s.%N)
    $CLI extract -e "$engram_file" -m "$manifest_file" -o "$extract_dir" -v 2>&1 | tee "$OUTPUT_DIR/${name}_extract.log"
    end_time=$(date +%s.%N)
    local extract_time=$(echo "$end_time - $start_time" | bc)
    local extract_throughput=$(echo "scale=2; $input_size / 1048576 / $extract_time" | bc)

    echo "- Time: ${extract_time}s" >> "$RESULTS_FILE"
    echo "- Throughput: ${extract_throughput} MB/s" >> "$RESULTS_FILE"

    # Verify integrity
    echo "" >> "$RESULTS_FILE"
    echo "### Integrity Verification" >> "$RESULTS_FILE"
    local total_files=$(find "$input_dir" -type f | wc -l)
    local extracted_files=$(find "$extract_dir" -type f | wc -l)
    echo "- Original files: $total_files" >> "$RESULTS_FILE"
    echo "- Extracted files: $extracted_files" >> "$RESULTS_FILE"

    # Check file-by-file integrity (sample first 10)
    local matched=0
    local mismatched=0
    for f in $(find "$input_dir" -type f | head -10); do
        local rel_path="${f#$input_dir/}"
        local extracted_f="$extract_dir/$rel_path"
        if [ -f "$extracted_f" ]; then
            if diff -q "$f" "$extracted_f" > /dev/null 2>&1; then
                ((matched++))
            else
                ((mismatched++))
                echo "  - MISMATCH: $rel_path" >> "$RESULTS_FILE"
            fi
        else
            ((mismatched++))
            echo "  - MISSING: $rel_path" >> "$RESULTS_FILE"
        fi
    done
    local accuracy=$(echo "scale=2; $matched * 100 / ($matched + $mismatched)" | bc)
    echo "- Sample accuracy: ${accuracy}% ($matched/$((matched+mismatched)) files)" >> "$RESULTS_FILE"

    echo "" >> "$RESULTS_FILE"
    echo "---" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"

    # Return metrics for summary
    echo "$name,$input_size_human,$ingest_time,$ingest_throughput,$engram_size_human,$overhead,$extract_time,$extract_throughput,$accuracy"
}

# Run benchmarks
echo "Starting Embeddenator Benchmark Suite..."
echo ""

echo "## Summary Table" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "| Dataset | Size | Ingest (s) | Ingest MB/s | Engram | Overhead% | Extract (s) | Extract MB/s | Accuracy |" >> "$RESULTS_FILE"
echo "|---------|------|------------|-------------|--------|-----------|-------------|--------------|----------|" >> "$RESULTS_FILE"

# Benchmark each category
for category in structured unstructured binary; do
    if [ -d "$SCRIPT_DIR/$category" ]; then
        result=$(benchmark "$category" "$SCRIPT_DIR/$category")
        IFS=',' read -r name size ingest_t ingest_tp engram_sz overhead extract_t extract_tp accuracy <<< "$result"
        echo "| $name | $size | $ingest_t | $ingest_tp | $engram_sz | $overhead | $extract_t | $extract_tp | $accuracy% |" >> "$RESULTS_FILE"
    fi
done

# Combined benchmark
echo ""
echo "Running combined benchmark (all data)..."
result=$(benchmark "combined" "$SCRIPT_DIR")
IFS=',' read -r name size ingest_t ingest_tp engram_sz overhead extract_t extract_tp accuracy <<< "$result"
echo "| **combined** | $size | $ingest_t | $ingest_tp | $engram_sz | $overhead | $extract_t | $extract_tp | $accuracy% |" >> "$RESULTS_FILE"

echo "" >> "$RESULTS_FILE"
echo "## Detailed Logs" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "Logs saved in: \`$OUTPUT_DIR/\`" >> "$RESULTS_FILE"

echo ""
echo "Benchmark complete! Results saved to: $RESULTS_FILE"
cat "$RESULTS_FILE"
