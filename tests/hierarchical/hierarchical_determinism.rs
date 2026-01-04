use std::collections::BTreeMap;
use std::fs;

use embeddenator::{save_sub_engrams_dir, EmbrFS, ReversibleVSAConfig};

fn list_filenames(dir: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    out.sort();
    out
}

#[test]
fn bundle_hier_is_deterministic_and_sharding_stable() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // Force multiple chunks under the same prefix to exercise sharding.
    fs::create_dir_all(root.join("a")).expect("mkdir a");

    // Make a file large enough to create multiple chunks.
    let big = vec![0xABu8; embeddenator::DEFAULT_CHUNK_SIZE * 3 + 123];
    fs::write(root.join("a/big.bin"), &big).expect("write big.bin");

    let config = ReversibleVSAConfig::default();
    let mut fsys = EmbrFS::new();
    fsys.ingest_directory(root, false, &config).expect("ingest");

    let h1 = fsys
        .bundle_hierarchically_with_options(500, Some(2), false, &config)
        .expect("bundle 1");
    let h2 = fsys
        .bundle_hierarchically_with_options(500, Some(2), false, &config)
        .expect("bundle 2");

    // Compare stable, order-normalized views of the manifest and sub-engram metadata.
    let levels1: Vec<(u32, Vec<(String, String)>)> = h1
        .levels
        .iter()
        .map(|lvl| {
            let mut items: Vec<(String, String)> = lvl
                .items
                .iter()
                .map(|it| (it.path.clone(), it.sub_engram_id.clone()))
                .collect();
            items.sort();
            (lvl.level, items)
        })
        .collect();

    let levels2: Vec<(u32, Vec<(String, String)>)> = h2
        .levels
        .iter()
        .map(|lvl| {
            let mut items: Vec<(String, String)> = lvl
                .items
                .iter()
                .map(|it| (it.path.clone(), it.sub_engram_id.clone()))
                .collect();
            items.sort();
            (lvl.level, items)
        })
        .collect();

    assert_eq!(levels1, levels2, "levels/items drifted between runs");

    let sub_meta = |h: &embeddenator::HierarchicalManifest| {
        let mut m: BTreeMap<String, (Vec<usize>, Vec<String>, usize)> = BTreeMap::new();
        for (id, sub) in &h.sub_engrams {
            m.insert(
                id.clone(),
                (sub.chunk_ids.clone(), sub.children.clone(), sub.chunk_count),
            );
        }
        m
    };

    assert_eq!(sub_meta(&h1), sub_meta(&h2), "sub-engram metadata drifted");

    // Sub-engram directory writes should be stable (same file set).
    let d1 = root.join("sub_engrams_1");
    let d2 = root.join("sub_engrams_2");
    save_sub_engrams_dir(&h1.sub_engrams, &d1).expect("save 1");
    save_sub_engrams_dir(&h2.sub_engrams, &d2).expect("save 2");

    assert_eq!(list_filenames(&d1), list_filenames(&d2), "sub-engram filenames drifted");
}
