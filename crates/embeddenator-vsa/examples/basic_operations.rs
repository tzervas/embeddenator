//! Basic VSA Operations Example
//!
//! Demonstrates core Vector Symbolic Architecture operations:
//! - Creating vectors from data
//! - Bundle (superposition)
//! - Bind (association)
//! - Similarity computation

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

fn main() {
    println!("=== Embeddenator-VSA: Basic Operations ===\n");

    // Create configuration
    let config = ReversibleVSAConfig::default();

    // -------------------------------------------------------------------------
    // 1. Creating Vectors
    // -------------------------------------------------------------------------
    println!("1. Creating hypervectors from data...\n");

    let apple = SparseVec::encode_data(b"apple", &config, None);
    let orange = SparseVec::encode_data(b"orange", &config, None);
    let fruit = SparseVec::encode_data(b"fruit", &config, None);

    println!("   Created vectors for: apple, orange, fruit");
    println!(
        "   apple: {} positive, {} negative indices",
        apple.pos.len(),
        apple.neg.len()
    );

    // -------------------------------------------------------------------------
    // 2. Similarity
    // -------------------------------------------------------------------------
    println!("\n2. Computing similarity...\n");

    // Same data produces identical vectors
    let apple2 = SparseVec::encode_data(b"apple", &config, None);
    let same_sim = apple.cosine(&apple2);
    println!("   apple ~ apple:  {:.3} (same data)", same_sim);

    // Different data produces low similarity
    let diff_sim = apple.cosine(&orange);
    println!("   apple ~ orange: {:.3} (different data)", diff_sim);

    // -------------------------------------------------------------------------
    // 3. Bundle (Superposition)
    // -------------------------------------------------------------------------
    println!("\n3. Bundling (creating sets)...\n");

    // Bundle creates a "set" containing both
    let fruits = apple.bundle(&orange);
    println!("   Created: fruits = apple ⊕ orange");

    // Bundled vector is similar to both components
    let sim_apple = fruits.cosine(&apple);
    let sim_orange = fruits.cosine(&orange);
    println!("   fruits ~ apple:  {:.3}", sim_apple);
    println!("   fruits ~ orange: {:.3}", sim_orange);

    // Bundle more items
    let banana = SparseVec::encode_data(b"banana", &config, None);
    let all_fruits = SparseVec::bundle_sum_many([&apple, &orange, &banana]);
    println!("\n   Created: all_fruits = apple ⊕ orange ⊕ banana");
    println!("   all_fruits ~ banana: {:.3}", all_fruits.cosine(&banana));

    // -------------------------------------------------------------------------
    // 4. Bind (Association)
    // -------------------------------------------------------------------------
    println!("\n4. Binding (creating associations)...\n");

    // Bind creates an association between concepts
    let apple_is_fruit = apple.bind(&fruit);
    println!("   Created: apple ⊙ fruit (apple IS-A fruit)");

    // Bound vector is dissimilar to both inputs
    let sim_to_apple = apple_is_fruit.cosine(&apple);
    let sim_to_fruit = apple_is_fruit.cosine(&fruit);
    println!("   (apple ⊙ fruit) ~ apple: {:.3} (low)", sim_to_apple);
    println!("   (apple ⊙ fruit) ~ fruit: {:.3} (low)", sim_to_fruit);

    // Self-inverse property: unbinding recovers the other element
    println!("\n   Using self-inverse property to query:");
    let what_is_apple = apple_is_fruit.bind(&apple);
    let sim = what_is_apple.cosine(&fruit);
    println!("   Query: (apple ⊙ fruit) ⊙ apple ~ fruit: {:.3}", sim);

    // -------------------------------------------------------------------------
    // 5. Permutation (Sequence Encoding)
    // -------------------------------------------------------------------------
    println!("\n5. Permutation (encoding position)...\n");

    let word = SparseVec::encode_data(b"word", &config, None);
    let pos1 = word.permute(100);
    let pos2 = word.permute(200);

    println!("   Created: word at position 1 and position 2");
    println!("   pos1 ~ pos2: {:.3} (dissimilar)", pos1.cosine(&pos2));

    // Inverse permutation recovers original
    let recovered = pos1.inverse_permute(100);
    println!(
        "   inverse_permute(pos1) ~ word: {:.3} (recovered)",
        recovered.cosine(&word)
    );

    // -------------------------------------------------------------------------
    // Summary
    // -------------------------------------------------------------------------
    println!("\n=== Summary ===");
    println!("  • Bundle (⊕): Creates superposition, similar to all inputs");
    println!("  • Bind (⊙): Creates association, self-inverse property");
    println!("  • Permute: Encodes position, reversible");
    println!("  • Cosine: Measures similarity [-1, 1]");
}
