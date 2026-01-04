use embeddenator::{ReversibleVSAConfig, SparseVec};
use embeddenator::signature::{SignatureQueryOptions, TernarySignatureIndex};
use std::collections::HashMap;

#[test]
fn signature_index_candidates_include_self() {
    let cfg = ReversibleVSAConfig::default();

    let q = SparseVec::encode_data(b"doc-0", &cfg, None);
    let other = SparseVec::encode_data(b"doc-1", &cfg, None);

    let mut map = HashMap::new();
    map.insert(0, q.clone());
    map.insert(1, other);

    let idx = TernarySignatureIndex::build_from_map(&map);

    let cand = idx.candidates_with_options(
        &q,
        SignatureQueryOptions {
            max_candidates: 50,
            probe_radius: 1,
            max_probes: 1_000,
        },
    );

    assert!(cand.contains(&0));
}

#[test]
fn signature_index_is_deterministic_over_build() {
    let cfg = ReversibleVSAConfig::default();

    let mut map = HashMap::new();
    for i in 0..50usize {
        map.insert(i, SparseVec::encode_data(format!("doc-{i}").as_bytes(), &cfg, None));
    }

    let a = TernarySignatureIndex::build_from_map(&map);
    let b = TernarySignatureIndex::build_from_map(&map);

    assert_eq!(a.probe_dims(), b.probe_dims());

    let q = SparseVec::encode_data(b"doc-7", &cfg, None);
    let ca = a.candidates_with_options(&q, SignatureQueryOptions { max_candidates: 100, ..SignatureQueryOptions::default() });
    let cb = b.candidates_with_options(&q, SignatureQueryOptions { max_candidates: 100, ..SignatureQueryOptions::default() });

    assert_eq!(ca, cb);
}
