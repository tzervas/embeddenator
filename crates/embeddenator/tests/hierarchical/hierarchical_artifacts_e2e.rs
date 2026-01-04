use std::fs;

use embeddenator::{
    query_hierarchical_codebook_with_store, DirectorySubEngramStore, EmbrFS,
    HierarchicalQueryBounds, ReversibleVSAConfig, SparseVec,
};

#[test]
fn bundle_hier_and_store_backed_query_can_find_expected_chunks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // Create a small tree with one distinctive file.
    fs::create_dir_all(root.join("a")).expect("mkdir a");
    fs::create_dir_all(root.join("b")).expect("mkdir b");

    let needle_path = root.join("a/needle.txt");
    let hay_path = root.join("b/hay.txt");

    let needle_bytes = b"needle needle needle\n".to_vec();
    fs::write(&needle_path, &needle_bytes).expect("write needle");
    fs::write(&hay_path, b"just hay\n").expect("write hay");

    // Ingest.
    let config = ReversibleVSAConfig::default();
    let mut fsys = EmbrFS::new();
    fsys.ingest_directory(root, false, &config).expect("ingest_directory");

    // Identify which chunks correspond to the needle file.
    let needle_logical = "a/needle.txt";
    let needle_entry = fsys
        .manifest
        .files
        .iter()
        .find(|f| f.path == needle_logical)
        .expect("needle file entry");
    let needle_chunks = needle_entry.chunks.clone();
    assert!(!needle_chunks.is_empty());

    // Build hierarchical artifacts.
    let hierarchical = fsys
        .bundle_hierarchically(500, false, &config)
        .expect("bundle_hierarchically");

    // Persist sub-engrams and a manifest JSON that contains no embedded sub_engrams (store-backed only).
    let sub_dir = root.join("sub_engrams");
    embeddenator::save_sub_engrams_dir(&hierarchical.sub_engrams, &sub_dir)
        .expect("save_sub_engrams_dir");

    let hier_path = root.join("hier.json");
    let mut store_only = hierarchical;
    store_only.sub_engrams.clear();
    embeddenator::save_hierarchical_manifest(&store_only, &hier_path)
        .expect("save_hierarchical_manifest");
    let loaded = embeddenator::load_hierarchical_manifest(&hier_path).expect("load_hierarchical_manifest");

    let store = DirectorySubEngramStore::new(&sub_dir);

    // Query for the needle bytes. Chunks are encoded with an unknown path shift, so sweep.
    let base_query = SparseVec::encode_data(&needle_bytes, &config, None);
    let bounds = HierarchicalQueryBounds {
        k: 10,
        ..HierarchicalQueryBounds::default()
    };

    let mut found = false;
    for depth in 0..config.max_path_depth.max(1) {
        let shift = depth * config.base_shift;
        let q = base_query.permute(shift);
        let hits = query_hierarchical_codebook_with_store(&loaded, &store, &fsys.engram.codebook, &q, &bounds);
        if hits
            .iter()
            .any(|h| needle_chunks.iter().any(|cid| *cid == h.chunk_id))
        {
            found = true;
            break;
        }
    }

    assert!(found, "did not recover any needle chunk via hierarchical query");
}
