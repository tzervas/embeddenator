#![cfg(feature = "bt-phase-2")]

use embeddenator::{SparseVec, DIM};

fn mk_dense_sparsevec(seed: usize, pos_count: usize, neg_count: usize) -> SparseVec {
    assert!(pos_count + neg_count <= DIM);

    // Deterministic, allocation-light unique index stream.
    // DIM = 10_000, stride 73 is coprime, so we get a full-cycle permutation.
    let stride: usize = 73;
    let mut idx = seed % DIM;

    let mut pos = Vec::with_capacity(pos_count);
    let mut neg = Vec::with_capacity(neg_count);

    for _ in 0..pos_count {
        pos.push(idx);
        idx = (idx + stride) % DIM;
    }
    for _ in 0..neg_count {
        neg.push(idx);
        idx = (idx + stride) % DIM;
    }

    pos.sort_unstable();
    neg.sort_unstable();
    SparseVec { pos, neg }
}

fn sparse_to_dense_sign(v: &SparseVec) -> Vec<i8> {
    let mut out = vec![0i8; DIM];
    for &i in &v.pos {
        out[i] = 1;
    }
    for &i in &v.neg {
        out[i] = -1;
    }
    out
}

fn dense_sign_to_sparse(signs: &[i8]) -> SparseVec {
    let mut pos = Vec::new();
    let mut neg = Vec::new();
    for (i, &s) in signs.iter().enumerate() {
        if s > 0 {
            pos.push(i);
        } else if s < 0 {
            neg.push(i);
        }
    }
    SparseVec { pos, neg }
}

fn ref_bundle(a: &SparseVec, b: &SparseVec) -> SparseVec {
    let da = sparse_to_dense_sign(a);
    let db = sparse_to_dense_sign(b);

    let mut out = vec![0i8; DIM];
    for i in 0..DIM {
        let av = da[i];
        let bv = db[i];
        out[i] = if av == 0 {
            bv
        } else if bv == 0 {
            av
        } else if av == bv {
            av
        } else {
            0
        };
    }

    dense_sign_to_sparse(&out)
}

fn ref_bind(a: &SparseVec, b: &SparseVec) -> SparseVec {
    let da = sparse_to_dense_sign(a);
    let db = sparse_to_dense_sign(b);

    let mut out = vec![0i8; DIM];
    for i in 0..DIM {
        let av = da[i];
        let bv = db[i];
        out[i] = if av == 0 || bv == 0 { 0 } else { av * bv };
    }

    dense_sign_to_sparse(&out)
}

fn ref_cosine(a: &SparseVec, b: &SparseVec) -> f64 {
    let da = sparse_to_dense_sign(a);
    let db = sparse_to_dense_sign(b);

    let mut dot: i32 = 0;
    let mut nnz_a: i32 = 0;
    let mut nnz_b: i32 = 0;

    for i in 0..DIM {
        let av = da[i] as i32;
        let bv = db[i] as i32;
        dot += av * bv;
        nnz_a += (av != 0) as i32;
        nnz_b += (bv != 0) as i32;
    }

    if nnz_a == 0 || nnz_b == 0 {
        return 0.0;
    }

    let denom = (nnz_a as f64).sqrt() * (nnz_b as f64).sqrt();
    (dot as f64) / denom
}

fn assert_sparse_eq(got: &SparseVec, expect: &SparseVec) {
    assert_eq!(got.pos, expect.pos, "pos mismatch");
    assert_eq!(got.neg, expect.neg, "neg mismatch");
}

fn fresh_thread<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::spawn(f).join().expect("thread panicked")
}

fn dirty_tls_with_varied_ops() {
    // Use a few dense vectors to keep the packed paths hot.
    // Mix densities to maximize the chance of catching stale bits.
    let very_dense_a = mk_dense_sparsevec(0xA11CE, 4500, 4500);
    let very_dense_b = mk_dense_sparsevec(0xBADC0DE, 4500, 4500);

    for i in 0..25usize {
        let x = mk_dense_sparsevec(0x1000 + i, 3200, 2600);
        let y = mk_dense_sparsevec(0x2000 + i * 3, 3100, 2700);

        let _ = very_dense_a.bundle(&x);
        let _ = very_dense_b.bind(&y);
        let _ = x.bundle(&y);
        let _ = x.bind(&y);
        let _ = x.cosine(&y);
    }
}

#[test]
fn bt_phase2_tls_scratch_bundle_no_leak_and_bit_exact() {
    let a = mk_dense_sparsevec(0xC0FFEE, 3200, 2600);
    let b = mk_dense_sparsevec(0xFACE00, 3100, 2700);

    // Reference result (pure, allocation-heavy but deterministic).
    let expect = ref_bundle(&a, &b);

    // Fresh thread reference: same public API, but with pristine TLS.
    let fresh = {
        let a = a.clone();
        let b = b.clone();
        fresh_thread(move || a.bundle(&b))
    };

    // Dirty TLS in current thread, then compute.
    dirty_tls_with_varied_ops();
    let got1 = a.bundle(&b);

    // Dirty again to ensure repeated reuse is stable.
    dirty_tls_with_varied_ops();
    let got2 = a.bundle(&b);

    assert_sparse_eq(&fresh, &expect);
    assert_sparse_eq(&got1, &expect);
    assert_sparse_eq(&got2, &expect);
}

#[test]
fn bt_phase2_tls_scratch_bind_no_leak_and_bit_exact() {
    let a = mk_dense_sparsevec(0x123456, 3000, 2800);
    let b = mk_dense_sparsevec(0xABCDEF, 3100, 2700);

    let expect = ref_bind(&a, &b);

    let fresh = {
        let a = a.clone();
        let b = b.clone();
        fresh_thread(move || a.bind(&b))
    };

    dirty_tls_with_varied_ops();
    let got1 = a.bind(&b);

    dirty_tls_with_varied_ops();
    let got2 = a.bind(&b);

    assert_sparse_eq(&fresh, &expect);
    assert_sparse_eq(&got1, &expect);
    assert_sparse_eq(&got2, &expect);
}

#[test]
fn bt_phase2_tls_scratch_cosine_no_leak_and_bit_exact() {
    let a = mk_dense_sparsevec(0x0DDC0DE, 3200, 2600);
    let b = mk_dense_sparsevec(0xD15EA5E, 3100, 2700);

    let expect = ref_cosine(&a, &b);

    let fresh = {
        let a = a.clone();
        let b = b.clone();
        fresh_thread(move || a.cosine(&b))
    };

    dirty_tls_with_varied_ops();
    let got1 = a.cosine(&b);

    dirty_tls_with_varied_ops();
    let got2 = a.cosine(&b);

    assert_eq!(fresh.to_bits(), expect.to_bits(), "fresh vs ref bits");
    assert_eq!(got1.to_bits(), expect.to_bits(), "dirty vs ref bits");
    assert_eq!(got2.to_bits(), expect.to_bits(), "repeat vs ref bits");
}

#[test]
fn bt_phase2_tls_scratch_multi_thread_sanity() {
    let a = mk_dense_sparsevec(0x515151, 3000, 2800);
    let b = mk_dense_sparsevec(0xA0A0A0, 3100, 2700);

    let expect_bundle = ref_bundle(&a, &b);
    let expect_bind = ref_bind(&a, &b);
    let expect_cos_bits = ref_cosine(&a, &b).to_bits();

    let threads = 4;
    let mut joins = Vec::with_capacity(threads);

    for t in 0..threads {
        let a = a.clone();
        let b = b.clone();
        let eb = expect_bundle.clone();
        let ei = expect_bind.clone();
        joins.push(std::thread::spawn(move || {
            // Each thread should have independent TLS scratch; pollute locally.
            for i in 0..10usize {
                let x = mk_dense_sparsevec(0x9000 + t * 100 + i, 4500, 4500);
                let y = mk_dense_sparsevec(0xA000 + t * 100 + i, 4500, 4500);
                let _ = x.bundle(&y);
                let _ = x.bind(&y);
                let _ = x.cosine(&y);
            }

            let got_bundle = a.bundle(&b);
            let got_bind = a.bind(&b);
            let got_cos = a.cosine(&b);

            assert_sparse_eq(&got_bundle, &eb);
            assert_sparse_eq(&got_bind, &ei);
            assert_eq!(got_cos.to_bits(), expect_cos_bits);
        }));
    }

    for j in joins {
        j.join().expect("thread panicked");
    }
}
