use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use embeddenator::{ReversibleVSAConfig, SparseVec};

fn bench_sparsevec_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparsevec_ops");

    // Deterministic vectors for stable benches
    let a = SparseVec::from_data(b"alpha");
    let b = SparseVec::from_data(b"beta");
    let cvec = SparseVec::from_data(b"gamma");

    group.bench_function("bundle", |bencher| {
        bencher.iter(|| black_box(&a).bundle(black_box(&b)))
    });

    group.bench_function("bind", |bencher| {
        bencher.iter(|| black_box(&a).bind(black_box(&b)))
    });

    group.bench_function("cosine", |bencher| {
        bencher.iter(|| black_box(&a).cosine(black_box(&b)))
    });

    group.bench_function("bundle_chain_8", |bencher| {
        bencher.iter(|| {
            let mut acc = black_box(a.clone());
            for _ in 0..7 {
                acc = acc.bundle(black_box(&b));
            }
            black_box(acc)
        })
    });

    group.bench_function("bind_chain_8", |bencher| {
        bencher.iter(|| {
            let mut acc = black_box(a.clone());
            for _ in 0..7 {
                acc = acc.bind(black_box(&b));
            }
            black_box(acc)
        })
    });

    // Ensure we still exercise a non-trivial cosine shape
    group.bench_function("cosine_chain_mix", |bencher| {
        bencher.iter(|| {
            let mixed = black_box(&a).bundle(black_box(&b)).bind(black_box(&cvec));
            black_box(mixed.cosine(black_box(&a)))
        })
    });

    group.finish();
}

fn bench_reversible_encode_decode(c: &mut Criterion) {
    let config = ReversibleVSAConfig::default();

    let sizes = [64usize, 256, 1024, 4096];

    let mut group = c.benchmark_group("reversible_encode_decode");
    for size in sizes {
        let data: Vec<u8> = (0..size).map(|i| (i as u8).wrapping_mul(31)).collect();

        group.bench_with_input(BenchmarkId::new("encode", size), &data, |bencher, data| {
            bencher.iter(|| {
                let v = SparseVec::encode_data(black_box(data), black_box(&config), Some("/bench/path"));
                black_box(v)
            })
        });

        let encoded = SparseVec::encode_data(&data, &config, Some("/bench/path"));
        group.bench_with_input(BenchmarkId::new("decode", size), &encoded, |bencher, encoded| {
            bencher.iter(|| {
                let out = black_box(encoded).decode_data(black_box(&config), Some("/bench/path"), size);
                black_box(out)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_sparsevec_ops, bench_reversible_encode_decode);
criterion_main!(benches);
