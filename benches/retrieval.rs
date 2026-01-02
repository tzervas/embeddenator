use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use embeddenator::{SparseVec, TernaryInvertedIndex};

fn bench_retrieval_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieval_index");

    // Build a deterministic corpus.
    let corpus_sizes = [1_000usize, 5_000];
    for n in corpus_sizes {
        group.bench_with_input(BenchmarkId::new("build", n), &n, |bencher, &n| {
            bencher.iter(|| {
                let mut index = TernaryInvertedIndex::new();
                for i in 0..n {
                    let v = SparseVec::from_data(black_box(format!("doc-{i}").as_bytes()));
                    index.add(i, &v);
                }
                index.finalize();
                black_box(index)
            })
        });

        // Build once for query bench.
        let mut index = TernaryInvertedIndex::new();
        for i in 0..n {
            let v = SparseVec::from_data(format!("doc-{i}").as_bytes());
            index.add(i, &v);
        }
        index.finalize();

        let query = SparseVec::from_data(b"doc-123");
        group.bench_with_input(BenchmarkId::new("query_top_k_20", n), &n, |bencher, &_n| {
            bencher.iter(|| {
                let hits = index.query_top_k(black_box(&query), 20);
                black_box(hits)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_retrieval_index);
criterion_main!(benches);
