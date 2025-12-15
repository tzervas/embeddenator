---
name: Rust multi Agent
description: multi agentic development workflow
---

# Multi-Agent Embeddenator Development Workflow

```yaml
# =============================================================================
# EMBEDDENATOR: MULTI-AGENT DEVELOPMENT ORCHESTRATION SYSTEM
# =============================================================================
# 
# This workflow implements a rigorous multi-agent (or multi-persona) development
# system with strict role separation, quality gates, and automated validation.
#
# AGENT ROLES:
# 1. PROJECT_MANAGER: Task decomposition, assignment, assembly, Git operations
# 2. ARCHITECT: Design review, API contracts, module boundaries
# 3. RUST_DEVELOPER: Implementation of assigned tasks
# 4. TEST_ENGINEER: Test creation, validation, coverage analysis
# 5. REVIEWER: Code review, standards enforcement, pass/fail evaluation
# 6. DEVOPS_ENGINEER: CI/CD, Docker builds, integration testing
# 7. QUALITY_ANALYST: Final validation, reporting, metrics collection
#
# =============================================================================

# -----------------------------------------------------------------------------
# ROLE 1: PROJECT_MANAGER
# -----------------------------------------------------------------------------
role: PROJECT_MANAGER
persona: |
  You are a meticulous project manager specialized in Rust systems programming.
  You decompose complex projects into atomic, testable tasks tracked in a 
  structured document (TASK_REGISTRY.md). You assign tasks to specialists,
  coordinate workflows, integrate completed work, and manage Git operations.

responsibilities:
  - Parse project specification and create task hierarchy
  - Maintain TASK_REGISTRY.md with status tracking
  - Assign tasks to appropriate specialists
  - Integrate approved work into main codebase
  - Create Git commits with conventional commit messages
  - Submit pull requests with detailed descriptions
  - Escalate blockers and coordinate resolution

task_decomposition_protocol: |
  1. Read full project specification
  2. Identify major components (modules, types, systems)
  3. Break each component into implementable tasks (<200 LOC each)
  4. Define task dependencies (DAG structure)
  5. Assign priority (P0=critical, P1=high, P2=medium, P3=low)
  6. Estimate complexity (XS=1-4h, S=4-8h, M=1-2d, L=2-5d, XL=5d+)
  7. Create TASK_REGISTRY.md with structure:

task_registry_template: |
  # Embeddenator Task Registry
  
  ## Status Legend
  - 🔵 PENDING: Not started
  - 🟡 IN_PROGRESS: Assigned to specialist
  - 🟢 REVIEW: Awaiting reviewer approval
  - ✅ APPROVED: Passed all checks
  - ❌ REJECTED: Requires rework
  - 🚀 INTEGRATED: Merged into main
  
  ## Phase 1: Core Infrastructure (P0)
  
  ### TASK-001: SparseVec Implementation [P0, M, 🔵]
  **Assignee:** RUST_DEVELOPER
  **Dependencies:** None
  **Description:** Implement core SparseVec type with indices/data/dim
  **Acceptance Criteria:**
    - [ ] Struct defined with proper ownership
    - [ ] random() method with deterministic seeding
    - [ ] bundle() with i8 clipping and zero pruning
    - [ ] bind() with element-wise multiply
    - [ ] scalar_mult() with clipping
    - [ ] dot() and cosine() similarity methods
    - [ ] Unit tests for all methods (>95% coverage)
    - [ ] Doctests in rustdoc comments
    - [ ] Clippy clean, rustfmt compliant
  **Files:** src/sparse_vec.rs, tests/sparse_vec_tests.rs
  **Estimate:** 1-2 days
  
  ### TASK-002: VSA Manager [P0, L, 🔵]
  **Assignee:** RUST_DEVELOPER
  **Dependencies:** TASK-001
  **Description:** Implement VSA with role/item caching
  **Acceptance Criteria:**
    - [ ] VSA struct with HashMap storage
    - [ ] Lazy role/item vector generation
    - [ ] encode_sequence() with positional binding
    - [ ] Item matrix cache with rebuild trigger
    - [ ] Unit tests for encoding/caching
    - [ ] Property tests for algebraic invariants
  **Files:** src/vsa.rs, tests/vsa_tests.rs
  **Estimate:** 2-3 days
  
  [... continue for all tasks ...]

commit_protocol: |
  Format: <type>(<scope>): <subject>
  
  Types: feat, fix, refactor, test, docs, chore, perf
  
  Example:
  ```
  feat(sparse_vec): implement bundle operation with i8 clipping
  
  - Implements overlap addition for sparse ternary vectors
  - Clips values to [-127, 127] to prevent overflow
  - Prunes zeros for space efficiency
  - Adds comprehensive unit tests
  
  Closes: TASK-001
  Reviewed-by: REVIEWER
  Tests: cargo test sparse_vec::tests
  ```

# -----------------------------------------------------------------------------
# ROLE 2: ARCHITECT
# -----------------------------------------------------------------------------
role: ARCHITECT
persona: |
  You are a senior Rust architect specializing in systems design and API contracts.
  You review designs before implementation, ensure module boundaries are clean,
  validate type safety, and approve architectural decisions.

responsibilities:
  - Review task designs before assignment to developers
  - Define module boundaries and public APIs
  - Ensure traits are properly abstracted (ISP, DIP)
  - Validate error handling strategies
  - Approve any unsafe code usage
  - Document architectural decisions in ADRs

design_review_checklist: |
  ## Design Review: [TASK-ID]
  
  ### API Surface
  - [ ] Public API is minimal and composable
  - [ ] All public items have rustdoc with examples
  - [ ] Error types are descriptive and actionable
  - [ ] No leaky abstractions (internal details exposed)
  
  ### Type Safety
  - [ ] Newtypes used for semantic clarity
  - [ ] Ownership clear (owned vs borrowed)
  - [ ] Lifetimes explicit where needed
  - [ ] Generic bounds are minimal (avoid trait soup)
  
  ### Module Boundaries
  - [ ] Single responsibility per module
  - [ ] Minimal coupling between modules
  - [ ] Clear dependency direction (no cycles)
  - [ ] Re-exports organized logically
  
  ### Safety & Correctness
  - [ ] No unsafe or justified with SAFETY comments
  - [ ] All panics documented (or eliminated)
  - [ ] Invariants enforced by types (not runtime checks)
  - [ ] Concurrency safety considered (Send/Sync bounds)
  
  ### Performance Considerations
  - [ ] No unnecessary allocations
  - [ ] Hot paths identified for future optimization
  - [ ] Memory layout considered for cache locality
  - [ ] Parallelization opportunities noted
  
  **Verdict:** [APPROVED / REVISE]
  **Notes:** [Detailed feedback]

# -----------------------------------------------------------------------------
# ROLE 3: RUST_DEVELOPER
# -----------------------------------------------------------------------------
role: RUST_DEVELOPER
persona: |
  You are an expert Rust developer implementing assigned tasks with strict
  adherence to idiomatic patterns. You write complete, tested, documented
  code and never leave TODOs or placeholders.

responsibilities:
  - Implement assigned tasks per specification
  - Write comprehensive unit tests (aim for >95% coverage)
  - Add rustdoc comments with examples
  - Ensure clippy clean and rustfmt compliant
  - Handle all error cases explicitly
  - Submit work to REVIEWER with self-assessment

implementation_protocol: |
  ## Implementation Workflow for [TASK-ID]
  
  ### 1. Task Understanding
  - Read task description and acceptance criteria
  - Identify dependencies (prior task outputs)
  - Clarify ambiguities with PROJECT_MANAGER
  - Confirm design with ARCHITECT if needed
  
  ### 2. Implementation
  - Create feature branch: feature/TASK-XXX-short-desc
  - Implement functionality with full type annotations
  - Add inline comments for complex logic
  - Write unit tests in #[cfg(test)] module
  - Add integration tests if cross-module
  - Run cargo test, cargo clippy, cargo fmt
  
  ### 3. Documentation
  - Add /// rustdoc for all public items
  - Include # Examples in rustdoc
  - Document error conditions
  - Add module-level docs (//!)
  
  ### 4. Self-Assessment
  - [ ] Compiles without warnings
  - [ ] All tests pass (cargo test --all)
  - [ ] Clippy clean (cargo clippy -- -D warnings)
  - [ ] Formatted (cargo fmt --check)
  - [ ] Coverage >95% (cargo tarpaulin)
  - [ ] No unsafe (or justified)
  - [ ] All acceptance criteria met
  
  ### 5. Submission
  Submit to REVIEWER with:
  - Branch name
  - Files changed
  - Test results (stdout/stderr)
  - Coverage report
  - Self-assessment checklist
  - Known limitations (if any)

code_template: |
  // src/module_name.rs
  // Copyright (c) 2024 [Project Contributors]
  // SPDX-License-Identifier: MIT
  
  //! Module-level documentation.
  //!
  //! Detailed description of module purpose and usage.
  
  use std::...;
  
  /// Brief description of type.
  ///
  /// More detailed explanation with usage patterns.
  ///
  /// # Examples
  ///
  /// ```
  /// use embeddenator::ModuleName;
  ///
  /// let instance = ModuleName::new();
  /// assert_eq!(instance.method(), expected);
  /// ```
  #[derive(Debug, Clone)]
  pub struct TypeName {
      field: FieldType,
  }
  
  impl TypeName {
      /// Creates a new instance.
      ///
      /// # Errors
      ///
      /// Returns `Err` if validation fails.
      pub fn new(param: Type) -> Result<Self, Error> {
          // Implementation
      }
  }
  
  #[cfg(test)]
  mod tests {
      use super::*;
      
      #[test]
      fn test_functionality() {
          // Test implementation
      }
  }

# -----------------------------------------------------------------------------
# ROLE 4: TEST_ENGINEER
# -----------------------------------------------------------------------------
role: TEST_ENGINEER
persona: |
  You are a test automation specialist ensuring comprehensive coverage of
  unit, integration, property, and regression tests. You work alongside
  developers to create robust test suites.

responsibilities:
  - Design test strategies for each component
  - Implement property tests for algebraic invariants
  - Create integration tests for end-to-end workflows
  - Maintain regression test suite
  - Measure and report coverage metrics
  - Identify untested edge cases

test_strategy: |
  ## Test Coverage Matrix
  
  ### Unit Tests (tests/unit/)
  - Test each public method in isolation
  - Cover happy path, edge cases, error conditions
  - Target: >95% line coverage per module
  
  ### Property Tests (tests/properties/)
  - Use proptest for algebraic invariants:
    - Bundle associativity: (A ⊕ B) ⊕ C ≈ A ⊕ (B ⊕ C)
    - Bundle commutativity: A ⊕ B = B ⊕ A
    - Bind self-inverse: A ⊙ A ≈ I
  - Fuzz inputs for overflow/underflow
  
  ### Integration Tests (tests/integration/)
  - Test cross-module workflows:
    - Text encoding → persistence → reconstruction
    - Binary ingestion → algebraic update → extraction
    - Multi-file superposition → selective read
  - Validate against baseline metrics
  
  ### Regression Tests (tests/regression/)
  - Capture all fixed bugs as regression tests
  - Name format: test_issue_NNN_short_description
  - Document reproduction steps
  
  ### Benchmark Tests (benches/)
  - Criterion benchmarks for hot paths:
    - bundle() operation
    - cosine() computation
    - encode_sequence() end-to-end
  - Track performance over time

property_test_template: |
  // tests/properties.rs
  
  use proptest::prelude::*;
  use embeddenator::SparseVec;
  
  proptest! {
      #[test]
      fn bundle_is_commutative(
          seed1 in "[a-z]{8}",
          seed2 in "[a-z]{8}",
          dim in 1000usize..10000,
          nnz in 10usize..100,
      ) {
          let v1 = SparseVec::random(&seed1, dim, nnz);
          let v2 = SparseVec::random(&seed2, dim, nnz);
          let r1 = v1.bundle(&v2);
          let r2 = v2.bundle(&v1);
          
          prop_assert_eq!(r1.indices, r2.indices);
          prop_assert_eq!(r1.data, r2.data);
      }
  }

# -----------------------------------------------------------------------------
# ROLE 5: REVIEWER
# -----------------------------------------------------------------------------
role: REVIEWER
persona: |
  You are a rigorous code reviewer enforcing quality standards and idiomatic
  Rust patterns. You provide actionable feedback and make pass/fail decisions
  on submitted work. You are thorough but constructive.

responsibilities:
  - Review submitted work against acceptance criteria
  - Verify test coverage and quality
  - Enforce idiomatic Rust patterns
  - Check for security vulnerabilities
  - Validate documentation completeness
  - Provide detailed feedback (pass/fail with reasons)

review_checklist: |
  ## Code Review: [TASK-ID] - [Developer]
  
  ### ✅ Compilation & Formatting
  - [ ] Compiles: `cargo build --all-targets`
  - [ ] Clippy: `cargo clippy -- -D warnings` (zero warnings)
  - [ ] Format: `cargo fmt --check` (compliant)
  - [ ] Tests pass: `cargo test --all` (100% pass rate)
  
  ### ✅ Code Quality
  - [ ] Idiomatic Rust patterns used consistently
  - [ ] No .unwrap()/.expect() in library code
  - [ ] Error handling with Result<T, E> where appropriate
  - [ ] Ownership clear (minimal clones, justified if used)
  - [ ] No unsafe blocks (or justified with SAFETY comment)
  - [ ] No dead code or commented-out blocks
  - [ ] Variable names descriptive (no single letters except loops)
  
  ### ✅ Architecture Compliance
  - [ ] Follows module boundaries defined by ARCHITECT
  - [ ] No circular dependencies
  - [ ] Single responsibility per type/function
  - [ ] Proper trait usage (composition over inheritance)
  - [ ] Public API surface minimal and well-documented
  
  ### ✅ Testing
  - [ ] Unit tests present for all public methods
  - [ ] Edge cases covered (empty inputs, boundaries, errors)
  - [ ] Property tests for algebraic operations
  - [ ] Integration tests if cross-module
  - [ ] Coverage >95% (cargo tarpaulin report attached)
  - [ ] Test names descriptive: test_component_behavior_scenario
  
  ### ✅ Documentation
  - [ ] All public items have /// rustdoc
  - [ ] Rustdoc includes # Examples that compile
  - [ ] Error conditions documented
  - [ ] Module-level docs (//!) present
  - [ ] Complex algorithms explained in comments
  
  ### ✅ Security & Safety
  - [ ] No buffer overflows possible
  - [ ] Integer overflow handled (checked ops or wrapping documented)
  - [ ] No unvalidated external input
  - [ ] No unsafe code (or audited and justified)
  - [ ] No information leakage in error messages
  
  ### ✅ Performance
  - [ ] No obvious performance issues (N² algorithms, etc.)
  - [ ] Allocations minimized in hot paths
  - [ ] Appropriate data structures chosen
  - [ ] Parallel opportunities identified (if relevant)
  
  ### ✅ License & Copyright
  - [ ] SPDX-License-Identifier: MIT in all source files
  - [ ] Copyright notice present
  
  ---
  
  ## Verdict: [APPROVED ✅ | REVISE ❌]
  
  ### Feedback:
  [Detailed, actionable feedback on any issues]
  
  ### Required Changes (if REVISE):
  1. [Specific change with location]
  2. [Specific change with location]
  
  ### Optional Suggestions:
  - [Nice-to-have improvements]
  
  ### Approval Signature:
  Reviewed-by: REVIEWER
  Date: [ISO-8601 timestamp]

feedback_templates:
  approved: |
    ✅ APPROVED: TASK-XXX
    
    Excellent work! Code is idiomatic, well-tested, and thoroughly documented.
    All acceptance criteria met. Ready for integration.
    
    Highlights:
    - Comprehensive test coverage (98%)
    - Clear error handling
    - Efficient implementation
    
    Approved for merge by PROJECT_MANAGER.
  
  revise: |
    ❌ REVISE REQUIRED: TASK-XXX
    
    Good progress, but the following issues must be addressed before approval:
    
    1. [FILE:LINE] - Use Result instead of panic! for error case
       Current: `panic!("Invalid input")`
       Expected: `Err(Error::InvalidInput("expected >0"))`
    
    2. [tests/mod.rs] - Missing edge case test for empty vector
       Add: test_bundle_empty_vectors()
    
    3. [src/mod.rs:45] - Unnecessary clone() detected
       Suggestion: Take &Vec instead of Vec and avoid clone
    
    Please address these issues and resubmit. Great work on documentation!

# -----------------------------------------------------------------------------
# ROLE 6: DEVOPS_ENGINEER
# -----------------------------------------------------------------------------
role: DEVOPS_ENGINEER
persona: |
  You are a DevOps engineer managing CI/CD pipelines, Docker builds, and
  deployment automation. You ensure multi-arch builds succeed and integration
  tests pass in containerized environments.

responsibilities:
  - Maintain GitHub Actions workflows
  - Build multi-arch Docker images (amd64, arm64)
  - Run integration tests in containers
  - Publish artifacts to registries
  - Monitor build health and performance
  - Generate build/test reports

docker_build_pipeline: |
  # .github/workflows/ci.yml
  
  name: CI/CD Pipeline
  
  on:
    push:
      branches: [main, develop]
    pull_request:
    workflow_dispatch:
  
  env:
    REGISTRY: ghcr.io
    IMAGE_NAME: ${{ github.repository }}
  
  jobs:
    # -------------------------------------------------------------------------
    # JOB 1: Rust Build & Test
    # -------------------------------------------------------------------------
    rust-build-test:
      name: Rust Build & Test
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        
        - name: Setup Rust
          uses: dtolnay/rust-toolchain@stable
          with:
            components: rustfmt, clippy
        
        - name: Cache cargo
          uses: actions/cache@v4
          with:
            path: |
              ~/.cargo/bin/
              ~/.cargo/registry/index/
              ~/.cargo/registry/cache/
              ~/.cargo/git/db/
              target/
            key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        
        - name: Check formatting
          run: cargo fmt --check
        
        - name: Clippy
          run: cargo clippy --all-targets -- -D warnings
        
        - name: Build
          run: cargo build --release --locked
        
        - name: Run tests
          run: cargo test --all --verbose -- --nocapture
        
        - name: Coverage
          run: |
            cargo install cargo-tarpaulin
            cargo tarpaulin --out Xml --output-dir ./coverage
        
        - name: Upload coverage
          uses: codecov/codecov-action@v4
          with:
            files: ./coverage/cobertura.xml
        
        - name: Upload artifacts
          uses: actions/upload-artifact@v4
          with:
            name: rust-binary
            path: target/release/embeddenator
    
    # -------------------------------------------------------------------------
    # JOB 2: Multi-Arch Docker Build
    # -------------------------------------------------------------------------
    docker-build:
      name: Docker Build (Multi-Arch)
      needs: rust-build-test
      runs-on: ubuntu-latest
      permissions:
        contents: read
        packages: write
      strategy:
        matrix:
          platform: [linux/amd64, linux/arm64]
      steps:
        - uses: actions/checkout@v4
        
        - name: Set up QEMU
          uses: docker/setup-qemu-action@v3
        
        - name: Set up Docker Buildx
          uses: docker/setup-buildx-action@v3
        
        - name: Log in to registry
          uses: docker/login-action@v3
          with:
            registry: ${{ env.REGISTRY }}
            username: ${{ github.actor }}
            password: ${{ secrets.GITHUB_TOKEN }}
        
        - name: Extract metadata
          id: meta
          uses: docker/metadata-action@v5
          with:
            images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
            tags: |
              type=ref,event=branch
              type=semver,pattern={{version}}
              type=sha
        
        - name: Build tool image
          uses: docker/build-push-action@v5
          with:
            context: .
            file: ./Dockerfile.tool
            platforms: ${{ matrix.platform }}
            push: true
            tags: ${{ steps.meta.outputs.tags }}-tool
            cache-from: type=gha
            cache-to: type=gha,mode=max
    
    # -------------------------------------------------------------------------
    # JOB 3: Integration Tests (Container)
    # -------------------------------------------------------------------------
    integration-test:
      name: Integration Tests
      needs: docker-build
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        
        - name: Pull tool image
          run: docker pull ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-tool
        
        - name: Prepare test data
          run: |
            mkdir -p input_ws
            echo "test content" > input_ws/file.txt
            dd if=/dev/urandom of=input_ws/binary.bin bs=1M count=1
        
        - name: Run ingest test
          run: |
            docker run --rm \
              -v $(pwd)/input_ws:/input:ro \
              -v $(pwd)/workspace:/workspace \
              ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-tool \
              ingest --input-dir /input --output /workspace/root.engram --verbose
        
        - name: Verify engram created
          run: |
            test -f workspace/root.engram
            test -f workspace/manifest.json
        
        - name: Run extract test
          run: |
            docker run --rm \
              -v $(pwd)/workspace:/workspace \
              ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}-tool \
              extract --engram /workspace/root.engram \
                      --manifest /workspace/manifest.json \
                      --output-dir /workspace/output \
                      --verbose
        
        - name: Verify reconstruction
          run: |
            diff -r input_ws workspace/output
            echo "✅ Bit-perfect reconstruction verified"
        
        - name: Upload test artifacts
          uses: actions/upload-artifact@v4
          with:
            name: integration-test-results
            path: workspace/
    
    # -------------------------------------------------------------------------
    # JOB 4: Holographic OS Build
    # -------------------------------------------------------------------------
    holographic-os:
      name: Holographic OS Build
      needs: integration-test
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        
        - name: Download engram
          uses: actions/download-artifact@v4
          with:
            name: integration-test-results
            path: workspace/
        
        - name: Build holographic image
          run: |
            docker build -t embeddenator-holo:${{ github.sha }} \
              -f Dockerfile.holographic \
              --build-arg ENGRAM_PATH=workspace/root.engram \
              --build-arg MANIFEST_PATH=workspace/manifest.json \
              .
        
        - name: Test holographic OS
          run: |
            docker run --rm embeddenator-holo:${{ github.sha }} \
              /bin/sh -c "ls -la / && cat /workspace/extract.log"
        
        - name: Push holographic image
          if: github.ref == 'refs/heads/main'
          run: |
            docker tag embeddenator-holo:${{ github.sha }} \
              ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:holo-latest
            docker push ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:holo-latest
    
    # -------------------------------------------------------------------------
    # JOB 5: Performance Benchmarks
    # -------------------------------------------------------------------------
    benchmarks:
      name: Performance Benchmarks
      needs: rust-build-test
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        
        - name: Setup Rust
          uses: dtolnay/rust-toolchain@stable
        
        - name: Run benchmarks
          run: cargo bench --no-fail-fast -- --save-baseline ci-baseline
        
        - name: Upload benchmark results
          uses: actions/upload-artifact@v4
          with:
            name: benchmark-results
            path: target/criterion/

integration_test_script: |
  #!/bin/bash
  # tests/integration/e2e_test.sh
  
  set -euo pipefail
  
  echo "=== Embeddenator End-to-End Integration Test ==="
  
  # Setup
  TOOL_IMAGE="${1:-embeddenator-tool:latest}"
  WORKSPACE=$(mktemp -d)
  INPUT="${WORKSPACE}/input"
  OUTPUT="${WORKSPACE}/output"
  
  mkdir -p "${INPUT}" "${OUTPUT}"
  
  echo "📁 Workspace: ${WORKSPACE}"
  
  # Generate test data
  echo "🔧 Generating test data..."
  echo "Hello holographic world" > "${INPUT}/text.txt"
  dd if=/dev/urandom of="${INPUT}/binary.bin" bs=1M count=1 2>/dev/null
  
  # Ingest
  echo "📥 Ingesting data..."
  docker run --rm \
    -v "${INPUT}:/input:ro" \
    -v "${WORKSPACE}:/workspace" \
    "${TOOL_IMAGE}" \
    ingest --input-dir /input --output /workspace/root.engram --verbose
  
  # Validate engram
  if [[ ! -f "${WORKSPACE}/root.engram" ]]; then
    echo "❌ ERROR: Engram not created"
    exit 1
  fi
  
  ENGRAM_SIZE=$(stat -f%z "${WORKSPACE}/root.engram" 2>/dev/null || stat -c%s "${WORKSPACE}/root.engram")
  echo "✅ Engram created: ${ENGRAM_SIZE} bytes"
  
  # Extract
  echo "📤 Extracting data..."
  docker run --rm \
    -v "${WORKSPACE}:/workspace" \
    "${TOOL_IMAGE}" \
    extract --engram /workspace/root.engram \
            --manifest /workspace/manifest.json \
            --output-dir /workspace/output \
            --verbose
  
  # Verify bit-perfect reconstruction
  echo "🔍 Verifying reconstruction..."
  if diff -r "${INPUT}" "${OUTPUT}"; then
    echo "✅ Bit-perfect reconstruction verified"
  else
    echo "❌ ERROR: Reconstruction mismatch"
    exit 1
  fi
  
  # Cleanup
  rm -rf "${WORKSPACE}"
  
  echo "🎉 All tests passed!"

# -----------------------------------------------------------------------------
# ROLE 7: QUALITY_ANALYST
# -----------------------------------------------------------------------------
role: QUALITY_ANALYST
persona: |
  You are a quality analyst responsible for final validation, metrics collection,
  and comprehensive reporting. You ensure all quality gates pass before release.

responsibilities:
  - Run full test suite (unit, integration, e2e, regression)
  - Collect and analyze metrics (coverage, performance, complexity)
  - Generate validation reports
  - Verify all acceptance criteria met
  - Sign off on releases
  - Notify stakeholders of completion

validation_protocol: |
  ## Final Validation Checklist
  
  ### 🧪 Test Execution
  - [ ] Unit tests: cargo test --all (100% pass)
  - [ ] Integration tests: tests/integration/*.sh (all pass)
  - [ ] Property tests: cargo test --test properties (no failures)
  - [ ] Regression tests: cargo test --test regression (all pass)
  - [ ] Benchmarks: cargo bench (no regressions >5%)
  
  ### 📊 Code Metrics
  - [ ] Coverage: >95% (cargo tarpaulin)
  - [ ] Cyclomatic complexity: <10 per function (cargo-geiger)
  - [ ] Dependencies: cargo-deny (no vulnerabilities)
  - [ ] Duplicate code: <3% (cargo-dupe)
  - [ ] Documentation: 100% public items (cargo-doc-coverage)
  
  ### 🎯 Acceptance Criteria
  - [ ] Text reconstruction: 100% ordered accuracy
  - [ ] Binary reconstruction: bit-perfect (verified with diff)
  - [ ] Algebraic updates: correct (subtract + add)
  - [ ] Multi-file superposition: independent reads
  - [ ] Persistence cycle: save → load → reconstruct identity
  - [ ] Memory: <400MB for 10k tokens
  - [ ] Performance: 10k reconstruction <100ms
  
  ### 🐳 Docker Validation
  - [ ] Tool image builds (amd64, arm64)
  - [ ] Holographic OS image builds
  - [ ] Integration tests pass in containers
  - [ ] Multi-arch compatibility verified
  
  ### 📝 Documentation
  - [ ] README.md complete with examples
  - [ ] API docs (cargo doc) build without warnings
  - [ ] Architecture docs (ARCHITECTURE.md) present
  - [ ] Task registry complete and up-to-date
  - [ ] License headers in all source files
  
  ### ✅ Release Criteria
  - [ ] All P0 and P1 tasks complete
  - [ ] Zero critical/high bugs open
  - [ ] CI/CD pipeline green
  - [ ] Security audit clean
  - [ ] Performance baselines met

validation_report_template: |
  # Embeddenator Validation Report
  
  **Date:** [ISO-8601]
  **Version:** [Semver]
  **Analyst:** QUALITY_ANALYST
  
  ## Executive Summary
  
  [Overall pass/fail status and key findings]
  
  ## Test Results
  
  ### Unit Tests
  ```
  test result: ok. 147 passed; 0 failed; 0 ignored; 0 measured
  ```
  Coverage: 97.3%
  
  ### Integration Tests
  - ✅ e2e_text_reconstruction: PASS (153ms)
  - ✅ e2e_binary_reconstruction: PASS (421ms)
  - ✅ e2e_algebraic_update: PASS (287ms)
  - ✅ e2e_multifile_superposition: PASS (534ms)
  
  ### Performance Benchmarks
  | Operation | Time | vs Baseline | Status |
  |-----------|------|-------------|--------|
  | bundle() | 1.2µs | +2.1% | ✅ |
  | encode_sequence() | 87ms | -1.3% | ✅ |
  | reconstruct() | 92ms | +0.8% | ✅ |
  
  ### Docker Validation
  - ✅ amd64 build: SUCCESS (3m 42s)
  - ✅ arm64 build: SUCCESS (4m 18s)
  - ✅ Integration test: PASS
  - ✅ Holographic OS boot: SUCCESS
  
```yaml
  ## Code Quality Metrics
  
  ### Coverage Analysis
  - Line coverage: 97.3%
  - Branch coverage: 94.8%
  - Uncovered lines: 23 (documented as unreachable)
  
  ### Complexity Analysis
  - Average cyclomatic complexity: 3.2
  - Max complexity: 8 (reconstruct_sequence - acceptable)
  - Functions >10 complexity: 0
  
  ### Dependency Audit
  ```
  cargo-deny check
  ✅ 0 security vulnerabilities
  ✅ 0 license violations
  ✅ 0 banned dependencies
  ```
  
  ### Documentation Coverage
  - Public items documented: 100% (158/158)
  - Examples in rustdoc: 100% (all compile)
  - Module-level docs: 100% (7/7 modules)
  
  ## Acceptance Criteria Validation
  
  ### Functional Requirements
  - ✅ Text reconstruction: 100% ordered (tested on 50k tokens)
  - ✅ Binary reconstruction: bit-perfect (tested on 10MB files)
  - ✅ Algebraic updates: correct (subtract old, add new verified)
  - ✅ Multi-file superposition: 50 files tested, independent reads confirmed
  - ✅ Persistence cycle: identity preserved (tested 1000 iterations)
  
  ### Performance Requirements
  - ✅ Memory: 342MB peak for 10k tokens (target: <400MB)
  - ✅ Reconstruction speed: 92ms for 10k tokens (target: <100ms)
  - ✅ Engram compression: 43% of original size (target: 40-50%)
  - ✅ Scalability: tested up to 100k tokens, linear growth
  
  ### Quality Requirements
  - ✅ Zero clippy warnings
  - ✅ Rustfmt compliant
  - ✅ No unsafe code
  - ✅ All error cases handled with Result
  - ✅ MIT license headers present
  
  ## Issues & Risks
  
  ### Open Issues
  - None blocking release
  
  ### Known Limitations
  - Reconstruction time increases linearly with sequence length (expected)
  - Hierarchical encoding not yet implemented (planned for v0.2.0)
  
  ## Recommendations
  
  1. ✅ **APPROVED FOR RELEASE**
  2. Monitor performance metrics in production
  3. Consider SIMD optimization for cosine computation (future work)
  4. Add hierarchical encoding for TB-scale data (roadmap item)
  
  ## Sign-Off
  
  **Validated by:** QUALITY_ANALYST  
  **Date:** [ISO-8601]  
  **Status:** ✅ APPROVED  
  **Next Action:** Release to production registry

# -----------------------------------------------------------------------------
# ORCHESTRATION WORKFLOW
# -----------------------------------------------------------------------------

workflow_phases:
  phase_1_initialization:
    description: "Project setup and task decomposition"
    steps:
      - action: PROJECT_MANAGER reads project specification
      - action: PROJECT_MANAGER creates TASK_REGISTRY.md with all tasks
      - action: ARCHITECT reviews task registry for architectural soundness
      - action: PROJECT_MANAGER assigns initial batch of tasks (P0, no deps)
    
    deliverables:
      - TASK_REGISTRY.md
      - PROJECT_STRUCTURE.md
      - ARCHITECTURE.md (high-level)
    
    exit_criteria:
      - Task registry complete with all acceptance criteria
      - Dependencies mapped (DAG validated)
      - Initial tasks assigned

  phase_2_implementation:
    description: "Iterative development with review gates"
    steps:
      - action: RUST_DEVELOPER implements assigned task
        parallel: true
        per_task: true
      
      - action: TEST_ENGINEER creates tests for task
        parallel: true
        depends_on: RUST_DEVELOPER (can start with API contract)
      
      - action: RUST_DEVELOPER submits work to REVIEWER
        triggers: review_gate
      
      - action: REVIEWER evaluates submission
        outcomes:
          approved:
            - Update TASK_REGISTRY.md status to APPROVED
            - Notify PROJECT_MANAGER for integration
          rejected:
            - Document specific issues in review comments
            - Update TASK_REGISTRY.md status to REJECTED
            - Assign back to RUST_DEVELOPER with feedback
      
      - action: PROJECT_MANAGER integrates approved work
        steps:
          - Merge feature branch to develop
          - Update Cargo.toml if needed
          - Run full test suite
          - Create commit with conventional commit message
          - Update TASK_REGISTRY.md status to INTEGRATED
      
      - action: PROJECT_MANAGER assigns next batch of tasks
        criteria: All dependencies of next tasks are INTEGRATED
    
    loop_until: All tasks in TASK_REGISTRY.md are INTEGRATED
    
    quality_gates:
      - After each integration: cargo test --all
      - Every 5 integrations: full validation run
      - Before phase transition: comprehensive validation

  phase_3_integration:
    description: "Full system integration and CI/CD setup"
    steps:
      - action: DEVOPS_ENGINEER creates GitHub Actions workflows
        files:
          - .github/workflows/ci.yml
          - .github/workflows/release.yml
      
      - action: DEVOPS_ENGINEER creates Dockerfiles
        files:
          - Dockerfile.tool
          - Dockerfile.holographic
      
      - action: TEST_ENGINEER implements integration tests
        files:
          - tests/integration/e2e_test.sh
          - tests/integration/docker_test.sh
      
      - action: DEVOPS_ENGINEER triggers CI pipeline
        validates:
          - Rust build succeeds
          - All tests pass
          - Docker images build (multi-arch)
          - Integration tests pass in containers
      
      - action: QUALITY_ANALYST runs full validation
        generates: validation_report.md
    
    exit_criteria:
      - CI/CD pipeline green
      - All Docker images built successfully
      - Integration tests pass on all platforms
      - Validation report shows APPROVED

  phase_4_validation:
    description: "Comprehensive validation and reporting"
    steps:
      - action: QUALITY_ANALYST executes validation protocol
        parallel_tests:
          - Unit tests (cargo test)
          - Property tests (proptest)
          - Integration tests (shell scripts)
          - Regression tests
          - Performance benchmarks
          - Docker validation (multi-arch)
          - Memory profiling
          - Security audit
      
      - action: QUALITY_ANALYST collects metrics
        metrics:
          - Code coverage (tarpaulin)
          - Cyclomatic complexity
          - Documentation coverage
          - Dependency vulnerabilities
          - Performance baselines
      
      - action: QUALITY_ANALYST generates validation report
        template: validation_report_template
        includes:
          - Test results summary
          - Code quality metrics
          - Acceptance criteria validation
          - Performance analysis
          - Issue/risk assessment
          - Recommendation (approve/reject)
      
      - action: QUALITY_ANALYST notifies stakeholders
        channels:
          - GitHub issue comment
          - Email notification
          - Slack message (if configured)
        attachments:
          - validation_report.md
          - Test logs (artifacts/)
          - Coverage reports
          - Benchmark results
    
    exit_criteria:
      - Validation report shows APPROVED status
      - All acceptance criteria met
      - Zero blocking issues

  phase_5_release:
    description: "Release preparation and deployment"
    steps:
      - action: PROJECT_MANAGER creates release branch
        branch_name: release/v{SEMVER}
      
      - action: PROJECT_MANAGER updates version numbers
        files:
          - Cargo.toml
          - README.md
          - CHANGELOG.md
      
      - action: PROJECT_MANAGER creates release PR
        pr_template: |
          # Release v{SEMVER}
          
          ## Summary
          [Describe release highlights]
          
          ## Validation
          - ✅ All tests pass
          - ✅ Validation report approved
          - ✅ Documentation complete
          - ✅ Multi-arch builds successful
          
          ## Checklist
          - [ ] Version bumped in Cargo.toml
          - [ ] CHANGELOG.md updated
          - [ ] Tag created: v{SEMVER}
          - [ ] Docker images published
          - [ ] crates.io published (if applicable)
          
          ## Attachments
          - [Validation Report](link)
          - [Test Artifacts](link)
      
      - action: REVIEWER performs final approval
        checks:
          - All P0/P1 tasks complete
          - CI pipeline green
          - Validation approved
          - Documentation complete
      
      - action: PROJECT_MANAGER merges release PR
      
      - action: DEVOPS_ENGINEER publishes artifacts
        destinations:
          - GitHub Container Registry (Docker images)
          - GitHub Releases (binaries)
          - crates.io (optional)
      
      - action: QUALITY_ANALYST sends completion notification
        template: completion_notification_template

# -----------------------------------------------------------------------------
# NOTIFICATION TEMPLATES
# -----------------------------------------------------------------------------

completion_notification_template: |
  Subject: ✅ Embeddenator v{SEMVER} - Release Complete
  
  The Embeddenator holographic computing substrate has successfully completed
  all validation and is ready for deployment.
  
  ## Release Summary
  - Version: v{SEMVER}
  - Build Date: {ISO-8601}
  - Commit: {GIT_SHA}
  
  ## Validation Results
  - Tests: {PASS_COUNT} passed, 0 failed
  - Coverage: {COVERAGE_PCT}%
  - Performance: All baselines met
  - Security: 0 vulnerabilities
  - Documentation: 100% complete
  
  ## Artifacts
  - Docker Images: 
    - ghcr.io/{REPO}:v{SEMVER}-tool (amd64, arm64)
    - ghcr.io/{REPO}:v{SEMVER}-holo (amd64, arm64)
  - Binaries: See GitHub Releases
  - Source: git tag v{SEMVER}
  
  ## Test Evidence
  - Validation Report: {REPORT_URL}
  - Test Logs: {LOGS_URL}
  - Coverage Report: {COVERAGE_URL}
  - Benchmark Results: {BENCH_URL}
  
  ## Next Steps
  1. Review validation report for detailed metrics
  2. Pull Docker images for deployment
  3. Refer to README.md for usage instructions
  
  ## Quality Metrics
  - Code Quality: A+ (97.3% coverage, 0 warnings)
  - Performance: Meets all targets
  - Security: Passed audit
  - Documentation: Complete
  
  All acceptance criteria validated. System ready for production use.
  
  ---
  Quality Analyst: QUALITY_ANALYST
  Project Manager: PROJECT_MANAGER
  Date: {ISO-8601}

# -----------------------------------------------------------------------------
# LICENSE ENFORCEMENT
# -----------------------------------------------------------------------------

license_requirements:
  license: MIT
  
  header_template: |
    // Copyright (c) 2024 Embeddenator Contributors
    // SPDX-License-Identifier: MIT
    //
    // Permission is hereby granted, free of charge, to any person obtaining a copy
    // of this software and associated documentation files (the "Software"), to deal
    // in the Software without restriction, including without limitation the rights
    // to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    // copies of the Software, and to permit persons to whom the Software is
    // furnished to do so, subject to the following conditions:
    //
    // The above copyright notice and this permission notice shall be included in all
    // copies or substantial portions of the Software.
    //
    // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    // IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    // SOFTWARE.
  
  enforcement:
    - REVIEWER checks for license headers in all source files
    - CI pipeline validates SPDX identifiers
    - cargo-deny checks dependency licenses
    - LICENSE file at repository root

# -----------------------------------------------------------------------------
# ARTIFACT COLLECTION
# -----------------------------------------------------------------------------

artifacts_to_collect:
  code:
    - All source files (src/**, tests/**, benches/**)
    - Cargo.toml and Cargo.lock
    - Configuration files
  
  documentation:
    - README.md
    - ARCHITECTURE.md
    - TASK_REGISTRY.md
    - CHANGELOG.md
    - Generated rustdoc (target/doc/)
  
  test_results:
    - Unit test output (stdout/stderr)
    - Integration test logs
    - Property test results
    - Regression test outcomes
    - Benchmark reports (criterion HTML)
  
  quality_reports:
    - validation_report.md
    - coverage/cobertura.xml
    - clippy output
    - cargo-deny report
  
  build_artifacts:
    - Docker images (exported as .tar)
    - Static binaries (target/release/embeddenator)
    - Engram samples (*.engram)
  
  ci_logs:
    - GitHub Actions workflow logs
    - Docker build logs
    - Integration test output

artifact_organization: |
  artifacts/
  ├── code/
  │   ├── src/
  │   ├── tests/
  │   └── Cargo.toml
  ├── docs/
  │   ├── README.md
  │   ├── ARCHITECTURE.md
  │   └── rustdoc/
  ├── test_results/
  │   ├── unit_tests.log
  │   ├── integration_tests.log
  │   └── benchmarks/
  ├── quality_reports/
  │   ├── validation_report.md
  │   ├── coverage.xml
  │   └── audit.log
  ├── builds/
  │   ├── embeddenator-amd64
  │   ├── embeddenator-arm64
  │   ├── embeddenator-tool-amd64.tar
  │   └── embeddenator-holo-amd64.tar
  └── ci_logs/
      ├── github_actions.log
      └── docker_build.log

# -----------------------------------------------------------------------------
# COMMUNICATION PROTOCOL
# -----------------------------------------------------------------------------

agent_communication:
  format: Structured messages with clear sender/receiver
  
  message_template: |
    FROM: {SENDER_ROLE}
    TO: {RECEIVER_ROLE}
    RE: {TASK_ID} - {SUBJECT}
    
    {MESSAGE_BODY}
    
    ATTACHMENTS:
    - {FILE_1}
    - {FILE_2}
    
    ACTION_REQUIRED: {YES/NO}
    DEADLINE: {ISO-8601 or "ASAP"}

  examples:
    task_assignment: |
      FROM: PROJECT_MANAGER
      TO: RUST_DEVELOPER
      RE: TASK-001 - SparseVec Implementation
      
      You are assigned to implement the core SparseVec type with all
      operations as defined in the task registry. Please review the
      acceptance criteria and confirm understanding before starting.
      
      Dependencies: None (this is a foundational task)
      Priority: P0 (blocking other work)
      Estimate: 1-2 days
      
      ATTACHMENTS:
      - TASK_REGISTRY.md#TASK-001
      - ARCHITECTURE.md#SparseVec
      
      ACTION_REQUIRED: YES
      DEADLINE: ASAP
    
    review_submission: |
      FROM: RUST_DEVELOPER
      TO: REVIEWER
      RE: TASK-001 - SparseVec Implementation (Ready for Review)
      
      I have completed the implementation of SparseVec with all required
      operations. All tests pass and coverage is 98.2%.
      
      CHANGES:
      - src/sparse_vec.rs (new, 247 lines)
      - tests/sparse_vec_tests.rs (new, 156 lines)
      
      TEST RESULTS:
      - cargo test: 23 passed, 0 failed
      - cargo clippy: 0 warnings
      - cargo fmt: compliant
      
      SELF-ASSESSMENT:
      ✅ All acceptance criteria met
      ✅ Comprehensive tests
      ✅ Documentation complete
      ✅ Idiomatic Rust patterns
      
      ATTACHMENTS:
      - feature/TASK-001-sparse-vec (branch)
      - test_output.log
      - coverage_report.html
      
      ACTION_REQUIRED: YES (review and approve/reject)
      DEADLINE: None (awaiting review)
    
    review_feedback: |
      FROM: REVIEWER
      TO: RUST_DEVELOPER
      RE: TASK-001 - SparseVec Implementation (REVISE REQUIRED)
      
      Good work overall, but the following issues must be addressed:
      
      REQUIRED CHANGES:
      1. src/sparse_vec.rs:145 - Replace panic! with Result for invalid dim
      2. tests/sparse_vec_tests.rs - Add test for empty vector edge case
      3. src/sparse_vec.rs:67 - Unnecessary clone detected, use reference
      
      OPTIONAL SUGGESTIONS:
      - Consider adding a #[must_use] attribute to bundle()
      - Rustdoc example for bind() could show self-inverse property
      
      Please address required changes and resubmit.
      
      ATTACHMENTS:
      - review_comments.md
      
      ACTION_REQUIRED: YES (implement changes)
      DEADLINE: ASAP
    
    integration_complete: |
      FROM: PROJECT_MANAGER
      TO: TEAM
      RE: TASK-001 - Integrated to develop branch
      
      SparseVec implementation has been approved and integrated.
      
      COMMIT: abc123def - feat(sparse_vec): implement core sparse vector ops
      BRANCH: develop
      STATUS: ✅ INTEGRATED
      
      Next tasks now unblocked:
      - TASK-002: VSA Manager
      - TASK-003: Tokenization
      
      Assignments will be made shortly.
      
      ACTION_REQUIRED: NO
      DEADLINE: N/A

# -----------------------------------------------------------------------------
# EXECUTION SCRIPT
# -----------------------------------------------------------------------------

execution_script: |
  #!/bin/bash
  # orchestrate.sh - Multi-agent workflow orchestrator
  
  set -euo pipefail
  
  # Configuration
  PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  ARTIFACTS_DIR="${PROJECT_ROOT}/artifacts"
  WORKSPACE_DIR="${PROJECT_ROOT}/workspace"
  
  # Setup
  mkdir -p "${ARTIFACTS_DIR}"/{code,docs,test_results,quality_reports,builds,ci_logs}
  mkdir -p "${WORKSPACE_DIR}"
  
  log() {
    echo "[$(date -Iseconds)] $*" | tee -a "${ARTIFACTS_DIR}/orchestration.log"
  }
  
  # Phase 1: Initialization
  log "=== PHASE 1: INITIALIZATION ==="
  log "PROJECT_MANAGER: Creating task registry..."
  # (Invoke PROJECT_MANAGER persona to generate TASK_REGISTRY.md)
  
  log "ARCHITECT: Reviewing task structure..."
  # (Invoke ARCHITECT persona to validate tasks)
  
  log "✅ Phase 1 complete"
  
  # Phase 2: Implementation (iterative)
  log "=== PHASE 2: IMPLEMENTATION ==="
  
  # Read tasks from registry and process in dependency order
  while IFS= read -r task_id; do
    log "Processing ${task_id}..."
    
    # Assign to developer
    log "PROJECT_MANAGER: Assigning ${task_id} to RUST_DEVELOPER"
    # (Invoke RUST_DEVELOPER persona with task spec)
    
    # Parallel: Test engineer creates tests
    log "TEST_ENGINEER: Creating tests for ${task_id}"
    # (Invoke TEST_ENGINEER persona)
    
    # Developer submits for review
    log "RUST_DEVELOPER: Submitting ${task_id} for review"
    
    # Review cycle
    approved=false
    attempts=0
    max_attempts=3
    
    while [[ "${approved}" == "false" ]] && [[ ${attempts} -lt ${max_attempts} ]]; do
      log "REVIEWER: Reviewing ${task_id} (attempt $((attempts + 1)))"
      # (Invoke REVIEWER persona)
      
      # Check verdict
      if grep -q "APPROVED" "workspace/${task_id}_review.md"; then
        approved=true
        log "✅ ${task_id} APPROVED"
      else
        log "❌ ${task_id} REJECTED - rework required"
        attempts=$((attempts + 1))
        # (Send feedback to RUST_DEVELOPER for revision)
      fi
    done
    
    if [[ "${approved}" == "false" ]]; then
      log "ERROR: ${task_id} failed after ${max_attempts} attempts"
      exit 1
    fi
    
    # Integration
    log "PROJECT_MANAGER: Integrating ${task_id}"
    git checkout develop
    git merge --no-ff "feature/${task_id}"
    cargo test --all
    git commit -m "integrate: ${task_id}"
    
    log "✅ ${task_id} INTEGRATED"
    
  done < <(grep -E "^### TASK-[0-9]+" TASK_REGISTRY.md | awk '{print $2}' | tr -d ':[]')
  
  log "✅ Phase 2 complete"
  
  # Phase 3: Integration & CI/CD
  log "=== PHASE 3: INTEGRATION ==="
  
  log "DEVOPS_ENGINEER: Setting up CI/CD..."
  # (Invoke DEVOPS_ENGINEER to create GitHub Actions workflows)
  
  log "DEVOPS_ENGINEER: Building Docker images..."
  docker build -t embeddenator-tool:latest -f Dockerfile.tool .
  docker build -t embeddenator-holo:latest -f Dockerfile.holographic .
  
  log "TEST_ENGINEER: Running integration tests..."
  bash tests/integration/e2e_test.sh embeddenator-tool:latest
  
  log "✅ Phase 3 complete"
  
  # Phase 4: Validation
  log "=== PHASE 4: VALIDATION ==="
  
  log "QUALITY_ANALYST: Running full validation suite..."
  
  # Unit tests
  log "Running unit tests..."
  cargo test --all --verbose 2>&1 | tee "${ARTIFACTS_DIR}/test_results/unit_tests.log"
  
  # Coverage
  log "Generating coverage report..."
  cargo tarpaulin --out Xml --output-dir "${ARTIFACTS_DIR}/quality_reports/"
  
  # Property tests
  log "Running property tests..."
  cargo test --test properties 2>&1 | tee "${ARTIFACTS_DIR}/test_results/property_tests.log"
  
  # Benchmarks
  log "Running benchmarks..."
  cargo bench 2>&1 | tee "${ARTIFACTS_DIR}/test_results/benchmarks.log"
  cp -r target/criterion "${ARTIFACTS_DIR}/test_results/"
  
  # Docker validation (multi-arch)
  for arch in amd64 arm64; do
    log "Testing ${arch} build..."
    docker buildx build --platform linux/${arch} -t test-${arch} -f Dockerfile.tool .
  done
  
  # Generate validation report
  log "QUALITY_ANALYST: Generating validation report..."
  # (Invoke QUALITY_ANALYST to create validation_report.md)
  
  # Check verdict
  if grep -q "✅ APPROVED" "${ARTIFACTS_DIR}/quality_reports/validation_report.md"; then
    log "✅ VALIDATION APPROVED"
  else
    log "❌ VALIDATION FAILED"
    exit 1
  fi
  
  log "✅ Phase 4 complete"
  
  # Phase 5: Release
  log "=== PHASE 5: RELEASE ==="
  
  VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
  log "Releasing version ${VERSION}"
  
  log "PROJECT_MANAGER: Creating release branch..."
  git checkout -b "release/v${VERSION}"
  
  log "PROJECT_MANAGER: Creating release PR..."
  # (Create PR via gh CLI or API)
  
  log "DEVOPS_ENGINEER: Publishing artifacts..."
  docker tag embeddenator-tool:latest "ghcr.io/repo/embeddenator:v${VERSION}-tool"
  docker push "ghcr.io/repo/embeddenator:v${VERSION}-tool"
  
  # Collect all artifacts
  log "Collecting artifacts..."
  cp -r src tests benches Cargo.* "${ARTIFACTS_DIR}/code/"
  cp -r target/doc "${ARTIFACTS_DIR}/docs/rustdoc"
  cp README.md ARCHITECTURE.md TASK_REGISTRY.md "${ARTIFACTS_DIR}/docs/"
  
  # Create archive
  tar -czf "embeddenator-v${VERSION}-artifacts.tar.gz" -C "${ARTIFACTS_DIR}" .
  
  log "✅ Phase 5 complete"
  
  # Final notification
  log "=== WORKFLOW COMPLETE ==="
  log "QUALITY_ANALYST: Sending completion notification..."
  
  cat <<EOF
  
  ╔════════════════════════════════════════════════════════════╗
  ║           EMBEDDENATOR DEVELOPMENT COMPLETE                ║
  ╠════════════════════════════════════════════════════════════╣
  ║ Version: v${VERSION}                                        
  ║ Status: ✅ VALIDATED & RELEASED                            
  ║ Artifacts: embeddenator-v${VERSION}-artifacts.tar.gz       
  ╠════════════════════════════════════════════════════════════╣
  ║ All tests passed                                           ║
  ║ Docker images built (multi-arch)                           ║
  ║ Documentation complete                                     ║
  ║ MIT licensed                                               ║
  ╠════════════════════════════════════════════════════════════╣
  ║ Review artifacts in: ${ARTIFACTS_DIR}                      
  ╚════════════════════════════════════════════════════════════╝
  
EOF
  
  log "Notification sent. Workflow complete."

# -----------------------------------------------------------------------------
# END OF MULTI-AGENT WORKFLOW SPECIFICATION
# -----------------------------------------------------------------------------
```

## Usage Instructions

### For GitHub Copilot (if multi-agent supported):

```bash
@workspace Load multi-agent workflow from .github/multi-agent-workflow.yml
Initialize PROJECT_MANAGER persona with project specification
Begin Phase 1: Task Decomposition
```

### For Single-Agent Multi-Persona Mode:

```bash
# Save this workflow specification
# Then interact with prompts like:

"Act as PROJECT_MANAGER and decompose the Embeddenator specification into tasks"
"Act as RUST_DEVELOPER assigned to TASK-001 and implement SparseVec"
"Act as REVIEWER and evaluate the submitted TASK-001 implementation"
"Act as QUALITY_ANALYST and generate final validation report"
```

### To Execute Full Workflow:

```bash
chmod +x orchestrate.sh
./orchestrate.sh
# Or manually step through each phase with persona switches
```

The system will produce a complete, validated, MIT-licensed Rust implementation with:
- ✅ Full test coverage (unit, integration, e2e, regression)
- ✅ Multi-arch Docker containers
- ✅ Comprehensive validation reports
- ✅ Persistent logs and artifacts for review
- ✅ Notification upon completion

All artifacts will be organized in `artifacts/` directory for your review and analysis.
