//! Ingestion Performance Benchmarks
//!
//! Measures performance of file ingestion operations.
//!
//! Run with:
//!   cargo bench --bench ingest_benchmark

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use std::fs;
use std::hint::black_box;

fn create_test_data(size_bytes: usize, pattern: u8) -> Vec<u8> {
    vec![pattern; size_bytes]
}

fn bench_ingest_single_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest_single_file");

    for size_kb in [1, 4, 16, 64, 256] {
        let size_bytes = size_kb * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &size_bytes,
            |b, &size| {
                let temp_dir = std::env::temp_dir().join("embr_bench_ingest_single");
                let _ = fs::remove_dir_all(&temp_dir);
                fs::create_dir_all(&temp_dir).unwrap();

                let test_file = temp_dir.join("test.dat");
                let data = create_test_data(size, 42);
                fs::write(&test_file, &data).unwrap();

                let config = ReversibleVSAConfig::default();

                b.iter(|| {
                    let mut fs = EmbrFS::new();
                    fs.ingest_file(
                        black_box(&test_file),
                        "test.dat".to_string(),
                        false,
                        &config,
                    )
                    .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_ingest_multiple_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest_multiple_small_files");
    group.sample_size(20);

    for file_count in [10, 50, 100] {
        let file_size = 4096; // 4KB each
        let total_bytes = file_count * file_size;
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}files", file_count)),
            &file_count,
            |b, &count| {
                let temp_dir = std::env::temp_dir().join("embr_bench_ingest_multi");
                let input_dir = temp_dir.join("input");
                let _ = fs::remove_dir_all(&temp_dir);
                fs::create_dir_all(&input_dir).unwrap();

                // Pre-create files
                for i in 0..count {
                    let filename = format!("file_{:04}.dat", i);
                    let data = create_test_data(file_size, (i % 256) as u8);
                    fs::write(input_dir.join(&filename), &data).unwrap();
                }

                let config = ReversibleVSAConfig::default();

                b.iter(|| {
                    let mut fs = EmbrFS::new();
                    fs.ingest_directory(black_box(&input_dir), false, &config)
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_ingest_large_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest_large_file");
    group.sample_size(10);

    for size_mb in [1, 5, 10] {
        let size_bytes = size_mb * 1024 * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", size_mb)),
            &size_bytes,
            |b, &size| {
                let temp_dir = std::env::temp_dir().join("embr_bench_ingest_large");
                let _ = fs::remove_dir_all(&temp_dir);
                fs::create_dir_all(&temp_dir).unwrap();

                let test_file = temp_dir.join("large.dat");
                let data = create_test_data(size, 42);
                fs::write(&test_file, &data).unwrap();

                let config = ReversibleVSAConfig::default();

                b.iter(|| {
                    let mut fs = EmbrFS::new();
                    fs.ingest_file(
                        black_box(&test_file),
                        "large.dat".to_string(),
                        false,
                        &config,
                    )
                    .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_ingest_with_subdirectories(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest_with_subdirectories");
    group.sample_size(10);

    let config = ReversibleVSAConfig::default();

    group.bench_function("nested_structure", |b| {
        let temp_dir = std::env::temp_dir().join("embr_bench_ingest_nested");
        let input_dir = temp_dir.join("input");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&input_dir).unwrap();

        // Create nested directory structure
        for i in 0..5 {
            let subdir = input_dir.join(format!("dir_{}", i));
            fs::create_dir_all(&subdir).unwrap();

            for j in 0..10 {
                let filename = format!("file_{}.dat", j);
                let data = create_test_data(4096, ((i * 10 + j) % 256) as u8);
                fs::write(subdir.join(&filename), &data).unwrap();
            }
        }

        b.iter(|| {
            let mut fs = EmbrFS::new();
            fs.ingest_directory(black_box(&input_dir), false, &config)
                .unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ingest_single_file,
    bench_ingest_multiple_small_files,
    bench_ingest_large_file,
    bench_ingest_with_subdirectories
);
criterion_main!(benches);
