use std::simd::prelude::*;

use super::gather::biased_to_i64;

/// The strided extent guard the fold kernels' `get_unchecked` bodies
/// cite: `(count − 1) · stride + offset < len`, computed checked so the
/// guard is total over the input type — a wrapping product cannot
/// forge an in-bounds extent in release (overflow-checks default off
/// there; the guard IS the safety invariant, so it must not wrap).
fn strided_extent_in(len: usize, stride: usize, offset: usize, count: usize) -> bool {
    stride > 0
        && count.checked_sub(1).is_none_or(|c| {
            c.checked_mul(stride)
                .and_then(|span| span.checked_add(offset))
                .is_some_and(|last| last < len)
        })
}

/// Contiguous strided sum of biased-i64 words over
/// `values[offset], values[offset + stride], ..` for `count` elements —
/// the dense-survivor fast form (no index loads). Stride 1 takes the
/// lane carry-count path through the bias identity:
/// each biased word is `value + 2^63 (mod 2^64)`, so
/// `Σ value = Σ word − count·2^63` exactly in i128.
///
/// # Panics
///
/// Only on a programmer-invariant violation: the strided extent
/// exceeding `values`.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn fold_sum_biased_i64(values: &[u64], stride: usize, offset: usize, count: usize) -> i128 {
    assert!(strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        let total = fold_sum_u64_dense(&values[offset..offset + count]);
        let bias = u128::from(count as u64) << 63;
        // Both fit i128 comfortably: total < count·2^64 ≤ 2^96-ish.
        return i128::try_from(total).expect("sum of u32-counted words fits i128")
            - i128::try_from(bias).expect("bias fits i128");
    }
    let mut acc = [0i128; 4];
    let mut i = 0;
    while i + 4 <= count {
        for (lane, slot) in acc.iter_mut().enumerate() {
            // SAFETY: the extent assert above covers every index.
            let word = unsafe { *values.get_unchecked((i + lane) * stride + offset) };
            *slot += i128::from(biased_to_i64(word));
        }
        i += 4;
    }
    while i < count {
        let word = unsafe { *values.get_unchecked(i * stride + offset) };
        acc[0] += i128::from(biased_to_i64(word));
        i += 1;
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Contiguous strided sum of u64 words (see [`fold_sum_biased_i64`]).
/// Stride 1 takes the lane carry-count path.
///
/// # Panics
///
/// Only on a programmer-invariant violation: the strided extent
/// exceeding `values`.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn fold_sum_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> u128 {
    assert!(strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        return fold_sum_u64_dense(&values[offset..offset + count]);
    }
    let mut acc = [0u128; 4];
    let mut i = 0;
    while i + 4 <= count {
        for (lane, slot) in acc.iter_mut().enumerate() {
            // SAFETY: the extent assert above covers every index.
            let word = unsafe { *values.get_unchecked((i + lane) * stride + offset) };
            *slot += u128::from(word);
        }
        i += 4;
    }
    while i < count {
        let word = unsafe { *values.get_unchecked(i * stride + offset) };
        acc[0] += u128::from(word);
        i += 1;
    }
    acc[0] + acc[1] + acc[2] + acc[3]
}

/// Contiguous strided (min, max) in one pass. Stride 1 takes the lane
/// path (`simd_min`/`simd_max` — on aarch64 the compare-select pair;
/// there is no 64-bit lane min/max instruction).
///
/// # Panics
///
/// Only on a programmer-invariant violation: zero `count` or the strided
/// extent exceeding `values`.
#[must_use]
pub fn fold_min_max_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> (u64, u64) {
    assert!(count > 0 && strided_extent_in(values.len(), stride, offset, count));
    if stride == 1 {
        return fold_min_max_u64_dense(&values[offset..offset + count]);
    }
    fold_min_max_u64_strided(values, stride, offset, count)
}

/// Dense exact-u128 sum via carry counting (`std::simd`, all targets —
/// PRD 03's Q2 fold verdict: measured parity with the retired NEON body
/// at 7.9–8.0 rows/ns, `unsafe` deleted): four 2-lane accumulators take
/// wrapping lane adds while a parallel counter lane counts carries —
/// unsigned overflow iff `new < old`, i.e. `old.simd_gt(new)` all-ones,
/// subtracted to count +1. Total = Σ lane lo + (Σ lane carries << 64):
/// exact, and bit-identical to any-association i128/u128 folding. Flag
/// ports untouched — the scalar exact loop's `adds/adcs` are confined
/// to 3 of 6 ALUs, which was the whole wall (the port-topology law).
///
/// Refuted (2026-07-16, interleaved A/B at 937abc33): the copy-free
/// twin — carry compare `v.simd_gt(new)`, the same overflow truth
/// (`new < old` iff `new < v`) — deletes all four of this loop's
/// `mov.16b` accumulator copies (21→17 insns, vector ALU µops 16→12
/// per 8-word iteration, sinks bit-identical) yet measured NEUTRAL:
/// the L2-tier kernel falsifier read 1.009 twin/base, inside the ±2%
/// band, and no family moved. The kernel already saturates its
/// ~7.9–8.0 rows/ns ceiling at L2; the copies are L1-relevance only,
/// and no L1-resident fold lane exists to see them — re-open with one
/// first. The W5 gravestone commit carries the full protocol.
fn fold_sum_u64_dense(values: &[u64]) -> u128 {
    let mut lows = [Simd::<u64, 2>::splat(0); 4];
    let mut carries = [Simd::<u64, 2>::splat(0); 4];
    let (chunks, tail) = values.as_chunks::<8>();
    for chunk in chunks {
        for lane in 0..4 {
            let v = Simd::<u64, 2>::from_slice(&chunk[lane * 2..lane * 2 + 2]);
            let new = lows[lane] + v;
            // Overflowed lanes read all-ones; subtracting adds 1.
            let carry = lows[lane].simd_gt(new).to_simd().cast::<u64>();
            carries[lane] -= carry;
            lows[lane] = new;
        }
    }
    let mut total: u128 = 0;
    for lane in 0..4 {
        for half in 0..2 {
            total += u128::from(lows[lane].as_array()[half])
                + (u128::from(carries[lane].as_array()[half]) << 64);
        }
    }
    for &v in tail {
        total += u128::from(v);
    }
    total
}

/// Dense (min, max) over a contiguous u64 slice (`std::simd`, all
/// targets — measured parity with the retired NEON body): compare-select
/// lanes, four vector accumulators to break the dependency chains,
/// scalar tail.
fn fold_min_max_u64_dense(values: &[u64]) -> (u64, u64) {
    let mut mins = [Simd::<u64, 2>::splat(u64::MAX); 4];
    let mut maxs = [Simd::<u64, 2>::splat(u64::MIN); 4];
    let (chunks, tail) = values.as_chunks::<8>();
    for chunk in chunks {
        for lane in 0..4 {
            let v = Simd::<u64, 2>::from_slice(&chunk[lane * 2..lane * 2 + 2]);
            mins[lane] = mins[lane].simd_min(v);
            maxs[lane] = maxs[lane].simd_max(v);
        }
    }
    let mut min_scalar = u64::MAX;
    let mut max_scalar = u64::MIN;
    for lane in 0..4 {
        min_scalar = min_scalar.min(mins[lane].reduce_min());
        max_scalar = max_scalar.max(maxs[lane].reduce_max());
    }
    for &v in tail {
        min_scalar = min_scalar.min(v);
        max_scalar = max_scalar.max(v);
    }
    (min_scalar, max_scalar)
}

/// The scalar strided (min, max) — the live path wherever the dense
/// lane form does not apply.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn fold_min_max_u64_strided(
    values: &[u64],
    stride: usize,
    offset: usize,
    count: usize,
) -> (u64, u64) {
    let mut mins = [u64::MAX; 4];
    let mut maxs = [u64::MIN; 4];
    let mut i = 0;
    while i + 4 <= count {
        for lane in 0..4 {
            // SAFETY: the caller asserted the strided extent.
            let word = unsafe { *values.get_unchecked((i + lane) * stride + offset) };
            mins[lane] = mins[lane].min(word);
            maxs[lane] = maxs[lane].max(word);
        }
        i += 4;
    }
    while i < count {
        let word = unsafe { *values.get_unchecked(i * stride + offset) };
        mins[0] = mins[0].min(word);
        maxs[0] = maxs[0].max(word);
        i += 1;
    }
    (
        mins.iter().copied().min().expect("four lanes"),
        maxs.iter().copied().max().expect("four lanes"),
    )
}
