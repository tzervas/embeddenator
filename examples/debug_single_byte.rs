use embeddenator::{EmbrFS, ReversibleVSAConfig, SparseVec};
use std::fs;

fn main() {
    let temp_dir = std::env::temp_dir().join("embr_test");
    let input_dir = temp_dir.join("input");
    let output_dir = temp_dir.join("output");
    
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();
    
    // Test with byte 0
    let byte_val: u8 = 0;
    let test_path = input_dir.join("byte_0.bin");
    fs::write(&test_path, &[byte_val]).unwrap();
    
    let mut embrfs = EmbrFS::new();
    let config = ReversibleVSAConfig::default();
    
    println!("=== Testing encode/decode directly ===");
    let data = vec![0u8];
    let encoded = SparseVec::encode_data(&data, &config, Some("test.bin"));
    println!("Encoded pos: {:?}", &encoded.pos[..encoded.pos.len().min(10)]);
    println!("Encoded neg: {:?}", &encoded.neg[..encoded.neg.len().min(10)]);
    println!("Encoded pos len: {}, neg len: {}", encoded.pos.len(), encoded.neg.len());
    
    let decoded = encoded.decode_data(&config, Some("test.bin"), 1);
    println!("Decoded: {:?}", decoded);
    
    println!("\n=== Testing via EmbrFS ===");
    println!("Ingesting single byte: {}", byte_val);
    embrfs.ingest_file(&test_path, "byte_0.bin".to_string(), true, &config).unwrap();
    
    println!("Manifest files: {:?}", embrfs.manifest.files);
    println!("Engram codebook keys: {:?}", embrfs.engram.codebook.keys().collect::<Vec<_>>());
    
    // Check the actual vector in the codebook
    if let Some(chunk_vec) = embrfs.engram.codebook.get(&0) {
        println!("Chunk 0 pos len: {}, neg len: {}", chunk_vec.pos.len(), chunk_vec.neg.len());
    }
    
    println!("Correction stats: {:?}", embrfs.correction_stats());
    
    println!("\nExtracting...");
    EmbrFS::extract(&embrfs.engram, &embrfs.manifest, &output_dir, true, &config).unwrap();
    
    let reconstructed = fs::read(output_dir.join("byte_0.bin")).unwrap();
    println!("Reconstructed: {:?}", reconstructed);
    
    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}
