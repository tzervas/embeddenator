# Profiling Toolkit Setup & Usage

## ✅ Installed Tools

- **linux-perf** (`perf`) - CPU profiling, cache statistics
- **valgrind** - Memory profiling and leak detection
- **cargo-flamegraph** - Interactive stack visualization

## 📊 Available Profiling Scripts

### 1. Quick CPU Profile
```bash
./profile_cpu_perf.sh ingest 100
```
- Identifies CPU hotspots
- Shows function time distribution
- Reports cache hit rates
- Output: `profile_results/ingest-*.perf` + text report

### 2. Memory Profile
```bash
./profile_memory.sh ingest 100
```
- Tracks peak memory usage
- Identifies allocation patterns
- Detects memory leaks
- Output: `profile_results/ingest-mem-*.massif`

### 3. Flamegraph (Interactive)
```bash
./profile_flamegraph.sh ingest 500
```
- Visual call stack depth + duration
- Click to zoom in/out
- Search for functions
- Output: `profile_results/ingest-*.svg` (open in browser)

### 4. Comprehensive Profile (All Tools)
```bash
./profile_comprehensive.sh 500
```
Runs all profilers in sequence:
1. CPU profiler (perf)
2. Memory profiler (Valgrind)
3. Flamegraph generation
4. Cache statistics
5. Generates interpretation guide

Output: `profile_results/` directory with all results + `PROFILING_SUMMARY_*.txt`

## 🚀 Quick Start

### For Phase 1 Validation
```bash
# Build optimized binary
cargo build --release --features "bt-phase-2,simd"

# Run comprehensive profiling on 100MB dataset
./profile_comprehensive.sh 100

# View results
firefox profile_results/ingest-*.svg &
perf report -i profile_results/ingest-cpu-*.perf
ms_print profile_results/ingest-mem-*.massif | less
```

### For Phase 2 Analysis (before optimization)
```bash
# Profile on larger dataset to see memory patterns
./profile_comprehensive.sh 1000

# Key metrics to track:
# - Peak memory (from ms_print output)
# - CPU cycles per instruction (IPC from perf)
# - Cache miss rate (LLC-load-misses from perf)
# - Function time distribution (from flamegraph)
```

## 📈 Understanding Results

### CPU Profile (perf)
Look for:
- **Hotspot functions** (>10% CPU) → Optimization candidates
- **Cache misses** (>5%) → Data structure issues
- **Branch misses** (>5%) → Prediction failures
- **LLC misses** → Working set too large

### Memory Profile (Valgrind)
Look for:
- **Peak heap > 1GB** → Compression/streaming candidates
- **Many small allocations** → Object pool opportunities
- **Steady growth** → Potential memory leak
- **Fragmentation** → Allocation strategy issues

### Flamegraph
Look for:
- **Wide functions** (horizontal bars) → Hot code
- **Deep stacks** → Call overhead, inlining opportunities
- **Repeated patterns** → Loop unrolling candidates
- **Component boundaries** → Component optimization priority

## 🔍 Advanced Usage

### Single-threaded baseline
```bash
# For comparison, single-threaded profile
RAYON_NUM_THREADS=1 ./profile_cpu_perf.sh ingest 100

# Compare perf output with parallel version
perf report -i profile_results/ingest-cpu-*.perf
```

### Profile specific operation
```bash
# Extract operation (different bottlenecks)
./profile_cpu_perf.sh extract 100

# Query operation
./profile_cpu_perf.sh query 100
```

### Generate flamegraph from existing perf data
```bash
perf script -i profile_results/ingest-cpu-*.perf | \
    stackcollapse-perf.pl | \
    flamegraph.pl > new_flamegraph.svg
```

### Detailed instruction analysis
```bash
perf report -i profile_results/ingest-cpu-*.perf --stdio \
    | grep -A 20 "Samples:" | head -40
```

## 📊 Profiling Workflow for Phase 2

1. **Baseline Profile** (current state)
   ```bash
   ./profile_comprehensive.sh 500
   # Save results: cp -r profile_results profile_results_baseline
   ```

2. **Implement Optimization** (e.g., sparse vectors)
   ```bash
   # Modify embeddenator-vsa to use sparse packing
   # Update embeddenator/src/lib.rs to use new version
   cargo build --release
   ```

3. **Compare Profile**
   ```bash
   ./profile_comprehensive.sh 500
   # Results in profile_results (for comparison)
   ```

4. **Analyze Differences**
   ```bash
   # CPU improvement
   perf report -i profile_results/ingest-cpu-*.perf > current.txt
   perf report -i profile_results_baseline/ingest-cpu-*.perf > baseline.txt
   diff baseline.txt current.txt
   
   # Memory improvement
   ms_print profile_results_baseline/ingest-mem-*.massif | grep "Peak" > baseline_mem.txt
   ms_print profile_results/ingest-mem-*.massif | grep "Peak" > current_mem.txt
   ```

## 🎯 Expected Phase 1 Results

With compiler optimizations and thread pool tuning, you should see:
- **CPU cycles**: 15-20% reduction
- **Cache misses**: Similar (no algorithm changes)
- **Memory**: Slightly increased (thread pool overhead)
- **Throughput**: 25-33 MB/s ingestion (vs 16.6 MB/s baseline)

## ⚠️ Profiling Notes

- Profiling adds 5-10% overhead (inherent to tools)
- Flamegraph requires symbols (use `CARGO_PROFILE_RELEASE_DEBUG=true`)
- Memory profiling slows execution ~5x (Valgrind overhead)
- For fastest results, use `profile_cpu_perf.sh` (low overhead)

## 📁 Output Structure

```
profile_results/
├── ingest-cpu-TIMESTAMP.perf         # Raw perf data
├── ingest-cpu-TIMESTAMP-report.txt   # Text report
├── ingest-mem-TIMESTAMP.massif       # Memory dump
├── ingest-TIMESTAMP.svg              # Flamegraph
└── PROFILING_SUMMARY_TIMESTAMP.txt   # Interpretation guide
```

## 🔧 Troubleshooting

**"Permission denied" on perf**
```bash
# May need kernel.perf_event_paranoid adjustment
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

**"No symbols" in flamegraph**
```bash
# Rebuild with debug symbols
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release
```

**Flamegraph too wide (all time in one function)**
```bash
# Likely a blocking operation (I/O wait)
# Use memory profiler instead to find data structures
./profile_memory.sh ingest 100
```

## 📚 Further Reading

- [perf wiki](https://perf.wiki.kernel.org/)
- [Valgrind manual](https://valgrind.org/docs/manual/)
- [flamegraph guide](http://www.brendangregg.com/flamegraphs.html)
- [Performance Tuning Guide](PERFORMANCE_TUNING_STRATEGY.md)
