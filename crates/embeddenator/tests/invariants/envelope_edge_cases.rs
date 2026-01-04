//! Edge-case tests for the EDN1 envelope format
//!
//! Tests boundary conditions, error handling, and malformed input rejection.

use embeddenator::envelope::unwrap_auto;
use embeddenator::PayloadKind;

/// Build a minimal valid or invalid EDN1 envelope for testing.
fn make_envelope(
    magic: &[u8],
    kind: u8,
    codec: u8,
    reserved: u16,
    uncompressed_len: u64,
    payload: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(magic);
    out.push(kind);
    out.push(codec);
    out.extend_from_slice(&reserved.to_le_bytes());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

// ---------------------------------------------------------------------------
// Truncated header tests
// ---------------------------------------------------------------------------

#[test]
fn truncated_header_returns_raw_bytes() {
    // Less than 16 bytes should be treated as legacy (raw) data
    let short = b"EDN1xxx";
    let result = unwrap_auto(PayloadKind::EngramBincode, short).expect("should succeed");
    assert_eq!(result, short.as_slice());
}

#[test]
fn exactly_header_with_no_payload_decompresses_to_empty() {
    // 16-byte header with uncompressed_len=0 and no payload should yield empty vec
    let data = make_envelope(b"EDN1", 1, 0, 0, 0, &[]);
    let result = unwrap_auto(PayloadKind::EngramBincode, &data).expect("should succeed");
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Bad magic handling
// ---------------------------------------------------------------------------

#[test]
fn bad_magic_returns_raw_bytes() {
    // Non-EDN1 magic should be treated as legacy raw data
    let data = make_envelope(b"XXXX", 1, 0, 0, 5, b"hello");
    let result = unwrap_auto(PayloadKind::EngramBincode, &data).expect("should succeed");
    assert_eq!(result, data);
}

// ---------------------------------------------------------------------------
// Unknown payload kind
// ---------------------------------------------------------------------------

#[test]
fn unknown_payload_kind_rejected() {
    // PayloadKind byte = 99 is undefined
    let data = make_envelope(b"EDN1", 99, 0, 0, 3, b"abc");
    let err = unwrap_auto(PayloadKind::EngramBincode, &data).unwrap_err();
    assert!(
        err.to_string().contains("unknown") || err.to_string().contains("payload"),
        "error should mention unknown payload kind: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Payload kind mismatch
// ---------------------------------------------------------------------------

#[test]
fn payload_kind_mismatch_rejected() {
    // Header says SubEngramBincode (2), but caller expects EngramBincode (1)
    let data = make_envelope(b"EDN1", 2, 0, 0, 3, b"abc");
    let err = unwrap_auto(PayloadKind::EngramBincode, &data).unwrap_err();
    assert!(
        err.to_string().contains("unexpected") || err.to_string().contains("kind"),
        "error should mention unexpected kind: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Unknown compression codec
// ---------------------------------------------------------------------------

#[test]
fn unknown_codec_rejected() {
    // Codec byte = 99 is undefined
    let data = make_envelope(b"EDN1", 1, 99, 0, 3, b"abc");
    let err = unwrap_auto(PayloadKind::EngramBincode, &data).unwrap_err();
    assert!(
        err.to_string().contains("unknown") || err.to_string().contains("codec"),
        "error should mention unknown codec: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Size mismatch detection (uncompressed codec)
// ---------------------------------------------------------------------------

#[test]
fn size_mismatch_rejected_when_uncompressed() {
    // codec=0 (None), uncompressed_len=10, but payload is only 3 bytes
    let data = make_envelope(b"EDN1", 1, 0, 0, 10, b"abc");
    let err = unwrap_auto(PayloadKind::EngramBincode, &data).unwrap_err();
    assert!(
        err.to_string().contains("size") || err.to_string().contains("mismatch"),
        "error should mention size mismatch: {}",
        err
    );
}

#[test]
fn uncompressed_exact_match_succeeds() {
    // codec=0, uncompressed_len=5, payload exactly 5 bytes
    let data = make_envelope(b"EDN1", 1, 0, 0, 5, b"hello");
    let result = unwrap_auto(PayloadKind::EngramBincode, &data).expect("should succeed");
    assert_eq!(result, b"hello");
}

// ---------------------------------------------------------------------------
// Legacy raw data passthrough
// ---------------------------------------------------------------------------

#[test]
fn legacy_raw_data_passthrough() {
    // Random bytes that don't start with EDN1 should pass through unchanged
    let legacy = b"\x00\x01\x02\x03some binary data here";
    let result = unwrap_auto(PayloadKind::EngramBincode, legacy).expect("should succeed");
    assert_eq!(result, legacy.as_slice());
}

#[test]
fn empty_input_returns_empty() {
    let result = unwrap_auto(PayloadKind::EngramBincode, &[]).expect("should succeed");
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// SubEngram payload kind tests
// ---------------------------------------------------------------------------

#[test]
fn subengram_kind_accepted_when_expected() {
    let data = make_envelope(b"EDN1", 2, 0, 0, 4, b"test");
    let result = unwrap_auto(PayloadKind::SubEngramBincode, &data).expect("should succeed");
    assert_eq!(result, b"test");
}

#[test]
fn engram_kind_rejected_when_subengram_expected() {
    // Header says EngramBincode (1), but caller expects SubEngramBincode (2)
    let data = make_envelope(b"EDN1", 1, 0, 0, 3, b"abc");
    let err = unwrap_auto(PayloadKind::SubEngramBincode, &data).unwrap_err();
    assert!(
        err.to_string().contains("unexpected") || err.to_string().contains("kind"),
        "error should mention unexpected kind: {}",
        err
    );
}
