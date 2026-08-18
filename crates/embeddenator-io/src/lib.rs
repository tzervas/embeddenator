//! # embeddenator-io
//!
//! I/O utilities and serialization for Embeddenator.
//!
//! Extracted from embeddenator core as part of Phase 2A component decomposition.
//!
//! ## Features
//!
//! - **Serialization**: Bincode and JSON support for efficient data encoding
//! - **Buffering**: Optimized buffered I/O with configurable buffer sizes
//! - **Streaming**: Memory-efficient streaming I/O for large files
//! - **Envelope Format**: Compressed binary envelope format with multiple codecs
//! - **Async Support**: Optional async I/O with tokio (enable `async` feature)
//!
//! ## Examples
//!
//! ### Serialization
//! ```
//! use embeddenator_io::{to_bincode, from_bincode};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct Data { value: u32 }
//!
//! let data = Data { value: 42 };
//! let bytes = to_bincode(&data).unwrap();
//! let decoded: Data = from_bincode(&bytes).unwrap();
//! assert_eq!(data, decoded);
//! ```
//!
//! ### Streaming
//! ```no_run
//! use embeddenator_io::stream_read_file;
//!
//! let mut total = 0;
//! stream_read_file("large_file.bin", |chunk| {
//!     total += chunk.len();
//!     Ok(())
//! }).unwrap();
//! ```
//!
//! ### Envelope Format
//! ```
//! use embeddenator_io::{PayloadKind, BinaryWriteOptions, wrap_or_legacy, unwrap_auto};
//!
//! let data = b"Hello, world!";
//! let opts = BinaryWriteOptions::default();
//! let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, data).unwrap();
//! let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();
//! assert_eq!(data, unwrapped.as_slice());
//! ```

pub mod io;
pub use io::*;

// Re-export commonly used types
pub use buffer::{
    buffered_reader, buffered_writer, copy_buffered, read_chunks, write_chunks, ChunkStream,
    DEFAULT_BUFFER_SIZE, LARGE_BUFFER_SIZE, SMALL_BUFFER_SIZE,
};
pub use serialize::{
    from_bincode, from_json, read_bincode_file, read_json_file, to_bincode, to_json,
    to_json_pretty, write_bincode_file, write_json_file,
};
pub use stream::{stream_read_file, stream_write_file, StreamReader, StreamWriter};
pub use stream_compress::{
    compress_file, decompress_file, stream_compress, stream_decompress, CompressionLevel,
    StreamCompressor, StreamDecompressor,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_loads() {
        // Verify core types are accessible
        let _ = PayloadKind::EngramBincode;
        let _ = CompressionCodec::None;
        let opts = BinaryWriteOptions::default();
        assert_eq!(opts.codec, CompressionCodec::None);
    }

    #[test]
    fn test_serialization_integration() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            id: u32,
            name: String,
        }

        let data = TestData {
            id: 123,
            name: "test".to_string(),
        };

        // Test bincode
        let bytes = to_bincode(&data).unwrap();
        let decoded: TestData = from_bincode(&bytes).unwrap();
        assert_eq!(data, decoded);

        // Test JSON
        let json = to_json(&data).unwrap();
        let decoded: TestData = from_json(&json).unwrap();
        assert_eq!(data.id, decoded.id);
    }

    #[test]
    fn test_envelope_integration() {
        let data = b"Test data for envelope";
        let opts = BinaryWriteOptions::default();

        // Wrap and unwrap
        let wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, data).unwrap();
        let unwrapped = unwrap_auto(PayloadKind::EngramBincode, &wrapped).unwrap();

        assert_eq!(data, unwrapped.as_slice());
    }
}
