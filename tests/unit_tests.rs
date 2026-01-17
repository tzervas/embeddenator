//! Unit tests for Vector Symbolic Architecture (VSA)

use embeddenator::resonator::Resonator;
use embeddenator::vsa::ReversibleVSAConfig;
use embeddenator::vsa::SparseVec;
use std::collections::HashSet;

fn enc(data: &[u8]) -> SparseVec {
    SparseVec::encode_data(data, &ReversibleVSAConfig::default(), None)
}

#[test]
fn test_sparse_vec_bundle() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = SparseVec {
        pos: vec![2, 3, 7],
        neg: vec![5, 6, 8],
    };
    let result = v1.bundle(&v2);

    assert!(result.pos.contains(&2));
    assert!(result.pos.contains(&3));
}

#[test]
fn test_sparse_vec_bind() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = SparseVec {
        pos: vec![2, 3, 7],
        neg: vec![5, 6, 8],
    };
    let result = v1.bind(&v2);

    // Only overlapping indices survive; signs multiply.
    // Overlap at {2,3} is +*+ => +, and {5,6} is -* - => +.
    assert_eq!(result.pos, vec![2, 3, 5, 6]);
    assert!(result.neg.is_empty());

    // Outputs are kept sorted.
    assert!(result.pos.windows(2).all(|w| w[0] < w[1]));
    assert!(result.neg.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn test_sparse_vec_bind_disjoint_is_empty() {
    let a = SparseVec {
        pos: vec![1, 3, 5],
        neg: vec![2, 4, 6],
    };
    let b = SparseVec {
        pos: vec![10, 12],
        neg: vec![11, 13],
    };

    let r = a.bind(&b);
    assert!(r.pos.is_empty());
    assert!(r.neg.is_empty());
}

#[test]
fn test_sparse_vec_bind_sign_flip_paths() {
    let a = SparseVec {
        pos: vec![1, 10],
        neg: vec![2, 11],
    };
    let b = SparseVec {
        pos: vec![2, 10],
        neg: vec![1, 11],
    };

    // Index 10: + * + => +
    // Index 11: - * - => +
    // Index 1:  + * - => -
    // Index 2:  - * + => -
    let r = a.bind(&b);
    assert_eq!(r.pos, vec![10, 11]);
    assert_eq!(r.neg, vec![1, 2]);
}

#[test]
fn test_sparse_vec_cosine() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = v1.clone();
    let similarity = v1.cosine(&v2);

    assert!(similarity > 0.9);
}

#[test]
fn test_bundle_associativity() {
    let a = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let b = SparseVec {
        pos: vec![2, 3, 7],
        neg: vec![5, 6, 8],
    };
    let c = SparseVec {
        pos: vec![3, 7, 9],
        neg: vec![6, 8, 10],
    };

    let left = a.bundle(&b).bundle(&c);
    let right = a.bundle(&b.bundle(&c));

    let similarity = left.cosine(&right);
    assert!(
        similarity > 0.7,
        "Bundle associativity failed: similarity = {}",
        similarity
    );
}

#[test]
fn test_bundle_conflict_cancel_non_associative_minimal() {
    // Single-dimension counterexample: (+1) ⊕ (+1) ⊕ (-1)
    let a = SparseVec {
        pos: vec![0],
        neg: Vec::new(),
    };
    let b = SparseVec {
        pos: vec![0],
        neg: Vec::new(),
    };
    let c = SparseVec {
        pos: Vec::new(),
        neg: vec![0],
    };

    let left = a.bundle(&b).bundle(&c); // ((+1)+(+1)) then +(-1) -> 0
    let right = a.bundle(&b.bundle(&c)); // (+1)+(0) -> +1

    assert_eq!(
        left.pos.len() + left.neg.len(),
        0,
        "left should cancel to 0"
    );
    assert_eq!(right.pos, vec![0], "right should keep +1");
}

#[test]
fn test_bundle_sum_many_is_associative_on_three_vectors() {
    let a = SparseVec {
        pos: vec![0],
        neg: Vec::new(),
    };
    let b = SparseVec {
        pos: vec![0],
        neg: Vec::new(),
    };
    let c = SparseVec {
        pos: Vec::new(),
        neg: vec![0],
    };

    let via_two_groups = SparseVec::bundle_sum_many([&a, &b]).bundle(&c); // note: non-associative if using bundle
    let all_at_once = SparseVec::bundle_sum_many([&a, &b, &c]);

    // bundle_sum_many accumulates first, so it keeps +1 for index 0
    assert_eq!(all_at_once.pos, vec![0]);

    // For clarity, the old pairwise bundle path cancels in left grouping.
    assert_eq!(via_two_groups.pos.len() + via_two_groups.neg.len(), 0);

    // As a true associative baseline, compare two different groupings of bundle_sum_many inputs:
    let grouping1 = SparseVec::bundle_sum_many([&a, &b, &c]);
    let grouping2 = SparseVec::bundle_sum_many([&a, &SparseVec::bundle_sum_many([&b, &c])]);
    assert_eq!(grouping1.pos, grouping2.pos);
    assert_eq!(grouping1.neg, grouping2.neg);
}

#[test]
fn test_bundle_hybrid_many_uses_pairwise_when_sparse() {
    // Total nnz = 4 <= DIM/16 (DIM is 100_000), so hybrid should follow pairwise bundle path.
    let a = SparseVec {
        pos: vec![1],
        neg: Vec::new(),
    };
    let b = SparseVec {
        pos: Vec::new(),
        neg: vec![2],
    };
    let c = SparseVec {
        pos: vec![3],
        neg: vec![4],
    };

    let pairwise = a.bundle(&b).bundle(&c);
    let hybrid = SparseVec::bundle_hybrid_many([&a, &b, &c]);

    assert_eq!(hybrid.pos, pairwise.pos);
    assert_eq!(hybrid.neg, pairwise.neg);
}

#[test]
fn test_bundle_hybrid_many_uses_sum_when_dense() {
    // Build dense-ish vectors to trigger the associative branch.
    let make_dense = |offset: usize| SparseVec {
        pos: (offset..offset + 2000).step_by(2).collect(),
        neg: (offset + 1..offset + 2000).step_by(2).collect(),
    };

    let a = make_dense(0);
    let b = make_dense(500);
    let c = make_dense(1000);

    let sum_many = SparseVec::bundle_sum_many([&a, &b, &c]);
    let hybrid = SparseVec::bundle_hybrid_many([&a, &b, &c]);

    assert_eq!(hybrid.pos, sum_many.pos);
    assert_eq!(hybrid.neg, sum_many.neg);
}

#[test]
fn test_bind_self_inverse() {
    let a = SparseVec {
        pos: vec![1, 2, 3, 4, 5],
        neg: vec![6, 7, 8, 9, 10],
    };

    let result = a.bind(&a);
    assert!(!result.pos.is_empty() || !result.neg.is_empty());
}

#[test]
fn test_cosine_similarity_ranges() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = v1.clone();

    let self_sim = v1.cosine(&v2);
    assert!(self_sim > 0.9, "Self-similarity too low: {}", self_sim);

    let v3 = SparseVec {
        pos: vec![10, 20, 30],
        neg: vec![40, 50, 60],
    };
    let diff_sim = v1.cosine(&v3);
    assert!(
        diff_sim < 0.5,
        "Different vectors too similar: {}",
        diff_sim
    );
}

#[test]
#[allow(deprecated)]
fn test_from_data_determinism() {
    let data = b"test data for determinism";
    let v1 = SparseVec::from_data(data);
    let v2 = SparseVec::from_data(data);

    assert_eq!(v1.pos, v2.pos, "pos indices should match");
    assert_eq!(v1.neg, v2.neg, "neg indices should match");

    let similarity = v1.cosine(&v2);
    assert!(
        similarity > 0.999,
        "Determinism failed: identical data produced different vectors (similarity: {})",
        similarity
    );
}

#[test]
#[allow(deprecated)]
fn test_from_data_different_inputs() {
    let data1 = b"first input";
    let data2 = b"second input";

    let v1 = SparseVec::from_data(data1);
    let v2 = SparseVec::from_data(data2);

    assert_ne!(
        v1.pos, v2.pos,
        "Different inputs should produce different pos"
    );

    let similarity = v1.cosine(&v2);
    assert!(
        similarity < 0.5,
        "Different inputs too similar: {}",
        similarity
    );
}

#[test]
fn test_sparse_vec_random() {
    let v = SparseVec::random();

    assert!(
        !v.pos.is_empty(),
        "Random vector should have positive indices"
    );
    assert!(
        !v.neg.is_empty(),
        "Random vector should have negative indices"
    );

    let pos_set: HashSet<_> = v.pos.iter().collect();
    let neg_set: HashSet<_> = v.neg.iter().collect();
    assert!(
        pos_set.is_disjoint(&neg_set),
        "pos and neg should not overlap"
    );
}

#[test]
fn test_cleanup_threshold() {
    let correct = SparseVec {
        pos: vec![1, 2, 3, 4, 5],
        neg: vec![6, 7, 8, 9, 10],
    };

    let similar = SparseVec {
        pos: vec![1, 2, 3, 4, 11],
        neg: vec![6, 7, 8, 9, 12],
    };

    let noise = SparseVec {
        pos: vec![20, 21, 22, 23, 24],
        neg: vec![25, 26, 27, 28, 29],
    };

    let correct_sim = correct.cosine(&similar);
    let noise_sim = correct.cosine(&noise);

    assert!(
        correct_sim > 0.3,
        "Correct match should be >0.3: {}",
        correct_sim
    );
    assert!(noise_sim < 0.3, "Noise should be <0.3: {}", noise_sim);
}

#[test]
fn test_is_text_file() {
    use embeddenator::embrfs::is_text_file;

    let text_data = b"Hello, world!";
    assert!(is_text_file(text_data));

    let binary_data = vec![0u8, 1, 2, 3, 255, 0];
    assert!(!is_text_file(&binary_data));
}

#[test]
fn test_reversible_vsaconfig_default() {
    use embeddenator::vsa::ReversibleVSAConfig;

    let config = ReversibleVSAConfig::default();
    assert_eq!(config.block_size, 256);
    assert_eq!(config.max_path_depth, 10);
    assert_eq!(config.base_shift, 1000);
    assert_eq!(config.target_sparsity, 200);
}

#[test]
fn test_reversible_vsaconfig_presets() {
    use embeddenator::vsa::ReversibleVSAConfig;

    let small = ReversibleVSAConfig::small_blocks();
    assert_eq!(small.block_size, 64);
    assert_eq!(small.target_sparsity, 100);

    let default = ReversibleVSAConfig::default();
    assert_eq!(default.block_size, 256);
    assert_eq!(default.target_sparsity, 200);
}

#[test]
fn test_reversible_vsaconfig_serialization() {
    use embeddenator::vsa::ReversibleVSAConfig;

    let config = ReversibleVSAConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: ReversibleVSAConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(config.block_size, deserialized.block_size);
    assert_eq!(config.max_path_depth, deserialized.max_path_depth);
    assert_eq!(config.base_shift, deserialized.base_shift);
    assert_eq!(config.target_sparsity, deserialized.target_sparsity);
}

#[test]
fn test_permute_identity() {
    let vec = enc(b"test data");
    let permuted = vec.permute(0);

    // permute(0) should be identical
    assert_eq!(vec.pos, permuted.pos);
    assert_eq!(vec.neg, permuted.neg);
}

#[test]
fn test_permute_cycle() {
    let vec = enc(b"test data");
    let permuted = vec.permute(embeddenator::vsa::DIM);

    // permute(DIM) should complete cycle and be identical
    assert_eq!(vec.pos, permuted.pos);
    assert_eq!(vec.neg, permuted.neg);
}

#[test]
fn test_permute_changes_indices() {
    let vec = enc(b"test data");
    let permuted = vec.permute(100);

    // Non-zero shift should change indices (unless all indices happen to map to same positions)
    let pos_changed = vec.pos != permuted.pos;
    let neg_changed = vec.neg != permuted.neg;

    // At least one array should be different (very unlikely both remain identical)
    assert!(pos_changed || neg_changed);

    // But structure should be preserved
    assert_eq!(vec.pos.len(), permuted.pos.len());
    assert_eq!(vec.neg.len(), permuted.neg.len());
}

#[test]
fn test_permute_round_trip() {
    let vec = enc(b"test data");
    let shift = 123;

    let permuted = vec.permute(shift);
    let recovered = permuted.inverse_permute(shift);

    // Round-trip should recover original vector exactly
    assert_eq!(vec.pos, recovered.pos);
    assert_eq!(vec.neg, recovered.neg);
}

#[test]
fn test_permute_orthogonality() {
    let vec = enc(b"test data");

    // Test multiple shifts to ensure orthogonality
    for shift in [100, 500, 1000, 2500] {
        let permuted = vec.permute(shift);
        let similarity = vec.cosine(&permuted);

        // Permuted vectors should be nearly orthogonal to original
        // With DIM=10000 and ~200 non-zero elements, expect very low similarity
        assert!(
            similarity < 0.1,
            "Shift {} gave similarity {}",
            shift,
            similarity
        );
    }
}

#[test]
fn test_thin_reduces_density() {
    // Create a vector with ~400 non-zero elements (twice the target)
    let mut test_vec = SparseVec::new();
    for i in (0..400).step_by(2) {
        test_vec.pos.push(i);
        test_vec.neg.push(i + 1);
    }

    let thinned = test_vec.thin(200);
    let total_elements = thinned.pos.len() + thinned.neg.len();

    // Should reduce to approximately 200 elements
    assert!(
        total_elements <= 200,
        "Expected <= 200 elements, got {}",
        total_elements
    );
    assert!(
        total_elements > 180,
        "Expected > 180 elements, got {}",
        total_elements
    );
}

#[test]
fn test_thin_no_change_when_smaller() {
    // Create a vector with ~200 non-zero elements
    let mut test_vec = SparseVec::new();
    for i in (0..200).step_by(2) {
        test_vec.pos.push(i);
        test_vec.neg.push(i + 1);
    }

    let thinned = test_vec.thin(500); // Target larger than current

    // Should return unchanged
    assert_eq!(test_vec.pos, thinned.pos);
    assert_eq!(test_vec.neg, thinned.neg);
}

#[test]
fn test_bundle_with_config_thinning() {
    use embeddenator::vsa::ReversibleVSAConfig;

    let config = ReversibleVSAConfig::default(); // target_sparsity = 200

    // Create 10 vectors that will bundle to more than 200 non-zeros
    // This test relies on the historical `from_data` density characteristics to
    // ensure bundling exceeds `target_sparsity` and triggers thinning.
    #[allow(deprecated)]
    let vectors: Vec<SparseVec> = (0..10)
        .map(|i| SparseVec::from_data(format!("test data {}", i).as_bytes()))
        .collect();

    // Bundle them all with config
    let mut result = vectors[0].clone();
    for vec in &vectors[1..] {
        result = result.bundle_with_config(vec, Some(&config));
    }

    let total_elements = result.pos.len() + result.neg.len();

    // Should be thinned to approximately 200 elements
    assert!(
        total_elements <= 220,
        "Expected <= 220 elements, got {}",
        total_elements
    );
    assert!(
        total_elements >= 180,
        "Expected >= 180 elements, got {}",
        total_elements
    );
}

#[test]
fn test_resonator_new() {
    let resonator = Resonator::new();
    assert_eq!(resonator.max_iterations, 10);
    assert_eq!(resonator.convergence_threshold, 0.001);
    assert!(resonator.codebook.is_empty());
}

#[test]
fn test_resonator_with_params() {
    let codebook = vec![enc(b"pattern1"), enc(b"pattern2")];
    let resonator = Resonator::with_params(codebook.clone(), 20, 0.0001);
    assert_eq!(resonator.max_iterations, 20);
    assert_eq!(resonator.convergence_threshold, 0.0001);
    assert_eq!(resonator.codebook.len(), 2);
}

#[test]
fn test_resonator_project_clean_input() {
    let clean = enc(b"hello");
    let codebook = vec![clean.clone(), enc(b"world")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);

    // Clean input should project to itself
    let projected = resonator.project(&clean);
    let similarity = clean.cosine(&projected);
    assert!(similarity > 0.9, "Similarity was {}", similarity);
}

#[test]
fn test_resonator_project_empty_codebook() {
    let resonator = Resonator::new();
    let input = enc(b"test");

    // Empty codebook should return input unchanged
    let projected = resonator.project(&input);
    assert_eq!(input.pos, projected.pos);
    assert_eq!(input.neg, projected.neg);
}

#[test]
fn test_resonator_factorize_empty_codebook() {
    let resonator = Resonator::new();
    let compound = enc(b"test");

    let result = resonator.factorize(&compound, 2);
    assert!(result.factors.is_empty());
    assert_eq!(result.iterations, 0);
    assert_eq!(result.final_delta, 0.0);
}

#[test]
fn test_resonator_factorize_zero_factors() {
    let codebook = vec![enc(b"pattern1")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);
    let compound = enc(b"test");

    let result = resonator.factorize(&compound, 0);
    assert!(result.factors.is_empty());
    assert_eq!(result.iterations, 0);
    assert_eq!(result.final_delta, 0.0);
}

#[test]
fn test_resonator_factorize_convergence() {
    let factor1 = enc(b"hello");
    let factor2 = enc(b"world");
    let compound = factor1.bundle(&factor2);

    let codebook = vec![factor1.clone(), factor2.clone()];
    let resonator = Resonator::with_params(codebook, 20, 0.001);

    let result = resonator.factorize(&compound, 2);

    // Should return 2 factors
    assert_eq!(result.factors.len(), 2);
    // Should converge within reasonable iterations
    assert!(result.iterations <= 20);
    // Final delta should be reasonable
    assert!(result.final_delta >= 0.0);
    assert!(result.final_delta < 1.0);
}

#[test]
fn test_resonator_sign_threshold() {
    let resonator = Resonator::new();
    let similarities = vec![0.8, -0.3, 0.05, -0.9, 0.0];
    let ternary = resonator.sign_threshold(&similarities, 0.1);

    assert_eq!(ternary, vec![1, -1, 0, -1, 0]);
}

#[test]
fn test_resonator_sign_threshold_zero_threshold() {
    let resonator = Resonator::new();
    let similarities = vec![0.1, -0.1, 0.0];
    let ternary = resonator.sign_threshold(&similarities, 0.0);

    // With zero threshold, all non-zero values should be thresholded
    assert_eq!(ternary, vec![1, -1, 0]);
}

#[test]
fn test_resonator_sign_threshold_high_threshold() {
    let resonator = Resonator::new();
    let similarities = vec![0.5, -0.5, 0.05];
    let ternary = resonator.sign_threshold(&similarities, 0.6);

    // With high threshold, only strong similarities should pass
    assert_eq!(ternary, vec![0, 0, 0]);
}

#[test]
fn test_embrfs_resonator_integration() {
    use embeddenator::embrfs::EmbrFS;
    use embeddenator::vsa::{ReversibleVSAConfig, SparseVec};
    use tempfile::tempdir;

    let mut embrfs = EmbrFS::new();
    let resonator = Resonator::new();
    let config = ReversibleVSAConfig::default();
    embrfs.set_resonator(resonator);

    // Add a test file to the embrfs
    let test_data = b"Hello, World!";
    let file_entry = embeddenator::embrfs::FileEntry {
        path: "test.txt".to_string(),
        is_text: true,
        size: test_data.len(),
        chunks: vec![0],
        deleted: false,
    };
    embrfs.manifest.files.push(file_entry);
    embrfs.manifest.total_chunks = 1;
    // Create a SparseVec from the data for the codebook
    let chunk_vec = SparseVec::encode_data(&test_data[..], &config, Some("test.txt"));
    embrfs.engram.codebook.insert(0, chunk_vec);

    // Test extraction with resonator
    let temp_dir = tempdir().unwrap();
    let result = embrfs.extract_with_resonator(temp_dir.path(), false, &config);
    assert!(result.is_ok());

    // Verify file was extracted
    let extracted_path = temp_dir.path().join("test.txt");
    assert!(extracted_path.exists());
}

#[test]
fn test_embrfs_without_resonator_fallback() {
    use embeddenator::embrfs::EmbrFS;
    use embeddenator::vsa::{ReversibleVSAConfig, SparseVec};
    use tempfile::tempdir;

    let mut embrfs = EmbrFS::new(); // No resonator set
    let config = ReversibleVSAConfig::default();

    // Add a test file to the embrfs
    let test_data = b"Hello, World!";
    let file_entry = embeddenator::embrfs::FileEntry {
        path: "test.txt".to_string(),
        is_text: true,
        size: test_data.len(),
        chunks: vec![0],
        deleted: false,
    };
    embrfs.manifest.files.push(file_entry);
    embrfs.manifest.total_chunks = 1;
    // Create a SparseVec from the data for the codebook
    let chunk_vec = SparseVec::encode_data(&test_data[..], &config, Some("test.txt"));
    embrfs.engram.codebook.insert(0, chunk_vec);

    // Test extraction without resonator (should use standard extract)
    let temp_dir = tempdir().unwrap();
    let result = embrfs.extract_with_resonator(temp_dir.path(), false, &config);
    assert!(result.is_ok());

    // Verify file was extracted
    let extracted_path = temp_dir.path().join("test.txt");
    assert!(extracted_path.exists());
}

#[test]
fn test_hierarchical_bundling() {
    use embeddenator::embrfs::EmbrFS;
    use embeddenator::vsa::{ReversibleVSAConfig, SparseVec};

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add test files with hierarchical paths
    let test_files = vec![
        ("dir1/file1.txt", b"content1"),
        ("dir1/file2.txt", b"content2"),
        ("dir1/subdir/file3.txt", b"content3"),
        ("dir2/file4.txt", b"content4"),
    ];

    for (path, content) in test_files {
        let file_entry = embeddenator::embrfs::FileEntry {
            path: path.to_string(),
            is_text: true,
            size: content.len(),
            chunks: vec![fs.manifest.total_chunks],
            deleted: false,
        };
        fs.manifest.files.push(file_entry);
        // Create a SparseVec from the content for the codebook
        let chunk_vec = SparseVec::encode_data(&content[..], &config, Some(path));
        fs.engram
            .codebook
            .insert(fs.manifest.total_chunks, chunk_vec);
        fs.manifest.total_chunks += 1;
    }

    // Test hierarchical bundling
    let hierarchical = fs.bundle_hierarchically(200, false, &config);
    assert!(hierarchical.is_ok());

    let manifest = hierarchical.unwrap();
    assert_eq!(manifest.version, 1);
    assert!(manifest.levels.len() > 0);

    // Should have sub-engrams for components
    assert!(manifest.sub_engrams.len() > 0);

    // Verify that different levels have different structures
    for level in &manifest.levels {
        assert!(level.items.len() > 0);
        for item in &level.items {
            assert!(manifest.sub_engrams.contains_key(&item.sub_engram_id));
        }
    }
}

#[test]
fn test_hierarchical_extraction() {
    use embeddenator::embrfs::EmbrFS;
    use embeddenator::vsa::{ReversibleVSAConfig, SparseVec};
    use tempfile::tempdir;

    let mut fs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();

    // Add test files with hierarchical paths
    let test_files = vec![
        ("dir1/file1.txt", b"content1"),
        ("dir1/file2.txt", b"content2"),
        ("dir1/subdir/file3.txt", b"content3"),
        ("dir2/file4.txt", b"content4"),
    ];

    for (path, content) in &test_files {
        let file_entry = embeddenator::embrfs::FileEntry {
            path: path.to_string(),
            is_text: true,
            size: content.len(),
            chunks: vec![fs.manifest.total_chunks],
            deleted: false,
        };
        fs.manifest.files.push(file_entry);
        // Create a SparseVec from the content for the codebook
        let chunk_vec = SparseVec::encode_data(&content[..], &config, Some(*path));
        fs.engram
            .codebook
            .insert(fs.manifest.total_chunks, chunk_vec);
        fs.manifest.total_chunks += 1;
    }

    // Create hierarchical manifest
    let hierarchical = fs.bundle_hierarchically(200, false, &config).unwrap();

    // Test hierarchical extraction
    let temp_dir = tempdir().unwrap();
    let result = fs.extract_hierarchically(&hierarchical, temp_dir.path(), false, &config);
    assert!(result.is_ok());

    // Verify files were extracted (even if content is transformed)
    for (path, _) in &test_files {
        let extracted_path = temp_dir.path().join(path);
        assert!(extracted_path.exists(), "File {} should exist", path);
    }
}
