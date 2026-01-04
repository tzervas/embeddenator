use embeddenator::SparseVec;
use rand::Rng;

pub fn mk_random_sparsevec(rng: &mut impl Rng, dims: usize, sparsity: usize) -> SparseVec {
    use std::collections::HashSet;

    let mut used: HashSet<usize> = HashSet::with_capacity(sparsity.saturating_mul(2));
    let mut pos = Vec::with_capacity(sparsity / 2);
    let mut neg = Vec::with_capacity(sparsity / 2);

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

fn intersection_count_sorted(a: &[usize], b: &[usize]) -> usize {
    let mut i = 0;
    let mut j = 0;
    let mut count = 0;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                count += 1;
                i += 1;
                j += 1;
            }
        }
    }
    count
}

/// Sparse ternary dot product: (pp + nn) - (pn + np)
pub fn sparse_dot(a: &SparseVec, b: &SparseVec) -> i32 {
    let pp = intersection_count_sorted(&a.pos, &b.pos) as i32;
    let nn = intersection_count_sorted(&a.neg, &b.neg) as i32;
    let pn = intersection_count_sorted(&a.pos, &b.neg) as i32;
    let np = intersection_count_sorted(&a.neg, &b.pos) as i32;
    (pp + nn) - (pn + np)
}
