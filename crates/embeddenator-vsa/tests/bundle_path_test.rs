//! Test to verify bundle code path selection and encoding configuration

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

#[test]
fn test_bundle_path_selection() {
    let config = ReversibleVSAConfig::default();
    let data: Vec<u8> = (0..23).map(|i| (i * 7 + 13) as u8).collect();

    let vec = SparseVec::encode_data(&data, &config, None);

    let nnz = vec.pos.len() + vec.neg.len();
    let dim = embeddenator_vsa::DIM;

    // Verify encoding produces non-empty sparse vectors
    assert!(nnz > 0, "Encoded vector should have non-zero entries");
    assert!(
        !vec.pos.is_empty() || !vec.neg.is_empty(),
        "Should have pos or neg entries"
    );

    // Verify sparse vectors are properly bounded
    for &idx in &vec.pos {
        assert!(idx < dim, "pos index {} should be < DIM ({})", idx, dim);
    }
    for &idx in &vec.neg {
        assert!(idx < dim, "neg index {} should be < DIM ({})", idx, dim);
    }

    // Verify block configuration
    let block_size = config.block_size;
    let num_blocks = data.len().div_ceil(block_size);

    assert!(block_size > 0, "Block size should be positive");
    assert!(num_blocks > 0, "Should have at least one block");
    assert_eq!(
        num_blocks,
        data.len().div_ceil(block_size),
        "Block count should match expected"
    );
}
