use std::time::Instant;

use super::stats::stats;
use super::{Measurement, Protocol};

pub fn measure<F>(proto: Protocol, f: F) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    measure_batched(proto, 1, f)
}

/// Work counts sum across every call; batch 1 is the plain protocol.
pub fn measure_batched<F>(proto: Protocol, batch: u32, f: F) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    measure_interleaved(proto, batch, || (), f)
}

/// `between` runs before each warmup and measured batch, outside the timer.
/// # Panics
/// On a zero batch.
pub fn measure_interleaved<B, F>(
    proto: Protocol,
    batch: u32,
    mut between: B,
    mut f: F,
) -> Result<Measurement, String>
where
    B: FnMut(),
    F: FnMut() -> Result<u64, String>,
{
    assert!(batch >= 1, "a zero batch measures nothing");
    for _ in 0..proto.warmups {
        between();
        std::hint::black_box(f()?);
    }
    let mut samples = Vec::with_capacity(proto.samples as usize);
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
        work += std::hint::black_box(count);
    }
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
    })
}
