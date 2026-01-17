use embeddenator::{PackedTritVec, ReversibleVSAConfig, SparseVec, Trit, DIM};

#[test]
fn packed_roundtrip_sparsevec() {
    let config = ReversibleVSAConfig::default();
    let original = SparseVec::encode_data(b"packed-roundtrip", &config, None);
    let packed = PackedTritVec::from_sparsevec(&original, DIM);
    let decoded = packed.to_sparsevec();
    assert_eq!(decoded.pos, original.pos);
    assert_eq!(decoded.neg, original.neg);
}

#[test]
fn packed_get_set_smoke() {
    let mut v = PackedTritVec::new_zero(128);
    v.set(7, Trit::P);
    v.set(9, Trit::N);
    assert_eq!(v.get(7), Trit::P);
    assert_eq!(v.get(9), Trit::N);
    assert_eq!(v.get(8), Trit::Z);
}

#[test]
fn packed_dot_matches_sparse_intersections() {
    // Dot for SparseVec should equal:
    // |pp| + |nn| - |pn| - |np|
    let config = ReversibleVSAConfig::default();
    let a = SparseVec::encode_data(b"alpha", &config, None);
    let b = SparseVec::encode_data(b"beta", &config, None);

    let packed_a = PackedTritVec::from_sparsevec(&a, DIM);
    let packed_b = PackedTritVec::from_sparsevec(&b, DIM);
    let dot_packed = packed_a.dot(&packed_b);

    let pp = a
        .pos
        .iter()
        .filter(|x| b.pos.binary_search(x).is_ok())
        .count() as i32;
    let nn = a
        .neg
        .iter()
        .filter(|x| b.neg.binary_search(x).is_ok())
        .count() as i32;
    let pn = a
        .pos
        .iter()
        .filter(|x| b.neg.binary_search(x).is_ok())
        .count() as i32;
    let np = a
        .neg
        .iter()
        .filter(|x| b.pos.binary_search(x).is_ok())
        .count() as i32;

    let dot_sparse = (pp + nn) - (pn + np);
    assert_eq!(dot_packed, dot_sparse);
}

#[test]
fn packed_bundle_matches_sparse_bundle() {
    let config = ReversibleVSAConfig::default();
    let a = SparseVec::encode_data(b"bundle-a", &config, None);
    let b = SparseVec::encode_data(b"bundle-b", &config, None);

    let sparse = a.bundle(&b);
    let packed = PackedTritVec::from_sparsevec(&a, DIM)
        .bundle(&PackedTritVec::from_sparsevec(&b, DIM))
        .to_sparsevec();

    assert_eq!(packed.pos, sparse.pos);
    assert_eq!(packed.neg, sparse.neg);
}

#[test]
fn packed_bind_matches_sparse_bind() {
    let config = ReversibleVSAConfig::default();
    let a = SparseVec::encode_data(b"bind-a", &config, None);
    let b = SparseVec::encode_data(b"bind-b", &config, None);

    let sparse = a.bind(&b);
    let packed = PackedTritVec::from_sparsevec(&a, DIM)
        .bind(&PackedTritVec::from_sparsevec(&b, DIM))
        .to_sparsevec();

    assert_eq!(packed.pos, sparse.pos);
    assert_eq!(packed.neg, sparse.neg);
}
