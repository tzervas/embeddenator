//! Test to find where the overlap is created

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

#[test]
fn test_encode_block_directly() {
    let config = ReversibleVSAConfig::default();
    let data: Vec<u8> = (0..23).map(|i| (i * 7 + 13) as u8).collect();

    println!("Data: {:?}", &data[..10]);

    // Encode and check each step
    let vec = SparseVec::encode_data(&data, &config, None);

    println!("\nFinal vector:");
    println!("  pos.len: {}, neg.len: {}", vec.pos.len(), vec.neg.len());

    // Check for overlaps
    let mut pos_set = std::collections::HashSet::new();
    let mut neg_set = std::collections::HashSet::new();

    for &idx in &vec.pos {
        pos_set.insert(idx);
    }

    for &idx in &vec.neg {
        if pos_set.contains(&idx) {
            println!("  OVERLAP FOUND: index {} is in both pos and neg!", idx);
        }
        neg_set.insert(idx);
    }

    let overlap_count = vec.pos.iter().filter(|idx| neg_set.contains(idx)).count();
    println!("  Total overlaps: {}", overlap_count);

    if overlap_count > 0 {
        println!("\n  Overlapping indices:");
        for &idx in &vec.pos {
            if neg_set.contains(&idx) {
                println!("    {}", idx);
            }
        }
    }

    // Manually encode a single block to see if it has overlap
    println!("\n\nTesting single block encoding:");
    let single_block = &data[..8.min(data.len())];
    println!("Block data: {:?}", single_block);

    // Simulate encode_block logic
    let mut pos_manual = Vec::new();
    let mut neg_manual = Vec::new();
    let shift = 0;
    let dim = embeddenator_vsa::DIM;

    for (i, &byte) in single_block.iter().enumerate() {
        let base_idx = (i + shift) % dim;
        if byte & 0x80 != 0 {
            let idx = (base_idx + (byte & 0x7F) as usize) % dim;
            neg_manual.push(idx);
            println!("  byte[{}] = {} (0x{:02x}) -> neg[{}]", i, byte, byte, idx);
        } else {
            let idx = (base_idx + byte as usize) % dim;
            pos_manual.push(idx);
            println!("  byte[{}] = {} (0x{:02x}) -> pos[{}]", i, byte, byte, idx);
        }
    }

    pos_manual.sort_unstable();
    pos_manual.dedup();
    neg_manual.sort_unstable();
    neg_manual.dedup();

    println!(
        "Manual block: pos.len = {}, neg.len = {}",
        pos_manual.len(),
        neg_manual.len()
    );

    // Check for overlaps in this single block
    let overlap_single: Vec<_> = pos_manual
        .iter()
        .filter(|idx| neg_manual.contains(idx))
        .copied()
        .collect();
    println!("Single block overlaps: {}", overlap_single.len());
    if !overlap_single.is_empty() {
        println!("  Overlapping indices: {:?}", overlap_single);
    }

    // Assert no overlaps in encoded vector (the bug fix validation)
    assert_eq!(
        overlap_count, 0,
        "pos and neg should be disjoint after encoding"
    );
}
