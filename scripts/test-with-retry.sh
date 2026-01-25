#!/bin/bash
# Workspace-wide test runner with selective debug retry
# Runs tests quietly first, then re-runs failures with debug output
#
# Usage: ./scripts/test-with-retry.sh [package] [--debug]
#   package   Optional: specific package to test (e.g., embeddenator-vsa)
#   --debug   Force debug mode for all tests
#
# Examples:
#   ./scripts/test-with-retry.sh                    # Test all packages
#   ./scripts/test-with-retry.sh embeddenator-vsa   # Test specific package
#   ./scripts/test-with-retry.sh --debug            # Test all with debug
#   ./scripts/test-with-retry.sh embeddenator-vsa --debug

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$WORKSPACE_ROOT"

DEBUG_MODE=false
TARGET_PACKAGE=""

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --debug)
            DEBUG_MODE=true
            ;;
        *)
            TARGET_PACKAGE="$arg"
            ;;
    esac
done

# Packages to test
if [[ -n "$TARGET_PACKAGE" ]]; then
    PACKAGES=("$TARGET_PACKAGE")
else
    PACKAGES=(
        embeddenator
        embeddenator-cli
        embeddenator-contract-bench
        embeddenator-fs
        embeddenator-interop
        embeddenator-io
        embeddenator-obs
        embeddenator-retrieval
        embeddenator-testkit
        embeddenator-vsa
        embeddenator-workspace
    )
fi

echo "═══════════════════════════════════════════════════════════"
echo "  Workspace Test Suite with Selective Debug Retry"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Testing packages: ${PACKAGES[*]}"
echo "Debug mode: $DEBUG_MODE"
echo ""

# Track results
PASSED_PACKAGES=()
FAILED_PACKAGES=()

for pkg in "${PACKAGES[@]}"; do
    if [[ ! -d "$pkg" ]]; then
        echo "⚠️  Skipping $pkg (directory not found)"
        continue
    fi
    
    echo "───────────────────────────────────────────────────────────"
    echo "📦 Testing $pkg"
    echo "───────────────────────────────────────────────────────────"
    
    cd "$WORKSPACE_ROOT/$pkg"
    
    # First pass: quiet (no debug)
    if [[ "$DEBUG_MODE" == "true" ]]; then
        export RUST_BACKTRACE=full
        export RUST_LOG=debug
    else
        export RUST_BACKTRACE=0
        unset RUST_LOG
    fi
    
    FIRST_PASS_FAILED=false
    if ! cargo test --all-features 2>&1 | tee /tmp/test-output-$pkg.txt; then
        FIRST_PASS_FAILED=true
    fi
    
    # Second pass: debug output for failures (if not already in debug mode)
    if [[ "$FIRST_PASS_FAILED" == "true" && "$DEBUG_MODE" != "true" ]]; then
        echo ""
        echo "🔍 Re-running $pkg with debug output..."
        echo ""
        
        export RUST_BACKTRACE=full
        export RUST_LOG=debug
        
        if ! cargo test --all-features -- --nocapture 2>&1; then
            FAILED_PACKAGES+=("$pkg")
        else
            # Second pass succeeded (flaky test?)
            echo "⚠️  $pkg passed on retry (possible flaky test)"
            PASSED_PACKAGES+=("$pkg")
        fi
    elif [[ "$FIRST_PASS_FAILED" == "true" ]]; then
        FAILED_PACKAGES+=("$pkg")
    else
        PASSED_PACKAGES+=("$pkg")
    fi
    
    echo ""
done

cd "$WORKSPACE_ROOT"

echo "═══════════════════════════════════════════════════════════"
echo "  Summary"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Passed (${#PASSED_PACKAGES[@]}):"
for pkg in "${PASSED_PACKAGES[@]}"; do
    echo "  ✅ $pkg"
done

if [[ ${#FAILED_PACKAGES[@]} -gt 0 ]]; then
    echo ""
    echo "Failed (${#FAILED_PACKAGES[@]}):"
    for pkg in "${FAILED_PACKAGES[@]}"; do
        echo "  ❌ $pkg"
    done
    echo ""
    echo "💡 Tip: Re-run with --debug for verbose output:"
    echo "   ./scripts/test-with-retry.sh ${FAILED_PACKAGES[0]} --debug"
    exit 1
else
    echo ""
    echo "🎉 All packages passed!"
    exit 0
fi
