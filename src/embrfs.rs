//! EmbrFS - Holographic Filesystem Implementation
//!
//! Provides engram-based storage for entire filesystem trees with:
//! - Chunked encoding for efficient storage
//! - Manifest for file metadata
//! - Bit-perfect reconstruction

use crate::vsa::SparseVec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use walkdir::WalkDir;

/// Default chunk size for file encoding (4KB)
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

/// File entry in the manifest
#[derive(Serialize, Deserialize, Debug)]
pub struct FileEntry {
    pub path: String,
    pub is_text: bool,
    pub size: usize,
    pub chunks: Vec<usize>,
}

/// Manifest describing filesystem structure
#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub total_chunks: usize,
}

/// Engram: holographic encoding of a filesystem
#[derive(Serialize, Deserialize)]
pub struct Engram {
    pub root: SparseVec,
    pub codebook: HashMap<usize, Vec<u8>>,
}

/// EmbrFS - Holographic Filesystem
///
/// # Examples
///
/// ```
/// use embeddenator::EmbrFS;
/// use std::path::Path;
///
/// let mut fs = EmbrFS::new();
/// // Ingest and extract would require actual files, so we just test creation
/// assert_eq!(fs.manifest.total_chunks, 0);
/// assert_eq!(fs.manifest.files.len(), 0);
/// ```
pub struct EmbrFS {
    pub manifest: Manifest,
    pub engram: Engram,
}

impl Default for EmbrFS {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbrFS {
    /// Create a new empty EmbrFS instance
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::EmbrFS;
    ///
    /// let fs = EmbrFS::new();
    /// assert_eq!(fs.manifest.files.len(), 0);
    /// assert_eq!(fs.manifest.total_chunks, 0);
    /// ```
    pub fn new() -> Self {
        EmbrFS {
            manifest: Manifest {
                files: Vec::new(),
                total_chunks: 0,
            },
            engram: Engram {
                root: SparseVec::new(),
                codebook: HashMap::new(),
            },
        }
    }

    /// Ingest an entire directory into engram format
    pub fn ingest_directory<P: AsRef<Path>>(&mut self, dir: P, verbose: bool) -> io::Result<()> {
        let dir = dir.as_ref();
        if verbose {
            println!("Ingesting directory: {}", dir.display());
        }

        let mut files_to_process = Vec::new();

        for entry in WalkDir::new(dir).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                files_to_process.push(entry.path().to_path_buf());
            }
        }

        files_to_process.sort();

        for file_path in files_to_process {
            let relative = file_path.strip_prefix(dir).unwrap_or(file_path.as_path());
            self.ingest_file(&file_path, relative.to_string_lossy().to_string(), verbose)?;
        }

        Ok(())
    }

    /// Ingest a single file into the engram
    pub fn ingest_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        logical_path: String,
        verbose: bool,
    ) -> io::Result<()> {
        let file_path = file_path.as_ref();
        let mut file = File::open(file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let is_text = is_text_file(&data);

        if verbose {
            println!(
                "Ingesting {}: {} bytes ({})",
                logical_path,
                data.len(),
                if is_text { "text" } else { "binary" }
            );
        }

        let chunk_size = DEFAULT_CHUNK_SIZE;
        let mut chunks = Vec::new();

        for (i, chunk) in data.chunks(chunk_size).enumerate() {
            let chunk_id = self.manifest.total_chunks + i;
            let chunk_vec = SparseVec::from_data(chunk);

            self.engram.root = self.engram.root.bundle(&chunk_vec);
            self.engram.codebook.insert(chunk_id, chunk.to_vec());
            chunks.push(chunk_id);
        }

        self.manifest.files.push(FileEntry {
            path: logical_path,
            is_text,
            size: data.len(),
            chunks: chunks.clone(),
        });

        self.manifest.total_chunks += chunks.len();

        Ok(())
    }

    /// Save engram to file
    pub fn save_engram<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let encoded = bincode::serialize(&self.engram).map_err(io::Error::other)?;
        fs::write(path, encoded)?;
        Ok(())
    }

    /// Load engram from file
    pub fn load_engram<P: AsRef<Path>>(path: P) -> io::Result<Engram> {
        let data = fs::read(path)?;
        bincode::deserialize(&data).map_err(io::Error::other)
    }

    /// Save manifest to JSON file
    pub fn save_manifest<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &self.manifest)?;
        Ok(())
    }

    /// Load manifest from JSON file
    pub fn load_manifest<P: AsRef<Path>>(path: P) -> io::Result<Manifest> {
        let file = File::open(path)?;
        let manifest = serde_json::from_reader(file)?;
        Ok(manifest)
    }

    /// Extract files from engram to directory
    pub fn extract<P: AsRef<Path>>(
        engram: &Engram,
        manifest: &Manifest,
        output_dir: P,
        verbose: bool,
    ) -> io::Result<()> {
        let output_dir = output_dir.as_ref();

        if verbose {
            println!(
                "Extracting {} files to {}",
                manifest.files.len(),
                output_dir.display()
            );
        }

        for file_entry in &manifest.files {
            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut reconstructed = Vec::new();
            for &chunk_id in &file_entry.chunks {
                if let Some(chunk_data) = engram.codebook.get(&chunk_id) {
                    reconstructed.extend_from_slice(chunk_data);
                }
            }

            reconstructed.truncate(file_entry.size);

            fs::write(&file_path, reconstructed)?;

            if verbose {
                println!("Extracted: {}", file_entry.path);
            }
        }

        Ok(())
    }
}

/// Detect if data is text or binary
pub fn is_text_file(data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }

    let sample_size = data.len().min(8192);
    let sample = &data[..sample_size];

    let mut null_count = 0;
    let mut control_count = 0;

    for &byte in sample {
        if byte == 0 {
            null_count += 1;
        } else if byte < 32 && byte != b'\n' && byte != b'\r' && byte != b'\t' {
            control_count += 1;
        }
    }

    null_count == 0 && control_count < sample_size / 10
}
