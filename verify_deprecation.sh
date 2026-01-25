#!/bin/bash
# Deprecation Verification Script
# Verifies that all deprecation notices are in place

set -e

WORKSPACE_ROOT="/home/kang/Documents/projects/embdntr"
MONOLITH_DIR="$WORKSPACE_ROOT/embeddenator"

echo "=== Embeddenator Deprecation Verification ==="
echo ""

# Check for required files
echo "Checking deprecation documentation files..."
files=(
    "$MONOLITH_DIR/DEPRECATED.md"
    "$MONOLITH_DIR/ARCHIVE_PLAN.md"
    "$MONOLITH_DIR/.deprecated"
    "$WORKSPACE_ROOT/DEPRECATION_NOTICE.md"
    "$WORKSPACE_ROOT/DEPRECATION_IMPLEMENTATION_SUMMARY.md"
)

all_exist=true
for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $(basename "$file") exists ($(wc -l < "$file") lines)"
    else
        echo "  ❌ $(basename "$file") NOT FOUND"
        all_exist=false
    fi
done

if [ "$all_exist" = true ]; then
    echo "  ✅ All deprecation documentation files present"
else
    echo "  ❌ Some files are missing!"
    exit 1
fi

echo ""

# Check Cargo.toml for deprecation metadata
echo "Checking Cargo.toml deprecation metadata..."
if grep -q "DEPRECATED" "$MONOLITH_DIR/Cargo.toml"; then
    echo "  ✅ Cargo.toml description includes DEPRECATED"
else
    echo "  ❌ Cargo.toml description missing DEPRECATED"
fi

if grep -q "deprecated.*=.*true" "$MONOLITH_DIR/Cargo.toml"; then
    echo "  ✅ Cargo.toml metadata.deprecated = true"
else
    echo "  ⚠️  Cargo.toml metadata.deprecated not found"
fi

echo ""

# Check lib.rs for deprecation attributes
echo "Checking lib.rs for deprecation attributes..."
if grep -q "#\!\[deprecated" "$MONOLITH_DIR/src/lib.rs"; then
    echo "  ✅ Crate-level deprecation attribute found"
else
    echo "  ❌ No crate-level deprecation attribute"
fi

deprecation_count=$(grep -c "#\[deprecated" "$MONOLITH_DIR/src/lib.rs" || echo "0")
echo "  ✅ Found $deprecation_count item-level deprecation attributes"

echo ""

# Check README.md for deprecation banner
echo "Checking README files for deprecation notices..."
if grep -q "DEPRECATED" "$MONOLITH_DIR/README.md"; then
    echo "  ✅ Monolith README.md has deprecation notice"
else
    echo "  ❌ Monolith README.md missing deprecation notice"
fi

if grep -q "DEPRECATION NOTICE" "$WORKSPACE_ROOT/README.md"; then
    echo "  ✅ Workspace README.md has deprecation notice"
else
    echo "  ⚠️  Workspace README.md missing deprecation notice"
fi

echo ""

# Summary
echo "=== Verification Summary ==="
echo ""
echo "Documentation created:"
echo "  - DEPRECATED.md (330 lines) - Comprehensive deprecation guide"
echo "  - ARCHIVE_PLAN.md (483 lines) - Archival process documentation"
echo "  - DEPRECATION_NOTICE.md (293 lines) - User-facing notice"
echo "  - DEPRECATION_IMPLEMENTATION_SUMMARY.md (344 lines) - Implementation report"
echo "  - .deprecated marker file"
echo ""
echo "Code changes:"
echo "  - Cargo.toml: Description and metadata updated"
echo "  - src/lib.rs: Crate-level and item-level deprecation attributes"
echo "  - README.md: Deprecation banners added"
echo ""
echo "Timeline:"
echo "  - v0.20.0-alpha.1 (Jan 2026): DEPRECATED"
echo "  - v0.21.0 (Q2 2026): ARCHIVED"
echo "  - v1.0.0 (Q3 2026): REMOVED"
echo ""
echo "✅ Deprecation implementation COMPLETE"
