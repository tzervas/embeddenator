# Retrieval Index (Semantic Search)

This document defines the project’s first *robust, elegant* semantic retrieval index for sparse ternary VSA vectors.

## Why cosine similarity is not enough
Cosine similarity is a scoring function. On its own, it requires comparing the query to **every** stored vector (linear scan).

To scale, we separate retrieval into:
1. **Candidate generation** (sub-linear indexing)
2. **Exact reranking** (cosine / dot on a small candidate set)

## Current implementation
- Module: `src/retrieval.rs`
- Type: `TernaryInvertedIndex`

### Data structure
For each dimension $d \in [0,\mathrm{DIM})$:
- `pos_postings[d]`: IDs with $+1$ at dimension $d$
- `neg_postings[d]`: IDs with $-1$ at dimension $d$

### Query scoring
For a query vector $q$:
- Iterate postings for every $d \in q.pos$ and $d \in q.neg$
- Accumulate sparse ternary dot contributions into integer scores
- Return top-$k$ by score

This yields candidate generation cost proportional to postings touched, not total corpus size.

## Next steps (planned)
- Rerank stage:
  - Use exact cosine similarity on candidates (`SparseVec::cosine`) after candidate generation.
- Add optional signatures (ternary LSH / multi-probe) for further speedups.
- Integrate with EmbrFS:
  - Index `Engram.codebook` and/or hierarchical sub-engrams.

