//! Similarity Search Example
//!
//! Demonstrates using VSA for similarity-based retrieval:
//! - Building a vocabulary
//! - Finding similar items
//! - Fuzzy matching

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

fn main() {
    println!("=== Embeddenator-VSA: Similarity Search ===\n");

    let config = ReversibleVSAConfig::default();

    // -------------------------------------------------------------------------
    // 1. Building a Vocabulary
    // -------------------------------------------------------------------------
    println!("1. Building vocabulary...\n");

    let words = vec![
        "apple",
        "application",
        "apply",
        "banana",
        "band",
        "bandana",
        "cherry",
        "computer",
        "computing",
        "orange",
        "organ",
        "organic",
    ];

    // Encode all words
    let vocab: Vec<(&str, SparseVec)> = words
        .iter()
        .map(|&word| (word, SparseVec::encode_data(word.as_bytes(), &config, None)))
        .collect();

    println!("   Created vocabulary with {} words", vocab.len());

    // -------------------------------------------------------------------------
    // 2. Finding Similar Words
    // -------------------------------------------------------------------------
    println!("\n2. Finding similar words...\n");

    let find_similar = |query_word: &str, top_k: usize| {
        let query = SparseVec::encode_data(query_word.as_bytes(), &config, None);

        // Calculate similarities
        let mut similarities: Vec<(&&str, f64)> = vocab
            .iter()
            .map(|(word, vec)| (word, query.cosine(vec)))
            .collect();

        // Sort by similarity (descending)
        // NaN values are treated as less than any valid similarity, effectively
        // pushing them to the end of the list. This handles edge cases where
        // cosine returns NaN for empty vectors.
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        println!("   Query: \"{}\"", query_word);
        println!("   Top {} matches:", top_k);
        for (word, sim) in similarities.iter().take(top_k) {
            println!("      {:<15} {:.3}", word, sim);
        }
        println!();
    };

    // Find words similar to "apple"
    find_similar("apple", 5);

    // Find words similar to "computer"
    find_similar("computer", 5);

    // Find words similar to "banana"
    find_similar("banana", 5);

    // -------------------------------------------------------------------------
    // 3. Fuzzy Query (Partial Match)
    // -------------------------------------------------------------------------
    println!("3. Fuzzy query (similar but not exact)...\n");

    // Query with slight variation
    find_similar("aple", 3); // Typo: "aple" instead of "apple"
    find_similar("banan", 3); // Partial: "banan" instead of "banana"
    find_similar("computr", 3); // Typo: "computr"

    // -------------------------------------------------------------------------
    // 4. Category Detection using Bundled Vectors
    // -------------------------------------------------------------------------
    println!("4. Category detection with bundled concepts...\n");

    // Create category vectors by bundling related words
    let apple_vec = SparseVec::encode_data(b"apple", &config, None);
    let banana_vec = SparseVec::encode_data(b"banana", &config, None);
    let cherry_vec = SparseVec::encode_data(b"cherry", &config, None);
    let orange_vec = SparseVec::encode_data(b"orange", &config, None);

    // Fruit category = bundle of fruit words
    let fruit_category =
        SparseVec::bundle_sum_many([&apple_vec, &banana_vec, &cherry_vec, &orange_vec]);

    let computer_vec = SparseVec::encode_data(b"computer", &config, None);
    let computing_vec = SparseVec::encode_data(b"computing", &config, None);
    let application_vec = SparseVec::encode_data(b"application", &config, None);

    // Tech category = bundle of tech words
    let tech_category =
        SparseVec::bundle_sum_many([&computer_vec, &computing_vec, &application_vec]);

    // Classify new words
    let classify = |word: &str| {
        let vec = SparseVec::encode_data(word.as_bytes(), &config, None);
        let fruit_sim = vec.cosine(&fruit_category);
        let tech_sim = vec.cosine(&tech_category);

        let category = if fruit_sim > tech_sim {
            "fruit"
        } else {
            "tech"
        };
        println!(
            "   \"{}\": fruit={:.3}, tech={:.3} → {}",
            word, fruit_sim, tech_sim, category
        );
    };

    println!("   Classifying words into categories:");
    classify("apple"); // Could be fruit or tech!
    classify("grape");
    classify("software");
    classify("mango");
    classify("programming");

    // -------------------------------------------------------------------------
    // 5. Similarity Threshold Search
    // -------------------------------------------------------------------------
    println!("\n5. Threshold-based search...\n");

    let threshold = 0.5;
    let query = SparseVec::encode_data(b"organic", &config, None);

    println!("   Query: \"organic\" (threshold: {:.1})", threshold);
    println!("   Words above threshold:");

    let mut found = false;
    for (word, vec) in &vocab {
        let sim = query.cosine(vec);
        if sim > threshold {
            println!("      {}: {:.3}", word, sim);
            found = true;
        }
    }
    if !found {
        println!("      (no matches above threshold)");
    }

    // -------------------------------------------------------------------------
    // 6. Nearest Neighbor with Exclusion
    // -------------------------------------------------------------------------
    println!("\n6. Finding nearest neighbor (excluding self)...\n");

    let find_nearest_excluding_self = |word: &str| {
        let query = SparseVec::encode_data(word.as_bytes(), &config, None);

        let mut best_match: Option<(&str, f64)> = None;
        for (w, vec) in &vocab {
            if *w == word {
                continue; // Skip self
            }
            let sim = query.cosine(vec);
            if best_match.is_none() || sim > best_match.unwrap().1 {
                best_match = Some((w, sim));
            }
        }

        if let Some((match_word, sim)) = best_match {
            println!(
                "   Nearest to \"{}\": \"{}\" (similarity: {:.3})",
                word, match_word, sim
            );
        }
    };

    find_nearest_excluding_self("apple");
    find_nearest_excluding_self("application");
    find_nearest_excluding_self("band");
    find_nearest_excluding_self("organic");

    // -------------------------------------------------------------------------
    // Summary
    // -------------------------------------------------------------------------
    println!("\n=== Summary ===");
    println!("  • VSA enables similarity-based search without exact matching");
    println!("  • Bundled vectors create category prototypes");
    println!("  • Cosine similarity measures relatedness [-1, 1]");
    println!("  • Useful for fuzzy matching and semantic search");
}
