//! Integration tests for envelope format and compression

use embeddenator_io::*;

#[test]
fn test_legacy_unwrap() {
    let raw = b"hello world";
    let wrapped = wrap_or_legacy(
        PayloadKind::EngramBincode,
        BinaryWriteOptions::default(),
        raw,
    )
    .unwrap();

    // With no compression, should be identical
    assert_eq!(wrapped, raw);

    let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();
    assert_eq!(unwrapped, raw);
}

#[test]
fn test_envelope_header() {
    let raw = b"test payload";

    #[cfg(feature = "compression-zstd")]
    {
        let opts = BinaryWriteOptions {
            codec: CompressionCodec::Zstd,
            level: Some(3),
        };
        let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, raw).unwrap();

        // Check magic number
        assert_eq!(&wrapped[0..4], b"EDN1");
        assert_eq!(wrapped[4], PayloadKind::EngramBincode as u8);
        assert_eq!(wrapped[5], CompressionCodec::Zstd as u8);

        let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();
        assert_eq!(unwrapped, raw);
    }
}

#[test]
fn test_roundtrip_no_compression() {
    let data = b"this is test data without compression";

    let opts = BinaryWriteOptions {
        codec: CompressionCodec::None,
        level: None,
    };

    let wrapped = wrap_or_legacy(PayloadKind::SubEngramBincode, opts, data).unwrap();
    let unwrapped = unwrap_auto(PayloadKind::SubEngramBincode, &wrapped).unwrap();

    assert_eq!(unwrapped, data);
}

#[cfg(feature = "compression-zstd")]
#[test]
fn test_zstd_compression() {
    let data = b"zstd compression test data that should compress well: ".repeat(20);

    let opts = BinaryWriteOptions {
        codec: CompressionCodec::Zstd,
        level: Some(10),
    };

    let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, &data).unwrap();

    // Wrapped should be smaller than original for repetitive data
    assert!(wrapped.len() < data.len(), "Compression should reduce size");

    let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();
    assert_eq!(unwrapped, data);
}

#[cfg(feature = "compression-lz4")]
#[test]
fn test_lz4_compression() {
    let data = b"lz4 compression test data: ".repeat(30);

    let opts = BinaryWriteOptions {
        codec: CompressionCodec::Lz4,
        level: None,
    };

    let wrapped = wrap_or_legacy(PayloadKind::SubEngramBincode, opts, &data).unwrap();

    // Check LZ4 compressed size
    assert!(wrapped.len() < data.len());

    let unwrapped = unwrap_auto(PayloadKind::SubEngramBincode, &wrapped).unwrap();
    assert_eq!(unwrapped, data);
}

#[cfg(any(feature = "compression-zstd", feature = "compression-lz4"))]
#[test]
fn test_payload_kind_mismatch() {
    let data = b"test data";

    let opts = BinaryWriteOptions {
        codec: CompressionCodec::Zstd,
        level: Some(3),
    };

    let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, data).unwrap();

    // Try to unwrap with wrong kind - should error
    let result = unwrap_auto(PayloadKind::SubEngramBincode, &wrapped);
    assert!(result.is_err(), "Should error on payload kind mismatch");
}

#[test]
fn test_large_payload() {
    let data = vec![0xAB; 10_000_000]; // 10 MB

    #[cfg(feature = "compression-zstd")]
    {
        let opts = BinaryWriteOptions {
            codec: CompressionCodec::Zstd,
            level: Some(3),
        };

        let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, &data).unwrap();

        // Highly compressible data (all same byte)
        assert!(
            wrapped.len() < data.len() / 100,
            "Should achieve >99% compression"
        );

        let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();
        assert_eq!(unwrapped.len(), data.len());
        assert_eq!(unwrapped, data);
    }
}

#[cfg(feature = "compression-zstd")]
#[test]
fn test_zstd_levels() {
    let data = b"compression level test".repeat(100);

    for level in [-7, 0, 3, 10, 19] {
        let opts = BinaryWriteOptions {
            codec: CompressionCodec::Zstd,
            level: Some(level),
        };

        let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, &data).unwrap();
        let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();

        assert_eq!(unwrapped, data, "Level {} should roundtrip", level);
    }
}

#[test]
fn test_empty_payload() {
    let data = b"";

    let opts = BinaryWriteOptions::default();
    let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, data).unwrap();
    let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();

    assert_eq!(unwrapped, data);
}

#[test]
fn test_binary_data() {
    let data: Vec<u8> = (0..=255).cycle().take(10_000).collect();

    #[cfg(feature = "compression-lz4")]
    {
        let opts = BinaryWriteOptions {
            codec: CompressionCodec::Lz4,
            level: None,
        };

        let wrapped = wrap_or_legacy(PayloadKind::SubEngramBincode, opts, &data).unwrap();
        let unwrapped = unwrap_auto(PayloadKind::SubEngramBincode, &wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }
}

#[cfg(all(feature = "compression-zstd", feature = "compression-lz4"))]
#[test]
fn test_codec_comparison() {
    let data = b"codec comparison test data with some repetition ".repeat(100);

    let zstd_opts = BinaryWriteOptions {
        codec: CompressionCodec::Zstd,
        level: Some(3),
    };

    let lz4_opts = BinaryWriteOptions {
        codec: CompressionCodec::Lz4,
        level: None,
    };

    let zstd_wrapped = wrap_or_legacy(PayloadKind::EngramBincode, zstd_opts, &data).unwrap();
    let lz4_wrapped = wrap_or_legacy(PayloadKind::EngramBincode, lz4_opts, &data).unwrap();

    println!("Original: {} bytes", data.len());
    println!("Zstd: {} bytes", zstd_wrapped.len());
    println!("LZ4: {} bytes", lz4_wrapped.len());

    // Both should compress
    assert!(zstd_wrapped.len() < data.len());
    assert!(lz4_wrapped.len() < data.len());

    // Verify both roundtrip
    let zstd_unwrapped = unwrap_auto(PayloadKind::EngramBincode, &zstd_wrapped).unwrap();
    let lz4_unwrapped = unwrap_auto(PayloadKind::EngramBincode, &lz4_wrapped).unwrap();

    assert_eq!(zstd_unwrapped, data);
    assert_eq!(lz4_unwrapped, data);
}
