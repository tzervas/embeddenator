//! Performance benchmarks for VersionedEmbrFS
//!
//! Run with: cargo bench

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use embeddenator_fs::VersionedEmbrFS;
use std::hint::black_box;
use std::sync::Arc;
use std::thread;

// Helper to create test data
fn create_data(size_bytes: usize, pattern: u8) -> Vec<u8> {
    vec![pattern; size_bytes]
}

// === Basic Operation Benchmarks ===

fn bench_write_small_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_small_file");

    for size_kb in [1, 4, 16, 64] {
        let size_bytes = size_kb * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &size_bytes,
            |b, &size| {
                let fs = VersionedEmbrFS::new();
                let data = create_data(size, 42);

                b.iter(|| {
                    let path = format!("bench_{}.dat", size);
                    fs.write_file(&path, black_box(&data), None).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_write_medium_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_medium_file");
    group.sample_size(20); // Reduce sample size for slower operations

    for size_mb in [1, 5, 10] {
        let size_bytes = size_mb * 1024 * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", size_mb)),
            &size_bytes,
            |b, &size| {
                let fs = VersionedEmbrFS::new();
                let data = create_data(size, 42);

                b.iter(|| {
                    fs.write_file("bench.dat", black_box(&data), None).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_read_small_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_small_file");

    for size_kb in [1, 4, 16, 64] {
        let size_bytes = size_kb * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &size_bytes,
            |b, &size| {
                let fs = VersionedEmbrFS::new();
                let data = create_data(size, 42);
                fs.write_file("bench.dat", &data, None).unwrap();

                b.iter(|| {
                    let (read_data, _) = fs.read_file(black_box("bench.dat")).unwrap();
                    black_box(read_data);
                });
            },
        );
    }

    group.finish();
}

fn bench_read_medium_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_medium_file");
    group.sample_size(20);

    for size_mb in [1, 5, 10] {
        let size_bytes = size_mb * 1024 * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", size_mb)),
            &size_bytes,
            |b, &size| {
                let fs = VersionedEmbrFS::new();
                let data = create_data(size, 42);
                fs.write_file("bench.dat", &data, None).unwrap();

                b.iter(|| {
                    let (read_data, _) = fs.read_file(black_box("bench.dat")).unwrap();
                    black_box(read_data);
                });
            },
        );
    }

    group.finish();
}

fn bench_update_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_file");

    for size_kb in [1, 4, 16, 64] {
        let size_bytes = size_kb * 1024;
        group.throughput(Throughput::Bytes(size_bytes as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &size_bytes,
            |b, &size| {
                let fs = VersionedEmbrFS::new();
                let data = create_data(size, 42);

                b.iter(|| {
                    // Write initial
                    let v1 = fs.write_file("bench.dat", &data, None).unwrap();

                    // Update
                    let new_data = create_data(size, 43);
                    fs.write_file("bench.dat", black_box(&new_data), Some(v1))
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

// === Concurrent Benchmarks ===

fn bench_concurrent_writes_different_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_writes_different_files");
    group.sample_size(10);

    for num_threads in [2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let fs = Arc::new(VersionedEmbrFS::new());
                    let data = create_data(4096, 42);
                    let mut handles = vec![];

                    for i in 0..threads {
                        let fs_clone = Arc::clone(&fs);
                        let data_clone = data.clone();
                        let handle = thread::spawn(move || {
                            let path = format!("thread_{}.dat", i);
                            fs_clone
                                .write_file(&path, black_box(&data_clone), None)
                                .unwrap();
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }

                    black_box(fs);
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_reads_same_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_reads_same_file");
    group.sample_size(10);

    for num_threads in [2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_threads", num_threads)),
            &num_threads,
            |b, &threads| {
                let fs = Arc::new(VersionedEmbrFS::new());
                let data = create_data(64 * 1024, 42);
                fs.write_file("shared.dat", &data, None).unwrap();

                b.iter(|| {
                    let mut handles = vec![];

                    for _ in 0..threads {
                        let fs_clone = Arc::clone(&fs);
                        let handle = thread::spawn(move || {
                            let (read_data, _) =
                                fs_clone.read_file(black_box("shared.dat")).unwrap();
                            black_box(read_data);
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_updates_different_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_updates_different_files");
    group.sample_size(10);

    for num_threads in [2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let fs = Arc::new(VersionedEmbrFS::new());
                    let data = create_data(4096, 42);

                    // Create initial files
                    for i in 0..threads {
                        let path = format!("file_{}.dat", i);
                        fs.write_file(&path, &data, None).unwrap();
                    }

                    // Concurrent updates
                    let new_data = create_data(4096, 43);
                    let mut handles = vec![];

                    for i in 0..threads {
                        let fs_clone = Arc::clone(&fs);
                        let data_clone = new_data.clone();
                        let handle = thread::spawn(move || {
                            let path = format!("file_{}.dat", i);
                            let (_, version) = fs_clone.read_file(&path).unwrap();
                            fs_clone
                                .write_file(&path, black_box(&data_clone), Some(version))
                                .unwrap();
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }

                    black_box(fs);
                });
            },
        );
    }

    group.finish();
}

// === Scalability Benchmarks ===

fn bench_many_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("many_small_files");
    group.sample_size(10);

    for num_files in [10, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", num_files)),
            &num_files,
            |b, &count| {
                b.iter(|| {
                    let fs = VersionedEmbrFS::new();
                    let data = create_data(1024, 42);

                    for i in 0..count {
                        let path = format!("file_{}.dat", i);
                        fs.write_file(&path, black_box(&data), None).unwrap();
                    }

                    black_box(fs);
                });
            },
        );
    }

    group.finish();
}

fn bench_file_listing(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_listing");

    for num_files in [10, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", num_files)),
            &num_files,
            |b, &count| {
                let fs = VersionedEmbrFS::new();
                let data = create_data(1024, 42);

                // Create files
                for i in 0..count {
                    let path = format!("file_{}.dat", i);
                    fs.write_file(&path, &data, None).unwrap();
                }

                b.iter(|| {
                    let files = fs.list_files();
                    black_box(files);
                });
            },
        );
    }

    group.finish();
}

// === Content Type Benchmarks ===

fn bench_compressible_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("compressible_data");
    group.sample_size(20);

    let size = 1024 * 1024; // 1MB
    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("write", |b| {
        let fs = VersionedEmbrFS::new();
        // Highly compressible (all zeros)
        let data = vec![0u8; size];

        b.iter(|| {
            fs.write_file("compressible.dat", black_box(&data), None)
                .unwrap();
        });
    });

    group.bench_function("read", |b| {
        let fs = VersionedEmbrFS::new();
        let data = vec![0u8; size];
        fs.write_file("compressible.dat", &data, None).unwrap();

        b.iter(|| {
            let (read_data, _) = fs.read_file(black_box("compressible.dat")).unwrap();
            black_box(read_data);
        });
    });

    group.finish();
}

fn bench_random_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_data");
    group.sample_size(20);

    let size = 1024 * 1024; // 1MB
    group.throughput(Throughput::Bytes(size as u64));

    // Generate pseudo-random data
    let data: Vec<u8> = (0..size)
        .map(|i: usize| ((i.wrapping_mul(2654435761)) % 256) as u8)
        .collect();

    group.bench_function("write", |b| {
        let fs = VersionedEmbrFS::new();

        b.iter(|| {
            fs.write_file("random.dat", black_box(&data), None).unwrap();
        });
    });

    group.bench_function("read", |b| {
        let fs = VersionedEmbrFS::new();
        fs.write_file("random.dat", &data, None).unwrap();

        b.iter(|| {
            let (read_data, _) = fs.read_file(black_box("random.dat")).unwrap();
            black_box(read_data);
        });
    });

    group.finish();
}

// === Transaction Pattern Benchmarks ===

fn bench_optimistic_locking_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimistic_locking_contention");
    group.sample_size(10);

    for num_threads in [2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let fs = Arc::new(VersionedEmbrFS::new());
                    fs.write_file("counter.dat", b"0", None).unwrap();

                    let mut handles = vec![];

                    for _ in 0..threads {
                        let fs_clone = Arc::clone(&fs);
                        let handle = thread::spawn(move || {
                            // Retry loop for optimistic locking
                            loop {
                                let (data, version) = fs_clone.read_file("counter.dat").unwrap();
                                let count: i32 =
                                    String::from_utf8_lossy(&data).parse().unwrap_or(0);
                                let new_count = count + 1;

                                match fs_clone.write_file(
                                    "counter.dat",
                                    new_count.to_string().as_bytes(),
                                    Some(version),
                                ) {
                                    Ok(_) => break,
                                    Err(_) => continue, // Retry on version mismatch
                                }
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }

                    black_box(fs);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    basic_ops,
    bench_write_small_file,
    bench_write_medium_file,
    bench_read_small_file,
    bench_read_medium_file,
    bench_update_file,
);

criterion_group!(
    concurrent_ops,
    bench_concurrent_writes_different_files,
    bench_concurrent_reads_same_file,
    bench_concurrent_updates_different_files,
);

criterion_group!(scalability, bench_many_small_files, bench_file_listing,);

criterion_group!(content_types, bench_compressible_data, bench_random_data,);

criterion_group!(transactions, bench_optimistic_locking_contention,);

criterion_main!(
    basic_ops,
    concurrent_ops,
    scalability,
    content_types,
    transactions
);
