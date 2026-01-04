//! Query command implementations

use anyhow::Result;
use embeddenator_fs::embrfs::{
    DirectorySubEngramStore, EmbrFS, HierarchicalQueryBounds,
    load_hierarchical_manifest, query_hierarchical_codebook_with_store,
};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub fn handle_query(
    engram: PathBuf,
    query: PathBuf,
    hierarchical_manifest: Option<PathBuf>,
    sub_engrams_dir: Option<PathBuf>,
    k: usize,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - Holographic Query",
            env!("CARGO_PKG_VERSION")
        );
        println!("=================================");
    }

    let engram_data = EmbrFS::load_engram(&engram)?;

    let mut query_file = File::open(&query)?;
    let mut query_data = Vec::new();
    query_file.read_to_end(&mut query_data)?;

    // Chunks are encoded with a path-hash bucket shift; when querying we don't know the
    // original path, so sweep possible buckets (bounded by config.max_path_depth).
    let config = ReversibleVSAConfig::default();
    let base_query = SparseVec::encode_data(&query_data, &config, None);

    // Build the codebook index once and reuse it across the sweep.
    let codebook_index = engram_data.build_codebook_index();

    let mut best_similarity = f64::MIN;
    let mut best_shift = 0usize;
    let mut best_top_cosine = f64::MIN;

    // Merge matches across shifts; keep the best score per chunk.
    let mut merged: HashMap<usize, (f64, i32)> = HashMap::new();

    // Optionally merge hierarchical hits too.
    let mut merged_hier: HashMap<(String, usize), (f64, i32)> = HashMap::new();

    let hierarchical_loaded = if let (Some(hier_path), Some(_)) =
        (hierarchical_manifest.as_ref(), sub_engrams_dir.as_ref())
    {
        Some(load_hierarchical_manifest(hier_path)?)
    } else {
        None
    };

    // Increase per-bucket cutoff so global top-k merge is less likely to miss true winners.
    let k_sweep = (k.saturating_mul(10)).max(100);
    let candidate_k = (k_sweep.saturating_mul(10)).max(200);

    for depth in 0..config.max_path_depth.max(1) {
        let shift = depth * config.base_shift;
        let query_vec = base_query.permute(shift);

        let similarity = query_vec.cosine(&engram_data.root);
        if similarity > best_similarity {
            best_similarity = similarity;
            best_shift = shift;
        }

        let matches = engram_data.query_codebook_with_index(
            &codebook_index,
            &query_vec,
            candidate_k,
            k_sweep,
        );

        if let Some(top) = matches.first() {
            if top.cosine > best_top_cosine {
                best_top_cosine = top.cosine;
                best_shift = shift;
                best_similarity = similarity;
            }
        }

        for m in matches {
            let entry = merged.entry(m.id).or_insert((m.cosine, m.approx_score));
            if m.cosine > entry.0 {
                *entry = (m.cosine, m.approx_score);
            }
        }
    }

    // Hierarchical query can be expensive (sub-engram loads + per-node indexing).
    // Run it once using the best shift from the sweep.
    if let (Some(hierarchical), Some(sub_dir)) =
        (hierarchical_loaded.as_ref(), sub_engrams_dir.as_ref())
    {
        let store = DirectorySubEngramStore::new(sub_dir);
        let bounds = HierarchicalQueryBounds {
            k,
            ..HierarchicalQueryBounds::default()
        };
        let query_vec = base_query.permute(best_shift);
        let hier_hits = query_hierarchical_codebook_with_store(
            hierarchical,
            &store,
            &engram_data.codebook,
            &query_vec,
            &bounds,
        );
        for h in hier_hits {
            let key = (h.sub_engram_id, h.chunk_id);
            let entry = merged_hier
                .entry(key)
                .or_insert((h.cosine, h.approx_score));
            if h.cosine > entry.0 {
                *entry = (h.cosine, h.approx_score);
            }
        }
    }

    println!("Query file: {}", query.display());
    if verbose {
        println!(
            "Best bucket-shift: {} (buckets 0..{})",
            best_shift,
            config.max_path_depth.saturating_sub(1)
        );
    }
    println!("Similarity to engram: {:.4}", best_similarity);

    let mut top_matches: Vec<(usize, f64, i32)> = merged
        .into_iter()
        .map(|(id, (cosine, approx))| (id, cosine, approx))
        .collect();
    top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    top_matches.truncate(k);

    if !top_matches.is_empty() {
        println!("Top codebook matches:");
        for (id, cosine, approx) in top_matches {
            println!("  chunk {}  cosine {:.4}  approx_dot {}", id, cosine, approx);
        }
    } else if verbose {
        println!("Top codebook matches: (none)");
    }

    let mut top_hier: Vec<(String, usize, f64, i32)> = merged_hier
        .into_iter()
        .map(|((sub_id, chunk_id), (cosine, approx))| (sub_id, chunk_id, cosine, approx))
        .collect();
    top_hier.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    top_hier.truncate(k);

    if !top_hier.is_empty() {
        println!("Top hierarchical matches:");
        for (sub_id, chunk_id, cosine, approx) in top_hier {
            println!(
                "  sub {}  chunk {}  cosine {:.4}  approx_dot {}",
                sub_id, chunk_id, cosine, approx
            );
        }
    } else if verbose && hierarchical_manifest.is_some() {
        println!("Top hierarchical matches: (none)");
    }

    if best_similarity > 0.75 {
        println!("Status: STRONG MATCH");
    } else if best_similarity > 0.3 {
        println!("Status: Partial match");
    } else {
        println!("Status: No significant match");
    }

    Ok(())
}

pub fn handle_query_text(
    engram: PathBuf,
    text: String,
    hierarchical_manifest: Option<PathBuf>,
    sub_engrams_dir: Option<PathBuf>,
    k: usize,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - Holographic Query (Text)",
            env!("CARGO_PKG_VERSION")
        );
        println!("========================================");
    }

    let engram_data = EmbrFS::load_engram(&engram)?;

    let config = ReversibleVSAConfig::default();
    let base_query = SparseVec::encode_data(text.as_bytes(), &config, None);

    let codebook_index = engram_data.build_codebook_index();

    let mut best_similarity = f64::MIN;
    let mut best_shift = 0usize;
    let mut best_top_cosine = f64::MIN;

    let mut merged: HashMap<usize, (f64, i32)> = HashMap::new();
    let mut merged_hier: HashMap<(String, usize), (f64, i32)> = HashMap::new();

    let hierarchical_loaded = if let (Some(hier_path), Some(_)) =
        (hierarchical_manifest.as_ref(), sub_engrams_dir.as_ref())
    {
        Some(load_hierarchical_manifest(hier_path)?)
    } else {
        None
    };

    let k_sweep = (k.saturating_mul(10)).max(100);
    let candidate_k = (k_sweep.saturating_mul(10)).max(200);

    for depth in 0..config.max_path_depth.max(1) {
        let shift = depth * config.base_shift;
        let query_vec = base_query.permute(shift);

        let similarity = query_vec.cosine(&engram_data.root);
        if similarity > best_similarity {
            best_similarity = similarity;
            best_shift = shift;
        }

        let matches = engram_data.query_codebook_with_index(
            &codebook_index,
            &query_vec,
            candidate_k,
            k_sweep,
        );

        if let Some(top) = matches.first() {
            if top.cosine > best_top_cosine {
                best_top_cosine = top.cosine;
                best_shift = shift;
                best_similarity = similarity;
            }
        }

        for m in matches {
            let entry = merged.entry(m.id).or_insert((m.cosine, m.approx_score));
            if m.cosine > entry.0 {
                *entry = (m.cosine, m.approx_score);
            }
        }
    }

    if let (Some(hierarchical), Some(sub_dir)) =
        (hierarchical_loaded.as_ref(), sub_engrams_dir.as_ref())
    {
        let store = DirectorySubEngramStore::new(sub_dir);
        let bounds = HierarchicalQueryBounds {
            k,
            ..HierarchicalQueryBounds::default()
        };
        let query_vec = base_query.permute(best_shift);
        let hier_hits = query_hierarchical_codebook_with_store(
            hierarchical,
            &store,
            &engram_data.codebook,
            &query_vec,
            &bounds,
        );
        for h in hier_hits {
            let key = (h.sub_engram_id, h.chunk_id);
            let entry = merged_hier
                .entry(key)
                .or_insert((h.cosine, h.approx_score));
            if h.cosine > entry.0 {
                *entry = (h.cosine, h.approx_score);
            }
        }
    }

    println!("Query text: {}", text);
    if verbose {
        println!(
            "Best bucket-shift: {} (buckets 0..{})",
            best_shift,
            config.max_path_depth.saturating_sub(1)
        );
    }
    println!("Similarity to engram: {:.4}", best_similarity);

    let mut top_matches: Vec<(usize, f64, i32)> = merged
        .into_iter()
        .map(|(id, (cosine, approx))| (id, cosine, approx))
        .collect();
    top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    top_matches.truncate(k);

    if !top_matches.is_empty() {
        println!("Top codebook matches:");
        for (id, cosine, approx) in top_matches {
            println!("  chunk {}  cosine {:.4}  approx_dot {}", id, cosine, approx);
        }
    } else if verbose {
        println!("Top codebook matches: (none)");
    }

    let mut top_hier: Vec<(String, usize, f64, i32)> = merged_hier
        .into_iter()
        .map(|((sub_id, chunk_id), (cosine, approx))| (sub_id, chunk_id, cosine, approx))
        .collect();
    top_hier.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    top_hier.truncate(k);

    if !top_hier.is_empty() {
        println!("Top hierarchical matches:");
        for (sub_id, chunk_id, cosine, approx) in top_hier {
            println!(
                "  sub {}  chunk {}  cosine {:.4}  approx_dot {}",
                sub_id, chunk_id, cosine, approx
            );
        }
    } else if verbose && hierarchical_manifest.is_some() {
        println!("Top hierarchical matches: (none)");
    }

    Ok(())
}
