use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use embeddenator::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Create a realistic test directory structure with depth and file variations
fn create_test_structure(dir: &TempDir, total_size: usize, depth: usize, files_per_level: usize) {
    let base_path = dir.path();
    let file_size = total_size / (files_per_level * depth);
    
    for level in 0..depth {
        let level_dir = if level == 0 {
            base_path.to_path_buf()
        } else {
            let path = base_path.join(format!("level_{}", level));
            fs::create_dir_all(&path).unwrap();
            path
        };
        
        for file_idx in 0..files_per_level {
            let file_path = level_dir.join(format!("file_{:04}.txt", file_idx));
            let mut file = fs::File::create(&file_path).unwrap();
            
            // Create varied content with some repetition
            let content = format!(
                "Level {} File {} - Test data with varying patterns {}\n",
                level,
                file_idx,
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(file_size / 100)
            );
            file.write_all(content.as_bytes()).unwrap();
        }
    }
}

fn bench_hierarchical_bundling(c: &mut Criterion) {
    let mut group = c.benchmark_group("hierarchical_bundling");
    
    // Test scales: (total_size, depth, files_per_level, label)
    // Note: Using more practical sizes for benchmarking (< 5min total)
    let test_cases = vec![
        (10 * 1024 * 1024, 3, 5, "10MB_depth3_5files"),
        (50 * 1024 * 1024, 4, 8, "50MB_depth4_8files"),
        (100 * 1024 * 1024, 5, 10, "100MB_depth5_10files"),
    ];
    
    for (size, depth, files, label) in test_cases {
        // Benchmark with default settings (no sharding)
        group.bench_with_input(
            BenchmarkId::new("no_sharding", label),
            &(size, depth, files),
            |bencher, &(size, depth, files)| {
                let config = ReversibleVSAConfig::default();
                
                bencher.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        create_test_structure(&temp_dir, size, depth, files);
                        let mut fs = EmbrFS::new();
                        fs.ingest_directory(temp_dir.path(), false, &config).unwrap();
                        (fs, temp_dir)
                    },
                    |(fs, _temp_dir)| {
                        let result = fs.bundle_hierarchically(500, false, &config);
                        black_box(result).unwrap()
                    },
                );
            },
        );
        
        // Benchmark with sharding (max 100 chunks per node)
        group.bench_with_input(
            BenchmarkId::new("with_sharding_100", label),
            &(size, depth, files),
            |bencher, &(size, depth, files)| {
                let config = ReversibleVSAConfig::default();
                
                bencher.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        create_test_structure(&temp_dir, size, depth, files);
                        let mut fs = EmbrFS::new();
                        fs.ingest_directory(temp_dir.path(), false, &config).unwrap();
                        (fs, temp_dir)
                    },
                    |(fs, _temp_dir)| {
                        let result = fs.bundle_hierarchically_with_options(
                            500,
                            Some(100),
                            false,
                            &config,
                        );
                        black_box(result).unwrap()
                    },
                );
            },
        );
        
        // Benchmark with aggressive sharding (max 50 chunks per node)
        group.bench_with_input(
            BenchmarkId::new("with_sharding_50", label),
            &(size, depth, files),
            |bencher, &(size, depth, files)| {
                let config = ReversibleVSAConfig::default();
                
                bencher.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        create_test_structure(&temp_dir, size, depth, files);
                        let mut fs = EmbrFS::new();
                        fs.ingest_directory(temp_dir.path(), false, &config).unwrap();
                        (fs, temp_dir)
                    },
                    |(fs, _temp_dir)| {
                        let result = fs.bundle_hierarchically_with_options(
                            500,
                            Some(50),
                            false,
                            &config,
                        );
                        black_box(result).unwrap()
                    },
                );
            },
        );
    }
    
    group.finish();
}

fn bench_bundle_memory_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_memory_scaling");
    group.sample_size(10); // Fewer samples for large benchmarks
    
    // Test memory/time characteristics at different scales
    // Note: Conservative sizes for reasonable benchmark duration
    let sizes = vec![
        (5 * 1024 * 1024, "5MB"),
        (20 * 1024 * 1024, "20MB"),
        (50 * 1024 * 1024, "50MB"),
    ];
    
    for (size, label) in sizes {
        group.bench_with_input(
            BenchmarkId::new("linear_scaling", label),
            &size,
            |bencher, &size| {
                let config = ReversibleVSAConfig::default();
                
                bencher.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        create_test_structure(&temp_dir, size, 3, 10);
                        let mut fs = EmbrFS::new();
                        fs.ingest_directory(temp_dir.path(), false, &config).unwrap();
                        (fs, temp_dir)
                    },
                    |(fs, _temp_dir)| {
                        let result = fs.bundle_hierarchically(500, false, &config);
                        black_box(result).unwrap()
                    },
                );
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_hierarchical_bundling,
    bench_bundle_memory_scaling
);
criterion_main!(benches);
