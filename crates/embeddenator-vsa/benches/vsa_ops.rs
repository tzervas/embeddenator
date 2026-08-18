use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec, SparsityScaling, VsaConfig, DIM};

fn bench_sparsevec_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparsevec_ops");

    // Deterministic vectors for stable benches
    let config = ReversibleVSAConfig::default();
    let a = SparseVec::encode_data(b"alpha", &config, None);
    let b = SparseVec::encode_data(b"beta", &config, None);
    let cvec = SparseVec::encode_data(b"gamma", &config, None);

    group.bench_function("bundle", |bencher| {
        bencher.iter(|| std::hint::black_box(&a).bundle(std::hint::black_box(&b)))
    });

    group.bench_function("bind", |bencher| {
        bencher.iter(|| std::hint::black_box(&a).bind(std::hint::black_box(&b)))
    });

    group.bench_function("cosine", |bencher| {
        bencher.iter(|| std::hint::black_box(&a).cosine(std::hint::black_box(&b)))
    });

    group.bench_function("bundle_chain_8", |bencher| {
        bencher.iter(|| {
            let mut acc = std::hint::black_box(a.clone());
            for _ in 0..7 {
                acc = acc.bundle(std::hint::black_box(&b));
            }
            std::hint::black_box(acc)
        })
    });

    group.bench_function("bind_chain_8", |bencher| {
        bencher.iter(|| {
            let mut acc = std::hint::black_box(a.clone());
            for _ in 0..7 {
                acc = acc.bind(std::hint::black_box(&b));
            }
            std::hint::black_box(acc)
        })
    });

    // Ensure we still exercise a non-trivial cosine shape
    group.bench_function("cosine_chain_mix", |bencher| {
        bencher.iter(|| {
            let mixed = std::hint::black_box(&a)
                .bundle(std::hint::black_box(&b))
                .bind(std::hint::black_box(&cvec));
            std::hint::black_box(mixed.cosine(std::hint::black_box(&a)))
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
                let v = SparseVec::encode_data(
                    std::hint::black_box(data),
                    std::hint::black_box(&config),
                    Some("/bench/path"),
                );
                std::hint::black_box(v)
            })
        });

        let encoded = SparseVec::encode_data(&data, &config, Some("/bench/path"));
        group.bench_with_input(
            BenchmarkId::new("decode", size),
            &encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    let out = std::hint::black_box(encoded).decode_data(
                        std::hint::black_box(&config),
                        Some("/bench/path"),
                        size,
                    );
                    std::hint::black_box(out)
                })
            },
        );
    }

    group.finish();
}

fn bench_bundle_modes(c: &mut Criterion) {
    let config = ReversibleVSAConfig::default();

    // Sparse inputs (low collision probability)
    let sa = SparseVec::encode_data(b"sparse-a", &config, None);
    let sb = SparseVec::encode_data(b"sparse-b", &config, None);
    let sc = SparseVec::encode_data(b"sparse-c", &config, None);

    // Dense-ish synthetic inputs to trigger packed/associative paths
    let make_dense = |offset: usize| SparseVec {
        pos: (offset..offset + 4000).step_by(2).collect(),
        neg: (offset + 1..offset + 4000).step_by(2).collect(),
    };
    let da = make_dense(0);
    let db = make_dense(500);
    let dc = make_dense(1000);

    // Mid-density synthetic inputs to probe the packed-threshold boundary.
    // These are sized so that pairwise (A.bundle(B)) may be just below/above DIM/4.
    let make_mid = |offset: usize, span: usize| SparseVec {
        pos: (offset..offset + span).step_by(2).collect(),
        neg: (offset + 1..offset + span).step_by(2).collect(),
    };
    // For DIM=10000, the packed threshold in SparseVec ops is currently DIM/4 (=2500) for
    // a *pairwise* operation. Since each synthetic vector here has nnz ~= span, using span
    // 1200 makes pairwise totals ~2400 (below), and span 1400 makes totals ~2800 (above).
    let ma_lo = make_mid(0, 1200);
    let mb_lo = make_mid(400, 1200);
    let mc_lo = make_mid(800, 1200);
    let ma_hi = make_mid(0, 1400);
    let mb_hi = make_mid(400, 1400);
    let mc_hi = make_mid(800, 1400);

    let mut group = c.benchmark_group("bundle_modes");

    group.bench_function("pairwise_sparse", |bch| {
        bch.iter(|| {
            let acc = std::hint::black_box(&sa)
                .bundle(std::hint::black_box(&sb))
                .bundle(std::hint::black_box(&sc));
            std::hint::black_box(acc)
        })
    });

    group.bench_function("sum_many_sparse", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_sum_many([
                std::hint::black_box(&sa),
                std::hint::black_box(&sb),
                std::hint::black_box(&sc),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("hybrid_sparse", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_hybrid_many([
                std::hint::black_box(&sa),
                std::hint::black_box(&sb),
                std::hint::black_box(&sc),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("pairwise_dense", |bch| {
        bch.iter(|| {
            let acc = std::hint::black_box(&da)
                .bundle(std::hint::black_box(&db))
                .bundle(std::hint::black_box(&dc));
            std::hint::black_box(acc)
        })
    });

    group.bench_function("sum_many_dense", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_sum_many([
                std::hint::black_box(&da),
                std::hint::black_box(&db),
                std::hint::black_box(&dc),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("hybrid_dense", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_hybrid_many([
                std::hint::black_box(&da),
                std::hint::black_box(&db),
                std::hint::black_box(&dc),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("pairwise_mid_lo", |bch| {
        bch.iter(|| {
            let acc = std::hint::black_box(&ma_lo)
                .bundle(std::hint::black_box(&mb_lo))
                .bundle(std::hint::black_box(&mc_lo));
            std::hint::black_box(acc)
        })
    });

    group.bench_function("sum_many_mid_lo", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_sum_many([
                std::hint::black_box(&ma_lo),
                std::hint::black_box(&mb_lo),
                std::hint::black_box(&mc_lo),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("hybrid_mid_lo", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_hybrid_many([
                std::hint::black_box(&ma_lo),
                std::hint::black_box(&mb_lo),
                std::hint::black_box(&mc_lo),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("pairwise_mid_hi", |bch| {
        bch.iter(|| {
            let acc = std::hint::black_box(&ma_hi)
                .bundle(std::hint::black_box(&mb_hi))
                .bundle(std::hint::black_box(&mc_hi));
            std::hint::black_box(acc)
        })
    });

    group.bench_function("sum_many_mid_hi", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_sum_many([
                std::hint::black_box(&ma_hi),
                std::hint::black_box(&mb_hi),
                std::hint::black_box(&mc_hi),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.bench_function("hybrid_mid_hi", |bch| {
        bch.iter(|| {
            let acc = SparseVec::bundle_hybrid_many([
                std::hint::black_box(&ma_hi),
                std::hint::black_box(&mb_hi),
                std::hint::black_box(&mc_hi),
            ]);
            std::hint::black_box(acc)
        })
    });

    group.finish();
}

fn bench_packed_path(c: &mut Criterion) {
    // These benches are intended to deterministically trigger the PackedTritVec fast paths.
    // Fast paths in `SparseVec::{bundle, bind, cosine}`:
    // - bundle/cosine: (a_nnz + b_nnz) > DIM/4 AND min(a_nnz, b_nnz) > DIM/32
    // - bind: a_nnz > DIM/4 AND b_nnz > DIM/4
    // With DIM=10000, DIM/4=2500 and DIM/32=312.

    let make_dense_span = |offset: usize, span: usize| {
        debug_assert!(offset + span <= DIM);
        SparseVec {
            pos: (offset..offset + span).step_by(2).collect(),
            neg: (offset + 1..offset + span).step_by(2).collect(),
        }
    };

    // nnz = span (pos ~= span/2, neg ~= span/2). Picking 8000 ensures we are far above all gates.
    let a = make_dense_span(0, 8000);
    let b = make_dense_span(1000, 8000);

    let mut group = c.benchmark_group("packed_path");

    group.bench_function("bundle_dense_nnz8000_each", |bencher| {
        bencher.iter(|| {
            let out = std::hint::black_box(&a).bundle(std::hint::black_box(&b));
            std::hint::black_box(out)
        })
    });

    group.bench_function("bind_dense_nnz8000_each", |bencher| {
        bencher.iter(|| {
            let out = std::hint::black_box(&a).bind(std::hint::black_box(&b));
            std::hint::black_box(out)
        })
    });

    group.bench_function("cosine_dense_nnz8000_each", |bencher| {
        bencher.iter(|| {
            let sim = std::hint::black_box(&a).cosine(std::hint::black_box(&b));
            std::hint::black_box(sim)
        })
    });

    group.finish();
}

fn bench_dynamic_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_config");

    // Test different accuracy presets
    let configs = vec![
        ("accuracy_90", VsaConfig::for_accuracy(0.90)),
        ("accuracy_95", VsaConfig::for_accuracy(0.95)),
        ("accuracy_99", VsaConfig::for_accuracy(0.99)),
    ];

    for (name, config) in configs {
        let a = SparseVec::random_with_config(&config);
        let b = SparseVec::random_with_config(&config);

        group.bench_with_input(
            BenchmarkId::new("cosine", name),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| std::hint::black_box(a).cosine(std::hint::black_box(b)))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("bundle", name),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| std::hint::black_box(a).bundle(std::hint::black_box(b)))
            },
        );
    }

    group.finish();
}

fn bench_scaling_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_modes");

    let dimension = 50_000;
    let density = 0.01;

    let configs = vec![
        (
            "fixed",
            VsaConfig::new(dimension)
                .with_density(density)
                .with_scaling(SparsityScaling::Fixed),
        ),
        (
            "sqrt",
            VsaConfig::new(dimension)
                .with_density(density)
                .with_scaling(SparsityScaling::SquareRoot),
        ),
        (
            "log",
            VsaConfig::new(dimension)
                .with_density(density)
                .with_scaling(SparsityScaling::Logarithmic),
        ),
    ];

    for (name, config) in configs {
        let sparsity = config.sparsity();
        let a = SparseVec::random_with_config(&config);
        let b = SparseVec::random_with_config(&config);

        group.bench_with_input(
            BenchmarkId::new(format!("cosine_nnz{}", sparsity * 2), name),
            &(&a, &b),
            |bencher, (a, b)| {
                bencher.iter(|| std::hint::black_box(a).cosine(std::hint::black_box(b)))
            },
        );
    }

    group.finish();
}

// GPU benchmarks (only compiled when cuda feature is enabled)
#[cfg(feature = "cuda")]
fn bench_gpu_batch_cosine(c: &mut Criterion) {
    use embeddenator_vsa::{GpuArch, GpuBackend, GpuConfig};

    let mut group = c.benchmark_group("gpu_batch_cosine");

    // Try to initialize GPU backend
    let config = GpuConfig::default().with_architecture(GpuArch::Sm120);
    let gpu_result = GpuBackend::new(config);

    let Ok(gpu) = gpu_result else {
        println!("GPU not available, skipping GPU benchmarks");
        group.finish();
        return;
    };

    println!(
        "GPU initialized with architecture: {:?}",
        gpu.architecture()
    );

    let vsa_config = ReversibleVSAConfig::default();
    let query = SparseVec::encode_data(b"query vector for similarity search", &vsa_config, None);

    // Test various batch sizes to find GPU/CPU crossover point
    let batch_sizes = vec![10, 50, 100, 500, 1000, 5000];

    for batch_size in batch_sizes {
        let candidates: Vec<SparseVec> = (0..batch_size)
            .map(|i| {
                SparseVec::encode_data(
                    format!("document-{}-with-some-content", i).as_bytes(),
                    &vsa_config,
                    None,
                )
            })
            .collect();

        // GPU batch cosine
        group.bench_with_input(
            BenchmarkId::new("gpu", batch_size),
            &(&query, &candidates),
            |bencher, (query, candidates)| {
                bencher.iter(|| {
                    let result = gpu.batch_cosine(
                        std::hint::black_box(query),
                        std::hint::black_box(candidates),
                    );
                    std::hint::black_box(result)
                })
            },
        );

        // CPU batch cosine for comparison
        group.bench_with_input(
            BenchmarkId::new("cpu", batch_size),
            &(&query, &candidates),
            |bencher, (query, candidates)| {
                bencher.iter(|| {
                    let results: Vec<f64> = candidates
                        .iter()
                        .map(|c| std::hint::black_box(query).cosine(std::hint::black_box(c)))
                        .collect();
                    std::hint::black_box(results)
                })
            },
        );
    }

    group.finish();
}

#[cfg(not(feature = "cuda"))]
fn bench_gpu_batch_cosine(_c: &mut Criterion) {
    // GPU benchmarks disabled when cuda feature is not enabled
}

criterion_group!(
    benches,
    bench_sparsevec_ops,
    bench_bundle_modes,
    bench_reversible_encode_decode,
    bench_packed_path,
    bench_dynamic_config,
    bench_scaling_modes,
    bench_gpu_batch_cosine
);
criterion_main!(benches);
