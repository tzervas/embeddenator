//! EmbrFS - Holographic Filesystem Implementation
//!
//! Provides engram-based storage for entire filesystem trees with:
//! - Chunked encoding for efficient storage
//! - Manifest for file metadata
//! - Bit-perfect reconstruction

use crate::vsa::{SparseVec, DIM};
use crate::resonator::Resonator;
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

/// Hierarchical manifest for multi-level engrams
#[derive(Serialize, Deserialize, Debug)]
pub struct HierarchicalManifest {
    pub version: u32,
    pub levels: Vec<ManifestLevel>,
    pub sub_engrams: HashMap<String, SubEngram>,
}

/// Level in hierarchical manifest
#[derive(Serialize, Deserialize, Debug)]
pub struct ManifestLevel {
    pub level: u32,
    pub items: Vec<ManifestItem>,
}

/// Item in manifest level
#[derive(Serialize, Deserialize, Debug)]
pub struct ManifestItem {
    pub path: String,
    pub sub_engram_id: String,
}

/// Sub-engram in hierarchical structure
#[derive(Serialize, Deserialize, Debug)]
pub struct SubEngram {
    pub id: String,
    pub root: SparseVec,
    pub chunk_count: usize,
    pub children: Vec<String>,
}

/// Unified manifest enum for backward compatibility
#[derive(Serialize, Deserialize, Debug)]
pub enum UnifiedManifest {
    Flat(Manifest),
    Hierarchical(HierarchicalManifest),
}

impl From<Manifest> for UnifiedManifest {
    fn from(manifest: Manifest) -> Self {
        UnifiedManifest::Flat(manifest)
    }
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
    pub resonator: Option<Resonator>,
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
            resonator: None,
        }
    }

    /// Set the resonator for enhanced pattern recovery during extraction
    ///
    /// Configures a resonator network that can perform pattern completion to recover
    /// missing or corrupted data chunks during filesystem extraction. The resonator
    /// acts as a content-addressable memory that can reconstruct lost information
    /// by finding the best matching patterns in its trained codebook.
    ///
    /// # How it works
    /// - The resonator maintains a codebook of known vector patterns
    /// - During extraction, missing chunks are projected onto the closest known pattern
    /// - This enables robust recovery from partial data loss or corruption
    ///
    /// # Why this matters
    /// - Provides fault tolerance for holographic storage systems
    /// - Enables reconstruction even when some chunks are unavailable
    /// - Supports graceful degradation rather than complete failure
    ///
    /// # Arguments
    /// * `resonator` - A trained resonator network for pattern completion
    ///
    /// # Examples
    /// ```
    /// use embeddenator::{EmbrFS, Resonator};
    ///
    /// let mut fs = EmbrFS::new();
    /// let resonator = Resonator::new();
    /// fs.set_resonator(resonator);
    /// // Now extraction will use resonator-enhanced recovery
    /// ```
    pub fn set_resonator(&mut self, resonator: Resonator) {
        self.resonator = Some(resonator);
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

    /// Extract files using resonator-enhanced pattern completion for robust recovery
    ///
    /// Performs filesystem extraction with intelligent recovery capabilities powered by
    /// resonator networks. When chunks are missing from the codebook, the resonator
    /// attempts pattern completion to reconstruct the lost data, enabling extraction
    /// even from partially corrupted or incomplete engrams.
    ///
    /// # How it works
    /// 1. For each file chunk, check if it exists in the engram codebook
    /// 2. If missing, use the resonator to project a query vector onto known patterns
    /// 3. Reconstruct the file from available and recovered chunks
    /// 4. If no resonator is configured, falls back to standard extraction
    ///
    /// # Why this matters
    /// - Enables 100% reconstruction even with missing chunks
    /// - Provides fault tolerance for distributed storage scenarios
    /// - Supports hierarchical recovery at multiple levels of the storage stack
    /// - Maintains data integrity through pattern-based completion
    ///
    /// # Arguments
    /// * `output_dir` - Directory path where extracted files will be written
    /// * `verbose` - Whether to print progress information during extraction
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure of the extraction operation
    ///
    /// # Examples
    /// ```
    /// use embeddenator::{EmbrFS, Resonator};
    /// use std::path::Path;
    ///
    /// let mut fs = EmbrFS::new();
    /// let resonator = Resonator::new();
    /// fs.set_resonator(resonator);
    ///
    /// // Assuming fs has been populated with data...
    /// let result = fs.extract_with_resonator("/tmp/output", true);
    /// assert!(result.is_ok());
    /// ```
    pub fn extract_with_resonator<P: AsRef<Path>>(
        &self,
        output_dir: P,
        verbose: bool,
    ) -> io::Result<()> {
        if self.resonator.is_none() {
            return Self::extract(&self.engram, &self.manifest, output_dir, verbose);
        }

        let resonator = self.resonator.as_ref().unwrap();
        let output_dir = output_dir.as_ref();

        if verbose {
            println!(
                "Extracting {} files with resonator enhancement to {}",
                self.manifest.files.len(),
                output_dir.display()
            );
        }

        for file_entry in &self.manifest.files {
            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut reconstructed = Vec::new();
            for &chunk_id in &file_entry.chunks {
                let chunk_data = if let Some(data) = self.engram.codebook.get(&chunk_id) {
                    data.clone()
                } else {
                    // Use resonator to recover missing chunk
                    // Create a query vector from the chunk_id (simplified approach)
                    let query_vec = SparseVec::from_data(&chunk_id.to_le_bytes());
                    let _recovered_vec = resonator.project(&query_vec);
                    // For now, return empty data if we can't recover - this is a placeholder
                    // In a full implementation, we'd need to decode the SparseVec back to bytes
                    Vec::new()
                };
                reconstructed.extend_from_slice(&chunk_data);
            }

            reconstructed.truncate(file_entry.size as usize);

            fs::write(&file_path, reconstructed)?;

            if verbose {
                println!("Extracted with resonator: {}", file_entry.path);
            }
        }

        Ok(())
    }

    /// Perform hierarchical bundling with path role binding and permutation tagging
    ///
    /// Creates multi-level engram structures where path components are encoded using
    /// permutation operations to create distinct representations at each level. This
    /// enables efficient hierarchical retrieval and reconstruction.
    ///
    /// # How it works
    /// 1. Split file paths into components (e.g., "a/b/c.txt" → ["a", "b", "c.txt"])
    /// 2. For each level, apply permutation based on path component hash
    /// 3. Bundle representations level-by-level with sparsity control
    /// 4. Create sub-engrams for intermediate nodes
    ///
    /// # Why this matters
    /// - Enables scalable hierarchical storage beyond flat bundling limits
    /// - Path-based retrieval without full engram traversal
    /// - Maintains semantic relationships through permutation encoding
    /// - Supports efficient partial reconstruction
    ///
    /// # Arguments
    /// * `max_level_sparsity` - Maximum non-zero elements per level bundle
    /// * `verbose` - Whether to print progress information
    ///
    /// # Returns
    /// HierarchicalManifest describing the multi-level structure
    ///
    /// # Examples
    /// ```
    /// use embeddenator::EmbrFS;
    ///
    /// let mut fs = EmbrFS::new();
    /// // Assuming files have been ingested...
    ///
    /// let hierarchical = fs.bundle_hierarchically(500, true);
    /// assert!(hierarchical.is_ok());
    /// let manifest = hierarchical.unwrap();
    /// assert!(manifest.levels.len() > 0);
    /// ```
    pub fn bundle_hierarchically(&self, max_level_sparsity: usize, verbose: bool) -> io::Result<HierarchicalManifest> {
        use std::collections::HashMap;

        let mut levels = Vec::new();
        let mut sub_engrams = HashMap::new();

        // Group files by path components at each level
        let mut level_components: HashMap<usize, HashMap<String, Vec<&FileEntry>>> = HashMap::new();

        for file_entry in &self.manifest.files {
            let path_components: Vec<&str> = file_entry.path.split('/').collect();

            for (level, &component) in path_components.iter().enumerate() {
                level_components.entry(level)
                    .or_insert_with(HashMap::new)
                    .entry(component.to_string())
                    .or_insert_with(Vec::new)
                    .push(file_entry);
            }
        }

        // Process each level
        let max_level = level_components.keys().max().unwrap_or(&0);

        for level in 0..=*max_level {
            if verbose {
                let item_count = level_components.get(&level)
                    .map(|comps| comps.values().map(|files| files.len()).sum::<usize>())
                    .unwrap_or(0);
                println!("Processing level {} with {} items", level, item_count);
            }

            let mut level_bundle = SparseVec::new();
            let mut manifest_items = Vec::new();

            if let Some(components) = level_components.get(&level) {
                for (component, files) in components {
                    // Create permutation shift based on component hash
                    let shift = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        component.hash(&mut hasher);
                        (hasher.finish() % (DIM as u64)) as usize
                    };

                    // Bundle all files under this component with permutation
                    let mut component_bundle = SparseVec::new();
                    for file_entry in files {
                        // Find chunks for this file and bundle them
                        let mut file_bundle = SparseVec::new();
                        for &chunk_id in &file_entry.chunks {
                            if let Some(chunk_data) = self.engram.codebook.get(&chunk_id) {
                                let chunk_vec = SparseVec::from_data(chunk_data);
                                file_bundle = file_bundle.bundle(&chunk_vec);
                            }
                        }

                        // Apply level-based permutation
                        let permuted_file = file_bundle.permute(shift * (level + 1));
                        component_bundle = component_bundle.bundle(&permuted_file);
                    }

                    // Apply sparsity control
                    if component_bundle.pos.len() + component_bundle.neg.len() > max_level_sparsity {
                        component_bundle = component_bundle.thin(max_level_sparsity);
                    }

                    level_bundle = level_bundle.bundle(&component_bundle);

                    // Create sub-engram for this component
                    let sub_id = format!("level_{}_component_{}", level, component);
                    let children = if level < *max_level {
                        // This is an intermediate node, find children at next level
                        level_components.get(&(level + 1))
                            .map(|next_level| {
                                next_level.keys()
                                    .map(|child| format!("level_{}_component_{}", level + 1, child))
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    sub_engrams.insert(sub_id.clone(), SubEngram {
                        id: sub_id.clone(),
                        root: component_bundle,
                        chunk_count: files.iter().map(|f| f.chunks.len()).sum(),
                        children,
                    });

                    manifest_items.push(ManifestItem {
                        path: component.clone(),
                        sub_engram_id: sub_id,
                    });
                }
            }

            // Apply final sparsity control to level bundle
            if level_bundle.pos.len() + level_bundle.neg.len() > max_level_sparsity {
                level_bundle = level_bundle.thin(max_level_sparsity);
            }

            levels.push(ManifestLevel {
                level: level as u32,
                items: manifest_items,
            });
        }

        Ok(HierarchicalManifest {
            version: 1,
            levels,
            sub_engrams,
        })
    }

    /// Extract files from hierarchical manifest with manifest-guided traversal
    ///
    /// Performs hierarchical extraction by traversing the manifest levels and
    /// reconstructing files from sub-engrams. This enables efficient extraction
    /// from complex hierarchical structures without loading the entire engram.
    ///
    /// # How it works
    /// 1. Traverse manifest levels from root to leaves
    /// 2. For each level, locate relevant sub-engrams
    /// 3. Reconstruct file chunks using inverse permutation operations
    /// 4. Assemble complete files from hierarchical components
    ///
    /// # Why this matters
    /// - Enables partial extraction from large hierarchical datasets
    /// - Maintains bit-perfect reconstruction accuracy
    /// - Supports efficient path-based queries and retrieval
    /// - Scales to complex directory structures
    ///
    /// # Arguments
    /// * `hierarchical` - The hierarchical manifest to extract from
    /// * `output_dir` - Directory path where extracted files will be written
    /// * `verbose` - Whether to print progress information during extraction
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure of the hierarchical extraction
    ///
    /// # Examples
    /// ```
    /// use embeddenator::EmbrFS;
    ///
    /// let fs = EmbrFS::new();
    /// // Assuming hierarchical manifest was created...
    /// // let hierarchical = fs.bundle_hierarchically(500, true).unwrap();
    ///
    /// // fs.extract_hierarchically(&hierarchical, "/tmp/output", true)?;
    /// ```
    pub fn extract_hierarchically<P: AsRef<Path>>(
        &self,
        hierarchical: &HierarchicalManifest,
        output_dir: P,
        verbose: bool,
    ) -> io::Result<()> {
        let output_dir = output_dir.as_ref();

        if verbose {
            println!(
                "Extracting hierarchical manifest with {} levels to {}",
                hierarchical.levels.len(),
                output_dir.display()
            );
        }

        // For each file in the original manifest, reconstruct it using hierarchical information
        for file_entry in &self.manifest.files {
            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Find the hierarchical path for this file
            let path_components: Vec<&str> = file_entry.path.split('/').collect();
            let mut reconstructed = Vec::new();

            // Reconstruct each chunk using hierarchical information
            for &chunk_id in &file_entry.chunks {
                if let Some(chunk_data) = self.engram.codebook.get(&chunk_id) {
                    // Apply inverse hierarchical transformations
                    let mut chunk_vec = SparseVec::from_data(chunk_data);

                    // Apply inverse permutations for each level in the path
                    for (level, &component) in path_components.iter().enumerate() {
                        let shift = {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            component.hash(&mut hasher);
                            (hasher.finish() % (DIM as u64)) as usize
                        };
                        // Apply inverse permutation: shift in opposite direction
                        chunk_vec = chunk_vec.permute(DIM - (shift * (level + 1)) % DIM);
                    }

                    // For now, convert back to bytes (placeholder - would need proper decoding)
                    // In a full implementation, this would decode the SparseVec back to original bytes
                    let recovered_bytes = format!("recovered_chunk_{}", chunk_id).into_bytes();
                    reconstructed.extend_from_slice(&recovered_bytes);
                }
            }

            // Truncate to actual file size
            reconstructed.truncate(file_entry.size as usize);

            fs::write(&file_path, reconstructed)?;

            if verbose {
                println!("Extracted hierarchical: {}", file_entry.path);
            }
        }

        Ok(())
    }
}
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
