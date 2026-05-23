struct Timed<T> {
    value: T,
    elapsed: Duration,
}

fn timed<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<Timed<T>, E> {
    let start = Instant::now();
    let value = f()?;
    Ok(Timed {
        value,
        elapsed: start.elapsed(),
    })
}

fn timed_samples<E>(samples: u64, mut f: impl FnMut() -> Result<(), E>) -> Result<TimingStats, E> {
    let mut durations = Vec::with_capacity(samples.min(usize::MAX as u64) as usize);
    for _ in 0..samples {
        let start = Instant::now();
        f()?;
        durations.push(start.elapsed());
    }
    Ok(TimingStats::from_samples(durations))
}

fn timed_bumbledb_samples<E>(
    samples: u64,
    mut f: impl FnMut() -> Result<QueryPlan, E>,
) -> Result<(TimingStats, CacheHitStats), E> {
    let mut durations = Vec::with_capacity(samples.min(usize::MAX as u64) as usize);
    let mut cache_hits = CacheHitStats::default();
    for _ in 0..samples {
        let start = Instant::now();
        let plan = f()?;
        durations.push(start.elapsed());
        if plan.query_image_cache.hits > 0 {
            cache_hits.query_image_cache_hits += 1;
        }
    }
    Ok((TimingStats::from_samples(durations), cache_hits))
}

fn duration_avg(duration: Duration, samples: u64) -> Duration {
    if samples == 0 {
        return Duration::ZERO;
    }
    let nanos = duration.as_nanos() / u128::from(samples);
    Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64)
}

fn percentile(samples: &[Duration], percentile: u64) -> Duration {
    let index = ((samples.len() as u64 - 1) * percentile).div_ceil(100) as usize;
    samples[index]
}

