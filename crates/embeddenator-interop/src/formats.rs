//! Format conversion utilities for embeddenator types.
//!
//! This module provides conversions between embeddenator native types
//! and various external formats:
//! - JSON (human-readable, cross-language)
//! - Bincode (binary, efficient)
//! - Text representations (debugging, CLI output)
//!
//! All conversions support round-trip guarantees where applicable.

use embeddenator_fs::{Engram, Manifest, SubEngram};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::io;

/// Supported output formats for conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON format (human-readable)
    Json,
    /// JSON format (pretty-printed)
    JsonPretty,
    /// Bincode format (binary, efficient)
    Bincode,
    /// Text format (debugging)
    Text,
}

/// Error type for format conversions
#[derive(Debug, Clone)]
pub enum FormatError {
    /// Serialization failed
    SerializationFailed(String),
    /// Deserialization failed
    DeserializationFailed(String),
    /// Unsupported format combination
    UnsupportedFormat(String),
    /// IO error
    IoError(String),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::SerializationFailed(msg) => write!(f, "serialization failed: {}", msg),
            FormatError::DeserializationFailed(msg) => {
                write!(f, "deserialization failed: {}", msg)
            }
            FormatError::UnsupportedFormat(msg) => write!(f, "unsupported format: {}", msg),
            FormatError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<io::Error> for FormatError {
    fn from(err: io::Error) -> Self {
        FormatError::IoError(err.to_string())
    }
}

// ============================================================================
// SparseVec Conversions
// ============================================================================

/// Convert SparseVec to specified format
pub fn sparse_vec_to_format(vec: &SparseVec, format: OutputFormat) -> Result<Vec<u8>, FormatError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_vec(vec).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::JsonPretty => serde_json::to_vec_pretty(vec)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::Bincode => {
            bincode::serialize(vec).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::Text => Ok(format_sparse_vec_text(vec).into_bytes()),
    }
}

/// Convert from bytes to SparseVec
pub fn sparse_vec_from_format(data: &[u8], format: OutputFormat) -> Result<SparseVec, FormatError> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => serde_json::from_slice(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Bincode => bincode::deserialize(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Text => Err(FormatError::UnsupportedFormat(
            "cannot deserialize from text format".to_string(),
        )),
    }
}

fn format_sparse_vec_text(vec: &SparseVec) -> String {
    format!(
        "SparseVec {{ pos: {:?}, neg: {:?}, nnz: {} }}",
        &vec.pos[..vec.pos.len().min(10)],
        &vec.neg[..vec.neg.len().min(10)],
        vec.pos.len() + vec.neg.len()
    )
}

// ============================================================================
// Engram Conversions
// ============================================================================

/// Convert Engram to specified format
pub fn engram_to_format(engram: &Engram, format: OutputFormat) -> Result<Vec<u8>, FormatError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_vec(engram).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::JsonPretty => serde_json::to_vec_pretty(engram)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::Bincode => {
            bincode::serialize(engram).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::Text => Ok(format_engram_text(engram).into_bytes()),
    }
}

/// Convert from bytes to Engram
pub fn engram_from_format(data: &[u8], format: OutputFormat) -> Result<Engram, FormatError> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => serde_json::from_slice(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Bincode => bincode::deserialize(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Text => Err(FormatError::UnsupportedFormat(
            "cannot deserialize from text format".to_string(),
        )),
    }
}

fn format_engram_text(engram: &Engram) -> String {
    format!(
        "Engram {{ root_nnz: {}, codebook_size: {} }}",
        engram.root.pos.len() + engram.root.neg.len(),
        engram.codebook.len()
    )
}

// ============================================================================
// Manifest Conversions
// ============================================================================

/// Convert Manifest to specified format
pub fn manifest_to_format(
    manifest: &Manifest,
    format: OutputFormat,
) -> Result<Vec<u8>, FormatError> {
    match format {
        OutputFormat::Json => serde_json::to_vec(manifest)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::JsonPretty => serde_json::to_vec_pretty(manifest)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::Bincode => bincode::serialize(manifest)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::Text => Ok(format_manifest_text(manifest).into_bytes()),
    }
}

/// Convert from bytes to Manifest
pub fn manifest_from_format(data: &[u8], format: OutputFormat) -> Result<Manifest, FormatError> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => serde_json::from_slice(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Bincode => bincode::deserialize(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Text => Err(FormatError::UnsupportedFormat(
            "cannot deserialize from text format".to_string(),
        )),
    }
}

fn format_manifest_text(manifest: &Manifest) -> String {
    format!(
        "Manifest {{ files: {}, total_chunks: {} }}",
        manifest.files.len(),
        manifest.total_chunks
    )
}

// ============================================================================
// SubEngram Conversions
// ============================================================================

/// Convert SubEngram to specified format
pub fn sub_engram_to_format(sub: &SubEngram, format: OutputFormat) -> Result<Vec<u8>, FormatError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_vec(sub).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::JsonPretty => serde_json::to_vec_pretty(sub)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::Bincode => {
            bincode::serialize(sub).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::Text => Ok(format_sub_engram_text(sub).into_bytes()),
    }
}

/// Convert from bytes to SubEngram
pub fn sub_engram_from_format(data: &[u8], format: OutputFormat) -> Result<SubEngram, FormatError> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => serde_json::from_slice(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Bincode => bincode::deserialize(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Text => Err(FormatError::UnsupportedFormat(
            "cannot deserialize from text format".to_string(),
        )),
    }
}

fn format_sub_engram_text(sub: &SubEngram) -> String {
    format!(
        "SubEngram {{ chunk_ids: {}, children: {} }}",
        sub.chunk_ids.len(),
        sub.children.len()
    )
}

// ============================================================================
// Config Conversions
// ============================================================================

/// Convert ReversibleVSAConfig to specified format
pub fn vsa_config_to_format(
    config: &ReversibleVSAConfig,
    format: OutputFormat,
) -> Result<Vec<u8>, FormatError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_vec(config).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::JsonPretty => serde_json::to_vec_pretty(config)
            .map_err(|e| FormatError::SerializationFailed(e.to_string())),
        OutputFormat::Bincode => {
            bincode::serialize(config).map_err(|e| FormatError::SerializationFailed(e.to_string()))
        }
        OutputFormat::Text => Ok(format_vsa_config_text(config).into_bytes()),
    }
}

/// Convert from bytes to ReversibleVSAConfig
pub fn vsa_config_from_format(
    data: &[u8],
    format: OutputFormat,
) -> Result<ReversibleVSAConfig, FormatError> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => serde_json::from_slice(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Bincode => bincode::deserialize(data)
            .map_err(|e| FormatError::DeserializationFailed(e.to_string())),
        OutputFormat::Text => Err(FormatError::UnsupportedFormat(
            "cannot deserialize from text format".to_string(),
        )),
    }
}

fn format_vsa_config_text(config: &ReversibleVSAConfig) -> String {
    format!(
        "ReversibleVSAConfig {{ block_size: {}, max_path_depth: {}, base_shift: {}, target_sparsity: {} }}",
        config.block_size, config.max_path_depth, config.base_shift, config.target_sparsity
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_vec_roundtrip_json() {
        let vec = SparseVec {
            pos: vec![1, 5, 9],
            neg: vec![2, 6, 10],
        };

        let bytes = sparse_vec_to_format(&vec, OutputFormat::Json).unwrap();
        let decoded = sparse_vec_from_format(&bytes, OutputFormat::Json).unwrap();

        assert_eq!(vec.pos, decoded.pos);
        assert_eq!(vec.neg, decoded.neg);
    }

    #[test]
    fn test_sparse_vec_roundtrip_bincode() {
        let vec = SparseVec {
            pos: vec![1, 5, 9],
            neg: vec![2, 6, 10],
        };

        let bytes = sparse_vec_to_format(&vec, OutputFormat::Bincode).unwrap();
        let decoded = sparse_vec_from_format(&bytes, OutputFormat::Bincode).unwrap();

        assert_eq!(vec.pos, decoded.pos);
        assert_eq!(vec.neg, decoded.neg);
    }

    #[test]
    fn test_sparse_vec_text_format() {
        let vec = SparseVec {
            pos: vec![1, 5, 9],
            neg: vec![2, 6, 10],
        };

        let bytes = sparse_vec_to_format(&vec, OutputFormat::Text).unwrap();
        let text = String::from_utf8(bytes).unwrap();

        assert!(text.contains("SparseVec"));
        assert!(text.contains("nnz: 6"));
    }

    #[test]
    fn test_vsa_config_roundtrip() {
        let config = ReversibleVSAConfig {
            block_size: 256,
            max_path_depth: 10,
            base_shift: 1000,
            target_sparsity: 200,
        };

        // JSON round-trip
        let bytes = vsa_config_to_format(&config, OutputFormat::Json).unwrap();
        let decoded = vsa_config_from_format(&bytes, OutputFormat::Json).unwrap();
        assert_eq!(config.block_size, decoded.block_size);
        assert_eq!(config.max_path_depth, decoded.max_path_depth);

        // Bincode round-trip
        let bytes = vsa_config_to_format(&config, OutputFormat::Bincode).unwrap();
        let decoded = vsa_config_from_format(&bytes, OutputFormat::Bincode).unwrap();
        assert_eq!(config.block_size, decoded.block_size);
        assert_eq!(config.max_path_depth, decoded.max_path_depth);
    }

    #[test]
    fn test_text_format_no_deserialize() {
        let vec = SparseVec {
            pos: vec![1, 5, 9],
            neg: vec![2, 6, 10],
        };

        let bytes = sparse_vec_to_format(&vec, OutputFormat::Text).unwrap();
        let result = sparse_vec_from_format(&bytes, OutputFormat::Text);

        assert!(result.is_err());
        assert!(matches!(result, Err(FormatError::UnsupportedFormat(_))));
    }
}
