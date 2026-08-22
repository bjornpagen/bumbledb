use super::Stats;

/// # Panics
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

/// # Panics
#[must_use]
pub fn normalized_p50(samples_ns: &[u64], ghz: &[f64]) -> u64 {
    assert_eq!(samples_ns.len(), ghz.len());
    let ghz_ref = ghz.iter().copied().fold(f64::MIN, f64::max);
    let mut normalized: Vec<u64> = samples_ns
        .iter()
        .zip(ghz)
        .map(|(&ns, &g)| {
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "normalized timing arithmetic accepts the bounded float conversion"
            )]
            {
                (ns as f64 * g / ghz_ref) as u64
            }
        })
        .collect();
    stats(&mut normalized).p50
}
