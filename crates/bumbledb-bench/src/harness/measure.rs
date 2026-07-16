use std::time::Instant;

use super::stats::{normalized_p50, stats};
use super::traced::traced_sample;
use super::{Measurement, Modes, Protocol};

/// [`measure_batched`] in plain per-call timing mode — the thin
/// convenience for the write/scenario families (the former three-layer
/// `measure`/`measure_with`/`measure_batched` stack collapsed to this
/// pair: no caller distinguished the middle layer from
/// `measure_batched(.., 1, ..)`).
///
/// # Errors
///
/// The closure's error, verbatim.
pub fn measure<F>(proto: Protocol, f: F) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    measure_batched(proto, Modes::default(), 1, f)
}

/// The one measurement loop, timing `batch` calls per sample and dividing the
/// elapsed time — the quantum check's mechanism. Work counts sum across
/// every call; batch 1 is the plain protocol.
///
/// # Errors
///
/// The closure's error; a request for both modes at once; an
/// alloc-window request on a build without the `obs` feature.
///
/// # Panics
///
/// On `batch == 0` (a programmer error).
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

/// [`measure_batched`] with an untimed `between` closure run before
/// every warmup call and every timed sample — the in-situ shape
/// (`crate::displaced`): foreign traffic streams BETWEEN engine passes
/// and never inside a timed span, so the sample prices the pass *given*
/// the displacement, not the displacement itself
/// (`docs/reference/apple-silicon-performance.md`
/// `m2max.mem.residency-is-interleaving`). With `batch > 1` the
/// `between` work runs once per timed sample (before the batch), not
/// per call.
///
/// # Errors
///
/// The closure's error; a request for both modes at once; an
/// alloc-window request on a build without the `obs` feature.
///
/// # Panics
///
/// On `batch == 0` (a programmer error).
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
    #[cfg(not(feature = "obs"))]
    if modes.alloc_window {
        return Err("the alloc window needs the obs feature (bumbledb/alloc-counter)".to_owned());
    }
    for _ in 0..proto.warmups {
        between();
        std::hint::black_box(f()?);
    }
    #[cfg(feature = "obs")]
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

    #[cfg(feature = "obs")]
    let alloc = modes.alloc_window.then(bumbledb::alloc_counter::snapshot);
    let trace = if modes.trace {
        Some(traced_sample(&mut f)?)
    } else {
        None
    };
    // Percentiles sort in place, so the normalized p50 (which needs the
    // pre-sort alignment with sample_ghz) computes first.
    let p50_norm = sample_ghz.as_ref().map(|ghz| normalized_p50(&samples, ghz));
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        p50_norm,
        #[cfg(feature = "obs")]
        alloc,
        trace,
    })
}
