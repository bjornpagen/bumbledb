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
