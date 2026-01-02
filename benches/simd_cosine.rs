//! Benchmark suite for SIMD-accelerated cosine similarity
//!
//! Compares scalar vs SIMD implementations across various vector sizes
//! and sparsity levels.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration, AxisScale};
use embeddenator::{ReversibleVSAConfig, SparseVec};

fn bench_cosine_scalar_vs_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_scalar_vs_simd");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    let config = ReversibleVSAConfig::default();

    // Test with various sparsity levels
    let test_cases = vec![
        ("identical", b"test data 123".as_slice(), b"test data 123".as_slice()),
        ("similar", b"test data 123".as_slice(), b"test data 124".as_slice()),
        ("different", b"test data 123".as_slice(), b"completely different".as_slice()),
        ("short", b"hi".as_slice(), b"hello".as_slice()),
        ("medium", b"the quick brown fox".as_slice(), b"the lazy dog".as_slice()),
        ("long", b"the quick brown fox jumps over the lazy dog again and again".as_slice(),
                 b"the quick brown fox jumps over the lazy dog one more time".as_slice()),
    ];

    for (name, data_a, data_b) in test_cases {
        let a = SparseVec::encode_data(data_a, &config, None);
        let b = SparseVec::encode_data(data_b, &config, None);

        // Benchmark scalar version
        group.bench_with_input(
            BenchmarkId::new("scalar", name),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| black_box(a).cosine_scalar(black_box(b)))
            },
        );

        // Benchmark SIMD version
        #[cfg(feature = "simd")]
        group.bench_with_input(
            BenchmarkId::new("simd", name),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| embeddenator::simd_cosine::cosine_simd(black_box(a), black_box(b)))
            },
        );
    }

    group.finish();
}

fn bench_cosine_synthetic_sparsity(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_synthetic_sparsity");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // Create synthetic vectors with controlled sparsity
    let sparsity_levels = vec![10, 50, 100, 200, 500, 1000, 2000];

    for sparsity in sparsity_levels {
        // Create two vectors with 50% overlap in indices
        let pos_a: Vec<usize> = (0..sparsity).map(|i| i * 2).collect();
        let neg_a: Vec<usize> = (0..sparsity).map(|i| i * 2 + 1).collect();
        
        let pos_b: Vec<usize> = (sparsity/2..sparsity + sparsity/2).map(|i| i * 2).collect();
        let neg_b: Vec<usize> = (sparsity/2..sparsity + sparsity/2).map(|i| i * 2 + 1).collect();

        let a = SparseVec { pos: pos_a, neg: neg_a };
        let b = SparseVec { pos: pos_b, neg: neg_b };

        // Benchmark scalar version
        group.bench_with_input(
            BenchmarkId::new("scalar", sparsity),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| black_box(a).cosine_scalar(black_box(b)))
            },
        );

        // Benchmark SIMD version
        #[cfg(feature = "simd")]
        group.bench_with_input(
            BenchmarkId::new("simd", sparsity),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| embeddenator::simd_cosine::cosine_simd(black_box(a), black_box(b)))
            },
        );
    }

    group.finish();
}

fn bench_cosine_query_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_query_workload");

    let config = ReversibleVSAConfig::default();
    
    // Simulate a realistic query workload: one query vector against many document vectors
    let query = SparseVec::encode_data(b"search query: machine learning embeddings", &config, None);
    
    let document_data: Vec<&[u8]> = vec![
        b"machine learning algorithms",
        b"deep neural networks",
        b"natural language processing",
        b"computer vision techniques",
        b"reinforcement learning",
        b"supervised learning",
        b"gradient descent optimization",
        b"convolutional neural networks",
        b"recurrent neural networks",
        b"transformer architecture",
    ];
    
    let documents: Vec<SparseVec> = document_data
        .iter()
        .map(|data| SparseVec::encode_data(*data, &config, None))
        .collect();

    // Benchmark scalar: compare query against all documents
    group.bench_function("scalar_query_10_docs", |bencher| {
        bencher.iter(|| {
            for doc in &documents {
                black_box(query.cosine_scalar(black_box(doc)));
            }
        })
    });

    // Benchmark SIMD: compare query against all documents
    #[cfg(feature = "simd")]
    group.bench_function("simd_query_10_docs", |bencher| {
        bencher.iter(|| {
            for doc in &documents {
                black_box(embeddenator::simd_cosine::cosine_simd(&query, black_box(doc)));
            }
        })
    });

    group.finish();
}

fn bench_cosine_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_edge_cases");

    // Empty vectors
    let empty = SparseVec { pos: vec![], neg: vec![] };
    let non_empty = SparseVec::from_data(b"test");

    group.bench_function("scalar_empty_vs_empty", |bencher| {
        bencher.iter(|| black_box(&empty).cosine_scalar(black_box(&empty)))
    });

    group.bench_function("scalar_empty_vs_non_empty", |bencher| {
        bencher.iter(|| black_box(&empty).cosine_scalar(black_box(&non_empty)))
    });

    #[cfg(feature = "simd")]
    {
        group.bench_function("simd_empty_vs_empty", |bencher| {
            bencher.iter(|| embeddenator::simd_cosine::cosine_simd(black_box(&empty), black_box(&empty)))
        });

        group.bench_function("simd_empty_vs_non_empty", |bencher| {
            bencher.iter(|| embeddenator::simd_cosine::cosine_simd(black_box(&empty), black_box(&non_empty)))
        });
    }

    // Identical vectors (self-similarity)
    let vec = SparseVec::from_data(b"test data for self-similarity");
    
    group.bench_function("scalar_self_similarity", |bencher| {
        bencher.iter(|| black_box(&vec).cosine_scalar(black_box(&vec)))
    });

    #[cfg(feature = "simd")]
    group.bench_function("simd_self_similarity", |bencher| {
        bencher.iter(|| embeddenator::simd_cosine::cosine_simd(black_box(&vec), black_box(&vec)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cosine_scalar_vs_simd,
    bench_cosine_synthetic_sparsity,
    bench_cosine_query_workload,
    bench_cosine_edge_cases,
);
criterion_main!(benches);
