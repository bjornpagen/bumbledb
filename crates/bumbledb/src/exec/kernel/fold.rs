use super::gather::biased_to_i64;
#[cfg(target_arch = "aarch64")]
use super::neon;

/// Contiguous strided sum of biased-i64 words over
/// `values[offset], values[offset + stride], ..` for `count` elements —
/// the dense-survivor fast form (no index loads). Stride 1 takes the
/// NEON carry-count path through the bias identity:
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
    assert!(stride > 0 && (count == 0 || (count - 1) * stride + offset < values.len()));
    #[cfg(target_arch = "aarch64")]
    if stride == 1 {
        let total = neon::fold_sum_u64_dense(&values[offset..offset + count]);
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
/// Stride 1 takes the NEON carry-count path.
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
    assert!(stride > 0 && (count == 0 || (count - 1) * stride + offset < values.len()));
    #[cfg(target_arch = "aarch64")]
    if stride == 1 {
        return neon::fold_sum_u64_dense(&values[offset..offset + count]);
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

/// Contiguous strided (min, max) in one pass. Stride 1 takes the NEON
/// lane path on aarch64 (`vcgtq_u64` + `vbslq_u64` — the compare-select
/// pair; there is no 64-bit lane min/max instruction).
///
/// # Panics
///
/// Only on a programmer-invariant violation: zero `count` or the strided
/// extent exceeding `values`.
#[must_use]
pub fn fold_min_max_u64(values: &[u64], stride: usize, offset: usize, count: usize) -> (u64, u64) {
    assert!(count > 0 && stride > 0 && (count - 1) * stride + offset < values.len());
    #[cfg(target_arch = "aarch64")]
    if stride == 1 {
        return neon::fold_min_max_u64_dense(&values[offset..offset + count]);
    }
    fold_min_max_u64_strided(values, stride, offset, count)
}

/// The scalar strided (min, max) — the live path wherever the NEON dense
/// form does not apply.
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
