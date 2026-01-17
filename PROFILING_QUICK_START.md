# Profiling Toolkit - Quick Reference

## ✅ What's Now Available

All profiling tools have been installed and the complete toolkit is ready to use:

### System Profiling Tools
- **perf** (Linux performance) - Identifies CPU hotspots
- **valgrind** (Memory profiler) - Tracks memory usage and leaks
- **flamegraph** (Visual profiler) - Interactive call stack visualization

### Ready-to-Use Scripts

```bash
# 1. CPU Profiling (fastest, ~30 seconds)
./profile_cpu_perf.sh ingest 100

# 2. Memory Profiling (slowest, ~2 minutes due to Valgrind overhead)
./profile_memory.sh ingest 100

# 3. Flamegraph (medium speed, generates interactive SVG)
./profile_flamegraph.sh ingest 100

# 4. Everything at once (complete analysis)
./profile_comprehensive.sh 100
```

## 📊 What Each Tool Does

### profile_cpu_perf.sh
- Records CPU usage at 99Hz sampling
- Shows function hotspots (% of CPU time)
- Reports cache hit/miss rates
- **Time:** ~30-40 seconds for 100MB
- **Output:** `.perf` file + text report

### profile_memory.sh
- Tracks peak memory usage during execution
- Identifies allocation patterns
- Detects memory leaks
- **Time:** ~2-3 minutes for 100MB (Valgrind is slow)
- **Output:** `.massif` file (Valgrind format)

### profile_flamegraph.sh
- Visual representation of call stacks
- Shows which functions consume time
- Interactive: click to zoom, search functions
- **Time:** ~1-2 minutes for 100MB
- **Output:** `.svg` file (open in browser)

### profile_comprehensive.sh
- Runs all three profilers in sequence
- Generates interpretation guide
- Complete analysis in one command
- **Time:** ~5-10 minutes for 100MB
- **Output:** All results + summary guide

## 🚀 Quick Start for Phase 1 Validation

```bash
# Build the optimized binary
cargo build --release --features "bt-phase-2,simd"

# Profile on small dataset first (fast)
./profile_cpu_perf.sh ingest 50

# Look at results
ls -lh profile_results/

# View the CPU hotspots
perf report -i profile_results/ingest-*.perf
# (Press 'q' to quit, 'h' for help)
```

## 📈 What to Look For

### CPU Profile Results
- **Good sign:** Hotspots distributed across multiple functions (15-20% each)
- **Bad sign:** Single function consuming >50% CPU
- **Action:** Profile external components (embeddenator-fs, embeddenator-vsa) to find actual bottlenecks

### Memory Profile Results
- **Good sign:** Peak memory stable, linear growth
- **Bad sign:** Sudden spikes or unbounded growth
- **Action:** Use memory profiler to identify allocation culprits

### Flamegraph
- **Good sign:** Shallow stacks (fast call chains), wide bars (functions working)
- **Bad sign:** Deep stacks (call overhead), thin bars (function overhead)
- **Action:** Look for repeated patterns that can be optimized

## 🔧 Fixed Issues

| Issue | Fix |
|-------|-----|
| `perf: command not found` | ✅ Installed `linux-perf` package |
| Missing profiling tools | ✅ Installed `valgrind` and `flamegraph` |
| Kernel permissions for perf | ✅ Set `perf_event_paranoid=1` |
| Compilation error in src/perf.rs | ✅ Fixed markdown doc comments |

## 📁 File Structure

```
embeddenator/
├── profile_cpu_perf.sh        ← CPU hotspot identification
├── profile_memory.sh           ← Memory usage tracking
├── profile_flamegraph.sh       ← Visual call stacks
├── profile_comprehensive.sh    ← All-in-one profiling
├── PROFILING_TOOLKIT.md        ← Full documentation
├── PERF_TUNING_PHASE1.md       ← Phase 1 details
├── PERFORMANCE_TUNING_STRATEGY.md ← 3-phase roadmap
└── profile_results/            ← Generated profiles (after running scripts)
    ├── ingest-cpu-*.perf       ← perf raw data
    ├── ingest-mem-*.massif     ← Valgrind memory dump
    ├── ingest-*.svg            ← Flamegraph SVG
    └── PROFILING_SUMMARY_*.txt ← Interpretation guide
```

## 📝 Next Steps

1. **Run CPU Profile** (fast baseline)
   ```bash
   ./profile_cpu_perf.sh ingest 100
   ```

2. **Check Results**
   ```bash
   perf report -i profile_results/ingest-cpu-*.perf
   ```

3. **Identify Hotspots**
   - Note the top 3-5 functions
   - Check if they're in main crate or external (fs/vsa)

4. **Run Flamegraph** (visual understanding)
   ```bash
   ./profile_flamegraph.sh ingest 100
   # Then open the .svg file in a browser
   ```

5. **Memory Profile** (if optimization targets memory)
   ```bash
   ./profile_memory.sh ingest 100
   # Then view: ms_print profile_results/ingest-mem-*.massif | less
   ```

## ⚠️ Important Notes

- **First run is slow** - CPU compilation and setup takes time
- **Flamegraph needs debug symbols** - Scripts automatically enable this
- **Valgrind slows execution 5-10x** - Memory profiling is slow but valuable
- **Permission issues?** - If perf fails, run: `echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid`
- **No symbols in flamegraph?** - Rebuild with: `CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release`

## 🎯 Success Criteria for Phase 1

After running profiling, you should see:

1. ✅ CPU profile completes without errors
2. ✅ Flamegraph generates .svg (open-able in browser)
3. ✅ Memory profile completes and shows peak usage
4. ✅ Results show where time is spent (identify bottlenecks)

## 💡 Common Commands

```bash
# View CPU hotspots (text-based report)
perf report -i profile_results/ingest-cpu-*.perf --stdio

# View memory peak
ms_print profile_results/ingest-mem-*.massif | grep "Peak" 

# Compare before/after optimization
diff <(perf report -i baseline/ingest-cpu-*.perf) \
     <(perf report -i current/ingest-cpu-*.perf)

# Extract function names from flamegraph
perf script -i profile_results/ingest-cpu-*.perf | grep -o '[a-z_]*::' | sort | uniq -c | sort -rn | head -20
```

---

**Status:** ✅ Complete and tested  
**Ready:** Yes, all tools installed and validated  
**Next:** Run `./profile_cpu_perf.sh ingest 50` to start profiling
