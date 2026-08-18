# embeddenator-retrieval

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/embeddenator-retrieval/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-retrieval/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/embeddenator-retrieval/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-retrieval/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

Semantic retrieval and search operations for VSA-based vector representations.

**Independent component** extracted from the Embeddenator monolithic repository. Part of the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

**Repository:** [https://github.com/tzervas/embeddenator-retrieval](https://github.com/tzervas/embeddenator-retrieval)

## Features

- **Fast Inverted Indexing**: Sub-linear candidate generation for large-scale search
- **Multiple Similarity Metrics**: Cosine, Hamming, Jaccard, and dot product
- **Search Strategies**: 
  - Exact search (brute force, ground truth)
  - Approximate search (inverted index, fast)
  - Two-stage search (candidate generation + reranking, balanced)
  - Hierarchical search (clustering-based)
- **Index Structures**: Brute force, hierarchical clustering
- **Resonator Networks**: Pattern completion and factorization
- **Algebraic Correction**: Guaranteed bitwise reconstruction
- **VSA Integration**: Native support for sparse ternary vectors

## Status

**** - Full retrieval functionality migrated from monolithic repo.

### Implementation Progress

-  Inverted index (TernaryInvertedIndex)
-  Similarity metrics (Cosine, Hamming, Jaccard, DotProduct)
-  Search strategies (Exact, Approximate, Two-stage, Batch)
-  Index structures (BruteForce, Hierarchical)
-  Resonator networks for pattern completion
-  Algebraic correction layer
-  Comprehensive test suite (similarity, search, index)
-  Performance benchmarks

## Usage

### Basic Retrieval

```rust
use embeddenator_retrieval::{TernaryInvertedIndex, search::two_stage_search, search::SearchConfig};
use embeddenator_vsa::SparseVec;
use std::collections::HashMap;

// Build index
let mut index = TernaryInvertedIndex::new();
let mut vectors = HashMap::new();

let vec1 = SparseVec::from_data(b"document one");
let vec2 = SparseVec::from_data(b"document two");
let vec3 = SparseVec::from_data(b"document three");

index.add(1, &vec1);
index.add(2, &vec2);
index.add(3, &vec3);
index.finalize();

vectors.insert(1, vec1);
vectors.insert(2, vec2);
vectors.insert(3, vec3);

// Search with two-stage retrieval (fast + accurate)
let query = SparseVec::from_data(b"document");
let config = SearchConfig::default();
let results = two_stage_search(&query, &index, &vectors, &config, 5);

for result in results {
    println!("ID: {}, Score: {:.3}, Rank: {}", 
        result.id, result.score, result.rank);
}
```

### Different Similarity Metrics

```rust
use embeddenator_retrieval::similarity::{compute_similarity, SimilarityMetric};
use embeddenator_vsa::SparseVec;

let a = SparseVec::from_data(b"hello");
let b = SparseVec::from_data(b"hello world");

let cosine = compute_similarity(&a, &b, SimilarityMetric::Cosine);
let hamming = compute_similarity(&a, &b, SimilarityMetric::Hamming);
let jaccard = compute_similarity(&a, &b, SimilarityMetric::Jaccard);

println!("Cosine: {:.3}, Hamming: {:.1}, Jaccard: {:.3}", 
    cosine, hamming, jaccard);
```

## Performance

Estimated benchmarks on a modern multi-core CPU (corpus size = 10,000 vectors):

| Strategy | Latency (avg) | Throughput | Recall@10 |
|----------|---------------|------------|-----------|
| Approximate | ~0.5ms | ~2000 QPS | ~0.85 |
| Two-stage (candidate_k=200) | ~2ms | ~500 QPS | ~0.98 |
| Exact | ~15ms | ~66 QPS | 1.00 |

> **Note**: Actual performance varies significantly based on hardware, vector dimensionality, data distribution, and query patterns. Run benchmarks on your system for accurate numbers:

Run benchmarks:
```bash
cargo bench --manifest-path embeddenator-retrieval/Cargo.toml
```

## Testing

```bash
# Run all tests
cargo test --manifest-path embeddenator-retrieval/Cargo.toml --all-features

# Run specific test suite
cargo test --manifest-path embeddenator-retrieval/Cargo.toml similarity_tests
cargo test --manifest-path embeddenator-retrieval/Cargo.toml search_tests

# Run with output
cargo test --manifest-path embeddenator-retrieval/Cargo.toml -- --nocapture
```

## Development

```bash
# Build
cargo build --manifest-path embeddenator-retrieval/Cargo.toml

# Local development with other Embeddenator components
# Add to workspace Cargo.toml:
[patch."https://github.com/tzervas/embeddenator-retrieval"]
embeddenator-retrieval = { path = "../embeddenator-retrieval" }
```

## Integration with Other Components

- **embeddenator-vsa**: Uses `SparseVec` for all vector operations
- **embeddenator-obs**: Provides observability for search operations
- **embeddenator-fs**: Uses retrieval for file similarity search

## Architecture

See [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md) for component decomposition rationale.

## License

MIT

