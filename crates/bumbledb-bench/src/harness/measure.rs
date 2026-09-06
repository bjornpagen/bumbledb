use std::time::Instant;

use super::stats::{normalized_p50, stats};
use super::{Measurement, Modes, Protocol};

/// # Errors
pub fn measure<F>(proto: Protocol, f: F) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    measure_batched(proto, Modes::default(), 1, f)
}

/// Work counts sum across every call; batch 1 is the plain protocol.
/// # Errors
/// # Panics
pub fn measure_batched<F>(
    proto: Protocol,
    modes: Modes,
    batch: u32,
    f: F,
) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    measure_interleaved(proto, modes, batch, || (), f)
}

/// `between` runs before each warmup and measured batch, outside the timer.
/// # Errors
/// # Panics
pub fn measure_interleaved<B, F>(
    proto: Protocol,
    modes: Modes,
    batch: u32,
    mut between: B,
    mut f: F,
) -> Result<Measurement, String>
where
    B: FnMut(),
    F: FnMut() -> Result<u64, String>,
{
    assert!(batch >= 1, "a zero batch measures nothing");
    if modes.alloc_window && !cfg!(feature = "alloc-counter") {
        return Err(
            "the alloc window needs the alloc-counter feature (bumbledb/alloc-counter)".to_owned(),
        );
    }
    for _ in 0..proto.warmups {
        between();
        std::hint::black_box(f()?);
    }
    if modes.alloc_window {
        bumbledb::alloc_counter::reset();
    }
    let mut samples = Vec::with_capacity(proto.samples as usize);
    let mut sample_ghz = modes
        .proxy_per_rep
        .then(|| Vec::with_capacity(proto.samples as usize));
    let mut work = 0u64;
    for _ in 0..proto.samples {
        let mut count = 0u64;
        between();
        let start = Instant::now();
        for _ in 0..batch {
            count += f()?;
        }
        let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        samples.push(elapsed / u64::from(batch));
        if let Some(ghz) = &mut sample_ghz {
            ghz.push(crate::clockproxy::effective_ghz());
        }
        work += std::hint::black_box(count);
    }

    let alloc = modes.alloc_window.then(bumbledb::alloc_counter::snapshot);

    let p50_norm = sample_ghz.as_ref().map(|ghz| normalized_p50(&samples, ghz));
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        p50_norm,
        alloc,
    })
}
