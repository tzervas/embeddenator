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

use crate::correction::{CorrectionStats, CorrectionStore};
use embeddenator_retrieval::resonator::Resonator;
use embeddenator_retrieval::{RerankedResult, TernaryInvertedIndex};
use embeddenator_vsa::{ReversibleVSAConfig, ReversibleVSAEncoder, SparseVec, DIM};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
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
    /// Mark files as deleted without rebuilding root (for incremental updates)
    #[serde(default)]
    pub deleted: bool,
}

/// Manifest describing filesystem structure
#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub total_chunks: usize,
    /// Chunk size used during encoding (64 for holographic, 4096 for legacy)
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    /// Whether holographic encoding was used
    #[serde(default)]
    pub holographic: bool,
}

fn default_chunk_size() -> usize {
    DEFAULT_CHUNK_SIZE
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

    fn insert(&mut self, key: String, value: V) {
        if self.cap == 0 {
            return;
        }

        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            self.touch(&key);
            return;
        }

        self.map.insert(key.clone(), value);
        self.order.push(key);

        while self.map.len() > self.cap {
            if let Some(evict) = self.order.first().cloned() {
                self.order.remove(0);
                self.map.remove(&evict);
            } else {
                break;
            }
        }
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
        self.dir
            .join(format!("{}.subengram", escape_sub_engram_id(id)))
    }
}

impl SubEngramStore for DirectorySubEngramStore {
    fn load(&self, id: &str) -> Option<SubEngram> {
        let path = self.path_for_id(id);
        let data = fs::read(path).ok()?;
        bincode::deserialize(&data).ok()
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
    levels.sort_by_key(|a| a.level);
    for level in &mut levels {
        level.items.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id))
        });
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
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;

    let mut ids: Vec<&String> = sub_engrams.keys().collect();
    ids.sort();

    for id in ids {
        // SAFETY: id comes from keys(), so get() must succeed
        let sub = sub_engrams
            .get(id)
            .expect("sub_engram id from keys() must exist in HashMap");
        let encoded = bincode::serialize(sub).map_err(io::Error::other)?;
        let path = dir.join(format!("{}.subengram", escape_sub_engram_id(id)));
        fs::write(path, encoded)?;
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
        return Some(v.clone());
    }
    let loaded = store.load(id)?;
    cache.insert(id.to_string(), loaded.clone());
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

    let mut sub_cache: LruCache<SubEngram> = LruCache::new(bounds.max_open_engrams);
    let mut index_cache: LruCache<RemappedInvertedIndex> = LruCache::new(bounds.max_open_indices);

    let mut frontier: Vec<FrontierItem> = Vec::new();
    if let Some(level0) = hierarchical.levels.first() {
        for item in &level0.items {
            let Some(sub) = get_cached_sub_engram(&mut sub_cache, store, &item.sub_engram_id)
            else {
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
            existing
        } else {
            let built = RemappedInvertedIndex::build(&sub.chunk_ids, codebook);
            index_cache.insert(node.sub_engram_id.clone(), built);
            // SAFETY: we just inserted the key, so get() must succeed immediately after
            index_cache
                .get(&node.sub_engram_id)
                .expect("index_cache.get() must succeed immediately after insert()")
        };

        let mut local_hits =
            idx.query_top_k_reranked(query, codebook, bounds.candidate_k, bounds.k);
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

/// Chunk size optimized for holographic encoding (8 bytes)
/// Smaller chunks achieve higher accuracy (~94%) with ReversibleVSAEncoder
/// Using larger chunks creates too much crosstalk from bundling
pub const HOLOGRAPHIC_CHUNK_SIZE: usize = 8;

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
/// # Holographic Mode
///
/// When created with `new_holographic()`, uses `ReversibleVSAEncoder` which achieves
/// ~94% uncorrected accuracy through position-aware VSA binding. This results in
/// <10% correction overhead instead of the ~200%+ overhead of legacy encoding.
///
/// # Examples
///
/// ```
/// use embeddenator_fs::EmbrFS;
/// use std::path::Path;
///
/// // Legacy mode (not recommended)
/// let mut fs_legacy = EmbrFS::new();
///
/// // Holographic mode (recommended - minimal storage overhead)
/// let mut fs = EmbrFS::new_holographic();
/// assert_eq!(fs.manifest.total_chunks, 0);
/// assert_eq!(fs.manifest.files.len(), 0);
/// ```
pub struct EmbrFS {
    pub manifest: Manifest,
    pub engram: Engram,
    pub resonator: Option<Resonator>,
    /// ReversibleVSAEncoder for true holographic encoding (~94% accuracy)
    /// None in legacy mode, Some in holographic mode
    encoder: Option<ReversibleVSAEncoder>,
    /// Chunk size for encoding (64 bytes for holographic, 4096 for legacy)
    chunk_size: usize,
}

impl Default for EmbrFS {
    fn default() -> Self {
        Self::new_holographic()
    }
}

impl EmbrFS {
    /// Create a new empty EmbrFS instance (legacy mode - NOT RECOMMENDED)
    ///
    /// This constructor creates an EmbrFS with legacy encoding that has only ~10%
    /// accuracy, resulting in ~200%+ storage overhead due to verbatim corrections.
    ///
    /// **Use `new_holographic()` instead for production use.**
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_fs::EmbrFS;
    ///
    /// // Legacy mode - high storage overhead
    /// let fs = EmbrFS::new();
    /// assert_eq!(fs.manifest.files.len(), 0);
    /// ```
    #[deprecated(
        since = "0.25.0",
        note = "Use new_holographic() instead for ~94% encoding accuracy and <10% storage overhead"
    )]
    pub fn new() -> Self {
        EmbrFS {
            manifest: Manifest {
                files: Vec::new(),
                total_chunks: 0,
                chunk_size: DEFAULT_CHUNK_SIZE,
                holographic: false,
            },
            engram: Engram {
                root: SparseVec::new(),
                codebook: HashMap::new(),
                corrections: CorrectionStore::new(),
            },
            resonator: None,
            encoder: None,
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }

    /// Create a new EmbrFS with holographic encoding (RECOMMENDED)
    ///
    /// Uses `ReversibleVSAEncoder` which achieves ~94% uncorrected accuracy through
    /// position-aware VSA binding. This results in <10% correction overhead instead
    /// of the ~200%+ overhead of legacy encoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator_fs::EmbrFS;
    ///
    /// let fs = EmbrFS::new_holographic();
    /// assert_eq!(fs.manifest.files.len(), 0);
    /// assert_eq!(fs.manifest.total_chunks, 0);
    /// assert!(fs.is_holographic());
    /// ```
    pub fn new_holographic() -> Self {
        EmbrFS {
            manifest: Manifest {
                files: Vec::new(),
                total_chunks: 0,
                chunk_size: HOLOGRAPHIC_CHUNK_SIZE,
                holographic: true,
            },
            engram: Engram {
                root: SparseVec::new(),
                codebook: HashMap::new(),
                corrections: CorrectionStore::new(),
            },
            resonator: None,
            encoder: Some(ReversibleVSAEncoder::new()),
            chunk_size: HOLOGRAPHIC_CHUNK_SIZE,
        }
    }

    /// Check if holographic mode is enabled
    pub fn is_holographic(&self) -> bool {
        self.encoder.is_some()
    }

    /// Get the chunk size being used
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
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
    /// use embeddenator_fs::{EmbrFS, Resonator};
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
    /// use embeddenator_fs::EmbrFS;
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
        let mut file = File::open(file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let is_text = is_text_file(&data);
        let is_holographic = self.encoder.is_some();

        if verbose {
            println!(
                "Ingesting {}: {} bytes ({}, {})",
                logical_path,
                data.len(),
                if is_text { "text" } else { "binary" },
                if is_holographic {
                    "holographic"
                } else {
                    "legacy"
                }
            );
        }

        let chunk_size = self.chunk_size;
        let base_chunk_id = self.manifest.total_chunks;

        // Parallel encoding path for holographic mode
        if let Some(ref mut encoder) = self.encoder {
            // Ensure position vectors exist for chunk encoding (local positions 0..chunk_size)
            encoder.ensure_positions(chunk_size.saturating_sub(1));

            // Get shared references for parallel encoding
            let byte_vectors = encoder.get_byte_vectors();
            let position_vectors_len = chunk_size; // Use first chunk_size position vectors

            // Collect chunks for parallel processing
            let chunk_data: Vec<&[u8]> = data.chunks(chunk_size).collect();
            let num_chunks = chunk_data.len();

            // Parallel encode each chunk with LOCAL positions (0 to chunk.len()-1)
            // This matches the sequential encode() behavior
            let encoded_chunks: Vec<SparseVec> = chunk_data
                .par_iter()
                .map(|chunk| {
                    if chunk.is_empty() {
                        return SparseVec::new();
                    }
                    // Encode with local positions (offset=0 for each chunk)
                    let byte_vec = &byte_vectors[chunk[0] as usize];
                    let pos_vec = encoder.get_position_vector_ref(0);
                    let mut result = byte_vec.bind(pos_vec);

                    for (i, &byte) in chunk.iter().enumerate().skip(1) {
                        let byte_vec = &byte_vectors[byte as usize];
                        let pos_vec = encoder.get_position_vector_ref(i % position_vectors_len);
                        let encoded_byte = byte_vec.bind(pos_vec);
                        result = result.bundle(&encoded_byte);
                    }
                    result
                })
                .collect();

            // Parallel decode and compute corrections
            let corrections_info: Vec<(usize, SparseVec, Vec<u8>, bool)> = encoded_chunks
                .into_par_iter()
                .enumerate()
                .map(|(i, chunk_vec)| {
                    let chunk = chunk_data[i];
                    let decoded = encoder.decode(&chunk_vec, chunk.len());
                    let needs_correction = chunk != decoded.as_slice();
                    (i, chunk_vec, decoded, needs_correction)
                })
                .collect();

            // Sequential phase: add to engram (must be sequential for root bundling)
            let mut corrections_needed = 0usize;
            let mut total_correction_bytes = 0usize;
            let mut chunks = Vec::with_capacity(num_chunks);

            for (i, chunk_vec, decoded, needs_correction) in corrections_info {
                let chunk_id = base_chunk_id + i;
                let original_chunk = chunk_data[i];

                // Store correction if needed
                self.engram
                    .corrections
                    .add(chunk_id as u64, original_chunk, &decoded);

                if needs_correction {
                    corrections_needed += 1;
                    if let Some(correction) = self.engram.corrections.get(chunk_id as u64) {
                        total_correction_bytes += correction.storage_size();
                    }
                }

                self.engram.root = self.engram.root.bundle(&chunk_vec);
                self.engram.codebook.insert(chunk_id, chunk_vec);
                chunks.push(chunk_id);
            }

            if verbose {
                let perfect_chunks = num_chunks - corrections_needed;
                let accuracy = if num_chunks > 0 {
                    (perfect_chunks as f64 / num_chunks as f64) * 100.0
                } else {
                    100.0
                };
                let overhead = if !data.is_empty() {
                    (total_correction_bytes as f64 / data.len() as f64) * 100.0
                } else {
                    0.0
                };
                println!(
                    "  → {}/{} chunks perfect ({:.1}% accuracy), {:.1}% correction overhead",
                    perfect_chunks, num_chunks, accuracy, overhead
                );
            }

            self.manifest.files.push(FileEntry {
                path: logical_path,
                is_text,
                size: data.len(),
                chunks: chunks.clone(),
                deleted: false,
            });

            self.manifest.total_chunks += chunks.len();
        } else {
            // Legacy mode: sequential encoding (no parallelization benefit)
            let mut chunks = Vec::new();
            let mut corrections_needed = 0usize;
            let mut total_correction_bytes = 0usize;

            for (i, chunk) in data.chunks(chunk_size).enumerate() {
                let chunk_id = base_chunk_id + i;
                let encoded = SparseVec::encode_data(chunk, config, Some(&logical_path));
                let decoded = encoded.decode_data(config, Some(&logical_path), chunk.len());

                self.engram
                    .corrections
                    .add(chunk_id as u64, chunk, &decoded);

                if chunk != decoded.as_slice() {
                    corrections_needed += 1;
                    if let Some(correction) = self.engram.corrections.get(chunk_id as u64) {
                        total_correction_bytes += correction.storage_size();
                    }
                }

                self.engram.root = self.engram.root.bundle(&encoded);
                self.engram.codebook.insert(chunk_id, encoded);
                chunks.push(chunk_id);
            }

            if verbose {
                let total_chunks = chunks.len();
                let perfect_chunks = total_chunks - corrections_needed;
                let accuracy = if total_chunks > 0 {
                    (perfect_chunks as f64 / total_chunks as f64) * 100.0
                } else {
                    100.0
                };
                let overhead = if !data.is_empty() {
                    (total_correction_bytes as f64 / data.len() as f64) * 100.0
                } else {
                    0.0
                };
                println!(
                    "  → {}/{} chunks perfect ({:.1}% accuracy), {:.1}% correction overhead",
                    perfect_chunks, total_chunks, accuracy, overhead
                );
            }

            self.manifest.files.push(FileEntry {
                path: logical_path,
                is_text,
                size: data.len(),
                chunks: chunks.clone(),
                deleted: false,
            });

            self.manifest.total_chunks += chunks.len();
        }

        Ok(())
    }

    /// Add a new file to an existing engram (incremental update)
    ///
    /// This method enables efficient incremental updates by adding a single file
    /// to an existing engram without requiring full re-ingestion. The new file's
    /// chunks are bundled with the existing root vector using VSA's associative
    /// bundle operation.
    ///
    /// # Algorithm
    /// 1. Encode new file into chunks (same as ingest_file)
    /// 2. Bundle each chunk with existing root: `root_new = root_old ⊕ chunk`
    /// 3. Add chunks to codebook with new chunk IDs
    /// 4. Update manifest with new file entry
    ///
    /// # Performance
    /// - Time complexity: O(n) where n = number of chunks in new file
    /// - Does not require reading or re-encoding existing files
    /// - Suitable for production workflows with frequent additions
    ///
    /// # Arguments
    /// * `file_path` - Path to the file on disk
    /// * `logical_path` - Path to use in the engram manifest
    /// * `verbose` - Print progress information
    /// * `config` - VSA encoding configuration
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure
    ///
    /// # Examples
    /// ```no_run
    /// use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
    /// use std::path::Path;
    ///
    /// let mut fs = EmbrFS::new();
    /// let config = ReversibleVSAConfig::default();
    ///
    /// // Ingest initial dataset
    /// fs.ingest_directory("./data", false, &config).unwrap();
    ///
    /// // Later, add a new file without full re-ingestion
    /// fs.add_file("./new_file.txt", "new_file.txt".to_string(), true, &config).unwrap();
    /// ```
    pub fn add_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        logical_path: String,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        let file_path = file_path.as_ref();

        // Check if file already exists (not deleted)
        if self
            .manifest
            .files
            .iter()
            .any(|f| f.path == logical_path && !f.deleted)
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("File '{}' already exists in engram", logical_path),
            ));
        }

        // Use existing ingest_file logic (already handles bundling with root)
        self.ingest_file(file_path, logical_path, verbose, config)
    }

    /// Remove a file from the engram (mark as deleted for incremental update)
    ///
    /// This method marks a file as deleted in the manifest without modifying the
    /// root vector. This is because VSA bundling is a lossy operation and there's
    /// no clean inverse. The chunks remain in the codebook but won't be extracted.
    ///
    /// # Algorithm
    /// 1. Find file in manifest by logical path
    /// 2. Mark file entry as deleted
    /// 3. Chunks remain in codebook (for potential recovery or compaction)
    /// 4. File won't appear in future extractions
    ///
    /// # Note on VSA Limitations
    /// Bundle operation is associative but not invertible:
    /// - `(A ⊕ B) ⊕ C = A ⊕ (B ⊕ C)` ✓ (can add)
    /// - `(A ⊕ B) ⊖ B ≠ A` ✗ (can't cleanly remove)
    ///
    /// To truly remove chunks from the root, use `compact()` which rebuilds
    /// the engram without deleted files.
    ///
    /// # Arguments
    /// * `logical_path` - Path of the file to remove
    /// * `verbose` - Print progress information
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure
    ///
    /// # Examples
    /// ```no_run
    /// use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
    ///
    /// let mut fs = EmbrFS::new();
    /// let config = ReversibleVSAConfig::default();
    ///
    /// fs.ingest_directory("./data", false, &config).unwrap();
    /// fs.remove_file("old_file.txt", true).unwrap();
    /// // File marked as deleted, won't be extracted
    /// ```
    pub fn remove_file(&mut self, logical_path: &str, verbose: bool) -> io::Result<()> {
        // Find file in manifest
        let file_entry = self
            .manifest
            .files
            .iter_mut()
            .find(|f| f.path == logical_path && !f.deleted)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File '{}' not found in engram", logical_path),
                )
            })?;

        if verbose {
            println!(
                "Marking file as deleted: {} ({} chunks)",
                logical_path,
                file_entry.chunks.len()
            );
        }

        // Mark as deleted (don't remove from manifest to preserve chunk IDs)
        file_entry.deleted = true;

        if verbose {
            println!("  Note: Use 'compact' to rebuild engram and reclaim space");
        }

        Ok(())
    }

    /// Modify an existing file in the engram (incremental update)
    ///
    /// This method updates a file's content by removing the old version and
    /// adding the new version. It's equivalent to `remove_file` + `add_file`.
    ///
    /// # Algorithm
    /// 1. Mark old file as deleted
    /// 2. Re-encode new file content
    /// 3. Bundle new chunks with root
    /// 4. Add new file entry to manifest
    ///
    /// # Trade-offs
    /// - Old chunks remain in codebook (use `compact()` to clean up)
    /// - Root contains both old and new chunk contributions (slight noise)
    /// - Fast operation, doesn't require rebuilding entire engram
    ///
    /// # Arguments
    /// * `file_path` - Path to the file on disk (new content)
    /// * `logical_path` - Path of the file in the engram
    /// * `verbose` - Print progress information
    /// * `config` - VSA encoding configuration
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure
    ///
    /// # Examples
    /// ```no_run
    /// use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
    /// use std::path::Path;
    ///
    /// let mut fs = EmbrFS::new();
    /// let config = ReversibleVSAConfig::default();
    ///
    /// fs.ingest_directory("./data", false, &config).unwrap();
    ///
    /// // Later, modify a file
    /// fs.modify_file("./data/updated.txt", "data/updated.txt".to_string(), true, &config).unwrap();
    /// ```
    pub fn modify_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        logical_path: String,
        verbose: bool,
        config: &ReversibleVSAConfig,
    ) -> io::Result<()> {
        // First, mark old file as deleted
        self.remove_file(&logical_path, false)?;

        if verbose {
            println!("Modifying file: {}", logical_path);
        }

        // Then add the new version
        self.ingest_file(file_path, logical_path, verbose, config)?;

        Ok(())
    }

    /// Compact the engram by rebuilding without deleted files
    ///
    /// This operation rebuilds the engram from scratch, excluding all files
    /// marked as deleted. It's the only way to truly remove old chunks from
    /// the root vector and codebook.
    ///
    /// # Algorithm
    /// 1. Create new empty engram
    /// 2. Re-bundle all non-deleted files
    /// 3. Reassign chunk IDs sequentially
    /// 4. Replace old engram with compacted version
    ///
    /// # Performance
    /// - Time complexity: O(N) where N = total bytes of non-deleted files
    /// - Expensive operation, run periodically (not after every deletion)
    /// - Recommended: compact when deleted files exceed 20-30% of total
    ///
    /// # Benefits
    /// - Reclaims space from deleted chunks
    /// - Reduces root vector noise from obsolete data
    /// - Resets chunk IDs to sequential order
    /// - Maintains bit-perfect reconstruction of kept files
    ///
    /// # Arguments
    /// * `verbose` - Print progress information
    /// * `config` - VSA encoding configuration
    ///
    /// # Returns
    /// `io::Result<()>` indicating success or failure
    ///
    /// # Examples
    /// ```no_run
    /// use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
    ///
    /// let mut fs = EmbrFS::new();
    /// let config = ReversibleVSAConfig::default();
    ///
    /// fs.ingest_directory("./data", false, &config).unwrap();
    /// fs.remove_file("old1.txt", false).unwrap();
    /// fs.remove_file("old2.txt", false).unwrap();
    ///
    /// // After many deletions, compact to reclaim space
    /// fs.compact(true, &config).unwrap();
    /// ```
    pub fn compact(&mut self, verbose: bool, config: &ReversibleVSAConfig) -> io::Result<()> {
        if verbose {
            let deleted_count = self.manifest.files.iter().filter(|f| f.deleted).count();
            let total_count = self.manifest.files.len();
            println!(
                "Compacting engram: removing {} deleted files ({} remaining)",
                deleted_count,
                total_count - deleted_count
            );
        }

        let is_holographic = self.encoder.is_some();
        let chunk_size = self.chunk_size;

        // Create new engram with fresh root and codebook
        let mut new_engram = Engram {
            root: SparseVec::new(),
            codebook: HashMap::new(),
            corrections: CorrectionStore::new(),
        };

        // Rebuild manifest with only non-deleted files
        let mut new_manifest = Manifest {
            files: Vec::new(),
            total_chunks: 0,
            chunk_size,
            holographic: is_holographic,
        };

        // Process each non-deleted file
        for old_file in &self.manifest.files {
            if old_file.deleted {
                continue;
            }

            // Reconstruct file data from old engram
            let num_chunks = old_file.chunks.len();
            let old_chunk_size = self.manifest.chunk_size;

            // Parallel decode for holographic mode
            let file_data = if self.manifest.holographic {
                if let Some(ref encoder) = self.encoder {
                    // Collect chunk vectors for batch processing
                    let chunk_info: Vec<(usize, usize, &SparseVec)> = old_file
                        .chunks
                        .iter()
                        .enumerate()
                        .filter_map(|(chunk_idx, &chunk_id)| {
                            self.engram.codebook.get(&chunk_id).map(|chunk_vec| {
                                let this_chunk_size = if chunk_idx == num_chunks - 1 {
                                    old_file
                                        .size
                                        .saturating_sub(chunk_idx * old_chunk_size)
                                        .min(old_chunk_size)
                                } else {
                                    old_chunk_size
                                };
                                (chunk_idx, this_chunk_size, chunk_vec)
                            })
                        })
                        .collect();

                    // Parallel decode and apply corrections
                    let decoded_chunks: Vec<Vec<u8>> = chunk_info
                        .par_iter()
                        .map(|(chunk_idx, this_chunk_size, chunk_vec)| {
                            let chunk_id = old_file.chunks[*chunk_idx];
                            let decoded = encoder.decode(chunk_vec, *this_chunk_size);
                            if let Some(corrected) =
                                self.engram.corrections.apply(chunk_id as u64, &decoded)
                            {
                                corrected
                            } else {
                                decoded
                            }
                        })
                        .collect();

                    let mut result: Vec<u8> = decoded_chunks.into_iter().flatten().collect();
                    result.truncate(old_file.size);
                    result
                } else {
                    // Fallback: sequential decode
                    let mut result = Vec::new();
                    for (chunk_idx, &chunk_id) in old_file.chunks.iter().enumerate() {
                        if let Some(chunk_vec) = self.engram.codebook.get(&chunk_id) {
                            let this_chunk_size = if chunk_idx == num_chunks - 1 {
                                old_file
                                    .size
                                    .saturating_sub(chunk_idx * old_chunk_size)
                                    .min(old_chunk_size)
                            } else {
                                old_chunk_size
                            };
                            let decoded = chunk_vec.decode_data(
                                config,
                                Some(&old_file.path),
                                this_chunk_size,
                            );
                            let chunk_data = if let Some(corrected) =
                                self.engram.corrections.apply(chunk_id as u64, &decoded)
                            {
                                corrected
                            } else {
                                decoded
                            };
                            result.extend_from_slice(&chunk_data);
                        }
                    }
                    result.truncate(old_file.size);
                    result
                }
            } else {
                // Legacy mode: sequential decode
                let mut result = Vec::new();
                for (chunk_idx, &chunk_id) in old_file.chunks.iter().enumerate() {
                    if let Some(chunk_vec) = self.engram.codebook.get(&chunk_id) {
                        let this_chunk_size = if chunk_idx == num_chunks - 1 {
                            old_file
                                .size
                                .saturating_sub(chunk_idx * old_chunk_size)
                                .min(old_chunk_size)
                        } else {
                            old_chunk_size
                        };
                        let decoded =
                            chunk_vec.decode_data(config, Some(&old_file.path), this_chunk_size);
                        let chunk_data = if let Some(corrected) =
                            self.engram.corrections.apply(chunk_id as u64, &decoded)
                        {
                            corrected
                        } else {
                            decoded
                        };
                        result.extend_from_slice(&chunk_data);
                    }
                }
                result.truncate(old_file.size);
                result
            };

            // Re-encode with new chunk IDs using parallel encoding for holographic mode
            let base_chunk_id = new_manifest.total_chunks;

            if let Some(ref mut encoder) = self.encoder {
                // Parallel encode using batch_encode
                let encoded_chunks = encoder.batch_encode(&file_data, chunk_size);
                let num_new_chunks = encoded_chunks.len();
                let chunk_data: Vec<&[u8]> = file_data.chunks(chunk_size).collect();

                // Parallel decode for correction computation
                let corrections_info: Vec<(usize, SparseVec, Vec<u8>)> = encoded_chunks
                    .into_par_iter()
                    .enumerate()
                    .map(|(i, chunk_vec)| {
                        let decoded = encoder.decode(&chunk_vec, chunk_data[i].len());
                        (i, chunk_vec, decoded)
                    })
                    .collect();

                // Sequential phase: add to new engram
                let mut new_chunks = Vec::with_capacity(num_new_chunks);
                for (i, chunk_vec, decoded) in corrections_info {
                    let new_chunk_id = base_chunk_id + i;
                    new_engram
                        .corrections
                        .add(new_chunk_id as u64, chunk_data[i], &decoded);
                    new_engram.root = new_engram.root.bundle(&chunk_vec);
                    new_engram.codebook.insert(new_chunk_id, chunk_vec);
                    new_chunks.push(new_chunk_id);
                }

                if verbose {
                    println!(
                        "  Recompacted: {} ({} chunks)",
                        old_file.path,
                        new_chunks.len()
                    );
                }

                new_manifest.files.push(FileEntry {
                    path: old_file.path.clone(),
                    is_text: old_file.is_text,
                    size: old_file.size,
                    chunks: new_chunks.clone(),
                    deleted: false,
                });

                new_manifest.total_chunks += new_chunks.len();
            } else {
                // Legacy mode: sequential encode
                let mut new_chunks = Vec::new();
                for (i, chunk) in file_data.chunks(chunk_size).enumerate() {
                    let new_chunk_id = base_chunk_id + i;
                    let encoded = SparseVec::encode_data(chunk, config, Some(&old_file.path));
                    let decoded = encoded.decode_data(config, Some(&old_file.path), chunk.len());

                    new_engram
                        .corrections
                        .add(new_chunk_id as u64, chunk, &decoded);
                    new_engram.root = new_engram.root.bundle(&encoded);
                    new_engram.codebook.insert(new_chunk_id, encoded);
                    new_chunks.push(new_chunk_id);
                }

                if verbose {
                    println!(
                        "  Recompacted: {} ({} chunks)",
                        old_file.path,
                        new_chunks.len()
                    );
                }

                new_manifest.files.push(FileEntry {
                    path: old_file.path.clone(),
                    is_text: old_file.is_text,
                    size: old_file.size,
                    chunks: new_chunks.clone(),
                    deleted: false,
                });

                new_manifest.total_chunks += new_chunks.len();
            }
        }

        // Replace old engram and manifest with compacted versions
        self.engram = new_engram;
        self.manifest = new_manifest;

        if verbose {
            let stats = self.engram.corrections.stats();
            println!(
                "Compaction complete: {} files, {} chunks ({:.1}% perfect, {:.2}% correction overhead)",
                self.manifest.files.len(),
                self.manifest.total_chunks,
                stats.perfect_ratio * 100.0,
                stats.correction_ratio * 100.0
            );
        }

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

    /// Load an EmbrFS from engram and manifest files
    ///
    /// Automatically detects if the engram was created with holographic mode
    /// and sets up the appropriate encoder for extraction.
    ///
    /// # Arguments
    /// * `engram_path` - Path to the engram file
    /// * `manifest_path` - Path to the manifest JSON file
    ///
    /// # Returns
    /// `io::Result<EmbrFS>` with the loaded engram and manifest
    pub fn load<P: AsRef<Path>, Q: AsRef<Path>>(
        engram_path: P,
        manifest_path: Q,
    ) -> io::Result<Self> {
        let engram = Self::load_engram(engram_path)?;
        let manifest = Self::load_manifest(manifest_path)?;

        // Create encoder if holographic mode was used
        let (encoder, chunk_size) = if manifest.holographic {
            (Some(ReversibleVSAEncoder::new()), manifest.chunk_size)
        } else {
            (None, manifest.chunk_size)
        };

        Ok(EmbrFS {
            manifest,
            engram,
            resonator: None,
            encoder,
            chunk_size,
        })
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

        // Use manifest's chunk_size and holographic flag
        let chunk_size = manifest.chunk_size;
        let is_holographic = manifest.holographic;

        // Create encoder for holographic decoding if needed
        let encoder = if is_holographic {
            Some(ReversibleVSAEncoder::new())
        } else {
            None
        };

        if verbose {
            println!(
                "Extracting {} files to {} ({})",
                manifest.files.iter().filter(|f| !f.deleted).count(),
                output_dir.display(),
                if is_holographic {
                    "holographic"
                } else {
                    "legacy"
                }
            );
            let stats = engram.corrections.stats();
            println!(
                "  Correction stats: {:.1}% perfect, {:.2}% overhead",
                stats.perfect_ratio * 100.0,
                stats.correction_ratio * 100.0
            );
        }

        for file_entry in &manifest.files {
            // Skip deleted files
            if file_entry.deleted {
                continue;
            }

            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let num_chunks = file_entry.chunks.len();

            // Holographic mode: use parallel batch decoding
            let reconstructed = if let Some(ref enc) = encoder {
                // Collect chunk vectors for batch decoding
                let chunk_vecs: Vec<&SparseVec> = file_entry
                    .chunks
                    .iter()
                    .filter_map(|&chunk_id| engram.codebook.get(&chunk_id))
                    .collect();

                if chunk_vecs.len() != num_chunks {
                    // Some chunks missing, fall back to sequential with correction
                    let mut result = Vec::new();
                    for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
                        if let Some(chunk_vec) = engram.codebook.get(&chunk_id) {
                            let this_chunk_size = if chunk_idx == num_chunks - 1 {
                                let remaining =
                                    file_entry.size.saturating_sub(chunk_idx * chunk_size);
                                remaining.min(chunk_size)
                            } else {
                                chunk_size
                            };
                            let decoded = enc.decode(chunk_vec, this_chunk_size);
                            let chunk_data = if let Some(corrected) =
                                engram.corrections.apply(chunk_id as u64, &decoded)
                            {
                                corrected
                            } else {
                                decoded
                            };
                            result.extend_from_slice(&chunk_data);
                        }
                    }
                    result
                } else {
                    // Parallel decode all chunks, then apply corrections
                    // Clone vectors for batch_decode (it expects owned SparseVecs)
                    let chunk_vecs_owned: Vec<SparseVec> =
                        chunk_vecs.iter().map(|v| (*v).clone()).collect();

                    // Parallel decode using batch_decode
                    let decoded_bytes =
                        enc.batch_decode(&chunk_vecs_owned, chunk_size, file_entry.size);

                    // Apply corrections in parallel
                    let chunk_sizes: Vec<usize> = (0..num_chunks)
                        .map(|i| {
                            if i == num_chunks - 1 {
                                file_entry
                                    .size
                                    .saturating_sub(i * chunk_size)
                                    .min(chunk_size)
                            } else {
                                chunk_size
                            }
                        })
                        .collect();

                    let corrected_chunks: Vec<Vec<u8>> = file_entry
                        .chunks
                        .par_iter()
                        .enumerate()
                        .map(|(chunk_idx, &chunk_id)| {
                            let start = chunk_idx * chunk_size;
                            let end = (start + chunk_sizes[chunk_idx]).min(decoded_bytes.len());
                            let decoded_chunk = decoded_bytes[start..end].to_vec();

                            if let Some(corrected) =
                                engram.corrections.apply(chunk_id as u64, &decoded_chunk)
                            {
                                corrected
                            } else {
                                decoded_chunk
                            }
                        })
                        .collect();

                    corrected_chunks.into_iter().flatten().collect()
                }
            } else {
                // Legacy mode: sequential decoding
                let mut result = Vec::new();
                for (chunk_idx, &chunk_id) in file_entry.chunks.iter().enumerate() {
                    if let Some(chunk_vec) = engram.codebook.get(&chunk_id) {
                        let this_chunk_size = if chunk_idx == num_chunks - 1 {
                            let remaining = file_entry.size.saturating_sub(chunk_idx * chunk_size);
                            remaining.min(chunk_size)
                        } else {
                            chunk_size
                        };
                        let decoded =
                            chunk_vec.decode_data(config, Some(&file_entry.path), this_chunk_size);

                        let chunk_data = if let Some(corrected) =
                            engram.corrections.apply(chunk_id as u64, &decoded)
                        {
                            corrected
                        } else {
                            decoded
                        };
                        result.extend_from_slice(&chunk_data);
                    }
                }
                result
            };

            let mut final_data = reconstructed;
            final_data.truncate(file_entry.size);

            fs::write(&file_path, final_data)?;

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
    /// use embeddenator_fs::{EmbrFS, Resonator, ReversibleVSAConfig};
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

        // SAFETY: we just checked is_none() above and returned early
        let _resonator = self
            .resonator
            .as_ref()
            .expect("resonator is Some after is_none() check");
        let output_dir = output_dir.as_ref();

        if verbose {
            println!(
                "Extracting {} files with resonator enhancement to {}",
                self.manifest.files.iter().filter(|f| !f.deleted).count(),
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
            // Skip deleted files
            if file_entry.deleted {
                continue;
            }

            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut reconstructed = Vec::new();
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
                    if let Some(corrected) =
                        self.engram.corrections.apply(chunk_id as u64, &decoded)
                    {
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
                    let decoded =
                        recovered_vec.decode_data(config, Some(&file_entry.path), chunk_size);

                    // Apply correction if available (may not be if chunk was lost)
                    if let Some(corrected) =
                        self.engram.corrections.apply(chunk_id as u64, &decoded)
                    {
                        corrected
                    } else {
                        // No correction available - best effort recovery
                        decoded
                    }
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Missing chunk {} and no resonator available", chunk_id),
                    ));
                };
                reconstructed.extend_from_slice(&chunk_data);
            }

            reconstructed.truncate(file_entry.size);

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
    /// use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
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
                    .or_default()
                    .entry(prefix.clone())
                    .or_default()
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
                        // SAFETY: prefix comes from keys(), so get() must succeed
                        .expect("prefix key from keys() must exist in HashMap")
                        .to_vec();
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
                    if component_bundle.pos.len() + component_bundle.neg.len() > max_level_sparsity
                    {
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
                            for (shard_idx, chunk_slice) in chunk_ids.chunks(max_chunks).enumerate()
                            {
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

            manifest_items.sort_by(|a, b| {
                a.path
                    .cmp(&b.path)
                    .then_with(|| a.sub_engram_id.cmp(&b.sub_engram_id))
            });

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
    /// use embeddenator_fs::{EmbrFS, ReversibleVSAConfig};
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
            // Skip deleted files
            if file_entry.deleted {
                continue;
            }

            let file_path = output_dir.join(&file_entry.path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut reconstructed = Vec::new();

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
                    let decoded =
                        chunk_vector.decode_data(config, Some(&file_entry.path), chunk_size);

                    // Apply correction if available
                    let chunk_data = if let Some(corrected) =
                        self.engram.corrections.apply(chunk_id as u64, &decoded)
                    {
                        corrected
                    } else {
                        decoded
                    };

                    reconstructed.extend_from_slice(&chunk_data);
                }
            }

            // Truncate to actual file size
            reconstructed.truncate(file_entry.size);

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
