# Embeddenator — Holographic Computing Substrate

> **⚠️ EARLY DEVELOPMENT:** This project is in active development (v0.20.0-alpha). APIs are unstable and subject to change. Not recommended for production use.

**Version 0.20.0-alpha** | Experimental Rust implementation of sparse ternary Vector Symbolic Architecture (VSA) for holographic data encoding.

**Author:** Tyler Zervas <tz-dev@vectorweight.com>  
**License:** MIT (see [LICENSE](LICENSE) file)  

[![CI](https://github.com/tzervas/embeddenator/workflows/CI/badge.svg)](https://github.com/tzervas/embeddenator/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Component Architecture

Embeddenator has been refactored into a **modular component architecture** with 6 independent library crates:

- **[embeddenator-vsa](https://github.com/tzervas/embeddenator-vsa)** - Sparse ternary VSA primitives
- **[embeddenator-io](https://github.com/tzervas/embeddenator-io)** - Codebook, manifest, engram I/O
- **[embeddenator-retrieval](https://github.com/tzervas/embeddenator-retrieval)** - Query engine with shift-sweep search
- **[embeddenator-fs](https://github.com/tzervas/embeddenator-fs)** - FUSE filesystem integration
- **[embeddenator-interop](https://github.com/tzervas/embeddenator-interop)** - Python/FFI bindings
- **[embeddenator-obs](https://github.com/tzervas/embeddenator-obs)** - Observability and metrics

**📚 Documentation:** [Component Architecture](docs/COMPONENT_ARCHITECTURE.md) | [Local Development](docs/LOCAL_DEVELOPMENT.md) | [Versioning](docs/VERSIONING.md)

**🐳 Docker:** Multi-arch images available at `ghcr.io/tzervas/embeddenator` ([amd64](https://github.com/tzervas/embeddenator/pkgs/container/embeddenator) + arm64)

## Current Capabilities

### Implemented Features
- **Engram Encoding/Decoding**: Create holographic encodings (`.engram` files) of filesystems
- **Bit-Perfect Reconstruction**: Verified reconstruction of text and binary files from engrams
- **VSA Operations**: Bundle, bind, and other vector symbolic operations on sparse ternary vectors
- **Hierarchical Encoding**: Multi-level chunking for handling larger datasets
- **SIMD Support**: Optional AVX2/NEON optimizations (experimental, 2-4x speedup on supported hardware)
- **CLI Tool**: Command-line interface for ingest, extract, and query operations
- **Component Architecture**: Modular design with 6 independent library crates
- **Test Coverage**: 160+ integration tests covering core functionality (97.6% pass rate)

### Experimental/In Development
- **FUSE Filesystem**: EmbrFS integration (partial implementation)
- **Query Performance**: Similarity search and retrieval (basic implementation)
- **Docker Support**: Multi-arch containers (in development)
- **Large-Scale Testing**: TB-scale validation (planned)
- **OS Container Encoding**: Full system encoding (proof-of-concept only)

## What's New in v0.20.0-alpha

- 🎯 **Deterministic hierarchical artifacts** - Stable manifest/sub-engram generation with sorted iteration
- 📊 **Optional node sharding** - `--max-chunks-per-node` cap for bounded per-node indexing cost
- 📂 **Multi-input ingest** - Ingest files and/or multiple directories with automatic namespacing
- ⚡ **Query performance** - Reusable codebook index across shift-sweep + increased candidate pool
- 🧪 **Expanded test coverage** - New determinism and E2E hierarchical artifact tests
- 📚 **Updated documentation** - CLI reference, hierarchical format, and selective unfolding guides

## What's New in v0.2.0

- ✨ **6 comprehensive E2E regression tests** including critical engram modification test
- 🧪 **Comprehensive test suite** (unit + integration + e2e + doc tests)
- 🔍 **Intelligent test runner** with accurate counting and debug mode
- 📦 **Dual versioning strategy** for OS builds (LTS + nightly)
- 🎯 **Zero clippy warnings** (29 fixes applied)
- 🐧 **Extended OS support**: Debian 12 LTS, Debian Testing/Sid, Ubuntu 24.04 LTS, Ubuntu Devel/Rolling
- 🚀 **Native amd64 CI** (required pre-merge check) + arm64 ready for self-hosted runners
- 📚 **Automated documentation** with rustdoc and 9 doc tests

## Core Concepts

### Vector Symbolic Architecture (VSA)

Embeddenator uses sparse ternary vectors to represent data holographically:

- **Bundle (⊕)**: Superposition operation for combining vectors
- **Bind (⊙)**: Compositional operation with approximate self-inverse property
- **Cosine Similarity**: Measure of vector similarity for retrieval

The ternary representation {-1, 0, +1} enables efficient computation:
- 39-40 trits can be encoded in a 64-bit register
- Sparse representation reduces memory and computation requirements
- Based on balanced ternary arithmetic

**Current Configuration**:
- 10,000 dimensions with ~1% sparsity (~100-200 non-zero elements per vector)
- Provides balance between collision resistance and computational efficiency
- Higher dimensions and sparsity configurations are under investigation

### Engrams

An **engram** is a holographic encoding of an entire filesystem or dataset:

- Single root vector containing superposition of all chunks
- Secure codebook with VSA-lens encoded data (not plaintext)
- Manifest tracking file structure and metadata

**Data Encoding**: The codebook stores encoded vector representations of data chunks. The encoding mechanism:
- Requires the codebook for reconstruction (codebook acts as a key)
- Uses sparse ternary vectors for holographic superposition
- Supports deterministic encoding and decoding
- **Security Note**: The cryptographic properties of this encoding are under research. Do not use for security-critical applications.

See [ADR-007](docs/adr/ADR-007-codebook-security.md) for details on the encoding model.

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/tzervas/embeddenator.git
cd embeddenator

# Build with Cargo
cargo build --release

# Or use the orchestrator
python3 orchestrator.py --mode build --verbose
```

### Basic Usage

```bash
# Ingest a directory into an engram
cargo run --release -- ingest -i ./input_ws -e root.engram -m manifest.json -v

# Extract from an engram
cargo run --release -- extract -e root.engram -m manifest.json -o ./output -v

# Query similarity
cargo run --release -- query -e root.engram -q ./test_file.txt -v
```

### Using the Orchestrator

The orchestrator provides unified build, test, and deployment workflows:

```bash
# Quick start: build, test, and package everything
python3 orchestrator.py --mode full --verbose -i

# Run integration tests
python3 orchestrator.py --mode test --verbose

# Build Docker image
python3 orchestrator.py --mode package --verbose

# Display system info
python3 orchestrator.py --mode info

# Clean all artifacts
python3 orchestrator.py --mode clean
```

## CLI Reference

Embeddenator provides the following commands for working with holographic engrams:

### `embeddenator --help`

Get comprehensive help information:

```bash
# Show main help with examples
embeddenator --help

# Show detailed help for a specific command
embeddenator ingest --help
embeddenator extract --help
embeddenator query --help
embeddenator query-text --help
embeddenator bundle-hier --help
```

### `ingest` - Create Holographic Engram

Process one or more files and/or directories and encode them into a holographic engram.

```bash
embeddenator ingest [OPTIONS] --input <PATH>...

Required:
  -i, --input <PATH>...   Input file(s) and/or directory(ies) to ingest

Options:
  -e, --engram <FILE>     Output engram file [default: root.engram]
  -m, --manifest <FILE>   Output manifest file [default: manifest.json]
  -v, --verbose           Enable verbose output with progress and statistics
  -h, --help             Print help information

Examples:
  # Basic ingestion
  embeddenator ingest -i ./myproject -e project.engram -m project.json

  # Mix files and directories (repeat -i/--input)
  embeddenator ingest -i ./src -i ./README.md -e project.engram -m project.json

  # With verbose output
  embeddenator ingest -i ~/Documents -e docs.engram -v

  # Custom filenames
  embeddenator ingest --input ./data --engram backup.engram --manifest backup.json
```

**What it does:**
- Recursively scans any input directories
- Ingests any input files directly
- Chunks files (4KB default)
- Encodes chunks using sparse ternary VSA
- Creates holographic superposition in root vector
- Saves engram (holographic data) and manifest (metadata)

### `extract` - Reconstruct Files

Bit-perfect reconstruction of all files from an engram.

```bash
embeddenator extract [OPTIONS] --output-dir <DIR>

Required:
  -o, --output-dir <DIR>  Output directory for reconstructed files

Options:
  -e, --engram <FILE>     Input engram file [default: root.engram]
  -m, --manifest <FILE>   Input manifest file [default: manifest.json]
  -v, --verbose           Enable verbose output with progress
  -h, --help             Print help information

Examples:
  # Basic extraction
  embeddenator extract -e project.engram -m project.json -o ./restored

  # With default filenames
  embeddenator extract -o ./output -v

  # From backup
  embeddenator extract --engram backup.engram --manifest backup.json --output-dir ~/restored
```

**What it does:**
- Loads engram and manifest
- Reconstructs directory structure
- Algebraically unbinds chunks from root vector
- Writes bit-perfect copies of all files
- Preserves file hierarchy and metadata

### `query` - Similarity Search

Compute cosine similarity between a query file and engram contents.

```bash
embeddenator query [OPTIONS] --query <FILE>

Required:
  -q, --query <FILE>      Query file or pattern to search for

Options:
  -e, --engram <FILE>     Engram file to query [default: root.engram]
  --hierarchical-manifest <FILE>  Optional hierarchical manifest (selective unfolding)
  --sub-engrams-dir <DIR>         Directory of `.subengram` files (used with --hierarchical-manifest)
  --k <K>              Top-k results to print for codebook/hierarchical search [default: 10]
  -v, --verbose           Enable verbose output with similarity details
  -h, --help             Print help information

Examples:
  # Query similarity
  embeddenator query -e archive.engram -q search.txt

  # With verbose output
  embeddenator query -e data.engram -q pattern.bin -v

  # Using default engram
  embeddenator query --query testfile.txt -v
```

**What it does:**
- Encodes query file using VSA
- Computes cosine similarity with engram
- Returns similarity score

If `--hierarchical-manifest` and `--sub-engrams-dir` are provided, it also runs a store-backed hierarchical query and prints the top hierarchical matches.

**Similarity interpretation:**
- **>0.75**: Strong match, likely contains similar content
- **0.3-0.75**: Moderate similarity, some shared patterns  
- **<0.3**: Low similarity, likely unrelated content

### `query-text` - Similarity Search (Text)

Encode a literal text string as a query vector and run the same retrieval path as `query`.

```bash
embeddenator query-text -e root.engram --text "search phrase" --k 10

# With hierarchical selective unfolding:
embeddenator query-text -e root.engram --text "search phrase" \
  --hierarchical-manifest hier.json --sub-engrams-dir ./sub_engrams --k 10
```

### `bundle-hier` - Build Hierarchical Retrieval Artifacts

Build a hierarchical manifest and a directory of sub-engrams from an existing flat `root.engram` + `manifest.json`. This enables store-backed selective unfolding queries.

```bash
embeddenator bundle-hier -e root.engram -m manifest.json \
  --out-hierarchical-manifest hier.json \
  --out-sub-engrams-dir ./sub_engrams

# Optional: deterministically shard large nodes (bounds per-node indexing cost)
embeddenator bundle-hier -e root.engram -m manifest.json \
  --max-chunks-per-node 2000 \
  --out-hierarchical-manifest hier.json \
  --out-sub-engrams-dir ./sub_engrams
```

## Docker Usage (Experimental)

> **Note:** Docker support is in development and may not be fully functional.

### Build Tool Image

```bash
docker build -f Dockerfile.tool -t embeddenator-tool:latest .
```

### Run in Container

```bash
# Ingest data
docker run -v $(pwd)/input_ws:/input -v $(pwd)/workspace:/workspace \
  embeddenator-tool:latest \
  ingest -i /input -e /workspace/root.engram -m /workspace/manifest.json -v

# Extract data
docker run -v $(pwd)/workspace:/workspace -v $(pwd)/output:/output \
  embeddenator-tool:latest \
  extract -e /workspace/root.engram -m /workspace/manifest.json -o /output -v
```

## Test Coverage

Embeddenator has comprehensive test coverage:

- **160+ integration tests** across 23 test suites
- **97.6% pass rate** (166/170 tests passing)
- **Test categories**: Balanced ternary, codebook operations, VSA properties, error recovery, hierarchical operations, CLI integration
- **Continuous testing**: All core functionality verified with each build

### Verified Capabilities

- ✅ **Text file reconstruction**: Byte-for-byte identical reconstruction verified
- ✅ **Binary file recovery**: Exact binary reconstruction tested
- ✅ **VSA operations**: Bundle, bind, and similarity operations tested
- ✅ **Hierarchical encoding**: Multi-level chunking verified
- ✅ **Error recovery**: Corruption and concurrency handling tested

### In Development

- ⚠️ **Large-scale testing**: TB-scale datasets not yet fully validated
- ⚠️ **Performance optimization**: Benchmarking and tuning ongoing
- ⚠️ **Security audit**: Cryptographic properties under research

## Architecture

### Core Components

1. **SparseVec**: Sparse ternary vector implementation
   - `pos`: Indices with +1 value
   - `neg`: Indices with -1 value
   - Efficient operations: bundle, bind, cosine similarity
   - Hardware-optimized: 39-40 trits per 64-bit register

2. **EmbrFS**: Holographic filesystem layer
   - Chunked encoding (4KB default)
   - Manifest for file metadata
   - Codebook for chunk storage

3. **CLI**: Command-line interface
   - Ingest: directory → engram
   - Extract: engram → directory
   - Query: similarity search

### Architecture Decision Records (ADRs)

Comprehensive architectural documentation is available in `docs/adr/`:

- **[ADR-001](docs/adr/ADR-001-sparse-ternary-vsa.md)**: Sparse Ternary VSA
  - Core VSA design and sparse ternary vectors
  - Balanced ternary mathematics and hardware optimization
  - 64-bit register encoding (39-40 trits per register)
  
- **[ADR-002](docs/adr/ADR-002-multi-agent-workflow-system.md)**: Multi-Agent Workflow System
  
- **[ADR-003](docs/adr/ADR-003-self-hosted-runner-architecture.md)**: Self-Hosted Runner Architecture
  
- **[ADR-004](docs/adr/ADR-004-holographic-os-container-design.md)**: Holographic OS Container Design
  - Configuration-driven builder for Debian/Ubuntu
  - Dual versioning strategy (LTS + nightly)
  - Package isolation capabilities
  
- **[ADR-005](docs/adr/ADR-005-hologram-package-isolation.md)**: Hologram-Based Package Isolation
  - Factoralization of holographic containers
  - Balanced ternary encoding for compact representation
  - Package-level granular updates
  - Hardware optimization strategy for 64-bit CPUs

- **[ADR-006](docs/adr/ADR-006-dimensionality-sparsity-scaling.md)**: Dimensionality and Sparsity Scaling
  - Scaling holographic space to TB-scale datasets
  - Adaptive sparsity strategy (maintain constant computational cost)
  - Performance analysis and collision probability projections
  - Impact on 100% bit-perfect guarantee
  - Deep operation resilience for factoralization

- **[ADR-007](docs/adr/ADR-007-codebook-security.md)**: Codebook Security and Reversible Encoding
  - VSA-as-a-lens cryptographic primitive
  - Quantum-resistant encoding mechanism
  - Mathematically trivial with key, impossible without
  - Bulk encryption with selective decryption
  - Integration with holographic indexing

See `docs/adr/README.md` for the complete ADR index.

### File Format

**Engram** (`.engram`):
- Binary serialized format (bincode)
- Contains root SparseVec and codebook
- Self-contained holographic state

**Manifest** (`.json`):
- Human-readable file listing
- Chunk mapping and metadata
- Required for extraction

## Development

### API Documentation

Comprehensive API documentation is available:

```bash
# Generate and open documentation locally
cargo doc --open

# Or use the automated script
./generate_docs.sh

# View online (after publishing)
# https://docs.rs/embeddenator
```

The documentation includes:
- Module-level overviews with examples
- Function documentation with usage patterns
- 9 runnable doc tests demonstrating API usage
- VSA operation examples (bundle, bind, cosine)

### Running Tests

```bash
# Recommended: everything Cargo considers testable (lib/bin/tests/examples/benches)
cargo test --workspace --all-targets

# Doc tests only
cargo test --doc

# Optimized build tests (useful before benchmarking)
cargo test --release --workspace --all-targets

# Feature-gated correctness/perf gates
cargo test --workspace --all-targets --features "bt-phase-2 proptest"

# Long-running/expensive tests are explicitly opt-in:
# - QA memory scaling (requires env var + ignored flag)
EMBEDDENATOR_RUN_QA_MEMORY=1 cargo test --features qa --test memory_scaled -- --ignored --nocapture
# - Multi-GB soak test (requires env var + ignored flag)
EMBEDDENATOR_RUN_SOAK=1 cargo test --release --features soak-memory --test soak_memory -- --ignored --nocapture

# Integration tests via orchestrator
python3 orchestrator.py --mode test --verbose

# Full test suite
python3 orchestrator.py --mode full --verbose
```

Notes:
- Seeing many tests marked as "ignored" during `cargo bench` is expected: Cargo runs the unit test
  harness in libtest's `--bench` mode, which skips normal `#[test]` functions (it prints `i` for each).
  Use `cargo test` (commands above) to actually execute tests.
- `cargo test --workspace --all-targets` will also compile/run Criterion benches in a fast "smoke" mode
  (they print `Testing ... Success`). This is intended to catch broken benches early.

### CI/CD and Build Monitoring

The project uses separated CI/CD workflows for optimal performance and reliability:

```bash
# Test CI build locally with monitoring
./ci_build_monitor.sh linux/amd64 build 300

# Monitor for specific timeout (in seconds)
./ci_build_monitor.sh linux/amd64 full 900
```

**CI Workflow Structure:**

Three separate workflows eliminate duplication and provide clear responsibilities:

1. **ci-pre-checks.yml** - Fast validation (fmt, clippy, unit tests, doc tests)
2. **ci-amd64.yml** - Full AMD64 build and test (**REQUIRED PRE-MERGE CHECK**)
3. **ci-arm64.yml** - ARM64 build and test (configured for self-hosted runners)

**CI Features:**
- Separated workflows prevent duplicate runs
- AMD64 workflow is a **required status check** - PRs cannot merge until it passes
- Parallel builds using all available cores
- Intelligent timeout management (15min tests, 10min builds, 30min total)
- Build artifact upload on failure
- Performance metrics reporting
- Automatic parallelization with `CARGO_BUILD_JOBS`

**Architecture Support:**

| Architecture | Status | Runner Type | Trigger | Notes |
|--------------|--------|-------------|---------|-------|
| **amd64 (x86_64)** | ✅ Production | GitHub-hosted (ubuntu-latest) | Every PR (required check) | Stable, 5-7min |
| **arm64 (aarch64)** | 🚧 Ready | Self-hosted (pending deployment) | Manual only | Will enable on merge to main |

**ARM64 Deployment Roadmap:**
- ✅ **Phase 1**: Root cause analysis completed - GitHub doesn't provide standard ARM64 runners
- ✅ **Phase 2**: Workflow configured for self-hosted runners with labels `["self-hosted", "linux", "ARM64"]`
- 🚧 **Phase 3**: Deploy self-hosted ARM64 infrastructure (in progress)
- ⏳ **Phase 4**: Manual testing and validation
- ⏳ **Phase 5**: Enable automatic trigger on merge to main only

**Why Self-Hosted for ARM64?**
- GitHub Actions doesn't provide standard hosted ARM64 runners
- Self-hosted provides native execution (no emulation overhead)
- Cost-effective for frequent builds
- Ready to deploy when infrastructure is available

See `.github/workflows/README.md` for complete CI/CD documentation and ARM64 setup guide.

### Self-Hosted Runner Automation

Embeddenator includes a comprehensive Python-based automation system for managing GitHub Actions self-hosted runners with complete lifecycle management and **multi-architecture support**:

**Features:**
- ✨ Automated registration with short-lived tokens
- 🔄 Complete lifecycle management (register → run → deregister)
- ⏱️ Configurable auto-deregistration after idle timeout
- 🎯 Manual mode for persistent runners
- 🚀 Multi-runner deployment support
- 🏗️ **Multi-architecture support (x64, ARM64, RISC-V)**
- 🔧 **QEMU emulation for cross-architecture runners**
- 📊 Health monitoring and status reporting
- 🧹 Automatic cleanup of Docker resources
- ⚙️ Flexible configuration via .env file or CLI arguments

**Supported Architectures:**
- **x64 (AMD64)** - Native x86_64 runners
- **ARM64 (aarch64)** - ARM64 runners (native or emulated via QEMU)
- **RISC-V (riscv64)** - RISC-V runners (native or emulated via QEMU)

**Quick Start:**

```bash
# 1. Copy and configure environment file
cp .env.example .env
# Edit .env and set GITHUB_REPOSITORY and GITHUB_TOKEN

# 2. Run in auto mode (registers, starts, monitors, auto-deregisters when idle)
python3 runner_manager.py run

# 3. Or use manual mode (keeps running until stopped)
RUNNER_MODE=manual python3 runner_manager.py run
```

**Multi-Architecture Examples:**

```bash
# Deploy ARM64 runners on x86_64 hardware (with emulation, auto-detect runtime)
RUNNER_TARGET_ARCHITECTURES=arm64 python3 runner_manager.py run

# Deploy runners for all architectures
RUNNER_TARGET_ARCHITECTURES=x64,arm64,riscv64 RUNNER_COUNT=6 python3 runner_manager.py run

# Deploy with automatic QEMU installation (requires sudo)
RUNNER_EMULATION_AUTO_INSTALL=true RUNNER_TARGET_ARCHITECTURES=arm64 python3 runner_manager.py run

# Use specific emulation method (docker, podman, or qemu)
RUNNER_EMULATION_METHOD=podman RUNNER_TARGET_ARCHITECTURES=arm64 python3 runner_manager.py run

# Use Docker for emulation
RUNNER_EMULATION_METHOD=docker RUNNER_TARGET_ARCHITECTURES=arm64,riscv64 python3 runner_manager.py run
```

**Individual Commands:**

```bash
# Register runner(s)
python3 runner_manager.py register

# Start runner service(s)
python3 runner_manager.py start

# Monitor and manage lifecycle
python3 runner_manager.py monitor

# Check status
python3 runner_manager.py status

# Stop and deregister
python3 runner_manager.py stop
```

**Advanced Usage:**

```bash
# Deploy multiple runners
python3 runner_manager.py run --runner-count 4

# Custom labels
python3 runner_manager.py register --labels self-hosted,linux,ARM64,large

# Auto-deregister after 10 minutes of inactivity
RUNNER_IDLE_TIMEOUT=600 python3 runner_manager.py run
```

**Configuration Options:**

Key environment variables (see `.env.example` for full list):
- `GITHUB_REPOSITORY` - Repository to register runners for (required)
- `GITHUB_TOKEN` - Personal access token with repo scope (required)
- `RUNNER_MODE` - Deployment mode: `auto` (default) or `manual`
- `RUNNER_IDLE_TIMEOUT` - Auto-deregister timeout in seconds (default: 300)
- `RUNNER_COUNT` - Number of runners to deploy (default: 1)
- `RUNNER_LABELS` - Comma-separated runner labels
- `RUNNER_EPHEMERAL` - Enable ephemeral runners (deregister after one job)
- `RUNNER_TARGET_ARCHITECTURES` - Target architectures: `x64`, `arm64`, `riscv64` (comma-separated)
- `RUNNER_ENABLE_EMULATION` - Enable QEMU emulation for cross-architecture (default: true)
- `RUNNER_EMULATION_METHOD` - Emulation method: `auto`, `qemu`, `docker`, `podman` (default: auto)
- `RUNNER_EMULATION_AUTO_INSTALL` - Auto-install QEMU if missing (default: false, requires sudo)

See `.env.example` for complete configuration documentation.

**Deployment Modes:**

1. **Auto Mode** (default): Runners automatically deregister after being idle for a specified timeout
   - Perfect for cost optimization
   - Ideal for CI/CD pipelines with sporadic builds
   - Runners terminate when queue is empty

2. **Manual Mode**: Runners keep running until manually stopped
   - Best for development environments
   - Useful for persistent infrastructure
   - Explicit control over runner lifecycle

See `.github/workflows/README.md` for complete CI/CD documentation and ARM64 setup guide.

### Project Structure

```
embeddenator/
├── Cargo.toml                  # Rust dependencies
├── src/
│   └── main.rs                 # Complete implementation
├── tests/
│   ├── e2e_regression.rs       # 6 E2E tests (includes critical engram modification test)
│   ├── integration_cli.rs      # 7 integration tests
│   └── unit_tests.rs           # 11 unit tests
├── Dockerfile.tool             # Static binary packaging
├── Dockerfile.holographic      # Holographic OS container
├── orchestrator.py             # Unified build/test/deploy
├── runner_manager.py           # Self-hosted runner automation entry point (NEW)
├── runner_automation/          # Runner automation package (NEW)
│   ├── __init__.py            # Package initialization (v1.1.0)
│   ├── config.py              # Configuration management
│   ├── github_api.py          # GitHub API client
│   ├── installer.py           # Runner installation
│   ├── runner.py              # Individual runner lifecycle
│   ├── manager.py             # Multi-runner orchestration
│   ├── emulation.py           # QEMU emulation for cross-arch (NEW)
│   ├── cli.py                 # Command-line interface
│   └── README.md              # Package documentation
├── .env.example                # Runner configuration template (NEW)
├── ci_build_monitor.sh         # CI hang detection and monitoring
├── generate_docs.sh            # Documentation generation
├── .github/
│   └── workflows/
│       ├── ci-pre-checks.yml       # Pre-build validation (every PR)
│       ├── ci-amd64.yml            # AMD64 build (required for merge)
│       ├── ci-arm64.yml            # ARM64 build (self-hosted, pending)
│       ├── build-holographic-os.yml# OS container builds
│       ├── build-push-images.yml   # Multi-OS image pipeline
│       ├── nightly-builds.yml      # Nightly bleeding-edge builds
│       └── README.md               # Complete CI/CD documentation
├── input_ws/                   # Example input (gitignored)
├── workspace/                  # Build artifacts (gitignored)
└── README.md               # This file
```

### Contributing

We welcome contributions to Embeddenator! Here's how you can help:

#### Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/embeddenator.git
   cd embeddenator
   ```
3. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-new-feature
   ```

#### Development Workflow

1. **Make your changes** with clear, focused commits
2. **Add tests** for new functionality:
   - Unit tests in `src/` modules
   - Integration tests in `tests/integration_*.rs`
   - End-to-end tests in `tests/e2e_*.rs`
3. **Run the full test suite**:
   ```bash
   # Run all Rust tests
   cargo test
   
   # Run integration tests via orchestrator
   python3 orchestrator.py --mode test --verbose
   
   # Run full validation suite
   python3 orchestrator.py --mode full --verbose
   ```
4. **Check code quality**:
   ```bash
   # Run Clippy linter (zero warnings required)
   cargo clippy -- -D warnings
   
   # Format code
   cargo fmt
   
   # Check Python syntax
   python3 -m py_compile *.py
   ```
5. **Test cross-platform** (if applicable):
   ```bash
   # Build Docker images
   docker build -f Dockerfile.tool -t embeddenator-tool:test .
   
   # Test on different architectures
   python3 orchestrator.py --platform linux/arm64 --mode test
   ```

#### Pull Request Guidelines

- **Write clear commit messages** describing what and why
- **Reference issues** in commit messages (e.g., "Fixes #123")
- **Keep PRs focused** - one feature or fix per PR
- **Update documentation** if you change CLI options or add features
- **Ensure all tests pass** before submitting
- **Maintain code coverage** - aim for >80% test coverage

#### Code Style

- **Rust**: Follow standard Rust conventions (use `cargo fmt`)
- **Python**: Follow PEP 8 style guide
- **Comments**: Document complex algorithms, especially VSA operations
- **Error handling**: Use proper error types, avoid `.unwrap()` in library code

#### Areas for Contribution

We especially welcome contributions in these areas:

- 🔬 **Performance optimizations** for VSA operations
- 📊 **Benchmarking tools** and performance analysis
- 🧪 **Additional test cases** covering edge cases
- 📚 **Documentation improvements** and examples
- 🐛 **Bug fixes** and error handling improvements
- 🌐 **Multi-platform support** (Windows, macOS testing)
- 🔧 **New features** (incremental updates, compression options, etc.)

#### Reporting Issues

When reporting bugs, please include:

- Embeddenator version (`embeddenator --version`)
- Operating system and architecture
- Rust version (`rustc --version`)
- Minimal reproduction steps
- Expected vs. actual behavior
- Relevant log output (use `--verbose` flag)

#### Questions and Discussions

- **Issues**: Bug reports and feature requests
- **Discussions**: Questions, ideas, and general discussion
- **Pull Requests**: Code contributions with tests

#### Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the technical merits
- Help others learn and grow

Thank you for contributing to Embeddenator! 🎉

## Advanced Usage

### Custom Chunk Size

Modify `chunk_size` in `EmbrFS::ingest_file` for different trade-offs:

```rust
let chunk_size = 8192; // Larger chunks = better compression, slower reconstruction
```

### Hierarchical Encoding

For very large datasets, implement multi-level engrams:

```rust
// Level 1: Individual files
// Level 2: Directory summaries
// Level 3: Root engram of all directories
```

### Algebraic Operations

Combine multiple engrams:

```rust
let combined = engram1.root.bundle(&engram2.root);
// Now combined contains both datasets holographically
```

## Troubleshooting

### Out of Memory

Reduce chunk size or process files in batches:

```bash
# Process directories separately
for dir in input_ws/*/; do
  cargo run --release -- ingest -i "$dir" -e "engrams/$(basename $dir).engram"
done
```

### Reconstruction Mismatches

Verify manifest and engram are from the same ingest:

```bash
# Check manifest metadata
jq '.total_chunks' workspace/manifest.json

# Re-ingest if needed
cargo run --release -- ingest -i ./input_ws -e root.engram -m manifest.json -v
```

## Performance Tips

1. **Use release builds**: `cargo build --release` is 10-100x faster
2. **Enable SIMD acceleration**: For query-heavy workloads, build with `--features simd` and `RUSTFLAGS="-C target-cpu=native"`
   ```bash
   # Build with SIMD optimizations
   RUSTFLAGS="-C target-cpu=native" cargo build --release --features simd
   ```
   See [docs/SIMD_OPTIMIZATION.md](docs/SIMD_OPTIMIZATION.md) for details on 2-4x query speedup
3. **Batch processing**: Ingest multiple directories separately for parallel processing
4. **SSD storage**: Engram I/O benefits significantly from fast storage
5. **Memory**: Ensure sufficient RAM for large codebooks (~100 bytes per chunk)

## License

MIT License - see LICENSE file for details

## References

### Vector Symbolic Architectures (VSA)
- Vector Symbolic Architectures: [Kanerva, P. (2009)](https://redwood.berkeley.edu/wp-content/uploads/2021/08/KanervaHyperdimensionalComputing09-JCSS.pdf)
- Sparse Distributed Representations
- Holographic Reduced Representations (HRR)

### Ternary Computing and Hardware Optimization
- [Balanced Ternary](https://en.wikipedia.org/wiki/Balanced_ternary) - Wikipedia overview
- [Ternary Computing](https://homepage.divms.uiowa.edu/~jones/ternary/) - Historical and mathematical foundations
- Three-Valued Logic and Quantum Computing
- Optimal encoding: 39-40 trits in 64-bit registers (39 for signed, 40 for unsigned)

### Architecture Documentation
- [ADR-001: Sparse Ternary VSA](docs/adr/ADR-001-sparse-ternary-vsa.md) - Core design and hardware optimization
- [ADR-005: Hologram Package Isolation](docs/adr/ADR-005-hologram-package-isolation.md) - Balanced ternary implementation
- [Complete ADR Index](docs/adr/README.md) - All architecture decision records

### Use Cases and Applications
- [Specialized AI Assistant Models](docs/SPECIALIZED_AI_ASSISTANTS.md) - Architecture for deploying coding and research assistant LLMs with embeddenator-enhanced retrieval, multi-model parallel execution, and document-driven development workflows

## Support

### Getting Help

- **Documentation**: This README and built-in help (`embeddenator --help`)
- **Issues**: Report bugs or request features at https://github.com/tzervas/embeddenator/issues
- **Discussions**: Ask questions and share ideas at https://github.com/tzervas/embeddenator/discussions
- **Examples**: See `examples/` directory (coming soon) for usage patterns

### Common Questions

**Q: What file types are supported?**  
A: All file types - text, binary, executables, images, etc. Embeddenator is file-format agnostic.

**Q: Is the reconstruction really bit-perfect?**  
A: Yes, for files tested so far. We have 160+ tests verifying reconstruction accuracy. However, large-scale (TB) testing is still in progress.

**Q: What's the project's development status?**  
A: This is alpha software (v0.20.0-alpha). Core functionality works and is tested, but APIs are unstable and not recommended for production use. See [PROJECT_STATUS.md](PROJECT_STATUS.md) for details.

**Q: Can I combine multiple engrams?**  
A: Yes! The bundle operation allows combining engrams. This is tested for basic cases but advanced algebraic operations are still experimental.

**Q: What's the maximum data size?**  
A: Hierarchical encoding is designed for large datasets. Currently tested with MB-scale data; TB-scale testing is planned but not yet validated.

**Q: How does this compare to compression?**  
A: Embeddenator is not primarily a compression tool. It creates holographic representations that enable algebraic operations on encoded data. Size characteristics vary by data type.

### Reporting Issues

When reporting bugs, please include:

- Embeddenator version: `embeddenator --version`
- Operating system and architecture
- Rust version: `rustc --version`
- Minimal reproduction steps
- Expected vs. actual behavior
- Relevant log output (use `--verbose` flag)

### Security

**⚠️ Security Notice**: The cryptographic properties of the encoding mechanism are under research. Do not use Embeddenator for security-critical applications or as a replacement for established cryptographic systems.

If you discover a security vulnerability, please email tz-dev@vectorweight.com or create a private security advisory on GitHub rather than opening a public issue.

## Documentation

### Project Documentation
- **[PROJECT_STATUS.md](PROJECT_STATUS.md)** - Complete status: what works, what's experimental, what's planned
- **[TESTING.md](TESTING.md)** - Comprehensive testing guide and infrastructure documentation
- **[LICENSE](LICENSE)** - MIT License terms

### Technical Documentation
- **[Component Architecture](docs/COMPONENT_ARCHITECTURE.md)** - Modular crate structure
- **[Local Development](docs/LOCAL_DEVELOPMENT.md)** - Development environment setup
- **[ADR Index](docs/adr/README.md)** - Architecture Decision Records

### API Documentation
```bash
# Generate and view API documentation
cargo doc --open
```

### Handoff Documentation
- **[QA to Documentation Handoff](docs/handoff/QA_TO_DOCUMENTATION_2026-01-09.md)** - Latest QA phase completion report

---

**License:** MIT - See [LICENSE](LICENSE) file for full text  
**Copyright:** 2025-2026 Tyler Zervas <tz-dev@vectorweight.com>

Built with Rust and Vector Symbolic Architecture principles.
