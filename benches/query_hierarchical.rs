use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use embeddenator::{
    query_hierarchical_codebook, EmbrFS, HierarchicalQueryBounds, ReversibleVSAConfig, SparseVec,
};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Create hierarchical test structure with controlled depth and width
fn create_hierarchical_structure(
    dir: &TempDir,
    depth: usize,
    width: usize,
    file_size: usize,
) -> usize {
    let base_path = dir.path();
    let mut total_files = 0;

    fn create_level(
        path: &std::path::Path,
        current_depth: usize,
        max_depth: usize,
        width: usize,
        file_size: usize,
        total_files: &mut usize,
    ) {
        if current_depth >= max_depth {
            return;
        }

        // Create files at this level (reduced to avoid storage issues)
        let files_at_level = width.min(5); // Limit files per level
        for file_idx in 0..files_at_level {
            let file_path = path.join(format!("file_{:04}.txt", file_idx));
            let mut file = fs::File::create(&file_path).unwrap();

            let content = format!(
                "Depth {} File {} Content: {}\n",
                current_depth,
                file_idx,
                "Sample data. ".repeat(file_size / 20) // Smaller content
            );
            file.write_all(content.as_bytes()).unwrap();
            *total_files += 1;
        }

        // Create subdirectories and recurse (limit branching)
        let subdirs = width.min(3); // Further limit directory branching
        for dir_idx in 0..subdirs {
            let subdir = path.join(format!("dir_{:04}", dir_idx));
            fs::create_dir_all(&subdir).unwrap();
            create_level(
                &subdir,
                current_depth + 1,
                max_depth,
                width,
                file_size,
                total_files,
            );
        }
    }

    create_level(base_path, 0, depth, width, file_size, &mut total_files);
    total_files
}

fn bench_hierarchical_query_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("hierarchical_query_depth");

    // Test different hierarchy depths with moderate width
    let depth_configs = vec![
        (2, 5, "depth_2_width_5"),
        (3, 5, "depth_3_width_5"),
        (4, 3, "depth_4_width_3"),
    ];

    for (depth, width, label) in depth_configs {
        group.bench_with_input(
            BenchmarkId::new("query_performance", label),
            &(depth, width),
            |bencher, &(depth, width)| {
                let config = ReversibleVSAConfig::default();

                // Setup: create structure and build hierarchical index
                let temp_dir = TempDir::new().unwrap();
                let _total_files = create_hierarchical_structure(&temp_dir, depth, width, 1024);
                let mut fs = EmbrFS::new();
                fs.ingest_directory(temp_dir.path(), false, &config)
                    .unwrap();

                let hierarchical = fs.bundle_hierarchically(500, false, &config).unwrap();

                // Extract codebook
                let codebook: HashMap<usize, SparseVec> = fs
                    .engram
                    .codebook
                    .iter()
                    .map(|(&id, vec)| (id, vec.clone()))
                    .collect();

                // Create query vector
                let query = SparseVec::encode_data(b"Sample data with patterns", &config, None);

                let bounds = HierarchicalQueryBounds {
                    k: 20,
                    candidate_k: 100,
                    beam_width: 10,
                    max_depth: depth,
                    max_expansions: 1000,
                    max_open_engrams: 100,
                    max_open_indices: 50,
                };

                bencher.iter(|| {
                    let results = query_hierarchical_codebook(
                        black_box(&hierarchical),
                        black_box(&codebook),
                        black_box(&query),
                        black_box(&bounds),
                    );
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

fn bench_hierarchical_query_width(c: &mut Criterion) {
    let mut group = c.benchmark_group("hierarchical_query_width");

    // Test different hierarchy widths with fixed depth
    let width_configs = vec![
        (2, 5, "depth_2_width_5"),
        (2, 10, "depth_2_width_10"),
        (2, 15, "depth_2_width_15"),
    ];

    for (depth, width, label) in width_configs {
        group.bench_with_input(
            BenchmarkId::new("query_performance", label),
            &(depth, width),
            |bencher, &(depth, width)| {
                let config = ReversibleVSAConfig::default();

                // Setup
                let temp_dir = TempDir::new().unwrap();
                let _total_files = create_hierarchical_structure(&temp_dir, depth, width, 512);
                let mut fs = EmbrFS::new();
                fs.ingest_directory(temp_dir.path(), false, &config)
                    .unwrap();

                let hierarchical = fs.bundle_hierarchically(500, false, &config).unwrap();

                let codebook: HashMap<usize, SparseVec> = fs
                    .engram
                    .codebook
                    .iter()
                    .map(|(&id, vec)| (id, vec.clone()))
                    .collect();

                let query = SparseVec::encode_data(b"Sample data with patterns", &config, None);

                let bounds = HierarchicalQueryBounds {
                    k: 20,
                    candidate_k: 100,
                    beam_width: 10,
                    max_depth: depth,
                    max_expansions: 1000,
                    max_open_engrams: 100,
                    max_open_indices: 50,
                };

                bencher.iter(|| {
                    let results = query_hierarchical_codebook(
                        black_box(&hierarchical),
                        black_box(&codebook),
                        black_box(&query),
                        black_box(&bounds),
                    );
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

fn bench_flat_vs_hierarchical(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_vs_hierarchical");

    // Compare flat and hierarchical query performance on same dataset
    let config = ReversibleVSAConfig::default();

    // Setup common structure
    let temp_dir = TempDir::new().unwrap();
    let _total_files = create_hierarchical_structure(&temp_dir, 3, 5, 512);
    let mut fs = EmbrFS::new();
    fs.ingest_directory(temp_dir.path(), false, &config)
        .unwrap();

    let hierarchical = fs.bundle_hierarchically(500, false, &config).unwrap();

    let codebook: HashMap<usize, SparseVec> = fs
        .engram
        .codebook
        .iter()
        .map(|(&id, vec)| (id, vec.clone()))
        .collect();

    let query = SparseVec::encode_data(b"Sample data with patterns", &config, None);

    // Hierarchical query
    let bounds = HierarchicalQueryBounds {
        k: 20,
        candidate_k: 100,
        beam_width: 10,
        max_depth: 3,
        max_expansions: 1000,
        max_open_engrams: 100,
        max_open_indices: 50,
    };

    group.bench_function("hierarchical_query", |bencher| {
        bencher.iter(|| {
            let results = query_hierarchical_codebook(
                black_box(&hierarchical),
                black_box(&codebook),
                black_box(&query),
                black_box(&bounds),
            );
            black_box(results)
        });
    });

    // Flat query using TernaryInvertedIndex for comparison
    use embeddenator::TernaryInvertedIndex;

    let mut flat_index = TernaryInvertedIndex::new();
    for (&chunk_id, vec) in &codebook {
        flat_index.add(chunk_id, vec);
    }
    flat_index.finalize();

    group.bench_function("flat_query", |bencher| {
        bencher.iter(|| {
            let results = flat_index.query_top_k(black_box(&query), 20);
            black_box(results)
        });
    });

    group.finish();
}

fn bench_beam_width_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("beam_width_scaling");

    let config = ReversibleVSAConfig::default();

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let _total_files = create_hierarchical_structure(&temp_dir, 3, 5, 512);
    let mut fs = EmbrFS::new();
    fs.ingest_directory(temp_dir.path(), false, &config)
        .unwrap();

    let hierarchical = fs.bundle_hierarchically(500, false, &config).unwrap();

    let codebook: HashMap<usize, SparseVec> = fs
        .engram
        .codebook
        .iter()
        .map(|(&id, vec)| (id, vec.clone()))
        .collect();

    let query = SparseVec::encode_data(b"Sample data with patterns", &config, None);

    // Test different beam widths
    for beam_width in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("beam_width", beam_width),
            &beam_width,
            |bencher, &beam_width| {
                let bounds = HierarchicalQueryBounds {
                    k: 20,
                    candidate_k: 100,
                    beam_width,
                    max_depth: 3,
                    max_expansions: 1000,
                    max_open_engrams: 100,
                    max_open_indices: 50,
                };

                bencher.iter(|| {
                    let results = query_hierarchical_codebook(
                        black_box(&hierarchical),
                        black_box(&codebook),
                        black_box(&query),
                        black_box(&bounds),
                    );
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_hierarchical_query_depth,
    bench_hierarchical_query_width,
    bench_flat_vs_hierarchical,
    bench_beam_width_scaling
);
criterion_main!(benches);
