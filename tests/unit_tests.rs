//! Unit tests for Vector Symbolic Architecture (VSA)

use embeddenator::vsa::SparseVec;
use std::collections::HashSet;

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

    assert!(result.pos.len() + result.neg.len() > 0);
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
