//! Encoding and Decoding Example
//!
//! Demonstrates data encoding patterns:
//! - Reversible VSA encoding
//! - Codebook-based projection
//! - Quality scoring

use embeddenator_vsa::{Codebook, ProjectionConfig, ReversibleVSAConfig, SparseVec};

fn main() {
    println!("=== Embeddenator-VSA: Encoding & Decoding ===\n");

    // -------------------------------------------------------------------------
    // 1. Basic Reversible Encoding
    // -------------------------------------------------------------------------
    println!("1. Basic VSA Encoding...\n");

    let config = ReversibleVSAConfig::default();
    let original = b"Hello, World!";

    println!("   Original: {:?}", String::from_utf8_lossy(original));
    println!("   Length: {} bytes", original.len());

    // Encode to hypervector
    let encoded = SparseVec::encode_data(original, &config, None);
    println!(
        "   Encoded: {} positive + {} negative indices",
        encoded.pos.len(),
        encoded.neg.len()
    );

    // Decode back
    let decoded = encoded.decode_data(&config, None, original.len());
    println!("   Decoded: {:?}", String::from_utf8_lossy(&decoded));

    // -------------------------------------------------------------------------
    // 2. Encoding with Different Configs
    // -------------------------------------------------------------------------
    println!("\n2. Different Configuration Profiles...\n");

    let data = b"The quick brown fox jumps over the lazy dog";

    // Small block config (for small data)
    let small_config = ReversibleVSAConfig::small_blocks();
    let encoded_small = SparseVec::encode_data(data, &small_config, None);
    println!(
        "   Small blocks: {} non-zero elements",
        encoded_small.pos.len() + encoded_small.neg.len()
    );

    // Large block config (for large data)
    let large_config = ReversibleVSAConfig::large_blocks();
    let encoded_large = SparseVec::encode_data(data, &large_config, None);
    println!(
        "   Large blocks: {} non-zero elements",
        encoded_large.pos.len() + encoded_large.neg.len()
    );

    // -------------------------------------------------------------------------
    // 3. Hierarchical Path Encoding
    // -------------------------------------------------------------------------
    println!("\n3. Hierarchical Path Encoding...\n");

    let file_data = b"file contents here";

    // Encode with path (for hierarchical storage)
    let path = "/documents/notes/meeting.txt";
    let encoded_with_path = SparseVec::encode_data(file_data, &config, Some(path));

    // Same data with different path produces different vector
    let other_path = "/documents/notes/todo.txt";
    let encoded_other_path = SparseVec::encode_data(file_data, &config, Some(other_path));

    let sim = encoded_with_path.cosine(&encoded_other_path);
    println!("   Path 1: {}", path);
    println!("   Path 2: {}", other_path);
    println!("   Similarity (different paths): {:.3}", sim);

    // Same path produces identical encoding
    let encoded_same_path = SparseVec::encode_data(file_data, &config, Some(path));
    let sim_same = encoded_with_path.cosine(&encoded_same_path);
    println!("   Similarity (same path): {:.3}", sim_same);

    // -------------------------------------------------------------------------
    // 4. Codebook-Based Encoding
    // -------------------------------------------------------------------------
    println!("\n4. Codebook-Based Projection...\n");

    // Create and initialize codebook
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();
    println!(
        "   Initialized codebook with {} basis vectors",
        codebook.basis_vectors.len()
    );

    // Project data onto codebook
    let text_data = b"the quick brown fox jumps over the lazy dog";
    let projection = codebook.project(text_data);

    println!("\n   Projection results:");
    println!(
        "   - Quality score: {:.2}%",
        projection.quality_score * 100.0
    );
    println!("   - Coefficients: {}", projection.coefficients.len());
    println!("   - Residual words: {}", projection.residual.len());
    println!("   - Outliers detected: {}", projection.outliers.len());

    // Reconstruct from projection
    let reconstructed = codebook.reconstruct(&projection, text_data.len());
    println!(
        "\n   Original:      {:?}",
        String::from_utf8_lossy(text_data)
    );
    println!(
        "   Reconstructed: {:?}",
        String::from_utf8_lossy(&reconstructed)
    );

    // -------------------------------------------------------------------------
    // 5. Custom Projection Configuration
    // -------------------------------------------------------------------------
    println!("\n5. Custom Projection Configuration...\n");

    let custom_config = ProjectionConfig {
        chunk_size: 32,            // Smaller chunks
        similarity_threshold: 0.2, // Lower threshold (more matches)
        max_basis_matches: 8,      // More basis vectors
        coefficient_scale: 500.0,
        coefficient_key_spacing: 500,
    };

    let custom_projection = codebook.project_with_config(text_data, &custom_config);
    println!("   Custom config projection:");
    println!(
        "   - Quality score: {:.2}%",
        custom_projection.quality_score * 100.0
    );
    println!(
        "   - Coefficients: {} (more with lower threshold)",
        custom_projection.coefficients.len()
    );

    // -------------------------------------------------------------------------
    // 6. Different Data Types
    // -------------------------------------------------------------------------
    println!("\n6. Encoding Different Data Types...\n");

    // Text data
    let text = b"Natural language text content";
    let text_proj = codebook.project(text);
    println!(
        "   Text data quality: {:.2}%",
        text_proj.quality_score * 100.0
    );

    // Binary data (zeros)
    let binary: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0];
    let binary_proj = codebook.project(&binary);
    println!(
        "   Binary (zeros) quality: {:.2}%",
        binary_proj.quality_score * 100.0
    );

    // Random-looking data
    let random: Vec<u8> = (0..64).map(|i| (i * 7 + 13) as u8).collect();
    let random_proj = codebook.project(&random);
    println!(
        "   Pseudo-random quality: {:.2}%",
        random_proj.quality_score * 100.0
    );

    // -------------------------------------------------------------------------
    // Summary
    // -------------------------------------------------------------------------
    println!("\n=== Summary ===");
    println!("  • encode_data(): Creates reversible hypervector from data");
    println!("  • decode_data(): Recovers data from hypervector");
    println!("  • Path encoding: Adds hierarchical context");
    println!("  • Codebook: Provides basis vectors for differential encoding");
    println!("  • Quality score: Measures how well basis captures data");
}
