#!/usr/bin/env bash
# homelab-build.sh - Automated Docker builds for homelab deployment
#
# Usage:
#   ./scripts/homelab-build.sh [OPTIONS]
#
# Options:
#   --version VERSION    Git tag to build (default: latest tag)
#   --arch ARCH          Target architecture: amd64, arm64, or both (default: both)
#   --push               Push to registry after build
#   --registry URL       Registry URL (default: ghcr.io/tzervas/embeddenator)
#   --dry-run            Show commands without executing
#   --skip-tests         Skip test execution before build
#   --help               Show this help message
#
# Examples:
#   # Build latest version, both architectures
#   ./scripts/homelab-build.sh --push
#
#   # Build specific version, amd64 only
#   ./scripts/homelab-build.sh --version v0.20.0 --arch amd64 --push
#
#   # Dry run to see what would be built
#   ./scripts/homelab-build.sh --version v0.21.0 --dry-run
#
# Requirements:
#   - Docker with buildx support
#   - QEMU for cross-compilation (if building arm64 on amd64)
#   - GitHub Container Registry authentication (for --push)
#
# Authentication:
#   echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCKERFILE="$PROJECT_ROOT/.docker/Dockerfile.embr-ci"
MANIFEST_SCRIPT="$PROJECT_ROOT/.docker/generate_manifest.py"

# Defaults
REGISTRY="${EMBEDDENATOR_REGISTRY:-ghcr.io/tzervas/embeddenator}"
VERSION=""
ARCH="both"
PUSH=false
DRY_RUN=false
SKIP_TESTS=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Show help
show_help() {
    sed -n '/^#/!q; s/^# //p; s/^#//p' "$0"
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --arch)
            ARCH="$2"
            if [[ ! "$ARCH" =~ ^(amd64|arm64|both)$ ]]; then
                log_error "Invalid architecture: $ARCH (must be amd64, arm64, or both)"
                exit 1
            fi
            shift 2
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --registry)
            REGISTRY="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            ;;
    esac
done

# Detect version if not specified
if [[ -z "$VERSION" ]]; then
    log_info "No version specified, detecting latest git tag..."
    cd "$PROJECT_ROOT"
    VERSION=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
    if [[ -z "$VERSION" ]]; then
        log_error "No git tags found. Create a tag or specify --version"
        exit 1
    fi
    log_info "Detected version: $VERSION"
fi

# Validate version format
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9\.]+)?$ ]]; then
    log_warn "Version doesn't match vX.Y.Z format: $VERSION"
fi

# Check prerequisites
check_prereqs() {
    log_info "Checking prerequisites..."
    
    # Docker
    if ! command -v docker &>/dev/null; then
        log_error "Docker not found. Install: https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    # Buildx
    if ! docker buildx version &>/dev/null; then
        log_error "Docker buildx not available. Install: https://docs.docker.com/buildx/working-with-buildx/"
        exit 1
    fi
    
    # QEMU (for cross-compilation)
    if [[ "$ARCH" == "both" || "$ARCH" == "arm64" ]] && [[ "$(uname -m)" != "aarch64" ]]; then
        if ! docker run --rm --privileged multiarch/qemu-user-static --reset -p yes &>/dev/null; then
            log_warn "QEMU not configured for cross-compilation. Installing..."
            docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
        fi
    fi
    
    # Dockerfile
    if [[ ! -f "$DOCKERFILE" ]]; then
        log_error "Dockerfile not found: $DOCKERFILE"
        exit 1
    fi
    
    # Manifest script
    if [[ ! -f "$MANIFEST_SCRIPT" ]]; then
        log_error "Manifest script not found: $MANIFEST_SCRIPT"
        exit 1
    fi
    
    # Python (for manifest generation)
    if [[ "$PUSH" == true ]] && ! command -v python3 &>/dev/null; then
        log_error "Python 3 required for manifest generation. Install: apt install python3"
        exit 1
    fi
    
    # Registry authentication (if pushing)
    if [[ "$PUSH" == true ]]; then
        if ! docker info 2>/dev/null | grep -q "ghcr.io"; then
            log_warn "Not authenticated to GHCR. Run: echo \$GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin"
        fi
    fi
    
    log_success "Prerequisites OK"
}

# Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == true ]]; then
        log_warn "Skipping tests (--skip-tests)"
        return
    fi
    
    log_info "Running tests before build..."
    cd "$PROJECT_ROOT"
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "DRY RUN: cargo test --all --release"
        return
    fi
    
    if ! cargo test --all --release; then
        log_error "Tests failed. Fix before building, or use --skip-tests"
        exit 1
    fi
    
    log_success "Tests passed"
}

# Build for single architecture
build_arch() {
    local arch=$1
    local platform
    case $arch in
        amd64) platform="linux/amd64" ;;
        arm64) platform="linux/arm64" ;;
        *) log_error "Invalid architecture: $arch"; exit 1 ;;
    esac
    
    local image_tag="${REGISTRY}:${VERSION}-${arch}"
    
    log_info "Building $arch image: $image_tag"
    
    local build_cmd=(
        docker buildx build
        --platform "$platform"
        --file "$DOCKERFILE"
        --tag "$image_tag"
        --build-arg "RUST_VERSION=1.84"
        --build-arg "VERSION=$VERSION"
    )
    
    if [[ "$PUSH" == true ]]; then
        build_cmd+=(--push)
    else
        build_cmd+=(--load)
    fi
    
    build_cmd+=("$PROJECT_ROOT")
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "DRY RUN: ${build_cmd[*]}"
        return
    fi
    
    if ! "${build_cmd[@]}"; then
        log_error "Build failed for $arch"
        exit 1
    fi
    
    log_success "Built $arch image: $image_tag"
}

# Generate and push manifest
create_manifest() {
    log_info "Creating multi-arch manifest..."
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "DRY RUN: python3 $MANIFEST_SCRIPT $VERSION --registry $REGISTRY"
        log_info "DRY RUN: docker manifest create ${REGISTRY}:latest --amend ${REGISTRY}:${VERSION}-amd64 --amend ${REGISTRY}:${VERSION}-arm64"
        return
    fi
    
    # Create version manifest
    if ! python3 "$MANIFEST_SCRIPT" "$VERSION" --registry "$REGISTRY"; then
        log_error "Manifest creation failed"
        exit 1
    fi
    
    # Create 'latest' manifest
    log_info "Creating 'latest' manifest..."
    docker manifest create "${REGISTRY}:latest" \
        --amend "${REGISTRY}:${VERSION}-amd64" \
        --amend "${REGISTRY}:${VERSION}-arm64"
    
    docker manifest push "${REGISTRY}:latest"
    
    log_success "Manifests created and pushed"
}

# Main execution
main() {
    log_info "=== Embeddenator Homelab Build ==="
    log_info "Version: $VERSION"
    log_info "Architecture: $ARCH"
    log_info "Registry: $REGISTRY"
    log_info "Push: $PUSH"
    log_info "Dry Run: $DRY_RUN"
    echo
    
    check_prereqs
    run_tests
    
    # Build architectures
    if [[ "$ARCH" == "both" ]]; then
        build_arch "amd64"
        build_arch "arm64"
        
        if [[ "$PUSH" == true ]]; then
            create_manifest
        else
            log_warn "Skipping manifest creation (--push not specified)"
        fi
    else
        build_arch "$ARCH"
        log_warn "Single architecture build, no manifest needed"
    fi
    
    # Summary
    echo
    log_success "=== Build Complete ==="
    log_info "Version: $VERSION"
    if [[ "$PUSH" == true ]]; then
        log_info "Images pushed to: $REGISTRY"
        log_info "  - ${REGISTRY}:${VERSION}"
        log_info "  - ${REGISTRY}:latest"
        if [[ "$ARCH" == "both" ]]; then
            log_info "  - ${REGISTRY}:${VERSION}-amd64"
            log_info "  - ${REGISTRY}:${VERSION}-arm64"
        else
            log_info "  - ${REGISTRY}:${VERSION}-${ARCH}"
        fi
    else
        log_info "Images built locally (use --push to upload)"
    fi
    
    echo
    log_info "Next steps:"
    if [[ "$PUSH" != true ]]; then
        log_info "  1. Test image: docker run --rm ${REGISTRY}:${VERSION}-${ARCH:-amd64} --version"
        log_info "  2. Push to registry: $0 --version $VERSION --push"
    else
        log_info "  1. Pull image: docker pull ${REGISTRY}:${VERSION}"
        log_info "  2. Deploy to homelab: docker-compose up -d embeddenator"
    fi
}

main
