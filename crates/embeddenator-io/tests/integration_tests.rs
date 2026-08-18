//! Integration tests for embeddenator-io
//!
//! Tests end-to-end workflows including serialization, buffering,
//! streaming, and envelope formats.

use embeddenator_io::*;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tempfile::tempdir;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct SampleData {
    id: u64,
    name: String,
    values: Vec<f64>,
    metadata: std::collections::HashMap<String, String>,
}

impl SampleData {
    fn sample() -> Self {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("author".to_string(), "test".to_string());
        metadata.insert("version".to_string(), "1.0".to_string());

        SampleData {
            id: 12345,
            name: "test_data".to_string(),
            values: vec![1.0, 2.5, std::f64::consts::PI, 4.0, 5.5],
            metadata,
        }
    }

    fn large() -> Self {
        let mut data = Self::sample();
        // Create larger dataset - using 100 instead of 10,000 to avoid massive debug output
        data.values = (0..100).map(|i| i as f64 * 0.1).collect();
        data
    }
}

#[test]
fn test_bincode_file_roundtrip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.bin");

    let data = SampleData::sample();
    write_bincode_file(&path, &data).unwrap();

    let loaded: SampleData = read_bincode_file(&path).unwrap();
    assert_eq!(data, loaded);
}

#[test]
fn test_json_file_roundtrip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");

    let data = SampleData::sample();
    write_json_file(&path, &data).unwrap();

    let loaded: SampleData = read_json_file(&path).unwrap();
    assert_eq!(data, loaded);
}

#[test]
fn test_envelope_with_compression() {
    let data = b"This is test data that should compress well. ".repeat(100);

    // Test without compression
    let opts_none = BinaryWriteOptions {
        codec: CompressionCodec::None,
        level: None,
    };
    let wrapped_none = wrap_or_legacy(PayloadKind::EngramBincode, opts_none, &data).unwrap();
    let unwrapped_none = unwrap_auto(PayloadKind::EngramBincode, &wrapped_none).unwrap();
    assert_eq!(data, unwrapped_none.as_slice());

    // Test with zstd compression (if feature enabled)
    #[cfg(feature = "compression-zstd")]
    {
        let opts_zstd = BinaryWriteOptions {
            codec: CompressionCodec::Zstd,
            level: Some(3),
        };
        let wrapped_zstd = wrap_or_legacy(PayloadKind::EngramBincode, opts_zstd, &data).unwrap();
        let unwrapped_zstd = unwrap_auto(PayloadKind::EngramBincode, &wrapped_zstd).unwrap();
        assert_eq!(data, unwrapped_zstd.as_slice());
        // Compression should reduce size
        assert!(wrapped_zstd.len() < wrapped_none.len());
    }

    // Test with lz4 compression (if feature enabled)
    #[cfg(feature = "compression-lz4")]
    {
        let opts_lz4 = BinaryWriteOptions {
            codec: CompressionCodec::Lz4,
            level: None,
        };
        let wrapped_lz4 = wrap_or_legacy(PayloadKind::EngramBincode, opts_lz4, &data).unwrap();
        let unwrapped_lz4 = unwrap_auto(PayloadKind::EngramBincode, &wrapped_lz4).unwrap();
        assert_eq!(data, unwrapped_lz4.as_slice());
        assert!(wrapped_lz4.len() < wrapped_none.len());
    }
}

#[test]
fn test_streaming_large_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("large.bin");

    // Write large file in chunks
    let chunk_size = 4096;
    let num_chunks = 100;
    let chunks: Vec<Vec<u8>> = (0..num_chunks).map(|i| vec![i as u8; chunk_size]).collect();

    write_chunks(&path, &chunks).unwrap();

    // Read back and verify
    let mut total_size = 0;
    let mut chunk_count = 0;
    read_chunks(&path, chunk_size, |chunk| {
        total_size += chunk.len();
        chunk_count += 1;
        Ok(())
    })
    .unwrap();

    assert_eq!(total_size, chunk_size * num_chunks);
    assert_eq!(chunk_count, num_chunks);
}

#[test]
fn test_stream_reader_with_large_data() {
    let data = vec![0u8; 100_000]; // 100KB of zeros
    let cursor = Cursor::new(data.clone());
    let mut reader = StreamReader::new(cursor);

    let count = reader.count_bytes().unwrap();
    assert_eq!(count, 100_000);
}

#[test]
fn test_stream_writer_multiple_chunks() {
    let mut buffer = Vec::new();
    let mut writer = StreamWriter::with_buffer_size(&mut buffer, 1024);

    // Write many small chunks
    for i in 0..100 {
        let chunk = format!("Chunk {}\n", i);
        writer.write_chunk(chunk.as_bytes()).unwrap();
    }
    writer.flush().unwrap();

    // Verify content
    let content = String::from_utf8(buffer).unwrap();
    assert!(content.contains("Chunk 0"));
    assert!(content.contains("Chunk 99"));
}

#[test]
fn test_chunk_stream_processing() {
    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let cursor = Cursor::new(data.clone());
    let mut stream = ChunkStream::with_chunk_size(cursor, 100);

    let mut reconstructed = Vec::new();
    while let Some(chunk) = stream.next_chunk().unwrap() {
        reconstructed.extend_from_slice(&chunk);
    }

    assert_eq!(reconstructed, data);
}

#[test]
fn test_buffered_copy() {
    let source_data = b"Source data to copy".repeat(1000);
    let mut src = Cursor::new(source_data.clone());
    let mut dst = Vec::new();

    let bytes_copied = copy_buffered(&mut src, &mut dst, 512).unwrap();

    assert_eq!(bytes_copied, source_data.len() as u64);
    assert_eq!(dst, source_data);
}

#[test]
fn test_mixed_serialization_formats() {
    let dir = tempdir().unwrap();

    let data = SampleData::large();

    // Write as bincode
    let bincode_path = dir.path().join("data.bin");
    write_bincode_file(&bincode_path, &data).unwrap();

    // Write as JSON
    let json_path = dir.path().join("data.json");
    write_json_file(&json_path, &data).unwrap();

    // Verify both can be loaded
    let from_bincode: SampleData = read_bincode_file(&bincode_path).unwrap();
    let from_json: SampleData = read_json_file(&json_path).unwrap();

    // Bincode should preserve exact values
    assert_eq!(data, from_bincode);

    // JSON may lose some floating point precision, so verify structure and approximate values
    assert_eq!(from_json.id, data.id);
    assert_eq!(from_json.name, data.name);
    assert_eq!(from_json.values.len(), data.values.len());
    assert_eq!(from_json.metadata, data.metadata);

    // Verify JSON values are approximately correct (within floating point tolerance)
    for (json_val, orig_val) in from_json.values.iter().zip(data.values.iter()) {
        assert!(
            (json_val - orig_val).abs() < 0.0001,
            "Value mismatch: {} vs {}",
            json_val,
            orig_val
        );
    }

    // Bincode should be more compact
    let bincode_size = std::fs::metadata(&bincode_path).unwrap().len();
    let json_size = std::fs::metadata(&json_path).unwrap().len();
    assert!(bincode_size < json_size);
}

#[test]
fn test_envelope_legacy_fallback() {
    // Data without envelope should be returned as-is
    let raw_data = b"Plain data without envelope header";

    let unwrapped = unwrap_auto(PayloadKind::EngramBincode, raw_data).unwrap();
    assert_eq!(unwrapped, raw_data);
}

#[test]
fn test_empty_data_handling() {
    let empty: Vec<u8> = vec![];

    // Bincode
    let bincode = to_bincode(&empty).unwrap();
    let decoded: Vec<u8> = from_bincode(&bincode).unwrap();
    assert_eq!(decoded, empty);

    // Envelope
    let opts = BinaryWriteOptions::default();
    let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, &empty).unwrap();
    let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();
    assert_eq!(unwrapped.as_slice(), empty.as_slice());
}

#[test]
fn test_stream_fold_accumulator() {
    let data: Vec<u8> = (0..100).collect();
    let cursor = Cursor::new(data.clone());
    let mut reader = StreamReader::with_buffer_size(cursor, 10);

    // Sum all bytes
    let sum = reader
        .fold(0u64, |acc, chunk| {
            let chunk_sum: u64 = chunk.iter().map(|&b| b as u64).sum();
            Ok(acc + chunk_sum)
        })
        .unwrap();

    let expected_sum: u64 = data.iter().map(|&b| b as u64).sum();
    assert_eq!(sum, expected_sum);
}

#[test]
fn test_concurrent_file_operations() {
    use std::thread;

    let dir = tempdir().unwrap();
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let path = dir.path().join(format!("file_{}.bin", i));
            thread::spawn(move || {
                let data = SampleData::sample();
                write_bincode_file(&path, &data).unwrap();
                let loaded: SampleData = read_bincode_file(&path).unwrap();
                assert_eq!(data, loaded);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_error_handling_invalid_bincode() {
    let invalid_data = b"This is not valid bincode";
    let result: Result<SampleData, _> = from_bincode(invalid_data);
    assert!(result.is_err());
}

#[test]
fn test_error_handling_invalid_json() {
    let invalid_json = "{invalid json}";
    let result: Result<SampleData, _> = from_json(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_buffer_sizes() {
    // Test different buffer sizes
    let sizes = [SMALL_BUFFER_SIZE, DEFAULT_BUFFER_SIZE, LARGE_BUFFER_SIZE];

    for size in sizes {
        let data = vec![42u8; size * 2];
        let cursor = Cursor::new(data.clone());
        let mut stream = ChunkStream::with_chunk_size(cursor, size);

        let mut reconstructed = Vec::new();
        while let Some(chunk) = stream.next_chunk().unwrap() {
            reconstructed.extend_from_slice(&chunk);
        }

        assert_eq!(reconstructed, data);
    }
}
