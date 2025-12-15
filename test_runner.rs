//! Test runner that provides accurate reporting
//! Filters out confusing "0 tests ran" blocks

use std::process::{Command, exit};

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("  Embeddenator Test Suite");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // Run cargo test and capture output
    let output = Command::new("cargo")
        .args(&["test", "--all", "--", "--test-threads=1"])
        .output()
        .expect("Failed to run tests");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Parse test results
    let mut integration_count = 0;
    let mut integration_passed = 0;
    let mut unit_count = 0;
    let mut unit_passed = 0;
    
    // Look for integration tests
    for line in stderr.lines().chain(stdout.lines()) {
        if line.contains("Running tests/integration_cli.rs") {
            // Next lines will have the counts
            continue;
        }
    }
    
    // Parse from stdout
    let lines: Vec<&str> = stdout.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("running") && line.contains("tests") {
            if let Some(num_str) = line.split_whitespace().nth(1) {
                if let Ok(count) = num_str.parse::<usize>() {
                    if count > 0 {
                        // Look ahead for the test result
                        for j in i+1..i+20 {
                            if j >= lines.len() { break; }
                            if lines[j].contains("test result:") && lines[j].contains("passed") {
                                if let Some(passed_str) = lines[j].split_whitespace()
                                    .find(|s| s.parse::<usize>().is_ok()) {
                                    if let Ok(passed) = passed_str.parse::<usize>() {
                                        // Determine which test suite this is
                                        if integration_count == 0 {
                                            integration_count = count;
                                            integration_passed = passed;
                                        } else if unit_count == 0 {
                                            unit_count = count;
                                            unit_passed = passed;
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Display results
    println!("Integration Tests (tests/integration_cli.rs):");
    if integration_count == 0 {
        println!("  ⚠️  SKIPPED: No tests found or 0 tests ran");
    } else {
        println!("  Running: {} tests", integration_count);
        println!("  Result:  {}/{} passed", integration_passed, integration_count);
    }
    println!();
    
    println!("Unit Tests (tests/unit_tests.rs):");
    if unit_count == 0 {
        println!("  ⚠️  SKIPPED: No tests found or 0 tests ran");
    } else {
        println!("  Running: {} tests", unit_count);
        println!("  Result:  {}/{} passed", unit_passed, unit_count);
    }
    println!();
    
    let total_count = integration_count + unit_count;
    let total_passed = integration_passed + unit_passed;
    
    println!("───────────────────────────────────────────────────────────");
    println!("  Summary");
    println!("───────────────────────────────────────────────────────────");
    println!("Total Tests:   {}", total_count);
    println!("Passed:        {}", total_passed);
    println!("Failed:        {}", total_count - total_passed);
    println!();
    
    // Determine exit code
    let exit_code = if total_count == 0 {
        println!("❌ ERROR: No tests were run! This is a critical failure.");
        1
    } else if integration_count == 0 {
        println!("⚠️  WARNING: Integration tests were not run (0 tests)");
        1
    } else if unit_count == 0 {
        println!("⚠️  WARNING: Unit tests were not run (0 tests)");
        1
    } else if total_passed != total_count {
        println!("❌ FAILED: {} test(s) failed", total_count - total_passed);
        1
    } else {
        println!("✅ SUCCESS: All {} tests passed", total_count);
        0
    };
    
    println!();
    exit(exit_code);
}
