use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use embeddenator_retrieval::{
    search::{approximate_search, exact_search, two_stage_search, SearchConfig},
    similarity::SimilarityMetric,
    TernaryInvertedIndex,
};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::collections::HashMap;
use std::hint::black_box;

fn build_corpus(size: usize) -> (TernaryInvertedIndex, HashMap<usize, SparseVec>) {
    let config = ReversibleVSAConfig::default();
    let mut index = TernaryInvertedIndex::new();
    let mut vectors = HashMap::new();

    for i in 0..size {
        let data = format!("document-{:06}", i);
        let vec = SparseVec::encode_data(data.as_bytes(), &config, None);
        index.add(i, &vec);
        vectors.insert(i, vec);
    }

    index.finalize();
    (index, vectors)
}

fn bench_two_stage_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_stage_search");

    let corpus_sizes = [100, 1_000, 10_000];
    let config = ReversibleVSAConfig::default();

    for size in corpus_sizes {
        let (index, vectors) = build_corpus(size);
        let query = SparseVec::encode_data(b"query-test-document", &config, None);
        let search_config = SearchConfig::default();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("corpus_size", size), &size, |b, _| {
            b.iter(|| {
                let results = two_stage_search(
                    black_box(&query),
                    black_box(&index),
                    black_box(&vectors),
                    &search_config,
                    20,
                );
                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_exact_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("exact_search");

    let corpus_sizes = [100, 1_000, 5_000];
    let config = ReversibleVSAConfig::default();

    for size in corpus_sizes {
        let (_index, vectors) = build_corpus(size);
        let query = SparseVec::encode_data(b"query-test-document", &config, None);

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("corpus_size", size), &size, |b, _| {
            b.iter(|| {
                let results = exact_search(
                    black_box(&query),
                    black_box(&vectors),
                    SimilarityMetric::Cosine,
                    20,
                );
                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_approximate_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("approximate_search");

    let corpus_sizes = [100, 1_000, 10_000, 50_000];
    let config = ReversibleVSAConfig::default();

    for size in corpus_sizes {
        let (index, _vectors) = build_corpus(size);
        let query = SparseVec::encode_data(b"query-test-document", &config, None);

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("corpus_size", size), &size, |b, _| {
            b.iter(|| {
                let results = approximate_search(black_box(&query), black_box(&index), 100);
                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_search_different_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_varying_k");

    let config = ReversibleVSAConfig::default();
    let (index, vectors) = build_corpus(10_000);
    let query = SparseVec::encode_data(b"query-test", &config, None);
    let search_config = SearchConfig::default();

    for k in [1, 5, 10, 20, 50, 100] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("k", k), &k, |b, &k| {
            b.iter(|| {
                let results = two_stage_search(
                    black_box(&query),
                    black_box(&index),
                    black_box(&vectors),
                    &search_config,
                    k,
                );
                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_search_candidate_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_varying_candidate_k");

    let config = ReversibleVSAConfig::default();
    let (index, vectors) = build_corpus(10_000);
    let query = SparseVec::encode_data(b"query-test", &config, None);

    for candidate_k in [10, 50, 100, 200, 500] {
        let search_config = SearchConfig {
            candidate_k,
            ..SearchConfig::default()
        };

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("candidate_k", candidate_k),
            &candidate_k,
            |b, _| {
                b.iter(|| {
                    let results = two_stage_search(
                        black_box(&query),
                        black_box(&index),
                        black_box(&vectors),
                        &search_config,
                        20,
                    );
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_index_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_building");

    let corpus_sizes = [100, 1_000, 10_000];
    let config = ReversibleVSAConfig::default();

    for size in corpus_sizes {
        // Pre-generate vectors
        let vectors: Vec<SparseVec> = (0..size)
            .map(|i| {
                let data = format!("document-{:06}", i);
                SparseVec::encode_data(data.as_bytes(), &config, None)
            })
            .collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("corpus_size", size), &size, |b, _| {
            b.iter(|| {
                let mut index = TernaryInvertedIndex::new();
                for (i, vec) in vectors.iter().enumerate() {
                    index.add(i, vec);
                }
                index.finalize();
                black_box(index)
            })
        });
    }

    group.finish();
}

fn bench_search_quality_tradeoff(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_quality_tradeoff");

    let config = ReversibleVSAConfig::default();
    let (index, vectors) = build_corpus(5_000);
    let query = SparseVec::encode_data(b"document-2500", &config, None);

    // Benchmark approximate (fast, less accurate)
    group.bench_function("approximate_only", |b| {
        b.iter(|| {
            let results = approximate_search(black_box(&query), black_box(&index), 20);
            black_box(results)
        })
    });

    // Benchmark two-stage (balanced)
    let search_config = SearchConfig::default();
    group.bench_function("two_stage_default", |b| {
        b.iter(|| {
            let results = two_stage_search(
                black_box(&query),
                black_box(&index),
                black_box(&vectors),
                &search_config,
                20,
            );
            black_box(results)
        })
    });

    // Benchmark exact (slow, most accurate)
    group.bench_function("exact_only", |b| {
        b.iter(|| {
            let results = exact_search(
                black_box(&query),
                black_box(&vectors),
                SimilarityMetric::Cosine,
                20,
            );
            black_box(results)
        })
    });

    group.finish();
}

fn bench_multi_query_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_query_batch");

    let config = ReversibleVSAConfig::default();
    let (index, vectors) = build_corpus(5_000);

    let queries: Vec<SparseVec> = (0..10)
        .map(|i| {
            let data = format!("query-{}", i);
            SparseVec::encode_data(data.as_bytes(), &config, None)
        })
        .collect();

    let search_config = SearchConfig::default();

    group.throughput(Throughput::Elements(10));
    group.bench_function("10_queries", |b| {
        b.iter(|| {
            let results: Vec<_> = queries
                .iter()
                .map(|query| {
                    two_stage_search(
                        black_box(query),
                        black_box(&index),
                        black_box(&vectors),
                        &search_config,
                        20,
                    )
                })
                .collect();
            black_box(results)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_two_stage_search,
    bench_exact_search,
    bench_approximate_search,
    bench_search_different_k,
    bench_search_candidate_k,
    bench_index_building,
    bench_search_quality_tradeoff,
    bench_multi_query_batch,
);
criterion_main!(benches);
