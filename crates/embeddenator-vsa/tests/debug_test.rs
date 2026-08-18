//! Simple debugging test to understand encoding behavior

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

#[test]
fn test_encoding_determinism() {
    let config = ReversibleVSAConfig::default();
    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];

    let vec1 = SparseVec::encode_data(&data, &config, None);
    let vec2 = SparseVec::encode_data(&data, &config, None);

    println!("vec1 pos: {:?}", &vec1.pos[..10.min(vec1.pos.len())]);
    println!("vec2 pos: {:?}", &vec2.pos[..10.min(vec2.pos.len())]);

    // Identical data should produce identical vectors
    assert_eq!(vec1.pos, vec2.pos, "pos indices should match");
    assert_eq!(vec1.neg, vec2.neg, "neg indices should match");

    // Self-similarity should be 1.0
    let self_sim = vec1.cosine(&vec1);
    println!("Self-similarity: {}", self_sim);
    assert!(
        (self_sim - 1.0).abs() < 0.01,
        "Self-similarity should be ~1.0, got {}",
        self_sim
    );

    // Cosine between identical encodings should be 1.0
    let cross_sim = vec1.cosine(&vec2);
    println!("Cross-similarity: {}", cross_sim);
    assert!(
        (cross_sim - 1.0).abs() < 0.01,
        "Similarity between identical encodings should be ~1.0, got {}",
        cross_sim
    );
}

#[test]
fn test_self_similarity_various_sizes() {
    let config = ReversibleVSAConfig::default();

    for size in [1, 5, 8, 16, 32, 64] {
        let data: Vec<u8> = (0..size).map(|i| i as u8).collect();
        let vec = SparseVec::encode_data(&data, &config, None);

        let self_sim = vec.cosine(&vec);
        println!("Size {}: self-similarity = {}", size, self_sim);

        assert!(
            (self_sim - 1.0).abs() < 0.01,
            "Size {}: Self-similarity should be ~1.0, got {}",
            size,
            self_sim
        );
    }
}
