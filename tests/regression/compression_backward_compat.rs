use embeddenator::{DirectorySubEngramStore, EmbrFS, ReversibleVSAConfig, SparseVec, SubEngram, SubEngramStore};
use std::fs;
use std::io;
use std::path::Path;

fn write_file<P: AsRef<Path>>(path: P, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

#[test]
fn load_engram_accepts_legacy_raw_bincode() {
    let td = tempfile::tempdir().expect("tempdir");
    let input_dir = td.path().join("in");
    let out_dir = td.path().join("out");
    fs::create_dir_all(&input_dir).expect("mkdir");

    let original_bytes = b"legacy raw bincode engram test\n\x00\x01\x02".to_vec();
    write_file(input_dir.join("x.bin"), &original_bytes).expect("write input");

    let mut fsys = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    fsys.ingest_directory(&input_dir, false, &config)
        .expect("ingest");

    // Simulate the pre-envelope format: raw bincode of `Engram`.
    let legacy_path = td.path().join("legacy.engram");
    let raw = bincode::serialize(&fsys.engram).expect("bincode serialize");
    fs::write(&legacy_path, raw).expect("write legacy engram");

    let manifest_path = td.path().join("manifest.json");
    fsys.save_manifest(&manifest_path).expect("save manifest");
    let manifest = EmbrFS::load_manifest(&manifest_path).expect("load manifest");

    let loaded = EmbrFS::load_engram(&legacy_path).expect("load legacy engram");
    EmbrFS::extract(&loaded, &manifest, &out_dir, false, &config).expect("extract");

    let extracted = fs::read(out_dir.join("x.bin")).expect("read extracted");
    assert_eq!(extracted, original_bytes);
}

#[test]
fn directory_sub_engram_store_loads_legacy_raw_bincode_subengram() {
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path().join("sub_engrams");
    fs::create_dir_all(&dir).expect("mkdir");

    let id = "node0";
    let sub = SubEngram {
        id: id.to_string(),
        root: SparseVec {
            pos: vec![1, 7, 42],
            neg: vec![3, 9],
        },
        chunk_ids: vec![10, 11, 12],
        chunk_count: 3,
        children: vec!["child".to_string()],
    };

    // Legacy format: raw bincode blob (no EDN1 envelope).
    let raw = bincode::serialize(&sub).expect("serialize subengram");
    fs::write(dir.join(format!("{}.subengram", id)), raw).expect("write subengram");

    let store = DirectorySubEngramStore::new(&dir);
    let loaded = store.load(id).expect("store load");

    assert_eq!(loaded.id, sub.id);
    assert_eq!(loaded.chunk_ids, sub.chunk_ids);
    assert_eq!(loaded.chunk_count, sub.chunk_count);
    assert_eq!(loaded.children, sub.children);
    assert_eq!(loaded.root.pos, sub.root.pos);
    assert_eq!(loaded.root.neg, sub.root.neg);
}

#[cfg(feature = "compression-zstd")]
#[test]
fn cli_ingest_extract_with_compressed_engram() {
    use std::process::Command;

    let td = tempfile::tempdir().expect("tempdir");
    let input_dir = td.path().join("in");
    let out_dir = td.path().join("out");
    fs::create_dir_all(&input_dir).expect("mkdir");

    let payload = b"hello from a compressed engram";
    write_file(input_dir.join("a.txt"), payload).expect("write input");

    let engram = td.path().join("root.engram");
    let manifest = td.path().join("manifest.json");

    let status = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "ingest",
            "-i",
            input_dir.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "--engram-compression",
            "zstd",
        ])
        .status()
        .expect("run ingest");
    assert!(status.success(), "ingest failed: {status}");

    let bytes = fs::read(&engram).expect("read engram");
    assert!(bytes.len() >= 4);
    assert_eq!(&bytes[..4], b"EDN1", "engram should be envelope-wrapped");

    let status = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "extract",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .expect("run extract");
    assert!(status.success(), "extract failed: {status}");

    let extracted = fs::read(out_dir.join("a.txt")).expect("read extracted");
    assert_eq!(extracted, payload);
}

#[cfg(feature = "compression-lz4")]
#[test]
fn cli_ingest_extract_with_compressed_engram_lz4() {
    use std::process::Command;

    let td = tempfile::tempdir().expect("tempdir");
    let input_dir = td.path().join("in");
    let out_dir = td.path().join("out");
    fs::create_dir_all(&input_dir).expect("mkdir");

    let payload = b"hello from an lz4-compressed engram";
    write_file(input_dir.join("a.txt"), payload).expect("write input");

    let engram = td.path().join("root.engram");
    let manifest = td.path().join("manifest.json");

    let status = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "ingest",
            "-i",
            input_dir.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "--engram-compression",
            "lz4",
        ])
        .status()
        .expect("run ingest");
    assert!(status.success(), "ingest failed: {status}");

    let bytes = fs::read(&engram).expect("read engram");
    assert!(bytes.len() >= 4);
    assert_eq!(&bytes[..4], b"EDN1", "engram should be envelope-wrapped");

    let status = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "extract",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .expect("run extract");
    assert!(status.success(), "extract failed: {status}");

    let extracted = fs::read(out_dir.join("a.txt")).expect("read extracted");
    assert_eq!(extracted, payload);
}

#[cfg(feature = "compression-zstd")]
#[test]
fn bundle_hier_compressed_subengrams_and_query_text() {
    use std::process::Command;

    let td = tempfile::tempdir().expect("tempdir");
    let input_dir = td.path().join("in");
    fs::create_dir_all(&input_dir).expect("mkdir");

    write_file(input_dir.join("note.txt"), b"alpha beta gamma delta")
        .expect("write input");

    let engram = td.path().join("root.engram");
    let manifest = td.path().join("manifest.json");

    let status = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "ingest",
            "-i",
            input_dir.to_str().unwrap(),
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
        ])
        .status()
        .expect("run ingest");
    assert!(status.success(), "ingest failed: {status}");

    let hier = td.path().join("hier.json");
    let sub_dir = td.path().join("sub_engrams");

    let status = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "bundle-hier",
            "-e",
            engram.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
            "--out-hierarchical-manifest",
            hier.to_str().unwrap(),
            "--out-sub-engrams-dir",
            sub_dir.to_str().unwrap(),
            "--sub-engram-compression",
            "zstd",
        ])
        .status()
        .expect("run bundle-hier");
    assert!(status.success(), "bundle-hier failed: {status}");

    // Ensure at least one subengram is written and envelope-wrapped.
    let mut saw_wrapped = false;
    for entry in fs::read_dir(&sub_dir).expect("read sub_dir") {
        let entry = entry.expect("dir entry");
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("subengram") {
            continue;
        }
        let bytes = fs::read(&p).expect("read subengram");
        if bytes.len() >= 4 && &bytes[..4] == b"EDN1" {
            saw_wrapped = true;
            break;
        }
    }
    assert!(saw_wrapped, "expected at least one wrapped .subengram");

    // Query using the store-backed hierarchical artifacts.
    let out = Command::new(env!("CARGO_BIN_EXE_embeddenator"))
        .args([
            "query-text",
            "-e",
            engram.to_str().unwrap(),
            "--text",
            "alpha",
            "--hierarchical-manifest",
            hier.to_str().unwrap(),
            "--sub-engrams-dir",
            sub_dir.to_str().unwrap(),
            "--k",
            "3",
            "-v",
        ])
        .output()
        .expect("run query-text");

    assert!(out.status.success(), "query-text failed: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Similarity to engram:"),
        "expected similarity output, got: {stdout}"
    );
    assert!(
        stdout.contains("Top hierarchical matches"),
        "expected hierarchical output, got: {stdout}"
    );
}
