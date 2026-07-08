use std::time::Instant;

use super::stats::stats;
use super::{Measurement, Protocol};

/// The cold protocol, defined exactly: per sample (warmups included),
/// `touch()` runs first — committing one fact, bumping the generation,
/// and evicting the image cache — then `f()` is timed once.
///
/// # Errors
///
/// Either closure's error, verbatim.
pub fn measure_cold<T, F>(proto: Protocol, mut touch: T, mut f: F) -> Result<Measurement, String>
where
    T: FnMut() -> Result<(), String>,
    F: FnMut() -> Result<u64, String>,
{
    let mut samples = Vec::with_capacity(proto.samples as usize);
    let mut work = 0u64;
    for round in 0..proto.warmups + proto.samples {
        touch()?;
        // Spin-settle (docs/silicon2/09, exp 17): the touch's commit
        // fsync just down-clocked this core (DVFS floor 1.05–1.46 GHz,
        // demand-driven recovery) — 2 ms of spin demand reaches the
        // ramp's knee, so the sample measures cold CACHE at working
        // CLOCK instead of conflating the two. NEVER sleep here: the
        // E-core wake lottery (25–40% at ≥ 5 ms) is exp 17's sharpest
        // trap.
        crate::clockproxy::warm_up(std::time::Duration::from_millis(2));
        let start = Instant::now();
        let count = f()?;
        let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        if round >= proto.warmups {
            samples.push(elapsed);
            work += std::hint::black_box(count);
        }
    }
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        p50_norm: None,
        #[cfg(feature = "obs")]
        alloc: None,
        trace: None,
    })
}

/// The canonical cold touch: commits one `Tag` fact whose label carries
/// the serial id under the `__touch_` prefix — unique forever (serials
/// never repeat) and disjoint from every corpus label (`tag-NNN`).
pub fn tag_touch<'d>(db: &'d bumbledb::Db<'_>) -> impl FnMut() -> Result<(), String> + 'd {
    move || {
        db.write(|tx| {
            let id: crate::schema::TagId = tx.alloc()?;
            tx.insert(&crate::schema::Tag {
                id: crate::schema::TagId(id.0),
                label: format!("__touch_{}", id.0),
            })
        })
        .map(|_| ())
        .map_err(|e| format!("cold touch: {e:?}"))
    }
}
