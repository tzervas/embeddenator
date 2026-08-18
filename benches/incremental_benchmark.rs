//! Incremental Operations Benchmarks
//!
//! Measures performance of incremental update operations.
//!
//! Run with:
//!   cargo bench --bench incremental_benchmark

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use std::hint::black_box;

fn create_base_engram(file_count: usize, file_size: usize) -> (EmbrFS, std::path::PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!("embr_bench_incr_{}", file_count));
    let input_dir = temp_dir.join("input");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&input_dir).unwrap();

    // Create initial files
    for i in 0..file_count {
        let filename = format!("file_{:04}.dat", i);
        let data = vec![((i * 42) % 256) as u8; file_size];
        fs::write(input_dir.join(&filename), &data).unwrap();
    }

    // Create engram
    let config = ReversibleVSAConfig::default();
    let mut fs = EmbrFS::new();
    fs.ingest_directory(&input_dir, false, &config).unwrap();

    (fs, temp_dir)
}

fn bench_add_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_file");

    for base_size in [10, 50, 100] {
        group.throughput(Throughput::Bytes(4096));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("base_{}files", base_size)),
            &base_size,
            |b, &size| {
                let temp_dir = std::env::temp_dir().join(format!("embr_bench_add_{}", size));
                let input_dir = temp_dir.join("input");
                let _ = std::fs::remove_dir_all(&temp_dir);
                std::fs::create_dir_all(&input_dir).unwrap();

                // Prepare files to add
                let new_file = temp_dir.join("new.dat");
                let data = vec![42u8; 4096];
                std::fs::write(&new_file, &data).unwrap();

                let config = ReversibleVSAConfig::default();

                b.iter_with_setup(
                    || {
                        // Setup: create fresh engram for each iteration
                        let (fs, _) = create_base_engram(size, 4096);
                        fs
                    },
                    |mut fs| {
                        fs.add_file(black_box(&new_file), "new.dat".to_string(), false, &config)
                            .unwrap();
                        black_box(fs);
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_remove_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_file");

    for base_size in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("base_{}files", base_size)),
            &base_size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        let (fs, _temp_dir) = create_base_engram(size, 4096);
                        fs
                    },
                    |mut fs| {
                        fs.remove_file(black_box("file_0000.dat"), false).unwrap();
                        black_box(fs);
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_modify_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("modify_file");
    group.throughput(Throughput::Bytes(4096));

    for base_size in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("base_{}files", base_size)),
            &base_size,
            |b, &size| {
                let temp_dir = std::env::temp_dir().join(format!("embr_bench_mod_{}", size));

                // Prepare modified file
                let modified_file = temp_dir.join("modified.dat");
                let data = vec![99u8; 4096];
                std::fs::write(&modified_file, &data).unwrap();

                let config = ReversibleVSAConfig::default();

                b.iter_with_setup(
                    || {
                        let (fs, _) = create_base_engram(size, 4096);
                        fs
                    },
                    |mut fs| {
                        fs.modify_file(
                            black_box(&modified_file),
                            "file_0000.dat".to_string(),
                            false,
                            &config,
                        )
                        .unwrap();
                        black_box(fs);
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_compact(c: &mut Criterion) {
    let mut group = c.benchmark_group("compact");
    group.sample_size(10);

    for base_size in [20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}files", base_size)),
            &base_size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        // Setup: create engram with some deleted files
                        let (mut fs, _temp_dir) = create_base_engram(size, 4096);

                        // Mark half the files as deleted
                        for i in 0..(size / 2) {
                            let path = format!("file_{:04}.dat", i);
                            fs.remove_file(&path, false).unwrap();
                        }

                        fs
                    },
                    |mut fs| {
                        let config = ReversibleVSAConfig::default();
                        fs.compact(false, &config).unwrap();
                        black_box(fs);
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_sequential_adds(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_adds");
    group.sample_size(10);

    let add_count = 10;

    group.bench_function(format!("add_{}files", add_count), |b| {
        let temp_dir = std::env::temp_dir().join("embr_bench_seq_adds");
        let input_dir = temp_dir.join("input");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&input_dir).unwrap();

        // Pre-create files to add
        for i in 0..add_count {
            let filename = format!("add_{:04}.dat", i);
            let data = vec![((i * 17) % 256) as u8; 4096];
            fs::write(input_dir.join(&filename), &data).unwrap();
        }

        b.iter(|| {
            let mut fs = EmbrFS::new();
            let config = ReversibleVSAConfig::default();

            for i in 0..add_count {
                let filename = format!("add_{:04}.dat", i);
                let file_path = input_dir.join(&filename);
                fs.add_file(black_box(&file_path), filename, false, &config)
                    .unwrap();
            }

            black_box(fs);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_add_file,
    bench_remove_file,
    bench_modify_file,
    bench_compact,
    bench_sequential_adds
);
criterion_main!(benches);
