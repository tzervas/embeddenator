use embeddenator::{ReversibleVSAConfig, SparseVec};

#[test]
fn query_can_recover_unknown_path_shift_by_sweeping_depth() {
    let config = ReversibleVSAConfig::default();

    // Use multi-block data so the test matches the common chunk encoding shape.
    let data = vec![0xABu8; config.block_size * 4];
    let path = "dir/file.bin";

    let encoded_with_path = SparseVec::encode_data(&data, &config, Some(path));
    let base_query = SparseVec::encode_data(&data, &config, None);

    // Without shift compensation, similarity should generally be lower than perfect.
    let unshifted = base_query.cosine(&encoded_with_path);

    // Sweep the bounded path depth and find the best match.
    let mut best = f64::MIN;
    for depth in 0..config.max_path_depth.max(1) {
        let shift = depth * config.base_shift;
        let candidate = base_query.permute(shift);
        let sim = candidate.cosine(&encoded_with_path);
        if sim > best {
            best = sim;
        }
    }

    // Sweeping should recover the correct shift (or an equivalent), reaching a perfect match.
    assert!(best >= 0.9999, "best={best} unshifted={unshifted}");
}
