//! Test ebook modification and reconstruction
//!
//! Demonstrates modifying some ebooks in an engram while leaving others unchanged,
//! then verifying all files are correctly reconstructed.

use embeddenator_fs::VersionedEmbrFS;
use std::collections::HashMap;

fn sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn main() {
    println!("=== Ebook Modification Test ===\n");

    // Create filesystem and load all ebooks
    let fs = VersionedEmbrFS::new();

    // Load original ebooks
    let ebook_dir = "/tmp/ebook-test";
    let mut original_hashes: HashMap<String, String> = HashMap::new();

    let files = vec![
        "discourses_epictetus.txt",
        "enchiridion_epictetus.epub",
        "enchiridion_epictetus.txt",
        "meditations_marcus_aurelius.epub",
        "meditations_marcus_aurelius.txt",
        "moral_letters_seneca.txt",
        "shortness_of_life_seneca.txt",
    ];

    println!("1. Loading ebooks into engram...");
    for file in &files {
        let path = format!("{}/{}", ebook_dir, file);
        let data = std::fs::read(&path).unwrap_or_else(|_| panic!("Failed to read {}", file));
        let hash = sha256(&data);
        original_hashes.insert(file.to_string(), hash.clone());
        fs.write_file(file, &data, None)
            .unwrap_or_else(|_| panic!("Failed to write {}", file));
        println!(
            "   Loaded {}: {} bytes, SHA256: {}...",
            file,
            data.len(),
            &hash[..16]
        );
    }

    let stats = fs.stats();
    println!(
        "\n   Total: {} files, {} chunks, version {}",
        stats.active_files, stats.total_chunks, stats.version
    );

    // Modify some files
    println!("\n2. Modifying selected ebooks (without full recompute)...");

    // Modify moral_letters_seneca.txt - add an annotation
    let (seneca_data, seneca_version) = fs
        .read_file("moral_letters_seneca.txt")
        .expect("Failed to read Seneca");
    let seneca_modified = format!(
        "=== ANNOTATED EDITION ===\n\
         This version has been modified for testing.\n\
         Original text follows:\n\n{}",
        String::from_utf8_lossy(&seneca_data)
    );
    fs.write_file(
        "moral_letters_seneca.txt",
        seneca_modified.as_bytes(),
        Some(seneca_version),
    )
    .expect("Failed to modify Seneca");
    println!("   ✓ Modified moral_letters_seneca.txt (added header annotation)");

    // Modify meditations - append a note
    let (meditations_data, meditations_version) = fs
        .read_file("meditations_marcus_aurelius.txt")
        .expect("Failed to read Meditations");
    let meditations_modified = format!(
        "{}\n\n=== END OF TEXT ===\n\
         Modification test appended on 2026-01-26.\n\
         This tests that modifications persist correctly.",
        String::from_utf8_lossy(&meditations_data)
    );
    fs.write_file(
        "meditations_marcus_aurelius.txt",
        meditations_modified.as_bytes(),
        Some(meditations_version),
    )
    .expect("Failed to modify Meditations");
    println!("   ✓ Modified meditations_marcus_aurelius.txt (appended footer)");

    // Add a completely new file
    let new_file_content = b"=== STOIC WISDOM COLLECTION ===\n\n\
        This file was added to the engram after initial ingestion.\n\
        It tests incremental file addition via VSA bundle operation.\n\n\
        Key Stoic Principles:\n\
        1. Focus on what you can control\n\
        2. Accept what you cannot change\n\
        3. Live according to nature\n\
        4. Practice virtue daily\n";
    fs.write_file("stoic_wisdom_notes.txt", new_file_content, None)
        .expect("Failed to add new file");
    println!("   ✓ Added new file: stoic_wisdom_notes.txt");

    let stats = fs.stats();
    println!(
        "\n   After modifications: {} files, {} chunks, version {}",
        stats.active_files, stats.total_chunks, stats.version
    );

    // Verify all files
    println!("\n3. Verifying all files (modified and unmodified)...");
    println!();

    let mut all_pass = true;

    // Check unmodified files - should match original hashes
    let unmodified_files = vec![
        "discourses_epictetus.txt",
        "enchiridion_epictetus.epub",
        "enchiridion_epictetus.txt",
        "meditations_marcus_aurelius.epub",
        "shortness_of_life_seneca.txt",
    ];

    println!("   Unmodified files (should match original):");
    for file in &unmodified_files {
        let (data, _) = fs
            .read_file(file)
            .unwrap_or_else(|_| panic!("Failed to read {}", file));
        let hash = sha256(&data);
        let original_hash = original_hashes.get(*file).unwrap();
        if &hash == original_hash {
            println!("   ✓ {}: UNCHANGED (hash matches)", file);
        } else {
            println!("   ✗ {}: CORRUPTED (hash mismatch!)", file);
            all_pass = false;
        }
    }

    // Check modified files - should be different from original
    println!();
    println!("   Modified files (should differ from original):");

    let (seneca_final, _) = fs.read_file("moral_letters_seneca.txt").unwrap();
    let seneca_final_str = String::from_utf8_lossy(&seneca_final);
    if seneca_final_str.contains("=== ANNOTATED EDITION ===") {
        println!("   ✓ moral_letters_seneca.txt: CORRECTLY MODIFIED");
    } else {
        println!("   ✗ moral_letters_seneca.txt: MODIFICATION LOST");
        all_pass = false;
    }

    let (meditations_final, _) = fs.read_file("meditations_marcus_aurelius.txt").unwrap();
    let meditations_final_str = String::from_utf8_lossy(&meditations_final);
    if meditations_final_str.contains("=== END OF TEXT ===") {
        println!("   ✓ meditations_marcus_aurelius.txt: CORRECTLY MODIFIED");
    } else {
        println!("   ✗ meditations_marcus_aurelius.txt: MODIFICATION LOST");
        all_pass = false;
    }

    // Check new file
    println!();
    println!("   New file (added incrementally):");
    match fs.read_file("stoic_wisdom_notes.txt") {
        Ok((data, _)) => {
            let content = String::from_utf8_lossy(&data);
            if content.contains("STOIC WISDOM COLLECTION") {
                println!("   ✓ stoic_wisdom_notes.txt: CORRECTLY ADDED");
            } else {
                println!("   ✗ stoic_wisdom_notes.txt: CONTENT WRONG");
                all_pass = false;
            }
        }
        Err(e) => {
            println!("   ✗ stoic_wisdom_notes.txt: NOT FOUND ({})", e);
            all_pass = false;
        }
    }

    println!();
    if all_pass {
        println!("=== ALL EBOOK MODIFICATION TESTS PASSED ===");
    } else {
        println!("=== SOME TESTS FAILED ===");
        std::process::exit(1);
    }

    println!();
    println!("Summary:");
    println!("• 5 files remained unchanged and verified bit-perfect");
    println!("• 2 files were modified in-place with optimistic locking");
    println!("• 1 new file was added via incremental bundle");
    println!("• All operations completed without rebuilding the engram");
}
