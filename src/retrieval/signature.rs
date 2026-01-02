//! Optional signature-based candidate generation for sparse ternary vectors.
//!
//! This is an opt-in alternative to inverted-index candidate generation.
//!
//! Design goals:
//! - Deterministic across runs (fixed probe dimensions + stable iteration)
//! - Fast to build + query
//! - Multi-probe support (radius-1) to soften bucket boundary effects

use crate::kernel_interop::CandidateGenerator;
use crate::vsa::{SparseVec, DIM};
use std::collections::{HashMap, HashSet};

/// How many probe dimensions are used for the default signature.
///
/// Each probe consumes 2 bits in the `u64` signature encoding.
pub const DEFAULT_SIGNATURE_PROBES: usize = 24;

/// Query-time knobs for signature candidate generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignatureQueryOptions {
    /// Maximum number of candidate IDs to return.
    pub max_candidates: usize,

    /// Multi-probe radius. Currently clamped to {0,1}.
    ///
    /// - 0: only exact signature bucket
    /// - 1: also probe one-dimension variants (two alternates per probe)
    pub probe_radius: u8,

    /// Upper bound on the number of signature buckets to probe.
    ///
    /// This protects against expensive probing when probe dimensions are large.
    pub max_probes: usize,
}

impl Default for SignatureQueryOptions {
    fn default() -> Self {
        Self {
            max_candidates: 1_000,
            probe_radius: 1,
            max_probes: 1 + (2 * DEFAULT_SIGNATURE_PROBES),
        }
    }
}

/// Signature-bucket index for sparse ternary vectors.
///
/// The signature is a compact encoding of the vector’s values at a fixed set of
/// probe dimensions. Vectors sharing signatures are likely to be similar.
#[derive(Clone, Debug)]
pub struct TernarySignatureIndex {
    probe_dims: Vec<usize>,
    buckets: HashMap<u64, Vec<usize>>, // signature -> sorted IDs
}

impl TernarySignatureIndex {
    /// Build a signature index from a codebook-style map.
    ///
    /// IDs do not need to be contiguous.
    pub fn build_from_map(map: &HashMap<usize, SparseVec>) -> Self {
        let probe_dims = default_probe_dims(DEFAULT_SIGNATURE_PROBES);
        Self::build_from_map_with_probes(map, probe_dims)
    }

    /// Build a signature index from a map using explicit probe dimensions.
    pub fn build_from_map_with_probes(map: &HashMap<usize, SparseVec>, probe_dims: Vec<usize>) -> Self {
        let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();

        // Deterministic build: iterate IDs in sorted order.
        let mut ids: Vec<usize> = map.keys().copied().collect();
        ids.sort_unstable();

        for id in ids {
            let Some(vec) = map.get(&id) else { continue };
            let sig = signature_for(vec, &probe_dims);
            buckets.entry(sig).or_default().push(id);
        }

        // Buckets are already in increasing ID order due to sorted iteration, but keep it explicit.
        for ids in buckets.values_mut() {
            ids.sort_unstable();
            ids.dedup();
        }

        Self { probe_dims, buckets }
    }

    pub fn probe_dims(&self) -> &[usize] {
        &self.probe_dims
    }

    /// Get candidate IDs for a query vector.
    pub fn candidates_with_options(&self, query: &SparseVec, opts: SignatureQueryOptions) -> Vec<usize> {
        if opts.max_candidates == 0 {
            return Vec::new();
        }

        let sig = signature_for(query, &self.probe_dims);
        let probe_radius = opts.probe_radius.min(1);
        let probe_sigs = probe_signatures(sig, self.probe_dims.len(), probe_radius, opts.max_probes);

        let mut seen: HashSet<usize> = HashSet::new();
        let mut out: Vec<usize> = Vec::new();

        for ps in probe_sigs {
            let Some(ids) = self.buckets.get(&ps) else { continue };
            for &id in ids {
                if seen.insert(id) {
                    out.push(id);
                    if out.len() >= opts.max_candidates {
                        break;
                    }
                }
            }
            if out.len() >= opts.max_candidates {
                break;
            }
        }

        // Keep deterministic ordering for downstream callers.
        out.sort_unstable();
        out
    }
}

impl CandidateGenerator<SparseVec> for TernarySignatureIndex {
    type Candidate = usize;

    /// Generate up to `k` candidate IDs.
    fn candidates(&self, query: &SparseVec, k: usize) -> Vec<Self::Candidate> {
        self.candidates_with_options(
            query,
            SignatureQueryOptions {
                max_candidates: k,
                ..SignatureQueryOptions::default()
            },
        )
    }
}

fn default_probe_dims(count: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(count);
    let mut seen = HashSet::with_capacity(count * 2);

    // Deterministic pseudo-random stream (SplitMix64).
    // Arbitrary fixed seed for deterministic probe selection.
    let mut state: u64 = 0xED00_0000_0000_0001u64;

    while out.len() < count {
        state = splitmix64(state);
        let d = (state as usize) % DIM;
        if seen.insert(d) {
            out.push(d);
        }
    }

    out
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Encode a signature into 2-bit lanes:
/// - 0 => 00
/// - +1 => 01
/// - -1 => 10
fn signature_for(vec: &SparseVec, probe_dims: &[usize]) -> u64 {
    let mut sig: u64 = 0;
    for (i, &d) in probe_dims.iter().enumerate() {
        let lane = match sign_at(vec, d) {
            0 => 0b00u64,
            1 => 0b01u64,
            -1 => 0b10u64,
            _ => 0b00u64,
        };
        sig |= lane << (2 * i);
    }
    sig
}

fn sign_at(vec: &SparseVec, dim: usize) -> i8 {
    if vec.pos.contains(&dim) {
        1
    } else if vec.neg.contains(&dim) {
        -1
    } else {
        0
    }
}

fn probe_signatures(base: u64, probes: usize, radius: u8, max_probes: usize) -> Vec<u64> {
    if max_probes == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    out.push(base);

    if radius == 0 {
        return out;
    }

    // Radius-1 probing: for each probe lane, flip it to the other two values.
    for i in 0..probes {
        if out.len() >= max_probes {
            break;
        }

        let shift = 2 * i;
        let mask = 0b11u64 << shift;
        let cur = (base & mask) >> shift;

        // Deterministic order: 00 -> 01 -> 10.
        for &alt in &[0b00u64, 0b01u64, 0b10u64] {
            if alt == cur {
                continue;
            }
            let next = (base & !mask) | (alt << shift);
            out.push(next);
            if out.len() >= max_probes {
                break;
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vsa::ReversibleVSAConfig;

    #[test]
    fn default_probe_dims_are_stable_and_in_range() {
        let a = default_probe_dims(DEFAULT_SIGNATURE_PROBES);
        let b = default_probe_dims(DEFAULT_SIGNATURE_PROBES);
        assert_eq!(a, b);
        assert_eq!(a.len(), DEFAULT_SIGNATURE_PROBES);
        for &d in &a {
            assert!(d < DIM);
        }

        let mut uniq = a.clone();
        uniq.sort_unstable();
        uniq.dedup();
        assert_eq!(uniq.len(), DEFAULT_SIGNATURE_PROBES);
    }

    #[test]
    fn candidates_are_deterministic_and_include_self_when_exact_bucket_hits() {
        let cfg = ReversibleVSAConfig::default();

        let v0 = SparseVec::encode_data(b"alpha", &cfg, None);
        let v1 = SparseVec::encode_data(b"beta", &cfg, None);

        let mut map = HashMap::new();
        map.insert(0, v0.clone());
        map.insert(1, v1);

        let idx = TernarySignatureIndex::build_from_map(&map);
        let opts = SignatureQueryOptions {
            max_candidates: 10,
            probe_radius: 0,
            max_probes: 1,
        };

        let c1 = idx.candidates_with_options(&v0, opts);
        let c2 = idx.candidates_with_options(&v0, opts);
        assert_eq!(c1, c2);
        assert!(c1.contains(&0));
    }

    #[test]
    fn probe_signatures_radius_one_includes_base_and_is_bounded() {
        let base = 0u64;
        let sigs = probe_signatures(base, 4, 1, 5);
        assert_eq!(sigs.len(), 5);
        assert_eq!(sigs[0], base);
    }
}
