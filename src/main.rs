use clap::{Parser, Subcommand};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ============================================================================
// Core VSA: Sparse Ternary Vector Symbolic Architecture
// ============================================================================

const DIM: usize = 10000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseVec {
    pub pos: Vec<usize>,
    pub neg: Vec<usize>,
}

impl SparseVec {
    pub fn new() -> Self {
        SparseVec {
            pos: Vec::new(),
            neg: Vec::new(),
        }
    }

    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let sparsity = DIM / 100; // ~1% density
        let mut indices: Vec<usize> = (0..DIM).collect();
        indices.sort_by_key(|_| rng.gen::<u32>());
        
        let pos = indices[..sparsity].to_vec();
        let neg = indices[sparsity..sparsity * 2].to_vec();
        
        SparseVec { pos, neg }
    }

    pub fn from_data(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        
        // SHA256 always produces 32 bytes, use first 32 bytes as seed
        let seed: [u8; 32] = hash[..32]
            .try_into()
            .expect("SHA256 output is always 32 bytes");
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        
        use rand::seq::SliceRandom;
        let mut indices: Vec<usize> = (0..DIM).collect();
        indices.shuffle(&mut rng);
        
        let sparsity = DIM / 100;
        let pos = indices[..sparsity].to_vec();
        let neg = indices[sparsity..sparsity * 2].to_vec();
        
        SparseVec { pos, neg }
    }

    pub fn bundle(&self, other: &SparseVec) -> SparseVec {
        use std::collections::HashSet;
        
        let pos_set: HashSet<_> = self.pos.iter().copied().collect();
        let neg_set: HashSet<_> = self.neg.iter().copied().collect();
        let other_pos_set: HashSet<_> = other.pos.iter().copied().collect();
        let other_neg_set: HashSet<_> = other.neg.iter().copied().collect();
        
        let mut result_pos: HashSet<usize> = HashSet::new();
        let mut result_neg: HashSet<usize> = HashSet::new();
        
        for &idx in &self.pos {
            if other_pos_set.contains(&idx) || !other_neg_set.contains(&idx) {
                result_pos.insert(idx);
            }
        }
        
        for &idx in &other.pos {
            if pos_set.contains(&idx) || !neg_set.contains(&idx) {
                result_pos.insert(idx);
            }
        }
        
        for &idx in &self.neg {
            if other_neg_set.contains(&idx) || !other_pos_set.contains(&idx) {
                result_neg.insert(idx);
            }
        }
        
        for &idx in &other.neg {
            if neg_set.contains(&idx) || !pos_set.contains(&idx) {
                result_neg.insert(idx);
            }
        }
        
        result_pos.retain(|&x| !result_neg.contains(&x));
        result_neg.retain(|&x| !result_pos.contains(&x));
        
        let mut pos: Vec<_> = result_pos.into_iter().collect();
        let mut neg: Vec<_> = result_neg.into_iter().collect();
        pos.sort_unstable();
        neg.sort_unstable();
        
        SparseVec { pos, neg }
    }

    pub fn bind(&self, other: &SparseVec) -> SparseVec {
        use std::collections::HashSet;
        
        let pos_set: HashSet<_> = self.pos.iter().copied().collect();
        let neg_set: HashSet<_> = self.neg.iter().copied().collect();
        
        let mut result_pos = Vec::new();
        let mut result_neg = Vec::new();
        
        for &idx in &other.pos {
            if pos_set.contains(&idx) {
                result_pos.push(idx);
            } else if neg_set.contains(&idx) {
                result_neg.push(idx);
            }
        }
        
        for &idx in &other.neg {
            if pos_set.contains(&idx) {
                result_neg.push(idx);
            } else if neg_set.contains(&idx) {
                result_pos.push(idx);
            }
        }
        
        result_pos.sort_unstable();
        result_neg.sort_unstable();
        
        SparseVec {
            pos: result_pos,
            neg: result_neg,
        }
    }

    pub fn cosine(&self, other: &SparseVec) -> f64 {
        use std::collections::HashSet;
        
        let pos_set: HashSet<_> = self.pos.iter().copied().collect();
        let neg_set: HashSet<_> = self.neg.iter().copied().collect();
        
        let mut dot = 0i32;
        for &idx in &other.pos {
            if pos_set.contains(&idx) {
                dot += 1;
            } else if neg_set.contains(&idx) {
                dot -= 1;
            }
        }
        
        for &idx in &other.neg {
            if pos_set.contains(&idx) {
                dot -= 1;
            } else if neg_set.contains(&idx) {
                dot += 1;
            }
        }
        
        let self_norm = (self.pos.len() + self.neg.len()) as f64;
        let other_norm = (other.pos.len() + other.neg.len()) as f64;
        
        if self_norm == 0.0 || other_norm == 0.0 {
            return 0.0;
        }
        
        dot as f64 / (self_norm.sqrt() * other_norm.sqrt())
    }
}

// ============================================================================
// Holographic Filesystem: EmbrFS
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct FileEntry {
    pub path: String,
    pub is_text: bool,
    pub size: usize,
    pub chunks: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub total_chunks: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Engram {
    pub root: SparseVec,
    pub codebook: HashMap<usize, Vec<u8>>,
}

pub struct EmbrFS {
    pub manifest: Manifest,
    pub engram: Engram,
}

impl EmbrFS {
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
            let relative = file_path
                .strip_prefix(dir)
                .unwrap_or_else(|_| file_path.as_path());
            self.ingest_file(&file_path, relative.to_string_lossy().to_string(), verbose)?;
        }

        Ok(())
    }

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

        let chunk_size = 4096;
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

    pub fn save_engram<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let encoded = bincode::serialize(&self.engram)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, encoded)?;
        Ok(())
    }

    pub fn load_engram<P: AsRef<Path>>(path: P) -> io::Result<Engram> {
        let data = fs::read(path)?;
        bincode::deserialize(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn save_manifest<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &self.manifest)?;
        Ok(())
    }

    pub fn load_manifest<P: AsRef<Path>>(path: P) -> io::Result<Manifest> {
        let file = File::open(path)?;
        let manifest = serde_json::from_reader(file)?;
        Ok(manifest)
    }

    pub fn extract<P: AsRef<Path>>(
        engram: &Engram,
        manifest: &Manifest,
        output_dir: P,
        verbose: bool,
    ) -> io::Result<()> {
        let output_dir = output_dir.as_ref();
        
        if verbose {
            println!("Extracting {} files to {}", manifest.files.len(), output_dir.display());
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

fn is_text_file(data: &[u8]) -> bool {
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

// ============================================================================
// CLI Interface
// ============================================================================

#[derive(Parser)]
#[command(name = "embeddenator")]
#[command(about = "Holographic computing substrate using sparse ternary VSA")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest files/directories into engram
    Ingest {
        /// Input directory to ingest
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output engram file (default: root.engram)
        #[arg(short, long, default_value = "root.engram")]
        engram: PathBuf,
        
        /// Output manifest file (default: manifest.json)
        #[arg(short, long, default_value = "manifest.json")]
        manifest: PathBuf,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Extract files from engram
    Extract {
        /// Input engram file (default: root.engram)
        #[arg(short, long, default_value = "root.engram")]
        engram: PathBuf,
        
        /// Input manifest file (default: manifest.json)
        #[arg(short, long, default_value = "manifest.json")]
        manifest: PathBuf,
        
        /// Output directory
        #[arg(short, long)]
        output_dir: PathBuf,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Query engram similarity
    Query {
        /// Engram file to query (default: root.engram)
        #[arg(short, long, default_value = "root.engram")]
        engram: PathBuf,
        
        /// Query file or pattern
        #[arg(short, long)]
        query: PathBuf,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            input,
            engram,
            manifest,
            verbose,
        } => {
            if verbose {
                println!("Embeddenator - Holographic Ingestion");
                println!("=====================================");
            }

            let mut fs = EmbrFS::new();
            fs.ingest_directory(&input, verbose)?;

            fs.save_engram(&engram)?;
            fs.save_manifest(&manifest)?;

            if verbose {
                println!("\nIngestion complete!");
                println!("  Engram: {}", engram.display());
                println!("  Manifest: {}", manifest.display());
                println!("  Files: {}", fs.manifest.files.len());
                println!("  Total chunks: {}", fs.manifest.total_chunks);
            }

            Ok(())
        }

        Commands::Extract {
            engram,
            manifest,
            output_dir,
            verbose,
        } => {
            if verbose {
                println!("Embeddenator - Holographic Extraction");
                println!("======================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;
            let manifest_data = EmbrFS::load_manifest(&manifest)?;

            EmbrFS::extract(&engram_data, &manifest_data, &output_dir, verbose)?;

            if verbose {
                println!("\nExtraction complete!");
                println!("  Output: {}", output_dir.display());
            }

            Ok(())
        }

        Commands::Query {
            engram,
            query,
            verbose,
        } => {
            if verbose {
                println!("Embeddenator - Holographic Query");
                println!("=================================");
            }

            let engram_data = EmbrFS::load_engram(&engram)?;

            let mut query_file = File::open(&query)?;
            let mut query_data = Vec::new();
            query_file.read_to_end(&mut query_data)?;

            let query_vec = SparseVec::from_data(&query_data);
            let similarity = query_vec.cosine(&engram_data.root);

            println!("Query file: {}", query.display());
            println!("Similarity to engram: {:.4}", similarity);

            if similarity > 0.75 {
                println!("Status: STRONG MATCH");
            } else if similarity > 0.3 {
                println!("Status: Partial match");
            } else {
                println!("Status: No significant match");
            }

            Ok(())
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_vec_bundle() {
        let v1 = SparseVec {
            pos: vec![1, 2, 3],
            neg: vec![4, 5, 6],
        };
        let v2 = SparseVec {
            pos: vec![2, 3, 7],
            neg: vec![5, 6, 8],
        };
        let result = v1.bundle(&v2);
        
        assert!(result.pos.contains(&2));
        assert!(result.pos.contains(&3));
    }

    #[test]
    fn test_sparse_vec_bind() {
        let v1 = SparseVec {
            pos: vec![1, 2, 3],
            neg: vec![4, 5, 6],
        };
        let v2 = SparseVec {
            pos: vec![2, 3, 7],
            neg: vec![5, 6, 8],
        };
        let result = v1.bind(&v2);
        
        assert!(result.pos.len() + result.neg.len() > 0);
    }

    #[test]
    fn test_sparse_vec_cosine() {
        let v1 = SparseVec {
            pos: vec![1, 2, 3],
            neg: vec![4, 5, 6],
        };
        let v2 = v1.clone();
        let similarity = v1.cosine(&v2);
        
        assert!(similarity > 0.9);
    }

    #[test]
    fn test_is_text_file() {
        let text_data = b"Hello, world!";
        assert!(is_text_file(text_data));
        
        let binary_data = vec![0u8, 1, 2, 3, 255, 0];
        assert!(!is_text_file(&binary_data));
    }
}
