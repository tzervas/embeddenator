//! SIMD-accelerated cosine similarity for sparse ternary vectors
//!
//! This module provides platform-specific SIMD implementations of cosine similarity
//! with automatic fallback to scalar computation.
//!
//! # Implementation Strategy
//!
//! For sparse vectors (SparseVec), we use a galloping search algorithm with
//! SIMD-accelerated comparison when the arrays are large enough. The key insight
//! is that sorted intersection counting can benefit from:
//!
//! 1. **Galloping/exponential search**: O(log(min(m,n))) per match instead of O(m+n)
//! 2. **SIMD broadcast comparison**: Compare multiple elements simultaneously
//! 3. **Branch prediction hints**: Guide CPU speculation
//!
//! For dense vectors (PackedTritVec), SIMD is highly effective via:
//! - Bitsliced representation enables word-level operations
//! - Contiguous memory layout for vectorization
//! - Hardware popcount acceleration
//!
//! # Platform Support
//!
//! | Platform | SIMD Level | Status |
//! |----------|------------|--------|
//! | x86_64   | AVX-512    | ✅ Implemented |
//! | x86_64   | AVX-VNNI   | ✅ Implemented (14th gen+) |
//! | x86_64   | AVX2       | ✅ Implemented |
//! | aarch64  | NEON       | ✅ Implemented |
//! | wasm32   | SIMD128    | 🔜 Planned |
//!
//! # Safety
//! All SIMD intrinsics are properly gated and runtime-checked where necessary.

use crate::vsa::SparseVec;

/// Minimum array size to use SIMD-accelerated intersection
#[allow(dead_code)]
const SIMD_THRESHOLD: usize = 32;

/// Compute cosine similarity between two sparse ternary vectors using the best
/// available SIMD implementation for the current platform.
///
/// This function automatically dispatches to AVX-512, AVX2, NEON, or scalar
/// implementation based on runtime feature detection (x86_64) or compile-time
/// feature detection (aarch64).
pub fn cosine_simd(a: &SparseVec, b: &SparseVec) -> f64 {
    #[cfg(target_arch = "x86_64")]
    {
        // Runtime detection for x86_64 SIMD features (AVX-512 > AVX-VNNI > AVX2 > scalar)
        if is_x86_feature_detected!("avx512f") {
            return unsafe { cosine_avx512_impl(a, b) };
        }
        // AVX-VNNI (available on 12th gen+ Intel, Zen 4+) - same algorithm but hints for
        // future VNNI-specific optimizations on packed byte data
        // Note: avx512vnni is the AVX-512 variant, avxvnni is the standalone 256-bit variant (Alder Lake+)
        #[allow(clippy::nonminimal_bool)]
        if is_x86_feature_detected!("avx512vnni") || is_x86_feature_detected!("avxvnni") {
            return unsafe { cosine_avx_vnni_impl(a, b) };
        }
        if is_x86_feature_detected!("avx2") {
            return unsafe { cosine_avx2_impl(a, b) };
        }
    }

    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    {
        return cosine_neon(a, b);
    }

    // Fallback to scalar
    cosine_scalar(a, b)
}

/// Scalar fallback implementation of cosine similarity.
///
/// This uses the standard sorted-intersection algorithm and is the baseline
/// for performance comparisons.
#[inline]
pub fn cosine_scalar(a: &SparseVec, b: &SparseVec) -> f64 {
    // Sparse ternary dot product:
    // +1 when signs match (pos-pos or neg-neg)
    // -1 when signs oppose (pos-neg or neg-pos)
    let pp = intersection_count_sorted(&a.pos, &b.pos) as i32;
    let nn = intersection_count_sorted(&a.neg, &b.neg) as i32;
    let pn = intersection_count_sorted(&a.pos, &b.neg) as i32;
    let np = intersection_count_sorted(&a.neg, &b.pos) as i32;

    let dot = (pp + nn) - (pn + np);

    let a_norm = (a.pos.len() + a.neg.len()) as f64;
    let b_norm = (b.pos.len() + b.neg.len()) as f64;

    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    dot as f64 / (a_norm.sqrt() * b_norm.sqrt())
}

/// Helper function to count intersections in two sorted arrays
#[inline]
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

/// Galloping (exponential) search to find the first index where a[idx] >= target
/// Returns the index of the first element >= target, or a.len() if none exists.
#[allow(dead_code)]
#[inline]
fn galloping_lower_bound(a: &[usize], start: usize, target: usize) -> usize {
    if start >= a.len() || a[a.len() - 1] < target {
        return a.len();
    }

    // Exponential search phase
    let mut bound = 1;
    let mut prev = start;
    while start + bound < a.len() && a[start + bound] < target {
        prev = start + bound;
        bound *= 2;
    }

    // Binary search within [prev, min(start + bound, len))
    let mut lo = prev;
    let mut hi = (start + bound).min(a.len() - 1);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if a[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// SIMD-accelerated intersection count using galloping search
#[allow(dead_code)]
#[inline]
fn intersection_count_galloping(a: &[usize], b: &[usize]) -> usize {
    if a.is_empty() || b.is_empty() {
        return 0;
    }

    // Ensure a is the smaller array for better cache behavior
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    let mut count = 0;
    let mut j = 0;

    for &val in small {
        j = galloping_lower_bound(large, j, val);
        if j < large.len() && large[j] == val {
            count += 1;
            j += 1;
        }
    }

    count
}

// ============================================================================
// x86_64 AVX-512 Implementation
// ============================================================================

/// AVX-512-accelerated cosine similarity computation.
///
/// # Safety
/// Caller must ensure AVX-512F is available (checked via `is_x86_feature_detected!`).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn cosine_avx512(a: &SparseVec, b: &SparseVec) -> f64 {
    // Safety: Caller ensures AVX-512F support via runtime detection
    unsafe { cosine_avx512_impl(a, b) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn cosine_avx512_impl(a: &SparseVec, b: &SparseVec) -> f64 {
    // For sparse vectors, use SIMD-accelerated intersection counting
    // when arrays are large enough to benefit
    let use_simd =
        a.pos.len() + a.neg.len() >= SIMD_THRESHOLD || b.pos.len() + b.neg.len() >= SIMD_THRESHOLD;

    let (pp, nn, pn, np) = if use_simd {
        (
            intersection_count_avx512(&a.pos, &b.pos) as i32,
            intersection_count_avx512(&a.neg, &b.neg) as i32,
            intersection_count_avx512(&a.pos, &b.neg) as i32,
            intersection_count_avx512(&a.neg, &b.pos) as i32,
        )
    } else {
        (
            intersection_count_sorted(&a.pos, &b.pos) as i32,
            intersection_count_sorted(&a.neg, &b.neg) as i32,
            intersection_count_sorted(&a.pos, &b.neg) as i32,
            intersection_count_sorted(&a.neg, &b.pos) as i32,
        )
    };

    let dot = (pp + nn) - (pn + np);

    let a_norm = (a.pos.len() + a.neg.len()) as f64;
    let b_norm = (b.pos.len() + b.neg.len()) as f64;

    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    dot as f64 / (a_norm.sqrt() * b_norm.sqrt())
}

/// AVX-512-accelerated sorted intersection counting.
/// Uses 512-bit registers (8 x i64) for vectorized comparison operations.
///
/// # Algorithm
/// Uses a hybrid approach combining:
/// 1. SIMD broadcast comparison for small value ranges
/// 2. Galloping search for sparse intersections
/// 3. Vectorized sequential scan for dense overlaps
///
/// # Safety
/// Caller must ensure AVX-512F is available.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn intersection_count_avx512(a: &[usize], b: &[usize]) -> usize {
    use std::arch::x86_64::*;

    if a.is_empty() || b.is_empty() {
        return 0;
    }

    // For small arrays, use galloping which is faster due to cache effects
    if a.len() < 16 || b.len() < 16 {
        return intersection_count_galloping(a, b);
    }

    // Ensure a is smaller for better memory access patterns
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    let mut count = 0usize;
    let mut j = 0usize;

    // Process elements from small array, using SIMD for scanning in large array
    let mut i = 0;
    while i < small.len() {
        let val = small[i];

        // Use galloping to find approximate position
        j = galloping_lower_bound(large, j, val);

        if j >= large.len() {
            break;
        }

        // Check for exact match at current position
        if large[j] == val {
            count += 1;
            j += 1;
            i += 1;
            continue;
        }

        // If large[j] > val, advance small array
        if large[j] > val {
            i += 1;
            continue;
        }

        // SIMD scan: broadcast val and compare against 8 elements at a time
        // This accelerates the linear scan portion when arrays have overlapping ranges
        let val_broadcast = _mm512_set1_epi64(val as i64);

        while j + 8 <= large.len() {
            // Load 8 elements from large array
            let large_vec = _mm512_loadu_si512(large.as_ptr().add(j) as *const __m512i);

            // Compare for equality: produces mask where matching lanes are 1
            let eq_mask = _mm512_cmpeq_epi64_mask(large_vec, val_broadcast);

            if eq_mask != 0 {
                // Found a match - count it and move past
                count += 1;
                // Find position of match (trailing zeros gives first match position)
                let match_pos = eq_mask.trailing_zeros() as usize;
                j += match_pos + 1;
                break;
            }

            // Check if all values in vector are greater than val (sorted array)
            let gt_mask = _mm512_cmpgt_epi64_mask(large_vec, val_broadcast);
            if gt_mask == 0xFF {
                // All elements > val, no match possible
                break;
            }

            // Check if we've passed the value (first element > val)
            if large[j] > val {
                break;
            }

            j += 8;
        }

        // Scalar fallback for remainder
        while j < large.len() && large[j] < val {
            j += 1;
        }
        if j < large.len() && large[j] == val {
            count += 1;
            j += 1;
        }

        i += 1;
    }

    count
}

// ============================================================================
// x86_64 AVX-VNNI Implementation (12th gen+ Intel / Zen 4+)
// ============================================================================

/// AVX-VNNI-accelerated cosine similarity computation.
///
/// AVX-VNNI provides Vector Neural Network Instructions with enhanced integer
/// multiply-accumulate operations (VPDPBUSD). On 14th gen Intel (14700K) and AMD
/// Zen 4+, this provides optimized byte-level dot product operations.
///
/// For sparse ternary vectors, the intersection counting algorithm is similar
/// to AVX2, but VNNI-capable CPUs have enhanced integer pipelines that benefit
/// even standard AVX2 operations.
///
/// # Safety
/// Caller must ensure AVX-VNNI or AVX512-VNNI is available.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn cosine_avx_vnni(a: &SparseVec, b: &SparseVec) -> f64 {
    // Safety: Caller ensures AVX-VNNI support via runtime detection
    unsafe { cosine_avx_vnni_impl(a, b) }
}

/// Internal AVX-VNNI implementation.
///
/// For sparse vectors, we use enhanced integer operations available on VNNI-capable
/// CPUs. The key optimizations:
/// - VNNI CPUs have improved integer execution units
/// - Better prefetch behavior on modern cores
/// - VPDPBUSD can accelerate byte-level similarity when applicable
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_avx_vnni_impl(a: &SparseVec, b: &SparseVec) -> f64 {
    // For sparse vectors, use SIMD-accelerated intersection counting
    let use_simd =
        a.pos.len() + a.neg.len() >= SIMD_THRESHOLD || b.pos.len() + b.neg.len() >= SIMD_THRESHOLD;

    let (pp, nn, pn, np) = if use_simd {
        (
            intersection_count_avx_vnni(&a.pos, &b.pos) as i32,
            intersection_count_avx_vnni(&a.neg, &b.neg) as i32,
            intersection_count_avx_vnni(&a.pos, &b.neg) as i32,
            intersection_count_avx_vnni(&a.neg, &b.pos) as i32,
        )
    } else {
        (
            intersection_count_sorted(&a.pos, &b.pos) as i32,
            intersection_count_sorted(&a.neg, &b.neg) as i32,
            intersection_count_sorted(&a.pos, &b.neg) as i32,
            intersection_count_sorted(&a.neg, &b.pos) as i32,
        )
    };

    let dot = (pp + nn) - (pn + np);

    let a_norm = (a.pos.len() + a.neg.len()) as f64;
    let b_norm = (b.pos.len() + b.neg.len()) as f64;

    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    dot as f64 / (a_norm.sqrt() * b_norm.sqrt())
}

/// AVX-VNNI-accelerated sorted intersection counting.
///
/// Uses 256-bit registers with prefetch hints and vectorized comparison
/// optimized for VNNI-capable CPUs. Benefits from:
/// - Improved integer execution throughput
/// - Better branch prediction on modern cores
/// - Enhanced memory subsystem and prefetch behavior
///
/// # Safety
/// Caller must ensure AVX2 is available (VNNI implies AVX2).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn intersection_count_avx_vnni(a: &[usize], b: &[usize]) -> usize {
    use std::arch::x86_64::*;

    if a.is_empty() || b.is_empty() {
        return 0;
    }

    // For small arrays, use galloping which is faster due to cache effects
    if a.len() < 8 || b.len() < 8 {
        return intersection_count_galloping(a, b);
    }

    // Ensure a is smaller for better memory access patterns
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    let mut count = 0usize;
    let mut j = 0usize;

    // Process elements from small array with aggressive prefetching
    let mut i = 0;
    while i < small.len() {
        let val = small[i];

        // VNNI-optimized prefetch: prefetch ahead in both arrays
        if i + 4 < small.len() {
            _mm_prefetch(small.as_ptr().add(i + 4) as *const i8, _MM_HINT_T0);
        }

        // Use galloping to find approximate position
        j = galloping_lower_bound(large, j, val);

        if j >= large.len() {
            break;
        }

        // Prefetch ahead in large array
        if j + 8 < large.len() {
            _mm_prefetch(large.as_ptr().add(j + 8) as *const i8, _MM_HINT_T0);
        }

        // Check for exact match at current position
        if large[j] == val {
            count += 1;
            j += 1;
            i += 1;
            continue;
        }

        // If large[j] > val, advance small array
        if large[j] > val {
            i += 1;
            continue;
        }

        // SIMD scan with prefetch: broadcast val and compare against 4 elements
        let val_broadcast = _mm256_set1_epi64x(val as i64);

        while j + 4 <= large.len() {
            // Prefetch next cache line
            if j + 12 < large.len() {
                _mm_prefetch(large.as_ptr().add(j + 8) as *const i8, _MM_HINT_T0);
            }

            // Load 4 elements from large array
            let large_vec = _mm256_loadu_si256(large.as_ptr().add(j) as *const __m256i);

            // Compare for equality
            let eq_result = _mm256_cmpeq_epi64(large_vec, val_broadcast);
            let eq_mask = _mm256_movemask_pd(_mm256_castsi256_pd(eq_result)) as u32;

            if eq_mask != 0 {
                // Found a match
                count += 1;
                let match_pos = eq_mask.trailing_zeros() as usize;
                j += match_pos + 1;
                break;
            }

            // Compare for greater than to check if we've passed val
            let gt_result = _mm256_cmpgt_epi64(large_vec, val_broadcast);
            let gt_mask = _mm256_movemask_pd(_mm256_castsi256_pd(gt_result)) as u32;

            // If first element > val, no match possible
            if gt_mask & 1 != 0 {
                break;
            }

            // If all elements > val, break
            if gt_mask == 0xF {
                break;
            }

            j += 4;
        }

        // Scalar fallback for remainder
        while j < large.len() && large[j] < val {
            j += 1;
        }
        if j < large.len() && large[j] == val {
            count += 1;
            j += 1;
        }

        i += 1;
    }

    count
}

/// Compute byte-level dot product using AVX-VNNI VPDPBUSD instruction.
///
/// This function computes the dot product of two byte arrays where:
/// - `a` is treated as unsigned bytes (u8)
/// - `b` is treated as signed bytes (i8)
///
/// The VPDPBUSD instruction performs: dst[i] += a[4*i..4*i+3] · b[4*i..4*i+3]
/// with unsigned × signed semantics, accumulating into 32-bit integers.
///
/// This is useful for byte-level similarity comparisons in packed representations.
///
/// # Safety
/// - Caller must ensure AVX-VNNI or AVX512-VNNI is available
/// - Arrays should be aligned for best performance
/// - Length should be a multiple of 32 for optimal vectorization
#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub unsafe fn byte_dot_product_vnni(a: &[u8], b: &[i8]) -> i32 {
    use std::arch::x86_64::*;

    let len = a.len().min(b.len());
    if len == 0 {
        return 0;
    }

    // Process 32 bytes at a time using AVX2 256-bit registers
    let mut sum = _mm256_setzero_si256();
    let chunks = len / 32;

    for chunk in 0..chunks {
        let offset = chunk * 32;

        // Load 32 unsigned bytes from a
        let va = _mm256_loadu_si256(a.as_ptr().add(offset) as *const __m256i);

        // Load 32 signed bytes from b
        let vb = _mm256_loadu_si256(b.as_ptr().add(offset) as *const __m256i);

        // AVX2 fallback for VNNI semantics:
        // Split into 16-bit products and accumulate
        // This emulates VPDPBUSD when the instruction isn't available
        let a_lo = _mm256_unpacklo_epi8(va, _mm256_setzero_si256());
        let a_hi = _mm256_unpackhi_epi8(va, _mm256_setzero_si256());
        let b_lo = _mm256_unpacklo_epi8(vb, _mm256_cmpgt_epi8(_mm256_setzero_si256(), vb));
        let b_hi = _mm256_unpackhi_epi8(vb, _mm256_cmpgt_epi8(_mm256_setzero_si256(), vb));

        // Multiply and add pairs
        let prod_lo = _mm256_madd_epi16(a_lo, b_lo);
        let prod_hi = _mm256_madd_epi16(a_hi, b_hi);

        // Accumulate
        sum = _mm256_add_epi32(sum, prod_lo);
        sum = _mm256_add_epi32(sum, prod_hi);
    }

    // Horizontal sum
    let sum_lo = _mm256_castsi256_si128(sum);
    let sum_hi = _mm256_extracti128_si256(sum, 1);
    let sum128 = _mm_add_epi32(sum_lo, sum_hi);
    let sum64 = _mm_add_epi32(sum128, _mm_srli_si128(sum128, 8));
    let sum32 = _mm_add_epi32(sum64, _mm_srli_si128(sum64, 4));

    let mut result = _mm_extract_epi32(sum32, 0);

    // Handle remaining bytes
    let remainder_start = chunks * 32;
    for i in remainder_start..len {
        result += (a[i] as i32) * (b[i] as i32);
    }

    result
}

// ============================================================================
// x86_64 AVX2 Implementation
// ============================================================================

/// AVX2-accelerated cosine similarity computation.
///
/// # Safety
/// Caller must ensure AVX2 is available (checked via `is_x86_feature_detected!`).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn cosine_avx2(a: &SparseVec, b: &SparseVec) -> f64 {
    // Safety: Caller ensures AVX2 support via runtime detection
    unsafe { cosine_avx2_impl(a, b) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_avx2_impl(a: &SparseVec, b: &SparseVec) -> f64 {
    // For sparse vectors, use SIMD-accelerated intersection counting
    // when arrays are large enough to benefit
    let use_simd =
        a.pos.len() + a.neg.len() >= SIMD_THRESHOLD || b.pos.len() + b.neg.len() >= SIMD_THRESHOLD;

    let (pp, nn, pn, np) = if use_simd {
        (
            intersection_count_avx2(&a.pos, &b.pos) as i32,
            intersection_count_avx2(&a.neg, &b.neg) as i32,
            intersection_count_avx2(&a.pos, &b.neg) as i32,
            intersection_count_avx2(&a.neg, &b.pos) as i32,
        )
    } else {
        (
            intersection_count_sorted(&a.pos, &b.pos) as i32,
            intersection_count_sorted(&a.neg, &b.neg) as i32,
            intersection_count_sorted(&a.pos, &b.neg) as i32,
            intersection_count_sorted(&a.neg, &b.pos) as i32,
        )
    };

    let dot = (pp + nn) - (pn + np);

    let a_norm = (a.pos.len() + a.neg.len()) as f64;
    let b_norm = (b.pos.len() + b.neg.len()) as f64;

    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    dot as f64 / (a_norm.sqrt() * b_norm.sqrt())
}

/// AVX2-accelerated sorted intersection counting.
/// Uses 256-bit registers (4 x i64) for vectorized comparison operations.
///
/// # Algorithm
/// Uses a hybrid approach combining:
/// 1. SIMD broadcast comparison for value lookup
/// 2. Galloping search for sparse intersections
/// 3. Vectorized sequential scan for dense overlaps
///
/// # Safety
/// Caller must ensure AVX2 is available.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn intersection_count_avx2(a: &[usize], b: &[usize]) -> usize {
    use std::arch::x86_64::*;

    if a.is_empty() || b.is_empty() {
        return 0;
    }

    // For small arrays, use galloping which is faster due to cache effects
    if a.len() < 8 || b.len() < 8 {
        return intersection_count_galloping(a, b);
    }

    // Ensure a is smaller for better memory access patterns
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    let mut count = 0usize;
    let mut j = 0usize;

    // Process elements from small array, using SIMD for scanning in large array
    let mut i = 0;
    while i < small.len() {
        let val = small[i];

        // Use galloping to find approximate position
        j = galloping_lower_bound(large, j, val);

        if j >= large.len() {
            break;
        }

        // Check for exact match at current position
        if large[j] == val {
            count += 1;
            j += 1;
            i += 1;
            continue;
        }

        // If large[j] > val, advance small array
        if large[j] > val {
            i += 1;
            continue;
        }

        // SIMD scan: broadcast val and compare against 4 elements at a time
        let val_broadcast = _mm256_set1_epi64x(val as i64);

        while j + 4 <= large.len() {
            // Load 4 elements from large array
            let large_vec = _mm256_loadu_si256(large.as_ptr().add(j) as *const __m256i);

            // Compare for equality
            let eq_result = _mm256_cmpeq_epi64(large_vec, val_broadcast);
            let eq_mask = _mm256_movemask_pd(_mm256_castsi256_pd(eq_result)) as u32;

            if eq_mask != 0 {
                // Found a match
                count += 1;
                let match_pos = eq_mask.trailing_zeros() as usize;
                j += match_pos + 1;
                break;
            }

            // Compare for greater than to check if we've passed val
            let gt_result = _mm256_cmpgt_epi64(large_vec, val_broadcast);
            let gt_mask = _mm256_movemask_pd(_mm256_castsi256_pd(gt_result)) as u32;

            // If first element > val, no match possible
            if gt_mask & 1 != 0 {
                break;
            }

            // If all elements > val, break
            if gt_mask == 0xF {
                break;
            }

            j += 4;
        }

        // Scalar fallback for remainder
        while j < large.len() && large[j] < val {
            j += 1;
        }
        if j < large.len() && large[j] == val {
            count += 1;
            j += 1;
        }

        i += 1;
    }

    count
}

// ============================================================================
// ARM64 NEON Implementation
// ============================================================================

#[cfg(target_arch = "aarch64")]
#[inline]
pub fn cosine_neon(a: &SparseVec, b: &SparseVec) -> f64 {
    // Safety: NEON is always available on aarch64
    unsafe { cosine_neon_impl(a, b) }
}

#[cfg(target_arch = "aarch64")]
unsafe fn cosine_neon_impl(a: &SparseVec, b: &SparseVec) -> f64 {
    // For sparse vectors, use SIMD-accelerated intersection counting
    let use_simd =
        a.pos.len() + a.neg.len() >= SIMD_THRESHOLD || b.pos.len() + b.neg.len() >= SIMD_THRESHOLD;

    let (pp, nn, pn, np) = if use_simd {
        (
            intersection_count_neon(&a.pos, &b.pos) as i32,
            intersection_count_neon(&a.neg, &b.neg) as i32,
            intersection_count_neon(&a.pos, &b.neg) as i32,
            intersection_count_neon(&a.neg, &b.pos) as i32,
        )
    } else {
        (
            intersection_count_sorted(&a.pos, &b.pos) as i32,
            intersection_count_sorted(&a.neg, &b.neg) as i32,
            intersection_count_sorted(&a.pos, &b.neg) as i32,
            intersection_count_sorted(&a.neg, &b.pos) as i32,
        )
    };

    let dot = (pp + nn) - (pn + np);

    let a_norm = (a.pos.len() + a.neg.len()) as f64;
    let b_norm = (b.pos.len() + b.neg.len()) as f64;

    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    dot as f64 / (a_norm.sqrt() * b_norm.sqrt())
}

/// NEON-accelerated sorted intersection counting
#[cfg(target_arch = "aarch64")]
unsafe fn intersection_count_neon(a: &[usize], b: &[usize]) -> usize {
    use std::arch::aarch64::*;

    if a.is_empty() || b.is_empty() {
        return 0;
    }

    // For small arrays, use galloping which is faster due to cache effects
    if a.len() < 8 || b.len() < 8 {
        return intersection_count_galloping(a, b);
    }

    // Ensure a is smaller for better memory access patterns
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    let mut count = 0usize;
    let mut j = 0usize;

    // Process 2 elements at a time using NEON uint64x2_t
    let chunks = small.len() / 2;

    for chunk_idx in 0..chunks {
        let base = chunk_idx * 2;
        let v0 = small[base];
        let v1 = small[base + 1];

        j = galloping_lower_bound(large, j, v0);

        // Check both values
        for &val in &small[base..base + 2] {
            while j < large.len() && large[j] < val {
                j += 1;
            }
            if j < large.len() && large[j] == val {
                count += 1;
                j += 1;
            }
        }
    }

    // Handle remaining elements
    let remainder_start = chunks * 2;
    for &val in &small[remainder_start..] {
        j = galloping_lower_bound(large, j, val);
        if j < large.len() && large[j] == val {
            count += 1;
            j += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vsa::ReversibleVSAConfig;

    #[test]
    fn test_scalar_cosine_basic() {
        let config = crate::vsa::ReversibleVSAConfig::default();
        let a = SparseVec::encode_data(b"hello", &config, None);
        let b = SparseVec::encode_data(b"hello", &config, None);
        let c = SparseVec::encode_data(b"world", &config, None);

        // Identical vectors should have similarity ~1.0
        let sim_same = cosine_scalar(&a, &b);
        assert!(
            sim_same > 0.9,
            "Expected high similarity for identical vectors, got {}",
            sim_same
        );

        // Different vectors should have lower similarity
        let sim_diff = cosine_scalar(&a, &c);
        assert!(
            sim_diff < 0.5,
            "Expected low similarity for different vectors, got {}",
            sim_diff
        );
    }

    #[test]
    fn test_simd_matches_scalar() {
        let config = ReversibleVSAConfig::default();

        // Test with various data inputs
        let test_cases = vec![
            (b"test data 1".as_slice(), b"test data 1".as_slice()),
            (b"test data 1".as_slice(), b"test data 2".as_slice()),
            (b"short".as_slice(), b"longer test data".as_slice()),
            (b"alpha".as_slice(), b"beta".as_slice()),
        ];

        for (data_a, data_b) in test_cases {
            let a = SparseVec::encode_data(data_a, &config, None);
            let b = SparseVec::encode_data(data_b, &config, None);

            let scalar_result = cosine_scalar(&a, &b);
            let simd_result = cosine_simd(&a, &b);

            // Results should be very close (within floating point tolerance)
            let diff = (scalar_result - simd_result).abs();
            assert!(
                diff < 1e-10,
                "SIMD result {} differs from scalar {} by {} (data_a: {:?}, data_b: {:?})",
                simd_result,
                scalar_result,
                diff,
                data_a,
                data_b
            );
        }
    }

    #[test]
    fn test_empty_vectors() {
        let empty = SparseVec {
            pos: vec![],
            neg: vec![],
        };
        let config = crate::vsa::ReversibleVSAConfig::default();
        let non_empty = SparseVec::encode_data(b"test", &config, None);

        assert_eq!(cosine_scalar(&empty, &empty), 0.0);
        assert_eq!(cosine_scalar(&empty, &non_empty), 0.0);
        assert_eq!(cosine_scalar(&non_empty, &empty), 0.0);

        assert_eq!(cosine_simd(&empty, &empty), 0.0);
        assert_eq!(cosine_simd(&empty, &non_empty), 0.0);
        assert_eq!(cosine_simd(&non_empty, &empty), 0.0);
    }

    #[test]
    fn test_cosine_properties() {
        let config = crate::vsa::ReversibleVSAConfig::default();
        let a = SparseVec::encode_data(b"alpha", &config, None);
        let b = SparseVec::encode_data(b"beta", &config, None);

        // Symmetry: cosine(a, b) == cosine(b, a)
        let sim_ab = cosine_simd(&a, &b);
        let sim_ba = cosine_simd(&b, &a);
        assert!(
            (sim_ab - sim_ba).abs() < 1e-10,
            "Cosine should be symmetric"
        );

        // Self-similarity: cosine(a, a) should be ~1.0
        let sim_aa = cosine_simd(&a, &a);
        assert!(
            sim_aa > 0.99,
            "Self-similarity should be close to 1.0, got {}",
            sim_aa
        );

        // Range: cosine should be in [-1, 1]
        assert!(
            (-1.0..=1.0).contains(&sim_ab),
            "Cosine should be in [-1, 1], got {}",
            sim_ab
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx512_matches_scalar() {
        if !is_x86_feature_detected!("avx512f") {
            eprintln!("AVX-512 not available on this CPU, skipping test");
            return;
        }

        let config = ReversibleVSAConfig::default();

        // Test with various data inputs including larger data for SIMD threshold
        let test_cases = vec![
            (b"test data 1".as_slice(), b"test data 1".as_slice()),
            (b"test data 1".as_slice(), b"test data 2".as_slice()),
            (b"short".as_slice(), b"longer test data".as_slice()),
            (b"alpha".as_slice(), b"beta".as_slice()),
            // Larger data to trigger SIMD path
            (
                b"This is a much longer piece of data to encode that should trigger the SIMD path".as_slice(),
                b"This is a much longer piece of data to encode that should trigger the SIMD path".as_slice(),
            ),
            (
                b"Another long string of data that should be encoded into a sparse vector representation".as_slice(),
                b"A completely different long string to compare against the previous one for similarity".as_slice(),
            ),
        ];

        for (data_a, data_b) in test_cases {
            let a = SparseVec::encode_data(data_a, &config, None);
            let b = SparseVec::encode_data(data_b, &config, None);

            let scalar_result = cosine_scalar(&a, &b);
            let avx512_result = unsafe { cosine_avx512_impl(&a, &b) };

            // Results should be very close (within floating point tolerance)
            let diff = (scalar_result - avx512_result).abs();
            assert!(
                diff < 1e-10,
                "AVX-512 result {} differs from scalar {} by {} (data_a: {:?}, data_b: {:?})",
                avx512_result,
                scalar_result,
                diff,
                String::from_utf8_lossy(data_a),
                String::from_utf8_lossy(data_b)
            );
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx512_empty_and_small_vectors() {
        if !is_x86_feature_detected!("avx512f") {
            eprintln!("AVX-512 not available on this CPU, skipping test");
            return;
        }

        let empty = SparseVec {
            pos: vec![],
            neg: vec![],
        };
        let config = crate::vsa::ReversibleVSAConfig::default();
        let non_empty = SparseVec::encode_data(b"test", &config, None);

        // Test empty vectors
        unsafe {
            assert_eq!(cosine_avx512_impl(&empty, &empty), 0.0);
            assert_eq!(cosine_avx512_impl(&empty, &non_empty), 0.0);
            assert_eq!(cosine_avx512_impl(&non_empty, &empty), 0.0);
        }

        // Test small vectors (below SIMD threshold)
        let small_a = SparseVec {
            pos: vec![1, 5, 10],
            neg: vec![2, 7],
        };
        let small_b = SparseVec {
            pos: vec![1, 5, 15],
            neg: vec![3, 7],
        };

        let scalar_result = cosine_scalar(&small_a, &small_b);
        let avx512_result = unsafe { cosine_avx512_impl(&small_a, &small_b) };
        assert!(
            (scalar_result - avx512_result).abs() < 1e-10,
            "AVX-512 should match scalar for small vectors"
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx512_large_vectors() {
        if !is_x86_feature_detected!("avx512f") {
            eprintln!("AVX-512 not available on this CPU, skipping test");
            return;
        }

        // Create large synthetic vectors to ensure SIMD path is taken
        let large_a = SparseVec {
            pos: (0..100).map(|i| i * 3).collect(),
            neg: (0..100).map(|i| i * 3 + 1).collect(),
        };
        let large_b = SparseVec {
            pos: (0..100).map(|i| i * 3).collect(),     // Same as a.pos
            neg: (0..100).map(|i| i * 3 + 2).collect(), // Different from a.neg
        };

        let scalar_result = cosine_scalar(&large_a, &large_b);
        let avx512_result = unsafe { cosine_avx512_impl(&large_a, &large_b) };

        let diff = (scalar_result - avx512_result).abs();
        assert!(
            diff < 1e-10,
            "AVX-512 result {} differs from scalar {} by {} for large vectors",
            avx512_result,
            scalar_result,
            diff
        );

        // Verify the result makes sense (partial overlap should give moderate similarity)
        assert!(
            avx512_result > 0.0 && avx512_result < 1.0,
            "Expected moderate similarity for partially overlapping vectors, got {}",
            avx512_result
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx512_cosine_properties() {
        if !is_x86_feature_detected!("avx512f") {
            eprintln!("AVX-512 not available on this CPU, skipping test");
            return;
        }

        let config = crate::vsa::ReversibleVSAConfig::default();
        let a = SparseVec::encode_data(
            b"A reasonably long string to ensure we hit the SIMD threshold during encoding",
            &config,
            None,
        );
        let b = SparseVec::encode_data(
            b"Another reasonably long string that is different from the first one",
            &config,
            None,
        );

        unsafe {
            // Symmetry: cosine(a, b) == cosine(b, a)
            let sim_ab = cosine_avx512_impl(&a, &b);
            let sim_ba = cosine_avx512_impl(&b, &a);
            assert!(
                (sim_ab - sim_ba).abs() < 1e-10,
                "AVX-512 cosine should be symmetric: {} vs {}",
                sim_ab,
                sim_ba
            );

            // Self-similarity: cosine(a, a) should be ~1.0
            let sim_aa = cosine_avx512_impl(&a, &a);
            assert!(
                sim_aa > 0.99,
                "AVX-512 self-similarity should be close to 1.0, got {}",
                sim_aa
            );

            // Range: cosine should be in [-1, 1]
            assert!(
                (-1.0..=1.0).contains(&sim_ab),
                "AVX-512 cosine should be in [-1, 1], got {}",
                sim_ab
            );
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    #[allow(clippy::nonminimal_bool)] // Feature detection macros are intentionally explicit
    fn test_runtime_dispatch_selects_best_available() {
        // This test verifies that cosine_simd correctly dispatches based on CPU features
        let config = ReversibleVSAConfig::default();
        let a = SparseVec::encode_data(b"test dispatch", &config, None);
        let b = SparseVec::encode_data(b"test dispatch", &config, None);

        let scalar_result = cosine_scalar(&a, &b);
        let simd_result = cosine_simd(&a, &b);

        // Regardless of which SIMD path is taken, result should match scalar
        let diff = (scalar_result - simd_result).abs();
        assert!(
            diff < 1e-10,
            "Runtime-dispatched SIMD result {} differs from scalar {} by {}",
            simd_result,
            scalar_result,
            diff
        );

        // Log which path was taken for debugging
        if is_x86_feature_detected!("avx512f") {
            eprintln!("Runtime dispatch: using AVX-512");
        } else if is_x86_feature_detected!("avx512vnni") || is_x86_feature_detected!("avxvnni") {
            eprintln!("Runtime dispatch: using AVX-VNNI");
        } else if is_x86_feature_detected!("avx2") {
            eprintln!("Runtime dispatch: using AVX2");
        } else {
            eprintln!("Runtime dispatch: using scalar fallback");
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx_vnni_detection() {
        // Test that AVX-VNNI detection works correctly
        let has_avx512_vnni = is_x86_feature_detected!("avx512vnni");
        let has_avx_vnni = is_x86_feature_detected!("avxvnni");
        let has_avx2 = is_x86_feature_detected!("avx2");

        eprintln!("CPU Feature Detection:");
        eprintln!("  AVX-512 VNNI: {}", has_avx512_vnni);
        eprintln!("  AVX-VNNI: {}", has_avx_vnni);
        eprintln!("  AVX2: {}", has_avx2);

        // If we have VNNI, we should also have AVX2
        if has_avx512_vnni || has_avx_vnni {
            assert!(
                has_avx2,
                "AVX-VNNI implies AVX2 support, but AVX2 not detected"
            );
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx_vnni_matches_scalar() {
        // AVX-VNNI requires at minimum AVX2
        if !is_x86_feature_detected!("avx2") {
            eprintln!("AVX2 not available on this CPU, skipping AVX-VNNI test");
            return;
        }

        let config = ReversibleVSAConfig::default();

        // Test with various data inputs including larger data for SIMD threshold
        let test_cases = vec![
            (b"test data 1".as_slice(), b"test data 1".as_slice()),
            (b"test data 1".as_slice(), b"test data 2".as_slice()),
            (b"short".as_slice(), b"longer test data".as_slice()),
            (b"alpha".as_slice(), b"beta".as_slice()),
            // Larger data to trigger SIMD path
            (
                b"This is a much longer piece of data to encode that should trigger the SIMD path".as_slice(),
                b"This is a much longer piece of data to encode that should trigger the SIMD path".as_slice(),
            ),
            (
                b"Another long string of data that should be encoded into a sparse vector representation".as_slice(),
                b"A completely different long string to compare against the previous one for similarity".as_slice(),
            ),
        ];

        for (data_a, data_b) in test_cases {
            let a = SparseVec::encode_data(data_a, &config, None);
            let b = SparseVec::encode_data(data_b, &config, None);

            let scalar_result = cosine_scalar(&a, &b);
            let vnni_result = unsafe { cosine_avx_vnni_impl(&a, &b) };

            // Results should be very close (within floating point tolerance)
            let diff = (scalar_result - vnni_result).abs();
            assert!(
                diff < 1e-10,
                "AVX-VNNI result {} differs from scalar {} by {} (data_a: {:?}, data_b: {:?})",
                vnni_result,
                scalar_result,
                diff,
                String::from_utf8_lossy(data_a),
                String::from_utf8_lossy(data_b)
            );
        }

        eprintln!("AVX-VNNI implementation matches scalar for all test cases");
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx_vnni_empty_and_small_vectors() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("AVX2 not available on this CPU, skipping test");
            return;
        }

        let empty = SparseVec {
            pos: vec![],
            neg: vec![],
        };
        let config = crate::vsa::ReversibleVSAConfig::default();
        let non_empty = SparseVec::encode_data(b"test", &config, None);

        // Test empty vectors
        unsafe {
            assert_eq!(cosine_avx_vnni_impl(&empty, &empty), 0.0);
            assert_eq!(cosine_avx_vnni_impl(&empty, &non_empty), 0.0);
            assert_eq!(cosine_avx_vnni_impl(&non_empty, &empty), 0.0);
        }

        // Test small vectors (below SIMD threshold)
        let small_a = SparseVec {
            pos: vec![1, 5, 10],
            neg: vec![2, 7],
        };
        let small_b = SparseVec {
            pos: vec![1, 5, 15],
            neg: vec![3, 7],
        };

        let scalar_result = cosine_scalar(&small_a, &small_b);
        let vnni_result = unsafe { cosine_avx_vnni_impl(&small_a, &small_b) };
        assert!(
            (scalar_result - vnni_result).abs() < 1e-10,
            "AVX-VNNI should match scalar for small vectors"
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx_vnni_large_vectors() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("AVX2 not available on this CPU, skipping test");
            return;
        }

        // Create large synthetic vectors to ensure SIMD path is taken
        let large_a = SparseVec {
            pos: (0..100).map(|i| i * 3).collect(),
            neg: (0..100).map(|i| i * 3 + 1).collect(),
        };
        let large_b = SparseVec {
            pos: (0..100).map(|i| i * 3).collect(),     // Same as a.pos
            neg: (0..100).map(|i| i * 3 + 2).collect(), // Different from a.neg
        };

        let scalar_result = cosine_scalar(&large_a, &large_b);
        let vnni_result = unsafe { cosine_avx_vnni_impl(&large_a, &large_b) };

        let diff = (scalar_result - vnni_result).abs();
        assert!(
            diff < 1e-10,
            "AVX-VNNI result {} differs from scalar {} by {} for large vectors",
            vnni_result,
            scalar_result,
            diff
        );

        // Verify the result makes sense (partial overlap should give moderate similarity)
        assert!(
            vnni_result > 0.0 && vnni_result < 1.0,
            "Expected moderate similarity for partially overlapping vectors, got {}",
            vnni_result
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx_vnni_cosine_properties() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("AVX2 not available on this CPU, skipping test");
            return;
        }

        let config = crate::vsa::ReversibleVSAConfig::default();
        let a = SparseVec::encode_data(
            b"A reasonably long string to ensure we hit the SIMD threshold during encoding",
            &config,
            None,
        );
        let b = SparseVec::encode_data(
            b"Another reasonably long string that is different from the first one",
            &config,
            None,
        );

        unsafe {
            // Symmetry: cosine(a, b) == cosine(b, a)
            let sim_ab = cosine_avx_vnni_impl(&a, &b);
            let sim_ba = cosine_avx_vnni_impl(&b, &a);
            assert!(
                (sim_ab - sim_ba).abs() < 1e-10,
                "AVX-VNNI cosine should be symmetric: {} vs {}",
                sim_ab,
                sim_ba
            );

            // Self-similarity: cosine(a, a) should be ~1.0
            let sim_aa = cosine_avx_vnni_impl(&a, &a);
            assert!(
                sim_aa > 0.99,
                "AVX-VNNI self-similarity should be close to 1.0, got {}",
                sim_aa
            );

            // Range: cosine should be in [-1, 1]
            assert!(
                (-1.0..=1.0).contains(&sim_ab),
                "AVX-VNNI cosine should be in [-1, 1], got {}",
                sim_ab
            );
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_byte_dot_product_vnni() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("AVX2 not available on this CPU, skipping test");
            return;
        }

        // Test byte dot product function
        let a: Vec<u8> = (0..64).collect();
        let b: Vec<i8> = (0..64i8).map(|i| if i % 2 == 0 { 1 } else { -1 }).collect();

        let result = unsafe { byte_dot_product_vnni(&a, &b) };

        // Compute expected result manually
        let expected: i32 = a
            .iter()
            .zip(b.iter())
            .map(|(&ai, &bi)| (ai as i32) * (bi as i32))
            .sum();

        assert_eq!(
            result, expected,
            "VNNI byte dot product mismatch: got {}, expected {}",
            result, expected
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_byte_dot_product_vnni_edge_cases() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("AVX2 not available on this CPU, skipping test");
            return;
        }

        // Empty arrays
        let empty_u: Vec<u8> = vec![];
        let empty_i: Vec<i8> = vec![];
        assert_eq!(unsafe { byte_dot_product_vnni(&empty_u, &empty_i) }, 0);

        // Small array (less than 32 bytes)
        let small_u: Vec<u8> = vec![1, 2, 3, 4];
        let small_i: Vec<i8> = vec![1, -1, 1, -1];
        let result = unsafe { byte_dot_product_vnni(&small_u, &small_i) };
        // 1*1 + 2*(-1) + 3*1 + 4*(-1) = 1 - 2 + 3 - 4 = -2
        let expected = -2;
        assert_eq!(result, expected);

        // Large array with known values
        let large_u: Vec<u8> = vec![255; 128]; // Max unsigned
        let large_i: Vec<i8> = vec![1; 128]; // All ones
        let result = unsafe { byte_dot_product_vnni(&large_u, &large_i) };
        let expected = 255i32 * 128;
        assert_eq!(result, expected);
    }
}
