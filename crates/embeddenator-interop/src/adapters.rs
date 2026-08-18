//! Adapter layers for external library integration.
//!
//! This module provides adapters to integrate embeddenator with external
//! libraries and tools, handling format conversions and API bridging.

use embeddenator_fs::{Engram, Manifest};
use embeddenator_io::{BinaryWriteOptions, CompressionCodec, PayloadKind};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::io;
use std::path::Path;

// Helper functions that wrap embeddenator_io functionality
fn write_json_to_file<P: AsRef<Path>, T: serde::Serialize>(path: P, value: &T) -> io::Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    std::fs::write(path, json)
}

fn read_json_from_file<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> io::Result<T> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data).map_err(io::Error::other)
}

fn write_bincode_to_file<P: AsRef<Path>, T: serde::Serialize>(
    path: P,
    value: &T,
) -> io::Result<()> {
    let bytes = bincode::serialize(value).map_err(io::Error::other)?;
    std::fs::write(path, bytes)
}

fn read_bincode_from_file<P: AsRef<Path>, T: serde::de::DeserializeOwned>(
    path: P,
) -> io::Result<T> {
    let data = std::fs::read(path)?;
    bincode::deserialize(&data).map_err(io::Error::other)
}

fn wrap_with_envelope(
    kind: PayloadKind,
    opts: BinaryWriteOptions,
    data: &[u8],
) -> io::Result<Vec<u8>> {
    // Use embeddenator-io's full envelope support with compression
    embeddenator_io::wrap_or_legacy(kind, opts, data)
}

fn unwrap_from_envelope(kind: PayloadKind, data: &[u8]) -> io::Result<Vec<u8>> {
    // Use embeddenator-io's full envelope unwrapping with decompression
    embeddenator_io::unwrap_auto(kind, data)
}

/// Adapter for embeddenator-io envelope format
pub struct EnvelopeAdapter;

impl EnvelopeAdapter {
    /// Wrap an Engram in envelope format with compression
    pub fn wrap_engram(
        engram: &Engram,
        codec: CompressionCodec,
        level: Option<i32>,
    ) -> io::Result<Vec<u8>> {
        let serialized = bincode::serialize(engram).map_err(io::Error::other)?;
        let opts = BinaryWriteOptions { codec, level };
        wrap_with_envelope(PayloadKind::EngramBincode, opts, &serialized)
    }

    /// Unwrap an Engram from envelope format
    pub fn unwrap_engram(data: &[u8]) -> io::Result<Engram> {
        let decoded = unwrap_from_envelope(PayloadKind::EngramBincode, data)?;
        bincode::deserialize(&decoded).map_err(io::Error::other)
    }

    /// Wrap a SubEngram in envelope format with compression
    pub fn wrap_sub_engram(
        sub: &embeddenator_fs::SubEngram,
        codec: CompressionCodec,
        level: Option<i32>,
    ) -> io::Result<Vec<u8>> {
        let serialized = bincode::serialize(sub).map_err(io::Error::other)?;
        let opts = BinaryWriteOptions { codec, level };
        wrap_with_envelope(PayloadKind::SubEngramBincode, opts, &serialized)
    }

    /// Unwrap a SubEngram from envelope format
    pub fn unwrap_sub_engram(data: &[u8]) -> io::Result<embeddenator_fs::SubEngram> {
        let decoded = unwrap_from_envelope(PayloadKind::SubEngramBincode, data)?;
        bincode::deserialize(&decoded).map_err(io::Error::other)
    }
}

/// Adapter for file-based operations
pub struct FileAdapter;

impl FileAdapter {
    /// Save Engram to file with envelope format
    pub fn save_engram<P: AsRef<Path>>(
        path: P,
        engram: &Engram,
        codec: CompressionCodec,
    ) -> io::Result<()> {
        let wrapped = EnvelopeAdapter::wrap_engram(engram, codec, None)?;
        std::fs::write(path, wrapped)
    }

    /// Load Engram from file with envelope format
    pub fn load_engram<P: AsRef<Path>>(path: P) -> io::Result<Engram> {
        let data = std::fs::read(path)?;
        EnvelopeAdapter::unwrap_engram(&data)
    }

    /// Save Manifest to JSON file
    pub fn save_manifest<P: AsRef<Path>>(path: P, manifest: &Manifest) -> io::Result<()> {
        write_json_to_file(path, manifest)
    }

    /// Load Manifest from JSON file
    pub fn load_manifest<P: AsRef<Path>>(path: P) -> io::Result<Manifest> {
        read_json_from_file(path)
    }

    /// Save SparseVec to bincode file
    pub fn save_sparse_vec<P: AsRef<Path>>(path: P, vec: &SparseVec) -> io::Result<()> {
        write_bincode_to_file(path, vec)
    }

    /// Load SparseVec from bincode file
    pub fn load_sparse_vec<P: AsRef<Path>>(path: P) -> io::Result<SparseVec> {
        read_bincode_from_file(path)
    }

    /// Save VSAConfig to JSON file
    pub fn save_vsa_config<P: AsRef<Path>>(
        path: P,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        write_json_to_file(path, config)
    }

    /// Load VSAConfig from JSON file
    pub fn load_vsa_config<P: AsRef<Path>>(path: P) -> io::Result<ReversibleVSAConfig> {
        read_json_from_file(path)
    }
}

/// Adapter for streaming operations
pub struct StreamAdapter;

impl StreamAdapter {
    /// Stream process data: encode in chunks
    pub fn stream_encode<R: io::Read>(
        mut reader: R,
        config: &ReversibleVSAConfig,
        chunk_size: usize,
    ) -> io::Result<Vec<SparseVec>> {
        let mut vectors = Vec::new();
        let mut buffer = vec![0u8; chunk_size];

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }

            let vec = SparseVec::encode_data(&buffer[..n], config, None);
            vectors.push(vec);
        }

        Ok(vectors)
    }

    /// Stream process vectors: decode and write
    pub fn stream_decode<W: io::Write>(
        vectors: &[SparseVec],
        config: &ReversibleVSAConfig,
        expected_size: usize,
        mut writer: W,
    ) -> io::Result<()> {
        for vec in vectors {
            let decoded = vec.decode_data(config, None, expected_size);
            writer.write_all(&decoded)?;
        }
        Ok(())
    }
}

/// Adapter for batch operations
pub struct BatchAdapter;

impl BatchAdapter {
    /// Batch encode multiple data chunks
    pub fn batch_encode(data_chunks: &[&[u8]], config: &ReversibleVSAConfig) -> Vec<SparseVec> {
        data_chunks
            .iter()
            .map(|chunk| SparseVec::encode_data(chunk, config, None))
            .collect()
    }

    /// Batch decode multiple vectors
    pub fn batch_decode(
        vectors: &[SparseVec],
        config: &ReversibleVSAConfig,
        expected_size: usize,
    ) -> Vec<Vec<u8>> {
        vectors
            .iter()
            .map(|vec| vec.decode_data(config, None, expected_size))
            .collect()
    }

    /// Batch compute similarities
    pub fn batch_similarity(query: &SparseVec, vectors: &[SparseVec]) -> Vec<f64> {
        vectors.iter().map(|vec| query.cosine(vec)).collect()
    }

    /// Batch bundle vectors
    pub fn batch_bundle(vectors: &[SparseVec]) -> Option<SparseVec> {
        if vectors.is_empty() {
            return None;
        }

        let mut result = vectors[0].clone();
        for vec in &vectors[1..] {
            result = result.bundle(vec);
        }
        Some(result)
    }
}

/// Adapter for format detection and auto-conversion
pub struct AutoFormatAdapter;

impl AutoFormatAdapter {
    /// Try to load Engram with automatic format detection
    pub fn auto_load_engram<P: AsRef<Path>>(path: P) -> io::Result<Engram> {
        let data = std::fs::read(path)?;

        // Try envelope format first
        if let Ok(engram) = EnvelopeAdapter::unwrap_engram(&data) {
            return Ok(engram);
        }

        // Try raw bincode
        if let Ok(engram) = bincode::deserialize::<Engram>(&data) {
            return Ok(engram);
        }

        // Try JSON
        if let Ok(json_str) = std::str::from_utf8(&data) {
            if let Ok(engram) = serde_json::from_str::<Engram>(json_str) {
                return Ok(engram);
            }
        }

        Err(io::Error::other("unable to detect engram format"))
    }

    /// Try to load Manifest with automatic format detection
    pub fn auto_load_manifest<P: AsRef<Path>>(path: P) -> io::Result<Manifest> {
        let data = std::fs::read(path)?;

        // Try JSON first
        if let Ok(json_str) = std::str::from_utf8(&data) {
            if let Ok(manifest) = serde_json::from_str::<Manifest>(json_str) {
                return Ok(manifest);
            }
        }

        // Try bincode
        if let Ok(manifest) = bincode::deserialize::<Manifest>(&data) {
            return Ok(manifest);
        }

        Err(io::Error::other("unable to detect manifest format"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_envelope_adapter_engram() {
        use embeddenator_fs::CorrectionStore;

        // Create a minimal engram
        let engram = Engram {
            root: SparseVec {
                pos: vec![1, 2, 3],
                neg: vec![],
            },
            codebook: std::collections::HashMap::new(),
            corrections: CorrectionStore::default(),
        };

        // Test with no compression
        let wrapped = EnvelopeAdapter::wrap_engram(&engram, CompressionCodec::None, None).unwrap();
        let unwrapped = EnvelopeAdapter::unwrap_engram(&wrapped).unwrap();
        assert_eq!(engram.root.pos, unwrapped.root.pos);
        assert_eq!(engram.root.neg, unwrapped.root.neg);
    }

    #[test]
    #[cfg(feature = "compression-zstd")]
    fn test_envelope_adapter_zstd_compression() {
        use embeddenator_fs::CorrectionStore;

        // Create an engram with more data to compress
        let mut codebook = std::collections::HashMap::new();
        for i in 0..100 {
            codebook.insert(
                i,
                SparseVec {
                    pos: vec![i, i + 1, i + 2],
                    neg: vec![i + 3, i + 4],
                },
            );
        }

        let engram = Engram {
            root: SparseVec {
                pos: (0..50).collect(),
                neg: (50..100).collect(),
            },
            codebook,
            corrections: CorrectionStore::default(),
        };

        // Test with zstd compression
        let wrapped =
            EnvelopeAdapter::wrap_engram(&engram, CompressionCodec::Zstd, Some(3)).unwrap();
        let unwrapped = EnvelopeAdapter::unwrap_engram(&wrapped).unwrap();

        assert_eq!(engram.root.pos, unwrapped.root.pos);
        assert_eq!(engram.root.neg, unwrapped.root.neg);
        assert_eq!(engram.codebook.len(), unwrapped.codebook.len());

        // Verify compression worked (compressed should be smaller)
        let uncompressed = bincode::serialize(&engram).unwrap();
        println!(
            "Uncompressed size: {}, Compressed size: {}",
            uncompressed.len(),
            wrapped.len()
        );
        assert!(wrapped.len() < uncompressed.len());
    }

    #[test]
    #[cfg(feature = "compression-lz4")]
    fn test_envelope_adapter_lz4_compression() {
        use embeddenator_fs::CorrectionStore;

        // Create an engram with repeating patterns (good for LZ4)
        let mut codebook = std::collections::HashMap::new();
        for i in 0..50 {
            codebook.insert(
                i,
                SparseVec {
                    pos: vec![1, 2, 3, 4, 5],
                    neg: vec![6, 7, 8],
                },
            );
        }

        let engram = Engram {
            root: SparseVec {
                pos: vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 5],
                neg: vec![6, 7, 8, 6, 7, 8],
            },
            codebook,
            corrections: CorrectionStore::default(),
        };

        // Test with lz4 compression
        let wrapped = EnvelopeAdapter::wrap_engram(&engram, CompressionCodec::Lz4, None).unwrap();
        let unwrapped = EnvelopeAdapter::unwrap_engram(&wrapped).unwrap();

        assert_eq!(engram.root.pos, unwrapped.root.pos);
        assert_eq!(engram.root.neg, unwrapped.root.neg);
        assert_eq!(engram.codebook.len(), unwrapped.codebook.len());

        // Verify compression worked
        let uncompressed = bincode::serialize(&engram).unwrap();
        println!(
            "Uncompressed size: {}, Compressed size: {}",
            uncompressed.len(),
            wrapped.len()
        );
        assert!(wrapped.len() < uncompressed.len());
    }

    #[test]
    fn test_envelope_adapter_sub_engram() {
        use embeddenator_fs::SubEngram;

        let sub = SubEngram {
            id: "test_sub".to_string(),
            root: SparseVec {
                pos: vec![10, 20, 30],
                neg: vec![40, 50],
            },
            chunk_ids: vec![1, 2, 3],
            chunk_count: 3,
            children: vec![],
        };

        // Test with no compression
        let wrapped = EnvelopeAdapter::wrap_sub_engram(&sub, CompressionCodec::None, None).unwrap();
        let unwrapped = EnvelopeAdapter::unwrap_sub_engram(&wrapped).unwrap();
        assert_eq!(sub.root.pos, unwrapped.root.pos);
        assert_eq!(sub.root.neg, unwrapped.root.neg);
        assert_eq!(sub.id, unwrapped.id);
    }

    #[test]
    fn test_file_adapter_with_compression() {
        use embeddenator_fs::CorrectionStore;
        let dir = tempdir().unwrap();

        let engram = Engram {
            root: SparseVec {
                pos: vec![1, 2, 3],
                neg: vec![4, 5],
            },
            codebook: std::collections::HashMap::new(),
            corrections: CorrectionStore::default(),
        };

        // Test saving and loading with no compression
        let path = dir.path().join("engram.bin");
        FileAdapter::save_engram(&path, &engram, CompressionCodec::None).unwrap();
        let loaded = FileAdapter::load_engram(&path).unwrap();
        assert_eq!(engram.root.pos, loaded.root.pos);
        assert_eq!(engram.root.neg, loaded.root.neg);
    }

    #[test]
    #[cfg(feature = "compression")]
    fn test_compression_round_trip() {
        use embeddenator_fs::CorrectionStore;

        let engram = Engram {
            root: SparseVec {
                pos: (0..100).collect(),
                neg: (100..200).collect(),
            },
            codebook: std::collections::HashMap::new(),
            corrections: CorrectionStore::default(),
        };

        // Test all compression codecs
        for codec in &[
            CompressionCodec::None,
            CompressionCodec::Zstd,
            CompressionCodec::Lz4,
        ] {
            let wrapped = EnvelopeAdapter::wrap_engram(&engram, *codec, Some(3)).unwrap();
            let unwrapped = EnvelopeAdapter::unwrap_engram(&wrapped).unwrap();
            assert_eq!(
                engram.root.pos, unwrapped.root.pos,
                "Failed for codec {:?}",
                codec
            );
            assert_eq!(
                engram.root.neg, unwrapped.root.neg,
                "Failed for codec {:?}",
                codec
            );
        }
    }

    #[test]
    fn test_file_adapter() {
        let dir = tempdir().unwrap();

        // Test sparse vec
        let vec = SparseVec {
            pos: vec![1, 2, 3],
            neg: vec![4, 5],
        };
        let vec_path = dir.path().join("vec.bin");
        FileAdapter::save_sparse_vec(&vec_path, &vec).unwrap();
        let loaded = FileAdapter::load_sparse_vec(&vec_path).unwrap();
        assert_eq!(vec.pos, loaded.pos);
        assert_eq!(vec.neg, loaded.neg);

        // Test config
        let config = ReversibleVSAConfig::default();
        let config_path = dir.path().join("config.json");
        FileAdapter::save_vsa_config(&config_path, &config).unwrap();
        let loaded_config = FileAdapter::load_vsa_config(&config_path).unwrap();
        assert_eq!(config.block_size, loaded_config.block_size);
        assert_eq!(config.max_path_depth, loaded_config.max_path_depth);
    }

    #[test]
    fn test_batch_adapter() {
        let config = ReversibleVSAConfig::default();
        let data_chunks = vec![b"hello".as_slice(), b"world".as_slice()];

        // Batch encode
        let vectors = BatchAdapter::batch_encode(&data_chunks, &config);
        assert_eq!(vectors.len(), 2);

        // Batch decode
        let decoded = BatchAdapter::batch_decode(&vectors, &config, 5);
        assert_eq!(decoded.len(), 2);

        // Batch similarity
        let query = SparseVec::new();
        let similarities = BatchAdapter::batch_similarity(&query, &vectors);
        assert_eq!(similarities.len(), 2);

        // Batch bundle
        let bundled = BatchAdapter::batch_bundle(&vectors);
        assert!(bundled.is_some());
    }

    #[test]
    fn test_stream_adapter() {
        let config = ReversibleVSAConfig::default();
        let data = b"hello world from streaming";
        let cursor = io::Cursor::new(data);

        // Stream encode
        let vectors = StreamAdapter::stream_encode(cursor, &config, 8).unwrap();
        assert!(!vectors.is_empty());

        // Stream decode
        let mut output = Vec::new();
        StreamAdapter::stream_decode(&vectors, &config, 8, &mut output).unwrap();
        // Note: decoded data may differ from original due to chunking
        assert!(!output.is_empty());
    }

    #[test]
    fn test_auto_format_adapter() {
        let dir = tempdir().unwrap();

        // Create a manifest and save as JSON
        let manifest = Manifest {
            files: Vec::new(),
            total_chunks: 0,
            chunk_size: 4096,
            holographic: false,
        };

        let path = dir.path().join("manifest.json");
        FileAdapter::save_manifest(&path, &manifest).unwrap();

        // Auto-load should detect JSON
        let loaded = AutoFormatAdapter::auto_load_manifest(&path).unwrap();
        assert_eq!(manifest.total_chunks, loaded.total_chunks);
    }
}
