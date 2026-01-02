use embeddenator::envelope::unwrap_auto;
use embeddenator::PayloadKind;

fn make_envelope(kind: u8, codec: u8, uncompressed_len: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(b"EDN1");
    out.push(kind);
    out.push(codec);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

#[test]
fn unwrap_auto_treats_truncated_envelope_as_legacy_bytes() {
    // Even if the magic prefix matches, unwrap_auto only treats it as an envelope
    // when the full 16-byte header is present.
    let truncated = b"EDN1\x01\x00".to_vec();
    let out = unwrap_auto(PayloadKind::EngramBincode, &truncated).unwrap();
    assert_eq!(out, truncated);
}

#[test]
fn unwrap_auto_rejects_unknown_kind() {
    let bytes = make_envelope(99, 0, 0, b"");
    let err = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap_err();
    assert!(err.to_string().contains("unknown envelope payload kind"));
}

#[test]
fn unwrap_auto_rejects_unexpected_kind() {
    // kind=SubEngramBincode but caller expects EngramBincode.
    let bytes = make_envelope(PayloadKind::SubEngramBincode as u8, 0, 0, b"");
    let err = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap_err();
    assert!(err.to_string().contains("unexpected envelope payload kind"));
}

#[test]
fn unwrap_auto_rejects_unknown_codec() {
    let bytes = make_envelope(PayloadKind::EngramBincode as u8, 99, 0, b"");
    let err = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap_err();
    assert!(err.to_string().contains("unknown envelope compression codec"));
}

#[test]
fn unwrap_auto_rejects_size_mismatch_for_none_codec() {
    // codec=0 (None) but uncompressed_len doesn't match payload length.
    let bytes = make_envelope(PayloadKind::EngramBincode as u8, 0, 5, b"xyz");
    let err = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap_err();
    assert!(err.to_string().contains("envelope size mismatch"));
}

#[test]
fn unwrap_auto_accepts_none_codec_when_sizes_match() {
    let bytes = make_envelope(PayloadKind::EngramBincode as u8, 0, 3, b"xyz");
    let out = unwrap_auto(PayloadKind::EngramBincode, &bytes).unwrap();
    assert_eq!(out, b"xyz");
}
