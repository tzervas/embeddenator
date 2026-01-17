use std::collections::HashMap;

use embeddenator::embrfs::{ManifestItem, ManifestLevel};
use embeddenator::{
    query_hierarchical_codebook, HierarchicalManifest, HierarchicalQueryBounds, SparseVec,
    SubEngram,
};
use embeddenator::{
    query_hierarchical_codebook_with_store, save_hierarchical_manifest, save_sub_engrams_dir,
    DirectorySubEngramStore,
};

fn sv(pos: &[usize], neg: &[usize]) -> SparseVec {
    let mut v = SparseVec::new();
    v.pos = pos.to_vec();
    v.neg = neg.to_vec();
    v
}

#[test]
fn hierarchical_unfolding_respects_bounds_and_is_deterministic() {
    // Codebook: chunk 0 is the best match; chunk 1 is a partial match; chunk 2 is anti-match.
    let query = sv(&[1, 2, 3, 10], &[]);

    let mut codebook: HashMap<usize, SparseVec> = HashMap::new();
    codebook.insert(0, sv(&[1, 2, 3, 10], &[]));
    codebook.insert(1, sv(&[1, 2], &[]));
    codebook.insert(2, sv(&[], &[1, 2, 3, 10]));

    // Two level-0 nodes; only one has the best chunk.
    let mut sub_engrams: HashMap<String, SubEngram> = HashMap::new();
    sub_engrams.insert(
        "A".to_string(),
        SubEngram {
            id: "A".to_string(),
            root: sv(&[1, 2, 3], &[]),
            chunk_ids: vec![0, 1],
            chunk_count: 2,
            children: vec!["A/child".to_string()],
        },
    );
    sub_engrams.insert(
        "A/child".to_string(),
        SubEngram {
            id: "A/child".to_string(),
            root: sv(&[1, 2, 3, 10], &[]),
            chunk_ids: vec![0],
            chunk_count: 1,
            children: vec![],
        },
    );
    sub_engrams.insert(
        "B".to_string(),
        SubEngram {
            id: "B".to_string(),
            root: sv(&[], &[1, 2, 3]),
            chunk_ids: vec![2],
            chunk_count: 1,
            children: vec![],
        },
    );

    let hierarchical = HierarchicalManifest {
        version: 1,
        levels: vec![ManifestLevel {
            level: 0,
            items: vec![
                ManifestItem {
                    path: "A".to_string(),
                    sub_engram_id: "A".to_string(),
                },
                ManifestItem {
                    path: "B".to_string(),
                    sub_engram_id: "B".to_string(),
                },
            ],
        }],
        sub_engrams,
    };

    // Tight bounds: only 1 expansion should occur.
    let bounds = HierarchicalQueryBounds {
        k: 2,
        candidate_k: 10,
        beam_width: 1,
        max_depth: 10,
        max_expansions: 1,
        max_open_indices: 2,
        max_open_engrams: 2,
    };

    let r1 = query_hierarchical_codebook(&hierarchical, &codebook, &query, &bounds);
    let r2 = query_hierarchical_codebook(&hierarchical, &codebook, &query, &bounds);

    assert_eq!(r1, r2);
    assert!(!r1.is_empty());

    // With only one expansion, we either expanded A or B. Beam width=1 should pick A (better cosine).
    assert!(r1.iter().any(|h| h.chunk_id == 0));

    // Ensure k is respected.
    assert!(r1.len() <= 2);
}

#[test]
fn hierarchical_unfolding_can_descend_into_children() {
    let query = sv(&[5, 6, 7], &[]);

    let mut codebook: HashMap<usize, SparseVec> = HashMap::new();
    codebook.insert(0, sv(&[5, 6, 7], &[]));
    codebook.insert(1, sv(&[5], &[]));

    let mut sub_engrams: HashMap<String, SubEngram> = HashMap::new();
    sub_engrams.insert(
        "root".to_string(),
        SubEngram {
            id: "root".to_string(),
            root: sv(&[5], &[]),
            chunk_ids: vec![1],
            chunk_count: 1,
            children: vec!["child".to_string()],
        },
    );
    sub_engrams.insert(
        "child".to_string(),
        SubEngram {
            id: "child".to_string(),
            root: sv(&[5, 6, 7], &[]),
            chunk_ids: vec![0],
            chunk_count: 1,
            children: vec![],
        },
    );

    let hierarchical = HierarchicalManifest {
        version: 1,
        levels: vec![ManifestLevel {
            level: 0,
            items: vec![ManifestItem {
                path: "root".to_string(),
                sub_engram_id: "root".to_string(),
            }],
        }],
        sub_engrams,
    };

    let bounds = HierarchicalQueryBounds {
        k: 1,
        candidate_k: 10,
        beam_width: 8,
        max_depth: 2,
        max_expansions: 8,
        max_open_indices: 8,
        max_open_engrams: 8,
    };

    let results = query_hierarchical_codebook(&hierarchical, &codebook, &query, &bounds);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].chunk_id, 0);
    assert_eq!(results[0].sub_engram_id, "child");
}

#[test]
fn hierarchical_unfolding_can_load_sub_engrams_from_directory_store() {
    let query = sv(&[9, 11], &[]);

    let mut codebook: HashMap<usize, SparseVec> = HashMap::new();
    codebook.insert(0, sv(&[9, 11], &[]));
    codebook.insert(1, sv(&[9], &[]));

    let mut sub_engrams: HashMap<String, SubEngram> = HashMap::new();
    sub_engrams.insert(
        "root".to_string(),
        SubEngram {
            id: "root".to_string(),
            root: sv(&[9], &[]),
            chunk_ids: vec![1],
            chunk_count: 1,
            children: vec!["child".to_string()],
        },
    );
    sub_engrams.insert(
        "child".to_string(),
        SubEngram {
            id: "child".to_string(),
            root: sv(&[9, 11], &[]),
            chunk_ids: vec![0],
            chunk_count: 1,
            children: vec![],
        },
    );

    // Persist sub-engrams to a temp directory.
    let tmp = tempfile::tempdir().expect("tempdir");
    let sub_dir = tmp.path().join("sub_engrams");
    save_sub_engrams_dir(&sub_engrams, &sub_dir).expect("save_sub_engrams_dir");

    // Create a manifest that contains no embedded sub_engrams (store-backed loading only).
    let hierarchical = HierarchicalManifest {
        version: 1,
        levels: vec![ManifestLevel {
            level: 0,
            items: vec![ManifestItem {
                path: "root".to_string(),
                sub_engram_id: "root".to_string(),
            }],
        }],
        sub_engrams: HashMap::new(),
    };

    // Also ensure save/load of the manifest works with empty sub_engrams.
    let hier_path = tmp.path().join("hier.json");
    save_hierarchical_manifest(&hierarchical, &hier_path).expect("save_hierarchical_manifest");
    let loaded_hier =
        embeddenator::load_hierarchical_manifest(&hier_path).expect("load_hierarchical_manifest");

    let store = DirectorySubEngramStore::new(&sub_dir);
    let bounds = HierarchicalQueryBounds {
        k: 1,
        candidate_k: 10,
        beam_width: 8,
        max_depth: 2,
        max_expansions: 8,
        max_open_indices: 2,
        max_open_engrams: 2,
    };

    let results =
        query_hierarchical_codebook_with_store(&loaded_hier, &store, &codebook, &query, &bounds);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].chunk_id, 0);
    assert_eq!(results[0].sub_engram_id, "child");
}
