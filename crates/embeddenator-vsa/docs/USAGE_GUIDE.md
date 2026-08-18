# Embeddenator-VSA Usage Guide

**Version**: 0.21.0-beta.1

---

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
embeddenator-vsa = { git = "https://github.com/tzervas/embeddenator-vsa" }
```

### Basic Example

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

fn main() {
    // Create configuration
    let config = ReversibleVSAConfig::default();
    
    // Encode data to hypervectors
    let vec1 = SparseVec::encode_data(b"hello", &config, None);
    let vec2 = SparseVec::encode_data(b"world", &config, None);
    
    // Compute similarity
    let similarity = vec1.cosine(&vec2);
    println!("Similarity between 'hello' and 'world': {:.3}", similarity);
    
    // Bundle (superposition)
    let bundled = vec1.bundle(&vec2);
    println!("Bundled vector similarity to 'hello': {:.3}", bundled.cosine(&vec1));
    
    // Bind (association)
    let bound = vec1.bind(&vec2);
    println!("Bound vector similarity to 'hello': {:.3}", bound.cosine(&vec1));
}
```

---

## Core Operations

### Creating Vectors

#### From Data (Reversible Encoding)

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();

// Basic encoding
let vec = SparseVec::encode_data(b"some data", &config, None);

// With hierarchical path
let vec_with_path = SparseVec::encode_data(b"some data", &config, Some("path/to/item"));

// Different configs for different use cases
let small_block_config = ReversibleVSAConfig::small_blocks();
let large_block_config = ReversibleVSAConfig::large_blocks();
```

#### Random Vectors

```rust
use embeddenator_vsa::SparseVec;

// Generate random sparse vector (~1% density)
let random_vec = SparseVec::random();
```

#### Manual Construction

```rust
use embeddenator_vsa::SparseVec;

// Create specific sparse vector
let vec = SparseVec {
    pos: vec![100, 500, 1000],  // Indices with +1
    neg: vec![200, 600, 2000],  // Indices with -1
};
```

### Bundle Operation (Superposition)

Bundle creates a superposition of vectors - the result is similar to all inputs.

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let apple = SparseVec::encode_data(b"apple", &config, None);
let orange = SparseVec::encode_data(b"orange", &config, None);
let banana = SparseVec::encode_data(b"banana", &config, None);

// Pairwise bundle
let fruits = apple.bundle(&orange);

// Bundle with sparsity control
let controlled = apple.bundle_with_config(&orange, Some(&config));

// Associative bundle for multiple vectors
let all_fruits = SparseVec::bundle_sum_many(&[&apple, &orange, &banana]);

// Query - bundled vector is similar to all components
println!("fruits ~ apple: {:.3}", fruits.cosine(&apple));   // ~0.5-0.7
println!("fruits ~ orange: {:.3}", fruits.cosine(&orange)); // ~0.5-0.7
```

### Bind Operation (Association)

Bind creates associations between concepts. Key property: `A ⊙ A ≈ Identity` (self-inverse).

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let color = SparseVec::encode_data(b"color", &config, None);
let red = SparseVec::encode_data(b"red", &config, None);

// Create association: "color is red"
let color_is_red = color.bind(&red);

// Query: "what is the color?" (uses self-inverse property)
let what_color = color_is_red.bind(&color);
println!("Query result ~ red: {:.3}", what_color.cosine(&red));  // High similarity

// Bound vectors are dissimilar to inputs
println!("bound ~ color: {:.3}", color_is_red.cosine(&color));  // Low similarity
println!("bound ~ red: {:.3}", color_is_red.cosine(&red));      // Low similarity
```

### Similarity (Cosine)

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let vec1 = SparseVec::encode_data(b"cat", &config, None);
let vec2 = SparseVec::encode_data(b"cat", &config, None);  // Same data
let vec3 = SparseVec::encode_data(b"dog", &config, None);  // Different data

// Identical data → high similarity
assert!((vec1.cosine(&vec2) - 1.0).abs() < 0.01);

// Different data → low similarity
let sim = vec1.cosine(&vec3);
assert!(sim < 0.3);

// Scalar version (always available)
let sim_scalar = vec1.cosine_scalar(&vec3);
```

### Permutation (Sequence Encoding)

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let word = SparseVec::encode_data(b"word", &config, None);

// Encode position in sequence
let position_1 = word.permute(0);
let position_2 = word.permute(100);
let position_3 = word.permute(200);

// Positions are dissimilar
println!("pos1 ~ pos2: {:.3}", position_1.cosine(&position_2)); // Low

// Reversible
let recovered = position_2.inverse_permute(100);
println!("recovered ~ original: {:.3}", recovered.cosine(&word)); // ~1.0
```

---

## Data Encoding and Decoding

### Reversible Encoding

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let original_data = b"Hello, World!";

// Encode
let encoded = SparseVec::encode_data(original_data, &config, None);

// Decode
let decoded = encoded.decode_data(&config, None, original_data.len());

// Note: Raw decode may have minor differences
// For 100% fidelity, use Codebook with residuals
```

### Codebook-Based Encoding

```rust
use embeddenator_vsa::{Codebook, ProjectionConfig};

// Create and initialize codebook
let mut codebook = Codebook::new(10000);
codebook.initialize_standard_basis();

let data = b"the quick brown fox";

// Project data onto codebook
let projection = codebook.project(data);
println!("Quality score: {:.2}", projection.quality_score);
println!("Coefficients: {}", projection.coefficients.len());
println!("Residual bytes: {}", projection.residual.len());

// Reconstruct from projection
let reconstructed = codebook.reconstruct(&projection, data.len());
```

### Custom Projection Config

```rust
use embeddenator_vsa::{Codebook, ProjectionConfig};

let mut codebook = Codebook::new(10000);
codebook.initialize_standard_basis();

let config = ProjectionConfig {
    chunk_size: 32,          // Smaller chunks
    similarity_threshold: 0.2, // Lower threshold (more matches)
    max_basis_matches: 8,    // More basis vectors per chunk
    coefficient_scale: 500.0,
    coefficient_key_spacing: 500,
};

let projection = codebook.project_with_config(b"custom config test", &config);
```

---

## Configuration

### ReversibleVSAConfig

```rust
use embeddenator_vsa::ReversibleVSAConfig;

// Default config (balanced)
let default_config = ReversibleVSAConfig::default();
// block_size: 256, max_path_depth: 10, base_shift: 1000, target_sparsity: 200

// Small blocks (good for small data)
let small_config = ReversibleVSAConfig::small_blocks();
// block_size: 64, max_path_depth: 5, base_shift: 500, target_sparsity: 100

// Large blocks (good for large data)
let large_config = ReversibleVSAConfig::large_blocks();
// block_size: 1024, max_path_depth: 20, base_shift: 2000, target_sparsity: 400

// Custom config
let custom_config = ReversibleVSAConfig {
    block_size: 128,
    max_path_depth: 8,
    base_shift: 800,
    target_sparsity: 150,
};
```

### ProjectionConfig

```rust
use embeddenator_vsa::ProjectionConfig;

let config = ProjectionConfig {
    chunk_size: 64,           // Bytes per chunk
    similarity_threshold: 0.3, // Minimum similarity to include basis
    max_basis_matches: 4,     // Max basis vectors per chunk
    coefficient_scale: 1000.0, // Scale for encoding similarity
    coefficient_key_spacing: 1000,
};
```

---

## Common Patterns

### Sequence Encoding

Encode ordered sequences using permutation:

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();

// Encode sequence: ["the", "cat", "sat"]
let the = SparseVec::encode_data(b"the", &config, None);
let cat = SparseVec::encode_data(b"cat", &config, None);
let sat = SparseVec::encode_data(b"sat", &config, None);

// Position encoding with permutation
let shift = 100;  // Shift per position
let pos0 = the.permute(0 * shift);
let pos1 = cat.permute(1 * shift);
let pos2 = sat.permute(2 * shift);

// Bundle the positioned elements
let sequence = SparseVec::bundle_sum_many(&[&pos0, &pos1, &pos2]);

// Query position 1 - "what's at position 1?"
let query_result = sequence.inverse_permute(1 * shift);
println!("Position 1 ~ cat: {:.3}", query_result.cosine(&cat)); // High
println!("Position 1 ~ the: {:.3}", query_result.cosine(&the)); // Lower
```

### Role-Filler Structures

Create structured representations:

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();

// Define roles
let agent = SparseVec::encode_data(b"AGENT", &config, None);
let action = SparseVec::encode_data(b"ACTION", &config, None);
let patient = SparseVec::encode_data(b"PATIENT", &config, None);

// Define fillers
let cat = SparseVec::encode_data(b"cat", &config, None);
let chase = SparseVec::encode_data(b"chase", &config, None);
let mouse = SparseVec::encode_data(b"mouse", &config, None);

// Create bindings
let agent_cat = agent.bind(&cat);
let action_chase = action.bind(&chase);
let patient_mouse = patient.bind(&mouse);

// Bundle into complete structure
let sentence = SparseVec::bundle_sum_many(&[&agent_cat, &action_chase, &patient_mouse]);

// Query: "Who is the agent?"
let who = sentence.bind(&agent);
println!("Agent ~ cat: {:.3}", who.cosine(&cat));   // High
println!("Agent ~ mouse: {:.3}", who.cosine(&mouse)); // Low
```

### Similarity Search

Find the most similar item in a collection:

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();

// Build a vocabulary
let vocab: Vec<(&str, SparseVec)> = vec![
    ("apple", SparseVec::encode_data(b"apple", &config, None)),
    ("orange", SparseVec::encode_data(b"orange", &config, None)),
    ("banana", SparseVec::encode_data(b"banana", &config, None)),
    ("grape", SparseVec::encode_data(b"grape", &config, None)),
];

// Query vector
let query = SparseVec::encode_data(b"apple", &config, None);

// Find most similar
let mut best_match = ("", -1.0f64);
for (name, vec) in &vocab {
    let sim = query.cosine(vec);
    if sim > best_match.1 {
        best_match = (name, sim);
    }
}

println!("Best match: {} (similarity: {:.3})", best_match.0, best_match.1);
```

---

## Performance Tips

### Use Appropriate Representation

```rust
use embeddenator_vsa::{SparseVec, PackedTritVec};

// Sparse vectors: efficient for < 25% density
let sparse = SparseVec::random();

// Dense operations may benefit from PackedTritVec
// The library handles this automatically in many cases
```

### Batch Operations

```rust
use embeddenator_vsa::SparseVec;

// For bundling many vectors, use bundle_sum_many (associative)
let vectors: Vec<SparseVec> = vec![/* ... */];
let bundled = SparseVec::bundle_sum_many(vectors.iter());

// This is more efficient than sequential pairwise bundling
```

### Thinning for Sparsity Control

```rust
use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};

let config = ReversibleVSAConfig::default();
let v1 = SparseVec::encode_data(b"word1", &config, None);
let v2 = SparseVec::encode_data(b"word2", &config, None);

// Bundle with automatic thinning
let bundled = v1.bundle_with_config(&v2, Some(&config));

// Or manual thinning
let dense_vec = v1.bundle(&v2);
let thinned = dense_vec.thin(200);  // Target 200 non-zero elements
```

---

## Feature Flags

### `simd` (Default: disabled)

Enables SIMD-accelerated operations. Currently uses optimized scalar fallbacks.

```toml
[dependencies]
embeddenator-vsa = { git = "...", features = ["simd"] }
```

### `block-sparse` (Experimental)

Enables block-sparse operations for very large vectors.

```toml
[dependencies]
embeddenator-vsa = { git = "...", features = ["block-sparse"] }
```

---

## Error Handling

The library uses a unified error type `VsaError`:

```rust
use embeddenator_vsa::{BalancedTernaryWord, WordMetadata, VsaError};

// Range checking
let result = BalancedTernaryWord::new(i64::MAX, WordMetadata::Data);
match result {
    Ok(word) => println!("Created word"),
    Err(VsaError::ValueOutOfRange { value, min, max }) => {
        println!("Value {} outside range [{}, {}]", value, min, max);
    }
    Err(e) => println!("Error: {}", e),
}
```

---

## Next Steps

- See [Architecture Guide](ARCHITECTURE.md) for design details
- See [Bitsliced Ternary Design](BITSLICED_TERNARY_DESIGN.md) for internal representation
- See [SIMD Decision](SIMD_DECISION.md) for performance optimization status

---

**Document Version**: 1.0
**Last Updated**: 2026-01-14
