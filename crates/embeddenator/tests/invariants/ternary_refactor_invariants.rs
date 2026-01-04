#![cfg(feature = "ternary-refactor")]

use embeddenator::SparseVec;
use rand::{Rng, SeedableRng};

fn ref_bundle(a: &SparseVec, b: &SparseVec) -> SparseVec {
    use std::collections::HashSet;

    let apos: HashSet<usize> = a.pos.iter().copied().collect();
    let aneg: HashSet<usize> = a.neg.iter().copied().collect();
    let bpos: HashSet<usize> = b.pos.iter().copied().collect();
    let bneg: HashSet<usize> = b.neg.iter().copied().collect();

    let mut pos: HashSet<usize> = HashSet::new();
    let mut neg: HashSet<usize> = HashSet::new();

    // Majority for two vectors: sign survives unless opposed.
    for &idx in &a.pos {
        if bpos.contains(&idx) || !bneg.contains(&idx) {
            pos.insert(idx);
        }
    }
    for &idx in &b.pos {
        if apos.contains(&idx) || !aneg.contains(&idx) {
            pos.insert(idx);
        }
    }
    for &idx in &a.neg {
        if bneg.contains(&idx) || !bpos.contains(&idx) {
            neg.insert(idx);
        }
    }
    for &idx in &b.neg {
        if aneg.contains(&idx) || !apos.contains(&idx) {
            neg.insert(idx);
        }
    }

    // Cancel any conflicts.
    pos.retain(|x| !neg.contains(x));
    neg.retain(|x| !pos.contains(x));

    let mut pos: Vec<usize> = pos.into_iter().collect();
    let mut neg: Vec<usize> = neg.into_iter().collect();
    pos.sort_unstable();
    neg.sort_unstable();
    SparseVec { pos, neg }
}

fn ref_bind(a: &SparseVec, b: &SparseVec) -> SparseVec {
    use std::collections::HashSet;

    let apos: HashSet<usize> = a.pos.iter().copied().collect();
    let aneg: HashSet<usize> = a.neg.iter().copied().collect();

    let mut pos = Vec::new();
    let mut neg = Vec::new();

    for &idx in &b.pos {
        if apos.contains(&idx) {
            pos.push(idx);
        } else if aneg.contains(&idx) {
            neg.push(idx);
        }
    }

    for &idx in &b.neg {
        if apos.contains(&idx) {
            neg.push(idx);
        } else if aneg.contains(&idx) {
            pos.push(idx);
        }
    }

    pos.sort_unstable();
    neg.sort_unstable();
    SparseVec { pos, neg }
}

fn ref_dot(a: &SparseVec, b: &SparseVec) -> i32 {
    use std::collections::HashSet;

    let apos: HashSet<usize> = a.pos.iter().copied().collect();
    let aneg: HashSet<usize> = a.neg.iter().copied().collect();

    let mut dot = 0i32;
    for &idx in &b.pos {
        if apos.contains(&idx) {
            dot += 1;
        } else if aneg.contains(&idx) {
            dot -= 1;
        }
    }
    for &idx in &b.neg {
        if apos.contains(&idx) {
            dot -= 1;
        } else if aneg.contains(&idx) {
            dot += 1;
        }
    }
    dot
}

fn mk_random_sparsevec(rng: &mut impl Rng, dims: usize, sparsity: usize) -> SparseVec {
    use std::collections::HashSet;

    let mut used: HashSet<usize> = HashSet::new();
    let mut pos = Vec::new();
    let mut neg = Vec::new();

    // Roughly half pos/half neg.
    let target_each = sparsity / 2;
    while pos.len() < target_each {
        let idx = rng.gen_range(0..dims);
        if used.insert(idx) {
            pos.push(idx);
        }
    }
    while neg.len() < target_each {
        let idx = rng.gen_range(0..dims);
        if used.insert(idx) {
            neg.push(idx);
        }
    }

    pos.sort_unstable();
    neg.sort_unstable();
    SparseVec { pos, neg }
}

#[test]
fn invariant_bundle_matches_reference() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xBADC0FFEE);

    for _case in 0..500 {
        let a = mk_random_sparsevec(&mut rng, 10_000, 200);
        let b = mk_random_sparsevec(&mut rng, 10_000, 200);

        let got = a.bundle(&b);
        let expect = ref_bundle(&a, &b);
        assert_eq!(got.pos, expect.pos);
        assert_eq!(got.neg, expect.neg);
    }
}

#[test]
fn invariant_bind_matches_reference() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xFEEDFACE);

    for _case in 0..500 {
        let a = mk_random_sparsevec(&mut rng, 10_000, 200);
        let b = mk_random_sparsevec(&mut rng, 10_000, 200);

        let got = a.bind(&b);
        let expect = ref_bind(&a, &b);
        assert_eq!(got.pos, expect.pos);
        assert_eq!(got.neg, expect.neg);
    }
}

#[test]
fn invariant_cosine_dot_matches_reference() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xD15EA5E);

    for _case in 0..500 {
        let a = mk_random_sparsevec(&mut rng, 10_000, 200);
        let b = mk_random_sparsevec(&mut rng, 10_000, 200);

        let dot_ref = ref_dot(&a, &b);

        // Recover dot from cosine definition used in SparseVec::cosine:
        // cosine = dot / (sqrt(|a|) * sqrt(|b|))
        let denom = ((a.pos.len() + a.neg.len()) as f64).sqrt() * ((b.pos.len() + b.neg.len()) as f64).sqrt();
        let dot_from_cos = if denom == 0.0 { 0.0 } else { a.cosine(&b) * denom };

        // Floating error margin; dot is integer.
        let rounded = dot_from_cos.round() as i32;
        assert_eq!(rounded, dot_ref);
    }
}
