//! CUDA Kernel Benchmarks
//!
//! Benchmarks for GPU-accelerated VSA operations.
//!
//! Run with: `cargo bench --features cuda cuda`
//!
//! NOTE: These benchmarks require CUDA-capable GPU hardware.
//! They are excluded from CI and should be run locally before
//! merging GPU-related changes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

// Stubs for non-CUDA builds
#[cfg(not(feature = "cuda"))]
fn cuda_benchmarks(_c: &mut Criterion) {
    eprintln!("CUDA benchmarks require --features cuda flag");
}

#[cfg(feature = "cuda")]
fn cuda_benchmarks(c: &mut Criterion) {
    use embeddenator_vsa::{GpuBackend, GpuConfig, GpuKernelRunner, ReversibleVSAEncoder};

    // Try to initialize GPU
    let gpu = match GpuBackend::new(GpuConfig::default()) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("GPU not available: {}", e);
            return;
        }
    };

    let mut runner = match GpuKernelRunner::new(&gpu) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create kernel runner: {}", e);
            return;
        }
    };

    let mut cpu_encoder = ReversibleVSAEncoder::new();

    // Benchmark different data sizes
    let sizes = [64, 256, 1024, 4096];

    // Create benchmark group for decode throughput
    let mut group = c.benchmark_group("cuda_decode_throughput");

    for size in sizes {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let encoded = cpu_encoder.encode(&data);

        group.throughput(Throughput::Bytes(size as u64));

        // CPU decode baseline
        group.bench_with_input(BenchmarkId::new("cpu", size), &size, |b, _| {
            b.iter(|| {
                let decoded = cpu_encoder.decode(black_box(&encoded), size);
                black_box(decoded)
            })
        });

        // GPU decode
        group.bench_with_input(BenchmarkId::new("gpu_kernel", size), &size, |b, _| {
            b.iter(|| {
                let decoded = runner.batch_decode(black_box(&encoded), size).unwrap();
                black_box(decoded)
            })
        });
    }

    group.finish();

    // Benchmark chunked decode
    let mut chunk_group = c.benchmark_group("cuda_chunked_decode");
    let chunk_size = 64;

    for total_size in [256, 1024, 4096] {
        let data: Vec<u8> = (0..total_size).map(|i| (i % 256) as u8).collect();
        let chunks = cpu_encoder.encode_chunked(&data, chunk_size);

        chunk_group.throughput(Throughput::Bytes(total_size as u64));

        // CPU chunked decode
        chunk_group.bench_with_input(
            BenchmarkId::new("cpu_chunked", total_size),
            &total_size,
            |b, _| {
                b.iter(|| {
                    let decoded =
                        cpu_encoder.decode_chunked(black_box(&chunks), chunk_size, total_size);
                    black_box(decoded)
                })
            },
        );

        // GPU chunked decode
        chunk_group.bench_with_input(
            BenchmarkId::new("gpu_chunked", total_size),
            &total_size,
            |b, _| {
                b.iter(|| {
                    let decoded = runner
                        .batch_decode_chunked(black_box(&chunks), chunk_size, total_size)
                        .unwrap();
                    black_box(decoded)
                })
            },
        );
    }

    chunk_group.finish();

    // Benchmark batch cosine (from gpu.rs)
    let mut cosine_group = c.benchmark_group("cuda_batch_cosine");

    for batch_size in [100, 500, 1000, 5000] {
        use embeddenator_vsa::SparseVec;

        let query = SparseVec::random();
        let candidates: Vec<SparseVec> = (0..batch_size).map(|_| SparseVec::random()).collect();

        cosine_group.throughput(Throughput::Elements(batch_size as u64));

        // CPU sequential
        cosine_group.bench_with_input(
            BenchmarkId::new("cpu_sequential", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    let sims: Vec<f64> = candidates.iter().map(|c| query.cosine(c)).collect();
                    black_box(sims)
                })
            },
        );

        // GPU batch
        cosine_group.bench_with_input(
            BenchmarkId::new("gpu_batch", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    let sims = gpu
                        .batch_cosine(black_box(&query), black_box(&candidates))
                        .unwrap();
                    black_box(sims)
                })
            },
        );
    }

    cosine_group.finish();
}

criterion_group!(benches, cuda_benchmarks);
criterion_main!(benches);
