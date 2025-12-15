#!/usr/bin/env python3
"""
Test Runner with Debug Logging
Provides accurate test reporting and detects when tests are skipped
"""

import subprocess
import sys
import re
from typing import Tuple

# Debug mode - set to True to see detailed parsing info
DEBUG = False

def debug_log(msg):
    """Print debug message if DEBUG is enabled"""
    if DEBUG:
        print(f"[DEBUG] {msg}", file=sys.stderr)

def parse_test_output(output: str) -> Tuple[int, int, int, int]:
    """
    Parse cargo test output and return (integration_count, integration_passed, unit_count, unit_passed)
    Simpler strategy: The first non-zero "running X tests" is integration, second is unit
    """
    lines = output.split('\n')
    
    integration_count = 0
    integration_passed = 0
    unit_count = 0
    unit_passed = 0
    
    test_run_index = 0  # Track which non-zero test run we're on
    
    for i, line in enumerate(lines):
        # Parse "running X tests" line
        if line.startswith('running ') and ' test' in line:
            match = re.search(r'running (\d+) tests?', line)
            if match:
                count = int(match.group(1))
                if count > 0:
                    test_run_index += 1
                    if test_run_index == 1:
                        integration_count = count
                        debug_log(f"Line {i}: Set integration_count = {count} (first non-zero run)")
                    elif test_run_index == 2:
                        unit_count = count
                        debug_log(f"Line {i}: Set unit_count = {count} (second non-zero run)")
        
        # Parse "test result: ok. X passed" line
        if 'test result:' in line and 'passed' in line:
            match = re.search(r'(\d+) passed', line)
            if match:
                passed = int(match.group(1))
                if passed > 0:
                    if integration_count > 0 and integration_passed == 0:
                        integration_passed = passed
                        debug_log(f"Line {i}: Set integration_passed = {passed}")
                    elif unit_count > 0 and unit_passed == 0:
                        unit_passed = passed
                        debug_log(f"Line {i}: Set unit_passed = {passed}")
    
    return integration_count, integration_passed, unit_count, unit_passed

def main():
    print("═══════════════════════════════════════════════════════════")
    print("  Embeddenator Test Suite")
    print("═══════════════════════════════════════════════════════════")
    print()
    
    # Run cargo test
    debug_log("Running cargo test --all")
    result = subprocess.run(
        ['cargo', 'test', '--all'],
        capture_output=True,
        text=True
    )
    
    # Combine stdout and stderr for parsing
    full_output = result.stderr + '\n' + result.stdout
    
    debug_log(f"Test command exit code: {result.returncode}")
    debug_log("="*60)
    debug_log("Parsing test output...")
    
    # Parse the output
    integration_count, integration_passed, unit_count, unit_passed = parse_test_output(full_output)
    
    debug_log("="*60)
    debug_log(f"Parsed results: integration={integration_count}/{integration_passed}, unit={unit_count}/{unit_passed}")
    
    # Display results
    print("Integration Tests (tests/integration_cli.rs):")
    if integration_count == 0:
        print("  ⚠️  SKIPPED: No tests found or 0 tests ran")
    else:
        print(f"  Running: {integration_count} tests")
        print(f"  Result:  {integration_passed}/{integration_count} passed")
    print()
    
    print("Unit Tests (tests/unit_tests.rs):")
    if unit_count == 0:
        print("  ⚠️  SKIPPED: No tests found or 0 tests ran")
    else:
        print(f"  Running: {unit_count} tests")
        print(f"  Result:  {unit_passed}/{unit_count} passed")
    print()
    
    total_count = integration_count + unit_count
    total_passed = integration_passed + unit_passed
    
    print("───────────────────────────────────────────────────────────")
    print("  Summary")
    print("───────────────────────────────────────────────────────────")
    print(f"Total Tests:   {total_count}")
    print(f"Passed:        {total_passed}")
    print(f"Failed:        {total_count - total_passed}")
    print()
    
    # Validation
    exit_code = 0
    
    if total_count == 0:
        print("❌ ERROR: No tests were run! This is a critical failure.")
        exit_code = 1
    elif integration_count == 0:
        print("⚠️  WARNING: Integration tests were not run (0 tests)")
        exit_code = 1
    elif unit_count == 0:
        print("⚠️  WARNING: Unit tests were not run (0 tests)")
        exit_code = 1
    elif total_passed != total_count:
        print(f"❌ FAILED: {total_count - total_passed} test(s) failed")
        exit_code = 1
    else:
        print(f"✅ SUCCESS: All {total_count} tests passed")
        exit_code = 0
    
    print()
    sys.exit(exit_code)

if __name__ == '__main__':
    main()
