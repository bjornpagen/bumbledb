use super::Stats;

/// Sorts the samples in place and takes **nearest-rank** percentiles:
/// `idx = ceil(p/100 × n) - 1` over the ascending sort (so p50 of
/// `[10, 20]` is 10, p99 of 100 samples is the 99th). `mean_ns` is the
/// integer mean. Reproducible by hand — no interpolation.
///
/// # Panics
///
/// On an empty sample vector (a programmer error — protocols demand at
/// least one sample).
#[must_use]
pub fn stats(samples: &mut [u64]) -> Stats {
    assert!(!samples.is_empty(), "stats over zero samples");
    samples.sort_unstable();
    let n = samples.len() as u64;
    let rank = |p: u64| {
        let idx = (p * n).div_ceil(100) - 1;
        samples[usize::try_from(idx).expect("index fits")]
    };
    Stats {
        min: samples[0],
        p50: rank(50),
        p90: rank(90),
        p95: rank(95),
        p99: rank(99),
        max: samples[samples.len() - 1],
        mean_ns: samples.iter().sum::<u64>() / n,
    }
}

/// The per-rep normalization (docs/silicon2/00): each sample's elapsed
/// time is rescaled to the cohort's best observed clock
/// (`ns × ghz / ghz_ref`), so a sample that ran slow only because the
/// clock was low stops hiding structural findings — and a sample that
/// is genuinely slow stays slow. Returns the normalized p50.
///
/// # Panics
///
/// On mismatched lengths (a programmer error).
#[must_use]
pub fn normalized_p50(samples_ns: &[u64], ghz: &[f64]) -> u64 {
    assert_eq!(samples_ns.len(), ghz.len());
    let ghz_ref = ghz.iter().copied().fold(f64::MIN, f64::max);
    let mut normalized: Vec<u64> = samples_ns
        .iter()
        .zip(ghz)
        .map(|(&ns, &g)| {
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            {
                (ns as f64 * g / ghz_ref) as u64
            }
        })
        .collect();
    stats(&mut normalized).p50
}
