//! # embeddenator-interop
//!
//! Interoperability layer for Embeddenator: format conversions, FFI, and language bindings.
//!
//! Extracted from embeddenator core as part of Phase 2A component decomposition.
//!
//! ## Features
//!
//! - **Format Conversions**: Convert between JSON, bincode, and text formats
//! - **FFI Bindings**: C-compatible interface for cross-language integration
//! - **Python Bindings**: PyO3-based Python API (optional, enable `python` feature)
//! - **Adapters**: High-level adapters for common integration patterns
//! - **Kernel Interop**: Backend-agnostic VSA operations for runtime integration
//!
//! ## Modules
//!
//! - [`formats`]: Format conversion utilities
//! - [`ffi`]: C FFI bindings (unsafe but documented)
//! - [`bindings`]: Python bindings (requires `python` feature)
//! - [`adapters`]: High-level adapter layers
//! - [`kernel_interop`]: Kernel/runtime interop abstractions
//!
//! ## Examples
//!
//! ### Format Conversion
//! ```
//! use embeddenator_interop::formats::{sparse_vec_to_format, OutputFormat};
//! use embeddenator_vsa::SparseVec;
//!
//! let vec = SparseVec::new();
//! let json_bytes = sparse_vec_to_format(&vec, OutputFormat::Json).unwrap();
//! let text_bytes = sparse_vec_to_format(&vec, OutputFormat::Text).unwrap();
//! ```
//!
//! ### File Operations
//! ```no_run
//! use embeddenator_interop::adapters::FileAdapter;
//! use embeddenator_vsa::SparseVec;
//! use embeddenator_io::CompressionCodec;
//!
//! let vec = SparseVec::new();
//! FileAdapter::save_sparse_vec("vector.bin", &vec).unwrap();
//! let loaded = FileAdapter::load_sparse_vec("vector.bin").unwrap();
//! ```

pub mod adapters;
pub mod ffi;
pub mod formats;
pub mod kernel_interop;

#[cfg(feature = "python")]
pub mod bindings;

// Re-export kernel interop types
pub use kernel_interop::*;

// Re-export format types
pub use formats::{FormatError, OutputFormat};

// Re-export adapter types
pub use adapters::{AutoFormatAdapter, BatchAdapter, EnvelopeAdapter, FileAdapter, StreamAdapter};

// Convenience functions for JSON export/import
pub fn export_to_json(vec: &embeddenator_vsa::SparseVec) -> Result<String, FormatError> {
    let bytes = formats::sparse_vec_to_format(vec, OutputFormat::Json)?;
    String::from_utf8(bytes).map_err(|e| FormatError::SerializationFailed(e.to_string()))
}

pub fn import_from_json(json: &str) -> Result<embeddenator_vsa::SparseVec, FormatError> {
    formats::sparse_vec_from_format(json.as_bytes(), OutputFormat::Json)
}

/// Handle-based interface for C FFI compatibility
pub struct VSAHandle {
    vec: embeddenator_vsa::SparseVec,
}

impl VSAHandle {
    pub fn from_sparse_vec(vec: embeddenator_vsa::SparseVec) -> Self {
        Self { vec }
    }

    pub fn to_sparse_vec(&self) -> embeddenator_vsa::SparseVec {
        self.vec.clone()
    }

    pub fn dimensions(&self) -> (usize, usize) {
        let nnz = self.vec.pos.len() + self.vec.neg.len();
        (embeddenator_vsa::DIM, nnz)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use adapters::BatchAdapter;
    use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
    use formats::{sparse_vec_from_format, sparse_vec_to_format};

    #[test]
    fn test_format_roundtrip() {
        let vec = SparseVec {
            pos: vec![1, 2, 3],
            neg: vec![4, 5],
        };

        // JSON round-trip
        let json = sparse_vec_to_format(&vec, OutputFormat::Json).unwrap();
        let from_json = sparse_vec_from_format(&json, OutputFormat::Json).unwrap();
        assert_eq!(vec.pos, from_json.pos);

        // Bincode round-trip
        let bincode = sparse_vec_to_format(&vec, OutputFormat::Bincode).unwrap();
        let from_bincode = sparse_vec_from_format(&bincode, OutputFormat::Bincode).unwrap();
        assert_eq!(vec.pos, from_bincode.pos);
    }

    #[test]
    fn test_batch_operations() {
        let config = ReversibleVSAConfig::default();
        let data = vec![b"test1".as_slice(), b"test2".as_slice()];

        let vectors = BatchAdapter::batch_encode(&data, &config);
        assert_eq!(vectors.len(), 2);

        let decoded = BatchAdapter::batch_decode(&vectors, &config, 5);
        assert_eq!(decoded.len(), 2);
    }

    #[test]
    fn test_kernel_backend() {
        let backend = SparseVecBackend;
        let config = ReversibleVSAConfig::default();

        let data = b"test data";
        let vec = backend.encode_data(data, &config, None);
        let decoded = backend.decode_data(&vec, &config, None, data.len());

        // VSA encoding is lossy, so we just verify the decoded data has the correct length
        assert_eq!(data.len(), decoded.len());

        // Verify operations work
        let vec2 = backend.encode_data(b"other", &config, None);
        let _bundled = backend.bundle(&vec, &vec2);
        let _bound = backend.bind(&vec, &vec2);
        let similarity = backend.cosine(&vec, &vec2);

        assert!((0.0..=1.0).contains(&similarity));
    }
}
