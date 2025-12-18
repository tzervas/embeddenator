#!/bin/bash
# Test runner with clear reporting and validation
# Ensures we report accurate test counts and detect when tests are skipped

set -e

echo "═══════════════════════════════════════════════════════════"
echo "  Embeddenator Test Suite"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Run all tests and capture output
TEST_OUTPUT=$(cargo test --all 2>&1)

# Parse integration tests (look for the .rs file)
INTEGRATION_LINE=$(echo "$TEST_OUTPUT" | grep -A 1 "Running tests/integration_cli.rs" | grep "running" || echo "running 0 tests")
INTEGRATION_RESULT=$(echo "$TEST_OUTPUT" | grep -A 2 "Running tests/integration_cli.rs" | grep "test result" || echo "")
INTEGRATION_COUNT=$(echo "$INTEGRATION_LINE" | grep -oP '\d+' | head -1)
INTEGRATION_PASSED=$(echo "$INTEGRATION_RESULT" | grep -oP '\d+(?= passed)' || echo "0")

# Parse unit tests (look for the .rs file)
UNIT_LINE=$(echo "$TEST_OUTPUT" | grep -A 1 "Running tests/unit_tests.rs" | grep "running" || echo "running 0 tests")
UNIT_RESULT=$(echo "$TEST_OUTPUT" | grep -A 2 "Running tests/unit_tests.rs" | grep "test result" || echo "")
UNIT_COUNT=$(echo "$UNIT_LINE" | grep -oP '\d+' | head -1)
UNIT_PASSED=$(echo "$UNIT_RESULT" | grep -oP '\d+(?= passed)' || echo "0")

# Calculate totals
TOTAL_COUNT=$((INTEGRATION_COUNT + UNIT_COUNT))
TOTAL_PASSED=$((INTEGRATION_PASSED + UNIT_PASSED))

echo "Integration Tests (tests/integration_cli.rs):"
if [ "$INTEGRATION_COUNT" -eq 0 ]; then
    echo "  ⚠️  SKIPPED: No tests found or 0 tests ran"
else
    echo "  Running: $INTEGRATION_COUNT tests"
    echo "  Result:  $INTEGRATION_PASSED/$INTEGRATION_COUNT passed"
fi
echo ""

echo "Unit Tests (tests/unit_tests.rs):"
if [ "$UNIT_COUNT" -eq 0 ]; then
    echo "  ⚠️  SKIPPED: No tests found or 0 tests ran"
else
    echo "  Running: $UNIT_COUNT tests"
    echo "  Result:  $UNIT_PASSED/$UNIT_COUNT passed"
fi
echo ""

echo "───────────────────────────────────────────────────────────"
echo "  Summary"
echo "───────────────────────────────────────────────────────────"
echo "Total Tests:   $TOTAL_COUNT"
echo "Passed:        $TOTAL_PASSED"
echo "Failed:        $((TOTAL_COUNT - TOTAL_PASSED))"
echo ""

# Validation
EXIT_CODE=0

if [ "$TOTAL_COUNT" -eq 0 ]; then
    echo "❌ ERROR: No tests were run! This is a critical failure."
    EXIT_CODE=1
elif [ "$INTEGRATION_COUNT" -eq 0 ]; then
    echo "⚠️  WARNING: Integration tests were not run (0 tests)"
    EXIT_CODE=1
elif [ "$UNIT_COUNT" -eq 0 ]; then
    echo "⚠️  WARNING: Unit tests were not run (0 tests)"
    EXIT_CODE=1
elif [ "$TOTAL_PASSED" -ne "$TOTAL_COUNT" ]; then
    echo "❌ FAILED: $((TOTAL_COUNT - TOTAL_PASSED)) test(s) failed"
    EXIT_CODE=1
else
    echo "✅ SUCCESS: All $TOTAL_COUNT tests passed"
    EXIT_CODE=0
fi

echo ""
exit $EXIT_CODE
