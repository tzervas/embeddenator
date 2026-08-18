//! Test to find the actual bug with prime-sized data

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

#[test]
fn test_prime_sized_encoding() {
    let config = ReversibleVSAConfig::default();

    // Test with prime-sized data to see what's happening
    for size in [13, 17, 23, 31] {
        let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();

        // Encode twice - should be identical
        let vec1 = SparseVec::encode_data(&data, &config, None);
        let vec2 = SparseVec::encode_data(&data, &config, None);

        println!("\n=== Size {} ===", size);
        println!(
            "vec1 pos len: {}, neg len: {}",
            vec1.pos.len(),
            vec1.neg.len()
        );
        println!(
            "vec2 pos len: {}, neg len: {}",
            vec2.pos.len(),
            vec2.neg.len()
        );
        println!(
            "Vectors identical: {}",
            vec1.pos == vec2.pos && vec1.neg == vec2.neg
        );

        // Check self-similarity
        let self_sim = vec1.cosine(&vec1);
        println!("Self-similarity: {}", self_sim);

        // Check cross-similarity
        let cross_sim = vec1.cosine(&vec2);
        println!("Cross-similarity: {}", cross_sim);

        // This should pass if encoding is correct
        assert_eq!(vec1.pos, vec2.pos, "Size {}: pos indices differ", size);
        assert_eq!(vec1.neg, vec2.neg, "Size {}: neg indices differ", size);

        // Self-similarity should be 1.0
        if (self_sim - 1.0).abs() >= 0.01 {
            println!(
                "WARNING: Self-similarity {} is not 1.0 for size {}",
                self_sim, size
            );
            println!("This indicates a bug in cosine similarity calculation");
            println!("pos.len: {}, neg.len: {}", vec1.pos.len(), vec1.neg.len());
            println!("DIM: {}", embeddenator_vsa::DIM);
        }
    }
}

#[test]
fn test_cosine_self_similarity_bug() {
    let config = ReversibleVSAConfig::default();
    let data: Vec<u8> = (0..23).map(|i| (i * 7 + 13) as u8).collect();

    let vec = SparseVec::encode_data(&data, &config, None);

    println!("Testing cosine similarity calculation...");
    println!(
        "Vector: pos.len()={}, neg.len()={}",
        vec.pos.len(),
        vec.neg.len()
    );
    let self_nnz = vec.pos.len() + vec.neg.len();
    println!("nnz: {}", self_nnz);
    println!("DIM: {}", embeddenator_vsa::DIM);

    // Calculate self-similarity using different methods
    let cosine_result = vec.cosine(&vec);
    let cosine_scalar_result = vec.cosine_scalar(&vec);

    println!("cosine(&self): {}", cosine_result);
    println!("cosine_scalar(&self): {}", cosine_scalar_result);

    // Manual calculation
    let pp = count_intersection(&vec.pos, &vec.pos);
    let nn = count_intersection(&vec.neg, &vec.neg);
    let pn = count_intersection(&vec.pos, &vec.neg);
    let np = count_intersection(&vec.neg, &vec.pos);

    println!("Manual calculation:");
    println!("  pp = {} (should be pos.len() = {})", pp, vec.pos.len());
    println!("  nn = {} (should be neg.len() = {})", nn, vec.neg.len());
    println!("  pn = {}", pn);
    println!("  np = {}", np);
    println!(
        "  dot = {} - {} = {}",
        pp + nn,
        pn + np,
        (pp + nn) as i32 - (pn + np) as i32
    );
    println!("  norm = {}", self_nnz);

    let manual_dot = (pp + nn) as i32 - (pn + np) as i32;
    let manual_similarity = manual_dot as f64 / (self_nnz as f64);
    println!("  manual similarity = {}", manual_similarity);

    // For self-similarity, dot should equal nnz (all elements match with themselves)
    assert_eq!(
        pp,
        vec.pos.len(),
        "pp should equal pos.len() for self-similarity"
    );
    assert_eq!(
        nn,
        vec.neg.len(),
        "nn should equal neg.len() for self-similarity"
    );
    assert_eq!(
        pn, 0,
        "pn should be 0 for self-similarity (pos and neg are disjoint)"
    );
    assert_eq!(
        np, 0,
        "np should be 0 for self-similarity (pos and neg are disjoint)"
    );

    let expected_dot = self_nnz as i32;
    assert_eq!(
        manual_dot, expected_dot,
        "dot product with self should equal nnz"
    );

    let expected_similarity =
        expected_dot as f64 / (self_nnz as f64).sqrt() / (self_nnz as f64).sqrt();
    println!("Expected similarity: {}", expected_similarity);

    assert!(
        (expected_similarity - 1.0).abs() < 0.01,
        "Expected similarity should be ~1.0, got {}",
        expected_similarity
    );
}

fn count_intersection(a: &[usize], b: &[usize]) -> usize {
    let mut i = 0;
    let mut j = 0;
    let mut count = 0;

    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                count += 1;
                i += 1;
                j += 1;
            }
        }
    }
    count
}
