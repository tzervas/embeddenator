//! Query Performance Benchmarks
//!
//! Measures performance of similarity query operations.
//!
//! Run with:
//!   cargo bench --bench query_benchmark

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
use embeddenator_vsa::SparseVec;
use std::fs;
use std::hint::black_box;

fn create_test_corpus(
    dir: &std::path::Path,
    file_count: usize,
    file_size: usize,
) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;

    for i in 0..file_count {
        let filename = format!("file_{:04}.dat", i);
        let mut data = Vec::with_capacity(file_size);

        for j in 0..file_size {
            data.push(((i * file_size + j) % 256) as u8);
        }

        fs::write(dir.join(&filename), &data)?;
    }

    Ok(())
}

fn bench_query_codebook(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_codebook");

    let temp_dir = std::env::temp_dir().join("embr_bench_query");
    let input_dir = temp_dir.join("input");
    let _ = fs::remove_dir_all(&temp_dir);

    // Create test corpus
    create_test_corpus(&input_dir, 100, 4096).unwrap();

    // Ingest corpus
    let config = ReversibleVSAConfig::default();
    let mut fs = EmbrFS::new();
    fs.ingest_directory(&input_dir, false, &config).unwrap();

    // Create query vector
    let query_data = vec![42u8; 4096];
    let query_vec = SparseVec::encode_data(&query_data, &config, None);

    // Build index once for all benchmarks
    let codebook_index = fs.engram.build_codebook_index();

    for k in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("top_{}", k)),
            &k,
            |b, &top_k| {
                b.iter(|| {
                    let results = fs.engram.query_codebook_with_index(
                        black_box(&codebook_index),
                        black_box(&query_vec),
                        top_k * 10,
                        top_k,
                    );
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn bench_query_with_path_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_with_path_sweep");

    let temp_dir = std::env::temp_dir().join("embr_bench_query_sweep");
    let input_dir = temp_dir.join("input");
    let _ = fs::remove_dir_all(&temp_dir);

    create_test_corpus(&input_dir, 100, 4096).unwrap();

    let config = ReversibleVSAConfig::default();
    let mut fs = EmbrFS::new();
    fs.ingest_directory(&input_dir, false, &config).unwrap();

    let query_data = vec![42u8; 4096];
    let base_query = SparseVec::encode_data(&query_data, &config, None);

    let codebook_index = fs.engram.build_codebook_index();

    group.bench_function("full_sweep", |b| {
        b.iter(|| {
            let mut merged = std::collections::HashMap::new();

            for depth in 0..config.max_path_depth.max(1) {
                let shift = depth * config.base_shift;
                let query_vec = base_query.permute(shift);

                let matches = fs.engram.query_codebook_with_index(
                    black_box(&codebook_index),
                    black_box(&query_vec),
                    100,
                    20,
                );

                for m in matches {
                    let entry = merged.entry(m.id).or_insert(m.cosine);
                    if m.cosine > *entry {
                        *entry = m.cosine;
                    }
                }
            }

            black_box(merged);
        });
    });

    group.finish();
}

fn bench_query_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_scaling");
    group.sample_size(20);

    for file_count in [50, 100, 200] {
        group.throughput(Throughput::Elements(file_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}files", file_count)),
            &file_count,
            |b, &count| {
                let temp_dir =
                    std::env::temp_dir().join(format!("embr_bench_query_scale_{}", count));
                let input_dir = temp_dir.join("input");
                let _ = fs::remove_dir_all(&temp_dir);

                create_test_corpus(&input_dir, count, 4096).unwrap();

                let config = ReversibleVSAConfig::default();
                let mut fs = EmbrFS::new();
                fs.ingest_directory(&input_dir, false, &config).unwrap();

                let query_data = vec![42u8; 4096];
                let query_vec = SparseVec::encode_data(&query_data, &config, None);

                let codebook_index = fs.engram.build_codebook_index();

                b.iter(|| {
                    let results = fs.engram.query_codebook_with_index(
                        black_box(&codebook_index),
                        black_box(&query_vec),
                        200,
                        20,
                    );
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn bench_codebook_index_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("codebook_index_build");

    for file_count in [50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}files", file_count)),
            &file_count,
            |b, &count| {
                let temp_dir = std::env::temp_dir().join(format!("embr_bench_index_{}", count));
                let input_dir = temp_dir.join("input");
                let _ = fs::remove_dir_all(&temp_dir);

                create_test_corpus(&input_dir, count, 4096).unwrap();

                let config = ReversibleVSAConfig::default();
                let mut fs = EmbrFS::new();
                fs.ingest_directory(&input_dir, false, &config).unwrap();

                b.iter(|| {
                    let index = fs.engram.build_codebook_index();
                    black_box(index);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_query_codebook,
    bench_query_with_path_sweep,
    bench_query_scaling,
    bench_codebook_index_build
);
criterion_main!(benches);
