//! Trace full 23-byte encoding

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

#[test]
fn test_full_23_byte_trace() {
    let config = ReversibleVSAConfig::default();
    let data: Vec<u8> = (0..23).map(|i| (i * 7 + 13) as u8).collect();

    println!("Full data (23 bytes): {:?}", data);

    // Manually encode all bytes
    let mut pos_manual = Vec::new();
    let mut neg_manual = Vec::new();
    let shift = 0;
    let dim = embeddenator_vsa::DIM;

    for (i, &byte) in data.iter().enumerate() {
        let base_idx = (i + shift) % dim;
        if byte & 0x80 != 0 {
            let idx = (base_idx + (byte & 0x7F) as usize) % dim;
            neg_manual.push(idx);
            println!(
                "byte[{}] = {} (0x{:02x}, MSB=1) -> neg[{}]",
                i, byte, byte, idx
            );
        } else {
            let idx = (base_idx + byte as usize) % dim;
            pos_manual.push(idx);
            println!(
                "byte[{}] = {} (0x{:02x}, MSB=0) -> pos[{}]",
                i, byte, byte, idx
            );
        }
    }

    pos_manual.sort_unstable();
    pos_manual.dedup();
    neg_manual.sort_unstable();
    neg_manual.dedup();

    println!("\nManual encoding result:");
    println!("  pos.len = {}", pos_manual.len());
    println!("  neg.len = {}", neg_manual.len());
    println!("  pos: {:?}", pos_manual);
    println!("  neg: {:?}", neg_manual);

    // Check for overlaps
    let overlap: Vec<_> = pos_manual
        .iter()
        .filter(|idx| neg_manual.contains(idx))
        .copied()
        .collect();
    println!("\nOverlaps in manual encoding: {}", overlap.len());
    if !overlap.is_empty() {
        println!("  Overlapping indices: {:?}", overlap);
    }

    // Now encode using the actual function
    let vec = SparseVec::encode_data(&data, &config, None);
    println!("\nActual encoding result:");
    println!("  pos.len = {}", vec.pos.len());
    println!("  neg.len = {}", vec.neg.len());
    println!("  pos: {:?}", vec.pos);
    println!("  neg: {:?}", vec.neg);

    // Check for overlaps
    let overlap_actual: Vec<_> = vec
        .pos
        .iter()
        .filter(|idx| vec.neg.contains(idx))
        .copied()
        .collect();
    println!("\nOverlaps in actual encoding: {}", overlap_actual.len());
    if !overlap_actual.is_empty() {
        println!("  Overlapping indices: {:?}", overlap_actual);
    }

    // Compare manual vs actual
    // Note: Manual encoding simulates naive behavior without overlap cleanup
    // Actual encoding removes overlaps (the bug fix), so they won't match
    println!("\nComparison:");
    println!("  pos match: {}", pos_manual == vec.pos);
    println!("  neg match: {}", neg_manual == vec.neg);

    // Assert no overlaps in encoded vector (the bug fix validation)
    assert_eq!(
        overlap_actual.len(),
        0,
        "pos and neg should be disjoint after encoding"
    );

    // Verify the actual encoding has reasonable characteristics
    assert!(
        vec.pos.len() + vec.neg.len() > 0,
        "Encoded vector should have non-zero entries"
    );

    // Verify indices are within bounds
    let dim = embeddenator_vsa::DIM;
    for &idx in &vec.pos {
        assert!(idx < dim, "pos index should be < DIM");
    }
    for &idx in &vec.neg {
        assert!(idx < dim, "neg index should be < DIM");
    }
}
