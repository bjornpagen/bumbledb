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
        // Spin-settle (measured): the touch's commit
        // fsync just down-clocked this core (DVFS floor 1.05–1.46 GHz,
        // demand-driven recovery) — 2 ms of spin demand reaches the
        // ramp's knee, so the sample measures cold CACHE at working
        // CLOCK instead of conflating the two. NEVER sleep here: the
        // E-core wake lottery (25–40% at ≥ 5 ms, measured) is the
        // sharpest trap.
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

/// The canonical cold touch: commits one `Org` fact whose name carries
/// the serial id under the `__touch_` prefix — distinct forever (serials
/// never repeat) and disjoint from every corpus name (`org-NN`).
pub fn org_touch(
    db: &bumbledb::Db<crate::schema::Ledger>,
) -> impl FnMut() -> Result<(), String> + '_ {
    move || {
        db.write(|tx| {
            let id: crate::schema::OrgId = tx.alloc()?;
            tx.insert(&crate::schema::Org {
                id: crate::schema::OrgId(id.0),
                name: &format!("__touch_{}", id.0),
            })
        })
        .map(|_| ())
        .map_err(|e| format!("cold touch: {e:?}"))
    }
}
