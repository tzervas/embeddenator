# Embeddenator Performance Benchmarks

This directory contains Criterion-based performance benchmarks for the embeddenator project.

## Benchmark Suites

### vsa_ops.rs
Core VSA operations benchmarking.

**Benchmarks:**
- `sparsevec_ops`: Bundle, bind, cosine operations
- `reversible_encode_decode`: Encode/decode at various data sizes
- `bundle_modes`: Pairwise vs sum-many vs hybrid bundling

**Run:**
```bash
cargo bench --bench vsa_ops
```

### retrieval.rs
Inverted index performance benchmarking.

**Benchmarks:**
- `retrieval_index/build`: Index construction at scale
- `retrieval_index/query_top_k_20`: Query performance

**Run:**
```bash
cargo bench --bench retrieval
```

### hierarchical_scale.rs
Hierarchical bundling performance at scale.

**Benchmarks:**
- `hierarchical_bundling`: Bundling at 10MB/50MB/100MB scales
  - Tests with/without sharding (max chunks per node)
  - Validates O(n) scaling
- `bundle_memory_scaling`: Memory usage characteristics

**Run:**
```bash
cargo bench --bench hierarchical_scale

# Run specific size
cargo bench --bench hierarchical_scale -- "10MB"

# Quick test with fewer samples
cargo bench --bench hierarchical_scale -- --sample-size 10
```

### real_world.rs
Real-world data encoding and retrieval benchmarks.

**Benchmarks:**
- `image_encoding`: PNG/JPEG-like gradient and noise patterns at various resolutions
- `video_frames`: Frame sequence encoding, similarity detection, temporal bundling
- `audio_encoding`: Audio sample encoding, fingerprint comparison
- `document_encoding`: Text document encoding, retrieval index performance
- `binary_encoding`: Executable/archive blob encoding and roundtrip
- `render_tasks`: Render output caching, tile similarity search
- `mixed_workload`: Cross-type encoding, similarity, bundling
- `streaming`: Chunked data processing, rolling window aggregation

**Data Sources:**
- Synthetic: Gradients, noise patterns, sine waves, lorem ipsum
- Real (optional): Download via `./scripts/fetch_benchmark_data.sh`

**Run:**
```bash
# Download sample data first (optional, improves realism)
./scripts/fetch_benchmark_data.sh

# Run all real-world benchmarks
cargo bench --bench real_world

# Run specific categories
cargo bench --bench real_world -- "image"
cargo bench --bench real_world -- "video"
cargo bench --bench real_world -- "render"
cargo bench --bench real_world -- "streaming"
```

### query_hierarchical.rs
Hierarchical query performance benchmarking.

**Benchmarks:**
- `hierarchical_query_depth`: Performance vs hierarchy depth
- `hierarchical_query_width`: Performance vs hierarchy width
- `flat_vs_hierarchical`: Comparison of query strategies
- `beam_width_scaling`: Beam width parameter tuning

**Run:**
```bash
cargo bench --bench query_hierarchical

# Run specific benchmark
cargo bench --bench query_hierarchical -- "flat_vs"
cargo bench --bench query_hierarchical -- "beam_width"
```

## Running Benchmarks

### All Benchmarks
```bash
# Run everything (may take 30+ minutes)
cargo bench

# Run with custom sample size (faster, less precise)
cargo bench -- --sample-size 10
```

### Specific Benchmark Suite
```bash
cargo bench --bench hierarchical_scale
cargo bench --bench query_hierarchical
```

### Filtered Benchmarks
```bash
# Filter by name pattern
cargo bench --bench hierarchical_scale -- "10MB"
cargo bench -- "bundle"

# Multiple filters (OR logic)
cargo bench -- "10MB|bundle"
```

### Compilation Only (No Execution)
```bash
cargo bench --no-run
```

## Understanding Results

### Criterion Output

Criterion provides:
- **Time**: Mean execution time with confidence intervals
- **Change**: Comparison to previous run (if available)
- **Outliers**: Statistical outlier detection
- **Plots**: Saved to `target/criterion/`

Example output:
```
hierarchical_bundling/no_sharding/10MB_depth3_5files
                        time:   [6.1028 ms 6.1794 ms 6.3389 ms]
                        change: [-2.5% -1.2% +0.5%] (no change)
Found 1 outliers among 10 measurements (10.00%)
```

### Interpreting Changes

- **No change**: Performance is stable (within noise)
- **Improved**: Statistically significant speedup
- **Regressed**: Statistically significant slowdown
- Look for changes > 10% as potentially significant

### Storage Location

- Results: `target/criterion/`
- Plots: `target/criterion/<benchmark-name>/report/`
- Historical data: Used for automatic comparison

## Regression Detection

### Automatic (Criterion)
Criterion automatically compares each run to the previous run.

### Manual Baseline
```bash
# Save baseline
cargo bench --bench hierarchical_scale > baseline_hierarchical.txt

# After changes, compare
cargo bench --bench hierarchical_scale > new_results.txt
diff baseline_hierarchical.txt new_results.txt
```

### CI Integration
For CI pipelines:
```bash
# Quick smoke test (fewer samples)
cargo bench -- --sample-size 5

# Save as artifact
cargo bench --bench hierarchical_scale -- --save-baseline main

# Compare PR to main
cargo bench --bench hierarchical_scale -- --baseline main
```

## Performance Baselines

See [docs/performance/THROUGHPUT.md](../docs/performance/THROUGHPUT.md) for:
- v0.3.0 performance baselines
- Hardware specifications
- Performance characteristics analysis
- TB-scale validation approach

### Quick Reference (v0.3.0)

**Hierarchical Bundling:**
- 10MB: ~6ms (linear O(n) scaling)
- Sharding overhead: ~1-2%
- Extrapolated 1TB: ~10 minutes

**Hierarchical Query:**
- Small datasets (<1000 chunks): Flat is faster
- Large datasets (>1000 chunks): Hierarchical shows O(log n) advantage
- Beam width 10: Good default balance

## Benchmark Design Guidelines

### Use iter_with_setup
Separate setup from measurement:
```rust
bencher.iter_with_setup(
    || {
        // Setup (not measured)
        let data = create_test_data();
        data
    },
    |data| {
        // Measured code
        process(data)
    }
);
```

### Use black_box
Prevent compiler optimizations:
```rust
bencher.iter(|| {
    let result = expensive_operation(black_box(&input));
    black_box(result)
});
```

### Realistic Test Data
- Use representative data sizes and structures
- Match production data characteristics
- Consider variation (don't just repeat same pattern)

### Sample Size
- Default: Criterion auto-determines (usually 100)
- For long benchmarks: `--sample-size 10`
- For precise measurements: `--sample-size 1000`

## Troubleshooting

### "No space left on device"
Benchmarks create temporary test files. If /tmp is full:
```bash
# Check space
df -h /tmp

# Clean old temp files
rm -rf /tmp/tmp.*

# Or set TMPDIR
export TMPDIR=/path/to/large/partition
cargo bench
```

### Benchmarks Take Too Long
Reduce sample size:
```bash
cargo bench -- --sample-size 10
```

Or filter to specific benchmarks:
```bash
cargo bench --bench hierarchical_scale -- "10MB"
```

### Inconsistent Results
Benchmarks can be noisy. For stable results:
- Close other applications
- Disable CPU frequency scaling
- Run multiple times and compare
- Use `--warm-up-time` for longer warm-up

### "Unable to complete N samples in 5.0s"
This warning is normal for long-running benchmarks. Criterion will adjust automatically.

## Contributing

When adding new benchmarks:

1. **Follow existing patterns** (see vsa_ops.rs, retrieval.rs)
2. **Use Criterion groups** for related benchmarks
3. **Document what you're testing** in comments
4. **Add to Cargo.toml** `[[bench]]` section
5. **Update this README** with new benchmark description
6. **Run locally** before submitting PR

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Project THROUGHPUT.md](../docs/performance/THROUGHPUT.md)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
