#!/usr/bin/env bash
#
# Fetch benchmark sample data for real-world benchmarks
#
# This script downloads various sample files for testing VSA encoding
# with realistic data types. Files are placed in benchmark_data/
#
# Usage:
#   ./scripts/fetch_benchmark_data.sh
#   ./scripts/fetch_benchmark_data.sh --small   # Minimal set only
#   ./scripts/fetch_benchmark_data.sh --clean   # Remove downloaded data
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DATA_DIR="$PROJECT_DIR/benchmark_data"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse arguments
SMALL_ONLY=false
CLEAN=false
for arg in "$@"; do
    case $arg in
        --small) SMALL_ONLY=true ;;
        --clean) CLEAN=true ;;
        --help|-h)
            echo "Usage: $0 [--small|--clean|--help]"
            echo "  --small  Download minimal sample set only"
            echo "  --clean  Remove all downloaded benchmark data"
            exit 0
            ;;
    esac
done

if $CLEAN; then
    log_info "Cleaning benchmark data directory..."
    rm -rf "$DATA_DIR"
    log_info "Done."
    exit 0
fi

# Create data directory
mkdir -p "$DATA_DIR"
cd "$DATA_DIR"

# Helper function to download with curl or wget
download() {
    local url="$1"
    local output="$2"
    
    if [ -f "$output" ]; then
        log_info "Already exists: $output"
        return 0
    fi
    
    log_info "Downloading: $output"
    if command -v curl &> /dev/null; then
        curl -sSL -o "$output" "$url" || return 1
    elif command -v wget &> /dev/null; then
        wget -q -O "$output" "$url" || return 1
    else
        log_error "Neither curl nor wget found"
        return 1
    fi
}

# Helper to generate synthetic data if download fails
generate_synthetic() {
    local type="$1"
    local output="$2"
    local size="${3:-65536}"
    
    log_info "Generating synthetic $type: $output"
    case $type in
        image)
            # Generate PPM image (simple uncompressed format)
            local width=256
            local height=256
            {
                echo "P6"
                echo "$width $height"
                echo "255"
                for ((y=0; y<height; y++)); do
                    for ((x=0; x<width; x++)); do
                        printf "\\x$(printf '%02x' $((x % 256)))\\x$(printf '%02x' $((y % 256)))\\x$(printf '%02x' $(((x+y) % 256)))"
                    done
                done
            } > "$output"
            ;;
        audio)
            # Generate raw PCM audio (sine wave)
            dd if=/dev/urandom bs=1 count=$size of="$output" 2>/dev/null
            ;;
        video)
            # Generate raw YUV frames
            dd if=/dev/urandom bs=1 count=$size of="$output" 2>/dev/null
            ;;
        binary)
            dd if=/dev/urandom bs=1 count=$size of="$output" 2>/dev/null
            ;;
        text)
            # Generate Lorem ipsum-like text
            for ((i=0; i<100; i++)); do
                echo "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris."
            done > "$output"
            ;;
    esac
}

log_info "Fetching benchmark data to: $DATA_DIR"
echo ""

# ============================================================================
# SAMPLE IMAGES
# ============================================================================
log_info "=== Images ==="

# Small test image (public domain)
download "https://www.w3.org/Graphics/PNG/nurbcup2si.png" "sample.png" || \
    generate_synthetic image "sample.ppm"

if ! $SMALL_ONLY; then
    # Larger test images
    download "https://upload.wikimedia.org/wikipedia/commons/thumb/4/47/PNG_transparency_demonstration_1.png/280px-PNG_transparency_demonstration_1.png" "rgba_sample.png" || \
        generate_synthetic image "rgba_sample.ppm"
fi

# ============================================================================
# SAMPLE AUDIO
# ============================================================================
log_info "=== Audio ==="

# Generate synthetic audio samples (WAV format is complex, use raw PCM)
generate_synthetic audio "sample_audio.raw" 88200  # 1 second @ 44.1kHz 16-bit mono

if ! $SMALL_ONLY; then
    generate_synthetic audio "sample_audio_5s.raw" 441000  # 5 seconds
fi

# ============================================================================
# SAMPLE VIDEO FRAMES
# ============================================================================
log_info "=== Video Frames ==="

# Generate synthetic video frames (raw YUV420p)
mkdir -p video_frames
for i in $(seq 0 9); do
    generate_synthetic video "video_frames/frame_$i.yuv" 115200  # 320x240 YUV420p
done

# ============================================================================
# SAMPLE DOCUMENTS
# ============================================================================
log_info "=== Documents ==="

# Generate various text documents
generate_synthetic text "sample_text.txt"

# Create a markdown document
cat > sample_doc.md << 'EOF'
# Sample Technical Document

## Introduction

This document serves as a benchmark sample for testing VSA encoding
of structured text documents. It contains various formatting elements
commonly found in technical documentation.

## Code Examples

```rust
fn main() {
    let config = ReversibleVSAConfig::default();
    let data = b"Hello, World!";
    let encoded = SparseVec::encode_data(data, &config, None);
    println!("Encoded to {} dimensions", encoded.nnz());
}
```

## Tables

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Encode    | O(n)       | Linear in data size |
| Decode    | O(n)       | Linear in data size |
| Bundle    | O(k)       | Linear in sparsity |
| Bind      | O(k)       | Linear in sparsity |
| Cosine    | O(k)       | Linear in sparsity |

## Mathematical Notation

The cosine similarity between two vectors is defined as:

$$\cos(\theta) = \frac{A \cdot B}{||A|| \cdot ||B||}$$

## Conclusion

This sample demonstrates encoding of structured documents.
EOF

# ============================================================================
# SAMPLE BINARIES
# ============================================================================
log_info "=== Binary Blobs ==="

# Generate various binary patterns
generate_synthetic binary "sample_binary_small.bin" 4096
generate_synthetic binary "sample_binary_medium.bin" 65536

if ! $SMALL_ONLY; then
    generate_synthetic binary "sample_binary_large.bin" 262144
fi

# ============================================================================
# SUMMARY
# ============================================================================
echo ""
log_info "=== Summary ==="
echo ""
echo "Downloaded/generated files:"
find "$DATA_DIR" -type f -exec ls -lh {} \; | awk '{print "  " $NF ": " $5}'
echo ""

TOTAL_SIZE=$(du -sh "$DATA_DIR" | cut -f1)
log_info "Total benchmark data size: $TOTAL_SIZE"
echo ""
log_info "Run benchmarks with: cargo bench --bench real_world"
