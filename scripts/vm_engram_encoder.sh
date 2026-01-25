#!/bin/bash
# =============================================================================
# VM Engram Encoder - Progressive Filesystem Encoding
# Converts directories to embeddenator engram format with compression profiles
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
ENGRAM_DIR="${ENGRAM_DIR:-$HOME/.local/share/engram}"
WORK_DIR="${WORK_DIR:-/tmp/engram-encoder}"
EMBEDDENATOR_CLI="${EMBEDDENATOR_CLI:-embeddenator-cli}"
LOG_FILE="${WORK_DIR}/encoder.log"
MANIFEST_FILE="${ENGRAM_DIR}/manifest.json"

# Encoding priorities (lower = encode first)
declare -A PRIORITY_MAP=(
    ["/usr/share"]=10
    ["/usr/lib"]=20
    ["/lib"]=25
    ["/etc"]=30
    ["/var/lib"]=40
    ["/opt"]=50
    ["/usr/bin"]=60
    ["/usr/sbin"]=65
    ["/bin"]=70
    ["/sbin"]=75
    ["/home"]=80
    ["/var/log"]=90
    ["/var/cache"]=100
)

# Compression profile mapping (based on embeddenator-io profiles)
declare -A COMPRESSION_PROFILES=(
    ["/boot"]="-p kernel"           # zstd-19 for kernel components
    ["/lib/modules"]="-p kernel"
    ["/usr/lib"]="-p libraries"     # zstd-9 for shared libraries
    ["/lib"]="-p libraries"
    ["/usr/bin"]="-p binaries"      # zstd-6 for executables
    ["/usr/sbin"]="-p binaries"
    ["/bin"]="-p binaries"
    ["/sbin"]="-p binaries"
    ["/etc"]="-p config"            # lz4 for config files
    ["/var/log"]="-p logs"          # zstd-3 for logs
    ["/tmp"]="-p runtime"           # no compression for temp
    ["/var/cache"]="-p runtime"
    ["/usr/share"]="-p default"     # zstd-6 default
    ["/home"]="-p default"
)

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
    [ -f "$LOG_FILE" ] && echo "[$(date -Iseconds)] INFO: $1" >> "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
    [ -f "$LOG_FILE" ] && echo "[$(date -Iseconds)] SUCCESS: $1" >> "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
    [ -f "$LOG_FILE" ] && echo "[$(date -Iseconds)] WARNING: $1" >> "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    [ -f "$LOG_FILE" ] && echo "[$(date -Iseconds)] ERROR: $1" >> "$LOG_FILE"
}

log_progress() {
    echo -e "${CYAN}[PROGRESS]${NC} $1"
    [ -f "$LOG_FILE" ] && echo "[$(date -Iseconds)] PROGRESS: $1" >> "$LOG_FILE"
}

# =============================================================================
# SECTION 1: Initialization & Validation
# =============================================================================

init_workspace() {
    log_info "Initializing encoder workspace..."
    
    mkdir -p "$WORK_DIR"
    mkdir -p "$ENGRAM_DIR"
    touch "$LOG_FILE"
    
    # Initialize manifest if it doesn't exist
    if [ ! -f "$MANIFEST_FILE" ]; then
        cat > "$MANIFEST_FILE" << EOF
{
    "version": "1.0.0",
    "created": "$(date -Iseconds)",
    "encoder": "vm_engram_encoder",
    "root_path": "/",
    "directories": {},
    "stats": {
        "total_files": 0,
        "total_bytes_original": 0,
        "total_bytes_encoded": 0,
        "directories_encoded": 0
    }
}
EOF
    fi
    
    log_success "Workspace initialized at $WORK_DIR"
}

check_embeddenator() {
    log_info "Checking embeddenator-cli availability..."
    
    # Check if embeddenator-cli is in PATH or local build
    local cli_paths=(
        "$EMBEDDENATOR_CLI"
        "./target/release/embeddenator-cli"
        "../embeddenator-cli/target/release/embeddenator-cli"
        "/usr/local/bin/embeddenator-cli"
    )
    
    for cli in "${cli_paths[@]}"; do
        if [ -x "$cli" ] || command -v "$cli" &> /dev/null; then
            EMBEDDENATOR_CLI="$cli"
            log_success "Found embeddenator-cli: $EMBEDDENATOR_CLI"
            return 0
        fi
    done
    
    log_warn "embeddenator-cli not found, attempting to build..."
    
    # Try to build from source
    if [ -d "../embeddenator-cli" ]; then
        pushd "../embeddenator-cli" > /dev/null
        cargo build --release
        popd > /dev/null
        EMBEDDENATOR_CLI="../embeddenator-cli/target/release/embeddenator-cli"
        
        if [ -x "$EMBEDDENATOR_CLI" ]; then
            log_success "Built embeddenator-cli: $EMBEDDENATOR_CLI"
            return 0
        fi
    fi
    
    log_error "Could not find or build embeddenator-cli"
    log_info "Please ensure embeddenator-cli is installed or set EMBEDDENATOR_CLI env var"
    return 1
}

validate_source_directory() {
    local source_dir="$1"
    
    if [ ! -d "$source_dir" ]; then
        log_error "Source directory does not exist: $source_dir"
        return 1
    fi
    
    if [ ! -r "$source_dir" ]; then
        log_error "Source directory is not readable: $source_dir"
        return 1
    fi
    
    return 0
}

# =============================================================================
# SECTION 2: Directory Analysis
# =============================================================================

analyze_directory() {
    local dir="$1"
    local analysis_file="${WORK_DIR}/analysis_$(echo "$dir" | tr '/' '_').json"
    
    log_info "Analyzing directory: $dir"
    
    # Count files and calculate sizes
    local file_count=$(find "$dir" -type f 2>/dev/null | wc -l)
    local total_size=$(du -sb "$dir" 2>/dev/null | cut -f1)
    local symlink_count=$(find "$dir" -type l 2>/dev/null | wc -l)
    local dir_count=$(find "$dir" -type d 2>/dev/null | wc -l)
    
    # Estimate encoding time (rough: 10MB/s)
    local estimate_seconds=$((total_size / 10485760 + 1))
    
    # File type distribution
    local text_files=$(find "$dir" -type f -exec file {} \; 2>/dev/null | grep -c "text" || echo 0)
    local binary_files=$((file_count - text_files))
    
    cat > "$analysis_file" << EOF
{
    "directory": "$dir",
    "analyzed_at": "$(date -Iseconds)",
    "file_count": $file_count,
    "directory_count": $dir_count,
    "symlink_count": $symlink_count,
    "total_bytes": $total_size,
    "total_size_human": "$(numfmt --to=iec $total_size 2>/dev/null || echo "${total_size}B")",
    "text_files": $text_files,
    "binary_files": $binary_files,
    "estimated_encode_seconds": $estimate_seconds
}
EOF
    
    log_success "Analysis complete: $file_count files, $(numfmt --to=iec $total_size 2>/dev/null || echo "${total_size}B")"
    cat "$analysis_file"
}

get_compression_profile() {
    local path="$1"
    
    # Check for specific matches first
    for prefix in "${!COMPRESSION_PROFILES[@]}"; do
        if [[ "$path" == "$prefix"* ]]; then
            echo "${COMPRESSION_PROFILES[$prefix]}"
            return
        fi
    done
    
    # Default profile
    echo "-p default"
}

get_encoding_priority() {
    local path="$1"
    
    for prefix in "${!PRIORITY_MAP[@]}"; do
        if [[ "$path" == "$prefix"* ]]; then
            echo "${PRIORITY_MAP[$prefix]}"
            return
        fi
    done
    
    # Default priority
    echo "50"
}

# =============================================================================
# SECTION 3: Encoding Operations
# =============================================================================

encode_file() {
    local source_file="$1"
    local relative_path="${source_file#/}"
    local engram_path="${ENGRAM_DIR}/${relative_path}.engram"
    local manifest_path="${ENGRAM_DIR}/${relative_path}.manifest.json"
    local profile=$(get_compression_profile "$source_file")
    
    # Create parent directory in engram store
    mkdir -p "$(dirname "$engram_path")"
    
    # Encode with embeddenator-cli ingest command
    if "$EMBEDDENATOR_CLI" ingest \
        --input "$source_file" \
        --engram "$engram_path" \
        --manifest "$manifest_path" \
        --verbose 2>> "$LOG_FILE"; then
        return 0
    else
        return 1
    fi
}

encode_directory() {
    local source_dir="$1"
    local dry_run="${2:-false}"
    local progress_file="${WORK_DIR}/progress_$(echo "$source_dir" | tr '/' '_').txt"
    
    log_info "Encoding directory: $source_dir (dry_run=$dry_run)"
    
    validate_source_directory "$source_dir" || return 1
    
    # Get file list
    local files=()
    while IFS= read -r -d '' file; do
        files+=("$file")
    done < <(find "$source_dir" -type f -print0 2>/dev/null)
    
    local total_files=${#files[@]}
    local encoded=0
    local skipped=0
    local failed=0
    local total_original_bytes=0
    local total_encoded_bytes=0
    
    log_progress "Found $total_files files to encode"
    
    # Initialize progress file
    echo "0" > "$progress_file"
    
    local profile=$(get_compression_profile "$source_dir")
    log_info "Using compression profile: $profile"
    
    for file in "${files[@]}"; do
        local relative_path="${file#/}"
        local engram_path="${ENGRAM_DIR}/${relative_path}.engram"
        
        # Skip if already encoded and source hasn't changed
        if [ -f "$engram_path" ]; then
            local source_mtime=$(stat -c %Y "$file" 2>/dev/null || echo 0)
            local engram_mtime=$(stat -c %Y "$engram_path" 2>/dev/null || echo 0)
            
            if [ "$engram_mtime" -gt "$source_mtime" ]; then
                ((skipped++))
                continue
            fi
        fi
        
        if [ "$dry_run" = "true" ]; then
            local file_size=$(stat -c %s "$file" 2>/dev/null || echo 0)
            total_original_bytes=$((total_original_bytes + file_size))
            ((encoded++))
        else
            # Actual encoding
            local file_size=$(stat -c %s "$file" 2>/dev/null || echo 0)
            total_original_bytes=$((total_original_bytes + file_size))
            
            if encode_file "$file"; then
                ((encoded++))
                
                local encoded_size=$(stat -c %s "$engram_path" 2>/dev/null || echo 0)
                total_encoded_bytes=$((total_encoded_bytes + encoded_size))
            else
                ((failed++))
                log_warn "Failed to encode: $file"
            fi
        fi
        
        # Update progress
        local progress=$(( (encoded + skipped + failed) * 100 / total_files ))
        echo "$progress" > "$progress_file"
        
        # Progress output every 100 files
        if [ $(( (encoded + skipped + failed) % 100 )) -eq 0 ]; then
            log_progress "Progress: $progress% (encoded: $encoded, skipped: $skipped, failed: $failed)"
        fi
    done
    
    # Calculate compression ratio
    local ratio="N/A"
    if [ "$total_original_bytes" -gt 0 ] && [ "$total_encoded_bytes" -gt 0 ]; then
        ratio=$(echo "scale=2; $total_encoded_bytes * 100 / $total_original_bytes" | bc)
    fi
    
    log_success "Encoding complete for $source_dir"
    echo ""
    echo "=== Encoding Summary ==="
    echo "  Total files:     $total_files"
    echo "  Encoded:         $encoded"
    echo "  Skipped:         $skipped (already up-to-date)"
    echo "  Failed:          $failed"
    echo "  Original size:   $(numfmt --to=iec $total_original_bytes 2>/dev/null || echo "${total_original_bytes}B")"
    echo "  Encoded size:    $(numfmt --to=iec $total_encoded_bytes 2>/dev/null || echo "${total_encoded_bytes}B")"
    echo "  Compression:     ${ratio}%"
    echo ""
    
    # Update manifest
    update_manifest "$source_dir" "$encoded" "$total_original_bytes" "$total_encoded_bytes"
    
    return 0
}

encode_directory_parallel() {
    local source_dir="$1"
    local jobs="${2:-4}"
    local dry_run="${3:-false}"
    
    log_info "Parallel encoding: $source_dir with $jobs workers"
    
    validate_source_directory "$source_dir" || return 1
    
    local file_list="${WORK_DIR}/files_$(echo "$source_dir" | tr '/' '_').txt"
    find "$source_dir" -type f > "$file_list" 2>/dev/null
    
    local total_files=$(wc -l < "$file_list")
    log_progress "Found $total_files files for parallel encoding"
    
    local profile=$(get_compression_profile "$source_dir")
    
    if [ "$dry_run" = "true" ]; then
        log_info "DRY RUN: Would encode $total_files files with profile: $profile"
        return 0
    fi
    
    # Use GNU parallel if available, otherwise fall back to xargs
    if command -v parallel &> /dev/null; then
        cat "$file_list" | parallel -j "$jobs" --bar \
            "$EMBEDDENATOR_CLI" encode $profile \
                --input {} \
                --output "${ENGRAM_DIR}/{#}.engram" \
                --verify
    else
        log_warn "GNU parallel not found, using xargs (no progress bar)"
        cat "$file_list" | xargs -P "$jobs" -I {} bash -c \
            "mkdir -p \$(dirname \"${ENGRAM_DIR}/\${1#/}.engram\") && \
             $EMBEDDENATOR_CLI encode $profile --input \"\$1\" --output \"${ENGRAM_DIR}/\${1#/}.engram\" --verify" _ {}
    fi
    
    log_success "Parallel encoding complete"
}

# =============================================================================
# SECTION 4: Verification & Validation
# =============================================================================

verify_engram() {
    local engram_file="$1"
    local original_file="${2:-}"
    
    # Decode to temp directory using extract command
    local temp_dir="${WORK_DIR}/verify_temp_$$"
    mkdir -p "$temp_dir"
    
    # Derive manifest path from engram path
    local manifest_file="${engram_file%.engram}.manifest.json"
    
    if [ ! -f "$manifest_file" ]; then
        log_error "Manifest not found: $manifest_file"
        rm -rf "$temp_dir"
        return 1
    fi
    
    if ! "$EMBEDDENATOR_CLI" extract \
        --engram "$engram_file" \
        --manifest "$manifest_file" \
        --output-dir "$temp_dir" 2>> "$LOG_FILE"; then
        log_error "Failed to extract: $engram_file"
        rm -rf "$temp_dir"
        return 1
    fi
    
    # If original provided, compare
    if [ -n "$original_file" ] && [ -f "$original_file" ]; then
        # Find the extracted file (should match original filename)
        local filename=$(basename "$original_file")
        local extracted=$(find "$temp_dir" -name "$filename" -type f | head -1)
        
        if [ -z "$extracted" ]; then
            log_error "Extracted file not found in output"
            rm -rf "$temp_dir"
            return 1
        fi
        
        if ! diff -q "$original_file" "$extracted" > /dev/null 2>&1; then
            log_error "Verification failed: decoded doesn't match original"
            log_error "  Original: $original_file"
            log_error "  Decoded:  $extracted"
            rm -rf "$temp_dir"
            return 1
        fi
    fi
    
    rm -rf "$temp_dir"
    return 0
}

verify_directory() {
    local source_dir="$1"
    local sample_rate="${2:-100}"  # Verify 1 in N files
    
    log_info "Verifying encoded directory: $source_dir (sample rate: 1/$sample_rate)"
    
    local verified=0
    local failed=0
    local count=0
    
    while IFS= read -r -d '' file; do
        ((count++))
        
        # Sample verification
        if [ $((count % sample_rate)) -ne 0 ]; then
            continue
        fi
        
        local relative_path="${file#/}"
        local engram_path="${ENGRAM_DIR}/${relative_path}.engram"
        
        if [ ! -f "$engram_path" ]; then
            log_warn "Missing engram for: $file"
            ((failed++))
            continue
        fi
        
        if verify_engram "$engram_path" "$file"; then
            ((verified++))
        else
            ((failed++))
        fi
        
        # Progress
        if [ $((verified + failed)) -gt 0 ] && [ $(( (verified + failed) % 10 )) -eq 0 ]; then
            log_progress "Verified: $verified, Failed: $failed"
        fi
    done < <(find "$source_dir" -type f -print0 2>/dev/null)
    
    log_success "Verification complete: $verified passed, $failed failed"
    
    if [ "$failed" -gt 0 ]; then
        return 1
    fi
    return 0
}

# =============================================================================
# SECTION 5: Manifest Management
# =============================================================================

update_manifest() {
    local directory="$1"
    local file_count="$2"
    local original_bytes="$3"
    local encoded_bytes="$4"
    
    local temp_manifest="${WORK_DIR}/manifest_temp.json"
    
    # Use jq if available, otherwise Python
    if command -v jq &> /dev/null; then
        jq --arg dir "$directory" \
           --argjson files "$file_count" \
           --argjson orig "$original_bytes" \
           --argjson enc "$encoded_bytes" \
           --arg time "$(date -Iseconds)" \
           '.directories[$dir] = {
               "encoded_at": $time,
               "file_count": $files,
               "original_bytes": $orig,
               "encoded_bytes": $enc
           } |
           .stats.total_files += $files |
           .stats.total_bytes_original += $orig |
           .stats.total_bytes_encoded += $enc |
           .stats.directories_encoded += 1' \
           "$MANIFEST_FILE" > "$temp_manifest"
        
        mv "$temp_manifest" "$MANIFEST_FILE"
    else
        log_warn "jq not available, manifest update skipped"
    fi
}

show_manifest() {
    log_info "Current encoding manifest:"
    
    if command -v jq &> /dev/null; then
        jq '.' "$MANIFEST_FILE"
    else
        cat "$MANIFEST_FILE"
    fi
}

# =============================================================================
# SECTION 6: Progressive Encoding Orchestration
# =============================================================================

plan_progressive_encoding() {
    local root="${1:-/}"
    
    log_info "Planning progressive encoding from: $root"
    
    # Build sorted list by priority
    declare -a plan=()
    
    for dir in "${!PRIORITY_MAP[@]}"; do
        local full_path="${root%/}${dir}"
        if [ -d "$full_path" ]; then
            local priority=${PRIORITY_MAP[$dir]}
            plan+=("$priority|$full_path")
        fi
    done
    
    # Sort by priority
    IFS=$'\n' sorted=($(sort -t'|' -k1 -n <<< "${plan[*]}"))
    unset IFS
    
    echo ""
    echo "=== Encoding Plan ==="
    echo "Priority | Directory | Profile"
    echo "---------|-----------|--------"
    for entry in "${sorted[@]}"; do
        local priority=$(echo "$entry" | cut -d'|' -f1)
        local dir=$(echo "$entry" | cut -d'|' -f2)
        local profile=$(get_compression_profile "$dir")
        printf "%-8s | %-30s | %s\n" "$priority" "$dir" "$profile"
    done
    echo ""
}

execute_progressive_encoding() {
    local root="${1:-/}"
    local dry_run="${2:-false}"
    local start_time=$(date +%s)
    
    log_info "Starting progressive encoding from: $root"
    
    # Build sorted list by priority
    declare -a plan=()
    for dir in "${!PRIORITY_MAP[@]}"; do
        local full_path="${root%/}${dir}"
        if [ -d "$full_path" ]; then
            local priority=${PRIORITY_MAP[$dir]}
            plan+=("$priority|$full_path")
        fi
    done
    
    IFS=$'\n' sorted=($(sort -t'|' -k1 -n <<< "${plan[*]}"))
    unset IFS
    
    local total_dirs=${#sorted[@]}
    local current=0
    
    for entry in "${sorted[@]}"; do
        ((current++))
        local dir=$(echo "$entry" | cut -d'|' -f2)
        
        echo ""
        echo "=============================================="
        log_progress "[$current/$total_dirs] Encoding: $dir"
        echo "=============================================="
        
        encode_directory "$dir" "$dry_run"
        
        # Brief pause between directories
        sleep 1
    done
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo ""
    echo "=============================================="
    log_success "Progressive encoding complete!"
    echo "Total time: ${duration}s"
    echo "=============================================="
    
    show_manifest
}

# =============================================================================
# SECTION 7: VM-Specific Operations
# =============================================================================

encode_mounted_vm() {
    local mount_point="${1:-/mnt/debian-vm}"
    local dry_run="${2:-false}"
    
    if [ ! -d "$mount_point" ]; then
        log_error "VM mount point not found: $mount_point"
        log_info "Please mount the VM first: debian_vm_setup.sh --mount"
        return 1
    fi
    
    log_info "Encoding mounted VM at: $mount_point"
    
    # Update paths for mounted filesystem
    export ENGRAM_DIR="${ENGRAM_DIR:-/var/lib/engram}/vm_$(basename "$mount_point")"
    mkdir -p "$ENGRAM_DIR"
    
    execute_progressive_encoding "$mount_point" "$dry_run"
}

create_bootable_engram_image() {
    local output_image="${1:-engram_boot.img}"
    local engram_source="${2:-$ENGRAM_DIR}"
    
    log_info "Creating bootable engram image: $output_image"
    
    # Create sparse image
    truncate -s 10G "$output_image"
    
    # Format with ext4
    mkfs.ext4 -F "$output_image"
    
    # Mount and copy engrams
    local temp_mount="${WORK_DIR}/boot_mount"
    mkdir -p "$temp_mount"
    sudo mount -o loop "$output_image" "$temp_mount"
    
    # Copy engram data
    cp -a "$engram_source"/* "$temp_mount/"
    
    # Install embeddenator-fuse for mounting
    # (This would be the FUSE filesystem that reads engrams)
    
    sudo umount "$temp_mount"
    rmdir "$temp_mount"
    
    log_success "Bootable engram image created: $output_image"
}

# =============================================================================
# Main Entry Point
# =============================================================================

show_help() {
    cat << 'EOF'
VM Engram Encoder - Progressive Filesystem Encoding

USAGE:
    vm_engram_encoder.sh [COMMAND] [OPTIONS]

COMMANDS:
    analyze <dir>           Analyze directory for encoding
    encode <dir>            Encode a single directory
    encode-parallel <dir>   Encode with parallel workers
    verify <dir>            Verify encoded directory
    plan [root]             Show encoding plan for root
    progressive [root]      Execute progressive encoding
    vm-encode <mount>       Encode mounted VM filesystem
    manifest                Show encoding manifest
    help                    Show this help

OPTIONS:
    --dry-run               Simulate without encoding
    --jobs <N>              Number of parallel workers (default: 4)
    --engram-dir <dir>      Output directory for engrams

EXAMPLES:
    # Analyze a directory
    ./vm_engram_encoder.sh analyze /usr/share

    # Dry-run progressive encoding
    ./vm_engram_encoder.sh progressive / --dry-run

    # Encode mounted VM
    ./vm_engram_encoder.sh vm-encode /mnt/debian-vm

    # Parallel encoding with 8 workers
    ./vm_engram_encoder.sh encode-parallel /usr/lib --jobs 8

ENVIRONMENT:
    ENGRAM_DIR          Output directory (default: /var/lib/engram)
    WORK_DIR            Working directory (default: /tmp/engram-encoder)
    EMBEDDENATOR_CLI    Path to embeddenator-cli
EOF
}

main() {
    local cmd="${1:-help}"
    shift || true
    
    # Parse global options
    local dry_run=false
    local jobs=4
    local positional=()
    
    while [ $# -gt 0 ]; do
        case "$1" in
            --dry-run)
                dry_run=true
                shift
                ;;
            --jobs)
                jobs="$2"
                shift 2
                ;;
            --engram-dir)
                ENGRAM_DIR="$2"
                shift 2
                ;;
            *)
                positional+=("$1")
                shift
                ;;
        esac
    done
    
    # Initialize
    init_workspace
    
    case "$cmd" in
        analyze)
            analyze_directory "${positional[0]:-/}"
            ;;
        encode)
            check_embeddenator
            encode_directory "${positional[0]:-/}" "$dry_run"
            ;;
        encode-parallel)
            check_embeddenator
            encode_directory_parallel "${positional[0]:-/}" "$jobs" "$dry_run"
            ;;
        verify)
            check_embeddenator
            verify_directory "${positional[0]:-/}" "${positional[1]:-100}"
            ;;
        plan)
            plan_progressive_encoding "${positional[0]:-/}"
            ;;
        progressive)
            check_embeddenator
            execute_progressive_encoding "${positional[0]:-/}" "$dry_run"
            ;;
        vm-encode)
            check_embeddenator
            encode_mounted_vm "${positional[0]:-/mnt/debian-vm}" "$dry_run"
            ;;
        manifest)
            show_manifest
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "Unknown command: $cmd"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
