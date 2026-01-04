//! Kernel ↔ VSA interop layer (non-FUSE).
//!
//! This module defines a minimal abstraction boundary between a kernel/runtime
//! and the VSA substrate. The goal is to keep non-FUSE builds first-class by
//! avoiding any dependency on the FUSE shim while still enabling:
//! - reversible encode/decode for byte payloads
//! - algebraic VSA ops (bundle, bind, similarity)
//! - a retrieval seam (candidate generation + optional rerank)

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
use std::collections::HashMap;
use std::fmt;

/// Errors from kernel↔VSA interop helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelInteropError {
    MissingVector { id: usize },
}

impl fmt::Display for KernelInteropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelInteropError::MissingVector { id } => {
                write!(f, "missing vector for id {id}")
            }
        }
    }
}

impl std::error::Error for KernelInteropError {}

/// Backend-agnostic VSA operations exposed to a kernel/runtime.
///
/// This is intentionally small: it captures the stable semantic contract while
/// allowing internal representation changes (e.g., migration to ternary-native
/// packed vectors).
pub trait VsaBackend {
    type Vector: Clone + Send + Sync + 'static;

    fn zero(&self) -> Self::Vector;

    fn bundle(&self, a: &Self::Vector, b: &Self::Vector) -> Self::Vector;

    fn bind(&self, a: &Self::Vector, b: &Self::Vector) -> Self::Vector;

    fn cosine(&self, a: &Self::Vector, b: &Self::Vector) -> f64;

    fn encode_data(
        &self,
        data: &[u8],
        config: &ReversibleVSAConfig,
        path: Option<&str>,
    ) -> Self::Vector;

    fn decode_data(
        &self,
        vec: &Self::Vector,
        config: &ReversibleVSAConfig,
        path: Option<&str>,
        expected_size: usize,
    ) -> Vec<u8>;
}

/// Default backend for today: the existing `SparseVec` substrate.
#[derive(Clone, Copy, Debug, Default)]
pub struct SparseVecBackend;

impl VsaBackend for SparseVecBackend {
    type Vector = SparseVec;

    fn zero(&self) -> Self::Vector {
        SparseVec::new()
    }

    fn bundle(&self, a: &Self::Vector, b: &Self::Vector) -> Self::Vector {
        a.bundle(b)
    }

    fn bind(&self, a: &Self::Vector, b: &Self::Vector) -> Self::Vector {
        a.bind(b)
    }

    fn cosine(&self, a: &Self::Vector, b: &Self::Vector) -> f64 {
        a.cosine(b)
    }

    fn encode_data(
        &self,
        data: &[u8],
        config: &ReversibleVSAConfig,
        path: Option<&str>,
    ) -> Self::Vector {
        SparseVec::encode_data(data, config, path)
    }

    fn decode_data(
        &self,
        vec: &Self::Vector,
        config: &ReversibleVSAConfig,
        path: Option<&str>,
        expected_size: usize,
    ) -> Vec<u8> {
        vec.decode_data(config, path, expected_size)
    }
}

/// Minimal vector store abstraction.
///
/// This matches typical kernel/runtime needs: fetch vectors by ID.
pub trait VectorStore<V> {
    fn get(&self, id: usize) -> Option<&V>;
}

impl VectorStore<SparseVec> for HashMap<usize, SparseVec> {
    fn get(&self, id: usize) -> Option<&SparseVec> {
        self.get(&id)
    }
}

/// Candidate generation seam. Intended to wrap e.g. `TernaryInvertedIndex`.
pub trait CandidateGenerator<V> {
    type Candidate;

    fn candidates(&self, query: &V, k: usize) -> Vec<Self::Candidate>;
}

/// Rerank a set of candidate IDs by exact cosine similarity.
///
/// Returns the top-k `(id, cosine)` pairs sorted by descending cosine.
///
/// This is deliberately backend/store-driven so it can operate on either
/// `SparseVec` today or a packed ternary vector later.
pub fn rerank_top_k_by_cosine<B, S>(
    backend: &B,
    store: &S,
    query: &B::Vector,
    candidate_ids: impl IntoIterator<Item = usize>,
    k: usize,
) -> Result<Vec<(usize, f64)>, KernelInteropError>
where
    B: VsaBackend,
    S: VectorStore<B::Vector>,
{
    if k == 0 {
        return Ok(Vec::new());
    }

    let mut scored = Vec::new();
    for id in candidate_ids {
        let vec = store
            .get(id)
            .ok_or(KernelInteropError::MissingVector { id })?;
        scored.push((id, backend.cosine(query, vec)));
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    Ok(scored)
}
