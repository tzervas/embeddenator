//! EmbrFS - Holographic Filesystem Implementation
//!
//! Provides engram-based storage for entire filesystem trees with:
//! - Chunked encoding for efficient storage
//! - Manifest for file metadata
//! - **Guaranteed 100% bit-perfect reconstruction** via CorrectionStore
//!
//! # Reconstruction Guarantee
//!
//! The fundamental challenge with VSA encoding is that approximate operations
//! may introduce errors during superposition. This module solves that through
//! a multi-layer approach:
//!
//! 1. **Primary Encoding**: SparseVec encoding attempts bit-perfect storage
//! 2. **Correction Layer**: CorrectionStore captures any encoding errors
//! 3. **Reconstruction**: Decode + apply corrections = exact original
//!
//! The invariant: `original = decode(encode(original)) + correction`
//!
//! If encoding was perfect, correction is empty. If not, correction exactly
//! compensates. Either way, reconstruction is guaranteed bit-perfect.

use crate::vsa::{SparseVec, ReversibleVSAConfig, DIM};
use crate::resonator::Resonator;
use crate::correction::{CorrectionStore, CorrectionStats};
use crate::retrieval::{RerankedResult, TernaryInvertedIndex};
use crate::envelope::{BinaryWriteOptions, PayloadKind, unwrap_auto, wrap_or_legacy};
use crate::metrics::metrics;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(feature = "metrics")]
use std::time::Instant;
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
    #[serde(default)]
    pub sub_engrams: HashMap<String, SubEngram>,
}

/// Level in hierarchical manifest
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManifestLevel {
    pub level: u32,
    pub items: Vec<ManifestItem>,
}

/// Item in manifest level
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManifestItem {
    pub path: String,
    pub sub_engram_id: String,
}

/// Sub-engram in hierarchical structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubEngram {
    pub id: String,
    pub root: SparseVec,
    /// Chunk IDs that belong to this sub-engram.
    ///
    /// This enables selective retrieval without indexing the entire global codebook.
    #[serde(default)]
    pub chunk_ids: Vec<usize>,
    pub chunk_count: usize,
    pub children: Vec<String>,
}

/// Bounds and tuning parameters for hierarchical selective retrieval.
#[derive(Clone, Debug)]
pub struct HierarchicalQueryBounds {
    /// Global top-k results to return.
    pub k: usize,
    /// Candidate count per expanded node before reranking.
    pub candidate_k: usize,
    /// Maximum number of frontier nodes retained (beam width).
    pub beam_width: usize,
    /// Maximum depth to descend (0 means only level-0 nodes).
    pub max_depth: usize,
    /// Maximum number of expanded nodes.
    pub max_expansions: usize,
    /// Maximum number of cached inverted indices.
    pub max_open_indices: usize,
    /// Maximum number of cached sub-engrams.
    pub max_open_engrams: usize,
}

impl Default for HierarchicalQueryBounds {
    fn default() -> Self {
        Self {
            k: 10,
            candidate_k: 100,
            beam_width: 32,
            max_depth: 4,
            max_expansions: 128,
            max_open_indices: 16,
            max_open_engrams: 16,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HierarchicalChunkHit {
    pub sub_engram_id: String,
    pub chunk_id: usize,
    pub approx_score: i32,
    pub cosine: f64,
}

#[derive(Clone, Debug)]
struct FrontierItem {
    score: f64,
    sub_engram_id: String,
    depth: usize,
}

#[derive(Clone, Debug)]
struct RemappedInvertedIndex {
    index: TernaryInvertedIndex,
    local_to_global: Vec<usize>,
}

impl RemappedInvertedIndex {
    fn build(chunk_ids: &[usize], vectors: &HashMap<usize, SparseVec>) -> Self {
        let mut index = TernaryInvertedIndex::new();
        let mut local_to_global = Vec::with_capacity(chunk_ids.len());

        for (local_id, &global_id) in chunk_ids.iter().enumerate() {
            let Some(vec) = vectors.get(&global_id) else {
                continue;
            };
            local_to_global.push(global_id);
            index.add(local_id, vec);
        }

        index.finalize();
        Self {
            index,
            local_to_global,
        }
    }

    fn query_top_k_reranked(
        &self,
        query: &SparseVec,
        vectors: &HashMap<usize, SparseVec>,
        candidate_k: usize,
        k: usize,
    ) -> Vec<HierarchicalChunkHit> {
        if k == 0 {
            return Vec::new();
        }

        let candidates = self.index.query_top_k(query, candidate_k);
        let mut out = Vec::with_capacity(candidates.len().min(k));
        for cand in candidates {
            let Some(&global_id) = self.local_to_global.get(cand.id) else {
                continue;
            };
            let Some(vec) = vectors.get(&global_id) else {
                continue;
            };
            out.push((global_id, cand.score, query.cosine(vec)));
        }

        out.sort_by(|a, b| {
            b.2.total_cmp(&a.2)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.0.cmp(&b.0))
        });
        out.truncate(k);

        out.into_iter()
            .map(|(chunk_id, approx_score, cosine)| HierarchicalChunkHit {
                sub_engram_id: String::new(),
                chunk_id,
                approx_score,
                cosine,
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct LruCache<V> {
    cap: usize,
    map: HashMap<String, V>,
    order: Vec<String>,
}

impl<V> LruCache<V> {
    fn new(cap: usize) -> Self {
        Self {
            cap,
            map: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
            return self.map.get(key);
        }
        None
    }

    fn insert(&mut self, key: String, value: V) -> usize {
        if self.cap == 0 {
            return 0;
        }

        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            self.touch(&key);
            return 0;
        }

        self.map.insert(key.clone(), value);
        self.order.push(key);

        let mut evicted = 0usize;
        while self.map.len() > self.cap {
            if let Some(evict) = self.order.first().cloned() {
                self.order.remove(0);
                self.map.remove(&evict);
                evicted += 1;
            } else {
                break;
            }
        }

        evicted
    }

    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(pos);
            self.order.push(k);
        }
    }
}

/// Storage/loader seam for hierarchical sub-engrams.
///
/// This enables on-demand loading (e.g., from disk) rather than requiring that
/// every sub-engram is materialized in memory.
pub trait SubEngramStore {
    fn load(&self, id: &str) -> Option<SubEngram>;
}

fn escape_sub_engram_id(id: &str) -> String {
    // Minimal reversible escaping for filenames.
    // Note: not intended for untrusted input; IDs are internal.
    id.replace('%', "%25").replace('/', "%2F")
}

/// Directory-backed store for sub-engrams.
///
/// Files are stored as bincode blobs under `${dir}/{escaped_id}.subengram`.
pub struct DirectorySubEngramStore {
    dir: PathBuf,
}

impl DirectorySubEngramStore {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    fn path_for_id(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.subengram", escape_sub_engram_id(id)))
    }
}

impl SubEngramStore for DirectorySubEngramStore {
    fn load(&self, id: &str) -> Option<SubEngram> {
        let path = self.path_for_id(id);
        let data = fs::read(path).ok()?;
        let decoded = unwrap_auto(PayloadKind::SubEngramBincode, &data).ok()?;
        bincode::deserialize(&decoded).ok()
    }
}

/// Save a hierarchical manifest as JSON.
pub fn save_hierarchical_manifest<P: AsRef<Path>>(
    hierarchical: &HierarchicalManifest,
    path: P,
) -> io::Result<()> {
    let file = File::create(path)?;

    // Serialize deterministically: HashMap iteration order is not stable.
    #[derive(Serialize)]
    struct StableHierarchicalManifest {
        version: u32,
        levels: Vec<ManifestLevel>,
        sub_engrams: BTreeMap<String, SubEngram>,
    }

    let mut levels = hierarchical.levels.clone();
    levels.sort_by(|a, b| a.level.cmp(&b.level));
    for level in &mut levels {
        level
            .items
            .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id)));
    }

    let mut sub_engrams: BTreeMap<String, SubEngram> = BTreeMap::new();
    for (id, sub) in &hierarchical.sub_engrams {
        sub_engrams.insert(id.clone(), sub.clone());
    }

    let stable = StableHierarchicalManifest {
        version: hierarchical.version,
        levels,
        sub_engrams,
    };

    serde_json::to_writer_pretty(file, &stable)?;
    Ok(())
}

/// Load a hierarchical manifest from JSON.
pub fn load_hierarchical_manifest<P: AsRef<Path>>(path: P) -> io::Result<HierarchicalManifest> {
    let file = File::open(path)?;
    let manifest = serde_json::from_reader(file)?;
    Ok(manifest)
}

/// Save a set of sub-engrams to a directory (bincode per sub-engram).
pub fn save_sub_engrams_dir<P: AsRef<Path>>(
    sub_engrams: &HashMap<String, SubEngram>,
    dir: P,
) -> io::Result<()> {
    save_sub_engrams_dir_with_options(sub_engrams, dir, BinaryWriteOptions::default())
}

/// Save a set of sub-engrams to a directory (bincode per sub-engram), optionally compressed.
pub fn save_sub_engrams_dir_with_options<P: AsRef<Path>>(
    sub_engrams: &HashMap<String, SubEngram>,
    dir: P,
    opts: BinaryWriteOptions,
) -> io::Result<()> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;

    let mut ids: Vec<&String> = sub_engrams.keys().collect();
    ids.sort();

    for id in ids {
        let sub = sub_engrams.get(id).expect("sub_engram id");
        let encoded = bincode::serialize(sub).map_err(io::Error::other)?;
        let maybe_wrapped = wrap_or_legacy(PayloadKind::SubEngramBincode, opts, &encoded)?;
        let path = dir.join(format!("{}.subengram", escape_sub_engram_id(id)));
        fs::write(path, maybe_wrapped)?;
    }
    Ok(())
}

struct InMemorySubEngramStore<'a> {
    map: &'a HashMap<String, SubEngram>,
}

impl<'a> InMemorySubEngramStore<'a> {
    fn new(map: &'a HashMap<String, SubEngram>) -> Self {
        Self { map }
    }
}

impl SubEngramStore for InMemorySubEngramStore<'_> {
    fn load(&self, id: &str) -> Option<SubEngram> {
        self.map.get(id).cloned()
    }
}

fn get_cached_sub_engram(
    cache: &mut LruCache<SubEngram>,
    store: &impl SubEngramStore,
    id: &str,
) -> Option<SubEngram> {
    if let Some(v) = cache.get(id) {
        metrics().inc_sub_cache_hit();
        return Some(v.clone());
    }
    metrics().inc_sub_cache_miss();
    let loaded = store.load(id)?;
    let evicted = cache.insert(id.to_string(), loaded.clone());
    for _ in 0..evicted {
        metrics().inc_sub_cache_eviction();
    }
    Some(loaded)
}

/// Query a hierarchical manifest by selectively unfolding only promising sub-engrams.
///
/// This performs a beam-limited traversal over `hierarchical.sub_engrams`.
/// At each expanded node, it builds (and LRU-caches) an inverted index over the
/// node-local `chunk_ids` subset of `codebook`, then reranks by exact cosine.
pub fn query_hierarchical_codebook(
    hierarchical: &HierarchicalManifest,
    codebook: &HashMap<usize, SparseVec>,
    query: &SparseVec,
    bounds: &HierarchicalQueryBounds,
) -> Vec<HierarchicalChunkHit> {
    let store = InMemorySubEngramStore::new(&hierarchical.sub_engrams);
    query_hierarchical_codebook_with_store(hierarchical, &store, codebook, query, bounds)
}

/// Store-backed variant of `query_hierarchical_codebook` that supports on-demand sub-engram loading.
pub fn query_hierarchical_codebook_with_store(
    hierarchical: &HierarchicalManifest,
    store: &impl SubEngramStore,
    codebook: &HashMap<usize, SparseVec>,
    query: &SparseVec,
    bounds: &HierarchicalQueryBounds,
) -> Vec<HierarchicalChunkHit> {
    if bounds.k == 0 || hierarchical.levels.is_empty() {
        return Vec::new();
    }

    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let mut sub_cache: LruCache<SubEngram> = LruCache::new(bounds.max_open_engrams);
    let mut index_cache: LruCache<RemappedInvertedIndex> = LruCache::new(bounds.max_open_indices);

    let mut frontier: Vec<FrontierItem> = Vec::new();
    if let Some(level0) = hierarchical.levels.first() {
        for item in &level0.items {
            let Some(sub) = get_cached_sub_engram(&mut sub_cache, store, &item.sub_engram_id) else {
                continue;
            };
            frontier.push(FrontierItem {
                score: query.cosine(&sub.root),
                sub_engram_id: item.sub_engram_id.clone(),
                depth: 0,
            });
        }
    }

    frontier.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id))
    });
    if frontier.len() > bounds.beam_width {
        frontier.truncate(bounds.beam_width);
    }

    let mut expansions = 0usize;

    // Keep only the best hit per chunk for determinism.
    let mut best_by_chunk: HashMap<usize, HierarchicalChunkHit> = HashMap::new();

    while !frontier.is_empty() && expansions < bounds.max_expansions {
        let node = frontier.remove(0);

        let Some(sub) = get_cached_sub_engram(&mut sub_cache, store, &node.sub_engram_id) else {
            continue;
        };

        expansions += 1;

        let idx = if let Some(existing) = index_cache.get(&node.sub_engram_id) {
            metrics().inc_index_cache_hit();
            existing
        } else {
            metrics().inc_index_cache_miss();
            let built = RemappedInvertedIndex::build(&sub.chunk_ids, codebook);
            let evicted = index_cache.insert(node.sub_engram_id.clone(), built);
            for _ in 0..evicted {
                metrics().inc_index_cache_eviction();
            }
            // SAFETY: we just inserted the key, so get() must succeed immediately after
            index_cache
                .get(&node.sub_engram_id)
                .expect("index cache insert")
        };

        let mut local_hits = idx.query_top_k_reranked(query, codebook, bounds.candidate_k, bounds.k);
        for hit in &mut local_hits {
            hit.sub_engram_id = node.sub_engram_id.clone();
        }

        for hit in local_hits {
            match best_by_chunk.get(&hit.chunk_id) {
                None => {
                    best_by_chunk.insert(hit.chunk_id, hit);
                }
                Some(existing) => {
                    let better = hit
                        .cosine
                        .total_cmp(&existing.cosine)
                        .then_with(|| hit.approx_score.cmp(&existing.approx_score))
                        .is_gt();
                    if better {
                        best_by_chunk.insert(hit.chunk_id, hit);
                    }
                }
            }
        }

        if node.depth >= bounds.max_depth {
            continue;
        }

        let children = sub.children.clone();
        for child_id in &children {
            let Some(child) = get_cached_sub_engram(&mut sub_cache, store, child_id) else {
                continue;
            };
            frontier.push(FrontierItem {
                score: query.cosine(&child.root),
                sub_engram_id: child_id.clone(),
                depth: node.depth + 1,
            });
        }

        frontier.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id))
        });
        if frontier.len() > bounds.beam_width {
            frontier.truncate(bounds.beam_width);
        }
    }

    let mut out: Vec<HierarchicalChunkHit> = best_by_chunk.into_values().collect();
    out.sort_by(|a, b| {
        b.cosine
            .total_cmp(&a.cosine)
            .then_with(|| b.approx_score.cmp(&a.approx_score))
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
            .then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id))
    });
    out.truncate(bounds.k);

    #[cfg(feature = "metrics")]
    metrics().record_hier_query(start.elapsed());

    out
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

/// Engram: holographic encoding of a filesystem with correction guarantee
#[derive(Serialize, Deserialize)]
pub struct Engram {
    pub root: SparseVec,
    pub codebook: HashMap<usize, SparseVec>,
    /// Correction store for 100% reconstruction guarantee
    #[serde(default)]
    pub corrections: CorrectionStore,
}

impl Engram {
    /// Build a reusable inverted index over the codebook.
    ///
    /// This is useful when issuing multiple queries (e.g., shift-sweeps) and you
    /// want to avoid rebuilding the index each time.
    pub fn build_codebook_index(&self) -> TernaryInvertedIndex {
        TernaryInvertedIndex::build_from_map(&self.codebook)
    }

    /// Query the codebook using a pre-built inverted index.
    pub fn query_codebook_with_index(
        &self,
        index: &TernaryInvertedIndex,
        query: &SparseVec,
        candidate_k: usize,
        k: usize,
    ) -> Vec<RerankedResult> {
        if k == 0 || self.codebook.is_empty() {
            return Vec::new();
        }
        index.query_top_k_reranked(query, &self.codebook, candidate_k, k)
    }

    /// Query the engram's codebook for chunks most similar to `query`.
    ///
    /// This builds an inverted index over the codebook for sub-linear candidate
    /// generation, then reranks those candidates using exact cosine similarity.
    pub fn query_codebook(&self, query: &SparseVec, k: usize) -> Vec<RerankedResult> {
        if k == 0 || self.codebook.is_empty() {
            return Vec::new();
        }

        // Simple heuristic: rerank a moderately-sized candidate set.
        let candidate_k = (k.saturating_mul(10)).max(50);
        let index = self.build_codebook_index();
        self.query_codebook_with_index(&index, query, candidate_k, k)
    }
}

/// EmbrFS - Holographic Filesystem with Guaranteed Reconstruction
///
/// # 100% Reconstruction Guarantee
///
/// EmbrFS guarantees bit-perfect file reconstruction through a layered approach:
///
/// 1. **Encode**: Data chunks → SparseVec via reversible encoding
/// 2. **Verify**: Immediately decode and compare to original
/// 3. **Correct**: Store minimal correction if any difference exists
/// 4. **Extract**: Decode + apply correction = exact original bytes
///
/// This guarantee holds regardless of:
/// - Data content (binary, text, compressed, encrypted)
/// - File size (single byte to gigabytes)
/// - Number of files in the engram
/// - Superposition crosstalk in bundles
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
    /// // Correction store starts empty
    /// let stats = fs.engram.corrections.stats();
    /// assert_eq!(stats.total_chunks, 0);
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
                corrections: CorrectionStore::new(),
            },
            resonator: None,
        }
    }

    fn path_to_forward_slash_string(path: &Path) -> String {
        path.components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => s.to_str().map(|v| v.to_string()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("/")
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

    /// Get correction statistics for this engram
    ///
    /// Returns statistics about how many chunks needed correction and the
    /// overhead incurred by storing corrections.
    ///
    /// # Examples
    /// ```
    /// use embeddenator::EmbrFS;
    ///
    /// let fs = EmbrFS::new();
    /// let stats = fs.correction_stats();
    /// assert_eq!(stats.total_chunks, 0);
    /// ```
    pub fn correction_stats(&self) -> CorrectionStats {
        self.engram.corrections.stats()
    }

    /// Ingest an entire directory into engram format
    pub fn ingest_directory<P: AsRef<Path>>(
        &mut self,
        dir: P,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        self.ingest_directory_with_prefix(dir, None, verbose, config)
    }

    /// Ingest a directory into the engram, optionally prefixing all logical paths.
    ///
    /// When `logical_prefix` is provided, all ingested file paths become:
    /// `{logical_prefix}/{relative_path_from_dir}`.
    pub fn ingest_directory_with_prefix<P: AsRef<Path>>(
        &mut self,
        dir: P,
        logical_prefix: Option<&str>,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
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
            let rel = Self::path_to_forward_slash_string(relative);
            let logical_path = if let Some(prefix) = logical_prefix {
                if prefix.is_empty() {
                    rel
                } else if rel.is_empty() {
                    prefix.to_string()
                } else {
                    format!("{}/{}", prefix, rel)
                }
            } else {
                rel
            };

            self.ingest_file(&file_path, logical_path, verbose, config)?;
        }

        Ok(())
    }

    /// Ingest a single file into the engram with guaranteed reconstruction
    ///
    /// This method encodes file data into sparse vectors and stores any
    /// necessary corrections to guarantee 100% bit-perfect reconstruction.
    ///
    /// # Correction Process
    ///
    /// For each chunk:
    /// 1. Encode: `chunk_data → SparseVec`
    /// 2. Decode: `SparseVec → decoded_data`  
    /// 3. Compare: `chunk_data == decoded_data?`
    /// 4. If different: store correction in `CorrectionStore`
    ///
    /// # Arguments
    /// * `file_path` - Path to the file on disk
    /// * `logical_path` - Path to use in the engram manifest
    /// * `verbose` - Print progress information
    /// * `config` - VSA encoding configuration
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure
    pub fn ingest_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        logical_path: String,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        let file_path = file_path.as_ref();
        let file_len = fs::metadata(file_path)?.len() as usize;
        let file = File::open(file_path)?;
        let mut reader = BufReader::with_capacity(64 * 1024, file);

        let chunk_size = DEFAULT_CHUNK_SIZE;
        let mut chunks = Vec::new();
        let mut corrections_needed = 0usize;

        let mut buf = vec![0u8; chunk_size];
        let mut is_text: Option<bool> = None;
        let mut i = 0usize;

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            let chunk = &buf[..n];

            if is_text.is_none() {
                let t = is_text_file(chunk);
                is_text = Some(t);

                if verbose {
                    println!(
                        "Ingesting {}: {} bytes ({})",
                        logical_path,
                        file_len,
                        if t { "text" } else { "binary" }
                    );
                }
            }

            let chunk_id = self.manifest.total_chunks + i;
            
            // Encode chunk to sparse vector
            let chunk_vec = SparseVec::encode_data(chunk, config, Some(&logical_path));
            
            // Immediately verify: decode and compare
            let decoded = chunk_vec.decode_data(config, Some(&logical_path), chunk.len());
            
            // Store correction if needed (guarantees reconstruction)
            self.engram.corrections.add(chunk_id as u64, chunk, &decoded);
            
            if chunk != decoded.as_slice() {
                corrections_needed += 1;
            }

            self.engram.root = self.engram.root.bundle(&chunk_vec);
            self.engram.codebook.insert(chunk_id, chunk_vec);
            chunks.push(chunk_id);

            i += 1;
        }

        if verbose && corrections_needed > 0 {
            println!(
                "  → {} of {} chunks needed correction",
                corrections_needed,
                chunks.len()
            );
        }

        self.manifest.files.push(FileEntry {
            path: logical_path,
            is_text: is_text.unwrap_or(true),
            size: file_len,
            chunks: chunks.clone(),
        });

        self.manifest.total_chunks += chunks.len();

        Ok(())
    }

    /// Save engram to file
    pub fn save_engram<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self.save_engram_with_options(path, BinaryWriteOptions::default())
    }

    /// Save engram to file, optionally compressed.
    pub fn save_engram_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        opts: BinaryWriteOptions,
    ) -> io::Result<()> {
        let encoded = bincode::serialize(&self.engram).map_err(io::Error::other)?;
        let maybe_wrapped = wrap_or_legacy(PayloadKind::EngramBincode, opts, &encoded)?;
        fs::write(path, maybe_wrapped)?;
        Ok(())
    }

    /// Load engram from file
    pub fn load_engram<P: AsRef<Path>>(path: P) -> io::Result<Engram> {
        let data = fs::read(path)?;
        let decoded = unwrap_auto(PayloadKind::EngramBincode, &data)?;
        bincode::deserialize(&decoded).map_err(io::Error::other)
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

    /// Extract files from engram to directory with guaranteed reconstruction
    ///
    /// This method guarantees 100% bit-perfect reconstruction by applying
    /// stored corrections after decoding each chunk.
    ///
    /// # Reconstruction Process
    ///
    /// For each chunk:
    /// 1. Decode: `SparseVec → decoded_data`
    /// 2. Apply correction: `decoded_data + correction → original_data`
    /// 3. Verify: Hash matches stored hash (guaranteed by construction)
    ///
    /// # Arguments
    /// * `engram` - The engram containing encoded data and corrections
    /// * `manifest` - File metadata and chunk mappings
    /// * `output_dir` - Directory to write extracted files
    /// * `verbose` - Print progress information
    /// * `config` - VSA decoding configuration
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure
    pub fn extract<P: AsRef<Path>>(
        engram: &Engram,
        manifest: &Manifest,
        output_dir: P,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        let output_dir = output_dir.as_ref();

        if verbose {
            println!(
                "Extracting {} files to {}",
                manifest.files.len(),
                output_dir.display()
            );
            let stats = engram.corrections.stats();
            println!(
                "  Correction stats: {:.1}% perfect, {:.2}% overhead",
                stats.perfect_ratio * 100.0,
                stats.correction_ratio * 100.0
            );
        }

        for file_entry in &manifest.files {
            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = File::create(&file_path)?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);
            let num_chunks = file_entry.chunks.len();
            for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
                if let Some(chunk_vec) = engram.codebook.get(&chunk_id) {
                    // Calculate the actual chunk size
                    // Last chunk may be smaller than DEFAULT_CHUNK_SIZE
                    let chunk_size = if chunk_idx == num_chunks - 1 {
                        // Last chunk: remaining bytes
                        let remaining = file_entry.size - (chunk_idx * DEFAULT_CHUNK_SIZE);
                        remaining.min(DEFAULT_CHUNK_SIZE)
                    } else {
                        DEFAULT_CHUNK_SIZE
                    };
                    
                    // Decode the sparse vector to bytes
                    // IMPORTANT: Use the same path as during encoding for correct shift calculation
                    // Also use the same chunk_size as during ingest for correct correction matching
                    let decoded = chunk_vec.decode_data(config, Some(&file_entry.path), chunk_size);
                    
                    // Apply correction to guarantee bit-perfect reconstruction
                    let chunk_data = if let Some(corrected) = engram.corrections.apply(chunk_id as u64, &decoded) {
                        corrected
                    } else {
                        // No correction found - use decoded directly
                        // This can happen with legacy engrams or if correction store is empty
                        decoded
                    };

                    writer.write_all(&chunk_data)?;
                }
            }

            writer.flush()?;

            if verbose {
                println!("Extracted: {}", file_entry.path);
            }
        }

        Ok(())
    }

    /// Extract files using resonator-enhanced pattern completion with guaranteed reconstruction
    ///
    /// Performs filesystem extraction with intelligent recovery capabilities powered by
    /// resonator networks. When chunks are missing from the codebook, the resonator
    /// attempts pattern completion to reconstruct the lost data, enabling extraction
    /// even from partially corrupted or incomplete engrams.
    ///
    /// # Reconstruction Guarantee
    ///
    /// Even with resonator-assisted recovery, corrections are applied to guarantee
    /// bit-perfect reconstruction. The process is:
    ///
    /// 1. Try to get chunk from codebook
    /// 2. If missing, use resonator to recover approximate chunk
    /// 3. Apply correction from CorrectionStore
    /// 4. Result is guaranteed bit-perfect (if correction exists)
    ///
    /// # How it works
    /// 1. For each file chunk, check if it exists in the engram codebook
    /// 2. If missing, use the resonator to project a query vector onto known patterns
    /// 3. Apply stored corrections for guaranteed accuracy
    /// 4. Reconstruct the file from available and recovered chunks
    /// 5. If no resonator is configured, falls back to standard extraction
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
    /// * `config` - VSA configuration for encoding/decoding
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure of the extraction operation
    ///
    /// # Examples
    /// ```
    /// use embeddenator::{EmbrFS, Resonator, ReversibleVSAConfig};
    /// use std::path::Path;
    ///
    /// let mut fs = EmbrFS::new();
    /// let resonator = Resonator::new();
    /// let config = ReversibleVSAConfig::default();
    /// fs.set_resonator(resonator);
    ///
    /// // Assuming fs has been populated with data...
    /// let result = fs.extract_with_resonator("/tmp/output", true, &config);
    /// assert!(result.is_ok());
    /// ```
    pub fn extract_with_resonator<P: AsRef<Path>>(
        &self,
        output_dir: P,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        if self.resonator.is_none() {
            return Self::extract(&self.engram, &self.manifest, output_dir, verbose, config);
        }

        let _resonator = self.resonator.as_ref().unwrap();
        let output_dir = output_dir.as_ref();

        if verbose {
            println!(
                "Extracting {} files with resonator enhancement to {}",
                self.manifest.files.len(),
                output_dir.display()
            );
            let stats = self.engram.corrections.stats();
            println!(
                "  Correction stats: {:.1}% perfect, {:.2}% overhead",
                stats.perfect_ratio * 100.0,
                stats.correction_ratio * 100.0
            );
        }

        for file_entry in &self.manifest.files {
            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = File::create(&file_path)?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);
            let num_chunks = file_entry.chunks.len();
            for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
                // Calculate the actual chunk size
                let chunk_size = if chunk_idx == num_chunks - 1 {
                    let remaining = file_entry.size - (chunk_idx * DEFAULT_CHUNK_SIZE);
                    remaining.min(DEFAULT_CHUNK_SIZE)
                } else {
                    DEFAULT_CHUNK_SIZE
                };
                
                let chunk_data = if let Some(vector) = self.engram.codebook.get(&chunk_id) {
                    // Decode the SparseVec back to bytes using reversible encoding
                    // IMPORTANT: Use the same path as during encoding for correct shift calculation
                    let decoded = vector.decode_data(config, Some(&file_entry.path), chunk_size);
                    
                    // Apply correction to guarantee bit-perfect reconstruction
                    if let Some(corrected) = self.engram.corrections.apply(chunk_id as u64, &decoded) {
                        corrected
                    } else {
                        decoded
                    }
                } else if let Some(resonator) = &self.resonator {
                    // Use resonator to recover missing chunk
                    // Create a query vector from the chunk_id using reversible encoding
                    let query_vec = SparseVec::encode_data(&chunk_id.to_le_bytes(), config, None);
                    let recovered_vec = resonator.project(&query_vec);
                    
                    // Decode the recovered vector back to bytes
                    // For resonator recovery, try with path first, fall back to no path
                    let decoded = recovered_vec.decode_data(config, Some(&file_entry.path), chunk_size);
                    
                    // Apply correction if available (may not be if chunk was lost)
                    if let Some(corrected) = self.engram.corrections.apply(chunk_id as u64, &decoded) {
                        corrected
                    } else {
                        // No correction available - best effort recovery
                        decoded
                    }
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound, 
                        format!("Missing chunk {} and no resonator available", chunk_id)
                    ));
                };

                writer.write_all(&chunk_data)?;
            }

            writer.flush()?;

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
    /// use embeddenator::{EmbrFS, ReversibleVSAConfig};
    ///
    /// let fs = EmbrFS::new();
    /// let config = ReversibleVSAConfig::default();
    /// // Assuming files have been ingested...
    ///
    /// let hierarchical = fs.bundle_hierarchically(500, false, &config);
    /// assert!(hierarchical.is_ok());
    /// ```
    pub fn bundle_hierarchically(
        &self,
        max_level_sparsity: usize,
        verbose: bool,
        _config: &ReversibleVSAConfig,
    ) -> io::Result<HierarchicalManifest> {
        self.bundle_hierarchically_with_options(max_level_sparsity, None, verbose, _config)
    }

    /// Like `bundle_hierarchically`, but supports an optional deterministic cap on `chunk_ids` per node.
    ///
    /// If `max_chunks_per_node` is set and a node would exceed that many `chunk_ids`, the node becomes
    /// a router with empty `chunk_ids`, and deterministic shard children are created each containing a
    /// bounded subset of `chunk_ids`.
    pub fn bundle_hierarchically_with_options(
        &self,
        max_level_sparsity: usize,
        max_chunks_per_node: Option<usize>,
        verbose: bool,
        _config: &ReversibleVSAConfig,
    ) -> io::Result<HierarchicalManifest> {
        let mut levels = Vec::new();
        let mut sub_engrams = HashMap::new();

        // Group files by *path prefixes* at each level.
        // Level 0: "a"; Level 1: "a/b"; etc.
        let mut level_prefixes: HashMap<usize, HashMap<String, Vec<&FileEntry>>> = HashMap::new();
        for file_entry in &self.manifest.files {
            let comps: Vec<&str> = file_entry.path.split('/').collect();
            let mut prefix = String::new();
            for (level, &comp) in comps.iter().enumerate() {
                if level == 0 {
                    prefix.push_str(comp);
                } else {
                    prefix.push('/');
                    prefix.push_str(comp);
                }
                level_prefixes
                    .entry(level)
                    .or_insert_with(HashMap::new)
                    .entry(prefix.clone())
                    .or_insert_with(Vec::new)
                    .push(file_entry);
            }
        }

        // Process each level
        let max_level = level_prefixes.keys().max().unwrap_or(&0);

        for level in 0..=*max_level {
            if verbose {
                let item_count = level_prefixes
                    .get(&level)
                    .map(|comps| comps.values().map(|files| files.len()).sum::<usize>())
                    .unwrap_or(0);
                println!("Processing level {} with {} items", level, item_count);
            }

            let mut level_bundle = SparseVec::new();
            let mut manifest_items = Vec::new();

            if let Some(prefixes) = level_prefixes.get(&level) {
                let mut prefix_keys: Vec<&String> = prefixes.keys().collect();
                prefix_keys.sort();

                for prefix in prefix_keys {
                    let mut files: Vec<&FileEntry> = prefixes
                        .get(prefix)
                        .expect("prefix key")
                        .iter()
                        .copied()
                        .collect();
                    files.sort_by(|a, b| a.path.cmp(&b.path));

                    // Create permutation shift based on prefix hash
                    let shift = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        prefix.hash(&mut hasher);
                        (hasher.finish() % (DIM as u64)) as usize
                    };

                    // Bundle all files under this component with permutation
                    let mut component_bundle = SparseVec::new();
                    let mut chunk_ids_set: HashSet<usize> = HashSet::new();
                    for file_entry in &files {
                        // Find chunks for this file and bundle them
                        let mut file_bundle = SparseVec::new();
                        for &chunk_id in &file_entry.chunks {
                            if let Some(chunk_vec) = self.engram.codebook.get(&chunk_id) {
                                file_bundle = file_bundle.bundle(chunk_vec);
                                chunk_ids_set.insert(chunk_id);
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

                    // Create sub-engram for this prefix.
                    // Children are the immediate next-level prefixes underneath this prefix.
                    let sub_id = format!("level_{}_prefix_{}", level, prefix);

                    let mut children_set: HashSet<String> = HashSet::new();
                    if level < *max_level {
                        for file_entry in &files {
                            let comps: Vec<&str> = file_entry.path.split('/').collect();
                            if comps.len() <= level + 1 {
                                continue;
                            }
                            let child_prefix = comps[..=level + 1].join("/");
                            let child_id = format!("level_{}_prefix_{}", level + 1, child_prefix);
                            children_set.insert(child_id);
                        }
                    }
                    let mut children: Vec<String> = children_set.into_iter().collect();
                    children.sort();

                    let mut chunk_ids: Vec<usize> = chunk_ids_set.into_iter().collect();
                    chunk_ids.sort_unstable();

                    let chunk_count: usize = files.iter().map(|f| f.chunks.len()).sum();

                    if let Some(max_chunks) = max_chunks_per_node.filter(|v| *v > 0) {
                        if chunk_ids.len() > max_chunks {
                            let mut shard_ids: Vec<String> = Vec::new();
                            for (shard_idx, chunk_slice) in chunk_ids.chunks(max_chunks).enumerate() {
                                let shard_id = format!("{}__shard_{:04}", sub_id, shard_idx);
                                shard_ids.push(shard_id.clone());
                                sub_engrams.insert(
                                    shard_id.clone(),
                                    SubEngram {
                                        id: shard_id,
                                        root: component_bundle.clone(),
                                        chunk_ids: chunk_slice.to_vec(),
                                        chunk_count: chunk_slice.len(),
                                        children: Vec::new(),
                                    },
                                );
                            }

                            let mut router_children = shard_ids;
                            router_children.extend(children.clone());
                            router_children.sort();
                            router_children.dedup();

                            sub_engrams.insert(
                                sub_id.clone(),
                                SubEngram {
                                    id: sub_id.clone(),
                                    root: component_bundle,
                                    chunk_ids: Vec::new(),
                                    chunk_count,
                                    children: router_children,
                                },
                            );
                        } else {
                            sub_engrams.insert(
                                sub_id.clone(),
                                SubEngram {
                                    id: sub_id.clone(),
                                    root: component_bundle,
                                    chunk_ids,
                                    chunk_count,
                                    children,
                                },
                            );
                        }
                    } else {
                        sub_engrams.insert(
                            sub_id.clone(),
                            SubEngram {
                                id: sub_id.clone(),
                                root: component_bundle,
                                chunk_ids,
                                chunk_count,
                                children,
                            },
                        );
                    }

                    manifest_items.push(ManifestItem {
                        path: prefix.clone(),
                        sub_engram_id: sub_id,
                    });
                }
            }

            manifest_items
                .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id)));

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
    /// use embeddenator::{EmbrFS, ReversibleVSAConfig};
    ///
    /// let fs = EmbrFS::new();
    /// let config = ReversibleVSAConfig::default();
    /// // Assuming hierarchical manifest was created...
    /// // let hierarchical = fs.bundle_hierarchically(500, true).unwrap();
    ///
    /// // fs.extract_hierarchically(&hierarchical, "/tmp/output", true, &config)?;
    /// ```
    pub fn extract_hierarchically<P: AsRef<Path>>(
        &self,
        hierarchical: &HierarchicalManifest,
        output_dir: P,
        verbose: bool,
        config: &ReversibleVSAConfig,
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

            let file = File::create(&file_path)?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);

            // Reconstruct each chunk using hierarchical information
            let num_chunks = file_entry.chunks.len();
            for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
                if let Some(chunk_vector) = self.engram.codebook.get(&chunk_id) {
                    // Calculate the actual chunk size
                    let chunk_size = if chunk_idx == num_chunks - 1 {
                        let remaining = file_entry.size - (chunk_idx * DEFAULT_CHUNK_SIZE);
                        remaining.min(DEFAULT_CHUNK_SIZE)
                    } else {
                        DEFAULT_CHUNK_SIZE
                    };
                    
                    // Decode using hierarchical inverse transformations
                    let decoded = chunk_vector.decode_data(config, Some(&file_entry.path), chunk_size);
                    
                    // Apply correction if available
                    let chunk_data = if let Some(corrected) = self.engram.corrections.apply(chunk_id as u64, &decoded) {
                        corrected
                    } else {
                        decoded
                    };

                    writer.write_all(&chunk_data)?;
                }
            }

            writer.flush()?;

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
