#[cfg(any(not(feature = "compression-zstd"), not(feature = "compression-lz4")))]
use embeddenator::envelope::unwrap_auto;

#[cfg(any(not(feature = "compression-zstd"), not(feature = "compression-lz4")))]
use embeddenator::PayloadKind;

#[cfg(any(not(feature = "compression-zstd"), not(feature = "compression-lz4")))]
fn make_fake_envelope(
    kind: PayloadKind,
    codec: u8,
    uncompressed_len: u64,
    payload: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(b"EDN1");
    out.push(kind as u8);
    out.push(codec);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

#[cfg(not(feature = "compression-zstd"))]
#[test]
fn unwrap_auto_rejects_zstd_when_feature_missing() {
    // codec=1 (Zstd), kind=EngramBincode, payload is arbitrary since the missing-feature
    // code path fails before attempting real decompression.
    let bytes = make_fake_envelope(PayloadKind::EngramBincode, 1, 3, b"xyz");
    let err = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("zstd") && msg.contains("not enabled"),
        "unexpected error: {msg}"
    );
}

#[cfg(not(feature = "compression-lz4"))]
#[test]
fn unwrap_auto_rejects_lz4_when_feature_missing() {
    // codec=2 (Lz4), kind=EngramBincode.
    let bytes = make_fake_envelope(PayloadKind::EngramBincode, 2, 3, b"xyz");
    let err = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("lz4") && msg.contains("not enabled"),
        "unexpected error: {msg}"
    );
}
