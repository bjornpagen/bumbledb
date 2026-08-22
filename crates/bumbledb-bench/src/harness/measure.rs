use std::time::Instant;

use super::stats::{normalized_p50, stats};
use super::traced::traced_sample;
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

/// `between` work runs once per timed sample (before the batch), not
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
    if modes.alloc_window && modes.trace {
        return Err("alloc-window and trace-capture are mutually exclusive modes".to_owned());
    }
    if modes.alloc_window {
        alloc_window::require()?;
    }
    for _ in 0..proto.warmups {
        between();
        std::hint::black_box(f()?);
    }
    if modes.alloc_window {
        alloc_window::arm();
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

    let alloc = modes.alloc_window.then(alloc_window::read).flatten();
    let trace = if modes.trace {
        Some(traced_sample(&mut f)?)
    } else {
        None
    };

    let p50_norm = sample_ghz.as_ref().map(|ghz| normalized_p50(&samples, ghz));
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        p50_norm,
        alloc,
        trace,
    })
}

/// Off, `require` is the typed refusal — so `arm` and `read` are unreachable
/// inert twins, honest about a counter that does not exist.
mod alloc_window {
    use bumbledb::alloc_counter::AllocSnapshot;

    #[cfg(feature = "obs")]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "signature twin of the feature-off refusal (the obs.rs law)"
    )]
    pub(super) fn require() -> Result<(), String> {
        Ok(())
    }

    #[cfg(feature = "obs")]
    pub(super) fn arm() {
        bumbledb::alloc_counter::reset();
    }

    #[cfg(feature = "obs")]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "signature twin of the feature-off `None` (the obs.rs law)"
    )]
    pub(super) fn read() -> Option<AllocSnapshot> {
        Some(bumbledb::alloc_counter::snapshot())
    }

    #[cfg(not(feature = "obs"))]
    pub(super) fn require() -> Result<(), String> {
        Err("the alloc window needs the obs feature (bumbledb/alloc-counter)".to_owned())
    }

    #[cfg(not(feature = "obs"))]
    pub(super) fn arm() {}

    #[cfg(not(feature = "obs"))]
    pub(super) fn read() -> Option<AllocSnapshot> {
        None
    }
}
